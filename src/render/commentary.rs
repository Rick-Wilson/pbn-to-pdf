use crate::config::Settings;
use crate::model::{CommentaryBlock, FormattedText, TextSpan};
use printpdf::{Color, IndirectFontRef, Mm, PdfLayerReference};

use super::colors::{SuitColors, BLACK};
use super::text_metrics::{get_measurer, get_serif_bold_measurer, get_serif_measurer};

/// Parameters for floating layout
#[derive(Debug, Clone)]
pub struct FloatLayout {
    /// Y position below which we switch to full width (page coordinates, so smaller = lower)
    pub float_until_y: f32,
    /// Left margin while floating (right side of page)
    pub float_left: f32,
    /// Max width while floating
    pub float_width: f32,
    /// Left margin after clearing float (full width)
    pub full_left: f32,
    /// Max width after clearing float
    pub full_width: f32,
}

/// Result of rendering with float layout
#[derive(Debug)]
pub struct FloatRenderResult {
    /// Total height used
    pub height: f32,
    /// Final Y position
    pub final_y: f32,
}

/// Renderer for commentary text
pub struct CommentaryRenderer<'a> {
    layer: &'a PdfLayerReference,
    font: &'a IndirectFontRef,
    bold_font: &'a IndirectFontRef,
    italic_font: &'a IndirectFontRef,
    symbol_font: &'a IndirectFontRef,  // Font with Unicode suit symbols (DejaVu Sans)
    colors: SuitColors,
    settings: &'a Settings,
}

impl<'a> CommentaryRenderer<'a> {
    pub fn new(
        layer: &'a PdfLayerReference,
        font: &'a IndirectFontRef,
        bold_font: &'a IndirectFontRef,
        italic_font: &'a IndirectFontRef,
        symbol_font: &'a IndirectFontRef,
        settings: &'a Settings,
    ) -> Self {
        Self {
            layer,
            font,
            bold_font,
            italic_font,
            symbol_font,
            colors: SuitColors::new(settings.black_color, settings.red_color),
            settings,
        }
    }

    /// Render a commentary block and return the height used
    pub fn render(&self, block: &CommentaryBlock, origin: (Mm, Mm), max_width: f32) -> f32 {
        self.render_formatted_text(&block.content, origin, max_width, None).height
    }

    /// Render a commentary block with floating layout
    /// Returns the result including final Y position for continuation
    pub fn render_float(
        &self,
        block: &CommentaryBlock,
        origin: (Mm, Mm),
        float_layout: &FloatLayout,
    ) -> FloatRenderResult {
        self.render_formatted_text(&block.content, origin, float_layout.float_width, Some(float_layout))
    }

    /// Render formatted text and return height used
    fn render_formatted_text(
        &self,
        text: &FormattedText,
        origin: (Mm, Mm),
        initial_max_width: f32,
        float_layout: Option<&FloatLayout>,
    ) -> FloatRenderResult {
        let (ox, oy) = origin;
        let font_size = self.settings.commentary_font_size;
        let line_height = self.settings.line_height;
        // Use serif measurers for text (TeX Gyre Termes) and sans measurer for symbols (DejaVu Sans)
        let regular_measurer = get_serif_measurer();
        let bold_measurer = get_serif_bold_measurer();
        let symbol_measurer = get_measurer();

        // Measure actual space width for accurate spacing (using regular serif font)
        let regular_space_width = regular_measurer.measure_width_mm(" ", font_size);
        let bold_space_width = bold_measurer.measure_width_mm(" ", font_size);

        let mut x = ox.0;
        let mut y = oy.0;

        // Track current layout state
        let mut current_line_start = ox.0;
        let mut max_width = initial_max_width;
        let mut in_float = float_layout.is_some();

        // Helper macro to handle line wrap and float transition
        macro_rules! wrap_line {
            () => {{
                y -= line_height;
                // Check if we've crossed the float boundary
                if in_float {
                    if let Some(fl) = float_layout {
                        if y < fl.float_until_y {
                            // Switch to full width layout
                            in_float = false;
                            current_line_start = fl.full_left;
                            max_width = fl.full_width;
                        }
                    }
                }
                x = current_line_start;
            }};
        }

        for span in &text.spans {
            match span {
                TextSpan::Plain(s) => {
                    let space_width = regular_space_width;

                    // Check if text starts with whitespace (need leading space)
                    if s.starts_with(char::is_whitespace) && x > current_line_start {
                        x += space_width;
                    }

                    // Word wrap
                    let words: Vec<&str> = s.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        let word_width = regular_measurer.measure_width_mm(word, font_size);

                        // Check if we need to wrap (but don't wrap punctuation-only fragments)
                        let is_punctuation = word.chars().all(|c| c.is_ascii_punctuation());
                        if x + word_width > current_line_start + max_width && x > current_line_start && !is_punctuation {
                            wrap_line!();
                        }

                        self.layer.set_fill_color(Color::Rgb(BLACK));
                        self.layer.use_text(
                            *word,
                            font_size,
                            Mm(x),
                            Mm(y),
                            self.font,
                        );

                        x += word_width;

                        // Add space after each word (between words within span)
                        if i < words.len() - 1 {
                            x += space_width;
                        }
                    }

                    // Add trailing space if original text ended with space
                    if s.ends_with(char::is_whitespace) && !words.is_empty() {
                        x += space_width;
                    }
                }
                TextSpan::Italic(s) => {
                    let space_width = regular_space_width;

                    // Check if text starts with whitespace (need leading space)
                    if s.starts_with(char::is_whitespace) && x > current_line_start {
                        x += space_width;
                    }

                    // Word wrap
                    let words: Vec<&str> = s.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        let word_width = regular_measurer.measure_width_mm(word, font_size);

                        // Check if we need to wrap (but don't wrap punctuation-only fragments)
                        let is_punctuation = word.chars().all(|c| c.is_ascii_punctuation());
                        if x + word_width > current_line_start + max_width && x > current_line_start && !is_punctuation {
                            wrap_line!();
                        }

                        self.layer.set_fill_color(Color::Rgb(BLACK));
                        self.layer.use_text(
                            *word,
                            font_size,
                            Mm(x),
                            Mm(y),
                            self.italic_font,  // Use italic font
                        );

                        x += word_width;

                        // Add space after each word (between words within span)
                        if i < words.len() - 1 {
                            x += space_width;
                        }
                    }

                    // Add trailing space if original text ended with space
                    if s.ends_with(char::is_whitespace) && !words.is_empty() {
                        x += space_width;
                    }
                }
                TextSpan::Bold(s) => {
                    let space_width = bold_space_width;

                    // Check if text starts with whitespace (need leading space)
                    if s.starts_with(char::is_whitespace) && x > current_line_start {
                        x += space_width;
                    }

                    // Word wrap
                    let words: Vec<&str> = s.split_whitespace().collect();
                    for (i, word) in words.iter().enumerate() {
                        let word_width = bold_measurer.measure_width_mm(word, font_size);

                        // Check if we need to wrap (but don't wrap punctuation-only fragments)
                        let is_punctuation = word.chars().all(|c| c.is_ascii_punctuation());
                        if x + word_width > current_line_start + max_width && x > current_line_start && !is_punctuation {
                            wrap_line!();
                        }

                        self.layer.set_fill_color(Color::Rgb(BLACK));
                        self.layer.use_text(
                            *word,
                            font_size,
                            Mm(x),
                            Mm(y),
                            self.bold_font,
                        );

                        x += word_width;

                        // Add space after each word (between words within span)
                        if i < words.len() - 1 {
                            x += space_width;
                        }
                    }

                    // Add trailing space if original text ended with space
                    if s.ends_with(char::is_whitespace) && !words.is_empty() {
                        x += space_width;
                    }
                }
                TextSpan::SuitSymbol(suit) => {
                    let symbol = suit.symbol().to_string();
                    let symbol_width = symbol_measurer.measure_width_mm(&symbol, font_size);

                    let color = self.colors.for_suit(suit);
                    self.layer.set_fill_color(Color::Rgb(color));
                    self.layer.use_text(
                        &symbol,
                        font_size,
                        Mm(x),
                        Mm(y),
                        self.symbol_font,  // Use symbol font for suit symbols
                    );
                    x += symbol_width;
                }
                TextSpan::CardRef { suit, rank } => {
                    let symbol = suit.symbol().to_string();
                    let symbol_width = symbol_measurer.measure_width_mm(&symbol, font_size);
                    let rank_str = rank.to_char().to_string();
                    let rank_width = regular_measurer.measure_width_mm(&rank_str, font_size);

                    // Render suit symbol with color using symbol font
                    let color = self.colors.for_suit(suit);
                    self.layer.set_fill_color(Color::Rgb(color));
                    self.layer.use_text(
                        &symbol,
                        font_size,
                        Mm(x),
                        Mm(y),
                        self.symbol_font,  // Use symbol font for suit symbols
                    );
                    x += symbol_width;

                    // Render rank in black using regular font
                    self.layer.set_fill_color(Color::Rgb(BLACK));
                    self.layer.use_text(
                        &rank_str,
                        font_size,
                        Mm(x),
                        Mm(y),
                        self.font,
                    );
                    x += rank_width;
                }
                TextSpan::LineBreak => {
                    wrap_line!();
                }
            }
        }

        // Return total height used and final Y position
        FloatRenderResult {
            height: oy.0 - y + line_height,
            final_y: y,
        }
    }
}
