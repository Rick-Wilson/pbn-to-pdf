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

use printpdf::{Mm, PdfDocument, PdfPage, PdfSaveOptions};

use crate::config::Settings;
use crate::error::RenderError;
use crate::model::{BidSuit, Board};

use crate::render::components::DeclarersPlanSmallRenderer;
use crate::render::helpers::card_assets::CardAssets;
use crate::render::helpers::colors::SuitColors;
use crate::render::helpers::fonts::FontManager;
use crate::render::helpers::layer::LayerBuilder;

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
        let card_assets = CardAssets::load(&mut doc)
            .map_err(|e| RenderError::CardAsset(e.to_string()))?;

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

        Ok(bytes)
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
            &fonts.serif.regular,
            &fonts.serif.bold,
            &fonts.sans.regular,
            colors,
        )
        .show_bounds(self.settings.debug_boxes);

        // Page layout: 2x2 grid
        // Margins and positions
        let margin_left = self.settings.margin_left;
        let margin_top = self.settings.margin_top;
        let page_width = self.settings.page_width;
        let page_height = self.settings.page_height;

        // Calculate quadrant positions
        let half_width = (page_width - 2.0 * margin_left) / 2.0;
        let half_height = (page_height - 2.0 * margin_top) / 2.0;

        // Origins for each quadrant (top-left corner of each)
        let positions = [
            (margin_left, page_height - margin_top),                    // Top-left
            (margin_left + half_width, page_height - margin_top),      // Top-right
            (margin_left, page_height - margin_top - half_height),     // Bottom-left
            (margin_left + half_width, page_height - margin_top - half_height), // Bottom-right
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

            // Format contract string
            let contract_str = board.contract.as_ref().map(|c| {
                let suit_symbol = match c.suit {
                    BidSuit::Clubs => "♣",
                    BidSuit::Diamonds => "♦",
                    BidSuit::Hearts => "♥",
                    BidSuit::Spades => "♠",
                    BidSuit::NoTrump => "NT",
                };
                format!("{}{} {}", c.level, suit_symbol, c.declarer)
            });

            renderer.render_with_info(
                layer,
                &board.deal.north,
                &board.deal.south,
                is_nt,
                opening_lead,
                board.number,
                contract_str.as_deref(),
                (Mm(x), Mm(y)),
            );
        }
    }
}
