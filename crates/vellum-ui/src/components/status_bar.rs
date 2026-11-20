//! Bottom Status Bar: Page Navigation Stepper, Zoom Slider, Layout Mode, and Document Geometry.

use crate::draw::{fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLayoutMode {
    Single,
    Facing,     // Two-page book view
    Continuous, // Scrolling roll
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StatusBarLayout {
    pub bounds: Rect,
    pub btn_prev_page: Rect,
    pub btn_next_page: Rect,
    pub btn_single: Rect,
    pub btn_facing: Rect,
    pub btn_continuous: Rect,
    pub btn_zoom_out: Rect,
    pub btn_zoom_in: Rect,
    pub btn_fit_width: Rect,
    pub btn_fit_page: Rect,
}

pub fn render_status_bar(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    current_page: usize,
    total_pages: usize,
    zoom_pct: u32,
    layout_mode: PageLayoutMode,
    page_w: f64,
    page_h: f64,
) -> StatusBarLayout {
    let bar_h = 30;
    let y = sh - bar_h;
    let bounds = Rect::new(0, y, sw, bar_h);

    // 1. Dark bottom status background (#0F172A) with top line (#1E293B)
    fill_rect(surface, sw, sh, bounds, Color::rgb(15, 23, 42));
    fill_rect(surface, sw, sh, Rect::new(0, y, sw, 1), Color::rgb(30, 41, 59));

    let mut layout = StatusBarLayout {
        bounds,
        ..Default::default()
    };

    let btn_y = y + 4;
    let btn_h = 22;

    // 2. Page Navigation Stepper: [ < ] Page 1 of 5 [ > ]
    let mut cur_x = 12;

    let prev_p_rect = Rect::new(cur_x, btn_y, 22, btn_h);
    fill_rounded_rect(surface, sw, sh, prev_p_rect, 3, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, cur_x + 7, btn_y + 5, "<", Color::WHITE, 1);
    layout.btn_prev_page = prev_p_rect;
    cur_x += 26;

    let page_str = format!("Page {} of {}", current_page + 1, total_pages.max(1));
    draw_text(surface, sw, sh, cur_x + 4, btn_y + 5, &page_str, Color::rgb(226, 232, 240), 1);
    cur_x += 84;

    let next_p_rect = Rect::new(cur_x, btn_y, 22, btn_h);
    fill_rounded_rect(surface, sw, sh, next_p_rect, 3, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, cur_x + 7, btn_y + 5, ">", Color::WHITE, 1);
    layout.btn_next_page = next_p_rect;
    cur_x += 36;

    // Dimensions Badge: e.g. "612 x 792 pt"
    let dim_str = format!("{:.0} x {:.0} pt", page_w, page_h);
    draw_text(surface, sw, sh, cur_x, btn_y + 5, &dim_str, Color::rgb(100, 116, 139), 1);

    // 3. Right Controls: Page Layout Mode, Zoom Controls, Fit Width/Page
    let mut right_x = sw - 12;

    // Fit Page
    right_x -= 58;
    let fit_page_rect = Rect::new(right_x, btn_y, 54, btn_h);
    fill_rounded_rect(surface, sw, sh, fit_page_rect, 3, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, right_x + 6, btn_y + 5, "Fit Page", Color::rgb(203, 213, 225), 1);
    layout.btn_fit_page = fit_page_rect;

    // Fit Width
    right_x -= 62;
    let fit_width_rect = Rect::new(right_x, btn_y, 58, btn_h);
    fill_rounded_rect(surface, sw, sh, fit_width_rect, 3, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, right_x + 6, btn_y + 5, "Fit Width", Color::rgb(203, 213, 225), 1);
    layout.btn_fit_width = fit_width_rect;

    // Zoom [ + ]
    right_x -= 26;
    let zoom_in_rect = Rect::new(right_x, btn_y, 22, btn_h);
    fill_rounded_rect(surface, sw, sh, zoom_in_rect, 3, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, right_x + 7, btn_y + 5, "+", Color::WHITE, 1);
    layout.btn_zoom_in = zoom_in_rect;

    // Zoom Percentage label
    right_x -= 44;
    let zoom_str = format!("{}%", zoom_pct);
    draw_text(surface, sw, sh, right_x + 4, btn_y + 5, &zoom_str, Color::rgb(56, 189, 248), 1);

    // Zoom [ - ]
    right_x -= 26;
    let zoom_out_rect = Rect::new(right_x, btn_y, 22, btn_h);
    fill_rounded_rect(surface, sw, sh, zoom_out_rect, 3, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, right_x + 7, btn_y + 5, "-", Color::WHITE, 1);
    layout.btn_zoom_out = zoom_out_rect;

    // Page Layout Buttons: [Single] [Facing] [Scroll]
    right_x -= 16;
    fill_rect(surface, sw, sh, Rect::new(right_x, btn_y + 2, 1, 18), Color::rgb(51, 65, 85));
    right_x -= 12;

    let layouts = [
        (PageLayoutMode::Continuous, "Scroll", 46),
        (PageLayoutMode::Facing, "Facing", 48),
        (PageLayoutMode::Single, "Single", 46),
    ];

    for (m, lbl, w) in layouts {
        right_x -= w;
        let is_sel = layout_mode == m;
        let r = Rect::new(right_x, btn_y, w, btn_h);
        if is_sel {
            fill_rounded_rect(surface, sw, sh, r, 3, Color::rgb(14, 165, 233));
            draw_text(surface, sw, sh, r.x + 6, btn_y + 5, lbl, Color::WHITE, 1);
        } else {
            fill_rounded_rect(surface, sw, sh, r, 3, Color::rgba(255, 255, 255, 6));
            draw_text(surface, sw, sh, r.x + 6, btn_y + 5, lbl, Color::rgb(148, 163, 184), 1);
        }

        match m {
            PageLayoutMode::Single => layout.btn_single = r,
            PageLayoutMode::Facing => layout.btn_facing = r,
            PageLayoutMode::Continuous => layout.btn_continuous = r,
        }
        right_x -= 4;
    }

    layout
}
