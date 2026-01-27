use egui::{
    Align2, Color32, Context, FontFamily, FontId, Margin, Order, Rect, Vec2, emath::TSTransform,
};
use egui_phosphor::regular as icons;

use crate::bookmarks::{Bookmark, render_bookmarks};

#[derive(PartialEq)]
enum Tab {
    Bookmarks,
    Labels,
}

pub struct Inspector {
    tab: Tab,
    open: bool,
    query: String,
}
impl Default for Inspector {
    fn default() -> Self {
        Self {
            tab: Tab::Bookmarks,
            open: true,
            query: String::new(),
        }
    }
}
impl Inspector {
    pub fn render(
        &mut self,
        ctx: &Context,
        labels: &[(&str, Rect)],
        bookmarks: &mut Vec<Bookmark>,
        current_transform: &mut TSTransform,
        (align, offset): (Align2, Vec2),
    ) {
        egui::Area::new(egui::Id::new("inspector"))
            .order(Order::Middle)
            .anchor(align, offset)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_black_alpha(128))
                    .corner_radius(4.)
                    .inner_margin(6.)
                    .show(ui, |ui| {
                        ui.set_width(180.);
                        ui.style_mut().override_font_id =
                            Some(FontId::new(11., FontFamily::Proportional));
                        self.render_inner(ui, labels, bookmarks, current_transform);
                    });
            });
    }

    fn render_inner(
        &mut self,
        ui: &mut egui::Ui,
        labels: &[(&str, Rect)],
        bookmarks: &mut Vec<Bookmark>,
        current_transform: &mut TSTransform,
    ) {
        justified(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::Bookmarks, "Bookmarks");
                    ui.selectable_value(&mut self.tab, Tab::Labels, "Labels");
                });
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
            },
        );

        if !self.open {
            return;
        }

        ui.add_space(2.);
        match self.tab {
            Tab::Bookmarks => {
                render_bookmarks(ui, bookmarks, current_transform);
            }
            Tab::Labels => {
                self.render_labels(ui, labels, current_transform);
            }
        }
    }

    fn render_labels(
        &mut self,
        ui: &mut egui::Ui,
        labels: &[(&str, Rect)],
        current_transform: &mut TSTransform,
    ) {
        ui.text_edit_singleline(&mut self.query);

        for (label, rect) in labels
            .iter()
            .filter(|(label, _)| label.to_lowercase().contains(&self.query))
        {
            ui.horizontal(|ui| {
                if ui
                    .button(icons::CROSSHAIR)
                    .on_hover_text("Go to view")
                    .clicked()
                {
                    let zoom = 1.;
                    let screen_rect = ui.ctx().content_rect();
                    let screen_center = screen_rect.center().to_vec2();
                    let world_center = rect.center().to_vec2();
                    *current_transform = TSTransform {
                        scaling: zoom,
                        translation: screen_center - (world_center * zoom),
                    }
                }

                egui::Frame::NONE
                    .fill(Color32::from_gray(0x0A))
                    .inner_margin(Margin {
                        left: 5,
                        right: 5,
                        top: 2,
                        bottom: 3,
                    })
                    .corner_radius(2)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.colored_label(Color32::from_gray(224), *label);
                    });
            });
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
