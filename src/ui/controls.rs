use eframe::egui;

use crate::app::state::AppState;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.heading("DetailedTM");
        ui.separator();
        crate::ui::search::show(ui, state);
        ui.separator();
        if ui.button("Refresh now").clicked() {
            state.request_refresh();
            state.last_status_message = "Refresh requested".to_owned();
        }

        let can_end = state.selected_row().is_some_and(|row| row.is_killable);
        if ui
            .add_enabled(can_end, egui::Button::new("End Task"))
            .on_disabled_hover_text("Select a killable process first")
            .clicked()
        {
            state.begin_end_task();
        }
    });
    ui.add_space(6.0);
}

pub fn show_status(ui: &mut egui::Ui, state: &AppState) {
    let visible = state.visible_indices().len();
    ui.add_space(3.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(format!(
            "Showing {visible} of {} processes",
            state.rows.len()
        ));
        ui.separator();
        ui.label(format!("Last refresh: {}", state.last_refresh));
        if let Some(pid) = state.selected_pid {
            ui.separator();
            ui.label(format!("Selected PID: {pid}"));
        }
        if !state.last_status_message.is_empty() {
            ui.separator();
            ui.label(&state.last_status_message);
        }
        if let Some(warning) = &state.backend_warning {
            ui.separator();
            ui.colored_label(ui.visuals().warn_fg_color, format!("Warning: {warning}"));
        }
    });
    ui.add_space(3.0);
}

pub fn show_confirmation(context: &egui::Context, state: &mut AppState) {
    let Some(pid) = state.pending_confirmation else {
        return;
    };
    let name = state
        .rows
        .iter()
        .find(|row| row.pid == pid)
        .map(|row| row.name.clone())
        .unwrap_or_else(|| "Unknown process".to_owned());

    egui::Window::new("End Task?")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(context, |ui| {
            ui.label(format!("End {name} (PID {pid})?"));
            ui.label("Unsaved work in this process may be lost.");
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Cancel").clicked() {
                    state.cancel_end_task();
                }
                if ui.button("End Task").clicked() {
                    state.confirm_end_task();
                }
            });
        });
}
