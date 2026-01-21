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
    pub fn allow_selection(&self) -> bool {
        matches!(self, Tool::Moving)
    }
}

impl Tool {
    pub fn update(
        &mut self,
        ctx: &Context,
        surface_clicked: bool,
        global_trans: TSTransform,
        objects: &mut State,
        selection: &mut SelectionState,
        rects: &[(Uuid, Rect)],
    ) -> bool {
        self.visualize(ctx, global_trans, objects);
        self.interact(
            ctx,
            surface_clicked,
            global_trans,
            objects,
            selection,
            rects,
        )
    }

    fn visualize(&mut self, ctx: &Context, global_trans: TSTransform, objects: &mut State) {
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
                    let mut trans = global_trans * *trans;
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
                    let tpos = global_trans.inverse().mul_pos(*position);
                    let mut trans = TSTransform {
                        translation: tpos.to_vec2(),
                        scaling: 1. / global_trans.scaling,
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

    // TODO reduce the args here?
    /// Return `true` when a change was made
    fn interact(
        &mut self,
        ctx: &Context,
        surface_clicked: bool,
        global_trans: TSTransform,
        objects: &mut State,
        selection: &mut SelectionState,
        rects: &[(Uuid, Rect)],
    ) -> bool {
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
                let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());

                if surface_clicked && let Some(pos) = interact_pos {
                    let tpos = global_trans.inverse().mul_pos(pos);
                    let existing = rects.iter().find_map(|(i, r)| {
                        (r.contains(pos)
                            && objects
                                .get(i)
                                .map(|obj| matches!(obj.data, ObjectKind::Text { .. }))
                                .unwrap_or(false))
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
                            scaling: 1. / global_trans.scaling,
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
                    if inp.pointer.primary_pressed()
                        && let Some(pos) = inp.pointer.press_origin()
                    {
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
                        let ids: Vec<_> = rects
                            .iter()
                            .filter(|(_, rect)| r.contains_rect(*rect))
                            .map(|(i, _)| *i)
                            .collect();

                        let shift_pressed = inp.modifiers.shift_only();
                        if shift_pressed {
                            selection.append(&ids);
                        } else {
                            selection.replace(&ids);
                        }
                        *rect = None;
                    }
                });
            }
            Tool::Placing {
                position,
                file_dialog,
            } => {
                if surface_clicked {
                    file_dialog.pick_multiple();
                    let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
                    *position = interact_pos;
                }
            }
        }
        changed
    }
}

pub fn toolbar(ctx: &Context, tool: &mut Tool) {
    egui::Area::new(egui::Id::new("tools"))
        .fixed_pos(egui::pos2(32.0, 32.0))
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
