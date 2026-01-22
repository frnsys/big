use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use egui::{Align, Align2, Area, Context, Id, Layout, Order, Vec2, mutex::Mutex};

static MESSAGES: LazyLock<Mutex<Vec<Notification>>> = LazyLock::new(|| Mutex::new(vec![]));

#[derive(Default)]
pub struct Notifications;
impl Notifications {
    pub fn push(message: String) {
        let expires_at = Instant::now() + Duration::from_secs(3);

        let mut messages = MESSAGES.lock();
        messages.push(Notification {
            expires_at,
            message,
        });
    }

    fn prune(&mut self) {
        let now = Instant::now();
        let mut messages = MESSAGES.lock();
        messages.retain(|n| now < n.expires_at);
    }

    pub fn show(&mut self, ctx: &Context, (align, offset): (Align2, Vec2)) {
        self.prune();

        let messages = MESSAGES.lock();
        Area::new(Id::new("notifications"))
            .order(Order::Tooltip)
            .anchor(align, offset)
            .show(ctx, |ui| {
                ui.set_width(240.);
                for n in messages.iter() {
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        ui.label(&n.message);
                    });
                }
            });

        if !messages.is_empty() {
            ctx.request_repaint();
        }
    }
}

struct Notification {
    message: String,
    expires_at: Instant,
}
