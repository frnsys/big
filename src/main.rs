use std::collections::BTreeMap;

use egui::{
    Align, Align2, Color32, FontId, Id, Key, LayerId, Order, Painter, PointerButton, Pos2, Rangef,
    Rect, Response, Sense, Stroke, StrokeKind, TextBuffer, UiBuilder, Vec2, ahash::HashSet,
    emath::TSTransform,
};
pub use egui_phosphor::regular as icons;
use uuid::Uuid;

enum DragMode {
    Panning,
    Moving,
    Resizing,
    Scaling,
}

#[derive(Default)]
struct StateDirtyTracker {
    is_dirty: bool,
}
impl StateDirtyTracker {
    fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }
}

type State = BTreeMap<Uuid, Object>;

struct Stack {
    snapshots: Vec<State>,
    position: usize,
}
impl Stack {
    pub fn new(snapshot: State) -> Self {
        Self {
            snapshots: vec![snapshot],
            position: 0,
        }
    }

    pub fn push(&mut self, snapshot: State) {
        let _ = self.snapshots.split_off(self.position + 1);
        self.snapshots.push(snapshot);
        self.position = self.snapshots.len() - 1;
    }

    pub fn undo(&mut self) -> Option<&State> {
        if self.snapshots.is_empty() {
            None
        } else {
            self.position = self.position.saturating_sub(1);
            Some(&self.snapshots[self.position])
        }
    }

    pub fn redo(&mut self) -> Option<&State> {
        if self.snapshots.is_empty() {
            None
        } else {
            self.position = (self.position + 1).min(self.snapshots.len() - 1);
            Some(&self.snapshots[self.position])
        }
    }

    pub fn current(&self) -> &State {
        &self.snapshots[self.position]
    }

    pub fn can_undo(&self) -> bool {
        self.position > 0
    }

    pub fn can_redo(&self) -> bool {
        self.position < self.snapshots.len() - 1
    }
}

struct Notification {
    expires_at: f64,
    message: String,
}

struct App {
    transform: TSTransform,
    objects: BTreeMap<Uuid, Object>,
    selected: Selection,
    tool: Tool,
    drag_mode: DragMode,
    state_dirty: StateDirtyTracker,
    stack: Stack,
    notifications: Vec<Notification>,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

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
            drag_mode: DragMode::Panning,
            transform: TSTransform::default(),
            selected: Selection::default(),
            state_dirty: StateDirtyTracker::default(),
            stack: Stack::new(objects.clone()),
            objects,
            notifications: vec![],
        }
    }
}

enum Tool {
    Moving,
    Typing {
        transform: Option<TSTransform>,
        string: String,
        width: f32,
        id: Option<Uuid>,
    },
    BoxSelect(Option<(Pos2, Pos2)>),
    Placing,
    Bookmark,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Object {
    transform: TSTransform,
    data: ObjectKind,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
enum ObjectKind {
    // Image(PathBuf), // TODO
    Rect {
        size: Vec2,
        color: Color32,
    },
    Text {
        text: String,
        width: f32,
        color: Color32,
    },
}
impl ObjectKind {
    fn paint(&self, painter: &Painter) -> Rect {
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
        }
    }

    fn order(&self) -> Order {
        match self {
            ObjectKind::Rect { .. } => Order::Middle,
            ObjectKind::Text { .. } => Order::Foreground,
        }
    }
}

#[derive(Default)]
struct Selection {
    ids: HashSet<Uuid>,
}
impl Selection {
    fn replace(&mut self, ids: &[Uuid]) {
        self.ids.clear();
        self.ids.extend(ids);
    }

    fn append(&mut self, ids: &[Uuid]) {
        self.ids.extend(ids);
    }

    fn remove(&mut self, id: Uuid) {
        self.ids.remove(&id);
    }

    fn clear(&mut self) {
        self.ids.clear();
    }

    fn contains(&self, id: Uuid) -> bool {
        self.ids.contains(&id)
    }

    fn toggle(&mut self, id: Uuid, append: bool) {
        if self.ids.contains(&id) {
            if !append {
                self.ids.clear();
            }
            self.ids.remove(&id);
        } else {
            if !append {
                self.replace(&[id]);
            } else {
                self.append(&[id]);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let clicked = ctx.input(|inp| inp.pointer.primary_clicked());
        let interact_pos = if clicked {
            ctx.input(|inp| inp.pointer.interact_pos())
        } else {
            None
        };

        let allow_select = !matches!(self.tool, Tool::Typing { .. });
        let mut rects = Vec::with_capacity(self.objects.len());
        let shift_pressed = ctx.input(|inp| inp.modifiers.shift_only());
        let mut selection_rect: Option<Rect> = None;
        let mut clicked = vec![];

        let skip_id = if let Tool::Typing { id, .. } = &self.tool {
            *id
        } else {
            None
        };

        for (i, obj) in self.objects.iter_mut() {
            let skip = skip_id.as_ref().is_some_and(|id| id == i);

            let mut trans = obj.transform;
            trans.scaling *= self.transform.scaling;
            trans.translation =
                (obj.transform.translation * self.transform.scaling) + self.transform.translation;

            let rect = if !skip {
                let layer = LayerId::new(obj.data.order(), Id::new(i));
                ctx.set_transform_layer(layer, trans);
                let painter = ctx.layer_painter(layer);
                obj.data.paint(&painter)
            } else {
                Rect::ZERO
            };

            if let Some(pos) = interact_pos
                && rect.contains(trans.inverse().mul_pos(pos))
            {
                if allow_select {
                    if !shift_pressed {
                        selection_rect = None;
                    }
                    self.selected.toggle(*i, shift_pressed);
                }
                clicked.push(obj);
            }

            let rect =
                Rect::from_min_size(trans.translation.to_pos2(), rect.size() * trans.scaling);

            if self.selected.contains(*i) {
                selection_rect = match selection_rect {
                    Some(base) => Some(base.union(rect)),
                    None => Some(rect),
                };
            }

            rects.push((*i, rect));
        }

        if matches!(self.drag_mode, DragMode::Panning) {
            let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
            let pointer_down = ctx.input(|inp| inp.pointer.primary_pressed());
            let selection_box_clicked = interact_pos
                .zip(selection_rect)
                .map(|(pos, rect)| rect.contains(pos) && pointer_down)
                .unwrap_or(false);
            if selection_box_clicked {
                self.drag_mode = DragMode::Moving;
            }
        } else {
            let pointer_up = ctx.input(|inp| inp.pointer.primary_released());
            if pointer_up {
                self.drag_mode = DragMode::Panning;
                self.state_dirty.mark_dirty();
            }
        }

        if let Some(rect) = selection_rect {
            let layer = LayerId::new(Order::Foreground, Id::new("selection"));
            let painter = ctx.layer_painter(layer);
            painter.rect_stroke(rect, 0., Stroke::new(2., Color32::RED), StrokeKind::Outside);

            if self.selected.ids.len() == 1
                && let Some(id) = self.selected.ids.iter().next()
                && self
                    .objects
                    .get(id)
                    .map(|obj| matches!(obj.data, ObjectKind::Text { .. }))
                    .unwrap_or(false)
            {
                // Text handle
                let width = 8.;
                let height = 16.;
                let x = rect.right();
                let y = rect.center().y - height / 2.;
                let handle = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
                painter.rect_filled(handle, 0., Color32::RED);

                if matches!(self.drag_mode, DragMode::Panning) {
                    let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
                    let pointer_down = ctx.input(|inp| inp.pointer.primary_pressed());
                    let handle_clicked = interact_pos
                        .map(|pos| handle.contains(pos) && pointer_down)
                        .unwrap_or(false);
                    if handle_clicked {
                        self.drag_mode = DragMode::Resizing;
                    }
                }
            }

            // Scale handle
            let width = 8.;
            let height = 8.;
            let x = rect.right();
            let y = rect.bottom();
            let handle = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
            painter.rect_filled(handle, 0., Color32::RED);

            if matches!(self.drag_mode, DragMode::Panning) {
                let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
                let pointer_down = ctx.input(|inp| inp.pointer.primary_pressed());
                let handle_clicked = interact_pos
                    .map(|pos| handle.contains(pos) && pointer_down)
                    .unwrap_or(false);
                if handle_clicked {
                    self.drag_mode = DragMode::Scaling;
                }
            }
        }

        // For canvas interaction (zooming & panning).
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut resp = create_surface(ui);
            let allow_drag = !matches!(self.tool, Tool::BoxSelect(_))
                && matches!(self.drag_mode, DragMode::Panning);
            update_transform(ui, &mut self.transform, &mut resp, allow_drag);

            // Note for drags we only mark the state dirty after releasing the pointer
            let dragged = resp.dragged_by(PointerButton::Primary);
            if dragged {
                match self.drag_mode {
                    DragMode::Moving => {
                        for i in self.selected.ids.iter() {
                            if let Some(obj) = self.objects.get_mut(i) {
                                obj.transform.translation += resp.drag_delta();
                            }
                        }
                    }
                    DragMode::Resizing => {
                        for i in self.selected.ids.iter() {
                            if let Some(obj) = self.objects.get_mut(i) {
                                match &mut obj.data {
                                    ObjectKind::Rect { size, color } => {}
                                    ObjectKind::Text { text, width, color } => {
                                        *width += resp.drag_delta().x;
                                    }
                                }
                            }
                        }
                    }
                    DragMode::Scaling => {
                        let pos = ui.input(|inp| inp.pointer.hover_pos());
                        if let Some(rect) = selection_rect
                            && let Some(pos) = pos
                        {
                            let br = rect.right_bottom();
                            let ratio = pos.x / br.x;
                            for i in self.selected.ids.iter() {
                                if let Some(obj) = self.objects.get_mut(i) {
                                    obj.transform.scaling *= ratio;
                                }
                            }
                        }
                    }
                    DragMode::Panning => {}
                }
            }
        });

        // TODO tool visual feedback
        match &mut self.tool {
            Tool::Moving => (),
            Tool::Typing {
                transform,
                string,
                width,
                ..
            } => {
                if let Some(trans) = transform {
                    let mut trans = *trans;
                    trans.scaling *= self.transform.scaling;
                    trans.translation =
                        (trans.translation * self.transform.scaling) + self.transform.translation;
                    trans.translation -= Vec2::new(1., 1.); // Offset to account for textedit border

                    let area = egui::Area::new(egui::Id::new("text-input"))
                        .order(Order::Foreground)
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
                                    egui::TextEdit::multiline(string)
                                        .desired_width(*width)
                                        .margin(0.)
                                        .frame(false)
                                        .font(FONT)
                                        .background_color(Color32::TRANSPARENT),
                                );
                                resp.request_focus();
                            });
                    });
                }
            }
            Tool::BoxSelect(rect) => {
                if let Some((start, end)) = rect {
                    let layer = LayerId::new(Order::Foreground, Id::new("selection-box"));
                    let painter = ctx.layer_painter(layer);
                    painter.rect_stroke(
                        Rect::from_two_pos(*start, *end),
                        0.,
                        Stroke::new(2., Color32::RED),
                        StrokeKind::Outside,
                    );
                }
            }
            Tool::Placing => todo!(),
            Tool::Bookmark => todo!(),
        }

        // tool interaction
        match &mut self.tool {
            Tool::Moving => (),
            Tool::Typing {
                transform,
                string,
                width,
                id,
            } => {
                let clicked = ctx.input(|inp| inp.pointer.primary_clicked());
                let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());

                if clicked && let Some(pos) = interact_pos {
                    let tpos = self.transform.inverse().mul_pos(pos);
                    let existing = rects.iter().find_map(|(i, r)| {
                        (r.contains(pos)
                            && self
                                .objects
                                .get(i)
                                .map(|obj| matches!(obj.data, ObjectKind::Text { .. }))
                                .unwrap_or(false))
                        .then_some(i)
                    });
                    if let Some(i) = existing
                        && let Some(obj) = self.objects.get(i)
                    {
                        *id = Some(*i);
                        if let ObjectKind::Text {
                            text,
                            width: w,
                            color,
                        } = &obj.data
                        {
                            *transform = Some(obj.transform);
                            string.clear();
                            string.push_str(&text);
                            *width = *w;
                        }
                    } else {
                        *transform = Some(TSTransform {
                            translation: tpos.to_vec2(),
                            scaling: 1. / self.transform.scaling,
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
                            && let Some(obj) = self.objects.get_mut(i)
                        {
                            if let ObjectKind::Text { text, .. } = &mut obj.data {
                                *text = string.take();
                            }
                        } else {
                            if !string.trim().is_empty() {
                                self.objects.insert(
                                    Uuid::new_v4(),
                                    Object {
                                        transform: trans,
                                        data: ObjectKind::Text {
                                            text: string.take(),
                                            width: *width,
                                            color: Color32::LIGHT_BLUE,
                                        },
                                    },
                                );
                            }
                        }
                        *transform = None;
                        string.clear();
                        *id = None;
                        self.state_dirty.mark_dirty();
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
                            .filter(|(_, rect)| rect.intersects(r))
                            .map(|(i, _)| *i)
                            .collect();
                        if shift_pressed {
                            self.selected.append(&ids);
                        } else {
                            self.selected.replace(&ids);
                        }
                        *rect = None;
                    }
                });
            }
            Tool::Placing => todo!(),
            Tool::Bookmark => todo!(),
        }

        if !ctx.memory(|mem| mem.focused().is_some()) {
            if ctx.input(|inp| inp.key_released(Key::X)) {
                for id in &self.selected.ids {
                    self.objects.remove(id);
                }
                self.selected.clear();
                self.state_dirty.mark_dirty();
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

            if ctx.input(|inp| inp.key_released(Key::S) && inp.modifiers.ctrl) {
                let duration = 3.0; // seconds
                let expires_at = ctx.input(|i| i.time) + duration;
                let ser = serde_yaml::to_string(&self.objects).unwrap();
                self.notifications.push(Notification {
                    expires_at,
                    message: "Saved".into(),
                });
                std::fs::write("/tmp/plan.yaml", ser).expect("Unable to write file");
            }
        }

        egui::Area::new(egui::Id::new("notifications"))
            .order(Order::Tooltip)
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-8., -8.))
            .show(ctx, |ui| {
                ui.set_width(240.);
                let now = ui.input(|i| i.time);
                self.notifications.retain(|n| now < n.expires_at);
                for n in &self.notifications {
                    ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                        ui.label(&n.message);
                    });
                }
                if !self.notifications.is_empty() {
                    ui.ctx().request_repaint();
                }
            });

        egui::Area::new(egui::Id::new("tools"))
            .fixed_pos(egui::pos2(32.0, 32.0))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                select_button(
                    ui,
                    icons::CURSOR,
                    &mut self.tool,
                    |mode| matches!(mode, Tool::Moving),
                    || Tool::Moving,
                )
                .on_hover_text("Move");
                select_button(
                    ui,
                    icons::IMAGES,
                    &mut self.tool,
                    |mode| matches!(mode, Tool::Placing),
                    || Tool::Placing,
                )
                .on_hover_text("Place Images");
                select_button(
                    ui,
                    icons::SELECTION,
                    &mut self.tool,
                    |mode| matches!(mode, Tool::BoxSelect(_)),
                    || Tool::BoxSelect(None),
                )
                .on_hover_text("Selection");
                select_button(
                    ui,
                    icons::CURSOR_TEXT,
                    &mut self.tool,
                    |mode| matches!(mode, Tool::Typing { .. }),
                    || Tool::Typing {
                        transform: None,
                        string: String::new(),
                        width: 180.,
                        id: None,
                    },
                )
                .on_hover_text("Insert Text");
                select_button(
                    ui,
                    icons::BOOKMARK_SIMPLE,
                    &mut self.tool,
                    |mode| matches!(mode, Tool::Bookmark),
                    || Tool::Bookmark,
                )
                .on_hover_text("Add Bookmark");
            });

        if self.state_dirty.is_dirty {
            if *self.stack.current() != self.objects {
                self.stack.push(self.objects.clone());
            }
            self.state_dirty.is_dirty = false;
        }
    }
}

fn select_button<T>(
    ui: &mut egui::Ui,
    text: &str,
    val: &mut T,
    pred: impl FnOnce(&T) -> bool,
    default: impl FnOnce() -> T,
) -> Response {
    let resp = ui.selectable_label(pred(val), text);
    if resp.clicked() {
        *val = default();
    }
    resp
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
