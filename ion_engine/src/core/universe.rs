use std::sync::atomic::AtomicU64;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use std::{
    fmt::Debug,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use ion_common::net::{NetworkPlayerInfo, NetworkServerInfo};
use ion_common::{Map, log_error, log_info};

use crate::input::input_state::InputState;

use super::{
    DEFAULT_UPS, FrameId,
    world::{ActionType, WorldId, WorldType},
};

// ---------------------------------------------------------- //
// --------------- Universe associated types ---------------- //
// ---------------------------------------------------------- //

/// Global data shared across all worlds within a universe.
///
/// Universe data represents information that should be accessible to all worlds
/// within the same universe, such as global player profiles, campaign progress,
/// universe-wide settings, or shared resources.
///
/// ## Design Principles
///
/// ### Thread Safety
/// Universe data must be `Send + Sync` to enable safe multithreaded access
/// from the universe simulation thread. Any mutability must be internal
/// (using `Mutex`, `RwLock`, `AtomicXxx`, etc.) since the trait itself
/// doesn't provide mutable access.
pub trait UniverseDataType: 'static + Send + Sync {
    type WorldType: WorldType<UniverseDataType = Self>;

    /// Returns the active player information for this universe instance.
    /// If none, the game is run in server-only mode.
    fn active_player(&self) -> Option<&NetworkPlayerInfo>;

    /// Deserializes universe data from a byte array.
    ///
    /// Used for loading saved universes from disk or receiving universe state
    /// over the network in multiplayer scenarios.
    ///
    /// Server and player info provided here is only valid for the current session.
    /// When universe is loaded next time, it may or may not be in multiplayer mode and it may or may not have an active player.
    /// As such, server and player info should not be saved.
    fn from_bytes(bytes: &[u8], server: Option<NetworkServerInfo>, player: Option<NetworkPlayerInfo>) -> Self;

    /// Serializes universe data and all associated worlds to a byte array.
    ///
    /// Used for saving universes to disk or sending complete universe state
    /// over the network. The implementation receives a lock to all worlds
    /// to include their state in the serialized data.
    fn as_bytes(&self, worlds: &MutexGuard<Map<WorldId, Self::WorldType>>) -> Vec<u8>;
}

// ---------------------------------------------------------- //
// ---------------- Universe implementation ----------------- //
// ---------------------------------------------------------- //

/// The central coordinator for multiple game worlds within a single universe.
///
/// A universe represents the top-level container for a game session, managing
/// multiple worlds, coordinating their simulation, and coordinating cross-world
/// concerns like networking, player management, and global state.
///
/// Only a single universe is allowed to exist at a time.
/// Single universe can contain multiple worlds. All the worlds are running in parallel.
/// "Active world" means the world where player is currently in and which is currently being rendered.
///
/// ## Architecture
///
/// The universe sits at the center of the Ion engine's architecture:
///
/// ```text
/// ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
/// │  Render Thread  │    │ Universe Thread │    │ Network Thread  │
/// │                 │    │                 │    │                 │
/// │ • UI Rendering  │◄──►│   UNIVERSE      │◄──►│ • Multiplayer   │
/// │ • Input Capture │    │                 │    │ • Action Sync   │
/// │ • Frame Display │    │ • World Updates │    │ • Player Mgmt   │
/// └─────────────────┘    │ • Action Proc   │    └─────────────────┘
///                        │ • State Mgmt    │
///                        └─────────────────┘
///                                 │
///                        ┌─────────────────┐
///                        │    World A      │
///                        │    World B      │  
///                        │    World C      │
///                        │     ...         │
///                        └─────────────────┘
/// ```
///
/// ## Threading Model
///
/// The universe primarily operates on the **Universe Thread**, separate from
/// the render thread. This allows:
/// - Consistent simulation timing independent of render framerate
/// - Deterministic processing for multiplayer synchronization  
/// - Processing next frame while the previous frame is being rendered.
pub struct Universe<W: WorldType> {
    paused: AtomicBool,
    pause_scheduled: AtomicU64,
    active_frame: AtomicU64,
    active_world_id: AtomicU32,
    universe_frame_time: AtomicU64,
    universe_data: Mutex<Option<W::UniverseDataType>>,
    worlds_data: Mutex<Map<WorldId, W>>,

    action_sender: Sender<ActionMessage<W::ActionType>>,
    action_receiver: Mutex<Receiver<ActionMessage<W::ActionType>>>,
}

impl<W: WorldType> Universe<W> {
    #[allow(clippy::new_without_default)]
    pub(crate) fn new() -> Self {
        let (action_sender, action_receiver) = mpsc::channel::<ActionMessage<W::ActionType>>();
        Self {
            paused: AtomicBool::new(true),
            pause_scheduled: AtomicU64::new(0),
            active_frame: AtomicU64::new(0),
            active_world_id: AtomicU32::new(u32::MAX),
            universe_frame_time: AtomicU64::new(1_000_000_000 / DEFAULT_UPS),
            universe_data: Mutex::new(None),
            worlds_data: Mutex::new(Map::default()),
            action_sender,
            action_receiver: Mutex::new(action_receiver),
        }
    }

    // ---------------------------------------------------------- //
    // ------------------- Universe pausing --------------------- //
    // ---------------------------------------------------------- //

    /// Returns whether the universe simulation is currently running.
    pub fn is_running(&self) -> bool {
        !self.paused.load(Ordering::Acquire)
    }

    /// Returns whether the universe simulation is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Immediately pauses the universe simulation.
    ///
    /// ## Notes
    /// - Pausing while in multiplayer will lead to server dropping this client.
    pub fn pause(&self) {
        log_info!("Pausing universe");
        self.paused.store(true, Ordering::Release);
    }

    /// Resumes universe simulation after being paused.
    pub fn unpause(&self) {
        assert!(self.lock_universe_data().is_some(), "Can't unpause empty universe");
        assert!(!self.lock_worlds_data().is_empty(), "Can't unpause empty universe");

        log_info!("Unpausing universe");
        self.paused.store(false, Ordering::Release);
    }

    /// Schedules the universe to pause automatically at a specific frame.
    ///
    /// ## Notes
    /// - Scheduling a pause for a past frame will be ignored.
    /// - Only one scheduled pause can be active at a time
    /// - Pausing while in multiplier will lead to server dropping this client.
    pub fn schedule_pause(&self, on_frame: FrameId) {
        if on_frame > self.active_frame.load(Ordering::SeqCst) {
            self.pause_scheduled.store(on_frame, Ordering::SeqCst);
            log_info!("Scheduling pause for frame {}", on_frame);
        } else {
            log_error!(
                "Tried to schedule pause for past frame; current_frame: {}, pause_frame: {}",
                self.active_frame(),
                on_frame
            );
        }
    }

    /// Returns the duration of each universe frame.
    pub fn universe_frame_time(&self) -> Duration {
        Duration::from_nanos(self.universe_frame_time.load(Ordering::Relaxed))
    }

    /// Returns the current universe simulation speed in frames per second.
    pub fn universe_speed(&self) -> u32 {
        (1_000_000_000 / self.universe_frame_time.load(Ordering::Relaxed)) as u32
    }

    /// Sets the universe simulation speed.
    /// Universe speed does not affect the simulation, i.e. frame x will have same state regardless of speed.
    pub fn set_universe_speed(&self, target_ups: u32) {
        self.universe_frame_time
            .store(1_000_000_000 / target_ups as u64, Ordering::Relaxed);
    }

    // ---------------------------------------------------------- //
    // ------------------- World management --------------------- //
    // ---------------------------------------------------------- //

    /// Loads universe data and worlds, replacing any existing content.
    ///
    /// This is the primary method for initializing a universe with game content,
    /// whether from a new game, saved file, or network synchronization.
    ///
    /// ## Behavior
    /// - Automatically unloads any existing universe content first
    /// - Inserts all provided worlds into the universe
    /// - Sets the frame counter if provided (used mainly for multiplayer synchronization)
    /// - Universe starts paused - call `unpause()` to run the universe
    pub fn load_universe(
        &self,
        universe_data: W::UniverseDataType,
        worlds_data: Vec<W>,
        active_frame: Option<FrameId>,
    ) {
        self.unload_universe();
        log_info!("Loading universe");

        let mut universe_data_lock = self.universe_data.lock().unwrap();
        let mut worlds_data_lock = self.worlds_data.lock().unwrap();
        *universe_data_lock = Some(universe_data);
        for world in worlds_data {
            worlds_data_lock.insert(world.id(), world);
        }

        if let Some(active_frame) = active_frame {
            self.active_frame.store(active_frame, Ordering::Release);
        }
    }

    /// Unloads all universe content and resets to empty state.
    ///
    /// This clears all worlds, universe data, and resets the frame counter.
    /// The universe will be automatically paused and no world will be active.
    pub fn unload_universe(&self) {
        log_info!("Clearing universe");
        let mut universe_data_lock = self.universe_data.lock().unwrap();
        let mut worlds_data_lock = self.worlds_data.lock().unwrap();
        worlds_data_lock.clear();
        universe_data_lock.take();

        drop(universe_data_lock);
        drop(worlds_data_lock);

        self.pause();
        self.active_world_id.store(u32::MAX, Ordering::SeqCst);
        self.active_frame.store(0, Ordering::SeqCst);
    }

    /// Adds a single world to the universe.
    pub fn load_world(&self, world: W) {
        let mut worlds_data_lock = self.worlds_data.lock().unwrap();
        worlds_data_lock.insert(world.id(), world);
    }

    /// Removes a world from the universe.
    pub fn unload_world(&self, world_id: WorldId) {
        assert!(
            self.active_world_id().map(|id| world_id != id).unwrap_or(true),
            "Can't unload active world"
        );
        let mut worlds_data_lock = self.worlds_data.lock().unwrap();
        worlds_data_lock.remove(&world_id);
    }

    /// Sends an action to the currently active world.
    pub fn send_action_to_active_world(&self, action: <W as WorldType>::ActionType) {
        if let Some(active_id) = self.active_world_id() {
            self.action_sender
                .send(ActionMessage {
                    target_world: Some(active_id),
                    is_stateful: action.is_stateful(),
                    action,
                })
                .unwrap();
        } else {
            panic!("Failed to send action to active world; No world is active");
        }
    }

    /// Sends an action to all worlds in the universe.
    pub fn send_action_to_all_worlds(&self, action: <W as WorldType>::ActionType) {
        self.action_sender
            .send(ActionMessage {
                target_world: None,
                is_stateful: action.is_stateful(),
                action,
            })
            .unwrap();
    }

    /// Sets the active world by name.
    pub fn set_active_world_by_name(&self, world_name: &str) {
        log_info!("Setting active world to {}", world_name);
        let worlds = self.worlds_data.lock().unwrap();
        let world_id = worlds.iter().find(|world| world.1.name() == world_name);

        assert!(world_id.is_some(), "Failed to activate world; The world does not exist");

        assert_ne!(
            self.active_world_id.load(Ordering::Acquire),
            *world_id.unwrap().0,
            "Failed to activate world; The world is already active"
        );

        self.active_world_id.store(*world_id.unwrap().0, Ordering::Release);
    }

    /// Sets the active world by ID.
    pub fn set_active_world_by_id(&self, world_id: WorldId) {
        log_info!("Setting active world to {:?}", world_id);
        let worlds_lock = self.worlds_data.lock().unwrap();
        assert!(
            worlds_lock.contains_key(&world_id),
            "Failed to active world; The world does not exist"
        );
        assert_ne!(
            self.active_world_id.load(Ordering::Acquire),
            world_id,
            "Failed to activate world; The world is already active"
        );

        self.active_world_id.store(world_id, Ordering::Release);
    }

    /// Clears the active world, leaving no world as active.
    pub fn clear_active_world(&self) {
        log_info!("Clearing active world");
        self.active_world_id.store(u32::MAX, Ordering::Release);
    }

    // ---------------------------------------------------------- //
    // ----------------- Low level management ------------------- //
    // ---------------------------------------------------------- //

    /// Acquires a mutex lock on the universe data.
    /// NOTE: This blocks the universe thread.
    pub fn lock_universe_data(&'_ self) -> MutexGuard<'_, Option<W::UniverseDataType>> {
        self.universe_data.lock().unwrap()
    }

    /// Acquires a mutex lock on all worlds data.   
    /// NOTE: This blocks the universe thread.
    pub fn lock_worlds_data(&'_ self) -> MutexGuard<'_, Map<WorldId, W>> {
        self.worlds_data.lock().unwrap()
    }

    /// Returns the current universe frame number.
    pub fn active_frame(&self) -> FrameId {
        self.active_frame.load(Ordering::Acquire)
    }

    /// Returns the ID of the currently active world, if any.
    pub fn active_world_id(&self) -> Option<WorldId> {
        let atomic_id = self.active_world_id.load(Ordering::Acquire);
        if atomic_id != u32::MAX { Some(atomic_id) } else { None }
    }

    // ---------------------------------------------------------- //
    // ----------------- Crate-only functions ------------------- //
    // ---------------------------------------------------------- //

    /// Clears all pending actions from the action queue.
    pub(crate) fn clear_actions(&self) {
        let action_receiver = self.action_receiver.lock().unwrap();
        while let Some(_) = action_receiver.try_iter().next() {}
    }

    /// Builds actions for the current frame from input and queued actions over network.
    #[allow(clippy::collapsible_else_if)]
    #[allow(clippy::type_complexity)]
    pub(crate) fn build_actions(
        &self,
        worlds_lock: &mut MutexGuard<Map<WorldId, W>>,
        input_state_universe: &InputState<W::CommandType>,
    ) -> (Map<WorldId, Vec<W::ActionType>>, Map<WorldId, Vec<W::ActionType>>) {
        let mut cur_frame_stateful_actions: Map<WorldId, Vec<W::ActionType>> = Map::default();
        let mut cur_frame_stateless_actions: Map<WorldId, Vec<W::ActionType>> = Map::default();

        let active_world_id = self.active_world_id();
        for world in worlds_lock.values_mut() {
            let is_active = active_world_id.map(|id| world.id() == id).unwrap_or(false);
            let actions_stateful = world.build_stateful_actions(input_state_universe, is_active);
            let actions_stateless = world.build_stateless_actions(input_state_universe, is_active);

            cur_frame_stateful_actions.insert(world.id(), actions_stateful);
            cur_frame_stateless_actions.insert(world.id(), actions_stateless);
        }

        let action_receiver = self.action_receiver.lock().unwrap();
        for action in action_receiver.try_iter() {
            match action.target_world {
                Some(world_id) => {
                    if action.is_stateful {
                        cur_frame_stateful_actions.entry(world_id).and_modify(|action_vec| {
                            action_vec.push(action.action);
                        });
                    } else {
                        cur_frame_stateless_actions.entry(world_id).and_modify(|action_vec| {
                            action_vec.push(action.action);
                        });
                    }
                }
                _ => {
                    if action.is_stateful {
                        cur_frame_stateful_actions.values_mut().for_each(|action_vec| {
                            action_vec.push(action.action.clone());
                        });
                    } else {
                        cur_frame_stateless_actions.values_mut().for_each(|action_vec| {
                            action_vec.push(action.action.clone());
                        });
                    }
                }
            }
        }
        (cur_frame_stateful_actions, cur_frame_stateless_actions)
    }

    /// Advances to the next universe frame.
    pub(crate) fn next_frame(&self) {
        let current_frame = self.active_frame.fetch_add(1, Ordering::Release) + 1;
        let pause_on = self.pause_scheduled.load(Ordering::Acquire);
        if pause_on == current_frame {
            log_info!("Pausing Universe (scheduled pause)");
            self.pause();
            self.pause_scheduled.store(0, Ordering::Release);
        }
    }
}

/// Internal message structure for action communication between threads.
#[derive(Debug)]
pub struct ActionMessage<C: ActionType> {
    /// Which world should receive this action.
    /// `None` means send to all worlds.
    pub target_world: Option<WorldId>,

    /// Whether this action affects game state.
    /// Cached from the action for efficient processing.
    pub is_stateful: bool,

    /// The actual action to process.
    pub action: C,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::world::{ActionType, CommandType, UiDataType, WorldType};
    use crate::core::{FrameId, UniverseFrameProps, coordinates::ChunkLocation};
    use crate::gfx::{GfxDebugData, GfxGlobalData, GfxSpriteData};
    use crate::input::input_state::InputState;
    use bincode::{Decode, Encode};
    use ion_common::{Map, net::NetworkPlayerInfo};
    use std::sync::atomic::AtomicU32;

    // ---------------------------------------------------------- //
    // -------------------- Mock Implementations ---------------- //
    // ---------------------------------------------------------- //

    /// Mock command type for testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
    enum TestCommand {
        MoveUp,
        MoveDown,
        Attack,
    }
    impl CommandType for TestCommand {}

    /// Mock action type for testing
    #[derive(Debug, Clone, PartialEq, Encode, Decode)]
    enum TestAction {
        Move(u32),
        Stateless(String),
        Stateful(i32),
    }
    impl ActionType for TestAction {
        fn is_stateful(&self) -> bool {
            match self {
                TestAction::Move(_) => true,
                TestAction::Stateful(_) => true,
                TestAction::Stateless(_) => false,
            }
        }
    }

    /// Mock UI data type for testing
    #[derive(Debug)]
    struct TestUiData {
        _message: String,
    }
    impl UiDataType for TestUiData {}

    /// Mock universe data for testing
    #[derive(Debug)]
    struct TestUniverseData {
        name: String,
        player: Option<NetworkPlayerInfo>,
        _counter: AtomicU32,
    }

    impl TestUniverseData {
        fn new(name: String) -> Self {
            Self {
                name,
                player: None,
                _counter: AtomicU32::new(0),
            }
        }
    }

    impl UniverseDataType for TestUniverseData {
        type WorldType = TestWorld;

        fn active_player(&self) -> Option<&NetworkPlayerInfo> {
            self.player.as_ref()
        }

        fn from_bytes(bytes: &[u8], _server: Option<NetworkServerInfo>, _player: Option<NetworkPlayerInfo>) -> Self {
            let name = String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "test".to_string());
            Self::new(name)
        }

        fn as_bytes(&self, _worlds: &MutexGuard<Map<WorldId, Self::WorldType>>) -> Vec<u8> {
            self.name.as_bytes().to_vec()
        }
    }

    /// Mock world for testing
    #[derive(Debug)]
    struct TestWorld {
        id: WorldId,
        name: String,
        action_count: u32,
        stateful_actions: Vec<TestAction>,
        stateless_actions: Vec<TestAction>,
    }

    impl TestWorld {
        fn new(id: WorldId, name: String) -> Self {
            Self {
                id,
                name,
                action_count: 0,
                stateful_actions: Vec::new(),
                stateless_actions: Vec::new(),
            }
        }
    }

    impl WorldType for TestWorld {
        type CommandType = TestCommand;
        type ActionType = TestAction;
        type UiDataType = TestUiData;
        type UniverseDataType = TestUniverseData;

        fn id(&self) -> WorldId {
            self.id
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn from_bytes(bytes: &[u8], _active_player_info: Option<NetworkPlayerInfo>) -> Option<Self> {
            let data = String::from_utf8(bytes.to_vec()).ok()?;
            let parts: Vec<&str> = data.split('|').collect();
            if parts.len() >= 2 {
                let id = parts[0].parse().ok()?;
                let name = parts[1].to_string();
                Some(Self::new(id, name))
            } else {
                None
            }
        }

        fn as_bytes(&self) -> Vec<u8> {
            format!("{}|{}", self.id, self.name).into_bytes()
        }

        fn build_stateful_actions(
            &self,
            _input: &InputState<Self::CommandType>,
            _is_active_world: bool,
        ) -> Vec<Self::ActionType> {
            self.stateful_actions.clone()
        }

        fn build_stateless_actions(
            &self,
            _input: &InputState<Self::CommandType>,
            _is_active_world: bool,
        ) -> Vec<Self::ActionType> {
            self.stateless_actions.clone()
        }

        fn execute_on_universe_frame(&mut self, _props: UniverseFrameProps<Self>) {
            self.action_count += 1;
        }

        fn build_render_data(
            &mut self,
            _frame: FrameId,
            _cached: &[ChunkLocation],
        ) -> (GfxGlobalData, GfxSpriteData, GfxDebugData) {
            use crate::core::coordinates::Location;
            use ion_common::Map;
            (
                GfxGlobalData {
                    frame: 0,
                    camera_loc: Location::new(0.0, 0.0),
                    camera_scale: 1.0,
                    lighting_ambient: 0.5,
                    lighting_sun: 1.0,
                    post_bloom: 0.0,
                },
                GfxSpriteData {
                    chunked_gfx: Map::default(),
                    dynamic_gfx: Vec::new(),
                },
                GfxDebugData {
                    debug_shapes: Vec::new(),
                    debug_labels: Vec::new(),
                },
            )
        }

        fn build_ui_data(&self, _frame: FrameId) -> Self::UiDataType {
            TestUiData {
                _message: format!("World: {}", self.name),
            }
        }
    }

    // ---------------------------------------------------------- //
    // ------------------------- Tests -------------------------- //
    // ---------------------------------------------------------- //

    #[test]
    fn test_universe_creation() {
        let universe: Universe<TestWorld> = Universe::new();

        // Test initial state
        assert!(universe.is_paused());
        assert!(!universe.is_running());
        assert_eq!(universe.active_frame(), 0);
        assert_eq!(universe.active_world_id(), None);
        assert_eq!(universe.universe_speed(), DEFAULT_UPS as u32);

        // Test universe data is empty
        let universe_data_lock = universe.lock_universe_data();
        assert!(universe_data_lock.is_none());
        drop(universe_data_lock);

        // Test no worlds initially
        let worlds_lock = universe.lock_worlds_data();
        assert!(worlds_lock.is_empty());
    }

    #[test]
    fn test_pause_unpause_functionality() {
        let universe: Universe<TestWorld> = Universe::new();

        // Initially paused
        assert!(universe.is_paused());
        assert!(!universe.is_running());

        // Load some data so we can unpause
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "test_world".to_string());
        universe.load_universe(universe_data, vec![world], None);

        // Unpause
        universe.unpause();
        assert!(!universe.is_paused());
        assert!(universe.is_running());

        // Pause again
        universe.pause();
        assert!(universe.is_paused());
        assert!(!universe.is_running());
    }

    #[test]
    #[should_panic(expected = "Can't unpause empty universe")]
    fn test_unpause_empty_universe_panics() {
        let universe: Universe<TestWorld> = Universe::new();
        universe.unpause();
    }

    #[test]
    fn test_scheduled_pause() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "test_world".to_string());
        universe.load_universe(universe_data, vec![world], None);

        // Schedule pause for frame 5
        universe.schedule_pause(5);
        universe.unpause();

        // Advance to frame 4 - should still be running
        for _ in 0..4 {
            universe.next_frame();
        }
        assert!(universe.is_running());
        assert_eq!(universe.active_frame(), 4);

        // Advance to frame 5 - should be paused
        universe.next_frame();
        assert!(universe.is_paused());
        assert_eq!(universe.active_frame(), 5);
    }

    #[test]
    fn test_schedule_pause_for_past_frame() {
        let universe: Universe<TestWorld> = Universe::new();

        // Advance to frame 10
        for _ in 0..10 {
            universe.next_frame();
        }

        // Try to schedule pause for past frame (should be ignored)
        universe.schedule_pause(5);

        // Should not affect current state
        assert!(universe.is_paused()); // Still paused because no universe data
    }

    #[test]
    fn test_universe_speed_control() {
        let universe: Universe<TestWorld> = Universe::new();

        // Test default speed
        assert_eq!(universe.universe_speed(), DEFAULT_UPS as u32);
        assert_eq!(
            universe.universe_frame_time(),
            Duration::from_nanos(1_000_000_000 / DEFAULT_UPS)
        );

        // Change speed to 30 UPS
        universe.set_universe_speed(30);
        assert_eq!(universe.universe_speed(), 30);
        assert_eq!(universe.universe_frame_time(), Duration::from_nanos(1_000_000_000 / 30));

        // Change speed to 120 UPS
        universe.set_universe_speed(120);
        assert_eq!(universe.universe_speed(), 120);
        assert_eq!(
            universe.universe_frame_time(),
            Duration::from_nanos(1_000_000_000 / 120)
        );
    }

    #[test]
    fn test_load_and_unload_universe() {
        let universe: Universe<TestWorld> = Universe::new();

        // Load universe data and worlds
        let universe_data = TestUniverseData::new("test_universe".to_string());
        let world1 = TestWorld::new(1, "world1".to_string());
        let world2 = TestWorld::new(2, "world2".to_string());
        universe.load_universe(universe_data, vec![world1, world2], Some(42));

        // Verify loaded
        let universe_data_lock = universe.lock_universe_data();
        assert!(universe_data_lock.is_some());
        assert_eq!(universe_data_lock.as_ref().unwrap().name, "test_universe");
        drop(universe_data_lock);

        let worlds_lock = universe.lock_worlds_data();
        assert_eq!(worlds_lock.len(), 2);
        assert!(worlds_lock.contains_key(&1));
        assert!(worlds_lock.contains_key(&2));
        drop(worlds_lock);

        assert_eq!(universe.active_frame(), 42);

        // Unload universe
        universe.unload_universe();

        // Verify unloaded
        let universe_data_lock = universe.lock_universe_data();
        assert!(universe_data_lock.is_none());
        drop(universe_data_lock);

        let worlds_lock = universe.lock_worlds_data();
        assert!(worlds_lock.is_empty());
        drop(worlds_lock);

        assert_eq!(universe.active_frame(), 0);
        assert!(universe.is_paused());
        assert_eq!(universe.active_world_id(), None);
    }

    #[test]
    fn test_load_and_unload_individual_worlds() {
        let universe: Universe<TestWorld> = Universe::new();

        // Load a world
        let world1 = TestWorld::new(1, "world1".to_string());
        universe.load_world(world1);

        let worlds_lock = universe.lock_worlds_data();
        assert_eq!(worlds_lock.len(), 1);
        assert!(worlds_lock.contains_key(&1));
        drop(worlds_lock);

        // Load another world
        let world2 = TestWorld::new(2, "world2".to_string());
        universe.load_world(world2);

        let worlds_lock = universe.lock_worlds_data();
        assert_eq!(worlds_lock.len(), 2);
        drop(worlds_lock);

        // Unload a world
        universe.unload_world(1);

        let worlds_lock = universe.lock_worlds_data();
        assert_eq!(worlds_lock.len(), 1);
        assert!(!worlds_lock.contains_key(&1));
        assert!(worlds_lock.contains_key(&2));
    }

    #[test]
    #[should_panic(expected = "Can't unload active world")]
    fn test_unload_active_world_panics() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "world1".to_string());
        universe.load_universe(universe_data, vec![world], None);
        universe.set_active_world_by_id(1);

        // Should panic
        universe.unload_world(1);
    }

    #[test]
    fn test_set_active_world_by_id() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world1 = TestWorld::new(1, "world1".to_string());
        let world2 = TestWorld::new(2, "world2".to_string());
        universe.load_universe(universe_data, vec![world1, world2], None);

        // Set active world
        universe.set_active_world_by_id(1);
        assert_eq!(universe.active_world_id(), Some(1));

        // Change active world
        universe.set_active_world_by_id(2);
        assert_eq!(universe.active_world_id(), Some(2));

        // Clear active world
        universe.clear_active_world();
        assert_eq!(universe.active_world_id(), None);
    }

    #[test]
    #[should_panic(expected = "Failed to active world; The world does not exist")]
    fn test_set_nonexistent_world_active_panics() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "world1".to_string());
        universe.load_universe(universe_data, vec![world], None);

        // Should panic
        universe.set_active_world_by_id(999);
    }

    #[test]
    #[should_panic(expected = "Failed to activate world; The world is already active")]
    fn test_set_already_active_world_panics() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "world1".to_string());
        universe.load_universe(universe_data, vec![world], None);
        universe.set_active_world_by_id(1);

        // Should panic
        universe.set_active_world_by_id(1);
    }

    #[test]
    fn test_set_active_world_by_name() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world1 = TestWorld::new(1, "world1".to_string());
        let world2 = TestWorld::new(2, "world2".to_string());
        universe.load_universe(universe_data, vec![world1, world2], None);

        // Set active world by name
        universe.set_active_world_by_name("world1");
        assert_eq!(universe.active_world_id(), Some(1));

        // Change active world by name
        universe.set_active_world_by_name("world2");
        assert_eq!(universe.active_world_id(), Some(2));
    }

    #[test]
    #[should_panic(expected = "Failed to activate world; The world does not exist")]
    fn test_set_nonexistent_world_by_name_panics() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "world1".to_string());
        universe.load_universe(universe_data, vec![world], None);

        // Should panic
        universe.set_active_world_by_name("nonexistent");
    }

    #[test]
    fn test_send_action_to_active_world() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "world1".to_string());
        universe.load_universe(universe_data, vec![world], None);
        universe.set_active_world_by_id(1);

        // Send action to active world
        let action = TestAction::Move(42);
        universe.send_action_to_active_world(action.clone());

        // Check that action was received
        let action_receiver = universe.action_receiver.lock().unwrap();
        let received: Vec<_> = action_receiver.try_iter().collect();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].action, action);
        assert_eq!(received[0].target_world, Some(1));
        assert!(received[0].is_stateful);
    }

    #[test]
    #[should_panic(expected = "Failed to send action to active world; No world is active")]
    fn test_send_action_to_no_active_world_panics() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world = TestWorld::new(1, "world1".to_string());
        universe.load_universe(universe_data, vec![world], None);

        // Should panic - no active world
        universe.send_action_to_active_world(TestAction::Move(42));
    }

    #[test]
    fn test_send_action_to_all_worlds() {
        let universe: Universe<TestWorld> = Universe::new();
        let universe_data = TestUniverseData::new("test".to_string());
        let world1 = TestWorld::new(1, "world1".to_string());
        let world2 = TestWorld::new(2, "world2".to_string());
        universe.load_universe(universe_data, vec![world1, world2], None);

        // Send action to all worlds
        let action = TestAction::Stateless("broadcast".to_string());
        universe.send_action_to_all_worlds(action.clone());

        // Check that action was received
        let action_receiver = universe.action_receiver.lock().unwrap();
        let received: Vec<_> = action_receiver.try_iter().collect();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].action, action);
        assert_eq!(received[0].target_world, None);
        assert!(!received[0].is_stateful);
    }

    #[test]
    fn test_clear_actions() {
        let universe: Universe<TestWorld> = Universe::new();

        // Send some actions
        universe
            .action_sender
            .send(ActionMessage {
                target_world: Some(1),
                is_stateful: true,
                action: TestAction::Move(1),
            })
            .unwrap();

        universe
            .action_sender
            .send(ActionMessage {
                target_world: Some(2),
                is_stateful: false,
                action: TestAction::Stateless("test".to_string()),
            })
            .unwrap();

        // Clear actions
        universe.clear_actions();

        // Verify actions are cleared
        let action_receiver = universe.action_receiver.lock().unwrap();
        let received: Vec<_> = action_receiver.try_iter().collect();
        assert_eq!(received.len(), 0);
    }

    #[test]
    fn test_frame_advancement() {
        let universe: Universe<TestWorld> = Universe::new();

        assert_eq!(universe.active_frame(), 0);

        universe.next_frame();
        assert_eq!(universe.active_frame(), 1);

        universe.next_frame();
        assert_eq!(universe.active_frame(), 2);

        // Advance multiple frames
        for _ in 0..10 {
            universe.next_frame();
        }
        assert_eq!(universe.active_frame(), 12);
    }

    #[test]
    fn test_build_actions() {
        use std::sync::mpsc;
        let universe: Universe<TestWorld> = Universe::new();

        // Create mock input state
        let (input_sender, input_receiver) = mpsc::channel();
        let (input_sender2, _) = mpsc::channel(); // Second sender for the array
        let input_state = InputState::new(input_receiver, [input_sender, input_sender2]);

        // Create test worlds with actions
        let mut world1 = TestWorld::new(1, "world1".to_string());
        world1.stateful_actions = vec![TestAction::Stateful(1)];
        world1.stateless_actions = vec![TestAction::Stateless("world1".to_string())];

        let mut world2 = TestWorld::new(2, "world2".to_string());
        world2.stateful_actions = vec![TestAction::Stateful(2)];
        world2.stateless_actions = vec![TestAction::Stateless("world2".to_string())];

        // Add actions directly to the universe via action sender
        universe
            .action_sender
            .send(ActionMessage {
                target_world: Some(1),
                is_stateful: true,
                action: TestAction::Stateful(1),
            })
            .unwrap();

        universe
            .action_sender
            .send(ActionMessage {
                target_world: Some(2),
                is_stateful: false,
                action: TestAction::Stateless("test".to_string()),
            })
            .unwrap();

        // Test the real build_actions method with a simplified setup
        let universe_data = TestUniverseData::new("test".to_string());
        universe.load_universe(universe_data, vec![world1, world2], None);

        let mut worlds_lock = universe.lock_worlds_data();
        universe.active_world_id.store(1, Ordering::Release);

        // Build actions
        let (stateful_actions, stateless_actions) = universe.build_actions(&mut worlds_lock, &input_state);

        // Verify actions were built correctly
        assert_eq!(stateful_actions.len(), 2);
        assert_eq!(stateless_actions.len(), 2);

        assert!(stateful_actions.contains_key(&1));
        assert!(stateful_actions.contains_key(&2));
        assert!(stateless_actions.contains_key(&1));
        assert!(stateless_actions.contains_key(&2));

        // World 1 should have its own action plus the injected action
        assert_eq!(stateful_actions[&1].len(), 2);
        // World 2 should have just its own action
        assert_eq!(stateful_actions[&2].len(), 1);
        // World 1 should have just its own stateless action
        assert_eq!(stateless_actions[&1].len(), 1);
        // World 2 should have its own action plus the injected stateless action
        assert_eq!(stateless_actions[&2].len(), 2);
    }

    #[test]
    fn test_action_message_debug() {
        let action_msg = ActionMessage {
            target_world: Some(1),
            is_stateful: true,
            action: TestAction::Move(42),
        };

        let debug_str = format!("{:?}", action_msg);
        assert!(debug_str.contains("ActionMessage"));
        assert!(debug_str.contains("target_world: Some(1)"));
        assert!(debug_str.contains("is_stateful: true"));
        assert!(debug_str.contains("Move(42)"));
    }
}
