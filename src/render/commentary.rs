use crate::config::Settings;
use crate::model::{CommentaryBlock, FormattedText, Suit, TextSpan};
use printpdf::{Color, FontId, Mm};

use super::colors::{SuitColors, BLACK};
use super::layer::LayerBuilder;
use super::text_metrics::{
    get_measurer, get_serif_bold_measurer, get_serif_measurer, TextMeasurer,
};
use crate::model::card::Rank;

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
    font: &'a FontId,
    bold_font: &'a FontId,
    italic_font: &'a FontId,
    symbol_font: &'a FontId, // Font with Unicode suit symbols (DejaVu Sans)
    colors: SuitColors,
    settings: &'a Settings,
}

/// A fragment is an atomic piece of text with a specific style
#[derive(Debug, Clone)]
enum RenderFragment {
    Text { text: String, style: TextStyle },
    SuitSymbol(Suit),
    CardRef { suit: Suit, rank: Rank },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextStyle {
    Plain,
    Bold,
    Italic,
}

/// A word group is a sequence of fragments that should be kept together (no whitespace between them)
#[derive(Debug, Clone)]
struct WordGroup {
    fragments: Vec<RenderFragment>,
    width: f32,
}

/// A render token is either a word group or a space
#[derive(Debug, Clone)]
enum RenderToken {
    WordGroup(WordGroup),
    Space,
    LineBreak,
}

/// Parse spans into render tokens, grouping fragments that should stay together
fn tokenize_spans(
    spans: &[TextSpan],
    font_size: f32,
    regular_measurer: &TextMeasurer,
    bold_measurer: &TextMeasurer,
    symbol_measurer: &TextMeasurer,
) -> Vec<RenderToken> {
    let mut tokens: Vec<RenderToken> = Vec::new();
    let mut current_group: Vec<RenderFragment> = Vec::new();
    let mut current_group_width: f32 = 0.0;

    // Helper to flush the current word group
    let flush_group =
        |tokens: &mut Vec<RenderToken>, group: &mut Vec<RenderFragment>, width: &mut f32| {
            if !group.is_empty() {
                tokens.push(RenderToken::WordGroup(WordGroup {
                    fragments: std::mem::take(group),
                    width: *width,
                }));
                *width = 0.0;
            }
        };

    for span in spans {
        match span {
            TextSpan::Plain(s) | TextSpan::Italic(s) | TextSpan::Bold(s) => {
                let style = match span {
                    TextSpan::Plain(_) => TextStyle::Plain,
                    TextSpan::Italic(_) => TextStyle::Italic,
                    TextSpan::Bold(_) => TextStyle::Bold,
                    _ => unreachable!(),
                };
                let measurer = match style {
                    TextStyle::Plain | TextStyle::Italic => regular_measurer,
                    TextStyle::Bold => bold_measurer,
                };

                let chars = s.chars();
                let mut current_word = String::new();

                for c in chars {
                    if c.is_whitespace() {
                        // Flush any accumulated word fragment
                        if !current_word.is_empty() {
                            let w = measurer.measure_width_mm(&current_word, font_size);
                            current_group.push(RenderFragment::Text {
                                text: std::mem::take(&mut current_word),
                                style,
                            });
                            current_group_width += w;
                        }
                        // Flush the word group before the space
                        flush_group(&mut tokens, &mut current_group, &mut current_group_width);
                        // Add space token
                        tokens.push(RenderToken::Space);
                    } else {
                        current_word.push(c);
                    }
                }

                // Don't forget remaining characters in current_word
                if !current_word.is_empty() {
                    let w = measurer.measure_width_mm(&current_word, font_size);
                    current_group.push(RenderFragment::Text {
                        text: current_word,
                        style,
                    });
                    current_group_width += w;
                }
            }
            TextSpan::SuitSymbol(suit) => {
                let w = symbol_measurer.measure_width_mm(&suit.symbol().to_string(), font_size);
                current_group.push(RenderFragment::SuitSymbol(*suit));
                current_group_width += w;
            }
            TextSpan::CardRef { suit, rank } => {
                let symbol_w =
                    symbol_measurer.measure_width_mm(&suit.symbol().to_string(), font_size);
                let rank_w =
                    regular_measurer.measure_width_mm(&rank.to_char().to_string(), font_size);
                current_group.push(RenderFragment::CardRef {
                    suit: *suit,
                    rank: *rank,
                });
                current_group_width += symbol_w + rank_w;
            }
            TextSpan::LineBreak => {
                flush_group(&mut tokens, &mut current_group, &mut current_group_width);
                tokens.push(RenderToken::LineBreak);
            }
        }
    }

    // Flush any remaining group
    flush_group(&mut tokens, &mut current_group, &mut current_group_width);

    tokens
}

impl<'a> CommentaryRenderer<'a> {
    pub fn new(
        font: &'a FontId,
        bold_font: &'a FontId,
        italic_font: &'a FontId,
        symbol_font: &'a FontId,
        settings: &'a Settings,
    ) -> Self {
        Self {
            font,
            bold_font,
            italic_font,
            symbol_font,
            colors: SuitColors::new(settings.black_color, settings.red_color),
            settings,
        }
    }

    /// Render a commentary block and return the height used
    pub fn render(&self, layer: &mut LayerBuilder, block: &CommentaryBlock, origin: (Mm, Mm), max_width: f32) -> f32 {
        self.render_formatted_text(layer, &block.content, origin, max_width, None)
            .height
    }

    /// Render a commentary block with floating layout
    /// Returns the result including final Y position for continuation
    pub fn render_float(
        &self,
        layer: &mut LayerBuilder,
        block: &CommentaryBlock,
        origin: (Mm, Mm),
        float_layout: &FloatLayout,
    ) -> FloatRenderResult {
        self.render_formatted_text(
            layer,
            &block.content,
            origin,
            float_layout.float_width,
            Some(float_layout),
        )
    }

    /// Render formatted text and return height used
    fn render_formatted_text(
        &self,
        layer: &mut LayerBuilder,
        text: &FormattedText,
        origin: (Mm, Mm),
        initial_max_width: f32,
        float_layout: Option<&FloatLayout>,
    ) -> FloatRenderResult {
        let (ox, oy) = origin;
        let font_size = self.settings.commentary_font_size;
        let line_height = self.settings.line_height;
        let justify = self.settings.justify;

        // Use serif measurers for text (TeX Gyre Termes) and sans measurer for symbols (DejaVu Sans)
        let regular_measurer = get_serif_measurer();
        let bold_measurer = get_serif_bold_measurer();
        let symbol_measurer = get_measurer();

        let base_space_width = regular_measurer.measure_width_mm(" ", font_size);

        // Track current layout state
        let mut current_line_start = ox.0;
        let mut max_width = initial_max_width;
        let mut in_float = float_layout.is_some();
        let mut y = oy.0;

        // Tokenize the spans into word groups and spaces
        let tokens = tokenize_spans(
            &text.spans,
            font_size,
            regular_measurer,
            bold_measurer,
            symbol_measurer,
        );

        // Process tokens and render lines on-the-fly
        // This allows us to handle width changes dynamically
        let mut token_idx = 0;

        while token_idx < tokens.len() {
            // Check if we've crossed the float boundary before starting a new line
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

            // Collect word groups for the current line using current max_width
            let mut line_groups: Vec<&WordGroup> = Vec::new();
            let mut line_width: f32 = 0.0;
            let mut is_paragraph_end = false;

            while token_idx < tokens.len() {
                match &tokens[token_idx] {
                    RenderToken::WordGroup(group) => {
                        // Calculate width if we add this word
                        let space_needed = if line_groups.is_empty() {
                            0.0
                        } else {
                            base_space_width
                        };
                        let new_width = line_width + space_needed + group.width;

                        if line_groups.is_empty() || new_width <= max_width {
                            // Word fits on this line
                            line_groups.push(group);
                            line_width = new_width;
                            token_idx += 1;
                        } else {
                            // Word doesn't fit, break line here (don't consume this token)
                            break;
                        }
                    }
                    RenderToken::Space => {
                        // Skip spaces (they're handled between word groups)
                        token_idx += 1;
                    }
                    RenderToken::LineBreak => {
                        // Explicit line break - end the current line as a paragraph end
                        is_paragraph_end = true;
                        token_idx += 1;
                        break;
                    }
                }
            }

            // If we collected no words (e.g., multiple consecutive line breaks), just move to next line
            if line_groups.is_empty() {
                y -= line_height;
                continue;
            }

            // Check if this is the last line (paragraph end)
            if token_idx >= tokens.len() {
                is_paragraph_end = true;
            }

            // Calculate space width for justification
            let space_count = if line_groups.len() > 1 {
                line_groups.len() - 1
            } else {
                0
            };
            let space_width = if justify && !is_paragraph_end && space_count > 0 {
                // Calculate total content width (word groups only, no spaces)
                let total_word_width: f32 = line_groups.iter().map(|g| g.width).sum();
                // Available space for distribution
                let available_space = max_width - total_word_width;
                // Distribute among spaces
                available_space / space_count as f32
            } else {
                // Use base space width for paragraph-ending lines or when not justifying
                base_space_width
            };

            // Render the line
            let mut x = current_line_start;

            for (i, group) in line_groups.iter().enumerate() {
                // Add space before word (except first word)
                if i > 0 {
                    x += space_width;
                }

                // Render all fragments in the group
                for fragment in &group.fragments {
                    match fragment {
                        RenderFragment::Text { text: txt, style } => {
                            let font = match style {
                                TextStyle::Plain => self.font,
                                TextStyle::Bold => self.bold_font,
                                TextStyle::Italic => self.italic_font,
                            };
                            let measurer = match style {
                                TextStyle::Plain | TextStyle::Italic => &regular_measurer,
                                TextStyle::Bold => &bold_measurer,
                            };
                            let width = measurer.measure_width_mm(txt, font_size);

                            layer.set_fill_color(Color::Rgb(BLACK));
                            layer.use_text(txt, font_size, Mm(x), Mm(y), font);
                            x += width;
                        }
                        RenderFragment::SuitSymbol(suit) => {
                            let symbol = suit.symbol().to_string();
                            let width = symbol_measurer.measure_width_mm(&symbol, font_size);

                            let color = self.colors.for_suit(suit);
                            layer.set_fill_color(Color::Rgb(color));
                            layer.use_text(&symbol, font_size, Mm(x), Mm(y), self.symbol_font);
                            x += width;
                        }
                        RenderFragment::CardRef { suit, rank } => {
                            let symbol = suit.symbol().to_string();
                            let symbol_width = symbol_measurer.measure_width_mm(&symbol, font_size);
                            let rank_str = rank.to_char().to_string();
                            let rank_width =
                                regular_measurer.measure_width_mm(&rank_str, font_size);

                            // Render suit symbol with color
                            let color = self.colors.for_suit(suit);
                            layer.set_fill_color(Color::Rgb(color));
                            layer.use_text(&symbol, font_size, Mm(x), Mm(y), self.symbol_font);
                            x += symbol_width;

                            // Render rank in black
                            layer.set_fill_color(Color::Rgb(BLACK));
                            layer.use_text(&rank_str, font_size, Mm(x), Mm(y), self.font);
                            x += rank_width;
                        }
                    }
                }
            }

            // Move to next line
            y -= line_height;
        }

        // Return total height used and final Y position
        // Adjust for the extra line_height we subtracted after the last line
        let final_y = y + line_height;
        FloatRenderResult {
            height: oy.0 - final_y,
            final_y,
        }
    }
}
