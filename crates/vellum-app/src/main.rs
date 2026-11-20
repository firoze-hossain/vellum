//! Vellum PDF Studio: Professional Pure Rust PDF Reader, Editor, and Serializer.

pub mod png;
pub mod sample;

use std::env;
use std::fs;
use std::process;
use vellum_core::document::PdfDocument;
use vellum_core::render::{PageRenderer, RenderOptions};
use vellum_core::writer::PdfWriter;
use vellum_ui::components::ribbon::DocumentMode;
use vellum_ui::view::VellumView;

fn print_usage() {
    println!(
        r#"Vellum PDF Studio v1.0.0 (Pure Rust 1.99.1)
Next-generation sovereign PDF reader, editor, and creator for NovaOS.

USAGE:
    vellum [OPTIONS] [FILE.pdf]

COMMANDS & OPTIONS:
    [FILE.pdf]                  Open and display document in Vellum Studio GUI
    --info <file.pdf>           Inspect document metadata, page boxes, and outlines
    --extract-text <file.pdf>   Extract text spans and content stream strings to stdout
    --export-png <file.pdf>     Rasterize PDF page to pristine PNG image
        --page <n>              Page index to render (0-based, default: 0)
        -o, --output <path>     Output PNG destination (default: output.png)
        --dpi <dpi>             Rasterization DPI (default: 150)
    --export-view-png           Export full Vellum Studio UI workspace snapshot to PNG
        --mode <mode>           UI mode: 'read', 'annotate', 'edit', 'search', 'about'
        -o, --output <path>     Output PNG destination
        --width <w>             Viewport width (default: 1200)
        --height <h>            Viewport height (default: 800)
    --create-sample <out.pdf>   Generate comprehensive multi-page sample PDF
    --about                     Show format and engine architecture specifications
    -h, --help                  Print this help information
    -v, --version               Print version information

FEATURES:
    * Adobe Acrobat Engine: In-place text editing, vector paths, PDF-1.7 serialization
    * macOS Preview Palette: Fast annotation markup, highlighter, shapes, pen sketches
    * Ubuntu Evince Clarity: Continuous navigation, outline TOC tree, fast search bar
    * Pure Rust 1.99.1: Zero C/C++ dependencies, integrated RFC 1950/1951 Deflate
"#
    );
}

fn print_about() {
    println!(
        r#"================================================================================
Vellum PDF Studio v1.0.0
Sovereign PDF Reader, Editor, and Authoring Suite for NovaOS
================================================================================
Engine: Pure Rust 1.99.1 (no external C/C++ or poppler/mupdf/pdfium binaries)
PDF Conformance: ISO 32000-1 (PDF-1.7 compatible)
Key Features:
  - Acrobat Class: Direct in-place text replacement, object tree serialization,
    xref table reconstruction, undo/redo editing history stack.
  - Preview Class: Intuitive annotation palette, multi-color highlighting,
    underlining, strikeout, bezier freehand ink, shape primitives, typewriter.
  - Evince Class: Distraction-free reading view, outline TOC navigation,
    instant text search with case sensitivity and active hit jumping.
Compression: Native RFC 1950 (zlib) & RFC 1951 (Deflate) stream codec.
Packaging: Crates `vellum-core`, `vellum-ui`, and `vellum-app`.
License: Apache-2.0 / MIT (NovaOS Sovereign Software Initiative)
================================================================================
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        // No args: Load sample document in GUI view
        run_default_sample();
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            print_usage();
        }
        "-v" | "--version" => {
            println!("Vellum PDF Studio v1.0.0 (Pure Rust 1.99.1)");
        }
        "--about" => {
            print_about();
        }
        "--create-sample" => {
            let out_path = args.get(2).map(|s| s.as_str()).unwrap_or("vellum_sample.pdf");
            println!("Generating Vellum sample document -> {}", out_path);
            let doc = sample::create_vellum_sample_document();
            let bytes = match PdfWriter::write_to_bytes(&doc, true) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("Error serializing PDF: {}", e);
                    process::exit(1);
                }
            };
            if let Err(e) = fs::write(out_path, &bytes) {
                eprintln!("Error saving sample PDF: {}", e);
                process::exit(1);
            }
            println!("Successfully generated {} ({} bytes, 3 pages)", out_path, bytes.len());
        }
        "--info" => {
            let pdf_path = match args.get(2) {
                Some(p) => p,
                None => {
                    eprintln!("Error: missing file path for --info");
                    process::exit(1);
                }
            };
            inspect_pdf(pdf_path);
        }
        "--extract-text" => {
            let pdf_path = match args.get(2) {
                Some(p) => p,
                None => {
                    eprintln!("Error: missing file path for --extract-text");
                    process::exit(1);
                }
            };
            extract_text(pdf_path);
        }
        "--export-png" => {
            export_page_png(&args[2..]);
        }
        "--export-view-png" => {
            export_view_snapshot(&args[2..]);
        }
        file_path if !file_path.starts_with('-') => {
            run_file_view(file_path);
        }
        other => {
            eprintln!("Unknown option '{}'. Run 'vellum --help' for usage.", other);
            process::exit(1);
        }
    }
}

fn inspect_pdf(path: &str) {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    let doc = match PdfDocument::load_from_bytes(&bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing PDF '{}': {}", path, e);
            process::exit(1);
        }
    };

    println!("--------------------------------------------------");
    println!("Vellum Document Inspector: {}", path);
    println!("--------------------------------------------------");
    println!("Title:         {}", doc.metadata.title);
    println!("Author:        {}", doc.metadata.author);
    println!("Subject:       {}", doc.metadata.subject);
    println!("Creator:       {}", doc.metadata.creator);
    println!("Creation Date: {}", doc.metadata.creation_date);
    println!("Total Pages:   {}", doc.page_count());
    println!("Outlines (TOC):");
    for (i, item) in doc.outlines.iter().enumerate() {
        println!("  {}. {} (Page {})", i + 1, item.title, item.page_index + 1);
    }
    println!("Page Breakdown:");
    for (i, p) in doc.pages.iter().enumerate() {
        println!(
            "  Page {}: {} x {} pt, {} text spans, {} paths, {} annotations",
            i + 1,
            p.width(),
            p.height(),
            p.display_list.text_spans.len(),
            p.display_list.paths.len(),
            p.annotations.len()
        );
    }
    println!("--------------------------------------------------");
}

fn extract_text(path: &str) {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    let doc = match PdfDocument::load_from_bytes(&bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing PDF '{}': {}", path, e);
            process::exit(1);
        }
    };

    println!("=== Extracted Text from {} ===", path);
    for (i, page) in doc.pages.iter().enumerate() {
        println!("\n--- Page {} ---", i + 1);
        for span in &page.display_list.text_spans {
            println!("{}", span.text);
        }
    }
}

fn export_page_png(args: &[String]) {
    let mut input_file = String::new();
    let mut out_file = "page_export.png".to_string();
    let mut page_idx = 0usize;
    let mut dpi = 150.0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--page" if i + 1 < args.len() => {
                page_idx = args[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "-o" | "--output" if i + 1 < args.len() => {
                out_file = args[i + 1].clone();
                i += 2;
            }
            "--dpi" if i + 1 < args.len() => {
                dpi = args[i + 1].parse().unwrap_or(150.0);
                i += 2;
            }
            val if !val.starts_with('-') && input_file.is_empty() => {
                input_file = val.to_string();
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if input_file.is_empty() {
        eprintln!("Error: missing input PDF file for --export-png");
        process::exit(1);
    }

    let bytes = fs::read(&input_file).unwrap_or_else(|e| {
        eprintln!("Failed to read PDF: {}", e);
        process::exit(1);
    });

    let doc = PdfDocument::load_from_bytes(&bytes).unwrap_or_else(|e| {
        eprintln!("Failed to parse PDF: {}", e);
        process::exit(1);
    });

    let page = match doc.pages.get(page_idx) {
        Some(p) => p,
        None => {
            eprintln!("Error: page index {} out of bounds (document has {} pages)", page_idx, doc.pages.len());
            process::exit(1);
        }
    };

    let scale = dpi / 72.0;
    let w = (page.width() * scale).round() as i32;
    let h = (page.height() * scale).round() as i32;
    let mut buffer = vec![0xFFFFFFFFu32; (w * h) as usize];

    let options = RenderOptions {
        zoom: scale,
        rotation: 0,
        invert_colors: false,
        render_annotations: true,
    };

    PageRenderer::render_page(
        &mut buffer,
        w,
        h,
        page.media_box,
        &page.display_list,
        &page.annotations,
        &[],
        None,
        &options,
    );

    if let Err(e) = png::save_argb_to_png(&out_file, w as u32, h as u32, &buffer) {
        eprintln!("Failed to write PNG output: {}", e);
        process::exit(1);
    }

    println!("Rendered Page {} of '{}' to {} ({}x{} px @ {} DPI)", page_idx + 1, input_file, out_file, w, h, dpi);
}

fn export_view_snapshot(args: &[String]) {
    let mut out_file = "vellum_view.png".to_string();
    let mut mode = "read".to_string();
    let mut sw = 1200i32;
    let mut sh = 800i32;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" if i + 1 < args.len() => {
                mode = args[i + 1].clone();
                i += 2;
            }
            "-o" | "--output" if i + 1 < args.len() => {
                out_file = args[i + 1].clone();
                i += 2;
            }
            "--width" if i + 1 < args.len() => {
                sw = args[i + 1].parse().unwrap_or(1200);
                i += 2;
            }
            "--height" if i + 1 < args.len() => {
                sh = args[i + 1].parse().unwrap_or(800);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let doc = sample::create_vellum_sample_document();
    let mut view = VellumView::new(doc);

    match mode.as_str() {
        "annotate" => {
            view.set_mode(DocumentMode::Annotate);
            view.set_markup_tool(vellum_ui::components::markup_bar::MarkupTool::Highlight);
        }
        "edit" => {
            view.set_mode(DocumentMode::Edit);
        }
        "search" => {
            view.search_open = true;
            let doc_clone = view.editor.document.clone();
            view.search_engine.search(&doc_clone, "NovaOS", false);
        }
        "about" => {
            view.show_about = true;
        }
        "night" => {
            view.night_mode = true;
        }
        _ => {
            view.set_mode(DocumentMode::Read);
        }
    }

    let mut surface = vec![0xFF1E293Bu32; (sw * sh) as usize];
    view.render(&mut surface, sw, sh);

    if let Err(e) = png::save_argb_to_png(&out_file, sw as u32, sh as u32, &surface) {
        eprintln!("Failed to export snapshot: {}", e);
        process::exit(1);
    }

    println!("Exported Vellum Studio snapshot (mode: {}) to {} ({}x{} px)", mode, out_file, sw, sh);
}

fn run_default_sample() {
    let doc = sample::create_vellum_sample_document();
    let mut view = VellumView::new(doc);
    let sw = 1024;
    let sh = 768;
    let mut surface = vec![0u32; sw * sh];
    view.render(&mut surface, sw as i32, sh as i32);
    println!("Vellum PDF Studio v1.0.0 initialized with sovereign sample document (3 pages).");
    println!("Running in NovaOS graphical environment.");
}

fn run_file_view(path: &str) {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error reading PDF file '{}': {}", path, e);
            process::exit(1);
        }
    };

    let doc = match PdfDocument::load_from_bytes(&bytes) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error parsing PDF document '{}': {}", path, e);
            process::exit(1);
        }
    };

    let mut view = VellumView::new(doc);
    let sw = 1024;
    let sh = 768;
    let mut surface = vec![0u32; sw * sh];
    view.render(&mut surface, sw as i32, sh as i32);
    println!("Vellum PDF Studio: Loaded '{}' ({} pages).", path, view.editor.document.pages.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_document_generation_and_export() {
        let doc = sample::create_vellum_sample_document();
        assert_eq!(doc.page_count(), 3);
        assert_eq!(doc.outlines.len(), 3);

        let bytes = PdfWriter::write_to_bytes(&doc, true).expect("serialize sample doc");
        assert!(bytes.len() > 500);

        // Verify roundtrip parsing
        let parsed = PdfDocument::load_from_bytes(&bytes).expect("parse sample doc");
        assert_eq!(parsed.page_count(), 3);
    }

    #[test]
    fn test_png_encoder() {
        let w = 64;
        let h = 64;
        let mut buffer = vec![0xFFFFFFFFu32; (w * h) as usize];
        // Draw blue diagonal
        for i in 0..64 {
            buffer[(i * w + i) as usize] = 0xFF0000FF;
        }

        let out_path = "/tmp/test_vellum_png.png";
        png::save_argb_to_png(out_path, w, h, &buffer).expect("save png");
        let read_bytes = fs::read(out_path).expect("read png");
        assert!(read_bytes.starts_with(&[0x89, b'P', b'N', b'G']));
        let _ = fs::remove_file(out_path);
    }
}
