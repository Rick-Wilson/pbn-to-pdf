use crate::error::RenderError;
use printpdf::{IndirectFontRef, PdfDocumentReference};
use std::io::Cursor;

// Embed the DejaVu Sans fonts at compile time (for Arial, Helvetica, sans-serif)
const DEJAVU_SANS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
const DEJAVU_SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");
const DEJAVU_SANS_OBLIQUE: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Oblique.ttf");

// Embed TeX Gyre Termes fonts at compile time (for Times New Roman, Times, serif)
// TeX Gyre Termes is a high-quality Times clone designed for professional typesetting
const TERMES_REGULAR: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-regular.ttf");
const TERMES_BOLD: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-bold.ttf");
const TERMES_ITALIC: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-italic.ttf");
const TERMES_BOLD_ITALIC: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-bolditalic.ttf");

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
pub struct FontSet {
    pub regular: IndirectFontRef,
    pub bold: IndirectFontRef,
    pub italic: IndirectFontRef,
    pub bold_italic: IndirectFontRef,
}

/// Font manager for PDF rendering
///
/// Uses embedded fonts with Unicode suit symbol support.
/// Provides sans-serif (DejaVu Sans) and serif (TeX Gyre Termes) font families.
pub struct FontManager {
    pub sans: FontSet,
    pub serif: FontSet,
    // Aliases for backward compatibility - points to serif (default for bridge docs)
    pub regular: IndirectFontRef,
    pub bold: IndirectFontRef,
    pub italic: IndirectFontRef,
}

impl FontManager {
    /// Load embedded fonts into the PDF document
    pub fn new(doc: &PdfDocumentReference) -> Result<Self, RenderError> {
        // Load DejaVu Sans family
        let sans_regular = doc
            .add_external_font(Cursor::new(DEJAVU_SANS))
            .map_err(|e| RenderError::FontLoad(format!("Failed to load DejaVuSans: {:?}", e)))?;
        let sans_bold = doc
            .add_external_font(Cursor::new(DEJAVU_SANS_BOLD))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load DejaVuSans-Bold: {:?}", e))
            })?;
        let sans_italic = doc
            .add_external_font(Cursor::new(DEJAVU_SANS_OBLIQUE))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load DejaVuSans-Oblique: {:?}", e))
            })?;

        // Load TeX Gyre Termes family (serif)
        let serif_regular = doc
            .add_external_font(Cursor::new(TERMES_REGULAR))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-Regular: {:?}", e))
            })?;
        let serif_bold = doc
            .add_external_font(Cursor::new(TERMES_BOLD))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-Bold: {:?}", e))
            })?;
        let serif_italic = doc
            .add_external_font(Cursor::new(TERMES_ITALIC))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-Italic: {:?}", e))
            })?;
        let serif_bold_italic = doc
            .add_external_font(Cursor::new(TERMES_BOLD_ITALIC))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-BoldItalic: {:?}", e))
            })?;

        Ok(Self {
            sans: FontSet {
                regular: sans_regular.clone(),
                bold: sans_bold.clone(),
                italic: sans_italic.clone(),
                bold_italic: sans_italic.clone(), // Sans doesn't have bold-italic, use italic as fallback
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
    pub fn for_spec(&self, family_name: &str, bold: bool, italic: bool) -> &IndirectFontRef {
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
