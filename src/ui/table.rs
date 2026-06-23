use eframe::egui;

use crate::backend::ProcessPortRow;

pub fn show(ui: &mut egui::Ui, rows: &[&ProcessPortRow]) {
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("process_table")
            .striped(true)
            .show(ui, |ui| {
                ui.strong("PID");
                ui.strong("Name");
                ui.strong("Ports");
                ui.strong("RAM");
                ui.strong("CPU");
                ui.strong("GPU");
                ui.strong("Status");
                ui.end_row();

                for row in rows {
                    ui.label(row.pid.to_string());
                    ui.label(&row.name).on_hover_text(format!(
                        "Extension: {}\nEnd Task eligible: {}\nLast seen: {} ms ago",
                        if row.extension.is_empty() {
                            "(none)"
                        } else {
                            &row.extension
                        },
                        if row.is_killable { "yes" } else { "no" },
                        row.last_seen.elapsed().as_millis()
                    ));
                    ui.label(row.ports_display());
                    ui.label(&row.ram_usage_display)
                        .on_hover_text(format!("{} bytes", row.ram_usage_bytes));
                    ui.label(format!("{:.1}%", row.cpu_usage_percent));
                    ui.label(
                        row.gpu_usage_percent
                            .map(|value| format!("{value:.1}%"))
                            .unwrap_or_else(|| "N/A".to_owned()),
                    );
                    ui.label(&row.status);
                    ui.end_row();
                }
            });
    });
}
