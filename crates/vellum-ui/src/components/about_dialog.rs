//! About Vellum Modal Dialog.

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

pub fn render_about_dialog(surface: &mut [u32], sw: i32, sh: i32) -> Rect {
    let dw = 420.min(sw - 30);
    let dh = 240.min(sh - 30);
    let dx = (sw - dw) / 2;
    let dy = (sh - dh) / 2;
    let bounds = Rect::new(dx, dy, dw, dh);

    // Dim background overlay
    fill_rect(surface, sw, sh, Rect::new(0, 0, sw, sh), Color::rgba(0, 0, 0, 160));

    // Acrylic modal card (#0F172A)
    fill_rounded_rect(surface, sw, sh, bounds, 10, Color::rgb(15, 23, 42));
    draw_rect_outline(surface, sw, sh, bounds, 2, Color::rgb(56, 189, 248));

    // Logo & Title
    fill_rounded_rect(surface, sw, sh, Rect::new(dx + 20, dy + 18, 30, 30), 6, Color::rgb(14, 165, 233));
    draw_text(surface, sw, sh, dx + 28, dy + 23, "V", Color::WHITE, 2);

    draw_text(surface, sw, sh, dx + 60, dy + 18, "VELLUM PDF STUDIO", Color::WHITE, 2);
    draw_text(surface, sw, sh, dx + 60, dy + 36, "v1.0.0 (Pure Rust 1.99.1)", Color::rgb(56, 189, 248), 1);

    fill_rect(surface, sw, sh, Rect::new(dx + 16, dy + 58, dw - 32, 1), Color::rgb(48, 54, 61));

    draw_text(
        surface,
        sw,
        sh,
        dx + 20,
        dy + 70,
        "High-Performance PDF Reader, Editor & Writer.",
        Color::rgb(226, 232, 240),
        1,
    );
    draw_text(
        surface,
        sw,
        sh,
        dx + 20,
        dy + 90,
        "Engine Architecture: Adobe Edit + macOS Preview + Evince Read",
        Color::rgb(148, 163, 184),
        1,
    );
    draw_text(
        surface,
        sw,
        sh,
        dx + 20,
        dy + 112,
        "Pure Rust 1.99.1 · Zero C/C++ · Zero External Libraries",
        Color::rgb(74, 222, 128),
        1,
    );
    draw_text(
        surface,
        sw,
        sh,
        dx + 20,
        dy + 132,
        "Full PDF 1.7 Spec: XRef, FlateDecode, Vector Paths, Forms & Redact",
        Color::rgb(245, 158, 11),
        1,
    );

    // Dismiss button [ Dismiss ]
    let btn_rect = Rect::new(dx + (dw - 90) / 2, dy + dh - 40, 90, 26);
    fill_rounded_rect(surface, sw, sh, btn_rect, 4, Color::rgb(14, 165, 233));
    draw_text(surface, sw, sh, btn_rect.x + 22, btn_rect.y + 7, "Dismiss", Color::WHITE, 1);

    btn_rect
}
