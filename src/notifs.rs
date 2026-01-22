use egui::{Align, Align2, Area, Context, Id, Layout, Order, Vec2};

pub struct Notifications {
    context: egui::Context,
    messages: Vec<Notification>,
}
impl Notifications {
    pub fn new(ctx: &Context) -> Self {
        Self {
            context: ctx.clone(),
            messages: Vec::default(),
        }
    }

    pub fn push(&mut self, message: String) {
        let duration = 3.0; // seconds
        let expires_at = self.context.input(|i| i.time) + duration;
        self.messages.push(Notification {
            expires_at,
            message,
        });
    }

    fn prune(&mut self) {
        let now = self.context.input(|i| i.time);
        self.messages.retain(|n| now < n.expires_at);
    }

    pub fn show(&mut self, ctx: &Context, (align, offset): (Align2, Vec2)) {
        self.prune();

        Area::new(Id::new("notifications"))
            .order(Order::Tooltip)
            .anchor(align, offset)
            .show(ctx, |ui| {
                ui.set_width(240.);
                for n in &self.messages {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.label(&n.message);
                    });
                }
            });

        if !self.messages.is_empty() {
            ctx.request_repaint();
        }
    }
}

struct Notification {
    expires_at: f64,
    message: String,
}
