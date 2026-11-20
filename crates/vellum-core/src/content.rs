//! PDF Content Stream Evaluator and Display List generator.

use crate::font::StandardFont;
use crate::parser::{Lexer, Token};
use crate::types::{Matrix, PdfObject, PdfRect, Point};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl ColorRgb {
    pub const BLACK: ColorRgb = ColorRgb { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: ColorRgb = ColorRgb { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };

    pub fn to_u32(&self) -> u32 {
        let a = (self.a.clamp(0.0, 1.0) * 255.0) as u32;
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    MoveTo(Point),
    LineTo(Point),
    CurveTo(Point, Point, Point), // (control1, control2, endpoint)
    Close,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorPath {
    pub segments: Vec<PathSegment>,
    pub stroke_color: Option<ColorRgb>,
    pub fill_color: Option<ColorRgb>,
    pub line_width: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub font_name: String,
    pub font_size: f64,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub color: ColorRgb,
}

impl TextSpan {
    pub fn bounding_rect(&self) -> PdfRect {
        PdfRect::new(self.x, self.y, self.x + self.width, self.y + self.height)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DisplayList {
    pub paths: Vec<VectorPath>,
    pub text_spans: Vec<TextSpan>,
}

#[derive(Debug, Clone)]
struct GraphicsState {
    ctm: Matrix,
    line_width: f64,
    stroke_color: ColorRgb,
    fill_color: ColorRgb,
    font_name: String,
    font_size: f64,
    text_matrix: Matrix,
    text_line_matrix: Matrix,
    pub _char_spacing: f64,
    pub _word_spacing: f64,
    leading: f64,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
            line_width: 1.0,
            stroke_color: ColorRgb::BLACK,
            fill_color: ColorRgb::BLACK,
            font_name: "Helvetica".to_string(),
            font_size: 12.0,
            text_matrix: Matrix::IDENTITY,
            text_line_matrix: Matrix::IDENTITY,
            _char_spacing: 0.0,
            _word_spacing: 0.0,
            leading: 14.0,
        }
    }
}

pub struct ContentEvaluator {
    state: GraphicsState,
    state_stack: Vec<GraphicsState>,
    current_path: Vec<PathSegment>,
    display_list: DisplayList,
}

impl ContentEvaluator {
    pub fn new() -> Self {
        Self {
            state: GraphicsState::default(),
            state_stack: Vec::new(),
            current_path: Vec::new(),
            display_list: DisplayList::default(),
        }
    }

    pub fn evaluate(mut self, data: &[u8]) -> DisplayList {
        let mut lexer = Lexer::new(data);
        let mut operands = Vec::new();

        while let Ok(Some(tok)) = lexer.next_token() {
            match tok {
                Token::Integer(i) => operands.push(PdfObject::Integer(i)),
                Token::Real(r) => operands.push(PdfObject::Real(r)),
                Token::StringLit(s) => operands.push(PdfObject::String(s, false)),
                Token::StringHex(s) => operands.push(PdfObject::String(s, true)),
                Token::Name(n) => operands.push(PdfObject::Name(n)),
                Token::ArrayOpen => {
                    // Collect array inline
                    let mut arr = Vec::new();
                    while let Ok(Some(t)) = lexer.next_token() {
                        match t {
                            Token::ArrayClose => break,
                            Token::Integer(i) => arr.push(PdfObject::Integer(i)),
                            Token::Real(r) => arr.push(PdfObject::Real(r)),
                            Token::StringLit(s) => arr.push(PdfObject::String(s, false)),
                            Token::StringHex(s) => arr.push(PdfObject::String(s, true)),
                            Token::Name(n) => arr.push(PdfObject::Name(n)),
                            _ => {}
                        }
                    }
                    operands.push(PdfObject::Array(arr));
                }
                Token::Keyword(op) => {
                    self.execute_operator(&op, &operands);
                    operands.clear();
                }
                _ => {}
            }
        }

        self.display_list
    }

    fn execute_operator(&mut self, op: &str, args: &[PdfObject]) {
        match op {
            // Graphics State Save / Restore
            "q" => {
                self.state_stack.push(self.state.clone());
            }
            "Q" => {
                if let Some(prev) = self.state_stack.pop() {
                    self.state = prev;
                }
            }
            // Transformation Matrix
            "cm" => {
                if args.len() >= 6 {
                    let m = Matrix::new(
                        args[0].as_f64().unwrap_or(1.0),
                        args[1].as_f64().unwrap_or(0.0),
                        args[2].as_f64().unwrap_or(0.0),
                        args[3].as_f64().unwrap_or(1.0),
                        args[4].as_f64().unwrap_or(0.0),
                        args[5].as_f64().unwrap_or(0.0),
                    );
                    self.state.ctm = self.state.ctm.multiply(&m);
                }
            }
            // Line width
            "w" => {
                if let Some(w) = args.first().and_then(|a| a.as_f64()) {
                    self.state.line_width = w;
                }
            }
            // Color Operators
            "rg" => {
                if args.len() >= 3 {
                    self.state.fill_color = ColorRgb {
                        r: args[0].as_f64().unwrap_or(0.0),
                        g: args[1].as_f64().unwrap_or(0.0),
                        b: args[2].as_f64().unwrap_or(0.0),
                        a: 1.0,
                    };
                }
            }
            "RG" => {
                if args.len() >= 3 {
                    self.state.stroke_color = ColorRgb {
                        r: args[0].as_f64().unwrap_or(0.0),
                        g: args[1].as_f64().unwrap_or(0.0),
                        b: args[2].as_f64().unwrap_or(0.0),
                        a: 1.0,
                    };
                }
            }
            "g" => {
                if let Some(gray) = args.first().and_then(|a| a.as_f64()) {
                    self.state.fill_color = ColorRgb { r: gray, g: gray, b: gray, a: 1.0 };
                }
            }
            "G" => {
                if let Some(gray) = args.first().and_then(|a| a.as_f64()) {
                    self.state.stroke_color = ColorRgb { r: gray, g: gray, b: gray, a: 1.0 };
                }
            }
            // Path Construction
            "m" => {
                if args.len() >= 2 {
                    let p = Point::new(args[0].as_f64().unwrap_or(0.0), args[1].as_f64().unwrap_or(0.0));
                    self.current_path.push(PathSegment::MoveTo(self.state.ctm.transform_point(p)));
                }
            }
            "l" => {
                if args.len() >= 2 {
                    let p = Point::new(args[0].as_f64().unwrap_or(0.0), args[1].as_f64().unwrap_or(0.0));
                    self.current_path.push(PathSegment::LineTo(self.state.ctm.transform_point(p)));
                }
            }
            "c" => {
                if args.len() >= 6 {
                    let c1 = Point::new(args[0].as_f64().unwrap_or(0.0), args[1].as_f64().unwrap_or(0.0));
                    let c2 = Point::new(args[2].as_f64().unwrap_or(0.0), args[3].as_f64().unwrap_or(0.0));
                    let p = Point::new(args[4].as_f64().unwrap_or(0.0), args[5].as_f64().unwrap_or(0.0));
                    self.current_path.push(PathSegment::CurveTo(
                        self.state.ctm.transform_point(c1),
                        self.state.ctm.transform_point(c2),
                        self.state.ctm.transform_point(p),
                    ));
                }
            }
            "re" => {
                if args.len() >= 4 {
                    let x = args[0].as_f64().unwrap_or(0.0);
                    let y = args[1].as_f64().unwrap_or(0.0);
                    let w = args[2].as_f64().unwrap_or(0.0);
                    let h = args[3].as_f64().unwrap_or(0.0);

                    let p1 = self.state.ctm.transform_point(Point::new(x, y));
                    let p2 = self.state.ctm.transform_point(Point::new(x + w, y));
                    let p3 = self.state.ctm.transform_point(Point::new(x + w, y + h));
                    let p4 = self.state.ctm.transform_point(Point::new(x, y + h));

                    self.current_path.push(PathSegment::MoveTo(p1));
                    self.current_path.push(PathSegment::LineTo(p2));
                    self.current_path.push(PathSegment::LineTo(p3));
                    self.current_path.push(PathSegment::LineTo(p4));
                    self.current_path.push(PathSegment::Close);
                }
            }
            "h" => {
                self.current_path.push(PathSegment::Close);
            }
            // Path Painting
            "S" | "s" => {
                self.display_list.paths.push(VectorPath {
                    segments: std::mem::take(&mut self.current_path),
                    stroke_color: Some(self.state.stroke_color),
                    fill_color: None,
                    line_width: self.state.line_width,
                });
            }
            "f" | "F" | "f*" => {
                self.display_list.paths.push(VectorPath {
                    segments: std::mem::take(&mut self.current_path),
                    stroke_color: None,
                    fill_color: Some(self.state.fill_color),
                    line_width: self.state.line_width,
                });
            }
            "B" | "b" | "B*" | "b*" => {
                self.display_list.paths.push(VectorPath {
                    segments: std::mem::take(&mut self.current_path),
                    stroke_color: Some(self.state.stroke_color),
                    fill_color: Some(self.state.fill_color),
                    line_width: self.state.line_width,
                });
            }
            "n" => {
                self.current_path.clear();
            }
            // Text Objects
            "BT" => {
                self.state.text_matrix = Matrix::IDENTITY;
                self.state.text_line_matrix = Matrix::IDENTITY;
            }
            "ET" => {}
            "Tf" => {
                if let Some(font_name) = args.first().and_then(|a| a.as_name()) {
                    self.state.font_name = font_name.to_string();
                }
                if let Some(font_size) = args.get(1).and_then(|a| a.as_f64()) {
                    self.state.font_size = font_size;
                }
            }
            "Td" => {
                if args.len() >= 2 {
                    let tx = args[0].as_f64().unwrap_or(0.0);
                    let ty = args[1].as_f64().unwrap_or(0.0);
                    let trans = Matrix::translation(tx, ty);
                    self.state.text_line_matrix = self.state.text_line_matrix.multiply(&trans);
                    self.state.text_matrix = self.state.text_line_matrix;
                }
            }
            "TD" => {
                if args.len() >= 2 {
                    let tx = args[0].as_f64().unwrap_or(0.0);
                    let ty = args[1].as_f64().unwrap_or(0.0);
                    self.state.leading = -ty;
                    let trans = Matrix::translation(tx, ty);
                    self.state.text_line_matrix = self.state.text_line_matrix.multiply(&trans);
                    self.state.text_matrix = self.state.text_line_matrix;
                }
            }
            "Tm" => {
                if args.len() >= 6 {
                    let m = Matrix::new(
                        args[0].as_f64().unwrap_or(1.0),
                        args[1].as_f64().unwrap_or(0.0),
                        args[2].as_f64().unwrap_or(0.0),
                        args[3].as_f64().unwrap_or(1.0),
                        args[4].as_f64().unwrap_or(0.0),
                        args[5].as_f64().unwrap_or(0.0),
                    );
                    self.state.text_matrix = m;
                    self.state.text_line_matrix = m;
                }
            }
            "T*" => {
                let trans = Matrix::translation(0.0, -self.state.leading);
                self.state.text_line_matrix = self.state.text_line_matrix.multiply(&trans);
                self.state.text_matrix = self.state.text_line_matrix;
            }
            "TL" => {
                if let Some(l) = args.first().and_then(|a| a.as_f64()) {
                    self.state.leading = l;
                }
            }
            "Tj" => {
                if let Some(bytes) = args.first().and_then(|a| a.as_bytes()) {
                    self.show_text_bytes(bytes);
                }
            }
            "TJ" => {
                if let Some(arr) = args.first().and_then(|a| a.as_array()) {
                    for item in arr {
                        if let Some(bytes) = item.as_bytes() {
                            self.show_text_bytes(bytes);
                        } else if let Some(offset) = item.as_f64() {
                            // Kerning offset in 1/1000 em
                            let dx = -offset * self.state.font_size / 1000.0;
                            let trans = Matrix::translation(dx, 0.0);
                            self.state.text_matrix = self.state.text_matrix.multiply(&trans);
                        }
                    }
                }
            }
            "'" => {
                // Move to next line and show string
                let trans = Matrix::translation(0.0, -self.state.leading);
                self.state.text_line_matrix = self.state.text_line_matrix.multiply(&trans);
                self.state.text_matrix = self.state.text_line_matrix;
                if let Some(bytes) = args.first().and_then(|a| a.as_bytes()) {
                    self.show_text_bytes(bytes);
                }
            }
            _ => {}
        }
    }

    fn show_text_bytes(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes).to_string();
        let font = StandardFont::from_name(&self.state.font_name);
        let width = font.measure_text(&text, self.state.font_size);
        let height = self.state.font_size;

        // Position in user space
        let pt = Point::new(0.0, 0.0);
        let eff_m = self.state.ctm.multiply(&self.state.text_matrix);
        let pos = eff_m.transform_point(pt);

        self.display_list.text_spans.push(TextSpan {
            text,
            font_name: self.state.font_name.clone(),
            font_size: self.state.font_size,
            x: pos.x,
            y: pos.y,
            width,
            height,
            color: self.state.fill_color,
        });

        // Advance text matrix
        let advance = Matrix::translation(width, 0.0);
        self.state.text_matrix = self.state.text_matrix.multiply(&advance);
    }
}
