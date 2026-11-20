//! Software drawing primitives: rounded rectangles, outlines, lines, and alpha blending.

use crate::geom::{Color, Rect};

pub fn fill_rect(surface: &mut [u32], sw: i32, sh: i32, rect: Rect, color: Color) {
    if surface.is_empty() || sw <= 0 || sh <= 0 {
        return;
    }
    let col = color.to_u32();
    let x0 = rect.x.clamp(0, sw);
    let y0 = rect.y.clamp(0, sh);
    let x1 = (rect.x + rect.w).clamp(0, sw);
    let y1 = (rect.y + rect.h).clamp(0, sh);

    for r in y0..y1 {
        let row_start = (r * sw) as usize;
        for c in x0..x1 {
            surface[row_start + c as usize] = col;
        }
    }
}

pub fn fill_rounded_rect(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    rect: Rect,
    radius: i32,
    color: Color,
) {
    if surface.is_empty() || sw <= 0 || sh <= 0 {
        return;
    }
    let col = color.to_u32();
    let r = radius.min(rect.w / 2).min(rect.h / 2);

    let x0 = rect.x.clamp(0, sw);
    let y0 = rect.y.clamp(0, sh);
    let x1 = (rect.x + rect.w).clamp(0, sw);
    let y1 = (rect.y + rect.h).clamp(0, sh);

    let is_blend = color.a < 255;
    let alpha = color.a as f32 / 255.0;
    let inv_alpha = 1.0 - alpha;
    let cr = color.r as f32;
    let cg = color.g as f32;
    let cb = color.b as f32;

    for y in y0..y1 {
        let rel_y = y - rect.y;
        for x in x0..x1 {
            let rel_x = x - rect.x;

            // Check 4 corner cutouts
            let in_corner = if rel_x < r && rel_y < r {
                let dx = r - rel_x;
                let dy = r - rel_y;
                dx * dx + dy * dy > r * r
            } else if rel_x >= rect.w - r && rel_y < r {
                let dx = rel_x - (rect.w - r - 1);
                let dy = r - rel_y;
                dx * dx + dy * dy > r * r
            } else if rel_x < r && rel_y >= rect.h - r {
                let dx = r - rel_x;
                let dy = rel_y - (rect.h - r - 1);
                dx * dx + dy * dy > r * r
            } else if rel_x >= rect.w - r && rel_y >= rect.h - r {
                let dx = rel_x - (rect.w - r - 1);
                let dy = rel_y - (rect.h - r - 1);
                dx * dx + dy * dy > r * r
            } else {
                false
            };

            if !in_corner {
                let idx = (y * sw + x) as usize;
                if is_blend {
                    let bg = surface[idx];
                    let br = ((bg >> 16) & 0xFF) as f32;
                    let bg_g = ((bg >> 8) & 0xFF) as f32;
                    let bb = (bg & 0xFF) as f32;

                    let out_r = (cr * alpha + br * inv_alpha) as u32;
                    let out_g = (cg * alpha + bg_g * inv_alpha) as u32;
                    let out_b = (cb * alpha + bb * inv_alpha) as u32;
                    surface[idx] = 0xFF000000 | (out_r << 16) | (out_g << 8) | out_b;
                } else {
                    surface[idx] = col;
                }
            }
        }
    }
}

pub fn draw_rect_outline(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    rect: Rect,
    thickness: i32,
    color: Color,
) {
    fill_rect(surface, sw, sh, Rect::new(rect.x, rect.y, rect.w, thickness), color);
    fill_rect(surface, sw, sh, Rect::new(rect.x, rect.y + rect.h - thickness, rect.w, thickness), color);
    fill_rect(surface, sw, sh, Rect::new(rect.x, rect.y, thickness, rect.h), color);
    fill_rect(surface, sw, sh, Rect::new(rect.x + rect.w - thickness, rect.y, thickness, rect.h), color);
}

pub fn draw_line(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Color,
    thickness: i32,
) {
    if surface.is_empty() || sw <= 0 || sh <= 0 {
        return;
    }
    let col = color.to_u32();
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if thickness <= 1 {
            if x0 >= 0 && x0 < sw && y0 >= 0 && y0 < sh {
                surface[(y0 * sw + x0) as usize] = col;
            }
        } else {
            let rad = thickness / 2;
            fill_rect(surface, sw, sh, Rect::new(x0 - rad, y0 - rad, thickness, thickness), color);
        }

        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

pub fn draw_circle(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Color,
) {
    if surface.is_empty() || sw <= 0 || sh <= 0 {
        return;
    }
    let col = color.to_u32();
    let r2 = radius * radius;
    let x0 = (cx - radius).clamp(0, sw);
    let x1 = (cx + radius).clamp(0, sw);
    let y0 = (cy - radius).clamp(0, sh);
    let y1 = (cy + radius).clamp(0, sh);

    for y in y0..=y1 {
        let dy = y - cy;
        for x in x0..=x1 {
            let dx = x - cx;
            if dx * dx + dy * dy <= r2 {
                surface[(y * sw + x) as usize] = col;
            }
        }
    }
}
