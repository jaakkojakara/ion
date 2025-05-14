use ion_engine::{
    KeyCode,
    core::world::ActionType,
    egui::{self, RichText},
    gfx::GfxFrameMode,
};

use crate::{
    state::{Props, state_game::GameState},
    universe::actions::Action,
};

pub fn draw_ui_debug(props: &mut Props, state: &mut GameState) {
    if props.ui_input_state.is_key_just_pressed(KeyCode::KeyF) {
        state.show_debug_ui_main = !state.show_debug_ui_main;
        state.show_debug_ui_lighting = state.show_debug_ui_lighting && state.show_debug_ui_main;
        Action::DebugSysEnabled(state.show_debug_ui_main).send_to_active(props);
    }

    if state.show_debug_ui_main {
        draw_ui_debug_main(props, state);
    }

    if state.show_debug_ui_lighting {
        draw_ui_debug_lighting(props);
    }
}

pub fn draw_ui_debug_main(props: &mut Props, state: &mut GameState) {
    egui::Window::new("debug_ui").show(props.ui_ctx, |ui| {
        ui.set_min_width(200.0);
        ui.label(RichText::new("Debug menu").heading());
        ui.add_space(10.0);

        if let Some(gfx_data) = props.gfx_data {
            ui.label(format!("Frame time: {:?}", gfx_data.timing_data.render_frame_duration));
            ui.label(format!(
                "Update time: {:?}",
                gfx_data.timing_data.universe_frame_duration
            ));
        }

        ui.add_space(10.0);

        if ui.button("Toggle chunk borders").clicked() {
            Action::DebugToggleChunkBorders.send_to_active(props);
        }

        if ui.button("Toggle tile borders").clicked() {
            Action::DebugToggleTileBorders.send_to_active(props);
        }

        ui.add_space(20.0);
        ui.label("Frame mode");

        if ui.button("Normal").clicked() {
            props.renderer.set_frame_mode(GfxFrameMode::Normal);
        }

        ui.horizontal(|ui| {
            if ui.button("RawColorPass").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::RawColorPass);
            }
            if ui.button("RawNormalPass").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::RawNormalPass);
            }
            if ui.button("RawHeightIdPass").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::RawHeightIdPass);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("RawLightPass").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::RawLightPass);
            }
            if ui.button("RawShadowPass").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::RawShadowPass);
            }
        });

        ui.horizontal(|ui| {
            if ui.button("SsaoPass").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::SsaoPass);
            }
            if ui.button("Complete lighting").clicked() {
                props.renderer.set_frame_mode(GfxFrameMode::LightPass);
            }
        });

        ui.add_space(20.0);

        if ui.button("Show lighting menu").clicked() {
            state.show_debug_ui_lighting = !state.show_debug_ui_lighting;
        }
    });
}

pub fn draw_ui_debug_lighting(props: &mut Props) {
    let ui_debug_data = props.ui_data.as_ref().unwrap().debug.as_ref().unwrap();
    let mut lighting_sun = ui_debug_data.lighting_sun;
    let mut lighting_ambient = ui_debug_data.lighting_ambient;

    egui::Window::new("debug_ui_lighting")
        .title_bar(false)
        .show(props.ui_ctx, |ui| {
            ui.label("Lighting");

            if ui
                .add(egui::Slider::new(&mut lighting_sun, 0.0..=10.0).text("Sun"))
                .changed()
            {
                Action::SetLightingSun(lighting_sun).send_to_active(props);
            }

            if ui
                .add(egui::Slider::new(&mut lighting_ambient, 0.0..=5.0).text("Ambient"))
                .changed()
            {
                Action::SetLightingAmbient(lighting_ambient).send_to_active(props);
            }
        });
}
