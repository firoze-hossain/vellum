//! Pure Rust PNG Encoder for Headless Document Snapshots.
//! Uses RFC 1950/1951 Deflate compression from vellum-core.

use std::fs::File;
use std::io::{self, Write};
use vellum_core::deflate::deflate_zlib;

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            if c & 1 != 0 {
                c = 0xEDB88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

fn update_crc(mut crc: u32, buf: &[u8]) -> u32 {
    for &b in buf {
        crc = CRC_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc
}

pub fn crc32(buf: &[u8]) -> u32 {
    update_crc(0xFFFFFFFF, buf) ^ 0xFFFFFFFF
}

fn write_chunk<W: Write>(w: &mut W, chunk_type: &[u8; 4], data: &[u8]) -> io::Result<()> {
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(chunk_type)?;
    w.write_all(data)?;

    let mut crc = update_crc(0xFFFFFFFF, chunk_type);
    crc = update_crc(crc, data) ^ 0xFFFFFFFF;
    w.write_all(&crc.to_be_bytes())?;
    Ok(())
}

/// Save an ARGB 32-bit pixel buffer (0xAARRGGBB) as a PNG file.
pub fn save_argb_to_png(path: &str, width: u32, height: u32, buffer: &[u32]) -> io::Result<()> {
    let mut file = File::create(path)?;

    // 1. PNG Header
    file.write_all(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])?;

    // 2. IHDR Chunk (13 bytes)
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // Bit depth
    ihdr.push(6); // Color type: RGBA
    ihdr.push(0); // Compression: deflate
    ihdr.push(0); // Filter: standard
    ihdr.push(0); // Interlace: none
    write_chunk(&mut file, b"IHDR", &ihdr)?;

    // 3. Scanline Filtering (Filter 0: None) & RGBA Conversion
    let stride = (width * 4 + 1) as usize;
    let mut raw_data = Vec::with_capacity(stride * height as usize);

    for y in 0..height {
        raw_data.push(0); // Filter type 0: None
        let row_start = (y * width) as usize;
        for x in 0..width {
            let px = buffer[row_start + x as usize];
            let a = ((px >> 24) & 0xFF) as u8;
            let r = ((px >> 16) & 0xFF) as u8;
            let g = ((px >> 8) & 0xFF) as u8;
            let b = (px & 0xFF) as u8;
            raw_data.push(r);
            raw_data.push(g);
            raw_data.push(b);
            raw_data.push(a);
        }
    }

    // 4. Deflate Compression via RFC 1950 zlib
    let compressed = deflate_zlib(&raw_data);
    write_chunk(&mut file, b"IDAT", &compressed)?;

    // 5. IEND Chunk
    write_chunk(&mut file, b"IEND", &[])?;

    Ok(())
}
