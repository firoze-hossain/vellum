//! Adobe Acrobat-Style Right Inspector Panel: Document Metadata, Object Properties, and Page Management.

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};
use vellum_core::annotations::Annotation;
use vellum_core::document::PdfDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorAction {
    RotateCw,
    RotateCcw,
    InsertPage,
    DeletePage,
}

#[derive(Debug, Clone, Default)]
pub struct InspectorLayout {
    pub bounds: Rect,
    pub btn_rotate_cw: Rect,
    pub btn_rotate_ccw: Rect,
    pub btn_insert_page: Rect,
    pub btn_delete_page: Rect,
}

pub fn render_inspector(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    top_y: i32,
    bot_y: i32,
    inspector_w: i32,
    current_page_idx: usize,
    selected_annot: Option<&Annotation>,
    doc: &PdfDocument,
) -> InspectorLayout {
    let x = sw - inspector_w;
    let bounds = Rect::new(x, top_y, inspector_w, bot_y - top_y);

    // 1. Inspector background (#0D1117) with left border (#30363D)
    fill_rect(surface, sw, sh, bounds, Color::rgb(13, 17, 23));
    fill_rect(
        surface,
        sw,
        sh,
        Rect::new(x, top_y, 1, bounds.h),
        Color::rgb(48, 54, 61),
    );

    let mut layout = InspectorLayout {
        bounds,
        ..Default::default()
    };

    // 2. Header: INSPECTOR
    let mut cur_y = top_y + 12;
    draw_text(
        surface,
        sw,
        sh,
        x + 16,
        cur_y,
        "PROPERTIES",
        Color::rgb(56, 189, 248),
        1,
    );
    cur_y += 18;
    fill_rect(surface, sw, sh, Rect::new(x + 12, cur_y, inspector_w - 24, 1), Color::rgb(48, 54, 61));
    cur_y += 10;

    // Document & Page Info
    let page_count = doc.pages.len();
    let current_page = doc.pages.get(current_page_idx);
    let (pw, ph, rot) = if let Some(p) = current_page {
        (p.width(), p.height(), p.rotation)
    } else {
        (612.0, 792.0, 0)
    };

    let title_line = format!("Title: {}", if doc.metadata.title.is_empty() { "Untitled" } else { &doc.metadata.title });
    draw_text(surface, sw, sh, x + 16, cur_y, &title_line, Color::rgb(226, 232, 240), 1);
    cur_y += 16;

    let ver_line = format!("PDF Version: {}", doc.metadata.pdf_version);
    draw_text(surface, sw, sh, x + 16, cur_y, &ver_line, Color::rgb(148, 163, 184), 1);
    cur_y += 16;

    let page_line = format!("Page: {} of {}", current_page_idx + 1, page_count);
    draw_text(surface, sw, sh, x + 16, cur_y, &page_line, Color::rgb(148, 163, 184), 1);
    cur_y += 16;

    let dim_line = format!("Size: {:.0} x {:.0} pt", pw, ph);
    draw_text(surface, sw, sh, x + 16, cur_y, &dim_line, Color::rgb(148, 163, 184), 1);
    cur_y += 16;

    let rot_line = format!("Rotation: {} deg", rot);
    draw_text(surface, sw, sh, x + 16, cur_y, &rot_line, Color::rgb(148, 163, 184), 1);
    cur_y += 24;

    // 3. Page Management Section (macOS Preview / Acrobat)
    fill_rect(surface, sw, sh, Rect::new(x + 12, cur_y, inspector_w - 24, 1), Color::rgb(48, 54, 61));
    cur_y += 10;
    draw_text(
        surface,
        sw,
        sh,
        x + 16,
        cur_y,
        "PAGE ACTIONS",
        Color::rgb(56, 189, 248),
        1,
    );
    cur_y += 18;

    let btn_w = (inspector_w - 38) / 2;
    let btn_h = 24;

    // Rotate CW / CCW buttons
    let rot_cw_rect = Rect::new(x + 16, cur_y, btn_w, btn_h);
    fill_rounded_rect(surface, sw, sh, rot_cw_rect, 4, Color::rgba(255, 255, 255, 12));
    draw_text(surface, sw, sh, rot_cw_rect.x + 8, rot_cw_rect.y + 6, "Rotate CW", Color::WHITE, 1);
    layout.btn_rotate_cw = rot_cw_rect;

    let rot_ccw_rect = Rect::new(x + 22 + btn_w, cur_y, btn_w, btn_h);
    fill_rounded_rect(surface, sw, sh, rot_ccw_rect, 4, Color::rgba(255, 255, 255, 12));
    draw_text(surface, sw, sh, rot_ccw_rect.x + 8, rot_ccw_rect.y + 6, "Rotate CCW", Color::WHITE, 1);
    layout.btn_rotate_ccw = rot_ccw_rect;
    cur_y += 32;

    // Insert Page / Delete Page buttons
    let ins_rect = Rect::new(x + 16, cur_y, btn_w, btn_h);
    fill_rounded_rect(surface, sw, sh, ins_rect, 4, Color::rgba(14, 165, 233, 40));
    draw_rect_outline(surface, sw, sh, ins_rect, 1, Color::rgb(14, 165, 233));
    draw_text(surface, sw, sh, ins_rect.x + 8, ins_rect.y + 6, "+ Insert", Color::WHITE, 1);
    layout.btn_insert_page = ins_rect;

    let del_rect = Rect::new(x + 22 + btn_w, cur_y, btn_w, btn_h);
    fill_rounded_rect(surface, sw, sh, del_rect, 4, Color::rgba(239, 68, 68, 30));
    draw_rect_outline(surface, sw, sh, del_rect, 1, Color::rgb(239, 68, 68));
    draw_text(surface, sw, sh, del_rect.x + 8, del_rect.y + 6, "Delete", Color::rgb(248, 113, 113), 1);
    layout.btn_delete_page = del_rect;
    cur_y += 36;

    // 4. Selected Object / Annotation Inspector
    fill_rect(surface, sw, sh, Rect::new(x + 12, cur_y, inspector_w - 24, 1), Color::rgb(48, 54, 61));
    cur_y += 10;
    draw_text(
        surface,
        sw,
        sh,
        x + 16,
        cur_y,
        "ANNOTATION DETAILS",
        Color::rgb(56, 189, 248),
        1,
    );
    cur_y += 18;

    if let Some(annot) = selected_annot {
        let id_str = format!("ID: #{}", annot.id);
        draw_text(surface, sw, sh, x + 16, cur_y, &id_str, Color::rgb(226, 232, 240), 1);
        cur_y += 16;

        let author_str = format!("Author: {}", annot.author);
        draw_text(surface, sw, sh, x + 16, cur_y, &author_str, Color::rgb(148, 163, 184), 1);
        cur_y += 16;

        if !annot.contents.is_empty() {
            let cont_str = format!("Note: {}", annot.contents);
            draw_text(surface, sw, sh, x + 16, cur_y, &cont_str, Color::rgb(203, 213, 225), 1);
        }
    } else {
        draw_text(
            surface,
            sw,
            sh,
            x + 16,
            cur_y,
            "No annotation selected.",
            Color::rgb(100, 116, 139),
            1,
        );
    }

    layout
}
