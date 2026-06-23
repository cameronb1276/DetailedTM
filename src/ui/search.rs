use eframe::egui;

use crate::app::state::{AppState, SearchMode};

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    egui::ComboBox::from_id_salt("search_mode")
        .selected_text(state.search_mode.label())
        .show_ui(ui, |ui| {
            for mode in SearchMode::ALL {
                ui.selectable_value(&mut state.search_mode, mode, mode.label());
            }
        });
    ui.add(
        egui::TextEdit::singleline(&mut state.search)
            .hint_text(format!("Search by {}", state.search_mode.label()))
            .desired_width(260.0),
    );
    if !state.search.is_empty() && ui.small_button("Clear").clicked() {
        state.search.clear();
    }
}
