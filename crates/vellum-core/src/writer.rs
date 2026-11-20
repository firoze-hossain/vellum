//! Conforming PDF 1.7 Serializer and Byte Generator.
//! Emits valid PDF byte streams with xref table, trailer, fonts, content streams, and annotations.

use crate::content::{DisplayList, PathSegment};
use crate::deflate::deflate_zlib;
use crate::document::PdfDocument;
use crate::types::escape_pdf_string;
use std::collections::BTreeMap;
use std::io::{self, Write};

pub struct PdfWriter;

impl PdfWriter {
    /// Serializes a `PdfDocument` into standard PDF-1.7 bytes.
    pub fn write_to_bytes(doc: &PdfDocument, compress_streams: bool) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(64 * 1024);
        let mut offsets: BTreeMap<u32, usize> = BTreeMap::new();

        // 1. Header (PDF 1.7 + binary marker comment)
        buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

        let mut next_obj_id = 1u32;

        // ID 1: Catalog
        let catalog_id = next_obj_id;
        next_obj_id += 1;

        // ID 2: Pages Root
        let pages_root_id = next_obj_id;
        next_obj_id += 1;

        // IDs 3..6: Standard Font Dictionaries
        let font_helvetica_id = next_obj_id;
        next_obj_id += 1;
        let font_helvetica_bold_id = next_obj_id;
        next_obj_id += 1;
        let font_times_id = next_obj_id;
        next_obj_id += 1;
        let font_courier_id = next_obj_id;
        next_obj_id += 1;

        // Allocate IDs for pages and content streams
        let page_count = doc.pages.len();
        let mut page_ids = Vec::with_capacity(page_count);
        let mut content_ids = Vec::with_capacity(page_count);
        let mut page_annot_ids: Vec<Vec<u32>> = Vec::with_capacity(page_count);

        for page in &doc.pages {
            let pid = next_obj_id;
            next_obj_id += 1;
            page_ids.push(pid);

            let cid = next_obj_id;
            next_obj_id += 1;
            content_ids.push(cid);

            let mut a_ids = Vec::with_capacity(page.annotations.len());
            for _ in &page.annotations {
                let aid = next_obj_id;
                next_obj_id += 1;
                a_ids.push(aid);
            }
            page_annot_ids.push(a_ids);
        }

        // Info dictionary ID
        let info_id = next_obj_id;
        next_obj_id += 1;

        // -------------------------------------------------------------------
        // Write Catalog Object (1)
        // -------------------------------------------------------------------
        record_offset(&mut offsets, catalog_id, buf.len());
        writeln!(
            buf,
            "{} 0 obj\n<< /Type /Catalog /Pages {} 0 R >>\nendobj",
            catalog_id, pages_root_id
        )?;

        // -------------------------------------------------------------------
        // Write Pages Root Object (2)
        // -------------------------------------------------------------------
        record_offset(&mut offsets, pages_root_id, buf.len());
        write!(
            buf,
            "{} 0 obj\n<< /Type /Pages /Count {} /Kids [",
            pages_root_id, page_count
        )?;
        for pid in &page_ids {
            write!(buf, " {} 0 R", pid)?;
        }
        writeln!(buf, " ] >>\nendobj")?;

        // -------------------------------------------------------------------
        // Write Standard Fonts (3..6)
        // -------------------------------------------------------------------
        record_offset(&mut offsets, font_helvetica_id, buf.len());
        writeln!(
            buf,
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj",
            font_helvetica_id
        )?;

        record_offset(&mut offsets, font_helvetica_bold_id, buf.len());
        writeln!(
            buf,
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj",
            font_helvetica_bold_id
        )?;

        record_offset(&mut offsets, font_times_id, buf.len());
        writeln!(
            buf,
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /Encoding /WinAnsiEncoding >>\nendobj",
            font_times_id
        )?;

        record_offset(&mut offsets, font_courier_id, buf.len());
        writeln!(
            buf,
            "{} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>\nendobj",
            font_courier_id
        )?;

        // -------------------------------------------------------------------
        // Write Pages and Content Streams
        // -------------------------------------------------------------------
        for (i, page) in doc.pages.iter().enumerate() {
            let pid = page_ids[i];
            let cid = content_ids[i];
            let a_ids = &page_annot_ids[i];

            // Page Object
            record_offset(&mut offsets, pid, buf.len());
            write!(
                buf,
                "{} 0 obj\n<< /Type /Page /Parent {} 0 R\n   /MediaBox [{} {} {} {}]\n   /CropBox [{} {} {} {}]\n   /Rotate {}\n   /Resources << /Font << /F1 {} 0 R /F2 {} 0 R /F3 {} 0 R /F4 {} 0 R >> >>\n   /Contents {} 0 R",
                pid,
                pages_root_id,
                page.media_box.llx,
                page.media_box.lly,
                page.media_box.urx,
                page.media_box.ury,
                page.crop_box.llx,
                page.crop_box.lly,
                page.crop_box.urx,
                page.crop_box.ury,
                page.rotation,
                font_helvetica_id,
                font_helvetica_bold_id,
                font_times_id,
                font_courier_id,
                cid,
            )?;

            if !a_ids.is_empty() {
                write!(buf, "\n   /Annots [")?;
                for aid in a_ids {
                    write!(buf, " {} 0 R", aid)?;
                }
                write!(buf, " ]")?;
            }
            writeln!(buf, "\n>>\nendobj")?;

            // Generate content stream commands from display_list
            let raw_content = generate_content_stream(&page.display_list);

            record_offset(&mut offsets, cid, buf.len());
            if compress_streams {
                let compressed = deflate_zlib(&raw_content);
                writeln!(
                    buf,
                    "{} 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream",
                    cid,
                    compressed.len()
                )?;
                buf.extend_from_slice(&compressed);
                writeln!(buf, "\nendstream\nendobj")?;
            } else {
                writeln!(buf, "{} 0 obj\n<< /Length {} >>\nstream", cid, raw_content.len())?;
                buf.extend_from_slice(&raw_content);
                writeln!(buf, "\nendstream\nendobj")?;
            }

            // Write annotations for this page
            for (j, annot) in page.annotations.iter().enumerate() {
                let aid = a_ids[j];
                record_offset(&mut offsets, aid, buf.len());
                let dict_obj = annot.to_pdf_dict();
                writeln!(buf, "{} 0 obj\n{}\nendobj", aid, dict_obj)?;
            }
        }

        // -------------------------------------------------------------------
        // Write Document Info Object
        // -------------------------------------------------------------------
        record_offset(&mut offsets, info_id, buf.len());
        writeln!(
            buf,
            "{} 0 obj\n<< /Title ({})\n   /Author ({})\n   /Subject ({})\n   /Creator ({})\n   /Producer ({})\n>>\nendobj",
            info_id,
            escape_pdf_string(doc.metadata.title.as_bytes()),
            escape_pdf_string(doc.metadata.author.as_bytes()),
            escape_pdf_string(doc.metadata.subject.as_bytes()),
            escape_pdf_string(doc.metadata.creator.as_bytes()),
            escape_pdf_string(doc.metadata.producer.as_bytes()),
        )?;

        // -------------------------------------------------------------------
        // Write Cross-Reference Table (xref)
        // -------------------------------------------------------------------
        let xref_offset = buf.len();
        let total_objs = next_obj_id;

        writeln!(buf, "xref\n0 {}", total_objs)?;
        // Entry 0 is always free with generation 65535
        writeln!(buf, "0000000000 65535 f \r")?;

        for id in 1..total_objs {
            if let Some(&off) = offsets.get(&id) {
                writeln!(buf, "{:010} 00000 n \r", off)?;
            } else {
                writeln!(buf, "0000000000 00000 f \r")?;
            }
        }

        // -------------------------------------------------------------------
        // Write Trailer & startxref
        // -------------------------------------------------------------------
        writeln!(
            buf,
            "trailer\n<< /Size {}\n   /Root {} 0 R\n   /Info {} 0 R\n>>",
            total_objs, catalog_id, info_id
        )?;
        writeln!(buf, "startxref\n{}\n%%EOF", xref_offset)?;

        Ok(buf)
    }
}

fn record_offset(offsets: &mut BTreeMap<u32, usize>, id: u32, offset: usize) {
    offsets.insert(id, offset);
}

/// Generates valid PDF content stream operators from a display list.
fn generate_content_stream(dl: &DisplayList) -> Vec<u8> {
    let mut out = Vec::new();

    // 1. Vector Paths
    for p in &dl.paths {
        out.extend_from_slice(b"q\n");
        if p.line_width > 0.0 {
            writeln!(out, "{:.2} w", p.line_width).ok();
        }
        if let Some(sc) = p.stroke_color {
            writeln!(out, "{:.3} {:.3} {:.3} RG", sc.r, sc.g, sc.b).ok();
        }
        if let Some(fc) = p.fill_color {
            writeln!(out, "{:.3} {:.3} {:.3} rg", fc.r, fc.g, fc.b).ok();
        }

        for seg in &p.segments {
            match seg {
                PathSegment::MoveTo(pt) => {
                    writeln!(out, "{:.2} {:.2} m", pt.x, pt.y).ok();
                }
                PathSegment::LineTo(pt) => {
                    writeln!(out, "{:.2} {:.2} l", pt.x, pt.y).ok();
                }
                PathSegment::CurveTo(c1, c2, pt) => {
                    writeln!(
                        out,
                        "{:.2} {:.2} {:.2} {:.2} {:.2} {:.2} c",
                        c1.x, c1.y, c2.x, c2.y, pt.x, pt.y
                    )
                    .ok();
                }
                PathSegment::Close => {
                    out.extend_from_slice(b"h\n");
                }
            }
        }

        match (p.stroke_color.is_some(), p.fill_color.is_some()) {
            (true, true) => out.extend_from_slice(b"B\nQ\n"),
            (true, false) => out.extend_from_slice(b"S\nQ\n"),
            (false, true) => out.extend_from_slice(b"f\nQ\n"),
            (false, false) => out.extend_from_slice(b"n\nQ\n"),
        }
    }

    // 2. Text Spans
    for span in &dl.text_spans {
        out.extend_from_slice(b"BT\n");
        let font_tag = match span.font_name.as_str() {
            "Helvetica-Bold" => "/F2",
            "Times-Roman" => "/F3",
            "Courier" => "/F4",
            _ => "/F1",
        };
        writeln!(out, "{} {:.2} Tf", font_tag, span.font_size).ok();
        writeln!(
            out,
            "{:.3} {:.3} {:.3} rg",
            span.color.r, span.color.g, span.color.b
        )
        .ok();
        writeln!(out, "{:.2} {:.2} Td", span.x, span.y).ok();
        writeln!(out, "({}) Tj", escape_pdf_string(span.text.as_bytes())).ok();
        out.extend_from_slice(b"ET\n");
    }

    out
}
