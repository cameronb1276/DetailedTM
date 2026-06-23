#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod ui;

#[cfg(test)]
mod tests;

use anyhow::Context;
use app::DetailedTmApp;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .ok();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1400.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "DetailedTM",
        options,
        Box::new(|creation_context| Ok(Box::new(DetailedTmApp::new(creation_context)))),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))
    .context("DetailedTM native window failed")
}
