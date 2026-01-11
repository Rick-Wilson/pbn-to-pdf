//! Text measurement utilities using rustybuzz for accurate font metrics
//!
//! This module provides functions to measure text dimensions before rendering,
//! allowing for precise layout calculations.

use rustybuzz::{Face, UnicodeBuffer};

// Embed fonts for measurement (same as in fonts.rs)
const DEJAVU_SANS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
const TERMES_REGULAR: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-regular.ttf");
const TERMES_BOLD: &[u8] = include_bytes!("../../assets/fonts/texgyretermes-bold.ttf");

/// Font family for measurement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementFont {
    SansSerif,  // DejaVu Sans
    Serif,      // TeX Gyre Termes
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
    /// Create metrics from a font face
    pub fn from_face(face: &Face) -> Self {
        Self {
            units_per_em: face.units_per_em(),
            ascender: face.ascender(),
            descender: face.descender(),
            line_gap: face.line_gap(),
            // Cap height is typically ~70% of ascender if not available
            cap_height: face.capital_height().unwrap_or((face.ascender() as f32 * 0.7) as i16),
        }
    }

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

/// Text measurement helper
pub struct TextMeasurer {
    face: Face<'static>,
    metrics: FontMetrics,
}

impl TextMeasurer {
    /// Create a text measurer from font bytes
    pub fn new(font_bytes: &'static [u8]) -> Option<Self> {
        let face = Face::from_slice(font_bytes, 0)?;
        let metrics = FontMetrics::from_face(&face);
        Some(Self { face, metrics })
    }

    /// Get the font metrics
    pub fn metrics(&self) -> &FontMetrics {
        &self.metrics
    }

    /// Measure the width of text in points
    pub fn measure_width_pt(&self, text: &str, font_size: f32) -> f32 {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);

        let output = rustybuzz::shape(&self.face, &[], buffer);

        let units_per_em = self.face.units_per_em() as f32;
        let scale = font_size / units_per_em;

        let total_advance: i32 = output.glyph_positions()
            .iter()
            .map(|pos| pos.x_advance)
            .sum();

        total_advance as f32 * scale
    }

    /// Measure the width of text in mm
    pub fn measure_width_mm(&self, text: &str, font_size: f32) -> f32 {
        self.measure_width_pt(text, font_size) * 0.3528
    }

    /// Get the cap height in mm for this font at a given size
    pub fn cap_height_mm(&self, font_size: f32) -> f32 {
        self.metrics.cap_height_mm(font_size)
    }

    /// Get the ascender height in mm
    pub fn ascender_mm(&self, font_size: f32) -> f32 {
        self.metrics.ascender_mm(font_size)
    }

    /// Get the descender depth in mm (positive value)
    pub fn descender_mm(&self, font_size: f32) -> f32 {
        self.metrics.descender_mm(font_size)
    }

    /// Get the recommended line height in mm
    pub fn line_height_mm(&self, font_size: f32) -> f32 {
        self.metrics.line_height_mm(font_size)
    }
}

/// Global text measurer using the embedded DejaVu Sans font (sans-serif)
pub fn get_measurer() -> &'static TextMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<TextMeasurer> = OnceLock::new();

    MEASURER.get_or_init(|| {
        TextMeasurer::new(DEJAVU_SANS)
            .expect("Failed to load embedded font for text measurement")
    })
}

/// Global text measurer using the embedded TeX Gyre Termes font (serif regular)
pub fn get_serif_measurer() -> &'static TextMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<TextMeasurer> = OnceLock::new();

    MEASURER.get_or_init(|| {
        TextMeasurer::new(TERMES_REGULAR)
            .expect("Failed to load embedded serif font for text measurement")
    })
}

/// Global text measurer using the embedded TeX Gyre Termes Bold font (serif bold)
pub fn get_serif_bold_measurer() -> &'static TextMeasurer {
    use std::sync::OnceLock;
    static MEASURER: OnceLock<TextMeasurer> = OnceLock::new();

    MEASURER.get_or_init(|| {
        TextMeasurer::new(TERMES_BOLD)
            .expect("Failed to load embedded serif bold font for text measurement")
    })
}

/// Get the appropriate measurer for a font family
pub fn get_measurer_for_font(font: MeasurementFont) -> &'static TextMeasurer {
    match font {
        MeasurementFont::SansSerif => get_measurer(),
        MeasurementFont::Serif => get_serif_measurer(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_measurement() {
        let measurer = get_measurer();

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
    fn test_font_metrics() {
        let measurer = get_measurer();
        let metrics = measurer.metrics();

        // Sanity checks
        assert!(metrics.units_per_em > 0);
        assert!(metrics.ascender > 0);
        assert!(metrics.descender < 0); // Descender is typically negative

        // Cap height should be less than ascender
        assert!(metrics.cap_height < metrics.ascender);
        assert!(metrics.cap_height > 0);
    }

    #[test]
    fn test_metric_conversions() {
        let measurer = get_measurer();

        // At 11pt font size
        let cap_height = measurer.cap_height_mm(11.0);
        let ascender = measurer.ascender_mm(11.0);

        println!("At 11pt:");
        println!("  Cap height: {:.2} mm", cap_height);
        println!("  Ascender: {:.2} mm", ascender);
        println!("  Line height: {:.2} mm", measurer.line_height_mm(11.0));

        // Cap height should be reasonable (roughly 2-3mm at 11pt)
        assert!(cap_height > 1.5 && cap_height < 4.0);
    }
}
