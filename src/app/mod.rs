pub mod state;

use eframe::egui;
use state::AppState;

pub struct DetailedTmApp {
    state: AppState,
}

impl DetailedTmApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::new(creation_context.egui_ctx.clone()),
        }
    }
}

impl eframe::App for DetailedTmApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.poll_snapshots();

        egui::TopBottomPanel::top("controls").show(context, |ui| {
            crate::ui::controls::show(ui, &mut self.state);
        });
        egui::TopBottomPanel::bottom("status").show(context, |ui| {
            crate::ui::controls::show_status(ui, &self.state);
        });
        egui::CentralPanel::default().show(context, |ui| {
            crate::ui::table::show(ui, &mut self.state);
        });

        crate::ui::controls::show_confirmation(context, &mut self.state);
        context.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
