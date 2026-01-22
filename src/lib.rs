mod bookmarks;
mod canvas;
mod images;
mod notifs;
mod objects;
mod select;
mod stack;
mod style;
mod tools;

use std::path::{Path, PathBuf};

use egui::{Align2, Color32, Context, Id, Key, LayerId, Order, Rect, Vec2, emath::TSTransform};
use uuid::Uuid;

use crate::{
    bookmarks::{Bookmark, BookmarksPanel},
    images::TextureCache,
    notifs::Notifications,
    objects::ObjectKind,
    select::{SelectionContext, SelectionState},
    stack::{Stack, State},
    tools::{Tool, ToolContext, toolbar},
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct SaveData {
    objects: State,
    bookmarks: Vec<Bookmark>,
    transform: TSTransform,
}

pub struct App {
    path: PathBuf,
    root: PathBuf,
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
    pub fn new(cc: &eframe::CreationContext<'_>, path: PathBuf) -> Self {
        style::apply_styles(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let SaveData {
            mut objects,
            bookmarks,
            transform,
        } = if path.exists() {
            load(&path)
        } else {
            SaveData::default()
        };

        // Get the provided file path as an absolute path,
        // so we know the parent directory (i.e. the project directory).
        let cwd = std::env::current_dir().unwrap();
        let path = cwd.join(path).canonicalize().unwrap();
        let root = path.parent().expect("has a parent").to_path_buf();

        for (_, obj) in &objects {
            if let ObjectKind::Image { source, .. } = &obj.data {
                TextureCache::request_load(source.as_path(), &cc.egui_ctx);
            }
        }

        Self {
            path,
            root,
            tool: Tool::Moving,
            selection: SelectionState::default(),
            stack: Stack::new(objects.clone()),
            transform,
            objects,
            bookmarks,
            bookmarks_panel: BookmarksPanel::default(),
            notifications: Notifications::default(),
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

        if ctx.input(|inp| inp.key_released(Key::Q) && inp.modifiers.ctrl && inp.modifiers.shift) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

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
            self.save();
        }

        changed
    }

    fn save(&mut self) {
        let data = SaveData {
            objects: self.objects.clone(),
            bookmarks: self.bookmarks.clone(),
            transform: self.transform,
        };
        match serde_yaml::to_string(&data) {
            Ok(ser) => match std::fs::write(&self.path, ser) {
                Ok(_) => Notifications::push("Saved".into()),
                Err(err) => {
                    Notifications::push(format!("Error writing file: {err:?}"));
                }
            },
            Err(err) => {
                Notifications::push(format!("Error serializing data: {err:?}"));
            }
        }
    }
}

fn load(path: &Path) -> SaveData {
    let data = std::fs::read_to_string(path).expect(&format!("Unable to read file: {path:?}"));
    serde_yaml::from_str(&data).unwrap()
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
            let allow_drag = self.tool.allow_dragging() && !self.selection.is_dragging();
            let resp = canvas::update_canvas(ui, &mut self.transform, allow_drag);

            let surface_clicked = resp.clicked();
            let allow_select = self.tool.allow_selection();
            let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
            let pointer_down = resp.is_pointer_button_down_on();

            let sel_ctx = SelectionContext {
                parent_transform: self.transform,
                drag_delta: ctx.input(|inp| inp.pointer.delta()) / self.transform.scaling,
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
                root: &self.root,
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
            &self.root,
            (Align2::LEFT_TOP, Vec2::new(24.0, 24.0)),
        );

        if is_dirty {
            if *self.stack.current() != self.objects {
                self.stack.push(self.objects.clone());
            }
        }
    }
}
