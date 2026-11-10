//! VieEdit — trình biên tập video tiếng Việt viết bằng Rust.
//!
//! Chạy được trên Windows và Fedora Linux. Toàn bộ giao diện bằng tiếng Việt,
//! có thể chuyển sang tiếng Anh trong menu Trợ giúp → Ngôn ngữ.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use viedit::app::VieEdit;

fn load_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    Some(egui::IconData {
        rgba: img.as_raw().clone(),
        width: img.width(),
        height: img.height(),
    })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1460.0, 920.0])
        .with_min_inner_size([1040.0, 640.0])
        .with_title("VieEdit — Trình biên tập video tiếng Việt");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "VieEdit",
        options,
        Box::new(|cc| Ok(Box::new(VieEdit::new(cc)))),
    )
}
