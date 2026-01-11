use crate::config::Settings;
use printpdf::Mm;

/// Layout calculator for positioning elements on the page
#[derive(Debug, Clone)]
pub struct LayoutEngine {
    settings: Settings,
}

impl LayoutEngine {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Get the starting position for a board on the page
    /// Returns (x, y) coordinates in mm from bottom-left
    pub fn board_origin(&self, board_index: usize) -> (Mm, Mm) {
        let bpp = self.settings.boards_per_page as usize;

        match bpp {
            1 => {
                // Single board, centered horizontally, top of page
                let x = self.settings.margin;
                let y = self.settings.page_height - self.settings.margin;
                (Mm(x), Mm(y))
            }
            2 => {
                // Two boards, stacked vertically
                let x = self.settings.margin;
                let half_height = self.settings.content_height() / 2.0;
                let y = if board_index == 0 {
                    self.settings.page_height - self.settings.margin
                } else {
                    self.settings.page_height - self.settings.margin - half_height
                };
                (Mm(x), Mm(y))
            }
            4 => {
                // Four boards in 2x2 grid
                let half_width = self.settings.content_width() / 2.0;
                let half_height = self.settings.content_height() / 2.0;

                let col = board_index % 2;
                let row = board_index / 2;

                let x = self.settings.margin + (col as f32 * half_width);
                let y = self.settings.page_height
                    - self.settings.margin
                    - (row as f32 * half_height);

                (Mm(x), Mm(y))
            }
            _ => {
                // Default to single board
                let x = self.settings.margin;
                let y = self.settings.page_height - self.settings.margin;
                (Mm(x), Mm(y))
            }
        }
    }

    /// Get the position for the hand diagram relative to board origin
    /// Returns (x, y) offset from board origin
    pub fn diagram_offset(&self) -> (Mm, Mm) {
        // Center the diagram horizontally in the content area
        let content_width = if self.settings.boards_per_page >= 2 {
            self.settings.content_width() / 2.0
        } else {
            self.settings.content_width()
        };

        let diagram_width = self.settings.diagram_width();
        let x_offset = (content_width - diagram_width) / 2.0;

        // Place diagram below the title
        let y_offset = -20.0; // Space for title

        (Mm(x_offset), Mm(y_offset))
    }

    /// Get the position for North hand relative to diagram origin
    pub fn north_hand_position(&self, diagram_origin: (Mm, Mm)) -> (Mm, Mm) {
        let (dx, dy) = diagram_origin;
        let x = dx.0 + self.settings.hand_width + (self.settings.compass_gap / 2.0)
            - (self.settings.hand_width / 2.0);
        let y = dy.0;
        (Mm(x), Mm(y))
    }

    /// Get the position for West hand relative to diagram origin
    pub fn west_hand_position(&self, diagram_origin: (Mm, Mm)) -> (Mm, Mm) {
        let (dx, dy) = diagram_origin;
        let x = dx.0;
        let y = dy.0 - self.settings.hand_height - (self.settings.compass_gap / 2.0);
        (Mm(x), Mm(y))
    }

    /// Get the position for East hand relative to diagram origin
    pub fn east_hand_position(&self, diagram_origin: (Mm, Mm)) -> (Mm, Mm) {
        let (dx, dy) = diagram_origin;
        let x = dx.0 + self.settings.hand_width + self.settings.compass_gap;
        let y = dy.0 - self.settings.hand_height - (self.settings.compass_gap / 2.0);
        (Mm(x), Mm(y))
    }

    /// Get the position for South hand relative to diagram origin
    pub fn south_hand_position(&self, diagram_origin: (Mm, Mm)) -> (Mm, Mm) {
        let (dx, dy) = diagram_origin;
        let x = dx.0 + self.settings.hand_width + (self.settings.compass_gap / 2.0)
            - (self.settings.hand_width / 2.0);
        let y = dy.0 - (self.settings.hand_height * 2.0) - self.settings.compass_gap;
        (Mm(x), Mm(y))
    }

    /// Get the center position for the compass rose
    pub fn compass_center(&self, diagram_origin: (Mm, Mm)) -> (Mm, Mm) {
        let (dx, dy) = diagram_origin;
        let x = dx.0 + self.settings.hand_width + (self.settings.compass_gap / 2.0);
        let y = dy.0 - self.settings.hand_height - (self.settings.compass_gap / 2.0);
        (Mm(x), Mm(y))
    }

    /// Get the Y position for the bidding table (below diagram)
    pub fn bidding_table_y(&self, diagram_origin: (Mm, Mm)) -> Mm {
        let (_, dy) = diagram_origin;
        Mm(dy.0 - self.settings.diagram_height() - 15.0)
    }

    /// Get the Y position for commentary (below bidding table)
    pub fn commentary_y(&self, bidding_table_y: Mm, bidding_rows: usize) -> Mm {
        let table_height = (bidding_rows as f32 + 1.0) * self.settings.bid_row_height;
        Mm(bidding_table_y.0 - table_height - 10.0)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }
}
