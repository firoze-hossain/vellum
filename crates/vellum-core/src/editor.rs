//! PDF Document Editor: In-place text modification, annotation editing, and page manipulation.

use crate::annotations::{Annotation, AnnotationType};
use crate::content::{ColorRgb, TextSpan};
use crate::document::{DocumentMetadata, PdfDocument, PdfPage};
use crate::types::{PdfRect, Point};

/// Editing commands for Undo / Redo history.
#[derive(Debug, Clone)]
pub enum EditCommand {
    AddAnnotation { page_idx: usize, annot: Annotation },
    RemoveAnnotation { page_idx: usize, annot: Annotation },
    ModifyText { page_idx: usize, span_idx: usize, old_text: String, new_text: String },
    InsertText { page_idx: usize, span: TextSpan },
    RotatePage { page_idx: usize, clockwise: bool },
    InsertPage { page_idx: usize, page: PdfPage },
    DeletePage { page_idx: usize, page: PdfPage },
}

/// Interactive Document Editor managing document mutation and history state.
pub struct DocumentEditor {
    pub document: PdfDocument,
    pub undo_stack: Vec<EditCommand>,
    pub redo_stack: Vec<EditCommand>,
    pub is_dirty: bool,
    next_annot_id: u32,
}

impl DocumentEditor {
    pub fn new(document: PdfDocument) -> Self {
        let mut max_id = 0;
        for p in &document.pages {
            for a in &p.annotations {
                max_id = max_id.max(a.id);
            }
        }
        Self {
            document,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_dirty: false,
            next_annot_id: max_id + 1,
        }
    }

    pub fn document(&self) -> &PdfDocument {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut PdfDocument {
        self.is_dirty = true;
        &mut self.document
    }

    // -----------------------------------------------------------------------
    // Annotation Operations
    // -----------------------------------------------------------------------

    pub fn add_annotation(&mut self, page_idx: usize, annot_type: AnnotationType) -> Option<u32> {
        let id = self.next_annot_id;
        self.next_annot_id += 1;

        let annot = Annotation::new(id, page_idx, annot_type);
        if let Some(page) = self.document.pages.get_mut(page_idx) {
            page.annotations.push(annot.clone());
            self.undo_stack.push(EditCommand::AddAnnotation {
                page_idx,
                annot,
            });
            self.redo_stack.clear();
            self.is_dirty = true;
            Some(id)
        } else {
            None
        }
    }

    pub fn remove_annotation(&mut self, page_idx: usize, annot_id: u32) -> bool {
        if let Some(page) = self.document.pages.get_mut(page_idx) {
            if let Some(pos) = page.annotations.iter().position(|a| a.id == annot_id) {
                let removed = page.annotations.remove(pos);
                self.undo_stack.push(EditCommand::RemoveAnnotation {
                    page_idx,
                    annot: removed,
                });
                self.redo_stack.clear();
                self.is_dirty = true;
                return true;
            }
        }
        false
    }

    pub fn select_annotation_at(&mut self, page_idx: usize, pt: Point) -> Option<u32> {
        if let Some(page) = self.document.pages.get_mut(page_idx) {
            let mut selected_id = None;
            for a in page.annotations.iter_mut().rev() {
                if a.bounding_rect().contains(pt) {
                    a.is_selected = true;
                    selected_id = Some(a.id);
                    break;
                } else {
                    a.is_selected = false;
                }
            }
            selected_id
        } else {
            None
        }
    }

    pub fn clear_selection(&mut self) {
        for page in &mut self.document.pages {
            for a in &mut page.annotations {
                a.is_selected = false;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Direct Text Editing Operations (Adobe Acrobat style)
    // -----------------------------------------------------------------------

    pub fn edit_text_span(&mut self, page_idx: usize, span_idx: usize, new_text: &str) -> bool {
        if let Some(page) = self.document.pages.get_mut(page_idx) {
            if let Some(span) = page.display_list.text_spans.get_mut(span_idx) {
                let old_text = span.text.clone();
                span.text = new_text.to_string();

                let font = crate::font::StandardFont::from_name(&span.font_name);
                span.width = font.measure_text(new_text, span.font_size);

                self.undo_stack.push(EditCommand::ModifyText {
                    page_idx,
                    span_idx,
                    old_text,
                    new_text: new_text.to_string(),
                });
                self.redo_stack.clear();
                self.is_dirty = true;
                return true;
            }
        }
        false
    }

    pub fn insert_text(
        &mut self,
        page_idx: usize,
        text: &str,
        font_name: &str,
        font_size: f64,
        pos: Point,
        color: ColorRgb,
    ) -> bool {
        let font = crate::font::StandardFont::from_name(font_name);
        let width = font.measure_text(text, font_size);

        let span = TextSpan {
            text: text.to_string(),
            font_name: font_name.to_string(),
            font_size,
            x: pos.x,
            y: pos.y,
            width,
            height: font_size,
            color,
        };

        if let Some(page) = self.document.pages.get_mut(page_idx) {
            page.display_list.text_spans.push(span.clone());
            self.undo_stack.push(EditCommand::InsertText { page_idx, span });
            self.redo_stack.clear();
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    /// Executes permanent redaction by blacking out text and removing it from page.
    pub fn apply_redactions(&mut self, page_idx: usize) -> usize {
        let mut count = 0;
        if let Some(page) = self.document.pages.get_mut(page_idx) {
            let redact_rects: Vec<PdfRect> = page
                .annotations
                .iter()
                .filter_map(|a| match a.annot_type {
                    AnnotationType::Redact { rect, .. } => Some(rect),
                    _ => None,
                })
                .collect();

            if !redact_rects.is_empty() {
                // Permanently delete any text span intersecting redaction rects
                page.display_list.text_spans.retain(|span| {
                    let s_rect = span.bounding_rect();
                    let hit = redact_rects.iter().any(|r| r.intersects(&s_rect));
                    if hit {
                        count += 1;
                    }
                    !hit
                });

                // Convert redaction annotations to solid black paths
                for r in &redact_rects {
                    page.display_list.paths.push(crate::content::VectorPath {
                        segments: vec![
                            crate::content::PathSegment::MoveTo(Point::new(r.llx, r.lly)),
                            crate::content::PathSegment::LineTo(Point::new(r.urx, r.lly)),
                            crate::content::PathSegment::LineTo(Point::new(r.urx, r.ury)),
                            crate::content::PathSegment::LineTo(Point::new(r.llx, r.ury)),
                            crate::content::PathSegment::Close,
                        ],
                        stroke_color: Some(ColorRgb::BLACK),
                        fill_color: Some(ColorRgb::BLACK),
                        line_width: 1.0,
                    });
                }

                // Remove the redaction annotations since they are now baked into the page
                page.annotations.retain(|a| !matches!(a.annot_type, AnnotationType::Redact { .. }));
                self.is_dirty = true;
            }
        }
        count
    }

    // -----------------------------------------------------------------------
    // Page Management (macOS Preview / Adobe Acrobat)
    // -----------------------------------------------------------------------

    pub fn insert_blank_page(&mut self, after_idx: usize, width: f64, height: f64) {
        let num = after_idx + 2;
        let page = PdfPage::new(num, PdfRect::new(0.0, 0.0, width, height));
        let insert_pos = (after_idx + 1).min(self.document.pages.len());
        self.document.pages.insert(insert_pos, page.clone());

        // Renumber pages
        for (i, p) in self.document.pages.iter_mut().enumerate() {
            p.page_num = i + 1;
        }

        self.undo_stack.push(EditCommand::InsertPage {
            page_idx: insert_pos,
            page,
        });
        self.redo_stack.clear();
        self.is_dirty = true;
    }

    pub fn delete_page(&mut self, page_idx: usize) -> bool {
        if self.document.pages.len() > 1 && page_idx < self.document.pages.len() {
            let removed = self.document.pages.remove(page_idx);
            for (i, p) in self.document.pages.iter_mut().enumerate() {
                p.page_num = i + 1;
            }
            self.undo_stack.push(EditCommand::DeletePage {
                page_idx,
                page: removed,
            });
            self.redo_stack.clear();
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    pub fn rotate_page(&mut self, page_idx: usize, clockwise: bool) {
        self.document.rotate_page(page_idx, clockwise);
        self.undo_stack.push(EditCommand::RotatePage {
            page_idx,
            clockwise,
        });
        self.redo_stack.clear();
        self.is_dirty = true;
    }

    pub fn set_metadata(&mut self, meta: DocumentMetadata) {
        self.document.metadata = meta;
        self.is_dirty = true;
    }

    // -----------------------------------------------------------------------
    // Undo / Redo
    // -----------------------------------------------------------------------

    pub fn undo(&mut self) -> bool {
        if let Some(cmd) = self.undo_stack.pop() {
            match &cmd {
                EditCommand::AddAnnotation { page_idx, annot } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        page.annotations.retain(|a| a.id != annot.id);
                    }
                }
                EditCommand::RemoveAnnotation { page_idx, annot } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        page.annotations.push(annot.clone());
                    }
                }
                EditCommand::ModifyText { page_idx, span_idx, old_text, .. } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        if let Some(span) = page.display_list.text_spans.get_mut(*span_idx) {
                            span.text = old_text.clone();
                        }
                    }
                }
                EditCommand::InsertText { page_idx, .. } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        page.display_list.text_spans.pop();
                    }
                }
                EditCommand::RotatePage { page_idx, clockwise } => {
                    self.document.rotate_page(*page_idx, !*clockwise);
                }
                EditCommand::InsertPage { page_idx, .. } => {
                    self.document.pages.remove(*page_idx);
                }
                EditCommand::DeletePage { page_idx, page } => {
                    self.document.pages.insert(*page_idx, page.clone());
                }
            }
            self.redo_stack.push(cmd);
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            match &cmd {
                EditCommand::AddAnnotation { page_idx, annot } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        page.annotations.push(annot.clone());
                    }
                }
                EditCommand::RemoveAnnotation { page_idx, annot } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        page.annotations.retain(|a| a.id != annot.id);
                    }
                }
                EditCommand::ModifyText { page_idx, span_idx, new_text, .. } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        if let Some(span) = page.display_list.text_spans.get_mut(*span_idx) {
                            span.text = new_text.clone();
                        }
                    }
                }
                EditCommand::InsertText { page_idx, span } => {
                    if let Some(page) = self.document.pages.get_mut(*page_idx) {
                        page.display_list.text_spans.push(span.clone());
                    }
                }
                EditCommand::RotatePage { page_idx, clockwise } => {
                    self.document.rotate_page(*page_idx, *clockwise);
                }
                EditCommand::InsertPage { page_idx, page } => {
                    self.document.pages.insert(*page_idx, page.clone());
                }
                EditCommand::DeletePage { page_idx, .. } => {
                    self.document.pages.remove(*page_idx);
                }
            }
            self.undo_stack.push(cmd);
            self.is_dirty = true;
            true
        } else {
            false
        }
    }
}
