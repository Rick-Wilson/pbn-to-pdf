//! Glyph collection for font subsetting
//!
//! Analyzes boards before rendering to determine which characters are needed,
//! enabling runtime font subsetting for minimal PDF file sizes.

use crate::config::Settings;
use crate::model::{Board, CommentaryBlock, FormattedText, TextSpan};
use std::collections::HashSet;

/// Fixed characters used in various regions of the PDF
pub mod fixed_glyphs {
    /// Characters used in compass directions (bold)
    pub const COMPASS: &str = "NESW";

    /// Characters used in hand diagrams (ranks + void marker)
    pub const HAND_RANKS: &str = "AKQJT98765432-";

    /// Characters used in HCP display (bold digits)
    pub const HCP_DIGITS: &str = "0123456789";

    /// Suit symbols (used with DejaVu Sans)
    pub const SUIT_SYMBOLS: &str = "\u{2660}\u{2665}\u{2666}\u{2663}"; // ♠♥♦♣

    /// Characters for bidding table headers and passes
    pub const BIDDING: &str = "WNESPassDblRdblAll ";

    /// Characters for contract display
    pub const CONTRACT: &str = "1234567NTXby ";

    /// Characters for vulnerability display
    pub const VULNERABILITY: &str = "NoneNS-onlyEW-onlyBoth";

    /// Characters for deal/dealer labels
    pub const LABELS: &str = "Deal Deals";

    /// Characters for lead display
    pub const LEAD: &str = "Lead: ";
}

/// Tracks which characters are used by each font category
#[derive(Debug, Default)]
pub struct GlyphSets {
    /// Characters for sans-serif regular (suit symbols)
    pub sans_regular: HashSet<char>,
    /// Characters for sans-serif bold (compass)
    pub sans_bold: HashSet<char>,
    /// Characters for serif regular (commentary, labels)
    pub serif_regular: HashSet<char>,
    /// Characters for serif bold (commentary bold, HCP)
    pub serif_bold: HashSet<char>,
    /// Characters for serif italic (commentary italic)
    pub serif_italic: HashSet<char>,
    /// Characters for serif bold-italic (deal number)
    pub serif_bold_italic: HashSet<char>,
}

impl GlyphSets {
    /// Create glyph sets with fixed characters pre-populated
    pub fn with_fixed_glyphs() -> Self {
        let mut sets = Self::default();

        // Sans regular: suit symbols
        sets.sans_regular.extend(fixed_glyphs::SUIT_SYMBOLS.chars());

        // Sans bold: compass directions
        sets.sans_bold.extend(fixed_glyphs::COMPASS.chars());

        // Serif regular: hand ranks, bidding, contract, labels, lead
        sets.serif_regular.extend(fixed_glyphs::HAND_RANKS.chars());
        sets.serif_regular.extend(fixed_glyphs::BIDDING.chars());
        sets.serif_regular.extend(fixed_glyphs::CONTRACT.chars());
        sets.serif_regular
            .extend(fixed_glyphs::VULNERABILITY.chars());
        sets.serif_regular.extend(fixed_glyphs::LABELS.chars());
        sets.serif_regular.extend(fixed_glyphs::LEAD.chars());

        // Serif bold: HCP digits
        sets.serif_bold.extend(fixed_glyphs::HCP_DIGITS.chars());

        // Serif bold-italic: deal numbers
        sets.serif_bold_italic
            .extend(fixed_glyphs::HCP_DIGITS.chars());
        sets.serif_bold_italic.extend("Deal ".chars());

        sets
    }

    /// Convert to strings for subsetting
    pub fn to_strings(&self) -> GlyphStrings {
        GlyphStrings {
            sans_regular: self.sans_regular.iter().collect(),
            sans_bold: self.sans_bold.iter().collect(),
            serif_regular: self.serif_regular.iter().collect(),
            serif_bold: self.serif_bold.iter().collect(),
            serif_italic: self.serif_italic.iter().collect(),
            serif_bold_italic: self.serif_bold_italic.iter().collect(),
        }
    }
}

/// Glyph sets as strings (for passing to subsetter)
#[derive(Debug)]
pub struct GlyphStrings {
    pub sans_regular: String,
    pub sans_bold: String,
    pub serif_regular: String,
    pub serif_bold: String,
    pub serif_italic: String,
    pub serif_bold_italic: String,
}

/// Collects glyphs used by a set of boards
pub struct GlyphCollector {
    sets: GlyphSets,
}

impl GlyphCollector {
    /// Create a new collector with fixed glyphs pre-populated
    pub fn new() -> Self {
        Self {
            sets: GlyphSets::with_fixed_glyphs(),
        }
    }

    /// Analyze boards and collect all used glyphs
    pub fn collect_from_boards(&mut self, boards: &[Board], _settings: &Settings) {
        for board in boards {
            self.collect_from_board(board);
        }
    }

    /// Collect glyphs from a single board
    fn collect_from_board(&mut self, board: &Board) {
        // Board number (serif bold-italic)
        if let Some(num) = board.number {
            for c in num.to_string().chars() {
                self.sets.serif_bold_italic.insert(c);
            }
        }

        // Dealer name (serif regular) - already covered by fixed glyphs

        // Commentary (the main variable content)
        for block in &board.commentary {
            self.collect_from_commentary(block);
        }

        // Auction annotations if present
        if let Some(ref auction) = board.auction {
            for call in &auction.calls {
                if let Some(ref annotation) = call.annotation {
                    // Annotations use serif regular
                    for c in annotation.chars() {
                        self.sets.serif_regular.insert(c);
                    }
                }
            }
        }
    }

    /// Collect glyphs from commentary block
    fn collect_from_commentary(&mut self, block: &CommentaryBlock) {
        self.collect_from_formatted_text(&block.content);
    }

    /// Collect glyphs from formatted text
    fn collect_from_formatted_text(&mut self, text: &FormattedText) {
        for span in &text.spans {
            match span {
                TextSpan::Plain(s) => {
                    for c in s.chars() {
                        self.sets.serif_regular.insert(c);
                    }
                }
                TextSpan::Bold(s) => {
                    for c in s.chars() {
                        self.sets.serif_bold.insert(c);
                    }
                }
                TextSpan::Italic(s) => {
                    for c in s.chars() {
                        self.sets.serif_italic.insert(c);
                    }
                }
                TextSpan::SuitSymbol(_) => {
                    // Suit symbols already in sans_regular from fixed glyphs
                }
                TextSpan::CardRef { rank, .. } => {
                    // Rank character in serif regular
                    self.sets.serif_regular.insert(rank.to_char());
                }
                TextSpan::LineBreak => {}
            }
        }
    }

    /// Get the collected glyph sets
    pub fn into_sets(self) -> GlyphSets {
        self.sets
    }

    /// Get glyph strings for subsetting
    pub fn into_strings(self) -> GlyphStrings {
        self.sets.to_strings()
    }
}

impl Default for GlyphCollector {
    fn default() -> Self {
        Self::new()
    }
}
