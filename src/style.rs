use egui::{
    Color32, Context, FontFamily,
    epaint::text::{FontInsert, FontPriority, InsertFontFamily},
};

pub fn apply_styles(ctx: &Context) {
    replace_fonts(ctx);
    set_visuals(ctx);
}

fn replace_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_name = "default";

    ctx.add_font(FontInsert::new(
        "phosphor",
        egui_phosphor::Variant::Regular.font_data(),
        vec![InsertFontFamily {
            family: FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));

    ctx.add_font(FontInsert::new(
        font_name,
        // egui::FontData::from_static(include_bytes!(
        //     "../assets/DMMono/DMMono-Regular.ttf"
        // )),
        egui::FontData::from_static(include_bytes!("../assets/Inter/Inter-Regular.ttf")),
        vec![InsertFontFamily {
            family: FontFamily::Proportional,
            priority: FontPriority::Highest,
        }],
    ));

    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    ctx.set_fonts(fonts);
}

fn set_visuals(ctx: &Context) {
    ctx.style_mut(|style| {
        // Affects tooltips
        style.visuals.popup_shadow = egui::Shadow::NONE;
        style.visuals.menu_corner_radius = 1.0.into();
        style.visuals.window_stroke = egui::Stroke::NONE;
        style.visuals.window_fill = Color32::from_gray(24);

        // Button backgrounds
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_gray(32);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_gray(32);

        // Selectable button background
        style.visuals.selection.bg_fill = Color32::from_rgb(0x00, 0x4E, 0xBA);
    });
}
