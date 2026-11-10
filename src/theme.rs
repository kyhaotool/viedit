//! Bảng màu và kiểu dáng giao diện của VieEdit.

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, TextStyle};

pub const BG: Color32 = Color32::from_rgb(15, 17, 22);
pub const PANEL: Color32 = Color32::from_rgb(21, 25, 34);
pub const PANEL_2: Color32 = Color32::from_rgb(27, 32, 48);
pub const PANEL_3: Color32 = Color32::from_rgb(35, 41, 60);
pub const BORDER: Color32 = Color32::from_rgb(38, 45, 61);
pub const BORDER_HI: Color32 = Color32::from_rgb(58, 68, 92);
pub const TEXT: Color32 = Color32::from_rgb(230, 235, 245);
pub const DIM: Color32 = Color32::from_rgb(139, 150, 171);
pub const FAINT: Color32 = Color32::from_rgb(96, 106, 126);

pub const ACCENT: Color32 = Color32::from_rgb(255, 92, 92);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(150, 50, 50);
pub const GOLD: Color32 = Color32::from_rgb(255, 184, 77);
pub const CYAN: Color32 = Color32::from_rgb(56, 189, 248);
pub const GREEN: Color32 = Color32::from_rgb(52, 211, 153);

pub const TRACK_VIDEO: Color32 = Color32::from_rgb(56, 120, 200);
pub const TRACK_OVERLAY: Color32 = Color32::from_rgb(170, 90, 200);
pub const TRACK_AUDIO: Color32 = Color32::from_rgb(40, 160, 130);
pub const TRACK_CAPTION: Color32 = Color32::from_rgb(214, 158, 46);

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "be-regular".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/BeVietnamPro-Regular.ttf")),
    );
    fonts.font_data.insert(
        "be-bold".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/BeVietnamPro-Bold.ttf")),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "be-regular".to_owned());

    fonts
        .families
        .entry(FontFamily::Name("bold".into()))
        .or_default()
        .push("be-bold".to_owned());
    fonts
        .families
        .entry(FontFamily::Name("bold".into()))
        .or_default()
        .push("be-regular".to_owned());

    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);

    let mut style = (*ctx.style()).clone();

    style.visuals = egui::Visuals::dark();
    let v = &mut style.visuals;
    v.override_text_color = Some(TEXT);
    v.window_fill = PANEL;
    v.panel_fill = PANEL;
    v.faint_bg_color = PANEL_2;
    v.extreme_bg_color = BG;
    v.window_stroke = egui::Stroke::new(1.0_f32, BORDER);
    v.selection.bg_fill = ACCENT.linear_multiply(0.35);
    v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = CYAN;

    v.widgets.noninteractive.bg_fill = PANEL_2;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, BORDER);
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);

    v.widgets.inactive.bg_fill = PANEL_3;
    v.widgets.inactive.weak_bg_fill = PANEL_3;
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, BORDER);
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
    v.widgets.inactive.rounding = egui::Rounding::same(7.0);

    v.widgets.hovered.bg_fill = BORDER_HI;
    v.widgets.hovered.weak_bg_fill = BORDER_HI;
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, BORDER_HI);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.hovered.rounding = egui::Rounding::same(7.0);

    v.widgets.active.bg_fill = ACCENT_DIM;
    v.widgets.active.weak_bg_fill = ACCENT_DIM;
    v.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, Color32::WHITE);
    v.widgets.active.rounding = egui::Rounding::same(7.0);

    v.widgets.open.bg_fill = PANEL_3;
    v.widgets.open.weak_bg_fill = PANEL_3;
    v.widgets.open.bg_stroke = egui::Stroke::new(1.0_f32, BORDER_HI);
    v.widgets.open.rounding = egui::Rounding::same(7.0);

    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(11.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.scroll.bar_width = 8.0;

    let bold = FontFamily::Name("bold".into());
    style.text_styles.insert(TextStyle::Heading, FontId::new(19.0, bold.clone()));
    style.text_styles.insert(TextStyle::Body, FontId::new(13.5, FontFamily::Proportional));
    style.text_styles.insert(TextStyle::Button, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );

    ctx.set_style(style);
}

/// Nút chính (màu nhấn).
pub fn accent_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).color(Color32::WHITE))
        .fill(ACCENT)
        .rounding(egui::Rounding::same(7.0))
}

/// Nút phụ (nền tối).
pub fn ghost_button(text: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(text.to_owned()).color(TEXT))
        .fill(PANEL_3)
        .rounding(egui::Rounding::same(7.0))
        .stroke(egui::Stroke::new(1.0_f32, BORDER))
}
