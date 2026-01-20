//! Dummy card display renderer
//!
//! Renders a hand as stacked cards by suit, like dummy's hand laid out on the table.
//! Each suit is a vertical stack with only the bottom card fully visible.

use printpdf::Mm;

use crate::model::{Hand, Rank, Suit};
use crate::render::helpers::card_assets::{CardAssets, CARD_HEIGHT_MM};
use crate::render::helpers::layer::LayerBuilder;

/// Gap between suit stacks in mm
const SUIT_GAP_MM: f32 = 2.0;

/// Default portion of card visible when overlapped (15% of card height)
const DEFAULT_OVERLAP_RATIO: f32 = 0.15;

/// Default suit order with alternating colors: Spades (black), Hearts (red), Clubs (black), Diamonds (red)
const ALTERNATING_SUIT_ORDER: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Clubs, Suit::Diamonds];

/// Standard suit order: Spades, Hearts, Diamonds, Clubs
const STANDARD_SUIT_ORDER: [Suit; 4] = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];

/// Renderer for dummy-style card display (vertical stacks by suit)
pub struct DummyRenderer<'a> {
    card_assets: &'a CardAssets,
    scale: f32,
    overlap_ratio: f32,
    first_suit: Suit,
    alternate_colors: bool,
    /// Whether to draw a debug rectangle showing the bounding box
    show_bounds: bool,
}

impl<'a> DummyRenderer<'a> {
    /// Create a new dummy renderer with the given card assets and scale factor
    ///
    /// Uses default settings: spades first, alternating colors
    pub fn new(card_assets: &'a CardAssets, scale: f32) -> Self {
        Self {
            card_assets,
            scale,
            overlap_ratio: DEFAULT_OVERLAP_RATIO,
            first_suit: Suit::Spades,
            alternate_colors: true,
            show_bounds: false,
        }
    }

    /// Create a new dummy renderer with custom overlap ratio
    ///
    /// overlap_ratio is the portion of card visible when overlapped (0.0 to 1.0)
    pub fn with_overlap(card_assets: &'a CardAssets, scale: f32, overlap_ratio: f32) -> Self {
        Self {
            card_assets,
            scale,
            overlap_ratio,
            first_suit: Suit::Spades,
            alternate_colors: true,
            show_bounds: false,
        }
    }

    /// Set the first suit to display (rotates the suit order)
    pub fn first_suit(mut self, suit: Suit) -> Self {
        self.first_suit = suit;
        self
    }

    /// Set whether to alternate suit colors (default: true)
    ///
    /// When true: uses order like S-H-C-D (black-red-black-red)
    /// When false: uses order like S-H-D-C
    pub fn alternate_colors(mut self, alternate: bool) -> Self {
        self.alternate_colors = alternate;
        self
    }

    /// Set whether to show a debug bounding box rectangle (default: false)
    pub fn show_bounds(mut self, show: bool) -> Self {
        self.show_bounds = show;
        self
    }

    /// Get the suit order based on configuration
    fn suit_order(&self) -> [Suit; 4] {
        let base_order = if self.alternate_colors {
            ALTERNATING_SUIT_ORDER
        } else {
            STANDARD_SUIT_ORDER
        };

        // Find the index of the first suit in the base order
        let start_idx = base_order
            .iter()
            .position(|&s| s == self.first_suit)
            .unwrap_or(0);

        // Rotate the order so first_suit is first
        [
            base_order[start_idx],
            base_order[(start_idx + 1) % 4],
            base_order[(start_idx + 2) % 4],
            base_order[(start_idx + 3) % 4],
        ]
    }

    /// Get the scaled card dimensions
    pub fn card_size(&self) -> (f32, f32) {
        self.card_assets.card_size_mm(self.scale)
    }

    /// Calculate the visible height for overlapped cards
    fn visible_height(&self) -> f32 {
        CARD_HEIGHT_MM * self.scale * self.overlap_ratio
    }

    /// Calculate the total dimensions needed to render a hand
    ///
    /// Returns (width, height) in mm
    pub fn dimensions(&self, hand: &Hand) -> (f32, f32) {
        let (card_width, card_height) = self.card_size();
        let visible_height = self.visible_height();

        // Width: 4 suit stacks with gaps between them
        let width = 4.0 * card_width + 3.0 * SUIT_GAP_MM;

        // Height: find the tallest stack
        let suits = self.suit_order();
        let max_cards = suits
            .iter()
            .map(|suit| hand.holding(*suit).len())
            .max()
            .unwrap_or(0);

        // Height: one full card + overlapped portions for remaining cards
        let height = if max_cards == 0 {
            0.0
        } else {
            card_height + (max_cards - 1) as f32 * visible_height
        };

        (width, height)
    }

    /// Render a hand in dummy layout
    ///
    /// Origin is the top-left corner of the display area.
    /// Cards are arranged in 4 columns based on suit order configuration.
    /// Within each column, cards are stacked vertically with highest rank at top.
    ///
    /// Returns the height used.
    pub fn render(&self, layer: &mut LayerBuilder, hand: &Hand, origin: (Mm, Mm)) -> f32 {
        let (card_width, card_height) = self.card_size();
        let visible_height = self.visible_height();

        // Draw bounding box if requested
        if self.show_bounds {
            let (width, height) = self.dimensions(hand);
            layer.set_outline_color(printpdf::Color::Rgb(printpdf::Rgb::new(
                1.0, 0.0, 0.0, None,
            )));
            layer.set_outline_thickness(1.0);
            layer.add_rect(
                origin.0,
                Mm(origin.1 .0 - height),
                Mm(origin.0 .0 + width),
                origin.1,
                printpdf::PaintMode::Stroke,
            );
        }

        let suits = self.suit_order();

        for (col, suit) in suits.iter().enumerate() {
            let col_x = origin.0 .0 + col as f32 * (card_width + SUIT_GAP_MM);
            let holding = hand.holding(*suit);

            if holding.is_empty() {
                continue;
            }

            // Get ranks sorted high to low (BTreeSet already gives this order)
            let ranks: Vec<Rank> = holding.ranks.iter().copied().collect();

            // Render cards from top (highest rank) to bottom (lowest rank)
            // so that lower cards render on top and naturally cover the cards above.
            // The bottom card (lowest rank, last rendered) will be fully visible.
            for (i, rank) in ranks.iter().enumerate() {
                let card_top_y = origin.1 .0 - i as f32 * visible_height;

                // The bottom of this card
                let card_bottom_y = card_top_y - card_height;

                // Place the card (transform is at bottom-left)
                let transform = self
                    .card_assets
                    .transform_at(col_x, card_bottom_y, self.scale);
                layer.use_xobject(self.card_assets.get(*suit, *rank).clone(), transform);
            }
        }

        // Return the height used
        let (_, height) = self.dimensions(hand);
        height
    }
}
