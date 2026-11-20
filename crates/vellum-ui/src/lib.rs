//! Vellum UI library.
//! Provides reusable graphical interface components, layout primitives, and interactive document viewport.

pub mod components;
pub mod draw;
pub mod font;
pub mod geom;
pub mod view;

pub use components::about_dialog::render_about_dialog;
pub use components::inspector::{render_inspector, InspectorLayout};
pub use components::markup_bar::{render_markup_bar, HighlightColor, MarkupBarLayout, MarkupTool};
pub use components::ribbon::{render_ribbon, DocumentMode, RibbonLayout};
pub use components::search_bar::{render_search_bar, SearchBarLayout};
pub use components::sidebar::{render_sidebar, SidebarLayout, SidebarTab};
pub use components::status_bar::{render_status_bar, PageLayoutMode, StatusBarLayout};
pub use draw::{draw_circle, draw_line, draw_rect_outline, fill_rect, fill_rounded_rect};
pub use font::{draw_text, measure_text};
pub use geom::{Color, Point, Rect};
pub use view::VellumView;

#[cfg(test)]
mod tests {
    use super::*;
    use vellum_core::document::PdfDocument;

    #[test]
    fn test_view_creation_and_navigation() {
        let mut doc = PdfDocument::new_blank(612.0, 792.0);
        doc.pages[0].display_list.text_spans.push(vellum_core::content::TextSpan {
            text: "Hello Vellum UI".to_string(),
            font_name: "Helvetica".to_string(),
            font_size: 14.0,
            x: 50.0,
            y: 700.0,
            width: 120.0,
            height: 14.0,
            color: vellum_core::content::ColorRgb { r: 0.1, g: 0.1, b: 0.1, a: 1.0 },
        });

        let mut view = VellumView::new(doc);
        assert_eq!(view.current_page_idx, 0);
        assert_eq!(view.mode, DocumentMode::Read);

        // Zoom tests
        view.zoom_in();
        assert!(view.zoom > 1.0);
        view.zoom_out();
        assert!((view.zoom - 1.0).abs() < 0.05);

        // Tool selection
        view.set_mode(DocumentMode::Annotate);
        assert_eq!(view.mode, DocumentMode::Annotate);
        view.set_markup_tool(MarkupTool::Highlight);
        assert_eq!(view.active_tool, MarkupTool::Highlight);
    }

    #[test]
    fn test_view_render_surface() {
        let doc = PdfDocument::new_blank(612.0, 792.0);
        let mut view = VellumView::new(doc);
        let width = 800;
        let height = 600;
        let mut buffer = vec![0u32; (width * height) as usize];

        view.render(&mut buffer, width, height);

        // Check that pixels were drawn (not all zero)
        let non_zero_count = buffer.iter().filter(|&&p| p != 0).count();
        assert!(non_zero_count > 1000);
    }

    #[test]
    fn test_font_and_draw() {
        let mut buffer = vec![0xFFFFFFFFu32; 100 * 100];
        let text = "Vellum";
        let w = measure_text(text, 1);
        assert!(w > 0);

        draw_text(&mut buffer, 100, 100, 10, 10, text, Color::rgb(0, 0, 0), 1);
        let drawn_pixels = buffer.iter().filter(|&&p| p != 0xFFFFFFFF).count();
        assert!(drawn_pixels > 0);
    }

    #[test]
    fn test_view_click_dispatch() {
        let doc = PdfDocument::new_blank(612.0, 792.0);
        let mut view = VellumView::new(doc);
        let sw = 1024;
        let sh = 768;

        // Render to calculate layout
        let mut buffer = vec![0u32; (sw * sh) as usize];
        view.render(&mut buffer, sw, sh);

        // Click mode tabs
        let ribbon = render_ribbon(
            &mut [],
            sw,
            sh,
            view.mode,
            &view.editor.document.metadata.title,
            false,
            view.sidebar_open,
            view.inspector_open,
            view.night_mode,
            view.search_open,
        );

        // Click Annotate tab
        view.handle_click(ribbon.tab_annotate.x + 5, ribbon.tab_annotate.y + 5, sw, sh);
        assert_eq!(view.mode, DocumentMode::Annotate);

        // Click Edit tab
        view.handle_click(ribbon.tab_edit.x + 5, ribbon.tab_edit.y + 5, sw, sh);
        assert_eq!(view.mode, DocumentMode::Edit);

        // Toggle Night Mode
        let initial_night = view.night_mode;
        view.handle_click(ribbon.btn_night.x + 5, ribbon.btn_night.y + 5, sw, sh);
        assert_eq!(view.night_mode, !initial_night);

        // Toggle Search Bar
        let initial_search = view.search_open;
        view.handle_click(ribbon.btn_search.x + 5, ribbon.btn_search.y + 5, sw, sh);
        assert_eq!(view.search_open, !initial_search);
    }
}
