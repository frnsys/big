use egui::{LayerId, PointerButton, Rangef, Response, Sense, UiBuilder, emath::TSTransform};

pub fn update_canvas(ui: &mut egui::Ui, transform: &mut TSTransform) -> Response {
    let mut resp = create_surface(ui);
    update_transform(ui, transform, &mut resp);
    resp
}

/// Create a surface for canvas interactions (panning).
fn create_surface(ui: &mut egui::Ui) -> Response {
    let scene_layer_id = LayerId::new(ui.layer_id().order, ui.id().with("scene_area"));
    ui.ctx().set_sublayer(ui.layer_id(), scene_layer_id);

    let mut local_ui = ui.new_child(
        UiBuilder::new()
            .layer_id(scene_layer_id)
            .sense(Sense::click_and_drag()),
    );
    local_ui.set_width(local_ui.available_width());
    local_ui.set_height(local_ui.available_height());

    local_ui.response()
}

/// Update the global transform & handle zooming.
fn update_transform(ui: &mut egui::Ui, transform: &mut TSTransform, drag_resp: &mut Response) {
    let dragged = drag_resp.dragged_by(PointerButton::Primary);
    if dragged {
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
