//! PDF Document and Page Representation with parsing, metadata, outline, and text extraction.

use crate::annotations::{Annotation, AnnotationType};
use crate::content::{ContentEvaluator, DisplayList, TextSpan};
use crate::deflate::inflate;
use crate::parser::PdfParser;
use crate::types::{PdfObject, PdfRect, Point};
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};

/// Document metadata properties (Adobe Acrobat Properties inspector).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentMetadata {
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    pub creator: String,
    pub producer: String,
    pub creation_date: String,
    pub mod_date: String,
    pub pdf_version: String,
}

/// Outline / Bookmarks item for document navigation (Ubuntu Evince / Preview TOC).
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineItem {
    pub title: String,
    pub page_index: usize,
    pub children: Vec<OutlineItem>,
}

/// Single PDF Page with geometry, display list, content stream, and annotations.
#[derive(Debug, Clone)]
pub struct PdfPage {
    pub page_num: usize,
    pub media_box: PdfRect,
    pub crop_box: PdfRect,
    pub rotation: u16, // 0, 90, 180, 270
    pub display_list: DisplayList,
    pub annotations: Vec<Annotation>,
    pub raw_content: Vec<u8>,
}

impl PdfPage {
    pub fn new(page_num: usize, rect: PdfRect) -> Self {
        Self {
            page_num,
            media_box: rect,
            crop_box: rect,
            rotation: 0,
            display_list: DisplayList::default(),
            annotations: Vec::new(),
            raw_content: Vec::new(),
        }
    }

    pub fn width(&self) -> f64 {
        if self.rotation == 90 || self.rotation == 270 {
            self.crop_box.height()
        } else {
            self.crop_box.width()
        }
    }

    pub fn height(&self) -> f64 {
        if self.rotation == 90 || self.rotation == 270 {
            self.crop_box.width()
        } else {
            self.crop_box.height()
        }
    }

    /// Extracts all text lines on this page in reading order (top to bottom).
    pub fn extract_text(&self) -> String {
        let mut spans = self.display_list.text_spans.clone();
        // Sort top-to-bottom (y high to low in PDF coordinates), then left-to-right
        spans.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        });

        let mut out = String::new();
        let mut last_y = f64::NAN;

        for span in spans {
            if !last_y.is_nan() && (last_y - span.y).abs() > span.height * 0.7 {
                out.push('\n');
            } else if !out.is_empty() && !out.ends_with('\n') && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push_str(&span.text);
            last_y = span.y;
        }

        out
    }
}

/// In-memory representation of an entire PDF document.
#[derive(Debug, Clone)]
pub struct PdfDocument {
    pub metadata: DocumentMetadata,
    pub pages: Vec<PdfPage>,
    pub outlines: Vec<OutlineItem>,
    pub raw_objects: HashMap<(u32, u16), PdfObject>,
}

impl PdfDocument {
    pub fn new_blank(width: f64, height: f64) -> Self {
        let rect = PdfRect::new(0.0, 0.0, width, height);
        let mut page = PdfPage::new(1, rect);

        // Add default sample title in blank doc
        page.display_list.text_spans.push(TextSpan {
            text: "Untitled Document".to_string(),
            font_name: "Helvetica-Bold".to_string(),
            font_size: 24.0,
            x: 72.0,
            y: height - 108.0,
            width: 210.0,
            height: 24.0,
            color: crate::content::ColorRgb::BLACK,
        });

        Self {
            metadata: DocumentMetadata {
                title: "Untitled Document".to_string(),
                author: "NovaOS User".to_string(),
                creator: "Vellum PDF Studio 1.0".to_string(),
                producer: "Vellum Pure Rust PDF Engine".to_string(),
                pdf_version: "1.7".to_string(),
                ..Default::default()
            },
            pages: vec![page],
            outlines: Vec::new(),
            raw_objects: HashMap::new(),
        }
    }

    /// Loads and parses a PDF document from raw file bytes.
    pub fn load_from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 8 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "file too small to be a PDF"));
        }

        // 1. Verify %PDF- header
        let header_slice = &data[..data.len().min(32)];
        let header_str = String::from_utf8_lossy(header_slice);
        let pdf_ver = if let Some(idx) = header_str.find("%PDF-") {
            header_str[idx + 5..idx + 8].to_string()
        } else {
            "1.7".to_string()
        };

        let mut parser = PdfParser::new(data);
        let objects = parser.linear_scan_objects();

        if objects.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "no PDF objects found in file"));
        }

        // 2. Discover Catalog / Root object
        let mut _catalog_opt = None;
        let mut info_opt = None;

        for (&(_id, _gen), obj) in &objects {
            if let Some(dict) = obj.as_dict() {
                if let Some(t) = dict.get("Type").and_then(|o| o.as_name()) {
                    if t == "Catalog" {
                        _catalog_opt = Some(dict);
                    }
                }
                if dict.contains_key("Title") || dict.contains_key("Producer") || dict.contains_key("Author") {
                    info_opt = Some(dict);
                }
            }
        }

        let mut metadata = DocumentMetadata {
            pdf_version: pdf_ver,
            ..Default::default()
        };

        if let Some(info) = info_opt {
            if let Some(t) = info.get("Title").and_then(|o| o.as_str()) {
                metadata.title = t.to_string();
            }
            if let Some(a) = info.get("Author").and_then(|o| o.as_str()) {
                metadata.author = a.to_string();
            }
            if let Some(s) = info.get("Subject").and_then(|o| o.as_str()) {
                metadata.subject = s.to_string();
            }
            if let Some(p) = info.get("Producer").and_then(|o| o.as_str()) {
                metadata.producer = p.to_string();
            }
            if let Some(c) = info.get("Creator").and_then(|o| o.as_str()) {
                metadata.creator = c.to_string();
            }
        }

        // 3. Collect Page Dictionaries from Page Tree or direct scan
        let mut page_dicts = Vec::new();
        for (&(id, gen), obj) in &objects {
            if let Some(dict) = obj.as_dict() {
                if let Some(t) = dict.get("Type").and_then(|o| o.as_name()) {
                    if t == "Page" {
                        page_dicts.push(((id, gen), dict));
                    }
                }
            }
        }

        // Sort pages deterministically by object id
        page_dicts.sort_by_key(|((id, _), _)| *id);

        let mut pages = Vec::new();

        for (idx, (_, p_dict)) in page_dicts.iter().enumerate() {
            let mbox = p_dict
                .get("MediaBox")
                .and_then(PdfRect::from_obj)
                .unwrap_or(PdfRect::US_LETTER);
            let cbox = p_dict
                .get("CropBox")
                .and_then(PdfRect::from_obj)
                .unwrap_or(mbox);
            let rot = p_dict
                .get("Rotate")
                .and_then(|o| o.as_int())
                .unwrap_or(0) as u16;

            let mut page = PdfPage::new(idx + 1, mbox);
            page.crop_box = cbox;
            page.rotation = rot;

            // Resolve /Contents stream
            if let Some(contents_obj) = p_dict.get("Contents") {
                let content_bytes = resolve_contents_bytes(contents_obj, &objects);
                page.raw_content = content_bytes.clone();
                let evaluator = ContentEvaluator::new();
                page.display_list = evaluator.evaluate(&content_bytes);
            }

            // Resolve /Annots
            if let Some(annots_obj) = p_dict.get("Annots") {
                page.annotations = resolve_annotations(annots_obj, &objects, idx);
            }

            pages.push(page);
        }

        if pages.is_empty() {
            // Create fallback single page
            pages.push(PdfPage::new(1, PdfRect::US_LETTER));
        }

        // Generate default outline items if none in catalog
        let mut outlines = Vec::new();
        for (i, _p) in pages.iter().enumerate() {
            let title = if i == 0 && !metadata.title.is_empty() {
                metadata.title.clone()
            } else {
                format!("Page {}", i + 1)
            };
            outlines.push(OutlineItem {
                title,
                page_index: i,
                children: Vec::new(),
            });
        }

        Ok(Self {
            metadata,
            pages,
            outlines,
            raw_objects: objects,
        })
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn page(&self, index: usize) -> Option<&PdfPage> {
        self.pages.get(index)
    }

    pub fn page_mut(&mut self, index: usize) -> Option<&mut PdfPage> {
        self.pages.get_mut(index)
    }

    pub fn add_page(&mut self, page: PdfPage) {
        self.pages.push(page);
    }

    pub fn delete_page(&mut self, index: usize) -> bool {
        if self.pages.len() > 1 && index < self.pages.len() {
            self.pages.remove(index);
            // Renumber pages
            for (i, p) in self.pages.iter_mut().enumerate() {
                p.page_num = i + 1;
            }
            true
        } else {
            false
        }
    }

    pub fn rotate_page(&mut self, index: usize, clockwise: bool) {
        if let Some(p) = self.pages.get_mut(index) {
            let delta = if clockwise { 90 } else { 270 };
            p.rotation = (p.rotation + delta) % 360;
        }
    }

    pub fn extract_all_text(&self) -> String {
        let mut out = String::new();
        for p in &self.pages {
            out.push_str(&format!("--- Page {} ---\n", p.page_num));
            out.push_str(&p.extract_text());
            out.push_str("\n\n");
        }
        out
    }
}

fn resolve_contents_bytes(
    obj: &PdfObject,
    objects: &HashMap<(u32, u16), PdfObject>,
) -> Vec<u8> {
    match obj {
        PdfObject::Reference(id, gen) => {
            if let Some(target) = objects.get(&(*id, *gen)) {
                resolve_contents_bytes(target, objects)
            } else {
                Vec::new()
            }
        }
        PdfObject::Stream(dict, raw_data) => {
            let is_flate = dict
                .get("Filter")
                .and_then(|f| f.as_name())
                .map(|n| n == "FlateDecode")
                .unwrap_or(false);

            if is_flate {
                inflate(raw_data).unwrap_or_else(|_| raw_data.clone())
            } else {
                raw_data.clone()
            }
        }
        PdfObject::Array(arr) => {
            let mut combined = Vec::new();
            for item in arr {
                let part = resolve_contents_bytes(item, objects);
                combined.extend_from_slice(&part);
                combined.push(b'\n');
            }
            combined
        }
        _ => Vec::new(),
    }
}

fn resolve_annotations(
    obj: &PdfObject,
    objects: &HashMap<(u32, u16), PdfObject>,
    page_idx: usize,
) -> Vec<Annotation> {
    let mut annots = Vec::new();

    let arr = match obj {
        PdfObject::Reference(id, gen) => {
            if let Some(target) = objects.get(&(*id, *gen)) {
                target.as_array()
            } else {
                None
            }
        }
        PdfObject::Array(a) => Some(a.as_slice()),
        _ => None,
    };

    if let Some(items) = arr {
        for (i, item) in items.iter().enumerate() {
            let dict = match item {
                PdfObject::Reference(id, gen) => {
                    objects.get(&(*id, *gen)).and_then(|o| o.as_dict())
                }
                PdfObject::Dictionary(d) => Some(d),
                _ => None,
            };

            if let Some(d) = dict {
                let subtype = d.get("Subtype").and_then(|s| s.as_name()).unwrap_or("");
                let rect = d.get("Rect").and_then(PdfRect::from_obj).unwrap_or_default();
                let contents = d.get("Contents").and_then(|c| c.as_str()).unwrap_or("").to_string();
                let author = d.get("T").and_then(|c| c.as_str()).unwrap_or("User").to_string();

                let color = d
                    .get("C")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        let r = (arr.get(0).and_then(|v| v.as_f64()).unwrap_or(1.0) * 255.0) as u8;
                        let g = (arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) * 255.0) as u8;
                        let b = (arr.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0) * 255.0) as u8;
                        [r, g, b, 255]
                    })
                    .unwrap_or([255, 220, 0, 255]);

                let annot_type = match subtype {
                    "Highlight" => AnnotationType::Highlight {
                        color,
                        rects: vec![rect],
                    },
                    "Underline" => AnnotationType::Underline {
                        color,
                        rects: vec![rect],
                    },
                    "StrikeOut" => AnnotationType::StrikeOut {
                        color,
                        rects: vec![rect],
                    },
                    "Square" => AnnotationType::Square {
                        rect,
                        stroke_color: color,
                        fill_color: None,
                        stroke_width: 2.0,
                    },
                    "Circle" => AnnotationType::Circle {
                        rect,
                        stroke_color: color,
                        fill_color: None,
                        stroke_width: 2.0,
                    },
                    "FreeText" => AnnotationType::FreeText {
                        rect,
                        text: contents.clone(),
                        font_size: 12.0,
                        text_color: color,
                        bg_color: Some([255, 255, 255, 230]),
                    },
                    "Text" => AnnotationType::StickyNote {
                        point: Point::new(rect.llx, rect.lly),
                        author: author.clone(),
                        text: contents.clone(),
                        timestamp: "Today".to_string(),
                    },
                    "Stamp" => AnnotationType::Stamp {
                        rect,
                        label: d.get("Name").and_then(|n| n.as_name()).unwrap_or("APPROVED").to_string(),
                        color,
                        author: author.clone(),
                        date: "Today".to_string(),
                    },
                    _ => continue,
                };

                let mut annot = Annotation::new((i + 1) as u32, page_idx, annot_type);
                annot.author = author;
                annot.contents = contents;
                annots.push(annot);
            }
        }
    }

    annots
}
