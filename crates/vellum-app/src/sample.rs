//! High-fidelity sample document generator for Vellum PDF Studio.

use vellum_core::annotations::{Annotation, AnnotationType};
use vellum_core::content::{ColorRgb, PathSegment, TextSpan, VectorPath};
use vellum_core::document::{OutlineItem, PdfDocument, PdfPage};
use vellum_core::types::{PdfRect, Point};

pub fn create_vellum_sample_document() -> PdfDocument {
    let mut doc = PdfDocument::new_blank(612.0, 792.0);
    doc.pages.clear();

    doc.metadata.title = "NovaOS Architecture & Vellum PDF Studio".to_string();
    doc.metadata.author = "NovaOS Systems Engineering Team".to_string();
    doc.metadata.subject = "Operating System Architecture and Pure Rust Document Engine".to_string();
    doc.metadata.creator = "Vellum PDF Studio v1.0.0".to_string();
    doc.metadata.creation_date = "D:20261120223000Z".to_string();

    doc.outlines = vec![
        OutlineItem {
            title: "1. Executive Overview".to_string(),
            page_index: 0,
            children: Vec::new(),
        },
        OutlineItem {
            title: "2. Pure Rust Microkernel Architecture".to_string(),
            page_index: 1,
            children: Vec::new(),
        },
        OutlineItem {
            title: "3. Vellum Document Studio Ecosystem".to_string(),
            page_index: 2,
            children: Vec::new(),
        },
    ];

    // Page 1: Executive Overview
    let mut p1 = PdfPage::new(1, PdfRect::new(0.0, 0.0, 612.0, 792.0));
    p1.display_list.paths.push(VectorPath {
        segments: vec![
            PathSegment::MoveTo(Point::new(50.0, 742.0)),
            PathSegment::LineTo(Point::new(562.0, 742.0)),
            PathSegment::LineTo(Point::new(562.0, 740.0)),
            PathSegment::LineTo(Point::new(50.0, 740.0)),
            PathSegment::Close,
        ],
        fill_color: Some(ColorRgb { r: 0.05, g: 0.45, b: 0.85, a: 1.0 }),
        stroke_color: None,
        line_width: 1.0,
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "NovaOS Technical Report: Architecture & Systems".to_string(),
        font_name: "HelveticaBold".to_string(),
        font_size: 22.0,
        x: 50.0,
        y: 750.0,
        width: 480.0,
        height: 22.0,
        color: ColorRgb { r: 0.06, g: 0.09, b: 0.16, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "Vellum Studio -- Next-Generation Document Processing in Pure Rust".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 13.0,
        x: 50.0,
        y: 720.0,
        width: 440.0,
        height: 13.0,
        color: ColorRgb { r: 0.35, g: 0.42, b: 0.50, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "1. Executive Overview".to_string(),
        font_name: "HelveticaBold".to_string(),
        font_size: 16.0,
        x: 50.0,
        y: 675.0,
        width: 200.0,
        height: 16.0,
        color: ColorRgb { r: 0.08, g: 0.12, b: 0.20, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "NovaOS introduces a unified desktop environment featuring sovereign, pure Rust userland".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 648.0,
        width: 500.0,
        height: 11.0,
        color: ColorRgb { r: 0.15, g: 0.20, b: 0.28, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "applications. Vellum PDF Studio combines the document editing power of Adobe Acrobat,".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 630.0,
        width: 500.0,
        height: 11.0,
        color: ColorRgb { r: 0.15, g: 0.20, b: 0.28, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "the swift annotation and markup palette of macOS Preview, and the lightweight clarity".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 612.0,
        width: 500.0,
        height: 11.0,
        color: ColorRgb { r: 0.15, g: 0.20, b: 0.28, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "and fast search experience of Ubuntu Evince into a single cohesive platform.".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 594.0,
        width: 500.0,
        height: 11.0,
        color: ColorRgb { r: 0.15, g: 0.20, b: 0.28, a: 1.0 },
    });

    // Callout Box
    p1.display_list.paths.push(VectorPath {
        segments: vec![
            PathSegment::MoveTo(Point::new(50.0, 560.0)),
            PathSegment::LineTo(Point::new(562.0, 560.0)),
            PathSegment::LineTo(Point::new(562.0, 480.0)),
            PathSegment::LineTo(Point::new(50.0, 480.0)),
            PathSegment::Close,
        ],
        fill_color: Some(ColorRgb { r: 0.94, g: 0.96, b: 0.99, a: 1.0 }),
        stroke_color: Some(ColorRgb { r: 0.20, g: 0.50, b: 0.90, a: 1.0 }),
        line_width: 1.5,
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "Core Architectural Highlights:".to_string(),
        font_name: "HelveticaBold".to_string(),
        font_size: 11.5,
        x: 65.0,
        y: 540.0,
        width: 300.0,
        height: 12.0,
        color: ColorRgb { r: 0.10, g: 0.30, b: 0.70, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "- Pure Rust 1.99.1 implementation with zero external C dependencies.".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 10.5,
        x: 75.0,
        y: 520.0,
        width: 450.0,
        height: 11.0,
        color: ColorRgb { r: 0.15, g: 0.22, b: 0.33, a: 1.0 },
    });

    p1.display_list.text_spans.push(TextSpan {
        text: "- Built-in RFC 1950/1951 Deflate/Inflate engine for stream compression.".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 10.5,
        x: 75.0,
        y: 502.0,
        width: 450.0,
        height: 11.0,
        color: ColorRgb { r: 0.15, g: 0.22, b: 0.33, a: 1.0 },
    });

    // Add a sample highlight annotation on page 1
    p1.annotations.push(Annotation {
        id: 1,
        page_index: 0,
        annot_type: AnnotationType::Highlight {
            rects: vec![PdfRect::new(50.0, 646.0, 500.0, 14.0)],
            color: [254, 240, 138, 140],
        },
        author: "Chief Architect".to_string(),
        contents: "Key architectural statement".to_string(),
        is_selected: false,
    });

    // Page 2: Microkernel Architecture
    let mut p2 = PdfPage::new(2, PdfRect::new(0.0, 0.0, 612.0, 792.0));
    p2.display_list.text_spans.push(TextSpan {
        text: "2. Pure Rust Microkernel Architecture".to_string(),
        font_name: "HelveticaBold".to_string(),
        font_size: 18.0,
        x: 50.0,
        y: 740.0,
        width: 400.0,
        height: 18.0,
        color: ColorRgb { r: 0.08, g: 0.12, b: 0.20, a: 1.0 },
    });

    p2.display_list.text_spans.push(TextSpan {
        text: "Memory safety is enforced from hardware boot to graphical presentation.".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 710.0,
        width: 480.0,
        height: 11.0,
        color: ColorRgb { r: 0.20, g: 0.25, b: 0.35, a: 1.0 },
    });

    // Diagram Box
    p2.display_list.paths.push(VectorPath {
        segments: vec![
            PathSegment::MoveTo(Point::new(80.0, 680.0)),
            PathSegment::LineTo(Point::new(530.0, 680.0)),
            PathSegment::LineTo(Point::new(530.0, 550.0)),
            PathSegment::LineTo(Point::new(80.0, 550.0)),
            PathSegment::Close,
        ],
        fill_color: Some(ColorRgb { r: 0.98, g: 0.98, b: 1.0, a: 1.0 }),
        stroke_color: Some(ColorRgb { r: 0.60, g: 0.65, b: 0.75, a: 1.0 }),
        line_width: 1.0,
    });

    p2.display_list.text_spans.push(TextSpan {
        text: "[ Userland: Aero Browser | Vellum PDF Studio | Spectra Media Player ]".to_string(),
        font_name: "Courier".to_string(),
        font_size: 10.0,
        x: 100.0,
        y: 650.0,
        width: 400.0,
        height: 10.0,
        color: ColorRgb { r: 0.1, g: 0.5, b: 0.2, a: 1.0 },
    });

    p2.display_list.text_spans.push(TextSpan {
        text: "[ Compositor / Window Manager (wm-rs) -- 32-bit ARGB Framebuffer ]".to_string(),
        font_name: "Courier".to_string(),
        font_size: 10.0,
        x: 100.0,
        y: 615.0,
        width: 400.0,
        height: 10.0,
        color: ColorRgb { r: 0.05, g: 0.35, b: 0.75, a: 1.0 },
    });

    p2.display_list.text_spans.push(TextSpan {
        text: "[ NovaOS Pure Rust Kernel Core -- Virtual Memory, Scheduler, IPC ]".to_string(),
        font_name: "Courier".to_string(),
        font_size: 10.0,
        x: 100.0,
        y: 580.0,
        width: 400.0,
        height: 10.0,
        color: ColorRgb { r: 0.8, g: 0.2, b: 0.1, a: 1.0 },
    });

    // Page 3: Vellum Ecosystem
    let mut p3 = PdfPage::new(3, PdfRect::new(0.0, 0.0, 612.0, 792.0));
    p3.display_list.text_spans.push(TextSpan {
        text: "3. Vellum Document Studio Ecosystem".to_string(),
        font_name: "HelveticaBold".to_string(),
        font_size: 18.0,
        x: 50.0,
        y: 740.0,
        width: 400.0,
        height: 18.0,
        color: ColorRgb { r: 0.08, g: 0.12, b: 0.20, a: 1.0 },
    });

    p3.display_list.text_spans.push(TextSpan {
        text: "Vellum provides end-to-end PDF lifecycle management: reading, in-place editing,".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 710.0,
        width: 500.0,
        height: 11.0,
        color: ColorRgb { r: 0.20, g: 0.25, b: 0.35, a: 1.0 },
    });

    p3.display_list.text_spans.push(TextSpan {
        text: "freehand annotation, form filling, text extraction, search indexing, and serialization.".to_string(),
        font_name: "Helvetica".to_string(),
        font_size: 11.0,
        x: 50.0,
        y: 692.0,
        width: 500.0,
        height: 11.0,
        color: ColorRgb { r: 0.20, g: 0.25, b: 0.35, a: 1.0 },
    });

    doc.pages.push(p1);
    doc.pages.push(p2);
    doc.pages.push(p3);

    doc
}
