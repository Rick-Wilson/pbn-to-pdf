//! Winners table component for declarer plan layout
//!
//! Renders a fillable form with:
//! - Winners section (by suit + total)
//! - Techniques section (Promotion, Length, Finesse, blank, End Play)

use printpdf::{BuiltinFont, Color, FontId, Mm, PaintMode, Rgb};

use crate::render::helpers::colors::{SuitColors, BLACK, WHITE};
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics;

/// Light blue for "Winners" header background
const HEADER_BLUE: Rgb = Rgb {
    r: 0.678, // #ADC8E6 approximately
    g: 0.784,
    b: 0.902,
    icc_profile: None,
};

/// Light green for "Techniques" header background
const HEADER_GREEN: Rgb = Rgb {
    r: 0.745, // #BED8BE approximately
    g: 0.847,
    b: 0.745,
    icc_profile: None,
};

/// Renderer for the winners/techniques table
pub struct WinnersTableRenderer<'a> {
    font: BuiltinFont,
    bold_font: BuiltinFont,
    symbol_font: &'a FontId,
    colors: SuitColors,
    /// Font size for header text (e.g., "Winners", "Techniques")
    header_font_size: f32,
    /// Font size for column labels
    label_font_size: f32,
    /// Column width in mm
    col_width: f32,
    /// Row height in mm
    row_height: f32,
    /// Header row height in mm
    header_height: f32,
    /// Border line thickness in points
    line_thickness: f32,
}

impl<'a> WinnersTableRenderer<'a> {
    /// Create a new winners table renderer with default settings
    pub fn new(
        font: BuiltinFont,
        bold_font: BuiltinFont,
        symbol_font: &'a FontId,
        colors: SuitColors,
    ) -> Self {
        Self {
            font,
            bold_font,
            symbol_font,
            colors,
            header_font_size: 14.0,
            label_font_size: 12.0,
            col_width: 16.0,
            row_height: 8.0,
            header_height: 6.0, // Reduced: just enough for font + small padding
            line_thickness: 0.5,
        }
    }

    /// Set font sizes
    pub fn font_sizes(mut self, header: f32, label: f32) -> Self {
        self.header_font_size = header;
        self.label_font_size = label;
        self
    }

    /// Set column width
    pub fn col_width(mut self, width: f32) -> Self {
        self.col_width = width;
        self
    }

    /// Set row height
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// Get the total dimensions of the table
    pub fn dimensions(&self) -> (f32, f32) {
        let width = self.col_width * 5.0; // 5 columns for suit row (techniques row uses 4 wider columns)
        let height = self.header_height * 2.0 + self.row_height * 4.0; // 2 headers + 4 data rows
        (width, height)
    }

    /// Render the table at the given origin (top-left corner)
    /// Returns the height used
    pub fn render(&self, layer: &mut LayerBuilder, origin: (Mm, Mm)) -> f32 {
        let (ox, oy) = (origin.0 .0, origin.1 .0);
        let (width, _) = self.dimensions();

        let mut current_y = oy;

        // === Winners Section ===

        // Header row: "Count Sure Winners" with blue background
        self.render_header_row(
            layer,
            ox,
            current_y,
            width,
            "Count Sure Winners",
            HEADER_BLUE,
        );
        current_y -= self.header_height;

        // Suit symbols row: ♠ ♥ ♦ ♣ Total (5 columns)
        self.render_suit_row(layer, ox, current_y);
        current_y -= self.row_height;

        // Empty input row (5 columns)
        self.render_empty_suit_row(layer, ox, current_y);
        current_y -= self.row_height;

        // === Techniques Section ===

        // Header row: "Decide How to Develop Winners" with green background
        self.render_header_row(
            layer,
            ox,
            current_y,
            width,
            "Decide How to Develop Winners",
            HEADER_GREEN,
        );
        current_y -= self.header_height;

        // Technique labels row (4 columns)
        self.render_techniques_row(layer, ox, current_y);
        current_y -= self.row_height;

        // Empty input row (4 columns)
        self.render_empty_techniques_row(layer, ox, current_y);
        current_y -= self.row_height;

        // Draw outer border
        self.draw_outer_border(layer, ox, oy, current_y);

        oy - current_y
    }

    /// Render a header row with colored background
    fn render_header_row(
        &self,
        layer: &mut LayerBuilder,
        x: f32,
        y: f32,
        width: f32,
        text: &str,
        bg_color: Rgb,
    ) {
        // Draw background
        layer.set_fill_color(Color::Rgb(bg_color));
        layer.add_rect(
            Mm(x),
            Mm(y - self.header_height),
            Mm(x + width),
            Mm(y),
            PaintMode::Fill,
        );

        // Draw border lines
        layer.set_outline_color(Color::Rgb(BLACK));
        layer.set_outline_thickness(self.line_thickness);
        layer.add_line(Mm(x), Mm(y), Mm(x + width), Mm(y)); // Top
        layer.add_line(
            Mm(x),
            Mm(y - self.header_height),
            Mm(x + width),
            Mm(y - self.header_height),
        ); // Bottom
        layer.add_line(Mm(x), Mm(y), Mm(x), Mm(y - self.header_height)); // Left
        layer.add_line(
            Mm(x + width),
            Mm(y),
            Mm(x + width),
            Mm(y - self.header_height),
        ); // Right

        // Draw centered text
        let measurer = text_metrics::get_serif_measurer();
        let text_width = measurer.measure_width_mm(text, self.header_font_size);
        let text_x = x + (width - text_width) / 2.0;
        // Vertical center: baseline position accounts for font metrics (descender ~20% of font size)
        let text_y = y - self.header_height
            + (self.header_height - self.header_font_size * 0.35) / 2.0
            + 1.0;

        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text_builtin(
            text,
            self.header_font_size,
            Mm(text_x),
            Mm(text_y),
            self.bold_font,
        );
    }

    /// Render the suit symbols row (♠ ♥ ♦ ♣ Total) - 5 columns
    fn render_suit_row(&self, layer: &mut LayerBuilder, x: f32, y: f32) {
        let suits = [
            ("♠", false), // Spades - black
            ("♥", true),  // Hearts - red
            ("♦", true),  // Diamonds - red
            ("♣", false), // Clubs - black
        ];

        let width = self.col_width * 5.0;

        // Fill with white background first
        layer.set_fill_color(Color::Rgb(WHITE));
        layer.add_rect(
            Mm(x),
            Mm(y - self.row_height),
            Mm(x + width),
            Mm(y),
            PaintMode::Fill,
        );

        // Draw row border
        self.draw_row_border_width(layer, x, y, width, 5);

        let measurer = text_metrics::get_measurer(); // Sans for symbols

        for (i, (symbol, is_red)) in suits.iter().enumerate() {
            let cell_x = x + (i as f32 * self.col_width);

            // Draw column separator
            if i > 0 {
                layer.set_outline_color(Color::Rgb(BLACK));
                layer.set_outline_thickness(self.line_thickness);
                layer.add_line(Mm(cell_x), Mm(y), Mm(cell_x), Mm(y - self.row_height));
            }

            // Center the symbol
            let symbol_width = measurer.measure_width_mm(symbol, self.label_font_size);
            let symbol_x = cell_x + (self.col_width - symbol_width) / 2.0;
            let symbol_y =
                y - self.row_height + (self.row_height - self.label_font_size * 0.35) / 2.0;

            if *is_red {
                layer.set_fill_color(Color::Rgb(self.colors.hearts.clone()));
            } else {
                layer.set_fill_color(Color::Rgb(BLACK));
            }
            layer.use_text(
                *symbol,
                self.label_font_size,
                Mm(symbol_x),
                Mm(symbol_y),
                self.symbol_font,
            );
        }

        // "Total" label in last column
        let total_x = x + 4.0 * self.col_width;
        layer.set_outline_color(Color::Rgb(BLACK));
        layer.set_outline_thickness(self.line_thickness);
        layer.add_line(Mm(total_x), Mm(y), Mm(total_x), Mm(y - self.row_height));

        let measurer = text_metrics::get_serif_measurer();
        let text_width = measurer.measure_width_mm("Total", self.label_font_size);
        let text_x = total_x + (self.col_width - text_width) / 2.0;
        let text_y = y - self.row_height + (self.row_height - self.label_font_size * 0.35) / 2.0;

        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text_builtin(
            "Total",
            self.label_font_size,
            Mm(text_x),
            Mm(text_y),
            self.font,
        );
    }

    /// Render the techniques labels row - 4 columns (wider than suit columns)
    fn render_techniques_row(&self, layer: &mut LayerBuilder, x: f32, y: f32) {
        let labels = ["Promotion", "Length", "Finesse", "End Play"];
        let total_width = self.col_width * 5.0; // Same total width as suit row
        let tech_col_width = total_width / 4.0; // 4 wider columns

        // Fill with white background first
        layer.set_fill_color(Color::Rgb(WHITE));
        layer.add_rect(
            Mm(x),
            Mm(y - self.row_height),
            Mm(x + total_width),
            Mm(y),
            PaintMode::Fill,
        );

        // Draw row border
        self.draw_row_border_width(layer, x, y, total_width, 4);

        let measurer = text_metrics::get_serif_measurer();

        for (i, label) in labels.iter().enumerate() {
            let cell_x = x + (i as f32 * tech_col_width);

            // Draw column separator
            if i > 0 {
                layer.set_outline_color(Color::Rgb(BLACK));
                layer.set_outline_thickness(self.line_thickness);
                layer.add_line(Mm(cell_x), Mm(y), Mm(cell_x), Mm(y - self.row_height));
            }

            // Center the label
            let text_width = measurer.measure_width_mm(label, self.label_font_size);
            let text_x = cell_x + (tech_col_width - text_width) / 2.0;
            let text_y =
                y - self.row_height + (self.row_height - self.label_font_size * 0.35) / 2.0;

            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text_builtin(
                *label,
                self.label_font_size,
                Mm(text_x),
                Mm(text_y),
                self.font,
            );
        }
    }

    /// Render an empty row for suit input (5 columns)
    fn render_empty_suit_row(&self, layer: &mut LayerBuilder, x: f32, y: f32) {
        let width = self.col_width * 5.0;

        // Fill with white background first
        layer.set_fill_color(Color::Rgb(WHITE));
        layer.add_rect(
            Mm(x),
            Mm(y - self.row_height),
            Mm(x + width),
            Mm(y),
            PaintMode::Fill,
        );

        self.draw_row_border_width(layer, x, y, width, 5);

        // Draw column separators
        for i in 1..5 {
            let cell_x = x + (i as f32 * self.col_width);
            layer.set_outline_color(Color::Rgb(BLACK));
            layer.set_outline_thickness(self.line_thickness);
            layer.add_line(Mm(cell_x), Mm(y), Mm(cell_x), Mm(y - self.row_height));
        }
    }

    /// Render an empty row for techniques input (4 columns)
    fn render_empty_techniques_row(&self, layer: &mut LayerBuilder, x: f32, y: f32) {
        let total_width = self.col_width * 5.0;
        let tech_col_width = total_width / 4.0;

        // Fill with white background first
        layer.set_fill_color(Color::Rgb(WHITE));
        layer.add_rect(
            Mm(x),
            Mm(y - self.row_height),
            Mm(x + total_width),
            Mm(y),
            PaintMode::Fill,
        );

        self.draw_row_border_width(layer, x, y, total_width, 4);

        // Draw column separators
        for i in 1..4 {
            let cell_x = x + (i as f32 * tech_col_width);
            layer.set_outline_color(Color::Rgb(BLACK));
            layer.set_outline_thickness(self.line_thickness);
            layer.add_line(Mm(cell_x), Mm(y), Mm(cell_x), Mm(y - self.row_height));
        }
    }

    /// Draw the border for a row with specified width and column count
    fn draw_row_border_width(
        &self,
        layer: &mut LayerBuilder,
        x: f32,
        y: f32,
        width: f32,
        _cols: usize,
    ) {
        layer.set_outline_color(Color::Rgb(BLACK));
        layer.set_outline_thickness(self.line_thickness);

        // Top line
        layer.add_line(Mm(x), Mm(y), Mm(x + width), Mm(y));
        // Bottom line
        layer.add_line(
            Mm(x),
            Mm(y - self.row_height),
            Mm(x + width),
            Mm(y - self.row_height),
        );
        // Left line
        layer.add_line(Mm(x), Mm(y), Mm(x), Mm(y - self.row_height));
        // Right line
        layer.add_line(Mm(x + width), Mm(y), Mm(x + width), Mm(y - self.row_height));
    }

    /// Draw the outer border of the entire table
    fn draw_outer_border(&self, layer: &mut LayerBuilder, x: f32, top_y: f32, bottom_y: f32) {
        let width = self.col_width * 5.0;

        layer.set_outline_color(Color::Rgb(BLACK));
        layer.set_outline_thickness(self.line_thickness * 2.0); // Thicker outer border

        layer.add_line(Mm(x), Mm(top_y), Mm(x + width), Mm(top_y)); // Top
        layer.add_line(Mm(x), Mm(bottom_y), Mm(x + width), Mm(bottom_y)); // Bottom
        layer.add_line(Mm(x), Mm(top_y), Mm(x), Mm(bottom_y)); // Left
        layer.add_line(Mm(x + width), Mm(top_y), Mm(x + width), Mm(bottom_y)); // Right
    }
}
