//! High-speed PDF text search engine (Ubuntu Evince / macOS Preview style).

use crate::document::PdfDocument;
use crate::types::PdfRect;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub page_index: usize,
    pub rect: PdfRect,
    pub matched_text: String,
    pub context_snippet: String,
}

#[derive(Debug, Clone, Default)]
pub struct SearchEngine {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub active_hit_index: Option<usize>,
    pub case_sensitive: bool,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn search(&mut self, doc: &PdfDocument, query: &str, case_sensitive: bool) {
        self.query = query.to_string();
        self.case_sensitive = case_sensitive;
        self.hits.clear();
        self.active_hit_index = None;

        if query.trim().is_empty() {
            return;
        }

        let q = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        for (p_idx, page) in doc.pages.iter().enumerate() {
            for span in &page.display_list.text_spans {
                let text_cmp = if case_sensitive {
                    span.text.clone()
                } else {
                    span.text.to_lowercase()
                };

                let mut start_idx = 0;
                while let Some(hit_offset) = text_cmp[start_idx..].find(&q) {
                    let actual_idx = start_idx + hit_offset;

                    // Calculate sub-rectangle of the hit within the text span
                    let font = crate::font::StandardFont::from_name(&span.font_name);
                    let prefix = &span.text[..actual_idx];
                    let matched = &span.text[actual_idx..actual_idx + query.len()];

                    let prefix_w = font.measure_text(prefix, span.font_size);
                    let match_w = font.measure_text(matched, span.font_size);

                    let hit_rect = PdfRect::new(
                        span.x + prefix_w,
                        span.y,
                        span.x + prefix_w + match_w,
                        span.y + span.height,
                    );

                    self.hits.push(SearchHit {
                        page_index: p_idx,
                        rect: hit_rect,
                        matched_text: matched.to_string(),
                        context_snippet: span.text.clone(),
                    });

                    start_idx = actual_idx + q.len().max(1);
                    if start_idx >= text_cmp.len() {
                        break;
                    }
                }
            }
        }

        if !self.hits.is_empty() {
            self.active_hit_index = Some(0);
        }
    }

    pub fn count(&self) -> usize {
        self.hits.len()
    }

    pub fn active_hit(&self) -> Option<&SearchHit> {
        self.active_hit_index.and_then(|idx| self.hits.get(idx))
    }

    pub fn next_hit(&mut self) -> Option<&SearchHit> {
        if self.hits.is_empty() {
            return None;
        }
        let next_idx = match self.active_hit_index {
            Some(i) => (i + 1) % self.hits.len(),
            None => 0,
        };
        self.active_hit_index = Some(next_idx);
        self.hits.get(next_idx)
    }

    pub fn prev_hit(&mut self) -> Option<&SearchHit> {
        if self.hits.is_empty() {
            return None;
        }
        let prev_idx = match self.active_hit_index {
            Some(i) => {
                if i == 0 {
                    self.hits.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.active_hit_index = Some(prev_idx);
        self.hits.get(prev_idx)
    }

    pub fn hits_for_page(&self, page_index: usize) -> Vec<PdfRect> {
        self.hits
            .iter()
            .filter(|h| h.page_index == page_index)
            .map(|h| h.rect)
            .collect()
    }
}
