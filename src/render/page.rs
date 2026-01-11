use crate::config::Settings;
use printpdf::{Mm, PdfDocumentReference, PdfLayerReference, PdfPageIndex};

/// Page manager for creating and managing PDF pages
pub struct PageManager {
    page_width: Mm,
    page_height: Mm,
    boards_per_page: u8,
}

impl PageManager {
    pub fn new(settings: &Settings) -> Self {
        Self {
            page_width: Mm(settings.page_width),
            page_height: Mm(settings.page_height),
            boards_per_page: settings.boards_per_page,
        }
    }

    /// Create a new page in the document
    pub fn create_page(
        &self,
        doc: &PdfDocumentReference,
        page_number: usize,
    ) -> (PdfPageIndex, PdfLayerReference) {
        let (page_idx, layer_idx) = doc.add_page(
            self.page_width,
            self.page_height,
            format!("Page {}", page_number + 1),
        );

        let layer = doc.get_page(page_idx).get_layer(layer_idx);

        (page_idx, layer)
    }

    /// Determine if we need a new page based on board index
    pub fn needs_new_page(&self, board_index: usize) -> bool {
        board_index % (self.boards_per_page as usize) == 0
    }

    /// Get the board position on the current page (0-based within page)
    pub fn board_position_on_page(&self, board_index: usize) -> usize {
        board_index % (self.boards_per_page as usize)
    }
}
