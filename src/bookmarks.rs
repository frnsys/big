use egui::emath::TSTransform;
use egui_phosphor::regular as icons;

pub struct Bookmark {
    label: String,
    transform: TSTransform,
}

pub fn render_bookmarks_panel(
    ui: &mut egui::Ui,
    bookmarks: &mut Vec<Bookmark>,
    current_transform: &mut TSTransform,
) {
    ui.label("Bookmarks");

    let resp = ui.button((icons::BOOKMARK_SIMPLE, "Add Bookmark"));
    if resp.clicked() {
        bookmarks.push(Bookmark {
            label: "new bookmark".into(),
            transform: current_transform.clone(),
        });
    }

    let mut to_delete = None;
    for (i, bookmark) in bookmarks.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let resp = ui.button(icons::FRAME_CORNERS);
            let edit = egui::TextEdit::singleline(&mut bookmark.label).desired_width(120.);
            ui.add(edit);
            if resp.clicked() {
                bookmark.transform = current_transform.clone();
            }

            let resp = ui.button(icons::CROSSHAIR);
            if resp.clicked() {
                *current_transform = bookmark.transform;
            }

            let resp = ui.button(icons::X);
            if resp.clicked() {
                to_delete = Some(i);
            }
        });
    }
    if let Some(i) = to_delete {
        bookmarks.remove(i);
    }
}
