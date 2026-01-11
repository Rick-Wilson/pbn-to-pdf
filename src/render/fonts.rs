use crate::error::RenderError;
use crate::render::glyph_collector::GlyphStrings;
use printpdf::{IndirectFontRef, PdfDocumentReference};
use std::io::Cursor;
use subsetter::GlyphRemapper;

// Embed full fonts at compile time (for runtime subsetting)
const DEJAVU_SANS_FULL: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
const DEJAVU_SANS_BOLD_FULL: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");

// TeX Gyre Termes - Times New Roman clone for professional typesetting
const TERMES_REGULAR_FULL: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-regular.ttf");
const TERMES_BOLD_FULL: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-bold.ttf");
const TERMES_ITALIC_FULL: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-italic.ttf");
const TERMES_BOLD_ITALIC_FULL: &[u8] =
    include_bytes!("../../assets/fonts/texgyretermes-bolditalic.ttf");

/// Subset a font to include only the specified characters
fn subset_font(font_data: &[u8], chars: &str) -> Result<Vec<u8>, RenderError> {
    let face = rustybuzz::Face::from_slice(font_data, 0)
        .ok_or_else(|| RenderError::FontLoad("Failed to parse font for subsetting".to_string()))?;

    let mut remapper = GlyphRemapper::new();

    // Always include glyph 0 (notdef)
    remapper.remap(0);

    // Map each character to its glyph ID
    for c in chars.chars() {
        if let Some(glyph_id) = face.glyph_index(c) {
            remapper.remap(glyph_id.0);
        }
    }

    subsetter::subset(font_data, 0, &remapper)
        .map_err(|e| RenderError::FontLoad(format!("Font subsetting failed: {:?}", e)))
}

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
    /// Load fonts with runtime subsetting based on collected glyphs
    ///
    /// This subsets fonts to include only the characters actually used in the document,
    /// resulting in minimal PDF file sizes.
    pub fn new_with_glyphs(
        doc: &PdfDocumentReference,
        glyphs: &GlyphStrings,
    ) -> Result<Self, RenderError> {
        // Subset and load DejaVu Sans family
        let sans_regular_data = subset_font(DEJAVU_SANS_FULL, &glyphs.sans_regular)?;
        let sans_regular = doc
            .add_external_font(Cursor::new(sans_regular_data))
            .map_err(|e| RenderError::FontLoad(format!("Failed to load DejaVuSans: {:?}", e)))?;

        let sans_bold_data = subset_font(DEJAVU_SANS_BOLD_FULL, &glyphs.sans_bold)?;
        let sans_bold = doc
            .add_external_font(Cursor::new(sans_bold_data))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load DejaVuSans-Bold: {:?}", e))
            })?;

        // Sans italic uses same glyphs as regular (not commonly used, but include for fallback)
        let sans_italic_data = subset_font(DEJAVU_SANS_FULL, &glyphs.sans_regular)?;
        let sans_italic = doc
            .add_external_font(Cursor::new(sans_italic_data))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load DejaVuSans-Oblique: {:?}", e))
            })?;

        // Subset and load TeX Gyre Termes family (serif)
        let serif_regular_data = subset_font(TERMES_REGULAR_FULL, &glyphs.serif_regular)?;
        let serif_regular = doc
            .add_external_font(Cursor::new(serif_regular_data))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-Regular: {:?}", e))
            })?;

        let serif_bold_data = subset_font(TERMES_BOLD_FULL, &glyphs.serif_bold)?;
        let serif_bold = doc
            .add_external_font(Cursor::new(serif_bold_data))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-Bold: {:?}", e))
            })?;

        let serif_italic_data = subset_font(TERMES_ITALIC_FULL, &glyphs.serif_italic)?;
        let serif_italic = doc
            .add_external_font(Cursor::new(serif_italic_data))
            .map_err(|e| {
                RenderError::FontLoad(format!("Failed to load TeXGyreTermes-Italic: {:?}", e))
            })?;

        let serif_bold_italic_data =
            subset_font(TERMES_BOLD_ITALIC_FULL, &glyphs.serif_bold_italic)?;
        let serif_bold_italic = doc
            .add_external_font(Cursor::new(serif_bold_italic_data))
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
