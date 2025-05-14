use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use ion_common::DateTime;
use ion_common::Map;
use ion_common::bincode::config::Configuration;
use ion_common::bincode::{Decode, Encode};
use ion_common::net::{NetworkPlayerInfo, NetworkServerInfo};

use crate::APP_VERSION;
use ion_engine::core::universe::UniverseDataType;
use ion_engine::core::world::WorldId;

use crate::universe::players::{UniversePlayers, UniversePlayersSaveState};
use crate::universe::world::World;

mod camera;
mod chunk;
mod debug;
mod entities;
mod players;

pub mod actions;
pub mod creator;
pub mod systems;
pub mod world;

#[derive(Debug)]
pub struct UniverseData {
    pub app_version: &'static str,
    pub name: String,
    pub seed: u64,
    pub stats: UniverseStats,
    pub server: Option<NetworkServerInfo>,
    pub player: Option<NetworkPlayerInfo>,
    pub players: Mutex<UniversePlayers>,
}

impl UniverseData {
    fn new(name: String, seed: u64, server: Option<NetworkServerInfo>, player: Option<NetworkPlayerInfo>) -> Self {
        Self {
            app_version: APP_VERSION,
            name,
            seed,
            stats: UniverseStats::default(),
            player,
            server,
            players: Mutex::new(UniversePlayers::new(34)),
        }
    }
}

impl UniverseDataType for UniverseData {
    type WorldType = World;

    fn active_player(&self) -> Option<&NetworkPlayerInfo> {
        self.player.as_ref()
    }

    fn from_bytes(bytes: &[u8], server: Option<NetworkServerInfo>, player: Option<NetworkPlayerInfo>) -> Self {
        let (save_state, _) =
            bincode::decode_from_slice::<UniverseSaveState, Configuration>(bytes, bincode::config::standard())
                .expect("Decoding universe data must succeed");
        save_state.into_actual(server, player)
    }

    fn as_bytes(&self, worlds: &MutexGuard<Map<WorldId, World>>) -> Vec<u8> {
        let save_state = UniverseSaveState::from_actual(self, worlds);
        bincode::encode_to_vec(save_state, bincode::config::standard()).unwrap()
    }
}

// ---------------------------------------------------------- //
// ------------------ Internal-ish helpers ------------------ //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Encode, Decode)]
pub struct UniverseStats {
    pub play_time: Duration,
    pub last_start: DateTime,
    pub last_played: DateTime,
}

impl Default for UniverseStats {
    fn default() -> Self {
        Self {
            play_time: Duration::from_secs(0),
            last_start: DateTime::now(),
            last_played: DateTime::now(),
        }
    }
}

// ---------------------------------------------------------- //
// ---------------------- Save states ----------------------- //
// ---------------------------------------------------------- //

#[derive(Debug, Clone, Encode, Decode)]
struct UniverseSaveState {
    app_version: String,
    name: String,
    seed: u64,
    stats: UniverseStats,
    players: UniversePlayersSaveState,
}

impl UniverseSaveState {
    fn into_actual(self, server: Option<NetworkServerInfo>, player: Option<NetworkPlayerInfo>) -> UniverseData {
        assert_eq!(APP_VERSION, self.app_version, "Save version and app version must match");
        let stats = UniverseStats {
            play_time: self.stats.play_time,
            last_start: DateTime::now(),
            last_played: DateTime::now(),
        };

        UniverseData {
            app_version: APP_VERSION,
            name: self.name,
            seed: self.seed,
            stats,
            server,
            player,
            players: Mutex::new(self.players.into_actual()),
        }
    }
    fn from_actual(actual: &UniverseData, worlds: &MutexGuard<Map<WorldId, World>>) -> Self {
        let session = DateTime::now()
            .duration_since(actual.stats.last_start)
            .unwrap_or(Duration::ZERO);
        let stats = UniverseStats {
            play_time: actual.stats.play_time + session,
            last_start: actual.stats.last_start,
            last_played: DateTime::now(),
        };

        Self {
            app_version: actual.app_version.to_string(),
            name: actual.name.clone(),
            seed: actual.seed,
            stats,
            players: UniversePlayersSaveState::from_actual(&actual.players, worlds),
        }
    }
}
