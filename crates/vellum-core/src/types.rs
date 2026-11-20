//! Core PDF object representation: types, dictionaries, streams, matrices, and geometry.

use std::collections::HashMap;
use std::fmt;

/// Primary PDF Object variants conforming to the PDF 1.7 Specification (ISO 32000-1).
#[derive(Debug, Clone, PartialEq)]
pub enum PdfObject {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(Vec<u8>, bool), // (data, is_hex)
    Name(String),
    Array(Vec<PdfObject>),
    Dictionary(HashMap<String, PdfObject>),
    Stream(HashMap<String, PdfObject>, Vec<u8>), // (metadata dict, stream data)
    Reference(u32, u16),                         // (object number, generation number)
}

impl PdfObject {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            PdfObject::Integer(i) => Some(*i),
            PdfObject::Real(r) => Some(*r as i64),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            PdfObject::Real(r) => Some(*r),
            PdfObject::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PdfObject::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&str> {
        match self {
            PdfObject::Name(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            PdfObject::String(bytes, _) => Some(bytes.as_slice()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            PdfObject::String(bytes, _) => std::str::from_utf8(bytes).ok(),
            PdfObject::Name(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&HashMap<String, PdfObject>> {
        match self {
            PdfObject::Dictionary(d) => Some(d),
            PdfObject::Stream(d, _) => Some(d),
            _ => None,
        }
    }

    pub fn as_dict_mut(&mut self) -> Option<&mut HashMap<String, PdfObject>> {
        match self {
            PdfObject::Dictionary(d) => Some(d),
            PdfObject::Stream(d, _) => Some(d),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[PdfObject]> {
        match self {
            PdfObject::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_stream(&self) -> Option<(&HashMap<String, PdfObject>, &[u8])> {
        match self {
            PdfObject::Stream(d, data) => Some((d, data.as_slice())),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Option<(u32, u16)> {
        match self {
            PdfObject::Reference(id, gen) => Some((*id, *gen)),
            _ => None,
        }
    }
}

impl fmt::Display for PdfObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PdfObject::Null => write!(f, "null"),
            PdfObject::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            PdfObject::Integer(i) => write!(f, "{}", i),
            PdfObject::Real(r) => write!(f, "{:.4}", r),
            PdfObject::String(bytes, is_hex) => {
                if *is_hex {
                    write!(f, "<")?;
                    for b in bytes {
                        write!(f, "{:02X}", b)?;
                    }
                    write!(f, ">")
                } else {
                    write!(f, "({})", escape_pdf_string(bytes))
                }
            }
            PdfObject::Name(name) => write!(f, "/{}", name),
            PdfObject::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            PdfObject::Dictionary(dict) => {
                write!(f, "<<")?;
                for (k, v) in dict {
                    write!(f, " /{} {}", k, v)?;
                }
                write!(f, " >>")
            }
            PdfObject::Stream(dict, data) => {
                write!(f, "<<")?;
                for (k, v) in dict {
                    write!(f, " /{} {}", k, v)?;
                }
                write!(f, " >>\nstream\n[{} bytes]\nendstream", data.len())
            }
            PdfObject::Reference(id, gen) => write!(f, "{} {} R", id, gen),
        }
    }
}

/// Escapes a raw byte string for PDF literal string format `(...)`.
pub fn escape_pdf_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 8);
    for &b in bytes {
        match b {
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\\' => out.push_str("\\\\"),
            b'\r' => out.push_str("\\r"),
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7E => out.push(b as char),
            _ => out.push_str(&format!("\\{:03o}", b)),
        }
    }
    out
}

/// Standard 2D Point in PDF coordinate space (points: 1/72 inch, origin at bottom-left).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// PDF Rectangle defined by lower-left and upper-right coordinates [llx, lly, urx, ury].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PdfRect {
    pub llx: f64,
    pub lly: f64,
    pub urx: f64,
    pub ury: f64,
}

impl PdfRect {
    pub const fn new(llx: f64, lly: f64, urx: f64, ury: f64) -> Self {
        Self { llx, lly, urx, ury }
    }

    pub fn from_obj(obj: &PdfObject) -> Option<Self> {
        let arr = obj.as_array()?;
        if arr.len() >= 4 {
            let llx = arr[0].as_f64()?;
            let lly = arr[1].as_f64()?;
            let urx = arr[2].as_f64()?;
            let ury = arr[3].as_f64()?;
            Some(Self::new(
                llx.min(urx),
                lly.min(ury),
                llx.max(urx),
                lly.max(ury),
            ))
        } else {
            None
        }
    }

    pub fn to_obj(&self) -> PdfObject {
        PdfObject::Array(vec![
            PdfObject::Real(self.llx),
            PdfObject::Real(self.lly),
            PdfObject::Real(self.urx),
            PdfObject::Real(self.ury),
        ])
    }

    pub fn width(&self) -> f64 {
        (self.urx - self.llx).abs()
    }

    pub fn height(&self) -> f64 {
        (self.ury - self.lly).abs()
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.llx && p.x <= self.urx && p.y >= self.lly && p.y <= self.ury
    }

    pub fn intersects(&self, other: &PdfRect) -> bool {
        self.llx <= other.urx && self.urx >= other.llx && self.lly <= other.ury && self.ury >= other.lly
    }

    /// Standard US Letter dimensions (8.5 x 11 inches = 612 x 792 points).
    pub const US_LETTER: PdfRect = PdfRect::new(0.0, 0.0, 612.0, 792.0);

    /// Standard ISO A4 dimensions (210 x 297 mm = 595.28 x 841.89 points).
    pub const A4: PdfRect = PdfRect::new(0.0, 0.0, 595.28, 841.89);
}

/// 2D Affine Transformation Matrix in PDF coordinate system:
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    pub fn multiply(&self, o: &Matrix) -> Matrix {
        Matrix {
            a: self.a * o.a + self.b * o.c,
            b: self.a * o.b + self.b * o.d,
            c: self.c * o.a + self.d * o.c,
            d: self.c * o.b + self.d * o.d,
            e: self.e * o.a + self.f * o.c + o.e,
            f: self.e * o.b + self.f * o.d + o.f,
        }
    }

    pub fn transform_point(&self, p: Point) -> Point {
        Point {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }

    pub fn translation(tx: f64, ty: f64) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    pub fn scaling(sx: f64, sy: f64) -> Self {
        Self::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    pub fn rotation(radians: f64) -> Self {
        let cos = radians.cos();
        let sin = radians.sin();
        Self::new(cos, sin, -sin, cos, 0.0, 0.0)
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}
