use ion_engine::egui;
use ion_engine::egui::Align2;

use crate::state::Props;

pub fn draw_ui_tips(props: &mut Props) {
    egui::Window::new("Tips")
        .title_bar(false)
        .resizable(false)
        .anchor(Align2::RIGHT_TOP, [-10.0, 10.0])
        .show(props.ui_ctx, |ui| {
            ui.heading("Tips:");
            ui.label("Movement: WASD");
            ui.label("Pause menu: Esc");
            ui.label("Debug menu: F");
            ui.label("\"Shoot\": Space");
        });
}
