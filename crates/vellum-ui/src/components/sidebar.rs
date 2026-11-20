//! Left Navigation Sidebar: Page Thumbnails, Outline (TOC), and Annotations List.

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};
use vellum_core::document::PdfDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Thumbnails,
    Outline,
    Annotations,
}

#[derive(Debug, Clone, Default)]
pub struct SidebarLayout {
    pub bounds: Rect,
    pub tab_thumbnails: Rect,
    pub tab_outline: Rect,
    pub tab_annotations: Rect,
    pub thumbnail_cards: Vec<(usize, Rect)>, // (page_idx, rect)
    pub outline_items: Vec<(usize, Rect)>,   // (target_page_idx, rect)
}

pub fn render_sidebar(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    top_y: i32,
    bot_y: i32,
    sidebar_w: i32,
    active_tab: SidebarTab,
    current_page_idx: usize,
    doc: &PdfDocument,
) -> SidebarLayout {
    let bounds = Rect::new(0, top_y, sidebar_w, bot_y - top_y);

    // 1. Sidebar background (#0D1117) with right border (#30363D)
    fill_rect(surface, sw, sh, bounds, Color::rgb(13, 17, 23));
    fill_rect(
        surface,
        sw,
        sh,
        Rect::new(sidebar_w - 1, top_y, 1, bounds.h),
        Color::rgb(48, 54, 61),
    );

    let mut layout = SidebarLayout {
        bounds,
        ..Default::default()
    };

    // 2. Sidebar Tab Switcher: [ Thumbnails ] [ Outline ] [ Notes ]
    let tab_bar_h = 28;
    fill_rect(
        surface,
        sw,
        sh,
        Rect::new(0, top_y, sidebar_w - 1, tab_bar_h),
        Color::rgb(22, 27, 34),
    );
    fill_rect(
        surface,
        sw,
        sh,
        Rect::new(0, top_y + tab_bar_h - 1, sidebar_w - 1, 1),
        Color::rgb(48, 54, 61),
    );

    let tab_w = (sidebar_w - 12) / 3;
    let tabs = [
        (SidebarTab::Thumbnails, "Pages", 0),
        (SidebarTab::Outline, "Outline", 1),
        (SidebarTab::Annotations, "Annots", 2),
    ];

    for (t, label, i) in tabs {
        let is_sel = active_tab == t;
        let rect = Rect::new(6 + i * tab_w, top_y + 3, tab_w, tab_bar_h - 6);
        if is_sel {
            fill_rounded_rect(surface, sw, sh, rect, 4, Color::rgb(14, 165, 233));
            draw_text(surface, sw, sh, rect.x + 8, rect.y + 5, label, Color::WHITE, 1);
        } else {
            draw_text(
                surface,
                sw,
                sh,
                rect.x + 8,
                rect.y + 5,
                label,
                Color::rgb(148, 163, 184),
                1,
            );
        }

        match t {
            SidebarTab::Thumbnails => layout.tab_thumbnails = rect,
            SidebarTab::Outline => layout.tab_outline = rect,
            SidebarTab::Annotations => layout.tab_annotations = rect,
        }
    }

    // 3. Tab Body
    let body_y = top_y + tab_bar_h + 8;
    let _body_h = bounds.h - tab_bar_h - 16;

    match active_tab {
        SidebarTab::Thumbnails => {
            let card_w = sidebar_w - 32;
            let card_h = (card_w as f64 * 1.3).round() as i32; // ~4:3 or 1.3 aspect ratio

            for (i, _) in doc.pages.iter().enumerate() {
                let card_y = body_y + (i as i32) * (card_h + 26);
                if card_y + card_h > top_y + bounds.h {
                    break;
                }

                let card_rect = Rect::new(16, card_y, card_w, card_h);
                let is_current = i == current_page_idx;

                // Miniature page paper canvas
                fill_rounded_rect(surface, sw, sh, card_rect, 4, Color::WHITE);
                let outline_col = if is_current {
                    Color::rgb(14, 165, 233)
                } else {
                    Color::rgb(71, 85, 105)
                };
                draw_rect_outline(surface, sw, sh, card_rect, if is_current { 2 } else { 1 }, outline_col);

                // Mini preview lines on page
                fill_rect(surface, sw, sh, Rect::new(card_rect.x + 10, card_rect.y + 12, card_w - 20, 6), Color::rgb(203, 213, 225));
                fill_rect(surface, sw, sh, Rect::new(card_rect.x + 10, card_rect.y + 24, card_w - 32, 4), Color::rgb(226, 232, 240));
                fill_rect(surface, sw, sh, Rect::new(card_rect.x + 10, card_rect.y + 32, card_w - 24, 4), Color::rgb(226, 232, 240));
                fill_rect(surface, sw, sh, Rect::new(card_rect.x + 10, card_rect.y + 40, card_w - 40, 4), Color::rgb(226, 232, 240));

                // Page Number Label
                let page_label = format!("Page {}", i + 1);
                let label_col = if is_current {
                    Color::rgb(56, 189, 248)
                } else {
                    Color::rgb(148, 163, 184)
                };
                draw_text(surface, sw, sh, card_rect.x + (card_w / 2) - 20, card_rect.bottom() + 6, &page_label, label_col, 1);

                layout.thumbnail_cards.push((i, card_rect));
            }
        }
        SidebarTab::Outline => {
            let mut item_y = body_y;
            for (_i, item) in doc.outlines.iter().enumerate() {
                if item_y + 24 > top_y + bounds.h {
                    break;
                }
                let item_rect = Rect::new(12, item_y, sidebar_w - 24, 22);
                let is_current = item.page_index == current_page_idx;

                if is_current {
                    fill_rounded_rect(surface, sw, sh, item_rect, 4, Color::rgba(14, 165, 233, 40));
                }
                draw_text(
                    surface,
                    sw,
                    sh,
                    item_rect.x + 6,
                    item_rect.y + 5,
                    &item.title,
                    if is_current { Color::WHITE } else { Color::rgb(203, 213, 225) },
                    1,
                );

                layout.outline_items.push((item.page_index, item_rect));
                item_y += 26;
            }
        }
        SidebarTab::Annotations => {
            let mut item_y = body_y;
            let mut count = 0;
            for (p_idx, page) in doc.pages.iter().enumerate() {
                for annot in &page.annotations {
                    if item_y + 28 > top_y + bounds.h {
                        break;
                    }
                    let item_rect = Rect::new(10, item_y, sidebar_w - 20, 26);
                    fill_rounded_rect(surface, sw, sh, item_rect, 4, Color::rgba(255, 255, 255, 6));

                    let desc = match &annot.annot_type {
                        vellum_core::annotations::AnnotationType::Highlight { .. } => "Highlight",
                        vellum_core::annotations::AnnotationType::Underline { .. } => "Underline",
                        vellum_core::annotations::AnnotationType::StrikeOut { .. } => "Strike",
                        vellum_core::annotations::AnnotationType::Ink { .. } => "Ink Sketch",
                        vellum_core::annotations::AnnotationType::Square { .. } => "Rectangle",
                        vellum_core::annotations::AnnotationType::Circle { .. } => "Circle",
                        vellum_core::annotations::AnnotationType::Line { .. } => "Line",
                        vellum_core::annotations::AnnotationType::FreeText { .. } => "Text Box",
                        vellum_core::annotations::AnnotationType::StickyNote { .. } => "Sticky Note",
                        vellum_core::annotations::AnnotationType::Stamp { label, .. } => label.as_str(),
                        vellum_core::annotations::AnnotationType::Redact { .. } => "Redaction",
                    };

                    let line = format!("P{}: {}", p_idx + 1, desc);
                    draw_text(surface, sw, sh, item_rect.x + 8, item_rect.y + 7, &line, Color::rgb(226, 232, 240), 1);
                    layout.outline_items.push((p_idx, item_rect));
                    item_y += 30;
                    count += 1;
                }
            }
            if count == 0 {
                draw_text(
                    surface,
                    sw,
                    sh,
                    16,
                    body_y + 12,
                    "No annotations yet.",
                    Color::rgb(100, 116, 139),
                    1,
                );
            }
        }
    }

    layout
}
