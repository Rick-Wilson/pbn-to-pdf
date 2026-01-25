use crate::config::Settings;
use crate::model::{CommentaryBlock, FormattedText, Suit, TextSpan};
use printpdf::{Color, FontId, Mm};

use crate::model::card::Rank;
use crate::render::helpers::colors::{SuitColors, BLACK};
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics::{
    get_measurer, get_sans_bold_measurer, get_serif_bold_measurer, get_serif_measurer, TextMeasurer,
};

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
    bold_italic_font: &'a FontId,
    symbol_font: &'a FontId, // Font with Unicode suit symbols (DejaVu Sans)
    colors: SuitColors,
    settings: &'a Settings,
    use_sans_measurer: bool,
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
    BoldItalic,
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

/// Check if a character is a valid card rank or placeholder in card lists.
/// Matches:
/// - Uppercase rank letters: A, K, Q, J, T
/// - Digits: 2-9
/// - Lowercase 'x' representing a low/unknown card
fn is_card_char(c: char) -> bool {
    c == 'x' || ((c.is_ascii_uppercase() || c.is_ascii_digit()) && Rank::from_char(c).is_some())
}

/// Check if a character is a valid card rank character for card lists in commentary.
/// Only matches uppercase letters (A, K, Q, J, T), digits (2-9), and lowercase 'x'.
/// Also checks that the character isn't followed by another letter (to avoid matching
/// "Joker" as J-oker).
fn is_rank_char_standalone(c: char, next_chars: &[char]) -> bool {
    // Must be a valid card character
    if !is_card_char(c) {
        return false;
    }

    // Check if followed by another letter (indicating it's part of a word like "Joker")
    // Look for the next non-whitespace character
    for &next in next_chars {
        if next.is_whitespace() {
            continue;
        }
        // If next non-whitespace is a letter that's NOT a card char, this isn't a standalone rank
        if next.is_alphabetic() && !is_card_char(next) {
            return false;
        }
        break;
    }

    true
}

/// Simple check for rank character (used for checking current word)
fn is_rank_char(c: char) -> bool {
    is_card_char(c)
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
    // Track if we're in a "card list" context (e.g., "♣K J 10")
    let mut in_card_list = false;

    // Helper to flush the current word group
    let flush_group = |tokens: &mut Vec<RenderToken>,
                       group: &mut Vec<RenderFragment>,
                       width: &mut f32,
                       in_card_list: &mut bool| {
        if !group.is_empty() {
            tokens.push(RenderToken::WordGroup(WordGroup {
                fragments: std::mem::take(group),
                width: *width,
            }));
            *width = 0.0;
        }
        *in_card_list = false;
    };

    for span in spans {
        match span {
            TextSpan::Plain(s)
            | TextSpan::Italic(s)
            | TextSpan::Bold(s)
            | TextSpan::BoldItalic(s) => {
                let style = match span {
                    TextSpan::Plain(_) => TextStyle::Plain,
                    TextSpan::Italic(_) => TextStyle::Italic,
                    TextSpan::Bold(_) => TextStyle::Bold,
                    TextSpan::BoldItalic(_) => TextStyle::BoldItalic,
                    _ => unreachable!(),
                };
                let measurer = match style {
                    TextStyle::Plain | TextStyle::Italic => regular_measurer,
                    TextStyle::Bold | TextStyle::BoldItalic => bold_measurer,
                };

                let chars: Vec<char> = s.chars().collect();
                let mut i = 0;
                let mut current_word = String::new();

                while i < chars.len() {
                    let c = chars[i];
                    if c.is_whitespace() {
                        // Flush any accumulated word fragment
                        if !current_word.is_empty() {
                            // Check if this word is a rank character to update card list state
                            let is_rank = current_word.len() == 1
                                && is_rank_char(current_word.chars().next().unwrap());
                            let w = measurer.measure_width_mm(&current_word, font_size);
                            current_group.push(RenderFragment::Text {
                                text: std::mem::take(&mut current_word),
                                style,
                            });
                            current_group_width += w;
                            // Update card list state
                            if is_rank && in_card_list {
                                // Continue in card list mode
                            } else {
                                in_card_list = false;
                            }
                        }

                        // Check if we should stay in card list context
                        let should_stay_in_card_list = in_card_list && {
                            // Look ahead to see if the next non-space char is a standalone rank
                            let rest = &chars[i + 1..];
                            if let Some(pos) = rest.iter().position(|ch| !ch.is_whitespace()) {
                                // Pass the characters after this position to check if it's standalone
                                is_rank_char_standalone(rest[pos], &rest[pos + 1..])
                            } else {
                                false
                            }
                        };

                        if should_stay_in_card_list {
                            // Keep space in the group - add it as a text fragment
                            let space_w = regular_measurer.measure_width_mm(" ", font_size);
                            current_group.push(RenderFragment::Text {
                                text: " ".to_string(),
                                style,
                            });
                            current_group_width += space_w;
                        } else {
                            // Normal case: flush the word group before the space
                            flush_group(
                                &mut tokens,
                                &mut current_group,
                                &mut current_group_width,
                                &mut in_card_list,
                            );
                            // Add space token
                            tokens.push(RenderToken::Space);
                        }
                    } else {
                        current_word.push(c);
                    }
                    i += 1;
                }

                // Don't forget remaining characters in current_word
                if !current_word.is_empty() {
                    // Check if this is a rank character
                    let is_rank = current_word.len() == 1
                        && is_rank_char(current_word.chars().next().unwrap());
                    let w = measurer.measure_width_mm(&current_word, font_size);
                    current_group.push(RenderFragment::Text {
                        text: current_word,
                        style,
                    });
                    current_group_width += w;
                    // Update card list state
                    if is_rank && in_card_list {
                        // Continue in card list mode
                    } else {
                        in_card_list = false;
                    }
                }
            }
            TextSpan::SuitSymbol(suit) => {
                let w = symbol_measurer.measure_width_mm(&suit.symbol().to_string(), font_size);
                current_group.push(RenderFragment::SuitSymbol(*suit));
                current_group_width += w;
                // Start card list mode
                in_card_list = true;
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
                // Start card list mode
                in_card_list = true;
            }
            TextSpan::LineBreak => {
                flush_group(
                    &mut tokens,
                    &mut current_group,
                    &mut current_group_width,
                    &mut in_card_list,
                );
                tokens.push(RenderToken::LineBreak);
            }
        }
    }

    // Flush any remaining group
    flush_group(
        &mut tokens,
        &mut current_group,
        &mut current_group_width,
        &mut in_card_list,
    );

    tokens
}

impl<'a> CommentaryRenderer<'a> {
    pub fn new(
        font: &'a FontId,
        bold_font: &'a FontId,
        italic_font: &'a FontId,
        bold_italic_font: &'a FontId,
        symbol_font: &'a FontId,
        settings: &'a Settings,
    ) -> Self {
        // Determine if we should use sans-serif measurement based on font settings
        let use_sans_measurer = settings
            .fonts
            .commentary
            .as_ref()
            .map(|f| f.is_sans_serif())
            .unwrap_or(false);

        Self {
            font,
            bold_font,
            italic_font,
            bold_italic_font,
            symbol_font,
            colors: SuitColors::new(settings.black_color, settings.red_color),
            settings,
            use_sans_measurer,
        }
    }

    /// Get the appropriate text measurer for regular text
    fn get_regular_measurer(&self) -> &'static TextMeasurer {
        if self.use_sans_measurer {
            get_measurer()
        } else {
            get_serif_measurer()
        }
    }

    /// Get the appropriate text measurer for bold text
    fn get_bold_measurer(&self) -> &'static TextMeasurer {
        if self.use_sans_measurer {
            get_sans_bold_measurer()
        } else {
            get_serif_bold_measurer()
        }
    }

    /// Measure the height of a commentary block without rendering
    pub fn measure_height(&self, block: &CommentaryBlock, max_width: f32) -> f32 {
        self.measure_formatted_text_height(&block.content, max_width)
    }

    /// Measure formatted text height without rendering
    fn measure_formatted_text_height(&self, text: &FormattedText, max_width: f32) -> f32 {
        let font_size = self.settings.commentary_font_size;
        let line_height = self.settings.line_height;

        let regular_measurer = self.get_regular_measurer();
        let bold_measurer = self.get_bold_measurer();
        let symbol_measurer = get_measurer();

        let base_space_width = regular_measurer.measure_width_mm(" ", font_size);

        // Tokenize the spans into word groups and spaces
        let tokens = tokenize_spans(
            &text.spans,
            font_size,
            regular_measurer,
            bold_measurer,
            symbol_measurer,
        );

        // Count lines by simulating the line-wrapping logic
        let mut token_idx = 0;
        let mut line_count = 0;

        while token_idx < tokens.len() {
            // Collect word groups for the current line
            let mut line_groups: Vec<&WordGroup> = Vec::new();
            let mut line_width: f32 = 0.0;
            let mut pending_spaces: usize = 0;

            while token_idx < tokens.len() {
                match &tokens[token_idx] {
                    RenderToken::WordGroup(group) => {
                        let space_needed = if line_groups.is_empty() {
                            0.0
                        } else {
                            base_space_width * pending_spaces.max(1) as f32
                        };
                        let new_width = line_width + space_needed + group.width;

                        if line_groups.is_empty() || new_width <= max_width {
                            line_groups.push(group);
                            line_width = new_width;
                            token_idx += 1;
                            pending_spaces = 0;
                        } else {
                            break;
                        }
                    }
                    RenderToken::Space => {
                        pending_spaces += 1;
                        token_idx += 1;
                    }
                    RenderToken::LineBreak => {
                        token_idx += 1;
                        break;
                    }
                }
            }

            // Count this line (even if empty due to consecutive line breaks)
            line_count += 1;
        }

        // Return total height: line_count * line_height
        // Adjust for the extra line_height similar to render method
        (line_count as f32) * line_height
    }

    /// Render a commentary block and return the height used
    pub fn render(
        &self,
        layer: &mut LayerBuilder,
        block: &CommentaryBlock,
        origin: (Mm, Mm),
        max_width: f32,
    ) -> f32 {
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

        // Use appropriate measurers based on font type (sans vs serif)
        // Symbol font (DejaVu Sans) always uses sans measurer
        let regular_measurer = self.get_regular_measurer();
        let bold_measurer = self.get_bold_measurer();
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
            // Track (word_group, preceding_space_count) for each word
            let mut line_groups: Vec<(&WordGroup, usize)> = Vec::new();
            let mut line_width: f32 = 0.0;
            let mut is_paragraph_end = false;
            let mut pending_spaces: usize = 0;

            while token_idx < tokens.len() {
                match &tokens[token_idx] {
                    RenderToken::WordGroup(group) => {
                        // Calculate width if we add this word
                        let space_needed = if line_groups.is_empty() {
                            0.0
                        } else {
                            base_space_width * pending_spaces.max(1) as f32
                        };
                        let new_width = line_width + space_needed + group.width;

                        if line_groups.is_empty() || new_width <= max_width {
                            // Word fits on this line
                            line_groups.push((group, pending_spaces));
                            line_width = new_width;
                            token_idx += 1;
                            pending_spaces = 0;
                        } else {
                            // Word doesn't fit, break line here (don't consume this token)
                            break;
                        }
                    }
                    RenderToken::Space => {
                        // Count consecutive spaces
                        pending_spaces += 1;
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

            // Calculate total space units needed (sum of all space counts between words)
            let total_space_units: usize = line_groups
                .iter()
                .skip(1)
                .map(|(_, count)| (*count).max(1))
                .sum();

            // Calculate space width for justification
            let space_width = if justify && !is_paragraph_end && total_space_units > 0 {
                // Calculate total content width (word groups only, no spaces)
                let total_word_width: f32 = line_groups.iter().map(|(g, _)| g.width).sum();
                // Available space for distribution (divided by total space units)
                let available_space = max_width - total_word_width;
                // Each space unit gets this width
                available_space / total_space_units as f32
            } else {
                // Use base space width for paragraph-ending lines or when not justifying
                base_space_width
            };

            // Render the line
            let mut x = current_line_start;

            for (i, (group, space_count)) in line_groups.iter().enumerate() {
                // Add space before word (except first word)
                // Use the recorded space count, but at least 1 if not the first word
                if i > 0 {
                    let num_spaces = (*space_count).max(1);
                    x += space_width * num_spaces as f32;
                }

                // Render all fragments in the group
                for fragment in &group.fragments {
                    match fragment {
                        RenderFragment::Text { text: txt, style } => {
                            let font = match style {
                                TextStyle::Plain => self.font,
                                TextStyle::Bold => self.bold_font,
                                TextStyle::Italic => self.italic_font,
                                TextStyle::BoldItalic => self.bold_italic_font,
                            };
                            let measurer = match style {
                                TextStyle::Plain | TextStyle::Italic => &regular_measurer,
                                TextStyle::Bold | TextStyle::BoldItalic => &bold_measurer,
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
