use crate::error::RenderError;
use printpdf::{BuiltinFont, FontId, ParsedFont, PdfDocument};

// Only embed a minimal DejaVu Sans subset for suit symbols (♠♥♦♣)
// Regular text uses PDF builtin fonts (Times-Roman, Helvetica)
// The subsetted font is ~4KB vs 757KB for the full font
const DEJAVU_SANS_SUITS: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans-Suits.ttf");

/// Font family for text rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    SansSerif, // Helvetica (PDF builtin)
    Serif,     // Times-Roman (PDF builtin)
}

impl FontFamily {
    /// Determine font family from a font name
    pub fn from_name(name: &str) -> Self {
        let name_lower = name.to_lowercase();
        if name_lower.contains("times")
            || name_lower.contains("serif")
            || name_lower.contains("georgia")
            || name_lower.contains("palatino")
            || name_lower.contains("garamond")
        {
            FontFamily::Serif
        } else {
            // Default to sans-serif for Arial, Helvetica, and unknown fonts
            FontFamily::SansSerif
        }
    }
}

/// A set of builtin PDF fonts (regular, bold, italic, bold-italic) for one family
#[derive(Debug, Clone, Copy)]
pub struct BuiltinFontSet {
    pub regular: BuiltinFont,
    pub bold: BuiltinFont,
    pub italic: BuiltinFont,
    pub bold_italic: BuiltinFont,
}

impl BuiltinFontSet {
    /// Get the Times font set (serif)
    pub const fn times() -> Self {
        Self {
            regular: BuiltinFont::TimesRoman,
            bold: BuiltinFont::TimesBold,
            italic: BuiltinFont::TimesItalic,
            bold_italic: BuiltinFont::TimesBoldItalic,
        }
    }

    /// Get the Helvetica font set (sans-serif)
    pub const fn helvetica() -> Self {
        Self {
            regular: BuiltinFont::Helvetica,
            bold: BuiltinFont::HelveticaBold,
            italic: BuiltinFont::HelveticaOblique,
            bold_italic: BuiltinFont::HelveticaBoldOblique,
        }
    }
}

/// Legacy: A set of external fonts (for backwards compatibility during migration)
#[derive(Clone)]
pub struct FontSet {
    pub regular: FontId,
    pub bold: FontId,
    pub italic: FontId,
    pub bold_italic: FontId,
}

/// Font manager for PDF rendering
///
/// Uses PDF builtin fonts (Times-Roman, Helvetica) for regular text
/// and an embedded DejaVu Sans font for suit symbols (♠♥♦♣).
#[derive(Clone)]
pub struct FontManager {
    /// Suit symbol font (DejaVu Sans with Unicode suit symbols)
    pub symbol_font: FontId,
    /// Builtin sans-serif fonts (Helvetica family)
    pub sans: BuiltinFontSet,
    /// Builtin serif fonts (Times family)
    pub serif: BuiltinFontSet,
}

impl FontManager {
    /// Load fonts into the document
    ///
    /// Only loads DejaVu Sans for suit symbols - regular text uses PDF builtin fonts.
    pub fn new(doc: &mut PdfDocument) -> Result<Self, RenderError> {
        let mut warnings = Vec::new();

        // Load minimal DejaVu Sans subset for suit symbols only
        let symbol_font_parsed = ParsedFont::from_bytes(DEJAVU_SANS_SUITS, 0, &mut warnings)
            .ok_or_else(|| RenderError::FontLoad("Failed to parse DejaVuSans-Suits".to_string()))?;
        let symbol_font = doc.add_font(&symbol_font_parsed);

        Ok(Self {
            symbol_font,
            sans: BuiltinFontSet::helvetica(),
            serif: BuiltinFontSet::times(),
        })
    }

    /// Get the builtin font set for a given family
    pub fn builtin_family(&self, family: FontFamily) -> BuiltinFontSet {
        match family {
            FontFamily::SansSerif => self.sans,
            FontFamily::Serif => self.serif,
        }
    }

    /// Get the appropriate builtin font for a font specification
    pub fn builtin_for_spec(&self, family_name: &str, bold: bool, italic: bool) -> BuiltinFont {
        let family = FontFamily::from_name(family_name);
        let set = self.builtin_family(family);

        match (bold, italic) {
            (true, true) => set.bold_italic,
            (true, false) => set.bold,
            (false, true) => set.italic,
            (false, false) => set.regular,
        }
    }

    /// Get the appropriate builtin font set for a font specification
    pub fn builtin_set_for_spec(&self, spec: Option<&crate::model::FontSpec>) -> BuiltinFontSet {
        match spec {
            Some(s) => {
                let family = FontFamily::from_name(&s.family);
                self.builtin_family(family)
            }
            None => self.serif, // Default to serif (Times)
        }
    }

    /// Get the symbol font for suit symbols
    pub fn symbol_font(&self) -> &FontId {
        &self.symbol_font
    }
}

/// Convert a suit to its display character
pub fn suit_char(suit: &crate::model::Suit) -> char {
    match suit {
        crate::model::Suit::Spades => '\u{2660}', // ♠ BLACK SPADE SUIT
        crate::model::Suit::Hearts => '\u{2665}', // ♥ BLACK HEART SUIT
        crate::model::Suit::Diamonds => '\u{2666}', // ♦ BLACK DIAMOND SUIT
        crate::model::Suit::Clubs => '\u{2663}',  // ♣ BLACK CLUB SUIT
    }
}

/// Get a text representation for suits when Unicode isn't available
pub fn suit_text(suit: &crate::model::Suit) -> &'static str {
    match suit {
        crate::model::Suit::Spades => "S",
        crate::model::Suit::Hearts => "H",
        crate::model::Suit::Diamonds => "D",
        crate::model::Suit::Clubs => "C",
    }
}
