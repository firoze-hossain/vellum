//! Pure Rust implementation of RFC 1950 (zlib) and RFC 1951 (Deflate).
//! Provides high-reliability decompression and compression for PDF `/FlateDecode` streams.

use std::io::{self, Error, ErrorKind};

/// Decompresses a zlib-wrapped Deflate byte stream (`/FlateDecode`).
pub fn inflate(data: &[u8]) -> io::Result<Vec<u8>> {
    if data.len() < 2 {
        return Err(Error::new(ErrorKind::UnexpectedEof, "zlib stream too short"));
    }

    // 1. Check zlib header
    let cmf = data[0];
    let flg = data[1];
    if (cmf as u16 * 256 + flg as u16) % 31 != 0 {
        // Some loose PDF producers omit zlib header and store raw deflate directly
        return inflate_raw(data);
    }

    let cm = cmf & 0x0F;
    if cm != 8 {
        // Not deflate compression method
        return inflate_raw(data);
    }

    let has_fdict = (flg & 0x20) != 0;
    let mut offset = 2;
    if has_fdict {
        if data.len() < 6 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "missing FDICT in zlib"));
        }
        offset += 4;
    }

    // Payload slice excluding 4-byte Adler32 checksum at the end if present
    let payload = if data.len() >= offset + 4 {
        &data[offset..data.len() - 4]
    } else {
        &data[offset..]
    };

    inflate_raw(payload)
}

/// Decompresses raw Deflate stream without zlib header/footer.
pub fn inflate_raw(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut reader = BitReader::new(data);
    let mut out = Vec::new();

    loop {
        let bfinal = reader.read_bits(1)? != 0;
        let btype = reader.read_bits(2)?;

        match btype {
            0 => {
                // Stored / Uncompressed block
                reader.align_to_byte();
                let len = reader.read_u16_le()?;
                let nlen = reader.read_u16_le()?;
                if len != !nlen {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "stored block length checksum mismatch",
                    ));
                }
                for _ in 0..len {
                    out.push(reader.read_byte()?);
                }
            }
            1 => {
                // Fixed Huffman codes
                let lit_tree = build_fixed_literal_tree();
                let dist_tree = build_fixed_distance_tree();
                decode_huffman_block(&mut reader, &lit_tree, &dist_tree, &mut out)?;
            }
            2 => {
                // Dynamic Huffman codes
                let hlit = reader.read_bits(5)? as usize + 257;
                let hdist = reader.read_bits(5)? as usize + 1;
                let hclen = reader.read_bits(4)? as usize + 4;

                const CODE_LENGTH_ORDER: [usize; 19] = [
                    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                ];

                let mut code_len_lengths = [0u8; 19];
                for i in 0..hclen {
                    code_len_lengths[CODE_LENGTH_ORDER[i]] = reader.read_bits(3)? as u8;
                }

                let code_len_tree = HuffmanTree::from_lengths(&code_len_lengths)?;

                // Decode literal/length and distance code lengths
                let mut all_lengths = Vec::with_capacity(hlit + hdist);
                while all_lengths.len() < hlit + hdist {
                    let sym = code_len_tree.decode(&mut reader)?;
                    match sym {
                        0..=15 => all_lengths.push(sym as u8),
                        16 => {
                            let last = *all_lengths
                                .last()
                                .ok_or_else(|| Error::new(ErrorKind::InvalidData, "repeat before first symbol"))?;
                            let repeat = reader.read_bits(2)? as usize + 3;
                            for _ in 0..repeat {
                                all_lengths.push(last);
                            }
                        }
                        17 => {
                            let repeat = reader.read_bits(3)? as usize + 3;
                            for _ in 0..repeat {
                                all_lengths.push(0);
                            }
                        }
                        18 => {
                            let repeat = reader.read_bits(7)? as usize + 11;
                            for _ in 0..repeat {
                                all_lengths.push(0);
                            }
                        }
                        _ => {
                            return Err(Error::new(ErrorKind::InvalidData, "invalid code length symbol"));
                        }
                    }
                }

                let lit_tree = HuffmanTree::from_lengths(&all_lengths[..hlit])?;
                let dist_tree = HuffmanTree::from_lengths(&all_lengths[hlit..])?;
                decode_huffman_block(&mut reader, &lit_tree, &dist_tree, &mut out)?;
            }
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "invalid deflate block type 3"));
            }
        }

        if bfinal {
            break;
        }
    }

    Ok(out)
}

/// Compresses a slice of bytes into conforming zlib (`/FlateDecode`) format.
pub fn deflate_zlib(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 32);

    // 1. zlib header: CMF = 0x78 (32K window, Deflate), FLG = 0x01
    out.push(0x78);
    out.push(0x01);

    // 2. Deflate payload (Stored uncompressed blocks, max 65535 bytes each)
    let chunks = data.chunks(65535);
    let count = chunks.len();

    if count == 0 {
        // Empty block
        out.push(0x01); // BFINAL = 1, BTYPE = 00
        out.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]);
    } else {
        for (i, chunk) in chunks.enumerate() {
            let is_final = i == count - 1;
            out.push(if is_final { 0x01 } else { 0x00 });
            let len = chunk.len() as u16;
            let nlen = !len;
            out.push((len & 0xFF) as u8);
            out.push(((len >> 8) & 0xFF) as u8);
            out.push((nlen & 0xFF) as u8);
            out.push(((nlen >> 8) & 0xFF) as u8);
            out.extend_from_slice(chunk);
        }
    }

    // 3. 4-byte Adler32 checksum (Big-endian)
    let adler = adler32(data);
    out.push(((adler >> 24) & 0xFF) as u8);
    out.push(((adler >> 16) & 0xFF) as u8);
    out.push(((adler >> 8) & 0xFF) as u8);
    out.push((adler & 0xFF) as u8);

    out
}

/// Calculates the standard Adler32 checksum.
pub fn adler32(data: &[u8]) -> u32 {
    let mut s1: u32 = 1;
    let mut s2: u32 = 0;
    const MOD_ADLER: u32 = 65521;

    for &b in data {
        s1 = (s1 + b as u32) % MOD_ADLER;
        s2 = (s2 + s1) % MOD_ADLER;
    }

    (s2 << 16) | s1
}

// ---------------------------------------------------------------------------
// Bit-level stream reader and Canonical Huffman decoder
// ---------------------------------------------------------------------------

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_buf: u32,
    bits_in_buf: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_buf: 0,
            bits_in_buf: 0,
        }
    }

    fn read_bits(&mut self, count: usize) -> io::Result<u32> {
        while self.bits_in_buf < count {
            if self.byte_pos >= self.data.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "unexpected end of bit stream"));
            }
            self.bit_buf |= (self.data[self.byte_pos] as u32) << self.bits_in_buf;
            self.byte_pos += 1;
            self.bits_in_buf += 8;
        }

        let mask = (1 << count) - 1;
        let val = self.bit_buf & mask;
        self.bit_buf >>= count;
        self.bits_in_buf -= count;
        Ok(val)
    }

    fn align_to_byte(&mut self) {
        let discard = self.bits_in_buf % 8;
        self.bit_buf >>= discard;
        self.bits_in_buf -= discard;
    }

    fn read_byte(&mut self) -> io::Result<u8> {
        self.read_bits(8).map(|v| v as u8)
    }

    fn read_u16_le(&mut self) -> io::Result<u16> {
        let b0 = self.read_byte()? as u16;
        let b1 = self.read_byte()? as u16;
        Ok((b1 << 8) | b0)
    }
}

struct HuffmanTree {
    table: Vec<(u16, u16)>, // (symbol, code)
    _max_bits: usize,
}

impl HuffmanTree {
    fn from_lengths(lengths: &[u8]) -> io::Result<Self> {
        let max_bits = lengths.iter().copied().max().unwrap_or(0) as usize;
        if max_bits == 0 {
            return Ok(Self {
                table: Vec::new(),
                _max_bits: 0,
            });
        }

        let mut bl_count = vec![0u16; max_bits + 1];
        for &l in lengths {
            if l > 0 {
                bl_count[l as usize] += 1;
            }
        }

        let mut next_code = vec![0u16; max_bits + 1];
        let mut code: u16 = 0;
        for bits in 1..=max_bits {
            code = (code + bl_count[bits - 1]) << 1;
            next_code[bits] = code;
        }

        let mut table = Vec::with_capacity(lengths.len());
        for (sym, &len) in lengths.iter().enumerate() {
            if len > 0 {
                let bits = len as usize;
                let c = next_code[bits];
                next_code[bits] += 1;
                table.push((sym as u16, c, bits));
            }
        }

        Ok(Self {
            table: table.into_iter().map(|(s, c, _)| (s, c)).collect(),
            _max_bits: max_bits,
        })
    }

    fn decode(&self, reader: &mut BitReader) -> io::Result<u16> {
        // Fast single bit step search
        let mut code = 0u16;
        for _len in 1..=15 {
            code = (code << 1) | (reader.read_bits(1)? as u16);
            for &(sym, c) in &self.table {
                if c == code {
                    return Ok(sym);
                }
            }
        }
        Err(Error::new(ErrorKind::InvalidData, "invalid huffman sequence"))
    }
}

fn build_fixed_literal_tree() -> HuffmanTree {
    let mut lengths = vec![0u8; 288];
    for i in 0..=143 {
        lengths[i] = 8;
    }
    for i in 144..=255 {
        lengths[i] = 9;
    }
    for i in 256..=279 {
        lengths[i] = 7;
    }
    for i in 280..=287 {
        lengths[i] = 8;
    }
    HuffmanTree::from_lengths(&lengths).unwrap()
}

fn build_fixed_distance_tree() -> HuffmanTree {
    let lengths = vec![5u8; 32];
    HuffmanTree::from_lengths(&lengths).unwrap()
}

fn decode_huffman_block(
    reader: &mut BitReader,
    lit_tree: &HuffmanTree,
    dist_tree: &HuffmanTree,
    out: &mut Vec<u8>,
) -> io::Result<()> {
    const LENGTH_BASE: [u16; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u8; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DIST_BASE: [u16; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DIST_EXTRA: [u8; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    loop {
        let sym = lit_tree.decode(reader)?;
        match sym {
            0..=255 => {
                out.push(sym as u8);
            }
            256 => {
                // End of block
                break;
            }
            257..=285 => {
                let idx = (sym - 257) as usize;
                let extra_bits = LENGTH_EXTRA[idx] as usize;
                let len = LENGTH_BASE[idx] as usize + reader.read_bits(extra_bits)? as usize;

                let dist_sym = dist_tree.decode(reader)? as usize;
                if dist_sym >= 30 {
                    return Err(Error::new(ErrorKind::InvalidData, "distance symbol >= 30"));
                }
                let dist_extra_bits = DIST_EXTRA[dist_sym] as usize;
                let dist = DIST_BASE[dist_sym] as usize + reader.read_bits(dist_extra_bits)? as usize;

                if dist > out.len() {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("lookback distance {} exceeds output length {}", dist, out.len()),
                    ));
                }

                let start = out.len() - dist;
                for i in 0..len {
                    let b = out[start + i];
                    out.push(b);
                }
            }
            _ => {
                return Err(Error::new(ErrorKind::InvalidData, "invalid literal symbol"));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adler32() {
        assert_eq!(adler32(b"Wikipedia"), 0x11E60398);
        assert_eq!(adler32(b""), 1);
    }

    #[test]
    fn test_deflate_zlib_roundtrip() {
        let original = b"Hello, Vellum PDF Studio! Testing pure Rust FlateDecode streams.";
        let compressed = deflate_zlib(original);
        assert!(compressed.len() > 6);
        let decompressed = inflate(&compressed).expect("decompression should succeed");
        assert_eq!(decompressed, original);
    }
}
