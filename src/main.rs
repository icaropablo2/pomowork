use eframe::egui;

mod app;
mod audio;
mod persistence;
mod state;
mod timer;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        centered: true,
        viewport: egui::ViewportBuilder::default()
            .with_title("Pomowork")
            .with_inner_size(ui::window_inner_size())
            .with_min_inner_size([340.0, 460.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Pomowork",
        options,
        Box::new(|_| Ok(Box::new(app::PomoworkApp::new(persistence::load_state())))),
    )
}
