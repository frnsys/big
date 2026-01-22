use plan::App;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 590.0]),
        ..Default::default()
    };
    eframe::run_native("canvas", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
