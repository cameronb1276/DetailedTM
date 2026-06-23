use eframe::egui;

pub fn show(ui: &mut egui::Ui, search: &mut String) {
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.text_edit_singleline(search);
    });
}
