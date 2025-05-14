use ion_engine::egui;

use crate::state::Props;

pub fn draw_ui_init_screen(props: &mut Props, progress: f32) {
    egui::CentralPanel::default().show(props.ui_ctx, |ui| {
        ui.vertical_centered_justified(|ui| {
            ui.add_space(80.0);
            ui.label("ION");
            ui.add_space(2.0);
            ui.label(format!("Loading... {}%", (progress * 100.0).round() as u32));
        });
    });
}
