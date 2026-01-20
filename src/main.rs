use egui::{
    Color32, FontId, Id, LayerId, Order, Painter, PointerButton, Pos2, Rangef, Rect, Response,
    Sense, Stroke, StrokeKind, UiBuilder, Vec2, ahash::HashSet, emath::TSTransform,
};

struct App {
    transform: TSTransform,
    objects: Vec<Object>,
    selected: HashSet<usize>,
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            transform: TSTransform::default(),
            selected: HashSet::default(),
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
    Moving,
    Typing,
    Selecting,
    Drawing,
    Placing,
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
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // For canvas interaction (zooming & panning).
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut resp = create_surface(ui);
            update_transform(ui, &mut self.transform, &mut resp);
        });

        let clicked = ctx.input(|inp| inp.pointer.primary_clicked());
        let interact_pos = if clicked {
            ctx.input(|inp| inp.pointer.interact_pos())
        } else {
            None
        };
        let shift_pressed = ctx.input(|inp| inp.modifiers.shift_only());
        let mut selection_rect: Option<Rect> = None;
        for (i, obj) in self.objects.iter_mut().enumerate() {
            let mut trans = obj.transform;
            trans.scaling *= self.transform.scaling;
            trans.translation =
                (obj.transform.translation * self.transform.scaling) + self.transform.translation;

            let layer = LayerId::new(Order::Middle, Id::new(i));
            ctx.set_transform_layer(layer, trans);
            let painter = ctx.layer_painter(layer);
            let rect = obj.data.paint(&painter);

            if let Some(pos) = interact_pos
                && rect.contains(trans.inverse().mul_pos(pos))
            {
                if self.selected.contains(&i) {
                    if !shift_pressed {
                        self.selected.clear();
                        selection_rect = None;
                    }
                    self.selected.remove(&i);
                } else {
                    if !shift_pressed {
                        self.selected.clear();
                        selection_rect = None;
                    }
                    self.selected.insert(i);
                }
            }

            if self.selected.contains(&i) {
                let rect =
                    Rect::from_min_size(trans.translation.to_pos2(), rect.size() * trans.scaling);
                selection_rect = match selection_rect {
                    Some(base) => Some(base.union(rect)),
                    None => Some(rect),
                };
            }
        }

        if let Some(rect) = selection_rect {
            let layer = LayerId::new(Order::Foreground, Id::new("selection"));
            let painter = ctx.layer_painter(layer);
            painter.rect_stroke(rect, 0., Stroke::new(2., Color32::RED), StrokeKind::Outside);
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
