//! Ubuntu Evince-Style Fast Document Search Bar.

use crate::draw::{draw_rect_outline, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};
use vellum_core::search::SearchEngine;

#[derive(Debug, Clone, Copy, Default)]
pub struct SearchBarLayout {
    pub bounds: Rect,
    pub input_box: Rect,
    pub btn_prev: Rect,
    pub btn_next: Rect,
    pub btn_case: Rect,
    pub btn_close: Rect,
}

pub fn render_search_bar(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    y: i32,
    search: &SearchEngine,
) -> SearchBarLayout {
    let bar_w = 360.min(sw - 20);
    let bar_h = 36;
    let bar_x = sw - bar_w - 20;
    let bounds = Rect::new(bar_x, y, bar_w, bar_h);

    // 1. Frosted acrylic floating search island
    fill_rounded_rect(surface, sw, sh, bounds, 8, Color::rgb(22, 27, 34));
    draw_rect_outline(surface, sw, sh, bounds, 1, Color::rgb(56, 189, 248));

    let mut layout = SearchBarLayout {
        bounds,
        ..Default::default()
    };

    let btn_y = y + 6;
    let btn_h = 24;

    // Search query input field
    let input_w = 160;
    let input_rect = Rect::new(bar_x + 8, btn_y, input_w, btn_h);
    fill_rounded_rect(surface, sw, sh, input_rect, 4, Color::rgb(13, 17, 23));
    draw_rect_outline(surface, sw, sh, input_rect, 1, Color::rgb(48, 54, 61));

    let display_q = if search.query.is_empty() {
        "Type to find..."
    } else {
        &search.query
    };
    let q_col = if search.query.is_empty() {
        Color::rgb(100, 116, 139)
    } else {
        Color::WHITE
    };
    draw_text(surface, sw, sh, input_rect.x + 8, btn_y + 6, display_q, q_col, 1);
    layout.input_box = input_rect;

    let mut cur_x = input_rect.right() + 8;

    // Match count badge
    let count_str = if search.count() > 0 {
        format!("{}/{}", search.active_hit_index.unwrap_or(0) + 1, search.count())
    } else if !search.query.is_empty() {
        "0/0".to_string()
    } else {
        "".to_string()
    };
    if !count_str.is_empty() {
        draw_text(surface, sw, sh, cur_x, btn_y + 6, &count_str, Color::rgb(245, 158, 11), 1);
    }
    cur_x += 42;

    // Prev [ < ] button
    let prev_rect = Rect::new(cur_x, btn_y, 24, btn_h);
    fill_rounded_rect(surface, sw, sh, prev_rect, 4, Color::rgba(255, 255, 255, 12));
    draw_text(surface, sw, sh, cur_x + 8, btn_y + 6, "<", Color::WHITE, 1);
    layout.btn_prev = prev_rect;
    cur_x += 28;

    // Next [ > ] button
    let next_rect = Rect::new(cur_x, btn_y, 24, btn_h);
    fill_rounded_rect(surface, sw, sh, next_rect, 4, Color::rgba(255, 255, 255, 12));
    draw_text(surface, sw, sh, cur_x + 8, btn_y + 6, ">", Color::WHITE, 1);
    layout.btn_next = next_rect;
    cur_x += 28;

    // Case Sensitivity [ Aa ]
    let case_rect = Rect::new(cur_x, btn_y, 28, btn_h);
    let case_bg = if search.case_sensitive { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 12) };
    fill_rounded_rect(surface, sw, sh, case_rect, 4, case_bg);
    draw_text(surface, sw, sh, cur_x + 6, btn_y + 6, "Aa", Color::WHITE, 1);
    layout.btn_case = case_rect;
    cur_x += 32;

    // Close [ x ]
    let close_rect = Rect::new(cur_x, btn_y, 22, btn_h);
    draw_text(surface, sw, sh, cur_x + 6, btn_y + 6, "X", Color::rgb(148, 163, 184), 1);
    layout.btn_close = close_rect;

    layout
}
