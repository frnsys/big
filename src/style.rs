use egui::{Color32, Context};

pub fn apply_styles(ctx: &Context) {
    replace_fonts(ctx);
    set_visuals(ctx);
}

fn replace_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();
    let font_name = "default";

    fonts.font_data.insert(
        font_name.to_owned(),
        // std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
        //     "../assets/DMMono/DMMono-Regular.ttf"
        // ))),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/Inter/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "phosphor".into(),
        egui_phosphor::Variant::Regular.font_data().into(),
    );

    fonts.families.insert(
        egui::FontFamily::Proportional,
        vec![font_name.into(), "phosphor".into()],
    );

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
