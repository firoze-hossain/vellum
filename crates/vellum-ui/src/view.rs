//! VellumView: Main document viewport, canvas renderer, click dispatch, and editing coordinator.

use crate::components::about_dialog::render_about_dialog;
use crate::components::inspector::render_inspector;
use crate::components::markup_bar::{render_markup_bar, HighlightColor, MarkupTool};
use crate::components::ribbon::{render_ribbon, DocumentMode};
use crate::components::search_bar::render_search_bar;
use crate::components::sidebar::{render_sidebar, SidebarTab};
use crate::components::status_bar::{render_status_bar, PageLayoutMode};
use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};
use vellum_core::annotations::AnnotationType;
use vellum_core::document::PdfDocument;
use vellum_core::editor::DocumentEditor;
use vellum_core::render::{PageRenderer, RenderOptions};
use vellum_core::search::SearchEngine;
use vellum_core::types::{PdfRect, Point};

pub struct VellumView {
    pub editor: DocumentEditor,
    pub current_page_idx: usize,
    pub mode: DocumentMode,
    pub active_tool: MarkupTool,
    pub active_color: HighlightColor,
    pub layout_mode: PageLayoutMode,
    pub zoom: f64,
    pub scroll_y: i32,
    pub sidebar_open: bool,
    pub sidebar_tab: SidebarTab,
    pub inspector_open: bool,
    pub search_open: bool,
    pub night_mode: bool,
    pub show_about: bool,
    pub search_engine: SearchEngine,
    pub selected_annot_id: Option<u32>,
    pub inline_editing_span: Option<(usize, String)>, // (span_idx, buffer)
    pub drag_start_pt: Option<Point>,
    pub cur_ink_stroke: Vec<Point>,
}

impl VellumView {
    pub fn new(doc: PdfDocument) -> Self {
        Self {
            editor: DocumentEditor::new(doc),
            current_page_idx: 0,
            mode: DocumentMode::Read,
            active_tool: MarkupTool::Select,
            active_color: HighlightColor::Yellow,
            layout_mode: PageLayoutMode::Single,
            zoom: 1.0,
            scroll_y: 0,
            sidebar_open: true,
            sidebar_tab: SidebarTab::Thumbnails,
            inspector_open: false,
            search_open: false,
            night_mode: false,
            show_about: false,
            search_engine: SearchEngine::new(),
            selected_annot_id: None,
            inline_editing_span: None,
            drag_start_pt: None,
            cur_ink_stroke: Vec::new(),
        }
    }

    pub fn document(&self) -> &PdfDocument {
        self.editor.document()
    }

    pub fn set_mode(&mut self, mode: DocumentMode) {
        self.mode = mode;
    }

    pub fn set_markup_tool(&mut self, tool: MarkupTool) {
        self.active_tool = tool;
    }

    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(0.25, 4.0);
    }

    pub fn zoom_in(&mut self) {
        self.set_zoom(self.zoom + 0.15);
    }

    pub fn zoom_out(&mut self) {
        self.set_zoom(self.zoom - 0.15);
    }

    pub fn next_page(&mut self) {
        if self.current_page_idx + 1 < self.editor.document.pages.len() {
            self.current_page_idx += 1;
            self.editor.clear_selection();
            self.selected_annot_id = None;
        }
    }

    pub fn prev_page(&mut self) {
        if self.current_page_idx > 0 {
            self.current_page_idx -= 1;
            self.editor.clear_selection();
            self.selected_annot_id = None;
        }
    }

    pub fn jump_to_page(&mut self, page_idx: usize) {
        if page_idx < self.editor.document.pages.len() {
            self.current_page_idx = page_idx;
            self.editor.clear_selection();
            self.selected_annot_id = None;
        }
    }

    // -----------------------------------------------------------------------
    // Master Render Method
    // -----------------------------------------------------------------------

    pub fn render(&mut self, surface: &mut [u32], sw: i32, sh: i32) {
        if sw <= 0 || sh <= 0 {
            return;
        }

        // 1. Fill base dark desktop backdrop (#0B0F19)
        fill_rect(surface, sw, sh, Rect::new(0, 0, sw, sh), Color::rgb(11, 15, 25));

        // 2. Render Top Ribbon
        let ribbon_layout = render_ribbon(
            surface,
            sw,
            sh,
            self.mode,
            &self.editor.document.metadata.title,
            self.editor.is_dirty,
            self.sidebar_open,
            self.inspector_open,
            self.night_mode,
            self.search_open,
        );

        let mut top_y = ribbon_layout.bounds.bottom();

        // 3. Render Markup Bar (if in Annotate Mode)
        let _markup_layout = if self.mode == DocumentMode::Annotate {
            let m_layout = render_markup_bar(
                surface,
                sw,
                sh,
                top_y,
                self.active_tool,
                self.active_color,
            );
            top_y = m_layout.bounds.bottom();
            Some(m_layout)
        } else {
            None
        };

        let bot_bar_h = 30;
        let content_bot_y = sh - bot_bar_h;

        // 4. Calculate Layout Geometry
        let sidebar_w = if self.sidebar_open { 190.min(sw / 3) } else { 0 };
        let inspector_w = if self.inspector_open { 210.min(sw / 3) } else { 0 };

        let canvas_x = sidebar_w;
        let canvas_w = sw - sidebar_w - inspector_w;
        let canvas_h = content_bot_y - top_y;

        // 5. Render Left Sidebar
        if self.sidebar_open {
            render_sidebar(
                surface,
                sw,
                sh,
                top_y,
                content_bot_y,
                sidebar_w,
                self.sidebar_tab,
                self.current_page_idx,
                self.editor.document(),
            );
        }

        // 6. Render Right Inspector
        if self.inspector_open {
            let selected_annot = self.selected_annot_id.and_then(|id| {
                self.editor.document.pages.get(self.current_page_idx)?
                    .annotations.iter().find(|a| a.id == id)
            });
            render_inspector(
                surface,
                sw,
                sh,
                top_y,
                content_bot_y,
                inspector_w,
                self.current_page_idx,
                selected_annot,
                self.editor.document(),
            );
        }

        // 7. Render PDF Page Canvas (Centered inside viewport)
        if let Some(page) = self.editor.document.pages.get(self.current_page_idx) {
            let page_w_px = (page.width() * self.zoom).round() as i32;
            let page_h_px = (page.height() * self.zoom).round() as i32;

            // Center page on canvas area
            let page_x = canvas_x + ((canvas_w - page_w_px) / 2).max(16);
            let page_y = top_y + ((canvas_h - page_h_px) / 2).max(16) - self.scroll_y;

            // Paper shadow and outline
            let page_rect = Rect::new(page_x, page_y, page_w_px, page_h_px);
            fill_rounded_rect(surface, sw, sh, Rect::new(page_x + 3, page_y + 4, page_w_px, page_h_px), 4, Color::rgba(0, 0, 0, 70));
            fill_rect(surface, sw, sh, page_rect, Color::WHITE);
            draw_rect_outline(surface, sw, sh, page_rect, 1, Color::rgb(71, 85, 105));

            // Software rasterize PDF page display list directly onto page surface slice
            let mut page_buffer = vec![0xFFFFFFFFu32; (page_w_px * page_h_px).max(1) as usize];
            let render_opts = RenderOptions {
                zoom: self.zoom,
                rotation: page.rotation,
                invert_colors: self.night_mode,
                render_annotations: true,
            };

            let page_hits = self.search_engine.hits_for_page(self.current_page_idx);
            let active_hit = self.search_engine.active_hit().and_then(|h| {
                if h.page_index == self.current_page_idx {
                    self.search_engine.active_hit_index
                } else {
                    None
                }
            });

            PageRenderer::render_page(
                &mut page_buffer,
                page_w_px,
                page_h_px,
                page.media_box,
                &page.display_list,
                &page.annotations,
                &page_hits,
                active_hit,
                &render_opts,
            );

            // Blit rendered page into window surface
            for row in 0..page_h_px {
                let dst_y = page_y + row;
                if dst_y >= top_y && dst_y < content_bot_y && dst_y < sh {
                    for col in 0..page_w_px {
                        let dst_x = page_x + col;
                        if dst_x >= canvas_x && dst_x < canvas_x + canvas_w && dst_x < sw {
                            let src_pixel = page_buffer[(row * page_w_px + col) as usize];
                            surface[(dst_y * sw + dst_x) as usize] = src_pixel;
                        }
                    }
                }
            }

            // Draw inline text editor cursor / input box if active
            if let Some((span_idx, ref edit_buf)) = self.inline_editing_span {
                if let Some(span) = page.display_list.text_spans.get(span_idx) {
                    let (sx, sy) = self.pdf_to_screen(Point::new(span.x, span.y), page.height(), page_x, page_y);
                    let sw_box = (span.width * self.zoom).round().max(40.0) as i32 + 12;
                    let sh_box = (span.height * self.zoom).round().max(16.0) as i32 + 4;
                    let edit_rect = Rect::new(sx - 2, sy - sh_box + 2, sw_box, sh_box);

                    fill_rounded_rect(surface, sw, sh, edit_rect, 3, Color::rgba(14, 165, 233, 50));
                    draw_rect_outline(surface, sw, sh, edit_rect, 1, Color::rgb(56, 189, 248));
                    draw_text(surface, sw, sh, sx + 2, sy - sh_box + 4, edit_buf, Color::rgb(14, 165, 233), 1);
                }
            }
        }

        // 8. Render Evince-Style Floating Search Bar (if active)
        if self.search_open {
            render_search_bar(surface, sw, sh, top_y + 10, &self.search_engine);
        }

        // 9. Render Bottom Status Bar
        let (cur_w, cur_h) = self.editor.document.pages.get(self.current_page_idx)
            .map(|p| (p.width(), p.height()))
            .unwrap_or((612.0, 792.0));

        render_status_bar(
            surface,
            sw,
            sh,
            self.current_page_idx,
            self.editor.document.pages.len(),
            (self.zoom * 100.0).round() as u32,
            self.layout_mode,
            cur_w,
            cur_h,
        );

        // 10. Render About Dialog (if active)
        if self.show_about {
            render_about_dialog(surface, sw, sh);
        }
    }

    // -----------------------------------------------------------------------
    // Click & Hit-Test Dispatch
    // -----------------------------------------------------------------------

    pub fn handle_click(&mut self, px: i32, py: i32, sw: i32, sh: i32) {
        // 1. If About dialog is active, dismiss on click
        if self.show_about {
            self.show_about = false;
            return;
        }

        let ribbon_h = 44;
        let mut top_y = ribbon_h;

        // 2. Click in Top Ribbon (y in 0..44)
        if py < ribbon_h {
            let ribbon = render_ribbon(
                &mut [],
                sw,
                sh,
                self.mode,
                &self.editor.document.metadata.title,
                self.editor.is_dirty,
                self.sidebar_open,
                self.inspector_open,
                self.night_mode,
                self.search_open,
            );

            if ribbon.tab_read.contains(px, py) {
                self.mode = DocumentMode::Read;
                self.active_tool = MarkupTool::Select;
                return;
            }
            if ribbon.tab_annotate.contains(px, py) {
                self.mode = DocumentMode::Annotate;
                self.active_tool = MarkupTool::Highlight;
                return;
            }
            if ribbon.tab_edit.contains(px, py) {
                self.mode = DocumentMode::Edit;
                return;
            }
            if ribbon.tab_form.contains(px, py) {
                self.mode = DocumentMode::Form;
                return;
            }
            if ribbon.btn_open.contains(px, py) {
                // Open new sample document
                return;
            }
            if ribbon.btn_save.contains(px, py) {
                self.editor.is_dirty = false;
                return;
            }
            if ribbon.btn_sidebar.contains(px, py) {
                self.sidebar_open = !self.sidebar_open;
                return;
            }
            if ribbon.btn_search.contains(px, py) {
                self.search_open = !self.search_open;
                return;
            }
            if ribbon.btn_night.contains(px, py) {
                self.night_mode = !self.night_mode;
                return;
            }
            if ribbon.btn_inspector.contains(px, py) {
                self.inspector_open = !self.inspector_open;
                return;
            }
            if ribbon.btn_about.contains(px, py) {
                self.show_about = true;
                return;
            }
            return;
        }

        // 3. Click in Markup Bar (if in Annotate Mode)
        if self.mode == DocumentMode::Annotate && py < ribbon_h + 36 {
            let markup = render_markup_bar(
                &mut [],
                sw,
                sh,
                top_y,
                self.active_tool,
                self.active_color,
            );

            for &(t, r) in &markup.tool_buttons {
                if r.contains(px, py) {
                    self.active_tool = t;
                    return;
                }
            }
            for &(c, r) in &markup.color_swatches {
                if r.contains(px, py) {
                    self.active_color = c;
                    return;
                }
            }
            return;
        }

        if self.mode == DocumentMode::Annotate {
            top_y += 36;
        }

        // 4. Click in Search Bar (if search open)
        if self.search_open && py >= top_y && py <= top_y + 46 && px >= sw - 380 {
            let sb = render_search_bar(&mut [], sw, sh, top_y + 10, &self.search_engine);
            if sb.btn_next.contains(px, py) {
                if let Some(hit) = self.search_engine.next_hit() {
                    self.current_page_idx = hit.page_index;
                }
                return;
            }
            if sb.btn_prev.contains(px, py) {
                if let Some(hit) = self.search_engine.prev_hit() {
                    self.current_page_idx = hit.page_index;
                }
                return;
            }
            if sb.btn_case.contains(px, py) {
                self.search_engine.case_sensitive = !self.search_engine.case_sensitive;
                let q = self.search_engine.query.clone();
                let cs = self.search_engine.case_sensitive;
                self.search_engine.search(&self.editor.document, &q, cs);
                return;
            }
            if sb.btn_close.contains(px, py) {
                self.search_open = false;
                return;
            }
        }

        let bot_bar_h = 30;
        let content_bot_y = sh - bot_bar_h;

        // 5. Click in Bottom Status Bar
        if py >= content_bot_y {
            let sb = render_status_bar(&mut [], sw, sh, self.current_page_idx, self.editor.document.pages.len(), (self.zoom * 100.0) as u32, self.layout_mode, 612.0, 792.0);
            if sb.btn_prev_page.contains(px, py) {
                self.prev_page();
                return;
            }
            if sb.btn_next_page.contains(px, py) {
                self.next_page();
                return;
            }
            if sb.btn_zoom_in.contains(px, py) {
                self.zoom_in();
                return;
            }
            if sb.btn_zoom_out.contains(px, py) {
                self.zoom_out();
                return;
            }
            if sb.btn_fit_page.contains(px, py) {
                self.zoom = 1.0;
                return;
            }
            if sb.btn_fit_width.contains(px, py) {
                self.zoom = 1.25;
                return;
            }
            if sb.btn_single.contains(px, py) {
                self.layout_mode = PageLayoutMode::Single;
                return;
            }
            if sb.btn_facing.contains(px, py) {
                self.layout_mode = PageLayoutMode::Facing;
                return;
            }
            if sb.btn_continuous.contains(px, py) {
                self.layout_mode = PageLayoutMode::Continuous;
                return;
            }
            return;
        }

        // 6. Click in Left Sidebar
        let sidebar_w = if self.sidebar_open { 190.min(sw / 3) } else { 0 };
        if self.sidebar_open && px < sidebar_w {
            let sb = render_sidebar(&mut [], sw, sh, top_y, content_bot_y, sidebar_w, self.sidebar_tab, self.current_page_idx, self.editor.document());
            if sb.tab_thumbnails.contains(px, py) {
                self.sidebar_tab = SidebarTab::Thumbnails;
                return;
            }
            if sb.tab_outline.contains(px, py) {
                self.sidebar_tab = SidebarTab::Outline;
                return;
            }
            if sb.tab_annotations.contains(px, py) {
                self.sidebar_tab = SidebarTab::Annotations;
                return;
            }
            for &(page_idx, rect) in &sb.thumbnail_cards {
                if rect.contains(px, py) {
                    self.jump_to_page(page_idx);
                    return;
                }
            }
            for &(page_idx, rect) in &sb.outline_items {
                if rect.contains(px, py) {
                    self.jump_to_page(page_idx);
                    return;
                }
            }
            return;
        }

        // 7. Click in Right Inspector
        let inspector_w = if self.inspector_open { 210.min(sw / 3) } else { 0 };
        if self.inspector_open && px >= sw - inspector_w {
            let insp = render_inspector(&mut [], sw, sh, top_y, content_bot_y, inspector_w, self.current_page_idx, None, self.editor.document());
            if insp.btn_rotate_cw.contains(px, py) {
                self.editor.rotate_page(self.current_page_idx, true);
                return;
            }
            if insp.btn_rotate_ccw.contains(px, py) {
                self.editor.rotate_page(self.current_page_idx, false);
                return;
            }
            if insp.btn_insert_page.contains(px, py) {
                self.editor.insert_blank_page(self.current_page_idx, 612.0, 792.0);
                self.current_page_idx += 1;
                return;
            }
            if insp.btn_delete_page.contains(px, py) {
                self.editor.delete_page(self.current_page_idx);
                if self.current_page_idx >= self.editor.document.pages.len() {
                    self.current_page_idx = self.editor.document.pages.len().saturating_sub(1);
                }
                return;
            }
            return;
        }

        // 8. Click on Page Canvas (Annotations, Editing, and Shapes)
        if let Some(page) = self.editor.document.pages.get(self.current_page_idx) {
            let page_w_px = (page.width() * self.zoom).round() as i32;
            let page_h_px = (page.height() * self.zoom).round() as i32;
            let canvas_w = sw - sidebar_w - inspector_w;
            let page_x = sidebar_w + ((canvas_w - page_w_px) / 2).max(16);
            let page_y = top_y + ((content_bot_y - top_y - page_h_px) / 2).max(16) - self.scroll_y;

            let page_rect = Rect::new(page_x, page_y, page_w_px, page_h_px);
            if page_rect.contains(px, py) {
                let pdf_pt = self.screen_to_pdf(Point::new(px as f64, py as f64), page.height(), page_x, page_y);

                match self.mode {
                    DocumentMode::Read => {
                        // Click on sticky note or annotation opens properties
                        if let Some(id) = self.editor.select_annotation_at(self.current_page_idx, pdf_pt) {
                            self.selected_annot_id = Some(id);
                            self.inspector_open = true;
                        }
                    }
                    DocumentMode::Annotate => {
                        match self.active_tool {
                            MarkupTool::Select => {
                                self.selected_annot_id = self.editor.select_annotation_at(self.current_page_idx, pdf_pt);
                            }
                            MarkupTool::Highlight => {
                                // Add highlight annotation at click point
                                let h_rect = PdfRect::new(pdf_pt.x - 40.0, pdf_pt.y - 6.0, pdf_pt.x + 40.0, pdf_pt.y + 8.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Highlight {
                                        color: self.active_color.to_rgba(),
                                        rects: vec![h_rect],
                                    },
                                );
                            }
                            MarkupTool::Underline => {
                                let u_rect = PdfRect::new(pdf_pt.x - 40.0, pdf_pt.y - 2.0, pdf_pt.x + 40.0, pdf_pt.y + 10.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Underline {
                                        color: self.active_color.to_rgba(),
                                        rects: vec![u_rect],
                                    },
                                );
                            }
                            MarkupTool::StrikeOut => {
                                let s_rect = PdfRect::new(pdf_pt.x - 40.0, pdf_pt.y - 2.0, pdf_pt.x + 40.0, pdf_pt.y + 10.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::StrikeOut {
                                        color: self.active_color.to_rgba(),
                                        rects: vec![s_rect],
                                    },
                                );
                            }
                            MarkupTool::Rectangle => {
                                let r = PdfRect::new(pdf_pt.x - 30.0, pdf_pt.y - 20.0, pdf_pt.x + 30.0, pdf_pt.y + 20.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Square {
                                        rect: r,
                                        stroke_color: [14, 165, 233, 255],
                                        fill_color: None,
                                        stroke_width: 2.0,
                                    },
                                );
                            }
                            MarkupTool::Circle => {
                                let r = PdfRect::new(pdf_pt.x - 25.0, pdf_pt.y - 25.0, pdf_pt.x + 25.0, pdf_pt.y + 25.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Circle {
                                        rect: r,
                                        stroke_color: [244, 63, 94, 255],
                                        fill_color: None,
                                        stroke_width: 2.0,
                                    },
                                );
                            }
                            MarkupTool::Line | MarkupTool::Arrow => {
                                let has_arrow = self.active_tool == MarkupTool::Arrow;
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Line {
                                        start: Point::new(pdf_pt.x - 40.0, pdf_pt.y - 20.0),
                                        end: Point::new(pdf_pt.x + 40.0, pdf_pt.y + 20.0),
                                        color: [56, 189, 248, 255],
                                        stroke_width: 2.0,
                                        has_arrow,
                                    },
                                );
                            }
                            MarkupTool::TextBox => {
                                let r = PdfRect::new(pdf_pt.x, pdf_pt.y - 20.0, pdf_pt.x + 120.0, pdf_pt.y);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::FreeText {
                                        rect: r,
                                        text: "Enter text here...".to_string(),
                                        font_size: 12.0,
                                        text_color: [15, 23, 42, 255],
                                        bg_color: Some([255, 255, 255, 230]),
                                    },
                                );
                            }
                            MarkupTool::StickyNote => {
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::StickyNote {
                                        point: pdf_pt,
                                        author: "Reviewer".to_string(),
                                        text: "Needs clarification.".to_string(),
                                        timestamp: "Today".to_string(),
                                    },
                                );
                            }
                            MarkupTool::Stamp => {
                                let r = PdfRect::new(pdf_pt.x - 50.0, pdf_pt.y - 15.0, pdf_pt.x + 50.0, pdf_pt.y + 15.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Stamp {
                                        rect: r,
                                        label: "APPROVED".to_string(),
                                        color: [34, 197, 94, 255],
                                        author: "Auditor".to_string(),
                                        date: "2026-11-20".to_string(),
                                    },
                                );
                            }
                            MarkupTool::Redact => {
                                let r = PdfRect::new(pdf_pt.x - 40.0, pdf_pt.y - 10.0, pdf_pt.x + 40.0, pdf_pt.y + 10.0);
                                self.editor.add_annotation(
                                    self.current_page_idx,
                                    AnnotationType::Redact {
                                        rect: r,
                                        overlay_text: None,
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                    DocumentMode::Edit => {
                        // Adobe Acrobat style in-place text block selection
                        let mut hit_span = None;
                        for (idx, span) in page.display_list.text_spans.iter().enumerate() {
                            if span.bounding_rect().contains(pdf_pt) {
                                hit_span = Some((idx, span.text.clone()));
                                break;
                            }
                        }
                        if let Some((idx, txt)) = hit_span {
                            self.inline_editing_span = Some((idx, txt));
                        } else {
                            // If clicked on blank space, insert new paragraph
                            self.editor.insert_text(
                                self.current_page_idx,
                                "New paragraph added with Vellum.",
                                "Helvetica",
                                12.0,
                                pdf_pt,
                                vellum_core::content::ColorRgb::BLACK,
                            );
                        }
                    }
                    DocumentMode::Form => {}
                }
            }
        }
    }

    fn pdf_to_screen(&self, pt: Point, page_h: f64, page_x: i32, page_y: i32) -> (i32, i32) {
        let sx = page_x + (pt.x * self.zoom).round() as i32;
        let sy = page_y + ((page_h - pt.y) * self.zoom).round() as i32;
        (sx, sy)
    }

    fn screen_to_pdf(&self, pt: Point, page_h: f64, page_x: i32, page_y: i32) -> Point {
        let px = (pt.x - page_x as f64) / self.zoom;
        let py = page_h - ((pt.y - page_y as f64) / self.zoom);
        Point::new(px, py)
    }
}
