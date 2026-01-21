use egui::Context;

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

    pub fn show(&mut self, ctx: &Context) {
        egui::Area::new(egui::Id::new("notifications"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-8., -8.))
            .show(ctx, |ui| {
                ui.set_width(240.);
                let now = ui.input(|i| i.time);
                self.messages.retain(|n| now < n.expires_at);
                for n in &self.messages {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.label(&n.message);
                    });
                }
                if !self.messages.is_empty() {
                    ui.ctx().request_repaint();
                }
            });
    }
}

struct Notification {
    expires_at: f64,
    message: String,
}
