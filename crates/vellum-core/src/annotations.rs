//! PDF Annotations & Markup Model (macOS Preview + Adobe Acrobat).

use crate::types::{PdfObject, PdfRect, Point};
use std::collections::HashMap;

/// Supported annotation types reflecting the best of macOS Preview and Adobe Acrobat.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationType {
    /// Text highlight with translucent color fill
    Highlight {
        color: [u8; 4], // RGBA
        rects: Vec<PdfRect>,
    },
    /// Text underline markup
    Underline {
        color: [u8; 4],
        rects: Vec<PdfRect>,
    },
    /// Text strikethrough markup
    StrikeOut {
        color: [u8; 4],
        rects: Vec<PdfRect>,
    },
    /// Freehand vector ink sketch (pen / pencil)
    Ink {
        strokes: Vec<Vec<Point>>,
        color: [u8; 4],
        stroke_width: f64,
    },
    /// Geometric rectangle shape
    Square {
        rect: PdfRect,
        stroke_color: [u8; 4],
        fill_color: Option<[u8; 4]>,
        stroke_width: f64,
    },
    /// Geometric ellipse / circle shape
    Circle {
        rect: PdfRect,
        stroke_color: [u8; 4],
        fill_color: Option<[u8; 4]>,
        stroke_width: f64,
    },
    /// Geometric line or directional arrow
    Line {
        start: Point,
        end: Point,
        color: [u8; 4],
        stroke_width: f64,
        has_arrow: bool,
    },
    /// Free text box / typewriter text
    FreeText {
        rect: PdfRect,
        text: String,
        font_size: f64,
        text_color: [u8; 4],
        bg_color: Option<[u8; 4]>,
    },
    /// Sticky note comment icon with popup text
    StickyNote {
        point: Point,
        author: String,
        text: String,
        timestamp: String,
    },
    /// Status stamp or signature
    Stamp {
        rect: PdfRect,
        label: String, // "APPROVED", "CONFIDENTIAL", "DRAFT", or custom
        color: [u8; 4],
        author: String,
        date: String,
    },
    /// Security redaction (blackout box)
    Redact {
        rect: PdfRect,
        overlay_text: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub id: u32,
    pub page_index: usize,
    pub annot_type: AnnotationType,
    pub author: String,
    pub contents: String,
    pub is_selected: bool,
}

impl Annotation {
    pub fn new(id: u32, page_index: usize, annot_type: AnnotationType) -> Self {
        Self {
            id,
            page_index,
            annot_type,
            author: "NovaOS User".to_string(),
            contents: String::new(),
            is_selected: false,
        }
    }

    /// Returns the overall bounding rectangle of this annotation in PDF points.
    pub fn bounding_rect(&self) -> PdfRect {
        match &self.annot_type {
            AnnotationType::Highlight { rects, .. }
            | AnnotationType::Underline { rects, .. }
            | AnnotationType::StrikeOut { rects, .. } => {
                if let Some(first) = rects.first() {
                    let mut b = *first;
                    for r in &rects[1..] {
                        b.llx = b.llx.min(r.llx);
                        b.lly = b.lly.min(r.lly);
                        b.urx = b.urx.max(r.urx);
                        b.ury = b.ury.max(r.ury);
                    }
                    b
                } else {
                    PdfRect::default()
                }
            }
            AnnotationType::Ink { strokes, stroke_width, .. } => {
                let mut min_x = f64::MAX;
                let mut min_y = f64::MAX;
                let mut max_x = f64::MIN;
                let mut max_y = f64::MIN;
                for stroke in strokes {
                    for p in stroke {
                        min_x = min_x.min(p.x);
                        min_y = min_y.min(p.y);
                        max_x = max_x.max(p.x);
                        max_y = max_y.max(p.y);
                    }
                }
                let pad = *stroke_width;
                if min_x <= max_x {
                    PdfRect::new(min_x - pad, min_y - pad, max_x + pad, max_y + pad)
                } else {
                    PdfRect::default()
                }
            }
            AnnotationType::Square { rect, .. } | AnnotationType::Circle { rect, .. } => *rect,
            AnnotationType::Line { start, end, stroke_width, .. } => {
                let pad = *stroke_width + 4.0;
                PdfRect::new(
                    start.x.min(end.x) - pad,
                    start.y.min(end.y) - pad,
                    start.x.max(end.x) + pad,
                    start.y.max(end.y) + pad,
                )
            }
            AnnotationType::FreeText { rect, .. }
            | AnnotationType::Stamp { rect, .. }
            | AnnotationType::Redact { rect, .. } => *rect,
            AnnotationType::StickyNote { point, .. } => {
                PdfRect::new(point.x - 12.0, point.y - 12.0, point.x + 12.0, point.y + 12.0)
            }
        }
    }

    /// Converts this annotation into a standard conforming PDF dictionary.
    pub fn to_pdf_dict(&self) -> PdfObject {
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), PdfObject::Name("Annot".to_string()));
        dict.insert("Rect".to_string(), self.bounding_rect().to_obj());

        if !self.contents.is_empty() {
            dict.insert(
                "Contents".to_string(),
                PdfObject::String(self.contents.as_bytes().to_vec(), false),
            );
        }
        if !self.author.is_empty() {
            dict.insert(
                "T".to_string(),
                PdfObject::String(self.author.as_bytes().to_vec(), false),
            );
        }

        match &self.annot_type {
            AnnotationType::Highlight { color, rects } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Highlight".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*color));
                let mut quads = Vec::new();
                for r in rects {
                    quads.extend_from_slice(&[
                        PdfObject::Real(r.llx),
                        PdfObject::Real(r.ury),
                        PdfObject::Real(r.urx),
                        PdfObject::Real(r.ury),
                        PdfObject::Real(r.llx),
                        PdfObject::Real(r.lly),
                        PdfObject::Real(r.urx),
                        PdfObject::Real(r.lly),
                    ]);
                }
                dict.insert("QuadPoints".to_string(), PdfObject::Array(quads));
            }
            AnnotationType::Underline { color, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Underline".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*color));
            }
            AnnotationType::StrikeOut { color, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("StrikeOut".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*color));
            }
            AnnotationType::Ink { strokes, color, stroke_width } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Ink".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*color));
                let mut ink_list = Vec::new();
                for stroke in strokes {
                    let mut stroke_arr = Vec::new();
                    for p in stroke {
                        stroke_arr.push(PdfObject::Real(p.x));
                        stroke_arr.push(PdfObject::Real(p.y));
                    }
                    ink_list.push(PdfObject::Array(stroke_arr));
                }
                dict.insert("InkList".to_string(), PdfObject::Array(ink_list));
                let mut bs = HashMap::new();
                bs.insert("W".to_string(), PdfObject::Real(*stroke_width));
                dict.insert("BS".to_string(), PdfObject::Dictionary(bs));
            }
            AnnotationType::Square { stroke_color, fill_color, stroke_width, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Square".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*stroke_color));
                if let Some(fc) = fill_color {
                    dict.insert("IC".to_string(), color_to_pdf_array(*fc));
                }
                let mut bs = HashMap::new();
                bs.insert("W".to_string(), PdfObject::Real(*stroke_width));
                dict.insert("BS".to_string(), PdfObject::Dictionary(bs));
            }
            AnnotationType::Circle { stroke_color, fill_color, stroke_width, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Circle".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*stroke_color));
                if let Some(fc) = fill_color {
                    dict.insert("IC".to_string(), color_to_pdf_array(*fc));
                }
                let mut bs = HashMap::new();
                bs.insert("W".to_string(), PdfObject::Real(*stroke_width));
                dict.insert("BS".to_string(), PdfObject::Dictionary(bs));
            }
            AnnotationType::Line { start, end, color, stroke_width, has_arrow } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Line".to_string()));
                dict.insert("C".to_string(), color_to_pdf_array(*color));
                dict.insert(
                    "L".to_string(),
                    PdfObject::Array(vec![
                        PdfObject::Real(start.x),
                        PdfObject::Real(start.y),
                        PdfObject::Real(end.x),
                        PdfObject::Real(end.y),
                    ]),
                );
                if *has_arrow {
                    dict.insert(
                        "LE".to_string(),
                        PdfObject::Array(vec![
                            PdfObject::Name("None".to_string()),
                            PdfObject::Name("ClosedArrow".to_string()),
                        ]),
                    );
                }
                let mut bs = HashMap::new();
                bs.insert("W".to_string(), PdfObject::Real(*stroke_width));
                dict.insert("BS".to_string(), PdfObject::Dictionary(bs));
            }
            AnnotationType::FreeText { text, font_size, text_color, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("FreeText".to_string()));
                dict.insert("Contents".to_string(), PdfObject::String(text.as_bytes().to_vec(), false));
                dict.insert("C".to_string(), color_to_pdf_array(*text_color));
                let da = format!("/Helvetica {} Tf 0 g", font_size);
                dict.insert("DA".to_string(), PdfObject::String(da.as_bytes().to_vec(), false));
            }
            AnnotationType::StickyNote { text, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Text".to_string()));
                dict.insert("Contents".to_string(), PdfObject::String(text.as_bytes().to_vec(), false));
                dict.insert("Name".to_string(), PdfObject::Name("Comment".to_string()));
            }
            AnnotationType::Stamp { label, color, .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Stamp".to_string()));
                dict.insert("Name".to_string(), PdfObject::Name(label.clone()));
                dict.insert("C".to_string(), color_to_pdf_array(*color));
            }
            AnnotationType::Redact { .. } => {
                dict.insert("Subtype".to_string(), PdfObject::Name("Redact".to_string()));
            }
        }

        PdfObject::Dictionary(dict)
    }
}

fn color_to_pdf_array(c: [u8; 4]) -> PdfObject {
    PdfObject::Array(vec![
        PdfObject::Real(c[0] as f64 / 255.0),
        PdfObject::Real(c[1] as f64 / 255.0),
        PdfObject::Real(c[2] as f64 / 255.0),
    ])
}
