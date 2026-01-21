use egui::{Color32, FontFamily, FontId, emath::TSTransform};
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

                let resp = ui.button(("+", icons::BOOKMARK_SIMPLE));
                if resp.clicked() {
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
                let resp = ui.button(icons::CROSSHAIR).on_hover_text("Go to view");
                if resp.clicked() {
                    *current_transform = bookmark.transform;
                }

                let edit = egui::TextEdit::singleline(&mut bookmark.label).desired_width(90.);
                ui.add(edit);

                let resp = ui.button(icons::CORNERS_OUT).on_hover_text("Set view");
                if resp.clicked() {
                    bookmark.transform = current_transform.clone();
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
