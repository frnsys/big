use egui::{TextEdit, emath::TSTransform};
use egui_phosphor::regular as icons;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bookmark {
    label: String,
    transform: TSTransform,
}

pub fn render_bookmarks(
    ui: &mut egui::Ui,
    bookmarks: &mut Vec<Bookmark>,
    current_transform: &mut TSTransform,
) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(("+", icons::BOOKMARK_SIMPLE)).clicked() {
                bookmarks.push(Bookmark {
                    label: "New bookmark".into(),
                    transform: *current_transform,
                });
            }
        });
    });

    if !bookmarks.is_empty() {
        ui.add_space(2.);
    }

    let mut to_delete = None;
    for (i, bookmark) in bookmarks.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if ui
                .button(icons::CROSSHAIR)
                .on_hover_text("Go to view")
                .clicked()
            {
                *current_transform = bookmark.transform;
            }

            ui.add(TextEdit::singleline(&mut bookmark.label).desired_width(90.));

            if ui
                .button(icons::CORNERS_OUT)
                .on_hover_text("Set view")
                .clicked()
            {
                bookmark.transform = *current_transform;
            }

            if ui.button(icons::X).clicked() {
                to_delete = Some(i);
            }
        });
    }
    if let Some(i) = to_delete {
        bookmarks.remove(i);
    }
}
