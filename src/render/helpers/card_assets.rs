//! Card SVG asset management
//!
//! This module handles loading and caching of playing card SVG images
//! as PDF XObjects for efficient reuse in rendering.

use printpdf::{PdfDocument, PdfWarnMsg, Pt, Px, Svg, XObjectId, XObjectTransform};
use std::collections::HashMap;

use crate::model::{Rank, Suit, RANKS_DISPLAY_ORDER, SUITS_DISPLAY_ORDER};

/// Card dimensions based on actual SVG assets
/// SVG viewport: 167.0869141pt × 242.6669922pt
/// Converted to mm: 58.94mm × 85.61mm (1pt = 25.4/72 mm)
pub const CARD_WIDTH_MM: f32 = 58.94;
pub const CARD_HEIGHT_MM: f32 = 85.61;

/// DPI used for SVG parsing (matches printpdf default)
const SVG_DPI: f32 = 300.0;

/// Manages SVG card images as XObjects for PDF rendering
pub struct CardAssets {
    cards: HashMap<(Suit, Rank), XObjectId>,
}

impl CardAssets {
    /// Load all 52 card SVGs and register them as XObjects in the document
    ///
    /// SVGs are embedded at compile time from assets/cards/
    pub fn load(doc: &mut PdfDocument) -> Result<Self, CardLoadError> {
        let mut cards = HashMap::new();
        let mut warnings: Vec<PdfWarnMsg> = Vec::new();

        // Load all 52 cards
        for suit in SUITS_DISPLAY_ORDER {
            for rank in RANKS_DISPLAY_ORDER {
                let svg_content = get_card_svg(suit, rank)?;
                let xobject = Svg::parse(svg_content, &mut warnings).map_err(|e| {
                    CardLoadError::SvgParseError {
                        suit,
                        rank,
                        message: e,
                    }
                })?;
                let id = doc.add_xobject(&xobject);
                cards.insert((suit, rank), id);
            }
        }

        Ok(Self { cards })
    }

    /// Get the XObjectId for a specific card
    pub fn get(&self, suit: Suit, rank: Rank) -> &XObjectId {
        self.cards
            .get(&(suit, rank))
            .expect("All cards should be loaded")
    }

    /// Get card dimensions in mm at a given scale factor
    pub fn card_size_mm(&self, scale: f32) -> (f32, f32) {
        (CARD_WIDTH_MM * scale, CARD_HEIGHT_MM * scale)
    }

    /// Create an XObjectTransform for placing a card at a given position and scale
    ///
    /// Position is the bottom-left corner of the card in mm
    pub fn transform_at(&self, x_mm: f32, y_mm: f32, scale: f32) -> XObjectTransform {
        self.transform_at_rotated(x_mm, y_mm, scale, 0.0)
    }

    /// Create an XObjectTransform for placing a card at a given position, scale, and rotation
    ///
    /// Position is the bottom-left corner of the card in mm.
    /// Rotation is in degrees, counter-clockwise around the card's bottom-left corner.
    pub fn transform_at_rotated(
        &self,
        x_mm: f32,
        y_mm: f32,
        scale: f32,
        rotate_degrees: f32,
    ) -> XObjectTransform {
        // Convert mm to points (1 mm = 2.834645669 pt)
        let mm_to_pt = 2.834_645_7;

        let rotate = if rotate_degrees.abs() < 0.001 {
            None
        } else {
            Some(printpdf::XObjectRotation {
                angle_ccw_degrees: rotate_degrees,
                rotation_center_x: Px(0),
                rotation_center_y: Px(0),
            })
        };

        XObjectTransform {
            translate_x: Some(Pt(x_mm * mm_to_pt)),
            translate_y: Some(Pt(y_mm * mm_to_pt)),
            scale_x: Some(scale),
            scale_y: Some(scale),
            rotate,
            dpi: Some(SVG_DPI),
        }
    }
}

/// Error type for card loading failures
#[derive(Debug)]
pub enum CardLoadError {
    MissingCard {
        suit: Suit,
        rank: Rank,
    },
    SvgParseError {
        suit: Suit,
        rank: Rank,
        message: String,
    },
}

impl std::fmt::Display for CardLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardLoadError::MissingCard { suit, rank } => {
                write!(f, "Missing card SVG: {} of {:?}", rank.to_char(), suit)
            }
            CardLoadError::SvgParseError {
                suit,
                rank,
                message,
            } => {
                write!(
                    f,
                    "Failed to parse SVG for {} of {:?}: {}",
                    rank.to_char(),
                    suit,
                    message
                )
            }
        }
    }
}

impl std::error::Error for CardLoadError {}

/// Get the embedded SVG content for a specific card
fn get_card_svg(suit: Suit, rank: Rank) -> Result<&'static str, CardLoadError> {
    let svg = match (suit, rank) {
        // Spades
        (Suit::Spades, Rank::Ace) => include_str!("../../../assets/cards/ace_of_spades.svg"),
        (Suit::Spades, Rank::King) => include_str!("../../../assets/cards/king_of_spades.svg"),
        (Suit::Spades, Rank::Queen) => include_str!("../../../assets/cards/queen_of_spades.svg"),
        (Suit::Spades, Rank::Jack) => include_str!("../../../assets/cards/jack_of_spades.svg"),
        (Suit::Spades, Rank::Ten) => include_str!("../../../assets/cards/10_of_spades.svg"),
        (Suit::Spades, Rank::Nine) => include_str!("../../../assets/cards/9_of_spades.svg"),
        (Suit::Spades, Rank::Eight) => include_str!("../../../assets/cards/8_of_spades.svg"),
        (Suit::Spades, Rank::Seven) => include_str!("../../../assets/cards/7_of_spades.svg"),
        (Suit::Spades, Rank::Six) => include_str!("../../../assets/cards/6_of_spades.svg"),
        (Suit::Spades, Rank::Five) => include_str!("../../../assets/cards/5_of_spades.svg"),
        (Suit::Spades, Rank::Four) => include_str!("../../../assets/cards/4_of_spades.svg"),
        (Suit::Spades, Rank::Three) => include_str!("../../../assets/cards/3_of_spades.svg"),
        (Suit::Spades, Rank::Two) => include_str!("../../../assets/cards/2_of_spades.svg"),

        // Hearts
        (Suit::Hearts, Rank::Ace) => include_str!("../../../assets/cards/ace_of_hearts.svg"),
        (Suit::Hearts, Rank::King) => include_str!("../../../assets/cards/king_of_hearts.svg"),
        (Suit::Hearts, Rank::Queen) => include_str!("../../../assets/cards/queen_of_hearts.svg"),
        (Suit::Hearts, Rank::Jack) => include_str!("../../../assets/cards/jack_of_hearts.svg"),
        (Suit::Hearts, Rank::Ten) => include_str!("../../../assets/cards/10_of_hearts.svg"),
        (Suit::Hearts, Rank::Nine) => include_str!("../../../assets/cards/9_of_hearts.svg"),
        (Suit::Hearts, Rank::Eight) => include_str!("../../../assets/cards/8_of_hearts.svg"),
        (Suit::Hearts, Rank::Seven) => include_str!("../../../assets/cards/7_of_hearts.svg"),
        (Suit::Hearts, Rank::Six) => include_str!("../../../assets/cards/6_of_hearts.svg"),
        (Suit::Hearts, Rank::Five) => include_str!("../../../assets/cards/5_of_hearts.svg"),
        (Suit::Hearts, Rank::Four) => include_str!("../../../assets/cards/4_of_hearts.svg"),
        (Suit::Hearts, Rank::Three) => include_str!("../../../assets/cards/3_of_hearts.svg"),
        (Suit::Hearts, Rank::Two) => include_str!("../../../assets/cards/2_of_hearts.svg"),

        // Diamonds
        (Suit::Diamonds, Rank::Ace) => include_str!("../../../assets/cards/ace_of_diamonds.svg"),
        (Suit::Diamonds, Rank::King) => include_str!("../../../assets/cards/king_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Queen) => {
            include_str!("../../../assets/cards/queen_of_diamonds.svg")
        }
        (Suit::Diamonds, Rank::Jack) => include_str!("../../../assets/cards/jack_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Ten) => include_str!("../../../assets/cards/10_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Nine) => include_str!("../../../assets/cards/9_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Eight) => include_str!("../../../assets/cards/8_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Seven) => include_str!("../../../assets/cards/7_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Six) => include_str!("../../../assets/cards/6_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Five) => include_str!("../../../assets/cards/5_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Four) => include_str!("../../../assets/cards/4_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Three) => include_str!("../../../assets/cards/3_of_diamonds.svg"),
        (Suit::Diamonds, Rank::Two) => include_str!("../../../assets/cards/2_of_diamonds.svg"),

        // Clubs
        (Suit::Clubs, Rank::Ace) => include_str!("../../../assets/cards/ace_of_clubs.svg"),
        (Suit::Clubs, Rank::King) => include_str!("../../../assets/cards/king_of_clubs.svg"),
        (Suit::Clubs, Rank::Queen) => include_str!("../../../assets/cards/queen_of_clubs.svg"),
        (Suit::Clubs, Rank::Jack) => include_str!("../../../assets/cards/jack_of_clubs.svg"),
        (Suit::Clubs, Rank::Ten) => include_str!("../../../assets/cards/10_of_clubs.svg"),
        (Suit::Clubs, Rank::Nine) => include_str!("../../../assets/cards/9_of_clubs.svg"),
        (Suit::Clubs, Rank::Eight) => include_str!("../../../assets/cards/8_of_clubs.svg"),
        (Suit::Clubs, Rank::Seven) => include_str!("../../../assets/cards/7_of_clubs.svg"),
        (Suit::Clubs, Rank::Six) => include_str!("../../../assets/cards/6_of_clubs.svg"),
        (Suit::Clubs, Rank::Five) => include_str!("../../../assets/cards/5_of_clubs.svg"),
        (Suit::Clubs, Rank::Four) => include_str!("../../../assets/cards/4_of_clubs.svg"),
        (Suit::Clubs, Rank::Three) => include_str!("../../../assets/cards/3_of_clubs.svg"),
        (Suit::Clubs, Rank::Two) => include_str!("../../../assets/cards/2_of_clubs.svg"),
    };
    Ok(svg)
}
