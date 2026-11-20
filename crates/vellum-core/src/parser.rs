//! PDF Syntax Parser: Lexer, recursive-descent object parser, cross-reference reader, and recovery scanner.

use crate::types::PdfObject;
use std::collections::HashMap;
use std::io::{self, Error, ErrorKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword(String),
    Integer(i64),
    Real(f64),
    StringLit(Vec<u8>),
    StringHex(Vec<u8>),
    Name(String),
    ArrayOpen,
    ArrayClose,
    DictOpen,
    DictClose,
}

pub struct Lexer<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    pub fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.data.len() {
            let b = self.data[self.pos];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' || b == 0x0C {
                self.pos += 1;
            } else if b == b'%' {
                // Skip comment until end of line
                while self.pos < self.data.len() && self.data[self.pos] != b'\r' && self.data[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    pub fn peek_byte(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(self.data[self.pos])
        } else {
            None
        }
    }

    pub fn next_token(&mut self) -> io::Result<Option<Token>> {
        self.skip_whitespace_and_comments();
        if self.pos >= self.data.len() {
            return Ok(None);
        }

        let b = self.data[self.pos];

        // Delimiters
        if b == b'[' {
            self.pos += 1;
            return Ok(Some(Token::ArrayOpen));
        }
        if b == b']' {
            self.pos += 1;
            return Ok(Some(Token::ArrayClose));
        }
        if b == b'<' {
            if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'<' {
                self.pos += 2;
                return Ok(Some(Token::DictOpen));
            } else {
                // Hex string <48656C6C6F>
                self.pos += 1;
                let start = self.pos;
                while self.pos < self.data.len() && self.data[self.pos] != b'>' {
                    self.pos += 1;
                }
                let hex_slice = &self.data[start..self.pos];
                if self.pos < self.data.len() {
                    self.pos += 1; // consume '>'
                }
                let bytes = parse_hex_string(hex_slice);
                return Ok(Some(Token::StringHex(bytes)));
            }
        }
        if b == b'>' {
            if self.pos + 1 < self.data.len() && self.data[self.pos + 1] == b'>' {
                self.pos += 2;
                return Ok(Some(Token::DictClose));
            } else {
                self.pos += 1;
                return Ok(Some(Token::Keyword(">".to_string())));
            }
        }
        if b == b'/' {
            // PDF Name: /Type, /Pages
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.data.len() {
                let c = self.data[self.pos];
                if is_delimiter(c) || is_whitespace(c) {
                    break;
                }
                self.pos += 1;
            }
            let name_str = String::from_utf8_lossy(&self.data[start..self.pos]).to_string();
            return Ok(Some(Token::Name(name_str)));
        }
        if b == b'(' {
            // PDF Literal String: (Hello World) with nested parens and escape sequences
            self.pos += 1;
            let mut depth = 1;
            let mut bytes = Vec::new();

            while self.pos < self.data.len() && depth > 0 {
                let c = self.data[self.pos];
                if c == b'\\' {
                    self.pos += 1;
                    if self.pos >= self.data.len() {
                        break;
                    }
                    let esc = self.data[self.pos];
                    match esc {
                        b'n' => bytes.push(b'\n'),
                        b'r' => bytes.push(b'\r'),
                        b't' => bytes.push(b'\t'),
                        b'b' => bytes.push(0x08),
                        b'f' => bytes.push(0x0C),
                        b'(' => bytes.push(b'('),
                        b')' => bytes.push(b')'),
                        b'\\' => bytes.push(b'\\'),
                        b'0'..=b'7' => {
                            // Octal escape \ddd
                            let mut octal = (esc - b'0') as u8;
                            let mut count = 1;
                            while count < 3 && self.pos + 1 < self.data.len() {
                                let next_c = self.data[self.pos + 1];
                                if next_c >= b'0' && next_c <= b'7' {
                                    self.pos += 1;
                                    octal = (octal << 3) + (next_c - b'0');
                                    count += 1;
                                } else {
                                    break;
                                }
                            }
                            bytes.push(octal);
                        }
                        _ => bytes.push(esc),
                    }
                    self.pos += 1;
                } else if c == b'(' {
                    depth += 1;
                    bytes.push(c);
                    self.pos += 1;
                } else if c == b')' {
                    depth -= 1;
                    if depth > 0 {
                        bytes.push(c);
                    }
                    self.pos += 1;
                } else {
                    bytes.push(c);
                    self.pos += 1;
                }
            }

            return Ok(Some(Token::StringLit(bytes)));
        }

        // Numbers and regular keywords
        let start = self.pos;
        while self.pos < self.data.len() {
            let c = self.data[self.pos];
            if is_delimiter(c) || is_whitespace(c) {
                break;
            }
            self.pos += 1;
        }

        let word = std::str::from_utf8(&self.data[start..self.pos])
            .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;

        if let Ok(i) = word.parse::<i64>() {
            return Ok(Some(Token::Integer(i)));
        }
        if let Ok(r) = word.parse::<f64>() {
            return Ok(Some(Token::Real(r)));
        }

        Ok(Some(Token::Keyword(word.to_string())))
    }
}

fn parse_hex_string(hex: &[u8]) -> Vec<u8> {
    let mut clean_hex = Vec::new();
    for &b in hex {
        if !is_whitespace(b) {
            clean_hex.push(b);
        }
    }
    if clean_hex.len() % 2 != 0 {
        clean_hex.push(b'0');
    }
    let mut out = Vec::with_capacity(clean_hex.len() / 2);
    for chunk in clean_hex.chunks(2) {
        if let (Some(h1), Some(h2)) = (hex_digit(chunk[0]), hex_digit(chunk[1])) {
            out.push((h1 << 4) | h2);
        }
    }
    out
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn is_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' || b == 0x0C || b == 0x00
}

fn is_delimiter(b: u8) -> bool {
    b == b'(' || b == b')' || b == b'<' || b == b'>' || b == b'[' || b == b']' || b == b'{' || b == b'}' || b == b'/' || b == b'%'
}

/// Parses high-level PDF objects using a lookahead token stream.
pub struct PdfParser<'a> {
    lexer: Lexer<'a>,
    data: &'a [u8],
    peeked: Option<Token>,
}

impl<'a> PdfParser<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            lexer: Lexer::new(data),
            data,
            peeked: None,
        }
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.lexer.set_pos(pos);
        self.peeked = None;
    }

    pub fn pos(&self) -> usize {
        self.lexer.pos()
    }

    pub fn next_token(&mut self) -> io::Result<Option<Token>> {
        if let Some(t) = self.peeked.take() {
            Ok(Some(t))
        } else {
            self.lexer.next_token()
        }
    }

    pub fn peek_token(&mut self) -> io::Result<Option<&Token>> {
        if self.peeked.is_none() {
            self.peeked = self.lexer.next_token()?;
        }
        Ok(self.peeked.as_ref())
    }

    /// Parses the next PDF object (primitive or container).
    pub fn parse_object(&mut self) -> io::Result<Option<PdfObject>> {
        let tok = match self.next_token()? {
            Some(t) => t,
            None => return Ok(None),
        };

        match tok {
            Token::Keyword(kw) => match kw.as_str() {
                "true" => Ok(Some(PdfObject::Boolean(true))),
                "false" => Ok(Some(PdfObject::Boolean(false))),
                "null" => Ok(Some(PdfObject::Null)),
                _ => Ok(Some(PdfObject::Name(kw))),
            },
            Token::Integer(i) => {
                // Lookahead to see if this is an indirect reference `12 0 R`
                if let Some(Token::Integer(gen)) = self.peek_token()?.cloned() {
                    let cur_pos = self.lexer.pos();
                    let peeked_cached = self.peeked.take();
                    let third = self.lexer.next_token()?;
                    if let Some(Token::Keyword(ref k)) = third {
                        if k == "R" {
                            return Ok(Some(PdfObject::Reference(i as u32, gen as u16)));
                        }
                    }
                    // Rewind if not reference
                    self.lexer.set_pos(cur_pos);
                    self.peeked = peeked_cached;
                }
                Ok(Some(PdfObject::Integer(i)))
            }
            Token::Real(r) => Ok(Some(PdfObject::Real(r))),
            Token::StringLit(bytes) => Ok(Some(PdfObject::String(bytes, false))),
            Token::StringHex(bytes) => Ok(Some(PdfObject::String(bytes, true))),
            Token::Name(name) => Ok(Some(PdfObject::Name(name))),
            Token::ArrayOpen => self.parse_array(),
            Token::DictOpen => self.parse_dictionary_or_stream(),
            Token::ArrayClose | Token::DictClose => Ok(None),
        }
    }

    fn parse_array(&mut self) -> io::Result<Option<PdfObject>> {
        let mut arr = Vec::new();
        while let Some(tok) = self.peek_token()? {
            if *tok == Token::ArrayClose {
                self.next_token()?;
                break;
            }
            if let Some(obj) = self.parse_object()? {
                arr.push(obj);
            } else {
                break;
            }
        }
        Ok(Some(PdfObject::Array(arr)))
    }

    fn parse_dictionary_or_stream(&mut self) -> io::Result<Option<PdfObject>> {
        let mut dict = HashMap::new();

        while let Some(tok) = self.peek_token()?.cloned() {
            if tok == Token::DictClose {
                self.next_token()?;
                break;
            }

            let key_token = self.next_token()?;
            let key = match key_token {
                Some(Token::Name(k)) => k,
                Some(Token::Keyword(k)) => k,
                _ => break,
            };

            if let Some(val) = self.parse_object()? {
                dict.insert(key, val);
            } else {
                break;
            }
        }

        // Check if a `stream` keyword immediately follows this dictionary
        self.lexer.skip_whitespace_and_comments();
        let cur_pos = self.lexer.pos();
        if cur_pos + 6 <= self.data.len() && &self.data[cur_pos..cur_pos + 6] == b"stream" {
            self.lexer.set_pos(cur_pos + 6);

            // Skip newline after `stream` (\r\n or \n)
            if self.lexer.pos() < self.data.len() && self.data[self.lexer.pos()] == b'\r' {
                self.lexer.set_pos(self.lexer.pos() + 1);
            }
            if self.lexer.pos() < self.data.len() && self.data[self.lexer.pos()] == b'\n' {
                self.lexer.set_pos(self.lexer.pos() + 1);
            }

            let stream_start = self.lexer.pos();

            // Find `endstream`
            let mut stream_end = stream_start;
            while stream_end + 9 <= self.data.len() {
                if &self.data[stream_end..stream_end + 9] == b"endstream" {
                    break;
                }
                stream_end += 1;
            }

            // Exclude trailing whitespace/newlines right before `endstream`
            let mut actual_end = stream_end;
            if actual_end > stream_start && self.data[actual_end - 1] == b'\n' {
                actual_end -= 1;
            }
            if actual_end > stream_start && self.data[actual_end - 1] == b'\r' {
                actual_end -= 1;
            }

            let stream_bytes = self.data[stream_start..actual_end].to_vec();
            self.lexer.set_pos(stream_end + 9); // advance past `endstream`

            return Ok(Some(PdfObject::Stream(dict, stream_bytes)));
        }

        Ok(Some(PdfObject::Dictionary(dict)))
    }

    /// Scans backwards from end-of-file to find `startxref <offset>`.
    pub fn find_startxref(&self) -> io::Result<usize> {
        let len = self.data.len();
        let search_start = len.saturating_sub(1024);
        let tail = &self.data[search_start..];

        // Search backwards for "startxref"
        for i in (0..tail.len().saturating_sub(9)).rev() {
            if &tail[i..i + 9] == b"startxref" {
                let rest = &tail[i + 9..];
                let mut num_str = String::new();
                for &b in rest {
                    if b.is_ascii_digit() {
                        num_str.push(b as char);
                    } else if !num_str.is_empty() && is_whitespace(b) {
                        break;
                    }
                }
                if let Ok(offset) = num_str.parse::<usize>() {
                    return Ok(offset);
                }
            }
        }

        Err(Error::new(ErrorKind::NotFound, "startxref not found in PDF"))
    }

    /// Performs resilient linear scan to discover all `id gen obj ... endobj` in the document.
    pub fn linear_scan_objects(&mut self) -> HashMap<(u32, u16), PdfObject> {
        let mut objects = HashMap::new();
        let mut pos = 0;
        let len = self.data.len();

        while pos + 5 < len {
            // Check for pattern: digit+ space digit+ space "obj"
            if &self.data[pos..pos + 3] == b"obj" && (pos == 0 || is_whitespace(self.data[pos - 1])) {
                // Search backwards for the numbers
                let mut p = pos - 1;
                while p > 0 && is_whitespace(self.data[p]) {
                    p -= 1;
                }
                let gen_end = p + 1;
                while p > 0 && self.data[p].is_ascii_digit() {
                    p -= 1;
                }
                let gen_start = if self.data[p].is_ascii_digit() { p } else { p + 1 };

                while p > 0 && is_whitespace(self.data[p]) {
                    p -= 1;
                }
                let id_end = p + 1;
                while p > 0 && self.data[p].is_ascii_digit() {
                    p -= 1;
                }
                let id_start = if self.data[p].is_ascii_digit() { p } else { p + 1 };

                if let (Ok(id_s), Ok(gen_s)) = (
                    std::str::from_utf8(&self.data[id_start..id_end]),
                    std::str::from_utf8(&self.data[gen_start..gen_end]),
                ) {
                    if let (Ok(id), Ok(gen)) = (id_s.parse::<u32>(), gen_s.parse::<u16>()) {
                        self.set_pos(pos + 3);
                        if let Ok(Some(obj)) = self.parse_object() {
                            objects.insert((id, gen), obj);
                        }
                    }
                }
            }
            pos += 1;
        }

        objects
    }
}
