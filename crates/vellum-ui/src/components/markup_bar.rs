//! macOS Preview-Style Markup & Annotation Toolbar: Pen, Highlighter, Shapes, Text Box, Stamps, Redaction.

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkupTool {
    #[default]
    Select,
    TextSelect,
    Highlight,
    Underline,
    StrikeOut,
    Pen,
    Rectangle,
    Circle,
    Line,
    Arrow,
    TextBox,
    StickyNote,
    Stamp,
    Redact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HighlightColor {
    #[default]
    Yellow,
    Green,
    Cyan,
    Coral,
    Purple,
}

impl HighlightColor {
    pub fn to_rgba(&self) -> [u8; 4] {
        match self {
            HighlightColor::Yellow => [254, 240, 138, 140],
            HighlightColor::Green => [187, 247, 208, 140],
            HighlightColor::Cyan => [186, 230, 253, 140],
            HighlightColor::Coral => [254, 205, 211, 140],
            HighlightColor::Purple => [233, 213, 255, 140],
        }
    }

    pub fn to_preview_color(&self) -> Color {
        match self {
            HighlightColor::Yellow => Color::rgb(250, 204, 21),
            HighlightColor::Green => Color::rgb(74, 222, 128),
            HighlightColor::Cyan => Color::rgb(56, 189, 248),
            HighlightColor::Coral => Color::rgb(244, 63, 94),
            HighlightColor::Purple => Color::rgb(192, 132, 252),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MarkupBarLayout {
    pub bounds: Rect,
    pub tool_buttons: [(MarkupTool, Rect); 14],
    pub color_swatches: [(HighlightColor, Rect); 5],
}

pub fn render_markup_bar(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    y_offset: i32,
    active_tool: MarkupTool,
    active_color: HighlightColor,
) -> MarkupBarLayout {
    let bar_h = 36;
    let bounds = Rect::new(0, y_offset, sw, bar_h);

    // 1. Frosted acrylic secondary strip (#161B22) with border (#30363D)
    fill_rect(surface, sw, sh, bounds, Color::rgb(22, 27, 34));
    fill_rect(
        surface,
        sw,
        sh,
        Rect::new(0, y_offset + bar_h - 1, sw, 1),
        Color::rgb(48, 54, 61),
    );

    let mut layout = MarkupBarLayout {
        bounds,
        ..Default::default()
    };

    let btn_y = y_offset + 5;
    let btn_h = 26;
    let mut cur_x = 16;

    let tools = [
        (MarkupTool::Select, "Select", 48),
        (MarkupTool::TextSelect, "Text", 42),
        (MarkupTool::Highlight, "Highlight", 68),
        (MarkupTool::Underline, "Underline", 68),
        (MarkupTool::StrikeOut, "Strike", 50),
        (MarkupTool::Pen, "Pen", 38),
        (MarkupTool::Rectangle, "Rect", 42),
        (MarkupTool::Circle, "Oval", 42),
        (MarkupTool::Line, "Line", 40),
        (MarkupTool::Arrow, "Arrow", 46),
        (MarkupTool::TextBox, "Text Box", 64),
        (MarkupTool::StickyNote, "Note", 42),
        (MarkupTool::Stamp, "Stamp", 48),
        (MarkupTool::Redact, "Redact", 54),
    ];

    for (i, (t, label, w)) in tools.into_iter().enumerate() {
        let is_sel = active_tool == t;
        let rect = Rect::new(cur_x, btn_y, w, btn_h);

        if is_sel {
            fill_rounded_rect(surface, sw, sh, rect, 4, Color::rgb(14, 165, 233));
            draw_text(surface, sw, sh, cur_x + 8, btn_y + 7, label, Color::WHITE, 1);
        } else {
            fill_rounded_rect(surface, sw, sh, rect, 4, Color::rgba(255, 255, 255, 8));
            draw_text(
                surface,
                sw,
                sh,
                cur_x + 8,
                btn_y + 7,
                label,
                Color::rgb(203, 213, 225),
                1,
            );
        }

        layout.tool_buttons[i] = (t, rect);
        cur_x += w + 4;
    }

    // Divider
    fill_rect(surface, sw, sh, Rect::new(cur_x + 6, btn_y + 3, 1, 20), Color::rgb(48, 54, 61));
    cur_x += 18;

    // 2. Color Swatches (Yellow, Green, Cyan, Coral, Purple)
    let colors = [
        HighlightColor::Yellow,
        HighlightColor::Green,
        HighlightColor::Cyan,
        HighlightColor::Coral,
        HighlightColor::Purple,
    ];

    draw_text(surface, sw, sh, cur_x, btn_y + 7, "Color:", Color::rgb(148, 163, 184), 1);
    cur_x += 42;

    for (i, c) in colors.into_iter().enumerate() {
        let is_sel = active_color == c;
        let swatch_rect = Rect::new(cur_x, btn_y + 4, 18, 18);
        let col = c.to_preview_color();

        fill_rounded_rect(surface, sw, sh, swatch_rect, 9, col);
        if is_sel {
            draw_rect_outline(surface, sw, sh, swatch_rect, 2, Color::WHITE);
        }

        layout.color_swatches[i] = (c, swatch_rect);
        cur_x += 24;
    }

    layout
}
