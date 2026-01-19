use crate::error::RenderError;
use printpdf::{FontId, ParsedFont, PdfDocument};

// Embed full fonts at compile time - printpdf 0.8 handles subsetting automatically
const DEJAVU_SANS_FULL: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans.ttf");
const DEJAVU_SANS_BOLD_FULL: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans-Bold.ttf");
const DEJAVU_SANS_OBLIQUE_FULL: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans-Oblique.ttf");
const DEJAVU_SANS_BOLD_OBLIQUE_FULL: &[u8] = include_bytes!("../../../assets/fonts/DejaVuSans-BoldOblique.ttf");

// TeX Gyre Termes - Times New Roman clone for professional typesetting
const TERMES_REGULAR_FULL: &[u8] = include_bytes!("../../../assets/fonts/texgyretermes-regular.ttf");
const TERMES_BOLD_FULL: &[u8] = include_bytes!("../../../assets/fonts/texgyretermes-bold.ttf");
const TERMES_ITALIC_FULL: &[u8] = include_bytes!("../../../assets/fonts/texgyretermes-italic.ttf");
const TERMES_BOLD_ITALIC_FULL: &[u8] =
    include_bytes!("../../../assets/fonts/texgyretermes-bolditalic.ttf");

/// Font family for a font set
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFamily {
    SansSerif, // Arial, Helvetica -> DejaVu Sans
    Serif,     // Times New Roman, Times -> DejaVu Serif
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

/// A set of fonts (regular, bold, italic, bold-italic) for one family
#[derive(Clone)]
pub struct FontSet {
    pub regular: FontId,
    pub bold: FontId,
    pub italic: FontId,
    pub bold_italic: FontId,
}

/// Font manager for PDF rendering
///
/// Uses embedded fonts with Unicode suit symbol support.
/// Provides sans-serif (DejaVu Sans) and serif (TeX Gyre Termes) font families.
#[derive(Clone)]
pub struct FontManager {
    pub sans: FontSet,
    pub serif: FontSet,
    // Aliases for backward compatibility - points to serif (default for bridge docs)
    pub regular: FontId,
    pub bold: FontId,
    pub italic: FontId,
}

impl FontManager {
    /// Load fonts into the document
    ///
    /// printpdf 0.8 handles subsetting automatically when saving the PDF,
    /// so we just load the full fonts here.
    pub fn new(doc: &mut PdfDocument) -> Result<Self, RenderError> {
        let mut warnings = Vec::new();

        // Load DejaVu Sans family
        let sans_regular_font = ParsedFont::from_bytes(DEJAVU_SANS_FULL, 0, &mut warnings)
            .ok_or_else(|| RenderError::FontLoad("Failed to parse DejaVuSans".to_string()))?;
        let sans_regular = doc.add_font(&sans_regular_font);

        let sans_bold_font = ParsedFont::from_bytes(DEJAVU_SANS_BOLD_FULL, 0, &mut warnings)
            .ok_or_else(|| RenderError::FontLoad("Failed to parse DejaVuSans-Bold".to_string()))?;
        let sans_bold = doc.add_font(&sans_bold_font);

        let sans_oblique_font = ParsedFont::from_bytes(DEJAVU_SANS_OBLIQUE_FULL, 0, &mut warnings)
            .ok_or_else(|| RenderError::FontLoad("Failed to parse DejaVuSans-Oblique".to_string()))?;
        let sans_italic = doc.add_font(&sans_oblique_font);

        let sans_bold_oblique_font = ParsedFont::from_bytes(DEJAVU_SANS_BOLD_OBLIQUE_FULL, 0, &mut warnings)
            .ok_or_else(|| RenderError::FontLoad("Failed to parse DejaVuSans-BoldOblique".to_string()))?;
        let sans_bold_italic = doc.add_font(&sans_bold_oblique_font);

        // Load TeX Gyre Termes family (serif)
        let serif_regular_font = ParsedFont::from_bytes(TERMES_REGULAR_FULL, 0, &mut warnings)
            .ok_or_else(|| {
                RenderError::FontLoad("Failed to parse TeXGyreTermes-Regular".to_string())
            })?;
        let serif_regular = doc.add_font(&serif_regular_font);

        let serif_bold_font = ParsedFont::from_bytes(TERMES_BOLD_FULL, 0, &mut warnings)
            .ok_or_else(|| {
                RenderError::FontLoad("Failed to parse TeXGyreTermes-Bold".to_string())
            })?;
        let serif_bold = doc.add_font(&serif_bold_font);

        let serif_italic_font = ParsedFont::from_bytes(TERMES_ITALIC_FULL, 0, &mut warnings)
            .ok_or_else(|| {
                RenderError::FontLoad("Failed to parse TeXGyreTermes-Italic".to_string())
            })?;
        let serif_italic = doc.add_font(&serif_italic_font);

        let serif_bold_italic_font =
            ParsedFont::from_bytes(TERMES_BOLD_ITALIC_FULL, 0, &mut warnings).ok_or_else(|| {
                RenderError::FontLoad("Failed to parse TeXGyreTermes-BoldItalic".to_string())
            })?;
        let serif_bold_italic = doc.add_font(&serif_bold_italic_font);

        Ok(Self {
            sans: FontSet {
                regular: sans_regular.clone(),
                bold: sans_bold.clone(),
                italic: sans_italic.clone(),
                bold_italic: sans_bold_italic,
            },
            serif: FontSet {
                regular: serif_regular.clone(),
                bold: serif_bold.clone(),
                italic: serif_italic.clone(),
                bold_italic: serif_bold_italic.clone(),
            },
            // Default aliases point to serif (Times New Roman is default for bridge docs)
            regular: serif_regular,
            bold: serif_bold,
            italic: serif_italic,
        })
    }

    /// Get the font set for a given family
    pub fn family(&self, family: FontFamily) -> &FontSet {
        match family {
            FontFamily::SansSerif => &self.sans,
            FontFamily::Serif => &self.serif,
        }
    }

    /// Get the appropriate font for a font specification
    pub fn for_spec(&self, family_name: &str, bold: bool, italic: bool) -> &FontId {
        let family = FontFamily::from_name(family_name);
        let set = self.family(family);

        if bold {
            &set.bold
        } else if italic {
            &set.italic
        } else {
            &set.regular
        }
    }

    /// Get the appropriate font set for a font specification
    pub fn set_for_spec(&self, spec: Option<&crate::model::FontSpec>) -> &FontSet {
        match spec {
            Some(s) => {
                let family = FontFamily::from_name(&s.family);
                self.family(family)
            }
            None => &self.serif, // Default to serif
        }
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
