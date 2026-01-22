use std::sync::Arc;

use egui::{
    Align2, Color32, Context, FontId, Id, LayerId, Order, Pos2, Rect, Stroke, StrokeKind,
    TextBuffer, Vec2, emath::TSTransform,
};
use egui_file_dialog::FileDialog;
use egui_phosphor::regular as icons;
use uuid::Uuid;

use crate::{
    images::TextureCache,
    objects::{Object, ObjectKind},
    select::SelectionState,
    stack::State,
};

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
        file_dialog: FileDialog,
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
    pub clicked_pos: Option<Pos2>,
    pub parent_transform: TSTransform,
    pub rects: &'a [(Uuid, Rect)],
    pub selection: &'a mut SelectionState,
}

impl Tool {
    pub fn update(&mut self, ctx: &Context, mut tctx: ToolContext, objects: &mut State) -> bool {
        self.visualize(ctx, tctx.parent_transform, objects);
        self.interact(ctx, &mut tctx, objects)
    }

    fn visualize(&mut self, ctx: &Context, parent_trans: TSTransform, objects: &mut State) {
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
                        Stroke::new(2., Color32::RED),
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
                        trans.translation.y += i as f32 * 5.;
                        TextureCache::request_load(&path);
                        objects.insert(
                            Uuid::new_v4(),
                            Object {
                                transform: trans,
                                data: ObjectKind::Image { source: path },
                            },
                        );
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
                        (r.contains(pos)
                            && objects
                                .get(i)
                                .is_some_and(|obj| matches!(obj.data, ObjectKind::Text { .. })))
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
                            string.push_str(&text);
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
                        inp.events.iter().any(|ev| match ev {
                            egui::Event::Key {
                                key: egui::Key::Escape,
                                pressed: true,
                                ..
                            } => true,
                            _ => false,
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
                                    Object {
                                        transform: trans,
                                        data: ObjectKind::Text {
                                            text: string.take(),
                                            width: *width,
                                            color: *color,
                                        },
                                    },
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
                ctx.input(|inp| {
                    if inp.pointer.is_decidedly_dragging() {
                        if let Some(pos) = inp.pointer.press_origin() {
                            *rect = Some((pos, pos))
                        }

                        if let Some((_, end)) = rect {
                            if inp.pointer.primary_down()
                                && let Some(pos) = inp.pointer.latest_pos()
                            {
                                *end = pos;
                            }
                        }

                        if inp.pointer.primary_released()
                            && let Some((start, end)) = rect
                        {
                            let r = Rect::from_two_pos(*start, *end);
                            let ids: Vec<_> = tctx
                                .rects
                                .iter()
                                .filter(|(_, rect)| r.contains_rect(*rect))
                                .map(|(i, _)| *i)
                                .collect();

                            let shift_pressed = inp.modifiers.shift_only();
                            if shift_pressed {
                                tctx.selection.append(&ids);
                            } else {
                                tctx.selection.replace(&ids);
                            }
                            *rect = None;
                        }
                    }
                });
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

pub fn toolbar(ctx: &Context, tool: &mut Tool, (align, offset): (Align2, Vec2)) {
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
                    file_dialog: image_file_dialog(),
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

fn image_file_dialog() -> FileDialog {
    FileDialog::new()
        .add_file_filter(
            "Images",
            Arc::new(|p| {
                let ext = p.extension().unwrap_or_default();
                ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "webp"
            }),
        )
        .show_devices(false)
        .title_bar(false)
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
