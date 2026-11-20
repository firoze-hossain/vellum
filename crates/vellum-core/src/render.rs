//! Software Page Rasterizer: Vector paths, text typography, annotations, and search highlights.

use crate::annotations::{Annotation, AnnotationType};
use crate::content::{ColorRgb, DisplayList, PathSegment, TextSpan, VectorPath};
use crate::font::StandardFont;
use crate::types::{PdfRect, Point};

/// Render configuration options.
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub zoom: f64,              // 1.0 = 100% (72 DPI)
    pub rotation: u16,          // 0, 90, 180, 270
    pub invert_colors: bool,    // Night reading mode
    pub render_annotations: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            rotation: 0,
            invert_colors: false,
            render_annotations: true,
        }
    }
}

/// Software rasterizer rendering PDF display lists to a 32-bit ARGB surface (`0xAARRGGBB`).
pub struct PageRenderer;

impl PageRenderer {
    pub fn render_page(
        surface: &mut [u32],
        surf_w: i32,
        surf_h: i32,
        page_rect: PdfRect,
        display_list: &DisplayList,
        annotations: &[Annotation],
        search_highlights: &[PdfRect],
        active_search_hit: Option<usize>,
        options: &RenderOptions,
    ) {
        if surf_w <= 0 || surf_h <= 0 {
            return;
        }

        // 1. Clear paper background (Pure white #FFFFFF or Dark #121826 if inverted)
        let paper_bg = if options.invert_colors { 0xFF121826 } else { 0xFFFFFFFF };
        surface.fill(paper_bg);

        let page_h = page_rect.height();

        // Coordinate conversion helper: PDF (y=0 at bottom) to Screen (y=0 at top)
        let to_screen = |p: Point| -> (i32, i32) {
            let sx = (p.x * options.zoom).round() as i32;
            let sy = ((page_h - p.y) * options.zoom).round() as i32;
            (sx, sy)
        };

        // 2. Render Vector Paths (borders, tables, shapes, charts)
        for path in &display_list.paths {
            Self::render_path(surface, surf_w, surf_h, path, page_h, options);
        }

        // 3. Render Search Highlights (behind text)
        for (i, hit_rect) in search_highlights.iter().enumerate() {
            let (x1, y2) = to_screen(Point::new(hit_rect.llx, hit_rect.lly));
            let (x2, y1) = to_screen(Point::new(hit_rect.urx, hit_rect.ury));

            let rx = x1.min(x2);
            let ry = y1.min(y2);
            let rw = (x2 - x1).abs().max(4);
            let rh = (y2 - y1).abs().max(4);

            let is_active = active_search_hit == Some(i);
            let highlight_col = if is_active {
                ColorRgb { r: 1.0, g: 0.6, b: 0.0, a: 0.65 } // Amber/Orange glow
            } else {
                ColorRgb { r: 1.0, g: 0.9, b: 0.2, a: 0.45 } // Soft yellow
            };

            Self::blend_rect(surface, surf_w, surf_h, rx, ry, rw, rh, highlight_col);
            if is_active {
                Self::draw_rect_outline(surface, surf_w, surf_h, rx, ry, rw, rh, 0xFFE65100, 2);
            }
        }

        // 4. Render Text Spans
        for span in &display_list.text_spans {
            Self::render_text_span(surface, surf_w, surf_h, span, page_h, options);
        }

        // 5. Render Annotations (Highlights, Pens, Shapes, Notes, Stamps, Redactions)
        if options.render_annotations {
            for annot in annotations {
                Self::render_annotation(surface, surf_w, surf_h, annot, page_h, options);
            }
        }

        // Invert colors if night reading mode is active
        if options.invert_colors {
            for px in surface.iter_mut() {
                let a = (*px >> 24) & 0xFF;
                let r = 255 - ((*px >> 16) & 0xFF);
                let g = 255 - ((*px >> 8) & 0xFF);
                let b = 255 - (*px & 0xFF);
                *px = (a << 24) | (r << 16) | (g << 8) | b;
            }
        }
    }

    fn render_path(
        surface: &mut [u32],
        surf_w: i32,
        surf_h: i32,
        path: &VectorPath,
        page_h: f64,
        options: &RenderOptions,
    ) {
        let mut cur_pt = Point::default();

        let to_scr = |p: Point| -> (i32, i32) {
            let sx = (p.x * options.zoom).round() as i32;
            let sy = ((page_h - p.y) * options.zoom).round() as i32;
            (sx, sy)
        };

        // If path has fill color, check if it's a rectangle for fast solid fill
        if let Some(fc) = path.fill_color {
            if path.segments.len() >= 4 {
                if let (PathSegment::MoveTo(p1), PathSegment::LineTo(p2), PathSegment::LineTo(p3)) =
                    (&path.segments[0], &path.segments[1], &path.segments[2])
                {
                    if (p1.y == p2.y && p2.x == p3.x) || (p1.x == p2.x && p2.y == p3.y) {
                        let (x1, y1) = to_scr(*p1);
                        let (x3, y3) = to_scr(*p3);
                        let rx = x1.min(x3);
                        let ry = y1.min(y3);
                        let rw = (x3 - x1).abs();
                        let rh = (y3 - y1).abs();
                        Self::fill_rect(surface, surf_w, surf_h, rx, ry, rw, rh, fc.to_u32());
                    }
                }
            }
        }

        // Draw stroked lines
        if let Some(sc) = path.stroke_color {
            let col = sc.to_u32();
            let lw = (path.line_width * options.zoom).round().max(1.0) as i32;

            for seg in &path.segments {
                match seg {
                    PathSegment::MoveTo(p) => cur_pt = *p,
                    PathSegment::LineTo(p) => {
                        let (x0, y0) = to_scr(cur_pt);
                        let (x1, y1) = to_scr(*p);
                        Self::draw_line(surface, surf_w, surf_h, x0, y0, x1, y1, col, lw);
                        cur_pt = *p;
                    }
                    PathSegment::CurveTo(c1, c2, p) => {
                        // Subdivide cubic bezier into 8 line segments
                        let mut prev = cur_pt;
                        for step in 1..=8 {
                            let t = step as f64 / 8.0;
                            let pt = eval_cubic_bezier(cur_pt, *c1, *c2, *p, t);
                            let (x0, y0) = to_scr(prev);
                            let (x1, y1) = to_scr(pt);
                            Self::draw_line(surface, surf_w, surf_h, x0, y0, x1, y1, col, lw);
                            prev = pt;
                        }
                        cur_pt = *p;
                    }
                    PathSegment::Close => {}
                }
            }
        }
    }

    fn render_text_span(
        surface: &mut [u32],
        surf_w: i32,
        surf_h: i32,
        span: &TextSpan,
        page_h: f64,
        options: &RenderOptions,
    ) {
        let sx = (span.x * options.zoom).round() as i32;
        let sy = ((page_h - (span.y + span.height)) * options.zoom).round() as i32;
        let font_size_px = (span.font_size * options.zoom).round().max(6.0) as i32;

        let col = span.color.to_u32();
        let font = StandardFont::from_name(&span.font_name);

        let mut cur_x = sx;
        for c in span.text.chars() {
            let glyph_w_pts = font.glyph_width(c) as f64 * span.font_size / 1000.0;
            let char_w = (glyph_w_pts * options.zoom).round().max(1.0) as i32;

            Self::draw_bitmap_char(surface, surf_w, surf_h, cur_x, sy, c, col, font_size_px);
            cur_x += char_w;
        }
    }

    fn render_annotation(
        surface: &mut [u32],
        surf_w: i32,
        surf_h: i32,
        annot: &Annotation,
        page_h: f64,
        options: &RenderOptions,
    ) {
        let to_scr = |p: Point| -> (i32, i32) {
            let sx = (p.x * options.zoom).round() as i32;
            let sy = ((page_h - p.y) * options.zoom).round() as i32;
            (sx, sy)
        };

        match &annot.annot_type {
            AnnotationType::Highlight { color, rects } => {
                let rgba = ColorRgb {
                    r: color[0] as f64 / 255.0,
                    g: color[1] as f64 / 255.0,
                    b: color[2] as f64 / 255.0,
                    a: (color[3] as f64 / 255.0).min(0.55),
                };
                for r in rects {
                    let (x1, y2) = to_scr(Point::new(r.llx, r.lly));
                    let (x2, y1) = to_scr(Point::new(r.urx, r.ury));
                    let rx = x1.min(x2);
                    let ry = y1.min(y2);
                    let rw = (x2 - x1).abs();
                    let rh = (y2 - y1).abs();
                    Self::blend_rect(surface, surf_w, surf_h, rx, ry, rw, rh, rgba);
                }
            }
            AnnotationType::Underline { color, rects } => {
                let col = ((color[3] as u32) << 24) | ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | (color[2] as u32);
                for r in rects {
                    let (x1, y) = to_scr(Point::new(r.llx, r.lly));
                    let (x2, _) = to_scr(Point::new(r.urx, r.lly));
                    Self::draw_line(surface, surf_w, surf_h, x1, y, x2, y, col, 2);
                }
            }
            AnnotationType::StrikeOut { color, rects } => {
                let col = ((color[3] as u32) << 24) | ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | (color[2] as u32);
                for r in rects {
                    let mid_y = (r.lly + r.ury) / 2.0;
                    let (x1, y) = to_scr(Point::new(r.llx, mid_y));
                    let (x2, _) = to_scr(Point::new(r.urx, mid_y));
                    Self::draw_line(surface, surf_w, surf_h, x1, y, x2, y, col, 2);
                }
            }
            AnnotationType::Ink { strokes, color, stroke_width } => {
                let col = ((color[3] as u32) << 24) | ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | (color[2] as u32);
                let lw = (stroke_width * options.zoom).round().max(1.0) as i32;
                for stroke in strokes {
                    for pair in stroke.windows(2) {
                        let (x0, y0) = to_scr(pair[0]);
                        let (x1, y1) = to_scr(pair[1]);
                        Self::draw_line(surface, surf_w, surf_h, x0, y0, x1, y1, col, lw);
                    }
                }
            }
            AnnotationType::Square { rect, stroke_color, fill_color, stroke_width } => {
                let (x1, y2) = to_scr(Point::new(rect.llx, rect.lly));
                let (x2, y1) = to_scr(Point::new(rect.urx, rect.ury));
                let rx = x1.min(x2);
                let ry = y1.min(y2);
                let rw = (x2 - x1).abs();
                let rh = (y2 - y1).abs();

                if let Some(fc) = fill_color {
                    let rgba = ColorRgb {
                        r: fc[0] as f64 / 255.0,
                        g: fc[1] as f64 / 255.0,
                        b: fc[2] as f64 / 255.0,
                        a: fc[3] as f64 / 255.0,
                    };
                    Self::blend_rect(surface, surf_w, surf_h, rx, ry, rw, rh, rgba);
                }
                let sc = ((stroke_color[3] as u32) << 24) | ((stroke_color[0] as u32) << 16) | ((stroke_color[1] as u32) << 8) | (stroke_color[2] as u32);
                let lw = (stroke_width * options.zoom).round().max(1.0) as i32;
                Self::draw_rect_outline(surface, surf_w, surf_h, rx, ry, rw, rh, sc, lw);
            }
            AnnotationType::Circle { rect, stroke_color, fill_color: _, stroke_width } => {
                let (x1, y2) = to_scr(Point::new(rect.llx, rect.lly));
                let (x2, y1) = to_scr(Point::new(rect.urx, rect.ury));
                let cx = (x1 + x2) / 2;
                let cy = (y1 + y2) / 2;
                let rx = (x2 - x1).abs() / 2;
                let ry = (y2 - y1).abs() / 2;

                let sc = ((stroke_color[3] as u32) << 24) | ((stroke_color[0] as u32) << 16) | ((stroke_color[1] as u32) << 8) | (stroke_color[2] as u32);
                let lw = (stroke_width * options.zoom).round().max(1.0) as i32;
                Self::draw_ellipse(surface, surf_w, surf_h, cx, cy, rx, ry, sc, lw);
            }
            AnnotationType::Line { start, end, color, stroke_width, has_arrow } => {
                let (x0, y0) = to_scr(*start);
                let (x1, y1) = to_scr(*end);
                let col = ((color[3] as u32) << 24) | ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | (color[2] as u32);
                let lw = (stroke_width * options.zoom).round().max(1.0) as i32;
                Self::draw_line(surface, surf_w, surf_h, x0, y0, x1, y1, col, lw);
                if *has_arrow {
                    // Draw arrowhead at end
                    let dx = (x1 - x0) as f64;
                    let dy = (y1 - y0) as f64;
                    let angle = dy.atan2(dx);
                    let arr_len = (12.0 * options.zoom).max(6.0);
                    let a1 = angle + std::f64::consts::PI * 0.85;
                    let a2 = angle - std::f64::consts::PI * 0.85;
                    let ax1 = x1 + (arr_len * a1.cos()).round() as i32;
                    let ay1 = y1 + (arr_len * a1.sin()).round() as i32;
                    let ax2 = x1 + (arr_len * a2.cos()).round() as i32;
                    let ay2 = y1 + (arr_len * a2.sin()).round() as i32;
                    Self::draw_line(surface, surf_w, surf_h, x1, y1, ax1, ay1, col, lw);
                    Self::draw_line(surface, surf_w, surf_h, x1, y1, ax2, ay2, col, lw);
                }
            }
            AnnotationType::FreeText { rect, text, font_size, text_color, bg_color } => {
                let (x1, y2) = to_scr(Point::new(rect.llx, rect.lly));
                let (x2, y1) = to_scr(Point::new(rect.urx, rect.ury));
                let rx = x1.min(x2);
                let ry = y1.min(y2);
                let rw = (x2 - x1).abs().max(10);
                let rh = (y2 - y1).abs().max(10);

                if let Some(bg) = bg_color {
                    let bg_rgba = ColorRgb {
                        r: bg[0] as f64 / 255.0,
                        g: bg[1] as f64 / 255.0,
                        b: bg[2] as f64 / 255.0,
                        a: bg[3] as f64 / 255.0,
                    };
                    Self::blend_rect(surface, surf_w, surf_h, rx, ry, rw, rh, bg_rgba);
                }
                Self::draw_rect_outline(surface, surf_w, surf_h, rx, ry, rw, rh, 0xFF38BDF8, 1);

                let tc = ((text_color[3] as u32) << 24) | ((text_color[0] as u32) << 16) | ((text_color[1] as u32) << 8) | (text_color[2] as u32);
                let fs_px = (font_size * options.zoom).round().max(8.0) as i32;
                let mut tx = rx + 4;
                for c in text.chars() {
                    Self::draw_bitmap_char(surface, surf_w, surf_h, tx, ry + 3, c, tc, fs_px);
                    tx += (fs_px * 6) / 10;
                }
            }
            AnnotationType::StickyNote { point, .. } => {
                let (nx, ny) = to_scr(*point);
                // Draw yellow note icon
                Self::fill_rect(surface, surf_w, surf_h, nx - 9, ny - 9, 18, 18, 0xFFFBBF24);
                Self::draw_rect_outline(surface, surf_w, surf_h, nx - 9, ny - 9, 18, 18, 0xFFD97706, 1);
                // Draw folded flap corner
                Self::fill_rect(surface, surf_w, surf_h, nx + 3, ny - 9, 6, 6, 0xFFF59E0B);
                // Mini horizontal lines
                Self::draw_line(surface, surf_w, surf_h, nx - 6, ny - 3, nx + 2, ny - 3, 0xFF78350F, 1);
                Self::draw_line(surface, surf_w, surf_h, nx - 6, ny + 1, nx + 5, ny + 1, 0xFF78350F, 1);
                Self::draw_line(surface, surf_w, surf_h, nx - 6, ny + 5, nx + 5, ny + 5, 0xFF78350F, 1);
            }
            AnnotationType::Stamp { rect, label, color, author: _, date: _ } => {
                let (x1, y2) = to_scr(Point::new(rect.llx, rect.lly));
                let (x2, y1) = to_scr(Point::new(rect.urx, rect.ury));
                let rx = x1.min(x2);
                let ry = y1.min(y2);
                let rw = (x2 - x1).abs().max(40);
                let rh = (y2 - y1).abs().max(20);

                let col = ((color[3] as u32) << 24) | ((color[0] as u32) << 16) | ((color[1] as u32) << 8) | (color[2] as u32);
                Self::draw_rect_outline(surface, surf_w, surf_h, rx, ry, rw, rh, col, 2);
                Self::draw_rect_outline(surface, surf_w, surf_h, rx + 3, ry + 3, rw - 6, rh - 6, col, 1);

                let fs_px = (rh as f64 * 0.45).clamp(8.0, 24.0) as i32;
                let mut tx = rx + 8;
                for c in label.chars() {
                    Self::draw_bitmap_char(surface, surf_w, surf_h, tx, ry + 6, c, col, fs_px);
                    tx += (fs_px * 6) / 10;
                }
            }
            AnnotationType::Redact { rect, .. } => {
                let (x1, y2) = to_scr(Point::new(rect.llx, rect.lly));
                let (x2, y1) = to_scr(Point::new(rect.urx, rect.ury));
                let rx = x1.min(x2);
                let ry = y1.min(y2);
                let rw = (x2 - x1).abs();
                let rh = (y2 - y1).abs();
                // Permanent black redaction block
                Self::fill_rect(surface, surf_w, surf_h, rx, ry, rw, rh, 0xFF000000);
            }
        }

        // Draw selection bounding outline and resize handles if annotation is active
        if annot.is_selected {
            let b = annot.bounding_rect();
            let (x1, y2) = to_scr(Point::new(b.llx, b.lly));
            let (x2, y1) = to_scr(Point::new(b.urx, b.ury));
            let rx = x1.min(x2) - 3;
            let ry = y1.min(y2) - 3;
            let rw = (x2 - x1).abs() + 6;
            let rh = (y2 - y1).abs() + 6;

            Self::draw_rect_outline(surface, surf_w, surf_h, rx, ry, rw, rh, 0xFF38BDF8, 1);
            // 4 corner handles
            Self::fill_rect(surface, surf_w, surf_h, rx - 3, ry - 3, 6, 6, 0xFF0284C7);
            Self::fill_rect(surface, surf_w, surf_h, rx + rw - 3, ry - 3, 6, 6, 0xFF0284C7);
            Self::fill_rect(surface, surf_w, surf_h, rx - 3, ry + rh - 3, 6, 6, 0xFF0284C7);
            Self::fill_rect(surface, surf_w, surf_h, rx + rw - 3, ry + rh - 3, 6, 6, 0xFF0284C7);
        }
    }

    // -----------------------------------------------------------------------
    // Low-Level Drawing Utilities
    // -----------------------------------------------------------------------

    pub fn fill_rect(surface: &mut [u32], sw: i32, sh: i32, x: i32, y: i32, w: i32, h: i32, col: u32) {
        let x0 = x.clamp(0, sw);
        let y0 = y.clamp(0, sh);
        let x1 = (x + w).clamp(0, sw);
        let y1 = (y + h).clamp(0, sh);

        for r in y0..y1 {
            let row_start = (r * sw) as usize;
            for c in x0..x1 {
                surface[row_start + c as usize] = col;
            }
        }
    }

    pub fn blend_rect(surface: &mut [u32], sw: i32, sh: i32, x: i32, y: i32, w: i32, h: i32, col: ColorRgb) {
        let x0 = x.clamp(0, sw);
        let y0 = y.clamp(0, sh);
        let x1 = (x + w).clamp(0, sw);
        let y1 = (y + h).clamp(0, sh);

        let alpha = col.a.clamp(0.0, 1.0);
        let inv_alpha = 1.0 - alpha;
        let cr = (col.r.clamp(0.0, 1.0) * 255.0) as f64;
        let cg = (col.g.clamp(0.0, 1.0) * 255.0) as f64;
        let cb = (col.b.clamp(0.0, 1.0) * 255.0) as f64;

        for r in y0..y1 {
            let row_start = (r * sw) as usize;
            for c in x0..x1 {
                let idx = row_start + c as usize;
                let bg = surface[idx];
                let br = ((bg >> 16) & 0xFF) as f64;
                let bg_g = ((bg >> 8) & 0xFF) as f64;
                let bb = (bg & 0xFF) as f64;

                let out_r = (cr * alpha + br * inv_alpha) as u32;
                let out_g = (cg * alpha + bg_g * inv_alpha) as u32;
                let out_b = (cb * alpha + bb * inv_alpha) as u32;

                surface[idx] = 0xFF000000 | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    }

    pub fn draw_rect_outline(
        surface: &mut [u32],
        sw: i32,
        sh: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        col: u32,
        thickness: i32,
    ) {
        Self::fill_rect(surface, sw, sh, x, y, w, thickness, col);
        Self::fill_rect(surface, sw, sh, x, y + h - thickness, w, thickness, col);
        Self::fill_rect(surface, sw, sh, x, y, thickness, h, col);
        Self::fill_rect(surface, sw, sh, x + w - thickness, y, thickness, h, col);
    }

    pub fn draw_line(
        surface: &mut [u32],
        sw: i32,
        sh: i32,
        mut x0: i32,
        mut y0: i32,
        x1: i32,
        y1: i32,
        col: u32,
        thickness: i32,
    ) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if thickness <= 1 {
                if x0 >= 0 && x0 < sw && y0 >= 0 && y0 < sh {
                    surface[(y0 * sw + x0) as usize] = col;
                }
            } else {
                let rad = thickness / 2;
                Self::fill_rect(surface, sw, sh, x0 - rad, y0 - rad, thickness, thickness, col);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn draw_ellipse(
        surface: &mut [u32],
        sw: i32,
        sh: i32,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        col: u32,
        thickness: i32,
    ) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let steps = 48;
        let mut prev_x = cx + rx;
        let mut prev_y = cy;

        for i in 1..=steps {
            let theta = (i as f64 * std::f64::consts::TAU) / steps as f64;
            let cur_x = cx + (rx as f64 * theta.cos()).round() as i32;
            let cur_y = cy + (ry as f64 * theta.sin()).round() as i32;
            Self::draw_line(surface, sw, sh, prev_x, prev_y, cur_x, cur_y, col, thickness);
            prev_x = cur_x;
            prev_y = cur_y;
        }
    }

    pub fn draw_bitmap_char(
        surface: &mut [u32],
        sw: i32,
        sh: i32,
        x: i32,
        y: i32,
        c: char,
        col: u32,
        font_size: i32,
    ) {
        let ascii = (c as u32).min(127) as usize;
        if ascii < 32 {
            return;
        }
        let glyph_idx = ascii - 32;
        let glyph = FONT_GLYPHS[glyph_idx];

        let scale = (font_size as f64 / 8.0).max(1.0);

        for (col_idx, &col_byte) in glyph.iter().enumerate() {
            for row_idx in 0..7 {
                if (col_byte & (1 << row_idx)) != 0 {
                    let px = x + (col_idx as f64 * scale).round() as i32;
                    let py = y + (row_idx as f64 * scale).round() as i32;
                    let pw = scale.ceil() as i32;
                    let ph = scale.ceil() as i32;
                    Self::fill_rect(surface, sw, sh, px, py, pw, ph, col);
                }
            }
        }
    }
}

fn eval_cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;

    Point {
        x: uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
        y: uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
    }
}

static FONT_GLYPHS: [[u8; 5]; 96] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], // ' '
    [0x00, 0x00, 0x5F, 0x00, 0x00], // '!'
    [0x00, 0x07, 0x00, 0x07, 0x00], // '"'
    [0x14, 0x7F, 0x14, 0x7F, 0x14], // '#'
    [0x24, 0x2A, 0x7F, 0x2A, 0x12], // '$'
    [0x23, 0x13, 0x08, 0x64, 0x62], // '%'
    [0x36, 0x49, 0x55, 0x22, 0x50], // '&'
    [0x00, 0x05, 0x03, 0x00, 0x00], // '\''
    [0x00, 0x1C, 0x22, 0x41, 0x00], // '('
    [0x00, 0x41, 0x22, 0x1C, 0x00], // ')'
    [0x14, 0x08, 0x3E, 0x08, 0x14], // '*'
    [0x08, 0x08, 0x3E, 0x08, 0x08], // '+'
    [0x00, 0x50, 0x30, 0x00, 0x00], // ','
    [0x08, 0x08, 0x08, 0x08, 0x08], // '-'
    [0x00, 0x60, 0x60, 0x00, 0x00], // '.'
    [0x20, 0x10, 0x08, 0x04, 0x02], // '/'
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // '0'
    [0x00, 0x42, 0x7F, 0x40, 0x00], // '1'
    [0x42, 0x61, 0x51, 0x49, 0x46], // '2'
    [0x21, 0x41, 0x45, 0x4B, 0x31], // '3'
    [0x18, 0x14, 0x12, 0x7F, 0x10], // '4'
    [0x27, 0x45, 0x45, 0x45, 0x39], // '5'
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // '6'
    [0x01, 0x71, 0x09, 0x05, 0x03], // '7'
    [0x36, 0x49, 0x49, 0x49, 0x36], // '8'
    [0x06, 0x49, 0x49, 0x29, 0x1E], // '9'
    [0x00, 0x36, 0x36, 0x00, 0x00], // ':'
    [0x00, 0x56, 0x36, 0x00, 0x00], // ';'
    [0x08, 0x14, 0x22, 0x41, 0x00], // '<'
    [0x14, 0x14, 0x14, 0x14, 0x14], // '='
    [0x00, 0x41, 0x22, 0x14, 0x08], // '>'
    [0x02, 0x01, 0x51, 0x09, 0x06], // '?'
    [0x32, 0x49, 0x79, 0x41, 0x3E], // '@'
    [0x7E, 0x11, 0x11, 0x11, 0x7E], // 'A'
    [0x7F, 0x49, 0x49, 0x49, 0x36], // 'B'
    [0x3E, 0x41, 0x41, 0x41, 0x22], // 'C'
    [0x7F, 0x41, 0x41, 0x22, 0x1C], // 'D'
    [0x7F, 0x49, 0x49, 0x49, 0x41], // 'E'
    [0x7F, 0x09, 0x09, 0x09, 0x01], // 'F'
    [0x3E, 0x41, 0x49, 0x49, 0x7A], // 'G'
    [0x7F, 0x08, 0x08, 0x08, 0x7F], // 'H'
    [0x00, 0x41, 0x7F, 0x41, 0x00], // 'I'
    [0x20, 0x40, 0x41, 0x3F, 0x01], // 'J'
    [0x7F, 0x08, 0x14, 0x22, 0x41], // 'K'
    [0x7F, 0x40, 0x40, 0x40, 0x40], // 'L'
    [0x7F, 0x02, 0x0C, 0x02, 0x7F], // 'M'
    [0x7F, 0x04, 0x08, 0x10, 0x7F], // 'N'
    [0x3E, 0x41, 0x41, 0x41, 0x3E], // 'O'
    [0x7F, 0x09, 0x09, 0x09, 0x06], // 'P'
    [0x3E, 0x41, 0x51, 0x21, 0x5E], // 'Q'
    [0x7F, 0x09, 0x19, 0x29, 0x46], // 'R'
    [0x46, 0x49, 0x49, 0x49, 0x31], // 'S'
    [0x01, 0x01, 0x7F, 0x01, 0x01], // 'T'
    [0x3F, 0x40, 0x40, 0x40, 0x3F], // 'U'
    [0x1F, 0x20, 0x40, 0x20, 0x1F], // 'V'
    [0x3F, 0x40, 0x38, 0x40, 0x3F], // 'W'
    [0x63, 0x14, 0x08, 0x14, 0x63], // 'X'
    [0x07, 0x08, 0x70, 0x08, 0x07], // 'Y'
    [0x61, 0x51, 0x49, 0x45, 0x43], // 'Z'
    [0x00, 0x7F, 0x41, 0x41, 0x00], // '['
    [0x02, 0x04, 0x08, 0x10, 0x20], // '\'
    [0x00, 0x41, 0x41, 0x7F, 0x00], // ']'
    [0x04, 0x02, 0x01, 0x02, 0x04], // '^'
    [0x40, 0x40, 0x40, 0x40, 0x40], // '_'
    [0x00, 0x01, 0x02, 0x04, 0x00], // '`'
    [0x20, 0x54, 0x54, 0x54, 0x78], // 'a'
    [0x7F, 0x48, 0x44, 0x44, 0x38], // 'b'
    [0x38, 0x44, 0x44, 0x44, 0x20], // 'c'
    [0x38, 0x44, 0x44, 0x48, 0x7F], // 'd'
    [0x38, 0x54, 0x54, 0x54, 0x18], // 'e'
    [0x08, 0x7E, 0x09, 0x01, 0x02], // 'f'
    [0x0C, 0x52, 0x52, 0x52, 0x3E], // 'g'
    [0x7F, 0x08, 0x04, 0x04, 0x78], // 'h'
    [0x00, 0x44, 0x7D, 0x40, 0x00], // 'i'
    [0x20, 0x40, 0x44, 0x3D, 0x00], // 'j'
    [0x7F, 0x10, 0x28, 0x44, 0x00], // 'k'
    [0x00, 0x41, 0x7F, 0x40, 0x00], // 'l'
    [0x7C, 0x04, 0x18, 0x04, 0x78], // 'm'
    [0x7C, 0x08, 0x04, 0x04, 0x78], // 'n'
    [0x38, 0x44, 0x44, 0x44, 0x38], // 'o'
    [0x7C, 0x14, 0x14, 0x14, 0x08], // 'p'
    [0x08, 0x14, 0x14, 0x18, 0x7C], // 'q'
    [0x7C, 0x08, 0x04, 0x04, 0x08], // 'r'
    [0x48, 0x54, 0x54, 0x54, 0x20], // 's'
    [0x04, 0x3F, 0x44, 0x40, 0x20], // 't'
    [0x3C, 0x40, 0x40, 0x20, 0x7C], // 'u'
    [0x1C, 0x20, 0x40, 0x20, 0x1C], // 'v'
    [0x3C, 0x40, 0x30, 0x40, 0x3C], // 'w'
    [0x44, 0x28, 0x10, 0x28, 0x44], // 'x'
    [0x0C, 0x50, 0x50, 0x50, 0x3C], // 'y'
    [0x44, 0x64, 0x54, 0x4C, 0x44], // 'z'
    [0x00, 0x08, 0x36, 0x41, 0x00], // '{'
    [0x00, 0x00, 0x7F, 0x00, 0x00], // '|'
    [0x00, 0x41, 0x36, 0x08, 0x00], // '}'
    [0x08, 0x08, 0x2A, 0x1C, 0x08], // '~'
    [0x7F, 0x7F, 0x7F, 0x7F, 0x7F], // del
];
