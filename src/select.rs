use egui::{
    Color32, Context, Id, LayerId, Order, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
    ahash::HashSet, emath::TSTransform,
};
use uuid::Uuid;

use crate::stack::State;

#[derive(Debug, Clone, Copy)]
enum DragMode {
    Moving,
    Resizing,
    Scaling,
}

const SELECTION_BOX_COLOR: Color32 = Color32::from_rgb(0xa2, 0x94, 0xff);
const SELECTION_HANDLE_COLOR: Color32 = Color32::from_rgb(0xeb, 0x40, 0x34);

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

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    /// Returns `Some` only if these is exactly one item selected.
    pub fn single(&self) -> Option<&Uuid> {
        if self.ids.len() == 1 {
            self.ids.iter().next()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Uuid> {
        self.ids.iter()
    }

    pub fn contains(&self, id: Uuid) -> bool {
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

pub struct SelectionContext<'a> {
    pub parent_transform: TSTransform,
    pub drag_delta: Vec2,
    pub clicked_pos: Option<Pos2>,
    pub pressed_pos: Option<Pos2>,
    pub hover_pos: Option<Pos2>,
    pub pointer_up: bool,
    pub append_selection: bool,
    pub rects: &'a [(Uuid, Rect)],
}

enum DragState {
    Idle,
    Dragging,
    Done,
}

impl SelectionState {
    fn id() -> Id {
        Id::new("selection-interaction")
    }

    pub fn is_dragging(&self) -> bool {
        self.drag_mode.is_some()
    }

    /// Return `true` if stopped dragging
    pub fn update(&mut self, ctx: &Context, sctx: SelectionContext, objects: &mut State) -> bool {
        let mut nothing_clicked = true;
        if let Some(pos) = sctx.clicked_pos {
            for (id, rect) in sctx.rects {
                if rect.contains(pos) {
                    self.selection.toggle(*id, sctx.append_selection);
                    nothing_clicked = false;

                    let layer = LayerId::new(Order::Background, Id::new(id));
                    ctx.move_to_top(layer);
                }
            }
        }

        if sctx.clicked_pos.is_some() && nothing_clicked {
            self.selection.clear();
        }

        let mut selection_rect: Option<Rect> = None;
        for (id, rect) in sctx.rects {
            if self.selection.contains(*id) {
                selection_rect = match selection_rect {
                    Some(base) => Some(base.union(*rect)),
                    None => Some(*rect),
                };
            }
        }

        if let Some(rect) = selection_rect {
            let layer = LayerId::new(Order::Background, Id::new("selection"));
            let painter = ctx.layer_painter(layer);
            self.render_selection_box(&painter, rect, objects, sctx.pressed_pos);

            let dragging = self.handle_drag(&sctx, rect, objects);
            match dragging {
                DragState::Idle => false,
                DragState::Dragging => {
                    // Claim dragging lock
                    ctx.set_dragged_id(Self::id());
                    false
                }
                DragState::Done => {
                    // Release dragging lock
                    ctx.stop_dragging();
                    true
                }
            }
        } else {
            false
        }
    }

    fn render_selection_box(
        &mut self,
        painter: &Painter,
        rect: Rect,
        objects: &State,
        pressed_pos: Option<Pos2>,
    ) {
        painter.rect_stroke(
            rect,
            0.,
            Stroke::new(1., SELECTION_BOX_COLOR),
            StrokeKind::Outside,
        );

        // Only show resize handle if one item is selected,
        // and it's resizable.
        if let Some(id) = self.selection.single()
            && objects.get(id).is_some_and(|obj| obj.is_resizable())
        {
            let pressed = render_resize_handle(painter, rect, pressed_pos);
            if pressed && self.drag_mode.is_none() {
                self.drag_mode = Some(DragMode::Resizing);
            }
        }

        let pressed = render_scale_handle(painter, rect, pressed_pos);
        if pressed && self.drag_mode.is_none() {
            self.drag_mode = Some(DragMode::Scaling);
        }
    }

    fn handle_drag(
        &mut self,
        sctx: &SelectionContext,
        rect: Rect,
        objects: &mut State,
    ) -> DragState {
        let mut dragging = DragState::Idle;

        if self.drag_mode.is_none() {
            let selection_box_clicked = sctx.pressed_pos.is_some_and(|pos| rect.contains(pos));
            if selection_box_clicked {
                self.drag_mode = Some(DragMode::Moving);
                dragging = DragState::Dragging;
            }
        }

        if let Some(mode) = self.drag_mode {
            dragging = DragState::Dragging;
            match mode {
                DragMode::Moving => {
                    for i in self.selection.ids.iter() {
                        if let Some(obj) = objects.get_mut(i) {
                            obj.transform.translation += sctx.drag_delta;
                        }
                    }
                }
                DragMode::Resizing => {
                    for i in self.selection.ids.iter() {
                        if let Some(obj) = objects.get_mut(i)
                            && let Some(width) = obj.width_mut()
                        {
                            *width += sctx.drag_delta.x;
                        }
                    }
                }
                DragMode::Scaling => {
                    if let Some(pos) = sctx.hover_pos {
                        let tl = rect.left_top();
                        let br = rect.right_bottom();
                        let ratio = pos.x / br.x;
                        for i in self.selection.ids.iter() {
                            if let Some(obj) = objects.get_mut(i) {
                                obj.transform.scaling *= ratio;

                                // Adjust position so that relative positions to selection
                                // pivot are maintained.
                                let world_pos =
                                    sctx.parent_transform * obj.transform.translation.to_pos2();
                                let world_pos_ = tl + (world_pos - tl) * ratio;
                                obj.transform.translation = sctx
                                    .parent_transform
                                    .inverse()
                                    .mul_pos(world_pos_)
                                    .to_vec2();
                            }
                        }
                    }
                }
            }
        }

        if self.drag_mode.is_some() && sctx.pointer_up {
            self.drag_mode = None;
            DragState::Done
        } else {
            dragging
        }
    }
}

fn render_handle(
    painter: &Painter,
    (x, y): (f32, f32),
    (width, height): (f32, f32),
    pressed_pos: Option<Pos2>,
) -> bool {
    let handle = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height));
    painter.rect_filled(handle, 0., SELECTION_HANDLE_COLOR);
    pressed_pos.is_some_and(|pos| handle.contains(pos))
}

fn render_resize_handle(
    painter: &Painter,
    selection_rect: Rect,
    pressed_pos: Option<Pos2>,
) -> bool {
    let width = 8.;
    let height = 16.;
    let x = selection_rect.right();
    let y = selection_rect.center().y - height / 2.;
    render_handle(painter, (x, y), (width, height), pressed_pos)
}

fn render_scale_handle(painter: &Painter, selection_rect: Rect, pressed_pos: Option<Pos2>) -> bool {
    let width = 8.;
    let height = 8.;
    let x = selection_rect.right();
    let y = selection_rect.bottom();
    render_handle(painter, (x, y), (width, height), pressed_pos)
}
