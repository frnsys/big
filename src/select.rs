use egui::{
    Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
    ahash::HashSet, emath::TSTransform,
};
use uuid::Uuid;

use crate::{objects::ObjectKind, stack::State};

#[derive(Clone, Copy)]
enum DragMode {
    Moving,
    Resizing,
    Scaling,
}

#[derive(Default)]
pub struct Selection {
    ids: HashSet<Uuid>,
}
impl Selection {
    pub fn replace(&mut self, ids: &[Uuid]) {
        self.ids.clear();
        self.ids.extend(ids);
    }

    pub fn append(&mut self, ids: &[Uuid]) {
        self.ids.extend(ids);
    }

    fn remove(&mut self, id: Uuid) {
        self.ids.remove(&id);
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &Uuid> {
        self.ids.iter()
    }

    pub fn contains(&self, id: Uuid) -> bool {
        self.ids.contains(&id)
    }

    pub fn toggle(&mut self, id: Uuid, append: bool) {
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

#[derive(Default)]
pub struct SelectionState {
    selection: Selection,
    drag_mode: Option<DragMode>,
}
impl std::ops::Deref for SelectionState {
    type Target = Selection;

    fn deref(&self) -> &Self::Target {
        &self.selection
    }
}
impl std::ops::DerefMut for SelectionState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.selection
    }
}
impl SelectionState {
    pub fn is_dragging(&self) -> bool {
        self.drag_mode.is_some()
    }

    /// Return `true` if stopped dragging
    pub fn update(
        &mut self,
        ctx: &Context,
        rect: Rect,
        drag_delta: Option<Vec2>,
        objects: &mut State,
        global_transform: TSTransform,
    ) -> bool {
        self.render_selection_box(ctx, rect, objects);
        self.handle_drag(ctx, rect, drag_delta, objects, global_transform)
    }

    fn render_selection_box(&mut self, ctx: &Context, rect: Rect, objects: &State) {
        let layer = LayerId::new(Order::Foreground, Id::new("selection"));
        let painter = ctx.layer_painter(layer);
        painter.rect_stroke(rect, 0., Stroke::new(2., Color32::RED), StrokeKind::Outside);

        // TODO this could be cleaned up
        if self.selection.ids.len() == 1
            && let Some(id) = self.selection.ids.iter().next()
            && objects
                .get(id)
                .map(|obj| matches!(obj.data, ObjectKind::Text { .. }))
                .unwrap_or(false)
        {
            let clicked = render_resize_handle(ctx, &painter, rect);
            if clicked && self.drag_mode.is_none() {
                self.drag_mode = Some(DragMode::Resizing);
            }
        }

        let clicked = render_scale_handle(ctx, &painter, rect);
        if clicked && self.drag_mode.is_none() {
            self.drag_mode = Some(DragMode::Scaling);
        }
    }

    /// Return `true` if stopped dragging
    fn handle_drag(
        &mut self,
        ctx: &Context,
        rect: Rect,
        drag_delta: Option<Vec2>,
        objects: &mut State,
        global_transform: TSTransform,
    ) -> bool {
        if self.drag_mode.is_none() {
            let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
            let pointer_down = ctx.input(|inp| inp.pointer.primary_pressed());
            let selection_box_clicked = interact_pos
                .map(|pos| rect.contains(pos) && pointer_down)
                .unwrap_or(false);
            if selection_box_clicked {
                self.drag_mode = Some(DragMode::Moving);
            }
        }

        if let Some(delta) = drag_delta
            && let Some(mode) = self.drag_mode
        {
            match mode {
                DragMode::Moving => {
                    for i in self.selection.ids.iter() {
                        if let Some(obj) = objects.get_mut(i) {
                            obj.transform.translation += delta / global_transform.scaling;
                        }
                    }
                }
                DragMode::Resizing => {
                    for i in self.selection.ids.iter() {
                        if let Some(obj) = objects.get_mut(i) {
                            match &mut obj.data {
                                ObjectKind::Text { text, width, color } => {
                                    *width += delta.x;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                DragMode::Scaling => {
                    let pos = ctx.input(|inp| inp.pointer.hover_pos());
                    if let Some(pos) = pos {
                        let br = rect.right_bottom();
                        let ratio = pos.x / br.x;
                        for i in self.selection.ids.iter() {
                            if let Some(obj) = objects.get_mut(i) {
                                obj.transform.scaling *= ratio;
                            }
                        }
                    }
                }
            }
        }

        let pointer_up = ctx.input(|inp| inp.pointer.primary_released());
        if self.drag_mode.is_some() && pointer_up {
            self.drag_mode = None;
            true
        } else {
            false
        }
    }
}

fn render_handle(
    ctx: &Context,
    painter: &Painter,
    (x, y): (f32, f32),
    (width, height): (f32, f32),
) -> bool {
    let handle = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
    painter.rect_filled(handle, 0., Color32::RED);

    let interact_pos = ctx.input(|inp| inp.pointer.interact_pos());
    let pointer_down = ctx.input(|inp| inp.pointer.primary_pressed());
    interact_pos
        .map(|pos| handle.contains(pos) && pointer_down)
        .unwrap_or(false)
}

fn render_resize_handle(ctx: &Context, painter: &Painter, selection_rect: Rect) -> bool {
    let width = 8.;
    let height = 16.;
    let x = selection_rect.right();
    let y = selection_rect.center().y - height / 2.;
    render_handle(ctx, painter, (x, y), (width, height))
}

fn render_scale_handle(ctx: &Context, painter: &Painter, selection_rect: Rect) -> bool {
    let width = 8.;
    let height = 8.;
    let x = selection_rect.right();
    let y = selection_rect.bottom();
    render_handle(ctx, painter, (x, y), (width, height))
}
