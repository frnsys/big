use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Pos2, Rect, Stroke, StrokeKind,
    TextBuffer, Vec2, emath::TSTransform,
};
use egui_file_dialog::FileDialog;
use egui_phosphor::regular as icons;
use pathdiff::diff_paths;
use uuid::Uuid;

use crate::{
    notifs::Notifications,
    objects::{Object, ObjectKind},
    select::SelectionState,
    stack::State,
};

const SELECTION_BOX_COLOR: Color32 = Color32::from_rgb(0x39, 0xB8, 0x6D);

pub enum Tool {
    Moving,
    Typing {
        transform: Option<TSTransform>,
        string: String,
        width: f32,
        color: Color32,
        id: Option<Uuid>,
    },
    BoxSelect(Option<(Pos2, Pos2)>),
    Placing {
        position: Option<Pos2>,
        file_dialog: Box<FileDialog>,
    },
}

impl Tool {
    /// Can others modify the selection while this tool is active?
    pub fn allow_selection(&self) -> bool {
        matches!(self, Tool::Moving | Tool::BoxSelect(None))
    }

    /// Can others use drags while this tool is active?
    pub fn allow_dragging(&self) -> bool {
        !matches!(self, Tool::BoxSelect(_))
    }
}

pub struct ToolContext<'a> {
    pub root: &'a Path,
    pub clicked_pos: Option<Pos2>,
    pub parent_transform: TSTransform,
    pub rects: &'a [(Uuid, Rect)],
    pub selection: &'a mut SelectionState,
}

impl Tool {
    pub fn update(&mut self, ctx: &Context, mut tctx: ToolContext, objects: &mut State) -> bool {
        self.visualize(ctx, tctx.root, tctx.parent_transform, objects);
        self.interact(ctx, &mut tctx, objects)
    }

    fn visualize(
        &mut self,
        ctx: &Context,
        root: &Path,
        parent_trans: TSTransform,
        objects: &mut State,
    ) {
        match self {
            Tool::Moving => (),
            Tool::Typing {
                transform,
                string,
                width,
                color,
                ..
            } => {
                if let Some(trans) = transform {
                    let mut trans = parent_trans * *trans;
                    trans.translation -= Vec2::new(1., 1.); // Offset to account for textedit border
                    floating_text_input(ctx, trans, string, *width, *color);
                }
            }
            Tool::BoxSelect(rect) => {
                if let Some((start, end)) = rect {
                    let layer = LayerId::new(Order::Background, Id::new("selection-box"));
                    let painter = ctx.layer_painter(layer);
                    painter.rect_stroke(
                        Rect::from_two_pos(*start, *end),
                        0.,
                        Stroke::new(1., SELECTION_BOX_COLOR),
                        StrokeKind::Outside,
                    );
                }
            }
            Tool::Placing {
                file_dialog,
                position,
            } => {
                file_dialog.update(ctx);
                if let Some(position) = position
                    && let Some(paths) = file_dialog.take_picked_multiple()
                {
                    let tpos = parent_trans.inverse().mul_pos(*position);
                    let mut trans = TSTransform {
                        translation: tpos.to_vec2(),
                        scaling: 1. / parent_trans.scaling,
                    };

                    for (i, path) in paths.into_iter().enumerate() {
                        trans.translation.x += i as f32 * 5.;
                        trans.translation.y += i as f32 * 5.;

                        // Keep paths relative to the project root.
                        if let Some(path) = diff_paths(path, root) {
                            match Object::image(path, trans) {
                                Ok(object) => {
                                    objects.insert(Uuid::new_v4(), object);
                                }
                                Err(err) => {
                                    Notifications::push(format!("Failed to create image: {err:?}"))
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Return `true` when a change was made
    fn interact(&mut self, ctx: &Context, tctx: &mut ToolContext, objects: &mut State) -> bool {
        let mut changed = false;
        match self {
            Tool::Moving => (),
            Tool::Typing {
                transform,
                string,
                width,
                color,
                id,
            } => {
                let ToolContext {
                    clicked_pos,
                    parent_transform,
                    rects,
                    ..
                } = tctx;

                if let Some(pos) = *clicked_pos {
                    let tpos = parent_transform.inverse().mul_pos(pos);
                    let existing = rects.iter().find_map(|(i, r)| {
                        (r.contains(pos) && objects.get(i).is_some_and(|obj| obj.is_text()))
                            .then_some(i)
                    });
                    if let Some(i) = existing
                        && let Some(obj) = objects.get(i)
                    {
                        *id = Some(*i);
                        if let ObjectKind::Text {
                            text,
                            width: w,
                            color: c,
                        } = &obj.data
                        {
                            *transform = Some(obj.transform);
                            string.clear();
                            string.push_str(text);
                            *width = *w;
                            *color = *c;
                        }
                    } else {
                        *transform = Some(TSTransform {
                            translation: tpos.to_vec2(),
                            scaling: 1. / parent_transform.scaling,
                        });
                    }
                }

                if let Some(trans) = *transform {
                    let escape = ctx.input(|inp| {
                        inp.events.iter().any(|ev| {
                            matches!(
                                ev,
                                egui::Event::Key {
                                    key: egui::Key::Escape,
                                    pressed: true,
                                    ..
                                }
                            )
                        })
                    });

                    if escape {
                        if let Some(i) = id
                            && let Some(obj) = objects.get_mut(i)
                        {
                            if let ObjectKind::Text { text, color: c, .. } = &mut obj.data {
                                *text = string.take();
                                *c = *color;
                            }
                        } else {
                            if !string.trim().is_empty() {
                                objects.insert(
                                    Uuid::new_v4(),
                                    Object::text(string.take(), *color, *width, trans),
                                );
                            }
                        }
                        *transform = None;
                        string.clear();
                        *id = None;
                        changed = true;
                    }
                }
            }
            Tool::BoxSelect(rect) => {
                let id = Id::new("box-select");
                let can_drag = !ctx.dragging_something_else(id);
                if can_drag {
                    let is_dragging = ctx.input(|inp| inp.pointer.is_decidedly_dragging());
                    if is_dragging {
                        let press_origin = ctx.input(|inp| inp.pointer.press_origin());
                        if let Some(pos) = press_origin {
                            *rect = Some((pos, pos))
                        }

                        let primary_down = ctx.input(|inp| inp.pointer.primary_down());
                        let latest_pos = ctx.input(|inp| inp.pointer.latest_pos());
                        if let Some((_, end)) = rect
                            && primary_down
                            && let Some(pos) = latest_pos
                        {
                            *end = pos;
                        }

                        let primary_released = ctx.input(|inp| inp.pointer.primary_released());
                        if primary_released && let Some((start, end)) = rect {
                            let r = Rect::from_two_pos(*start, *end);
                            let ids: Vec<_> = tctx
                                .rects
                                .iter()
                                .filter(|(_, rect)| r.contains_rect(*rect))
                                .map(|(i, _)| *i)
                                .collect();

                            let shift_pressed = ctx.input(|inp| inp.modifiers.shift_only());
                            if shift_pressed {
                                tctx.selection.append(&ids);
                            } else {
                                tctx.selection.replace(&ids);
                            }
                            *rect = None;
                        }
                    }
                }
            }
            Tool::Placing {
                position,
                file_dialog,
            } => {
                if let Some(pos) = tctx.clicked_pos {
                    file_dialog.pick_multiple();
                    *position = Some(pos);
                }
            }
        }
        changed
    }
}

pub fn toolbar(ctx: &Context, tool: &mut Tool, root: &Path, (align, offset): (Align2, Vec2)) {
    egui::Area::new(egui::Id::new("tools"))
        .anchor(align, offset)
        .order(Order::Middle)
        .show(ctx, |ui| {
            select_button(
                ui,
                icons::CURSOR,
                tool,
                |mode| matches!(mode, Tool::Moving),
                || Tool::Moving,
            )
            .on_hover_text("Move");
            select_button(
                ui,
                icons::IMAGES,
                tool,
                |mode| matches!(mode, Tool::Placing { .. }),
                || Tool::Placing {
                    position: None,
                    file_dialog: Box::new(image_file_dialog(root.to_path_buf())),
                },
            )
            .on_hover_text("Place Images");
            select_button(
                ui,
                icons::SELECTION,
                tool,
                |mode| matches!(mode, Tool::BoxSelect(_)),
                || Tool::BoxSelect(None),
            )
            .on_hover_text("Selection");
            ui.horizontal(|ui| {
                select_button(
                    ui,
                    icons::CURSOR_TEXT,
                    tool,
                    |mode| matches!(mode, Tool::Typing { .. }),
                    || Tool::Typing {
                        transform: None,
                        string: String::new(),
                        width: 180.,
                        color: Color32::BLACK,
                        id: None,
                    },
                )
                .on_hover_text("Insert Text");

                if let Tool::Typing { color, .. } = tool {
                    ui.color_edit_button_srgba(color);
                }
            });
        });
}

fn select_button<T>(
    ui: &mut egui::Ui,
    text: &str,
    val: &mut T,
    pred: impl FnOnce(&T) -> bool,
    default: impl FnOnce() -> T,
) -> egui::Response {
    let resp = ui.selectable_label(pred(val), text);
    if resp.clicked() {
        *val = default();
    }
    resp
}

fn image_file_dialog(root: PathBuf) -> FileDialog {
    FileDialog::new()
        .add_file_filter(
            "Images",
            Arc::new(|p| {
                let ext = p.extension().unwrap_or_default();
                ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp" || ext == "gif"
            }),
        )
        .show_left_panel(false)
        .show_devices(false)
        .title_bar(false)
        .initial_directory(root)
        .default_file_filter("Images")
}

fn floating_text_input(
    ctx: &Context,
    trans: TSTransform,
    text: &mut String,
    width: f32,
    color: Color32,
) {
    let area = egui::Area::new(egui::Id::new("text-input"))
        .order(Order::Middle)
        .anchor(Align2::LEFT_TOP, Vec2::ZERO);

    let layer = area.layer();
    ctx.set_transform_layer(layer, trans);
    area.show(ctx, |ui| {
        egui::Frame::NONE
            .stroke(Stroke::new(
                1.,
                Color32::from_rgba_premultiplied(0x22, 0x22, 0x22, 0xAA),
            ))
            .show(ui, |ui| {
                const FONT: FontId = FontId::proportional(12.);
                let resp = ui.add(
                    egui::TextEdit::multiline(text)
                        .desired_width(width)
                        .margin(0.)
                        .frame(false)
                        .font(FONT)
                        .text_color_opt(Some(color))
                        .background_color(Color32::TRANSPARENT),
                );
                resp.request_focus();
            });
    });
}
