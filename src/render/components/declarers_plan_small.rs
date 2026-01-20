//! Declarer's plan small component
//!
//! Renders a compact layout for one quadrant of a page showing:
//! - North hand in dummy view (top)
//! - South hand in fan view (below, with opening lead between)
//! - Opening lead card rotated 90° CW (between hands)
//! - Deal number and contract text (next to opening lead)
//! - Goal text with winners/losers blank (right-justified)
//! - Winners or Losers table (below south hand)
//!   - NT contracts: Winners table
//!   - Suit contracts: Losers table

use printpdf::{Color, FontId, Mm};

use crate::model::{Board, Card, Hand, Suit};
use crate::render::components::{
    DummyRenderer, FanRenderer, LosersTableRenderer, WinnersTableRenderer,
};
use crate::render::helpers::card_assets::{CardAssets, CARD_HEIGHT_MM, CARD_WIDTH_MM};
use crate::render::helpers::colors::{SuitColors, BLACK};
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics;

/// Gap between elements in mm
const ELEMENT_GAP: f32 = 4.0;

/// Portion of fan height to display (crop from bottom)
const FAN_CROP_RATIO: f32 = 0.5;

/// Nominal number of cards in a suit for calculating fixed dummy height
const NOMINAL_SUIT_LENGTH: usize = 5;

/// Scale factor for the opening lead card relative to other cards
const LEAD_SCALE_RATIO: f32 = 0.66;

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
            dummy_overlap: 0.20,
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

    /// Calculate the height of the opening lead gap (space between dummy and fan for the lead card)
    fn lead_gap_height(&self) -> f32 {
        // After 90° CCW rotation, card_width becomes visual height
        let (card_width, _) = self.card_assets.card_size_mm(self.card_scale * LEAD_SCALE_RATIO);
        // Gap includes the card height plus some padding
        card_width + ELEMENT_GAP
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

        // Total height: nominal dummy + lead gap + visible fan portion + gap + table
        let lead_gap = self.lead_gap_height();
        let height = nominal_dummy_height + lead_gap + visible_fan_height + ELEMENT_GAP + table_height;

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

        // Table header height used for vertical adjustments
        let table_header_height = 10.0;

        // Content starts at origin
        let content_x = ox;

        // Render dummy (North hand)
        let dummy_renderer = self.dummy_renderer();
        let (dummy_width, _actual_dummy_height) = dummy_renderer.dimensions(north);
        let dummy_x = content_x;
        let dummy_y = oy;
        dummy_renderer.render(layer, north, (Mm(dummy_x), Mm(dummy_y)));

        // Use nominal dummy height for positioning lower elements (consistent regardless of actual hand)
        let nominal_dummy_height = self.nominal_dummy_height();
        let lead_gap = self.lead_gap_height();

        // Render fan (South hand) - scaled to match dummy width
        // Position is based on nominal dummy height + lead gap (for opening lead between)
        // Move up by table_header_height to squeeze layout
        let fan_renderer = self.fan_renderer(dummy_width, south);
        let (_, fan_height) = fan_renderer.dimensions(south);
        let visible_fan_height = fan_height * FAN_CROP_RATIO;
        let fan_x = content_x;
        // Fan starts below nominal dummy position + lead gap, moved up by header height
        let fan_y = oy - nominal_dummy_height - lead_gap + table_header_height;
        fan_renderer.render(layer, south, (Mm(fan_x), Mm(fan_y)));

        // Render table below the VISIBLE portion of the fan (this covers the bottom of the fan)
        // Table is centered on the dummy width, moved up by 2x header height total
        let (table_width, _) = if is_nt {
            self.winners_table_renderer().dimensions()
        } else {
            self.losers_table_renderer().dimensions()
        };
        let table_x = content_x + (dummy_width - table_width) / 2.0; // Center on dummy
        let table_y = fan_y - visible_fan_height - ELEMENT_GAP + table_header_height; // Move up by header height

        if is_nt {
            let table = self.winners_table_renderer();
            table.render(layer, (Mm(table_x), Mm(table_y)));
        } else {
            let table = self.losers_table_renderer();
            table.render(layer, (Mm(table_x), Mm(table_y)));
        }

        // Calculate lead card dimensions for positioning
        let lead_scale = self.card_scale * LEAD_SCALE_RATIO;
        let (lead_card_width, lead_card_height) = self.card_assets.card_size_mm(lead_scale);
        // Offset lead card 1/4 card width to the right
        let lead_x_offset = CARD_WIDTH_MM * self.card_scale * 0.25;

        // Y position for lead gap area (moved up by header height)
        let nominal_dummy_bottom = oy - nominal_dummy_height;
        let adjusted_gap_top = nominal_dummy_bottom + table_header_height;
        let gap_center_y = adjusted_gap_top - lead_gap / 2.0;

        // Render opening lead card (rotated 90° CW) if present
        // Lower it by 1/4 of pre-rotated card width
        let lead_y_offset = CARD_WIDTH_MM * self.card_scale * 0.25;
        if let Some(card) = opening_lead {
            // After 90° CW rotation (-90° CCW) around bottom-left corner:
            // - Card extends RIGHT by card_height and DOWN by card_width from pivot
            // Visual height = lead_card_width (extends downward)
            let card_pivot_y = gap_center_y + lead_card_width / 2.0 - lead_y_offset;
            // X position: offset 1/4 card width to the right
            let card_pivot_x = content_x + lead_x_offset;

            let transform = self.card_assets.transform_at_rotated(
                card_pivot_x,
                card_pivot_y,
                lead_scale,
                -90.0, // 90° CW (clockwise)
            );
            layer.use_xobject(
                self.card_assets.get(card.suit, card.rank).clone(),
                transform,
            );
        }

        // Render deal number and contract text (to the right of the opening lead)
        // Font size increased by 30%
        let info_font_size = 13.0;
        let line_height = info_font_size * 0.4; // Approximate line height in mm
        // Position text to the right of the lead card
        let text_x = content_x + lead_x_offset + lead_card_height + 2.0; // 2mm gap after card
        // Position so lower line is just above the fan bounding box top
        let text_bottom_y = fan_y + 1.0; // 1mm above fan top
        let text_top_y = text_bottom_y + line_height + 2.0;

        layer.set_fill_color(Color::Rgb(BLACK));

        if let Some(deal_num) = deal_number {
            let deal_text = format!("Deal {}", deal_num);
            layer.use_text(&deal_text, info_font_size, Mm(text_x), Mm(text_top_y), self.bold_font);
        }

        if let Some(contract) = contract_str {
            // Use symbol_font for contract to render suit symbols properly
            layer.use_text(contract, info_font_size, Mm(text_x), Mm(text_bottom_y), self.symbol_font);
        }

        // Render "Goal:" text box (right-justified with dummy width)
        let goal_text = "Goal:";
        let goal_line2 = if is_nt {
            ">= ______ winners"
        } else {
            "<= ______ losers"
        };

        let measurer = text_metrics::get_serif_measurer();
        let goal_width = measurer.measure_width_mm(goal_text, info_font_size);
        let goal_line2_width = measurer.measure_width_mm(goal_line2, info_font_size);

        // Right edge is at content_x + dummy_width
        let right_edge = content_x + dummy_width;
        let goal_x = right_edge - goal_width.max(goal_line2_width);

        layer.use_text(goal_text, info_font_size, Mm(goal_x), Mm(text_top_y), self.bold_font);
        // Right-justify the second line
        let goal_line2_x = right_edge - goal_line2_width;
        layer.use_text(goal_line2, info_font_size, Mm(goal_line2_x), Mm(text_bottom_y), self.font);

        // Calculate total height used
        let (_, total_height) = self.dimensions(north, south, is_nt);
        total_height
    }
}
