use egui::{
    Color32, FontId, Id, LayerId, Order, Painter, PointerButton, Pos2, Rangef, Rect, Response,
    Sense, UiBuilder, Vec2, emath::TSTransform,
};

struct App {
    transform: TSTransform,
    objects: Vec<Object>,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            transform: TSTransform::default(),
            objects: vec![
                Object {
                    transform: TSTransform::default(),
                    data: ObjectKind::Rect {
                        size: Vec2::new(100., 100.),
                        color: Color32::YELLOW,
                    },
                },
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
                Object {
                    transform: TSTransform {
                        scaling: 1.0,
                        translation: Vec2::new(100., 100.),
                    },
                    data: ObjectKind::Text {
                        text: "Hello world".into(),
                        width: 120.,
                        color: Color32::BLACK,
                    },
                },
            ],
        }
    }
}

enum InteractMode {
    Panning,
    Zooming,
    Typing,
    Selecting,
    Drawing,
}

// Operating on a selection
enum EditMode {
    Moving,
    Scaling,
}

struct Object {
    transform: TSTransform,
    data: ObjectKind,
}

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
    fn paint(&self, painter: &Painter, pos: Pos2) {
        match self {
            ObjectKind::Rect { size, color } => {
                painter.rect_filled(Rect::from_min_size(pos, *size), 0., *color);
            }
            ObjectKind::Text { text, width, color } => {
                const FONT: FontId = FontId::proportional(12.);
                // PERF: Could probably cache this and only update when width or text changes.
                let galley = painter.layout(text.to_string(), FONT, *color, *width);
                painter.galley(pos, galley, *color);
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // For canvas interaction (zooming & panning).
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut resp = create_surface(ui);
            update_transform(ui, &mut self.transform, &mut resp);
        });

        for (i, obj) in self.objects.iter().enumerate() {
            let mut trans = obj.transform;
            trans.scaling *= self.transform.scaling;
            trans.translation =
                (obj.transform.translation * self.transform.scaling) + self.transform.translation;

            let layer = LayerId::new(Order::Middle, Id::new(i));
            ctx.set_transform_layer(layer, trans);
            let painter = ctx.layer_painter(layer);
            obj.data.paint(&painter, Pos2::ZERO);
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
            let zoom_range = Rangef::new(f32::EPSILON, 2.0);
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
