mod bookmarks;
mod images;
mod notifs;
mod objects;
mod select;
mod stack;
mod tools;

use egui::{
    Align2, Color32, Context, Id, Key, LayerId, Order, PointerButton, Rangef, Rect, Response,
    Sense, UiBuilder, Vec2, emath::TSTransform,
};
use uuid::Uuid;

use images::TextureCache;
use notifs::Notifications;
use stack::Stack;
use tools::{Tool, toolbar};

use crate::{
    bookmarks::{Bookmark, BookmarksPanel},
    objects::{Object, ObjectKind},
    select::{SelectionContext, SelectionState},
    stack::State,
    tools::ToolContext,
};

fn replace_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_name = "default";

    fonts.font_data.insert(
        font_name.to_owned(),
        // std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
        //     "../assets/DMMono/DMMono-Regular.ttf"
        // ))),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Inter/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "phosphor".into(),
        egui_phosphor::Variant::Regular.font_data().into(),
    );

    fonts.families.insert(
        egui::FontFamily::Proportional,
        vec![font_name.into(), "phosphor".into()],
    );

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    ctx.set_fonts(fonts);
}

struct App {
    tool: Tool,
    stack: Stack,
    transform: TSTransform,
    selection: SelectionState,
    bookmarks: Vec<Bookmark>,
    bookmarks_panel: BookmarksPanel,
    objects: State,
    notifications: Notifications,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        replace_fonts(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        cc.egui_ctx.style_mut(|style| {
            // Affects tooltips
            style.visuals.popup_shadow = egui::Shadow::NONE;
            style.visuals.menu_corner_radius = 1.0.into();
            style.visuals.window_stroke = egui::Stroke::NONE;
            style.visuals.window_fill = Color32::from_gray(24);

            // Button backgrounds
            style.visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(32);
            style.visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(32);

            // Selectable button background
            style.visuals.selection.bg_fill = Color32::from_rgb(0x00, 0x4E, 0xBA);
        });

        let objects: State = [
            (
                Uuid::new_v4(),
                Object {
                    transform: TSTransform::default(),
                    data: ObjectKind::Rect {
                        size: Vec2::new(100., 100.),
                        color: Color32::YELLOW,
                    },
                },
            ),
            (
                Uuid::new_v4(),
                Object {
                    transform: TSTransform {
                        scaling: 0.5,
                        translation: Vec2::new(100., 100.),
                    },
                    data: ObjectKind::Rect {
                        size: Vec2::new(100., 100.),
                        color: Color32::YELLOW,
                    },
                },
            ),
            (
                Uuid::new_v4(),
                Object {
                    transform: TSTransform {
                        scaling: 1.0,
                        translation: Vec2::new(100., 100.),
                    },
                    data: ObjectKind::Text {
                        text: "Hello world this is a long long long".into(),
                        width: 120.,
                        color: Color32::BLACK,
                    },
                },
            ),
        ]
        .into();

        Self {
            tool: Tool::Moving,
            transform: TSTransform::default(),
            selection: SelectionState::default(),
            stack: Stack::new(objects.clone()),
            objects,
            notifications: Notifications::new(&cc.egui_ctx),
            bookmarks: vec![],
            bookmarks_panel: BookmarksPanel::default(),
        }
    }

    fn render_objects(&mut self, ctx: &Context, skip: Option<Uuid>) -> Vec<(Uuid, Rect)> {
        let mut rects = Vec::with_capacity(self.objects.len());
        for (i, obj) in self.objects.iter_mut() {
            let dont_render = skip.as_ref().is_some_and(|id| id == i);

            let trans = self.transform * obj.transform;

            let rect = if !dont_render {
                let layer = LayerId::new(Order::Background, Id::new(i));
                ctx.set_transform_layer(layer, trans);
                let mut painter = ctx.layer_painter(layer);
                obj.data.paint(&mut painter)
            } else {
                Rect::ZERO
            };

            let rect =
                Rect::from_min_size(trans.translation.to_pos2(), rect.size() * trans.scaling);

            rects.push((*i, rect));
        }
        rects
    }

    fn handle_input(&mut self, ctx: &Context) -> bool {
        let mut changed = false;

        if ctx.input(|inp| inp.key_released(Key::X)) {
            for id in self.selection.iter() {
                self.objects.remove(id);
            }
            self.selection.clear();
            changed = true;
        }

        if ctx.input(|inp| inp.key_released(Key::Z)) {
            if let Some(state) = self.stack.undo() {
                self.objects = state.clone();
            }
        }

        if ctx.input(|inp| inp.key_released(Key::R)) {
            if let Some(state) = self.stack.redo() {
                self.objects = state.clone();
            }
        }

        if ctx.input(|inp| inp.key_released(Key::Escape)) {
            self.tool = Tool::Moving;
        }

        if ctx.input(|inp| inp.key_released(Key::S) && inp.modifiers.ctrl) {
            let ser = serde_yaml::to_string(&self.objects).unwrap();
            self.notifications.push("Saved".into());
            std::fs::write("/tmp/plan.yaml", ser).expect("Unable to write file");
        }

        changed
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut is_dirty = false;

        TextureCache::update(ctx);

        // Kind of hacky, but if we're editing a label we don't want
        // to both render the input and the text object at the same time.
        let skip_id = if let Tool::Typing { id, .. } = &self.tool {
            *id
        } else {
            None
        };

        let rects = self.render_objects(ctx, skip_id);

        // For canvas interaction (zooming & panning).
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut resp = create_surface(ui);
            let surface_clicked = resp.clicked();
            let pointer_down = resp.is_pointer_button_down_on();
            let allow_drag = self.tool.allow_dragging() && !self.selection.is_dragging();
            update_transform(ui, &mut self.transform, &mut resp, allow_drag);

            let dragged = resp.dragged_by(PointerButton::Primary);
            let delta = dragged.then_some(resp.drag_delta());
            let allow_select = self.tool.allow_selection();
            let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());

            let sel_ctx = SelectionContext {
                parent_transform: self.transform,
                drag_delta: delta.map(|delta| delta / self.transform.scaling),
                clicked_pos: (surface_clicked && allow_select)
                    .then_some(interact_pos)
                    .flatten(),
                pressed_pos: pointer_down.then_some(interact_pos).flatten(),
                hover_pos: ctx.input(|inp| inp.pointer.hover_pos()),
                pointer_up: ctx.input(|inp| inp.pointer.primary_released()),
                append_selection: ctx.input(|inp| inp.modifiers.shift_only()),
                rects: &rects,
            };
            is_dirty |= self.selection.update(ctx, sel_ctx, &mut self.objects);

            let tool_ctx = ToolContext {
                parent_transform: self.transform,
                clicked_pos: surface_clicked.then_some(interact_pos).flatten(),
                rects: &rects,
                selection: &mut self.selection,
            };
            is_dirty |= self.tool.update(ctx, tool_ctx, &mut self.objects);
        });

        if !ctx.memory(|mem| mem.focused().is_some()) {
            self.handle_input(ctx);
        }

        self.notifications
            .show(ctx, (Align2::RIGHT_BOTTOM, Vec2::new(-8., -8.)));

        egui::Area::new(egui::Id::new("inspector"))
            .order(Order::Middle)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-24., 24.))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_black_alpha(128))
                    .corner_radius(4.)
                    .inner_margin(6.)
                    .show(ui, |ui| {
                        ui.set_width(180.);
                        self.bookmarks_panel
                            .render(ui, &mut self.bookmarks, &mut self.transform);
                    });
            });

        toolbar(
            ctx,
            &mut self.tool,
            (Align2::LEFT_TOP, Vec2::new(24.0, 24.0)),
        );

        if is_dirty {
            if *self.stack.current() != self.objects {
                self.stack.push(self.objects.clone());
            }
        }
    }
}

fn create_surface(ui: &mut egui::Ui) -> Response {
    let scene_layer_id = LayerId::new(ui.layer_id().order, ui.id().with("scene_area"));
    ui.ctx().set_sublayer(ui.layer_id(), scene_layer_id);
    let sense = Sense::click_and_drag();

    let mut local_ui = ui.new_child(UiBuilder::new().layer_id(scene_layer_id).sense(sense));
    local_ui.set_width(local_ui.available_width());
    local_ui.set_height(local_ui.available_height());

    local_ui.response()
}

fn update_transform(
    ui: &mut egui::Ui,
    transform: &mut TSTransform,
    drag_resp: &mut Response,
    allow_drag: bool,
) {
    let dragged = drag_resp.dragged_by(PointerButton::Primary);
    if allow_drag && dragged {
        transform.translation += drag_resp.drag_delta();
        drag_resp.mark_changed();
    }
    if let Some(mouse_pos) = ui.input(|i| i.pointer.latest_pos())
        && drag_resp.contains_pointer()
    {
        let pointer_in_scene = transform.inverse() * mouse_pos;

        let zoom_delta = ui.input(|i| i.zoom_delta());
        let delta = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => Some(1. + delta.y / 5.),
                _ => None,
            })
        });
        let zoom_delta = delta.unwrap_or(zoom_delta);
        if zoom_delta != 1.0 {
            let zoom_range = Rangef::new(f32::EPSILON, 100.0);
            let zoom_delta = zoom_delta.clamp(
                zoom_range.min / transform.scaling,
                zoom_range.max / transform.scaling,
            );

            *transform = *transform
                * TSTransform::from_translation(pointer_in_scene.to_vec2())
                * TSTransform::from_scaling(zoom_delta)
                * TSTransform::from_translation(-pointer_in_scene.to_vec2());

            transform.scaling = zoom_range.clamp(transform.scaling);
            drag_resp.mark_changed();
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 590.0]),
        ..Default::default()
    };
    eframe::run_native("canvas", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
