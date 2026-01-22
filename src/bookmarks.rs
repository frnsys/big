use egui::{FontFamily, FontId, TextEdit, emath::TSTransform};
use egui_phosphor::regular as icons;

pub struct Bookmark {
    label: String,
    transform: TSTransform,
}

pub struct BookmarksPanel {
    open: bool,
}
impl Default for BookmarksPanel {
    fn default() -> Self {
        Self { open: true }
    }
}
impl BookmarksPanel {
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        bookmarks: &mut Vec<Bookmark>,
        current_transform: &mut TSTransform,
    ) {
        ui.style_mut().override_font_id = Some(FontId::new(11., FontFamily::Proportional));
        justified(
            ui,
            |ui| {
                ui.label("Bookmarks");
            },
            |ui| {
                let resp = ui.button(if self.open {
                    icons::EYE
                } else {
                    icons::EYE_CLOSED
                });
                if resp.clicked() {
                    self.open = !self.open;
                }

                if ui.button(("+", icons::BOOKMARK_SIMPLE)).clicked() {
                    bookmarks.push(Bookmark {
                        label: "New bookmark".into(),
                        transform: current_transform.clone(),
                    });
                }
            },
        );

        if !self.open {
            return;
        }

        if !bookmarks.is_empty() {
            ui.add_space(4.);
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
                    bookmark.transform = current_transform.clone();
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
}

fn justified(
    ui: &mut egui::Ui,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        left(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            right(ui);
        });
    });
}
