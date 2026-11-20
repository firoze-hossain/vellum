//! Top Ribbon & Menubar: Document Modes (Read, Annotate, Edit, Form), File Actions, and View Toggles.

use crate::draw::{draw_rect_outline, fill_rect, fill_rounded_rect};
use crate::font::draw_text;
use crate::geom::{Color, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentMode {
    Read,     // Evince / Preview clean reading
    Annotate, // macOS Preview markup toolbar
    Edit,     // Adobe Acrobat text & structure editing
    Form,     // Interactive form filling
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonAction {
    SetMode(DocumentMode),
    FileOpen,
    FileSave,
    FileExport,
    ToggleSidebar,
    ToggleInspector,
    ToggleSearch,
    ToggleNightMode,
    HelpAbout,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RibbonLayout {
    pub bounds: Rect,
    pub btn_open: Rect,
    pub btn_save: Rect,
    pub btn_export: Rect,
    pub tab_read: Rect,
    pub tab_annotate: Rect,
    pub tab_edit: Rect,
    pub tab_form: Rect,
    pub btn_sidebar: Rect,
    pub btn_search: Rect,
    pub btn_night: Rect,
    pub btn_inspector: Rect,
    pub btn_about: Rect,
}

pub fn render_ribbon(
    surface: &mut [u32],
    sw: i32,
    sh: i32,
    mode: DocumentMode,
    title: &str,
    is_dirty: bool,
    sidebar_open: bool,
    inspector_open: bool,
    night_mode: bool,
    search_open: bool,
) -> RibbonLayout {
    let ribbon_h = 44;
    let bounds = Rect::new(0, 0, sw, ribbon_h);

    // 1. Frosted deep navy background (#0F172A) with bottom border (#1E293B)
    fill_rect(surface, sw, sh, bounds, Color::rgb(15, 23, 42));
    fill_rect(
        surface,
        sw,
        sh,
        Rect::new(0, ribbon_h - 1, sw, 1),
        Color::rgb(30, 41, 59),
    );

    let mut layout = RibbonLayout {
        bounds,
        ..Default::default()
    };

    let btn_y = 9;
    let btn_h = 26;

    // 2. Vellum Brand Emblem & App Title
    let logo_x = 12;
    fill_rounded_rect(
        surface,
        sw,
        sh,
        Rect::new(logo_x, btn_y + 1, 24, 24),
        5,
        Color::rgb(14, 165, 233),
    );
    draw_text(surface, sw, sh, logo_x + 8, btn_y + 6, "V", Color::WHITE, 2);

    let mut cur_x = logo_x + 32;
    draw_text(
        surface,
        sw,
        sh,
        cur_x,
        btn_y + 6,
        "VELLUM",
        Color::rgb(241, 245, 249),
        2,
    );
    cur_x += 78;

    // File Action Buttons: [ Open ] [ Save ] [ Export ]
    let btn_open_rect = Rect::new(cur_x, btn_y, 48, btn_h);
    fill_rounded_rect(surface, sw, sh, btn_open_rect, 4, Color::rgba(255, 255, 255, 12));
    draw_rect_outline(surface, sw, sh, btn_open_rect, 1, Color::rgb(51, 65, 85));
    draw_text(surface, sw, sh, cur_x + 10, btn_y + 7, "Open", Color::rgb(226, 232, 240), 1);
    layout.btn_open = btn_open_rect;
    cur_x += 54;

    let btn_save_rect = Rect::new(cur_x, btn_y, 48, btn_h);
    let save_bg = if is_dirty { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 12) };
    fill_rounded_rect(surface, sw, sh, btn_save_rect, 4, save_bg);
    draw_rect_outline(surface, sw, sh, btn_save_rect, 1, Color::rgb(51, 65, 85));
    draw_text(surface, sw, sh, cur_x + 10, btn_y + 7, "Save", Color::WHITE, 1);
    layout.btn_save = btn_save_rect;
    cur_x += 54;

    let btn_exp_rect = Rect::new(cur_x, btn_y, 52, btn_h);
    fill_rounded_rect(surface, sw, sh, btn_exp_rect, 4, Color::rgba(255, 255, 255, 12));
    draw_rect_outline(surface, sw, sh, btn_exp_rect, 1, Color::rgb(51, 65, 85));
    draw_text(surface, sw, sh, cur_x + 8, btn_y + 7, "Export", Color::rgb(226, 232, 240), 1);
    layout.btn_export = btn_exp_rect;
    cur_x += 66;

    // Vertical Divider
    fill_rect(surface, sw, sh, Rect::new(cur_x, btn_y + 3, 1, 20), Color::rgb(51, 65, 85));
    cur_x += 14;

    // 3. Document Mode Tabs (Centered Group)
    let modes = [
        (DocumentMode::Read, "Read", 52),
        (DocumentMode::Annotate, "Annotate", 68),
        (DocumentMode::Edit, "Edit", 50),
        (DocumentMode::Form, "Form", 50),
    ];

    for (m, label, width) in modes {
        let is_sel = mode == m;
        let tab_rect = Rect::new(cur_x, btn_y, width, btn_h);
        if is_sel {
            fill_rounded_rect(surface, sw, sh, tab_rect, 5, Color::rgb(30, 41, 59));
            draw_rect_outline(surface, sw, sh, tab_rect, 1, Color::rgb(56, 189, 248));
            // Active underline indicator
            fill_rect(surface, sw, sh, Rect::new(cur_x + 6, btn_y + btn_h - 2, width - 12, 2), Color::rgb(56, 189, 248));
            draw_text(surface, sw, sh, cur_x + 10, btn_y + 7, label, Color::WHITE, 1);
        } else {
            draw_text(surface, sw, sh, cur_x + 10, btn_y + 7, label, Color::rgb(148, 163, 184), 1);
        }

        match m {
            DocumentMode::Read => layout.tab_read = tab_rect,
            DocumentMode::Annotate => layout.tab_annotate = tab_rect,
            DocumentMode::Edit => layout.tab_edit = tab_rect,
            DocumentMode::Form => layout.tab_form = tab_rect,
        }
        cur_x += width + 4;
    }

    // 4. Right Toolbar View Controls: [Sidebar] [Find] [Night] [Inspector] [?]
    let mut right_x = sw - 12;

    // About Button [ ? ]
    right_x -= 28;
    let btn_about_rect = Rect::new(right_x, btn_y, 26, btn_h);
    fill_rounded_rect(surface, sw, sh, btn_about_rect, 4, Color::rgba(255, 255, 255, 10));
    draw_text(surface, sw, sh, right_x + 8, btn_y + 7, "?", Color::rgb(148, 163, 184), 1);
    layout.btn_about = btn_about_rect;

    // Inspector Toggle [ Inspect ]
    right_x -= 64;
    let btn_insp_rect = Rect::new(right_x, btn_y, 60, btn_h);
    let insp_bg = if inspector_open { Color::rgb(30, 41, 59) } else { Color::rgba(255, 255, 255, 10) };
    fill_rounded_rect(surface, sw, sh, btn_insp_rect, 4, insp_bg);
    if inspector_open {
        draw_rect_outline(surface, sw, sh, btn_insp_rect, 1, Color::rgb(56, 189, 248));
    }
    draw_text(surface, sw, sh, right_x + 8, btn_y + 7, "Inspect", if inspector_open { Color::WHITE } else { Color::rgb(148, 163, 184) }, 1);
    layout.btn_inspector = btn_insp_rect;

    // Night Mode Toggle [ Night ]
    right_x -= 54;
    let btn_night_rect = Rect::new(right_x, btn_y, 50, btn_h);
    let night_bg = if night_mode { Color::rgb(14, 165, 233) } else { Color::rgba(255, 255, 255, 10) };
    fill_rounded_rect(surface, sw, sh, btn_night_rect, 4, night_bg);
    draw_text(surface, sw, sh, right_x + 8, btn_y + 7, "Night", Color::WHITE, 1);
    layout.btn_night = btn_night_rect;

    // Search Toggle [ Find ]
    right_x -= 58;
    let btn_srch_rect = Rect::new(right_x, btn_y, 54, btn_h);
    let srch_bg = if search_open { Color::rgb(30, 41, 59) } else { Color::rgba(255, 255, 255, 10) };
    fill_rounded_rect(surface, sw, sh, btn_srch_rect, 4, srch_bg);
    if search_open {
        draw_rect_outline(surface, sw, sh, btn_srch_rect, 1, Color::rgb(56, 189, 248));
    }
    draw_text(surface, sw, sh, right_x + 8, btn_y + 7, "Find", if search_open { Color::WHITE } else { Color::rgb(148, 163, 184) }, 1);
    layout.btn_search = btn_srch_rect;

    // Sidebar Toggle [ Sidebar ]
    right_x -= 64;
    let btn_sb_rect = Rect::new(right_x, btn_y, 60, btn_h);
    let sb_bg = if sidebar_open { Color::rgb(30, 41, 59) } else { Color::rgba(255, 255, 255, 10) };
    fill_rounded_rect(surface, sw, sh, btn_sb_rect, 4, sb_bg);
    if sidebar_open {
        draw_rect_outline(surface, sw, sh, btn_sb_rect, 1, Color::rgb(56, 189, 248));
    }
    draw_text(surface, sw, sh, right_x + 8, btn_y + 7, "Sidebar", if sidebar_open { Color::WHITE } else { Color::rgb(148, 163, 184) }, 1);
    layout.btn_sidebar = btn_sb_rect;

    // 5. Centered Document Title Badge (neatly placed between tabs and right view tools)
    let avail_left = cur_x + 16;
    let avail_right = right_x - 16;
    if avail_right > avail_left + 80 {
        let title_display = if title.is_empty() { "Untitled.pdf" } else { title };
        let dirty_mark = if is_dirty { " *" } else { "" };
        let title_full = format!("{}{}", title_display, dirty_mark);
        let max_w = (avail_right - avail_left).min(340);
        let t_w = ((title_full.len() as i32 * 6) + 16).min(max_w);
        let center_ideal = (sw - t_w) / 2;
        let t_x = center_ideal.clamp(avail_left, avail_right - t_w);

        fill_rounded_rect(surface, sw, sh, Rect::new(t_x, btn_y + 2, t_w, 22), 4, Color::rgba(0, 0, 0, 50));
        draw_rect_outline(surface, sw, sh, Rect::new(t_x, btn_y + 2, t_w, 22), 1, Color::rgb(51, 65, 85));

        let max_chars = ((t_w - 16) / 6).max(4) as usize;
        let display_str = if title_full.len() > max_chars {
            format!("{}...", &title_full[..max_chars.saturating_sub(3)])
        } else {
            title_full
        };
        draw_text(surface, sw, sh, t_x + 8, btn_y + 6, &display_str, Color::rgb(203, 213, 225), 1);
    }

    layout
}
