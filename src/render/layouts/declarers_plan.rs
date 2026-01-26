//! Declarer's Plan 4-Up Layout Renderer
//!
//! Generates PDF documents for declarer play practice.
//! Each page contains 4 deals in a 2x2 grid showing:
//! - North hand in dummy view
//! - South hand in fan view
//! - Opening lead card (rotated 90°)
//! - Deal number and contract
//! - Goal text (winners/losers to count)
//! - Winners table (NT) or Losers table (suit contracts)

use printpdf::{Color, Mm, PdfDocument, PdfPage, PdfSaveOptions, Rgb};

use crate::config::Settings;
use crate::error::RenderError;
use crate::model::{BidSuit, Board, Deal, Direction, Hand};

use crate::render::components::DeclarersPlanSmallRenderer;
use crate::render::helpers::card_assets::CardAssets;
use crate::render::helpers::colors::SuitColors;
use crate::render::helpers::compress::compress_pdf;
use crate::render::helpers::fonts::FontManager;
use crate::render::helpers::layer::LayerBuilder;

/// Separator line thickness (same as bidding sheets practice pages)
const SEPARATOR_THICKNESS: f32 = 2.0;

/// Separator line color (dark gray)
const SEPARATOR_COLOR: Rgb = Rgb {
    r: 0.3,
    g: 0.3,
    b: 0.3,
    icc_profile: None,
};

/// Padding inside each quadrant to center content
const QUADRANT_PADDING: f32 = 5.0;

/// Declarer's plan 4-up renderer
pub struct DeclarersPlanRenderer {
    settings: Settings,
}

impl DeclarersPlanRenderer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Generate a PDF with declarer's plan practice sheets (4 per page)
    pub fn render(&self, boards: &[Board]) -> Result<Vec<u8>, RenderError> {
        let title = boards
            .first()
            .and_then(|b| b.event.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Declarer's Plan Practice");

        let mut doc = PdfDocument::new(title);

        // Load fonts and card assets
        let fonts = FontManager::new(&mut doc)?;
        let card_assets =
            CardAssets::load(&mut doc).map_err(|e| RenderError::CardAsset(e.to_string()))?;

        let mut pages = Vec::new();

        // Process boards in groups of 4
        for chunk in boards.chunks(4) {
            let mut layer = LayerBuilder::new();
            self.render_page(&mut layer, chunk, &fonts, &card_assets);
            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));
        }

        doc.with_pages(pages);

        let mut warnings = Vec::new();
        let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

        // Compress PDF streams to reduce file size
        let compressed = compress_pdf(bytes.clone()).unwrap_or(bytes);
        Ok(compressed)
    }

    /// Render a single page with up to 4 deals
    fn render_page(
        &self,
        layer: &mut LayerBuilder,
        boards: &[Board],
        fonts: &FontManager,
        card_assets: &CardAssets,
    ) {
        let colors = SuitColors::new(self.settings.black_color, self.settings.red_color);

        let renderer = DeclarersPlanSmallRenderer::new(
            card_assets,
            fonts.serif.regular,
            fonts.serif.bold,
            fonts.symbol_font(),
            colors,
        )
        .show_bounds(self.settings.debug_boxes);

        // Page layout: 2x2 grid
        let margin_left = self.settings.margin_left;
        let margin_right = self.settings.margin_right;
        let margin_top = self.settings.margin_top;
        let margin_bottom = self.settings.margin_bottom;
        let page_width = self.settings.page_width;
        let page_height = self.settings.page_height;

        // Calculate content area
        let content_width = page_width - margin_left - margin_right;
        let content_height = page_height - margin_top - margin_bottom;

        // Calculate quadrant dimensions
        let half_width = content_width / 2.0;
        let half_height = content_height / 2.0;

        // Center lines for dividers
        let center_x = margin_left + half_width;
        let center_y = margin_bottom + half_height;

        // Draw separator lines
        self.draw_separator_lines(layer, center_x, center_y);

        // Origins for each quadrant (top-left corner of each, with padding)
        let positions = [
            (margin_left + QUADRANT_PADDING, page_height - margin_top), // Top-left
            (center_x + QUADRANT_PADDING, page_height - margin_top),    // Top-right
            (margin_left + QUADRANT_PADDING, center_y),                 // Bottom-left
            (center_x + QUADRANT_PADDING, center_y),                    // Bottom-right
        ];

        for (i, board) in boards.iter().enumerate() {
            if i >= 4 {
                break;
            }

            let (x, y) = positions[i];

            // Determine if NT contract
            let is_nt = board
                .contract
                .as_ref()
                .map(|c| c.suit == BidSuit::NoTrump)
                .unwrap_or(false);

            // Get opening lead if play sequence exists
            let opening_lead = board
                .play
                .as_ref()
                .and_then(|play| play.tricks.first().and_then(|trick| trick.cards[0]));

            // Format contract string (without declarer direction)
            let contract_str = board.contract.as_ref().map(|c| {
                let suit_symbol = match c.suit {
                    BidSuit::Clubs => "♣",
                    BidSuit::Diamonds => "♦",
                    BidSuit::Hearts => "♥",
                    BidSuit::Spades => "♠",
                    BidSuit::NoTrump => "NT",
                };
                format!("{}{}", c.level, suit_symbol)
            });

            // Rotate deal so declarer is always South (bottom of display)
            // Default to South if no declarer specified
            let declarer = board
                .contract
                .as_ref()
                .map(|c| c.declarer)
                .unwrap_or(Direction::South);
            let (dummy_hand, declarer_hand) = rotate_deal_for_declarer(&board.deal, declarer);

            // Get trump suit for suit ordering and color
            let trump = board.contract.as_ref().map(|c| c.suit);

            renderer.render_with_info(
                layer,
                &dummy_hand,
                &declarer_hand,
                is_nt,
                opening_lead,
                board.number,
                contract_str.as_deref(),
                trump,
                (Mm(x), Mm(y)),
            );
        }
    }

    /// Draw horizontal and vertical separator lines between quadrants
    fn draw_separator_lines(&self, layer: &mut LayerBuilder, center_x: f32, center_y: f32) {
        let margin_left = self.settings.margin_left;
        let margin_right = self.settings.margin_right;
        let margin_top = self.settings.margin_top;
        let margin_bottom = self.settings.margin_bottom;
        let page_width = self.settings.page_width;
        let page_height = self.settings.page_height;

        layer.set_outline_color(Color::Rgb(SEPARATOR_COLOR));
        layer.set_outline_thickness(SEPARATOR_THICKNESS);

        // Vertical line (from top margin to bottom margin)
        layer.add_line(
            Mm(center_x),
            Mm(margin_bottom),
            Mm(center_x),
            Mm(page_height - margin_top),
        );

        // Horizontal line (from left margin to right margin)
        layer.add_line(
            Mm(margin_left),
            Mm(center_y),
            Mm(page_width - margin_right),
            Mm(center_y),
        );
    }
}

/// Rotate a deal so that the declarer is always South.
/// Returns (dummy_hand, declarer_hand) where declarer is positioned as South.
///
/// This is used exclusively for the declarer's plan layout which always shows
/// declarer on the bottom (South position) and dummy on top (North position).
fn rotate_deal_for_declarer(deal: &Deal, declarer: Direction) -> (Hand, Hand) {
    match declarer {
        Direction::South => (deal.north.clone(), deal.south.clone()),
        Direction::North => (deal.south.clone(), deal.north.clone()),
        Direction::East => (deal.west.clone(), deal.east.clone()),
        Direction::West => (deal.east.clone(), deal.west.clone()),
    }
}
