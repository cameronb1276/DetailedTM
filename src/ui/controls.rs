use eframe::egui;

use crate::app::state::AppState;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.heading("DetailedTM");
        ui.label(format!("{} processes", state.rows.len()));
        if ui.button("Refresh").clicked() {
            state.refresh();
        }
    });
    if let Some(error) = &state.last_error {
        ui.colored_label(ui.visuals().error_fg_color, error);
    }
}
