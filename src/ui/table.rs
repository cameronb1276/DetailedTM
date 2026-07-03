use eframe::egui;
use egui_extras::{Column, TableBuilder};

use crate::app::state::{AppState, SortColumn};

const ROW_HEIGHT: f32 = 24.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let indices = state.visible_indices();
    let mut copied_network_details = None;
    if let Some(row) = state.selected_row() {
        let network_details = NetworkDetailsText::from_row(row);
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("Selected: {} (PID {})", row.name, row.pid));
            let ports = row.ports_display();
            ui.label(if ports.is_empty() {
                "Ports: none".to_owned()
            } else {
                format!("Ports: {ports}")
            });
            ui.separator();
            ui.label(format!(
                "Downloaded: {} | Uploaded: {}",
                optional_bytes(row.network_usage_available, row.download_bytes),
                optional_bytes(row.network_usage_available, row.upload_bytes)
            ));
        });
        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id(format!("network_details_{}", row.pid)),
            true,
        )
        .show_header(ui, |ui| {
            ui.label("Network activity and destinations");
            if ui
                .small_button("Copy")
                .on_hover_text("Copy the network activity and destinations shown here")
                .clicked()
            {
                ui.ctx().copy_text(network_details.copy_text());
                copied_network_details = Some((row.name.clone(), row.pid));
            }
        })
        .body(|ui| show_network_details(ui, &network_details));
        ui.separator();
    }
    let headers = [
        (state.header_label("PID", SortColumn::Pid), SortColumn::Pid),
        (
            state.header_label("name.extension", SortColumn::Name),
            SortColumn::Name,
        ),
        (
            state.header_label("port", SortColumn::Port),
            SortColumn::Port,
        ),
        (
            state.header_label("Ram Usage", SortColumn::Ram),
            SortColumn::Ram,
        ),
        (
            state.header_label("CPU Usage", SortColumn::Cpu),
            SortColumn::Cpu,
        ),
        (
            state.header_label("GPU Usage", SortColumn::Gpu),
            SortColumn::Gpu,
        ),
        (
            state.header_label("Download", SortColumn::Download),
            SortColumn::Download,
        ),
        (
            state.header_label("Upload", SortColumn::Upload),
            SortColumn::Upload,
        ),
    ];
    let mut requested_sort = None;
    let mut clicked_pid = None;
    let mut context_end_task = None;
    let mut context_open_file = None;
    let mut context_open_logs = None;

    TableBuilder::new(ui)
        .id_salt("process_table")
        .striped(true)
        .resizable(true)
        .sense(egui::Sense::click())
        .column(Column::initial(70.0).at_least(55.0))
        .column(Column::initial(240.0).at_least(120.0))
        .column(Column::remainder().at_least(180.0))
        .column(Column::initial(110.0).at_least(85.0))
        .column(Column::initial(100.0).at_least(80.0))
        .column(Column::initial(100.0).at_least(80.0))
        .column(Column::initial(110.0).at_least(90.0))
        .column(Column::initial(110.0).at_least(90.0))
        .header(28.0, |mut header| {
            for (label, column) in headers {
                header.col(|ui| {
                    if ui.button(label).clicked() {
                        requested_sort = Some(column);
                    }
                });
            }
        })
        .body(|body| {
            body.rows(ROW_HEIGHT, indices.len(), |mut table_row| {
                let row = &state.rows[indices[table_row.index()]];
                table_row.set_selected(state.selected_pid == Some(row.pid));
                let full_ports = row.ports_display();
                let short_ports = shorten(&full_ports, 72);

                table_row.col(|ui| {
                    if clickable_label(ui, row.pid.to_string()).clicked() {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    if clickable_label(ui, &row.name)
                        .on_hover_text(format!(
                            "Status: {}\nEnd Task eligible: {}\nLast seen: {} ms ago",
                            row.status,
                            if row.is_killable { "yes" } else { "no" },
                            row.last_seen.elapsed().as_millis()
                        ))
                        .clicked()
                    {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    if clickable_label(ui, short_ports)
                        .on_hover_text(if full_ports.is_empty() {
                            "No IPv4 TCP or UDP ports".to_owned()
                        } else {
                            full_ports.clone()
                        })
                        .clicked()
                    {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    if clickable_label(ui, &row.ram_usage_display)
                        .on_hover_text(format!("{} bytes", row.ram_usage_bytes))
                        .clicked()
                    {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    if clickable_label(ui, format!("{:.1}%", row.cpu_usage_percent)).clicked() {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    let gpu = row
                        .gpu_usage_percent
                        .map(|value| format!("{value:.1}%"))
                        .unwrap_or_else(|| "N/A".to_owned());
                    if clickable_label(ui, gpu).clicked() {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    if clickable_label(
                        ui,
                        optional_rate(
                            row.network_usage_available,
                            row.download_rate_bytes_per_second,
                        ),
                    )
                    .on_hover_text(format!(
                        "{} downloaded since DetailedTM started tracking this process",
                        optional_bytes(row.network_usage_available, row.download_bytes)
                    ))
                    .clicked()
                    {
                        clicked_pid = Some(row.pid);
                    }
                });
                table_row.col(|ui| {
                    if clickable_label(
                        ui,
                        optional_rate(
                            row.network_usage_available,
                            row.upload_rate_bytes_per_second,
                        ),
                    )
                    .on_hover_text(format!(
                        "{} uploaded since DetailedTM started tracking this process",
                        optional_bytes(row.network_usage_available, row.upload_bytes)
                    ))
                    .clicked()
                    {
                        clicked_pid = Some(row.pid);
                    }
                });

                let response = table_row.response();
                response.context_menu(|ui| {
                    if ui.button("End Task").clicked() {
                        context_end_task = Some(row.pid);
                        ui.close_menu();
                    }
                    if ui.button("Open File Location").clicked() {
                        context_open_file = Some(row.pid);
                        ui.close_menu();
                    }
                    if ui.button("Open Traffic Logs").clicked() {
                        context_open_logs = Some(row.pid);
                        ui.close_menu();
                    }
                });

                if response.clicked() {
                    clicked_pid = Some(row.pid);
                }
            });
        });

    if let Some(column) = requested_sort {
        state.set_sort(column);
    }
    if let Some(pid) = clicked_pid {
        state.select(pid);
    }
    if let Some(pid) = context_end_task {
        state.select(pid);
        state.begin_end_task();
    }
    if let Some(pid) = context_open_file {
        state.select(pid);
        state.open_selected_file_location();
    }
    if let Some(pid) = context_open_logs {
        state.select(pid);
        state.open_selected_traffic_logs();
    }
    if let Some((name, pid)) = copied_network_details {
        state.last_status_message =
            format!("Copied network activity and destinations for {name} (PID {pid})");
    }
}

fn clickable_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    ui.add(egui::Label::new(text).sense(egui::Sense::click()))
}

fn show_network_details(ui: &mut egui::Ui, details: &NetworkDetailsText) {
    ui.label(&details.current_rate);
    if let Some(warning) = &details.warning {
        ui.colored_label(ui.visuals().warn_fg_color, warning);
    }
    for destination in &details.destinations {
        ui.monospace(destination);
    }
    if let Some(note) = &details.destination_note {
        ui.label(note);
    }
    ui.small(details.content_note);
}

struct NetworkDetailsText {
    current_rate: String,
    warning: Option<String>,
    destinations: Vec<String>,
    destination_note: Option<String>,
    content_note: &'static str,
}

impl NetworkDetailsText {
    fn from_row(row: &crate::backend::ProcessPortRow) -> Self {
        let mut destinations = Vec::new();
        let mut visible_destinations = 0_usize;
        let mut has_hidden_destinations = false;

        for binding in &row.ports {
            let Some(remote_addr) = binding.remote_addr else {
                continue;
            };
            let Some(remote_port) = binding.remote_port else {
                continue;
            };
            if remote_port == 0 || remote_addr.is_unspecified() {
                continue;
            }
            visible_destinations += 1;
            if visible_destinations > 8 {
                has_hidden_destinations = true;
                break;
            }
            let sent = binding
                .bytes_sent
                .map(format_bytes)
                .unwrap_or_else(|| "unavailable".to_owned());
            let received = binding
                .bytes_received
                .map(format_bytes)
                .unwrap_or_else(|| "unavailable".to_owned());
            destinations.push(format!(
                "{} {}:{} → {}:{} {} | ↓ {} ↑ {}",
                binding.protocol,
                binding.local_addr,
                binding.local_port,
                remote_addr,
                remote_port,
                binding.state,
                received,
                sent
            ));
        }

        let destination_note = if destinations.is_empty() {
            Some("No active remote IPv4 TCP destination is visible for this process.".to_owned())
        } else if has_hidden_destinations {
            Some(
                "Additional destinations are hidden to keep the detail panel responsive."
                    .to_owned(),
            )
        } else {
            None
        };

        Self {
            current_rate: format!(
                "Current rate: ↓ {}  ↑ {}",
                optional_rate(
                    row.network_usage_available,
                    row.download_rate_bytes_per_second
                ),
                optional_rate(
                    row.network_usage_available,
                    row.upload_rate_bytes_per_second
                )
            ),
            warning: (!row.network_usage_available).then(|| {
                "TCP byte counters are unavailable. Run DetailedTM as administrator to enable upload/download measurement."
                    .to_owned()
            }),
            destinations,
            destination_note,
            content_note: "Content visibility: metadata only. HTTPS/TLS commands, files, request URLs, and response bodies are encrypted and are not captured or guessed. Plaintext payload capture is also off.",
        }
    }

    fn copy_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push(self.current_rate.clone());
        if let Some(warning) = &self.warning {
            lines.push(warning.clone());
        }
        lines.extend(self.destinations.iter().cloned());
        if let Some(note) = &self.destination_note {
            lines.push(note.clone());
        }
        lines.push(self.content_note.to_owned());
        lines.join("\n")
    }
}

fn format_rate(bytes_per_second: f64) -> String {
    format!("{}/s", format_bytes(bytes_per_second.max(0.0) as u64))
}

fn optional_rate(available: bool, bytes_per_second: f64) -> String {
    if available {
        format_rate(bytes_per_second)
    } else {
        "N/A".to_owned()
    }
}

fn optional_bytes(available: bool, bytes: u64) -> String {
    if available {
        format_bytes(bytes)
    } else {
        "N/A".to_owned()
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = bytes as f64;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", value / GB)
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", value / MB)
    } else if bytes >= 1024 {
        format!("{:.1} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn shorten(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_owned()
    } else {
        let mut shortened: String = value.chars().take(max_chars.saturating_sub(1)).collect();
        shortened.push('…');
        shortened
    }
}
