pub mod state;

use eframe::egui;
use state::AppState;

pub struct DetailedTmApp {
    state: AppState,
}

impl DetailedTmApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::new(),
        }
    }
}

impl eframe::App for DetailedTmApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.refresh_if_due();

        egui::CentralPanel::default().show(context, |ui| {
            crate::ui::controls::show(ui, &mut self.state);
            crate::ui::search::show(ui, &mut self.state.search);
            crate::ui::table::show(ui, &self.state.visible_rows());
        });

        context.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
