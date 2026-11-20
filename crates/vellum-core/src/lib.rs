//! Vellum Core: High-performance pure Rust PDF document reading, vector parsing,
//! annotation markup, text editing, and serialization engine.

pub mod annotations;
pub mod content;
pub mod deflate;
pub mod document;
pub mod editor;
pub mod font;
pub mod parser;
pub mod render;
pub mod search;
pub mod types;
pub mod writer;

// Re-export key types for convenient use
pub use annotations::{Annotation, AnnotationType};
pub use content::{ColorRgb, DisplayList, TextSpan, VectorPath};
pub use document::{DocumentMetadata, OutlineItem, PdfDocument, PdfPage};
pub use editor::DocumentEditor;
pub use font::StandardFont;
pub use render::{PageRenderer, RenderOptions};
pub use search::{SearchEngine, SearchHit};
pub use types::{Matrix, PdfObject, PdfRect, Point};
pub use writer::PdfWriter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blank_document_creation_and_text_extraction() {
        let doc = PdfDocument::new_blank(612.0, 792.0);
        assert_eq!(doc.page_count(), 1);
        let text = doc.extract_all_text();
        assert!(text.contains("Untitled Document"));
    }

    #[test]
    fn test_document_serialization_and_roundtrip_parsing() {
        let mut doc = PdfDocument::new_blank(612.0, 792.0);
        doc.metadata.title = "NovaOS System Architecture Specification".to_string();
        doc.metadata.author = "DeepMind & NovaOS Team".to_string();

        if let Some(p) = doc.pages.get_mut(0) {
            p.display_list.text_spans.push(TextSpan {
                text: "Section 1: Microkernel Memory Virtualization".to_string(),
                font_name: "Helvetica-Bold".to_string(),
                font_size: 14.0,
                x: 72.0,
                y: 650.0,
                width: 320.0,
                height: 14.0,
                color: ColorRgb::BLACK,
            });
            p.display_list.text_spans.push(TextSpan {
                text: "NovaOS implements 4-level paging with zero-copy shared memory.".to_string(),
                font_name: "Helvetica".to_string(),
                font_size: 11.0,
                x: 72.0,
                y: 630.0,
                width: 380.0,
                height: 11.0,
                color: ColorRgb::BLACK,
            });
        }

        // Add page 2
        let mut p2 = PdfPage::new(2, PdfRect::US_LETTER);
        p2.display_list.text_spans.push(TextSpan {
            text: "Section 2: High-Resolution Framebuffer Compositor".to_string(),
            font_name: "Helvetica-Bold".to_string(),
            font_size: 14.0,
            x: 72.0,
            y: 700.0,
            width: 340.0,
            height: 14.0,
            color: ColorRgb::BLACK,
        });
        doc.add_page(p2);
        assert_eq!(doc.page_count(), 2);

        // Serialize to PDF 1.7 bytes
        let pdf_bytes = PdfWriter::write_to_bytes(&doc, true).expect("PDF serialization failed");
        assert!(pdf_bytes.starts_with(b"%PDF-1.7"));
        assert!(pdf_bytes.ends_with(b"%%EOF\n") || pdf_bytes.ends_with(b"%%EOF"));

        // Re-parse the generated PDF
        let reparsed = PdfDocument::load_from_bytes(&pdf_bytes).expect("Roundtrip parsing failed");
        assert_eq!(reparsed.page_count(), 2);
        assert_eq!(reparsed.metadata.title, "NovaOS System Architecture Specification");

        let extracted = reparsed.extract_all_text();
        assert!(extracted.contains("Microkernel Memory Virtualization"));
        assert!(extracted.contains("zero-copy shared memory"));
        assert!(extracted.contains("Framebuffer Compositor"));
    }

    #[test]
    fn test_editor_annotations_and_search() {
        let doc = PdfDocument::new_blank(612.0, 792.0);
        let mut editor = DocumentEditor::new(doc);

        // 1. Add Text
        editor.insert_text(
            0,
            "Confidential Financial Audit Report 2026",
            "Helvetica-Bold",
            16.0,
            Point::new(72.0, 600.0),
            ColorRgb::BLACK,
        );

        // 2. Add Annotations (macOS Preview Highlight + Adobe Stamp)
        let highlight_id = editor.add_annotation(
            0,
            AnnotationType::Highlight {
                color: [255, 255, 0, 128], // Translucent yellow
                rects: vec![PdfRect::new(72.0, 595.0, 380.0, 620.0)],
            },
        );
        assert!(highlight_id.is_some());

        let stamp_id = editor.add_annotation(
            0,
            AnnotationType::Stamp {
                rect: PdfRect::new(400.0, 650.0, 520.0, 700.0),
                label: "APPROVED".to_string(),
                color: [34, 197, 94, 255], // Green
                author: "Auditor".to_string(),
                date: "2026-11-20".to_string(),
            },
        );
        assert!(stamp_id.is_some());

        assert_eq!(editor.document.pages[0].annotations.len(), 2);

        // 3. Search Engine
        let mut search = SearchEngine::new();
        search.search(&editor.document, "Financial", false);
        assert_eq!(search.count(), 1);
        let hit = search.active_hit().expect("Search hit should exist");
        assert_eq!(hit.page_index, 0);
        assert!(hit.matched_text.contains("Financial"));

        // 4. In-place Text Edit
        let span_count = editor.document.pages[0].display_list.text_spans.len();
        assert!(span_count >= 2);
        let edited = editor.edit_text_span(0, 1, "Public Financial Audit Report 2026");
        assert!(edited);
        assert!(editor.document.pages[0].display_list.text_spans[1].text.contains("Public"));

        // 5. Test Undo / Redo
        assert!(editor.undo());
        assert!(editor.document.pages[0].display_list.text_spans[1].text.contains("Confidential"));
        assert!(editor.redo());
        assert!(editor.document.pages[0].display_list.text_spans[1].text.contains("Public"));
    }
}
