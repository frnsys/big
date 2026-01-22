use bpaf::Bpaf;
use std::path::PathBuf;

use big::App;

/// infinite space
#[derive(Bpaf, Clone)]
#[bpaf(options, version)]
struct Cmd {
    #[bpaf(positional)]
    path: PathBuf,
}

fn main() -> eframe::Result {
    let cmd = cmd().run();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 590.0]),
        ..Default::default()
    };
    eframe::run_native(
        "canvas",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc, cmd.path)))),
    )
}
