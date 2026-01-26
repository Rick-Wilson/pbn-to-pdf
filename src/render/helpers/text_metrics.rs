//! Text measurement utilities for PDF builtin fonts
//!
//! This module provides functions to measure text dimensions before rendering,
//! allowing for precise layout calculations using PDF builtin font metrics.

use printpdf::BuiltinFont;

/// Trait for text measurement operations
pub trait TextMeasure {
    /// Measure text width in mm at a given font size
    fn measure_text(&self, text: &str, font_size: f32) -> f32;

    /// Get cap height in mm for a given font size
    fn cap_height_mm(&self, font_size: f32) -> f32;

    /// Get descender depth in mm (positive value)
    fn descender_mm(&self, font_size: f32) -> f32;
}

/// Font metrics for layout calculations
#[derive(Debug, Clone)]
pub struct FontMetrics {
    /// Units per em (for scaling)
    pub units_per_em: i32,
    /// Ascender height in font units
    pub ascender: i16,
    /// Descender depth in font units (typically negative)
    pub descender: i16,
    /// Line gap in font units
    pub line_gap: i16,
    /// Cap height in font units (height of capital letters)
    pub cap_height: i16,
}

impl FontMetrics {
    /// Convert font units to points at a given font size
    pub fn to_points(&self, font_units: i16, font_size: f32) -> f32 {
        (font_units as f32 / self.units_per_em as f32) * font_size
    }

    /// Convert font units to mm at a given font size
    /// 1 point = 0.3528 mm
    pub fn to_mm(&self, font_units: i16, font_size: f32) -> f32 {
        self.to_points(font_units, font_size) * 0.3528
    }

    /// Get ascender height in mm at a given font size
    pub fn ascender_mm(&self, font_size: f32) -> f32 {
        self.to_mm(self.ascender, font_size)
    }

    /// Get descender depth in mm at a given font size (positive value)
    pub fn descender_mm(&self, font_size: f32) -> f32 {
        self.to_mm(-self.descender, font_size) // Make positive
    }

    /// Get cap height in mm at a given font size
    pub fn cap_height_mm(&self, font_size: f32) -> f32 {
        self.to_mm(self.cap_height, font_size)
    }

    /// Get total line height in mm at a given font size
    pub fn line_height_mm(&self, font_size: f32) -> f32 {
        self.to_mm(self.ascender - self.descender + self.line_gap, font_size)
    }
}

// =============================================================================
// Builtin PDF Font Metrics
// =============================================================================
//
// PDF's Standard 14 fonts have well-defined metrics from Adobe's AFM files.
// Character widths are in 1000 units per em.

/// Text measurer for PDF builtin fonts
///
/// Uses hardcoded Adobe AFM metrics for accurate text measurement.
pub struct BuiltinFontMeasurer {
    font: BuiltinFont,
}

impl BuiltinFontMeasurer {
    pub fn new(font: BuiltinFont) -> Self {
        Self { font }
    }

    /// Get character width in 1000 units per em
    fn char_width(&self, c: char) -> u16 {
        // ASCII printable range only - builtin fonts are Win-1252
        if !c.is_ascii() {
            return 500; // Default width for non-ASCII
        }

        let code = c as u8;
        match self.font {
            BuiltinFont::TimesRoman => TIMES_ROMAN_WIDTHS
                .get(code as usize)
                .copied()
                .unwrap_or(250),
            BuiltinFont::TimesBold => TIMES_BOLD_WIDTHS.get(code as usize).copied().unwrap_or(250),
            BuiltinFont::TimesItalic => TIMES_ITALIC_WIDTHS
                .get(code as usize)
                .copied()
                .unwrap_or(250),
            BuiltinFont::TimesBoldItalic => TIMES_BOLD_ITALIC_WIDTHS
                .get(code as usize)
                .copied()
                .unwrap_or(250),
            BuiltinFont::Helvetica => HELVETICA_WIDTHS.get(code as usize).copied().unwrap_or(278),
            BuiltinFont::HelveticaBold => HELVETICA_BOLD_WIDTHS
                .get(code as usize)
                .copied()
                .unwrap_or(278),
            BuiltinFont::HelveticaOblique => {
                HELVETICA_WIDTHS.get(code as usize).copied().unwrap_or(278)
            }
            BuiltinFont::HelveticaBoldOblique => HELVETICA_BOLD_WIDTHS
                .get(code as usize)
                .copied()
                .unwrap_or(278),
            BuiltinFont::Courier
            | BuiltinFont::CourierBold
            | BuiltinFont::CourierOblique
            | BuiltinFont::CourierBoldOblique => 600, // Monospace
            BuiltinFont::Symbol | BuiltinFont::ZapfDingbats => 500,
        }
    }

    /// Measure text width in points
    pub fn measure_width_pt(&self, text: &str, font_size: f32) -> f32 {
        let total_width: u32 = text.chars().map(|c| self.char_width(c) as u32).sum();
        (total_width as f32 / 1000.0) * font_size
    }

    /// Measure text width in mm
    pub fn measure_width_mm(&self, text: &str, font_size: f32) -> f32 {
        self.measure_width_pt(text, font_size) * 0.3528
    }

    /// Get cap height in mm for the font at given size
    pub fn cap_height_mm(&self, font_size: f32) -> f32 {
        let cap_height = match self.font {
            BuiltinFont::TimesRoman
            | BuiltinFont::TimesBold
            | BuiltinFont::TimesItalic
            | BuiltinFont::TimesBoldItalic => 662, // Times
            BuiltinFont::Helvetica
            | BuiltinFont::HelveticaBold
            | BuiltinFont::HelveticaOblique
            | BuiltinFont::HelveticaBoldOblique => 718, // Helvetica
            BuiltinFont::Courier
            | BuiltinFont::CourierBold
            | BuiltinFont::CourierOblique
            | BuiltinFont::CourierBoldOblique => 562, // Courier
            BuiltinFont::Symbol | BuiltinFont::ZapfDingbats => 700,
        };
        (cap_height as f32 / 1000.0) * font_size * 0.3528
    }

    /// Get ascender height in mm
    pub fn ascender_mm(&self, font_size: f32) -> f32 {
        let ascender = match self.font {
            BuiltinFont::TimesRoman
            | BuiltinFont::TimesBold
            | BuiltinFont::TimesItalic
            | BuiltinFont::TimesBoldItalic => 683,
            BuiltinFont::Helvetica
            | BuiltinFont::HelveticaBold
            | BuiltinFont::HelveticaOblique
            | BuiltinFont::HelveticaBoldOblique => 718,
            BuiltinFont::Courier
            | BuiltinFont::CourierBold
            | BuiltinFont::CourierOblique
            | BuiltinFont::CourierBoldOblique => 629,
            BuiltinFont::Symbol | BuiltinFont::ZapfDingbats => 800,
        };
        (ascender as f32 / 1000.0) * font_size * 0.3528
    }

    /// Get descender depth in mm (positive value)
    pub fn descender_mm(&self, font_size: f32) -> f32 {
        let descender = match self.font {
            BuiltinFont::TimesRoman
            | BuiltinFont::TimesBold
            | BuiltinFont::TimesItalic
            | BuiltinFont::TimesBoldItalic => 217,
            BuiltinFont::Helvetica
            | BuiltinFont::HelveticaBold
            | BuiltinFont::HelveticaOblique
            | BuiltinFont::HelveticaBoldOblique => 207,
            BuiltinFont::Courier
            | BuiltinFont::CourierBold
            | BuiltinFont::CourierOblique
            | BuiltinFont::CourierBoldOblique => 157,
            BuiltinFont::Symbol | BuiltinFont::ZapfDingbats => 200,
        };
        (descender as f32 / 1000.0) * font_size * 0.3528
    }

    /// Get recommended line height in mm
    pub fn line_height_mm(&self, font_size: f32) -> f32 {
        self.ascender_mm(font_size) + self.descender_mm(font_size)
    }
}

impl TextMeasure for BuiltinFontMeasurer {
    fn measure_text(&self, text: &str, font_size: f32) -> f32 {
        self.measure_width_mm(text, font_size)
    }

    fn cap_height_mm(&self, font_size: f32) -> f32 {
        self.cap_height_mm(font_size)
    }

    fn descender_mm(&self, font_size: f32) -> f32 {
        self.descender_mm(font_size)
    }
}

/// Get a builtin font measurer for Times-Roman (serif regular)
pub fn get_times_measurer() -> &'static BuiltinFontMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<BuiltinFontMeasurer> = OnceLock::new();
    MEASURER.get_or_init(|| BuiltinFontMeasurer::new(BuiltinFont::TimesRoman))
}

/// Get a builtin font measurer for Times-Bold
pub fn get_times_bold_measurer() -> &'static BuiltinFontMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<BuiltinFontMeasurer> = OnceLock::new();
    MEASURER.get_or_init(|| BuiltinFontMeasurer::new(BuiltinFont::TimesBold))
}

/// Get a builtin font measurer for Times-Italic
pub fn get_times_italic_measurer() -> &'static BuiltinFontMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<BuiltinFontMeasurer> = OnceLock::new();
    MEASURER.get_or_init(|| BuiltinFontMeasurer::new(BuiltinFont::TimesItalic))
}

/// Get a builtin font measurer for Times-BoldItalic
pub fn get_times_bold_italic_measurer() -> &'static BuiltinFontMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<BuiltinFontMeasurer> = OnceLock::new();
    MEASURER.get_or_init(|| BuiltinFontMeasurer::new(BuiltinFont::TimesBoldItalic))
}

/// Get a builtin font measurer for Helvetica (sans-serif regular)
pub fn get_helvetica_measurer() -> &'static BuiltinFontMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<BuiltinFontMeasurer> = OnceLock::new();
    MEASURER.get_or_init(|| BuiltinFontMeasurer::new(BuiltinFont::Helvetica))
}

/// Get a builtin font measurer for Helvetica-Bold
pub fn get_helvetica_bold_measurer() -> &'static BuiltinFontMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<BuiltinFontMeasurer> = OnceLock::new();
    MEASURER.get_or_init(|| BuiltinFontMeasurer::new(BuiltinFont::HelveticaBold))
}

/// Get the appropriate builtin font measurer for a BuiltinFont
pub fn get_builtin_measurer(font: BuiltinFont) -> &'static BuiltinFontMeasurer {
    match font {
        BuiltinFont::TimesRoman => get_times_measurer(),
        BuiltinFont::TimesBold => get_times_bold_measurer(),
        BuiltinFont::TimesItalic => get_times_italic_measurer(),
        BuiltinFont::TimesBoldItalic => get_times_bold_italic_measurer(),
        BuiltinFont::Helvetica | BuiltinFont::HelveticaOblique => get_helvetica_measurer(),
        BuiltinFont::HelveticaBold | BuiltinFont::HelveticaBoldOblique => {
            get_helvetica_bold_measurer()
        }
        // Courier, Symbol, ZapfDingbats - default to Helvetica metrics
        _ => get_helvetica_measurer(),
    }
}

// =============================================================================
// Adobe AFM Character Width Tables (ASCII subset, in 1000 units per em)
// =============================================================================
//
// These are the standard character widths from Adobe's AFM files for the
// Standard 14 PDF fonts. Only ASCII printable characters (32-126) are included.

/// Times-Roman character widths (indices 0-127, only 32-126 are valid)
#[rustfmt::skip]
static TIMES_ROMAN_WIDTHS: [u16; 128] = [
    // 0-31: Control characters (use 0)
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    // 32-47: space ! " # $ % & ' ( ) * + , - . /
    250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278,
    // 48-63: 0 1 2 3 4 5 6 7 8 9 : ; < = > ?
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444,
    // 64-79: @ A B C D E F G H I J K L M N O
    921, 722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722,
    // 80-95: P Q R S T U V W X Y Z [ \ ] ^ _
    556, 722, 667, 556, 611, 722, 722, 944, 722, 722, 611, 333, 278, 333, 469, 500,
    // 96-111: ` a b c d e f g h i j k l m n o
    333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500, 278, 778, 500, 500,
    // 112-127: p q r s t u v w x y z { | } ~ DEL
    500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541, 0,
];

/// Times-Bold character widths
#[rustfmt::skip]
static TIMES_BOLD_WIDTHS: [u16; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    250, 333, 555, 500, 500, 1000, 833, 278, 333, 333, 500, 570, 250, 333, 250, 278,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 333, 333, 570, 570, 570, 500,
    930, 722, 667, 722, 722, 667, 611, 778, 778, 389, 500, 778, 667, 944, 722, 778,
    611, 778, 722, 556, 667, 722, 722, 1000, 722, 722, 667, 333, 278, 333, 581, 500,
    333, 500, 556, 444, 556, 444, 333, 500, 556, 278, 333, 556, 278, 833, 556, 500,
    556, 556, 444, 389, 333, 556, 500, 722, 500, 500, 444, 394, 220, 394, 520, 0,
];

/// Times-Italic character widths
#[rustfmt::skip]
static TIMES_ITALIC_WIDTHS: [u16; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    250, 333, 420, 500, 500, 833, 778, 214, 333, 333, 500, 675, 250, 333, 250, 278,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 333, 333, 675, 675, 675, 500,
    920, 611, 611, 667, 722, 611, 611, 722, 722, 333, 444, 667, 556, 833, 667, 722,
    611, 722, 611, 500, 556, 722, 611, 833, 611, 556, 556, 389, 278, 389, 422, 500,
    333, 500, 500, 444, 500, 444, 278, 500, 500, 278, 278, 444, 278, 722, 500, 500,
    500, 500, 389, 389, 278, 500, 444, 667, 444, 444, 389, 400, 275, 400, 541, 0,
];

/// Times-BoldItalic character widths
#[rustfmt::skip]
static TIMES_BOLD_ITALIC_WIDTHS: [u16; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    250, 389, 555, 500, 500, 833, 778, 278, 333, 333, 500, 570, 250, 333, 250, 278,
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 333, 333, 570, 570, 570, 500,
    832, 667, 667, 667, 722, 667, 667, 722, 778, 389, 500, 667, 611, 889, 722, 722,
    611, 722, 667, 556, 611, 722, 667, 889, 667, 611, 611, 333, 278, 333, 570, 500,
    333, 500, 500, 444, 500, 444, 333, 500, 556, 278, 278, 500, 278, 778, 556, 500,
    500, 500, 389, 389, 278, 556, 444, 667, 500, 444, 389, 348, 220, 348, 570, 0,
];

/// Helvetica character widths
#[rustfmt::skip]
static HELVETICA_WIDTHS: [u16; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, 0,
];

/// Helvetica-Bold character widths
#[rustfmt::skip]
static HELVETICA_BOLD_WIDTHS: [u16; 128] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584, 0,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_text_measurement() {
        let measurer = get_times_measurer();

        // Test basic text width measurement
        let width = measurer.measure_width_mm("Hello", 11.0);
        assert!(width > 0.0);

        // Longer text should be wider
        let longer_width = measurer.measure_width_mm("Hello World", 11.0);
        assert!(longer_width > width);

        // Larger font should be wider
        let bigger_width = measurer.measure_width_mm("Hello", 22.0);
        assert!((bigger_width - width * 2.0).abs() < 0.1); // Should be ~2x
    }

    #[test]
    fn test_builtin_font_metrics() {
        let measurer = get_times_measurer();

        // At 11pt font size
        let cap_height = measurer.cap_height_mm(11.0);
        let ascender = measurer.ascender_mm(11.0);

        println!("Times-Roman at 11pt:");
        println!("  Cap height: {:.2} mm", cap_height);
        println!("  Ascender: {:.2} mm", ascender);
        println!("  Line height: {:.2} mm", measurer.line_height_mm(11.0));

        // Cap height should be reasonable (roughly 2-3mm at 11pt)
        assert!(cap_height > 1.5 && cap_height < 4.0);
    }

    #[test]
    fn test_helvetica_vs_times() {
        let times = get_times_measurer();
        let helvetica = get_helvetica_measurer();

        // Both should measure text
        let times_width = times.measure_width_mm("Hello", 11.0);
        let helvetica_width = helvetica.measure_width_mm("Hello", 11.0);

        // Widths should be different (different fonts)
        assert!((times_width - helvetica_width).abs() > 0.01);

        // But both should be reasonable
        assert!(times_width > 0.0 && times_width < 50.0);
        assert!(helvetica_width > 0.0 && helvetica_width < 50.0);
    }
}
