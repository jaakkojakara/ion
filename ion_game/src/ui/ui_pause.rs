use std::sync::atomic::Ordering;

use ion_common::net::NetworkPlayerInfo;
use ion_engine::{
    core::{universe::UniverseDataType, world::WorldType},
    egui::{self, Align2},
};

use crate::{
    state::{Props, state_game::GameState},
    universe::{UniverseData, world::World},
};

pub fn draw_ui_pause(props: &mut Props, state: &mut GameState) {
    if state.is_paused {
        egui::Window::new("Pause")
            .title_bar(false)
            .pivot(Align2::LEFT_CENTER)
            .show(props.ui_ctx, |ui| {
                ui.label("Game Paused");

                if ui.button("Resume").clicked() {
                    state.is_paused = false;
                    props.universe.unpause();
                }

                if ui.button("Save").clicked() {
                    let save_name = "test";
                    // Acquire unique access to the universe, this also locks the universe thread.
                    let universe = props.universe.lock_universe_data();
                    let worlds = props.universe.lock_worlds_data();

                    let mut save_files_bytes: Vec<_> = worlds
                        .values()
                        .map(|world| (world.name.clone(), world.as_bytes()))
                        .collect();

                    save_files_bytes.push((
                        "universe".to_string(),
                        universe
                            .as_ref()
                            .expect("Universe must exist when saving")
                            .as_bytes(&worlds),
                    ));

                    props
                        .files
                        .export_save(save_name, save_files_bytes)
                        .expect("Exporting save must succeed");
                }

                if ui.button("Load").clicked() {
                    let save_name = "test";
                    let mut save_files_bytes = props.files.import_save(save_name).expect("Importing save must succeed");

                    let player = NetworkPlayerInfo {
                        id: 0,
                        name: "player_main".to_string(),
                        addr: "127.0.0.1:0".parse().unwrap(),
                    };

                    let universe = UniverseData::from_bytes(
                        &save_files_bytes
                            .remove("universe")
                            .expect("Universe must exist when loading"),
                        None,
                        Some(player.clone()),
                    );
                    let worlds = save_files_bytes
                        .into_iter()
                        .map(|(_, bytes)| {
                            World::from_bytes(&bytes, Some(player.clone())).expect("Parsing world data must succeed")
                        })
                        .collect();

                    props.universe.load_universe(universe, worlds, None);
                    props.universe.set_active_world_by_name("default_world");
                    props.universe.unpause();
                    state.is_paused = false;
                }

                if ui.button("Exit").clicked() {
                    props.engine_running.store(false, Ordering::Relaxed);
                }
            });
    }
}
