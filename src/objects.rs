use std::path::PathBuf;

use egui::{Color32, FontId, Painter, Pos2, Rect, Vec2, emath::TSTransform};

use crate::images::TextureCache;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub transform: TSTransform,
    pub data: ObjectKind,
}
impl Object {
    pub fn is_resizable(&self) -> bool {
        matches!(self.data, ObjectKind::Text { .. })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ObjectKind {
    Rect {
        size: Vec2,
        color: Color32,
    },
    Text {
        text: String,
        width: f32,
        color: Color32,
    },
    Image {
        source: PathBuf,
        hash: u128,
        size: u64,
    },
}
impl ObjectKind {
    pub fn paint(&self, painter: &mut Painter) -> Rect {
        match self {
            ObjectKind::Rect { size, color } => {
                let rect = Rect::from_min_size(Pos2::ZERO, *size);
                painter.rect_filled(rect, 0., *color);
                rect
            }
            ObjectKind::Text { text, width, color } => {
                const FONT: FontId = FontId::proportional(12.);
                // PERF: Could probably cache this and only update when width or text changes.
                let galley = painter.layout(text.to_string(), FONT, *color, *width);
                painter.galley(Pos2::ZERO, galley.clone(), *color);
                galley.rect
            }
            ObjectKind::Image { source, .. } => {
                if let Some(info) = TextureCache::get(&source) {
                    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                    let rect = Rect::from_min_size(Pos2::ZERO, info.size);
                    painter.set_clip_rect(rect);
                    painter.image(info.id, rect, uv, Color32::WHITE);
                    rect
                } else {
                    Rect::ZERO
                }
            }
        }
    }
}
