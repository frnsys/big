use std::path::PathBuf;

use egui::{Color32, FontId, Painter, Pos2, Rect, Vec2, emath::TSTransform};

use crate::{
    content::{FileInfo, calculate_hash, get_file_size},
    images::{ImageRequest, TextureCache},
};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Object {
    pub transform: TSTransform,
    pub data: ObjectKind,
    pub size: Vec2,
}
impl Object {
    pub fn is_resizable(&self) -> bool {
        matches!(self.data, ObjectKind::Text { .. })
    }

    pub fn is_text(&self) -> bool {
        matches!(self.data, ObjectKind::Text { .. })
    }

    pub fn width_mut(&mut self) -> Option<&mut f32> {
        match &mut self.data {
            ObjectKind::Text { width, .. } => Some(width),
            _ => None,
        }
    }

    pub fn file_info(&mut self) -> Option<FileInfo<'_>> {
        if let ObjectKind::Image {
            source, size, hash, ..
        } = &mut self.data
        {
            Some(FileInfo {
                path: source,
                hash: *hash,
                size: *size,
            })
        } else {
            None
        }
    }

    pub fn image(path: PathBuf, trans: TSTransform) -> std::io::Result<Self> {
        let size = get_file_size(&path)?;
        let hash = calculate_hash(&path)?;
        let dims = imagesize::size(&path).map_err(|err| std::io::Error::other(err))?;
        let dims = Vec2::new(dims.width as f32, dims.height as f32);
        Ok(Self {
            transform: trans,
            size: dims,
            data: ObjectKind::Image {
                source: path,
                hash,
                size,
                dims,
            },
        })
    }

    pub fn text(text: String, color: Color32, width: f32, trans: TSTransform) -> Self {
        Self {
            transform: trans,
            size: Vec2::INFINITY, // We won't know the size until the text is laid out
            data: ObjectKind::Text { text, width, color },
        }
    }

    pub fn rect(size: Vec2, color: Color32, trans: TSTransform) -> Self {
        Self {
            transform: trans,
            size,
            data: ObjectKind::Rect { size, color },
        }
    }

    pub fn paint(&mut self, painter: &mut Painter) -> Rect {
        let rect = self.data.paint(painter);
        if rect != Rect::ZERO {
            self.size = rect.size();
        }
        rect
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
        dims: Vec2,
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
            ObjectKind::Image {
                source, hash, dims, ..
            } => {
                // Get the scaling for this painter
                // so we know the image's target size.
                let layer = painter.layer_id();
                let scale = painter
                    .ctx()
                    .layer_transform_to_global(layer)
                    .unwrap()
                    .scaling;

                let image = ImageRequest {
                    source: source.as_path(),
                    source_size: *dims,
                    target_size: *dims * scale,
                    image_hash: *hash,
                };

                if let Some(info) = TextureCache::get(&image) {
                    let uv = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                    let rect = Rect::from_min_size(Pos2::ZERO, *dims);
                    painter.set_clip_rect(rect);
                    painter.image(info.id, rect, uv, Color32::WHITE);
                    rect
                } else {
                    TextureCache::request_load(image);
                    Rect::ZERO
                }
            }
        }
    }
}
