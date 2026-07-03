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
        ui.separator();
        if state.traffic_status.running {
            if ui.button("Stop Traffic Capture").clicked() {
                state.stop_traffic_capture();
            }
            ui.colored_label(ui.visuals().selection.bg_fill, "Capture: running");
        } else if ui.button("Start Traffic Capture").clicked() {
            state.start_traffic_capture();
        } else {
            ui.label("Capture: stopped");
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
        ui.separator();
        ui.label(&state.traffic_status.last_message);
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

pub fn show_log_viewer(context: &egui::Context, state: &mut AppState) {
    let Some(viewer) = &mut state.traffic_log_viewer else {
        return;
    };
    let mut open = true;
    egui::Window::new(&viewer.title)
        .open(&mut open)
        .default_width(900.0)
        .default_height(520.0)
        .show(context, |ui| {
            ui.label(format!("Log root: {}", viewer.root.display()));
            if viewer.files.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "No traffic logs found for this process.",
                );
                return;
            }

            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Files: {}", viewer.files.len()));
                ui.separator();
                ui.label(format!("Displayed entries: {}", viewer.entries.len()));
                ui.separator();
                ui.label(format!("Skipped malformed lines: {}", viewer.skipped_lines));
            });
            if !viewer.errors.is_empty() {
                ui.collapsing("Read warnings", |ui| {
                    for error in &viewer.errors {
                        ui.colored_label(ui.visuals().warn_fg_color, error);
                    }
                });
            }
            ui.collapsing("Files read", |ui| {
                for path in &viewer.files {
                    ui.monospace(path.display().to_string());
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for entry in &viewer.entries {
                        ui.monospace(entry);
                    }
                });
        });
    if !open {
        state.traffic_log_viewer = None;
    }
}
