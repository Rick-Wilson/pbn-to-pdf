//! Declarer's plan small component
//!
//! Renders a compact layout for one quadrant of a page showing:
//! - Header line: Deal # (left), Contract (center), Goal (right)
//! - North hand in dummy view
//! - South hand in fan view
//! - Winners or Losers table (below south hand)
//!   - NT contracts: Winners table
//!   - Suit contracts: Losers table

use printpdf::{Color, FontId, Mm};

use crate::model::{Board, Card, Hand, Suit};
use crate::render::components::{
    DummyRenderer, FanRenderer, LosersTableRenderer, WinnersTableRenderer,
};
use crate::render::helpers::card_assets::{CardAssets, CARD_HEIGHT_MM};
use crate::render::helpers::colors::{SuitColors, BLACK};
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics;

/// Gap between elements in mm
const ELEMENT_GAP: f32 = 4.0;

/// Portion of fan height to display (crop from bottom)
const FAN_CROP_RATIO: f32 = 0.5;

/// Nominal number of cards in a suit for calculating fixed dummy height
const NOMINAL_SUIT_LENGTH: usize = 5;

/// Font size for header text (increased for visibility)
const HEADER_FONT_SIZE: f32 = 14.0;

/// Height of the header line area
const HEADER_HEIGHT: f32 = 8.0;

/// Extra space to raise dummy (one line height)
const DUMMY_RAISE: f32 = 1.0;

/// Extra space to raise fan (~1 inch minus header savings)
const FAN_RAISE: f32 = 21.4;

/// Extra space to raise table (about 1.5x title row height minus header savings + row height)
const TABLE_RAISE: f32 = 11.5;

/// Renderer for a small declarer's plan layout (one quadrant of a page)
pub struct DeclarersPlanSmallRenderer<'a> {
    card_assets: &'a CardAssets,
    font: &'a FontId,
    bold_font: &'a FontId,
    symbol_font: &'a FontId,
    colors: SuitColors,
    /// Scale factor for card rendering
    card_scale: f32,
    /// Arc degrees for the fan display
    fan_arc: f32,
    /// Overlap ratio for dummy display
    dummy_overlap: f32,
    /// Whether to show debug bounding boxes
    show_bounds: bool,
}

impl<'a> DeclarersPlanSmallRenderer<'a> {
    /// Create a new declarer's plan small renderer
    pub fn new(
        card_assets: &'a CardAssets,
        font: &'a FontId,
        bold_font: &'a FontId,
        symbol_font: &'a FontId,
        colors: SuitColors,
    ) -> Self {
        Self {
            card_assets,
            font,
            bold_font,
            symbol_font,
            colors,
            card_scale: 0.35,
            fan_arc: 30.0,
            dummy_overlap: 0.18,  // Show some suit symbol on clipped cards
            show_bounds: false,
        }
    }

    /// Set the card scale factor
    pub fn card_scale(mut self, scale: f32) -> Self {
        self.card_scale = scale;
        self
    }

    /// Set the fan arc in degrees
    pub fn fan_arc(mut self, arc: f32) -> Self {
        self.fan_arc = arc;
        self
    }

    /// Set the dummy overlap ratio
    pub fn dummy_overlap(mut self, overlap: f32) -> Self {
        self.dummy_overlap = overlap;
        self
    }

    /// Set whether to show debug bounding boxes
    pub fn show_bounds(mut self, show: bool) -> Self {
        self.show_bounds = show;
        self
    }

    /// Get the dummy renderer configured for this layout
    fn dummy_renderer(&self) -> DummyRenderer<'a> {
        DummyRenderer::with_overlap(self.card_assets, self.card_scale, self.dummy_overlap)
            .first_suit(Suit::Spades)
            .show_bounds(self.show_bounds)
    }

    /// Calculate the nominal dummy height based on a 5-card suit
    /// This provides consistent positioning regardless of actual hand shape
    fn nominal_dummy_height(&self) -> f32 {
        let card_height = CARD_HEIGHT_MM * self.card_scale;
        let visible_height = card_height * self.dummy_overlap;
        // Height: one full card + overlapped portions for (NOMINAL_SUIT_LENGTH - 1) cards
        card_height + (NOMINAL_SUIT_LENGTH - 1) as f32 * visible_height
    }

    /// Get the fan renderer configured for this layout, scaled to match dummy width
    fn fan_renderer(&self, target_width: f32, hand: &Hand) -> FanRenderer<'a> {
        // Calculate scale to match target width
        let temp_renderer = FanRenderer::new(self.card_assets, 1.0).arc(self.fan_arc);
        let (temp_width, _) = temp_renderer.dimensions(hand);
        let scale = if temp_width > 0.0 {
            target_width / temp_width
        } else {
            self.card_scale
        };

        FanRenderer::new(self.card_assets, scale)
            .arc(self.fan_arc)
            .show_bounds(self.show_bounds)
    }

    /// Calculate dimensions needed for the layout
    ///
    /// Returns (width, height) in mm.
    /// Uses nominal dummy height (5-card suit) for consistent positioning.
    pub fn dimensions(&self, north: &Hand, south: &Hand, is_nt: bool) -> (f32, f32) {
        let dummy_renderer = self.dummy_renderer();
        let (dummy_width, _) = dummy_renderer.dimensions(north);

        // Use nominal height for consistent layout
        let nominal_dummy_height = self.nominal_dummy_height();

        let fan_renderer = self.fan_renderer(dummy_width, south);
        let (_, fan_height) = fan_renderer.dimensions(south);
        // Only count the visible (cropped) portion of the fan
        let visible_fan_height = fan_height * FAN_CROP_RATIO;

        // Table dimensions - both tables have the same dimensions
        let (table_width, table_height) = if is_nt {
            self.winners_table_renderer().dimensions()
        } else {
            self.losers_table_renderer().dimensions()
        };

        // Width is just the content area
        let width = dummy_width.max(table_width);

        // Total height: header + gap + dummy + gap + visible fan + gap + table
        let height = HEADER_HEIGHT + ELEMENT_GAP + nominal_dummy_height + ELEMENT_GAP
            + visible_fan_height + ELEMENT_GAP + table_height;

        (width, height)
    }

    /// Create the winners table renderer
    fn winners_table_renderer(&self) -> WinnersTableRenderer<'a> {
        WinnersTableRenderer::new(
            self.font,
            self.bold_font,
            self.symbol_font,
            self.colors.clone(),
        )
    }

    /// Create the losers table renderer
    fn losers_table_renderer(&self) -> LosersTableRenderer<'a> {
        LosersTableRenderer::new(
            self.font,
            self.bold_font,
            self.symbol_font,
            self.colors.clone(),
        )
    }

    /// Render the declarer's plan layout from a Board
    ///
    /// Origin is the top-left corner of the display area.
    /// Returns the height used.
    pub fn render_board(&self, layer: &mut LayerBuilder, board: &Board, origin: (Mm, Mm)) -> f32 {
        let north = &board.deal.north;
        let south = &board.deal.south;

        // Determine if NT contract
        let is_nt = board
            .contract
            .as_ref()
            .map(|c| c.suit == crate::model::BidSuit::NoTrump)
            .unwrap_or(false);

        // Get opening lead if play sequence exists
        let opening_lead = board
            .play
            .as_ref()
            .and_then(|play| play.tricks.first().and_then(|trick| trick.cards[0]));

        self.render(layer, north, south, is_nt, opening_lead, origin)
    }

    /// Render the declarer's plan layout with explicit parameters
    ///
    /// Origin is the top-left corner of the display area.
    /// The opening lead is rendered between dummy and fan.
    /// Returns the height used.
    pub fn render(
        &self,
        layer: &mut LayerBuilder,
        north: &Hand,
        south: &Hand,
        is_nt: bool,
        opening_lead: Option<Card>,
        origin: (Mm, Mm),
    ) -> f32 {
        self.render_with_info(layer, north, south, is_nt, opening_lead, None, None, origin)
    }

    /// Render the declarer's plan layout with deal info
    ///
    /// Origin is the top-left corner of the display area.
    /// deal_number: Optional deal number to display (e.g., "1", "2")
    /// contract_str: Optional contract string (e.g., "4H South")
    /// Returns the height used.
    #[allow(unused_variables)]
    pub fn render_with_info(
        &self,
        layer: &mut LayerBuilder,
        north: &Hand,
        south: &Hand,
        is_nt: bool,
        opening_lead: Option<Card>,
        deal_number: Option<u32>,
        contract_str: Option<&str>,
        origin: (Mm, Mm),
    ) -> f32 {
        let (ox, oy) = (origin.0 .0, origin.1 .0);

        // Content starts at origin
        let content_x = ox;

        // Get dummy dimensions for layout calculations
        let dummy_renderer = self.dummy_renderer();
        let (dummy_width, _) = dummy_renderer.dimensions(north);
        let nominal_dummy_height = self.nominal_dummy_height();

        // Right edge for right-justified text
        let right_edge = content_x + dummy_width;

        // Render header line at the top
        // Header Y position (baseline of text)
        let header_y = oy - HEADER_HEIGHT + 2.0; // 2mm from bottom of header area

        layer.set_fill_color(Color::Rgb(BLACK));
        let measurer = text_metrics::get_serif_measurer();

        // Left: "Deal #" followed by "Ctr: xx"
        let mut text_x = content_x;
        if let Some(deal_num) = deal_number {
            let deal_text = format!("Deal {}", deal_num);
            let deal_width = measurer.measure_width_mm(&deal_text, HEADER_FONT_SIZE);
            layer.use_text(&deal_text, HEADER_FONT_SIZE, Mm(text_x), Mm(header_y), self.bold_font);
            text_x += deal_width + 2.0; // Gap between deal and contract
        }

        // Contract right after deal number (abbreviated)
        // Use serif for "Ctr: " label, sans for contract value (has suit symbols)
        if let Some(contract) = contract_str {
            let label = "Ctr: ";
            let label_width = measurer.measure_width_mm(label, HEADER_FONT_SIZE);
            layer.use_text(label, HEADER_FONT_SIZE, Mm(text_x), Mm(header_y), self.font);
            layer.use_text(contract, HEADER_FONT_SIZE, Mm(text_x + label_width), Mm(header_y), self.symbol_font);
        }

        // Right: Goal text (right-justified)
        let goal_text = if is_nt {
            "Goal: at least ____ winners"
        } else {
            "Goal: at most ____ losers"
        };
        let goal_width = measurer.measure_width_mm(goal_text, HEADER_FONT_SIZE);
        let goal_x = right_edge - goal_width;
        layer.use_text(goal_text, HEADER_FONT_SIZE, Mm(goal_x), Mm(header_y), self.font);

        // Dummy (North hand) - positioned below header with gap, raised by DUMMY_RAISE
        let dummy_y = oy - HEADER_HEIGHT - ELEMENT_GAP + DUMMY_RAISE;
        dummy_renderer.render(layer, north, (Mm(content_x), Mm(dummy_y)));

        // Fan (South hand) - positioned below dummy, raised by FAN_RAISE
        let fan_renderer = self.fan_renderer(dummy_width, south);
        let (_, fan_height) = fan_renderer.dimensions(south);
        let visible_fan_height = fan_height * FAN_CROP_RATIO;
        let fan_y = dummy_y - nominal_dummy_height - ELEMENT_GAP + FAN_RAISE;
        fan_renderer.render(layer, south, (Mm(content_x), Mm(fan_y)));

        // Table below the VISIBLE portion of the fan (centered on dummy width), raised by TABLE_RAISE
        let (table_width, _) = if is_nt {
            self.winners_table_renderer().dimensions()
        } else {
            self.losers_table_renderer().dimensions()
        };
        let table_x = content_x + (dummy_width - table_width) / 2.0;
        let table_y = fan_y - visible_fan_height - ELEMENT_GAP + TABLE_RAISE;

        if is_nt {
            let table = self.winners_table_renderer();
            table.render(layer, (Mm(table_x), Mm(table_y)));
        } else {
            let table = self.losers_table_renderer();
            table.render(layer, (Mm(table_x), Mm(table_y)));
        }

        // Calculate total height used
        let (_, total_height) = self.dimensions(north, south, is_nt);
        total_height
    }
}
