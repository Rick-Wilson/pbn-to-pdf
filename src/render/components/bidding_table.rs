use crate::config::Settings;
use crate::model::{AnnotatedCall, Auction, BidSuit, Call, Direction, DirectionExt, PlayerNames};
use printpdf::{BuiltinFont, Color, FontId, Mm};

use crate::render::helpers::colors::{SuitColors, BLACK};
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics;

/// Size ratio for superscript text relative to body font
const SUPERSCRIPT_RATIO: f32 = 0.65;
/// Vertical offset for superscript as fraction of font size
const SUPERSCRIPT_RISE: f32 = 0.4;

/// Renderer for bidding tables
pub struct BiddingTableRenderer<'a> {
    font: BuiltinFont,
    #[allow(dead_code)]
    bold_font: BuiltinFont,
    italic_font: BuiltinFont,
    symbol_font: &'a FontId, // Font with Unicode suit symbols (DejaVu Sans)
    colors: SuitColors,
    settings: &'a Settings,
    use_sans_measurer: bool,
}

impl<'a> BiddingTableRenderer<'a> {
    pub fn new(
        font: BuiltinFont,
        bold_font: BuiltinFont,
        italic_font: BuiltinFont,
        symbol_font: &'a FontId,
        settings: &'a Settings,
    ) -> Self {
        // Determine if we should use sans-serif measurement based on font settings
        let use_sans_measurer = settings
            .fonts
            .hand_record
            .as_ref()
            .map(|f| f.is_sans_serif())
            .unwrap_or(false);

        Self {
            font,
            bold_font,
            italic_font,
            symbol_font,
            colors: SuitColors::new(settings.black_color, settings.red_color),
            settings,
            use_sans_measurer,
        }
    }

    /// Get the appropriate text measurer based on font type
    fn get_measurer(&self) -> &'static text_metrics::TextMeasurer {
        if self.use_sans_measurer {
            text_metrics::get_measurer()
        } else {
            text_metrics::get_serif_measurer()
        }
    }

    /// Calculate the height of the bidding table without rendering
    pub fn measure_height(&self, auction: &Auction) -> f32 {
        self.measure_height_with_players(auction, None)
    }

    /// Calculate the height of the bidding table with optional player names
    pub fn measure_height_with_players(
        &self,
        auction: &Auction,
        players: Option<&PlayerNames>,
    ) -> f32 {
        Self::measure_height_static(auction, players, self.settings)
    }

    /// Calculate the height of the bidding table with all options (static version)
    /// This can be called without creating a renderer instance, useful for layout measurement
    pub fn measure_height_static(
        auction: &Auction,
        players: Option<&PlayerNames>,
        settings: &Settings,
    ) -> f32 {
        let row_height = settings.bid_row_height;
        let two_col_auctions = settings.two_col_auctions;

        let calls = &auction.calls;

        // Check if we should use two-column mode for this auction
        let uncontested_pair = if two_col_auctions {
            auction.uncontested_pair()
        } else {
            None
        };

        // Check if auction is passed out (exactly 4 passes, no bids)
        let is_passed_out = calls.len() == 4 && calls.iter().all(|a| a.call == Call::Pass);

        // Check if auction ends with 3+ passes after bidding (for "All Pass" rendering)
        let trailing_passes = calls
            .iter()
            .rev()
            .take_while(|a| a.call == Call::Pass)
            .count();
        let show_all_pass = !is_passed_out && trailing_passes >= 3;
        let calls_to_render = if is_passed_out || show_all_pass {
            calls.len() - trailing_passes.min(calls.len())
        } else {
            calls.len()
        };

        // Start counting rows: spacing + header + optional player names row
        // Row 0 is spacing, Row 1 is header, Row 2 is player names (if present)
        let has_player_names = players.is_some_and(|p| p.has_any());
        let mut row = if has_player_names { 3 } else { 2 };

        // Handle passed out auction
        if is_passed_out {
            row += 1;
        } else if let Some((d1, d2)) = uncontested_pair {
            // Two-column mode: count only calls from the bidding pair
            let mut col = 0;
            let mut current_player = auction.dealer;

            for _ in calls.iter().take(calls_to_render) {
                if current_player == d1 || current_player == d2 {
                    col += 1;
                    if col >= 2 {
                        col = 0;
                        row += 1;
                    }
                }
                current_player = current_player.next();
            }

            // "All Pass" goes on next row after content
            if show_all_pass {
                if col > 0 {
                    row += 1; // Move past partial row
                }
                row += 1; // Row for "All Pass"
            }
        } else {
            // Standard 4-column mode: count rows for regular calls
            let start_col = auction.dealer.table_position();
            let mut col = start_col;

            for _ in calls.iter().take(calls_to_render) {
                col += 1;
                if col >= 4 {
                    col = 0;
                    row += 1;
                }
            }

            // "All Pass" goes on next row after content
            if show_all_pass {
                if col > 0 {
                    row += 1; // Move past partial row
                }
                row += 1; // Row for "All Pass"
            }
        }

        // Account for notes
        if !auction.notes.is_empty() {
            let note_font_size = settings.body_font_size * 0.85;
            let note_line_height = note_font_size * 1.3 * 0.352778; // Convert pt to mm
            let notes_height = (auction.notes.len() as f32) * note_line_height;
            row += (notes_height / row_height).ceil() as usize;
        }

        // Return total height used
        // Row counts the number of row slots used
        row as f32 * row_height
    }

    /// Render the bidding table and return the height used
    pub fn render(&self, layer: &mut LayerBuilder, auction: &Auction, origin: (Mm, Mm)) -> f32 {
        self.render_with_options(layer, auction, origin, None, false)
    }

    /// Render the bidding table with optional player names and return the height used
    pub fn render_with_players(
        &self,
        layer: &mut LayerBuilder,
        auction: &Auction,
        origin: (Mm, Mm),
        players: Option<&PlayerNames>,
    ) -> f32 {
        self.render_with_options(layer, auction, origin, players, self.settings.two_col_auctions)
    }

    /// Render the bidding table with all options
    pub fn render_with_options(
        &self,
        layer: &mut LayerBuilder,
        auction: &Auction,
        origin: (Mm, Mm),
        players: Option<&PlayerNames>,
        two_col_auctions: bool,
    ) -> f32 {
        let (ox, oy) = origin;
        let col_width = self.settings.bid_column_width;
        let row_height = self.settings.bid_row_height;

        // Check if we should use two-column mode for this auction
        let uncontested_pair = if two_col_auctions {
            auction.uncontested_pair()
        } else {
            None
        };

        // Render header row with spelled-out, italicized direction names
        layer.set_fill_color(Color::Rgb(BLACK));

        // Choose which directions to show in header
        let directions: &[Direction] = if let Some((d1, _d2)) = uncontested_pair
        {
            // Two-column mode: show only the bidding pair
            static WE: [Direction; 2] = [Direction::West, Direction::East];
            static NS: [Direction; 2] = [Direction::North, Direction::South];
            if d1 == Direction::West || d1 == Direction::East {
                &WE[..]
            } else {
                &NS[..]
            }
        } else {
            // Standard 4-column mode
            static ALL: [Direction; 4] = [
                Direction::West,
                Direction::North,
                Direction::East,
                Direction::South,
            ];
            &ALL[..]
        };

        // Add spacing before header row to separate from content above
        let header_y = Mm(oy.0 - row_height);

        for (i, dir) in directions.iter().enumerate() {
            let x = ox.0 + (i as f32 * col_width);
            layer.use_text_builtin(
                format!("{}", dir), // Use Display trait for full name
                self.settings.header_font_size,
                Mm(x),
                header_y,
                self.font, // Use regular font for direction names
            );
        }

        // Render player names below direction headers if provided
        let has_player_names = players.is_some_and(|p| p.has_any());
        if has_player_names {
            let players = players.unwrap();
            let name_y = Mm(header_y.0 - row_height);

            for (i, dir) in directions.iter().enumerate() {
                if let Some(name) = players.get(*dir) {
                    if !name.is_empty() {
                        let x = ox.0 + (i as f32 * col_width);
                        layer.use_text_builtin(
                            name,
                            self.settings.header_font_size,
                            Mm(x),
                            name_y,
                            self.italic_font,
                        );
                    }
                }
            }
        }

        let calls = &auction.calls;

        // Check if auction is passed out (exactly 4 passes, no bids)
        let is_passed_out = calls.len() == 4 && calls.iter().all(|a| a.call == Call::Pass);

        // Check if auction ends with 3+ passes after bidding (for "All Pass" rendering)
        let trailing_passes = calls
            .iter()
            .rev()
            .take_while(|a| a.call == Call::Pass)
            .count();
        let show_all_pass = !is_passed_out && trailing_passes >= 3;
        let calls_to_render = if is_passed_out || show_all_pass {
            calls.len() - trailing_passes.min(calls.len())
        } else {
            calls.len()
        };

        // Start row accounting for spacing + header + optional player names row
        // Row 0 is spacing, Row 1 is header, Row 2 is player names (if present)
        let mut row = if has_player_names { 3 } else { 2 };

        // Handle passed out auction
        if is_passed_out {
            // In two-column mode, show "Passed Out" at column 0
            let col = if uncontested_pair.is_some() { 0 } else { auction.dealer.table_position() };
            let x = ox.0 + (col as f32 * col_width);
            let y = oy.0 - (row as f32 * row_height);

            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text_builtin(
                "Passed Out",
                self.settings.body_font_size,
                Mm(x),
                Mm(y),
                self.font,
            );
            row += 1;
        } else if let Some(pair) = uncontested_pair {
            // Two-column mode: only show the bidding pair's calls
            let (d1, d2) = pair;
            let mut col = 0;
            let mut current_player = auction.dealer;

            for annotated in calls.iter().take(calls_to_render) {
                // Only render calls from the bidding pair
                if current_player == d1 || current_player == d2 {
                    // Determine which column (0 or 1) based on which player in the pair
                    let display_col = if current_player == d1 { 0 } else { 1 };
                    let x = ox.0 + (display_col as f32 * col_width);
                    let y = oy.0 - (row as f32 * row_height);

                    self.render_annotated_call(layer, annotated, (Mm(x), Mm(y)));

                    col += 1;
                    if col >= 2 {
                        col = 0;
                        row += 1;
                    }
                }
                current_player = current_player.next();
            }

            // Render "All Pass" on next row if needed
            if show_all_pass {
                // Move to next row if current row has content
                if col > 0 {
                    row += 1;
                }
                let x = ox.0;
                let y = oy.0 - (row as f32 * row_height);

                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin(
                    "All Pass",
                    self.settings.body_font_size,
                    Mm(x),
                    Mm(y),
                    self.font,
                );
                row += 1;
            }
        } else {
            // Standard 4-column mode
            let start_col = auction.dealer.table_position();
            let mut col = start_col;

            // Render regular calls (excluding trailing passes if we'll show "All Pass")
            for annotated in calls.iter().take(calls_to_render) {
                let x = ox.0 + (col as f32 * col_width);
                let y = oy.0 - (row as f32 * row_height);

                self.render_annotated_call(layer, annotated, (Mm(x), Mm(y)));

                col += 1;
                if col >= 4 {
                    col = 0;
                    row += 1;
                }
            }

            // Render "All Pass" on next row if needed
            if show_all_pass {
                // Move to next row if current row has content
                if col > 0 {
                    row += 1;
                }
                let x = ox.0;
                let y = oy.0 - (row as f32 * row_height);

                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin(
                    "All Pass",
                    self.settings.body_font_size,
                    Mm(x),
                    Mm(y),
                    self.font,
                );
                row += 1;
            }
        }

        // Render notes if present
        if !auction.notes.is_empty() {
            let notes_height =
                self.render_notes(layer, auction, (ox, Mm(oy.0 - (row as f32 * row_height))));
            row += (notes_height / row_height).ceil() as usize;
        }

        // Return total height used
        // Row counts the number of row positions used (0 through row-1)
        row as f32 * row_height
    }

    /// Render an annotated call (call with optional superscript annotation)
    fn render_annotated_call(
        &self,
        layer: &mut LayerBuilder,
        annotated: &AnnotatedCall,
        pos: (Mm, Mm),
    ) {
        let call_width = self.render_call(layer, &annotated.call, pos);

        // If there's an annotation, render it as superscript
        if let Some(ref annotation) = annotated.annotation {
            let sup_x = Mm(pos.0 .0 + call_width);
            let sup_y = Mm(pos.1 .0 + (self.settings.body_font_size * SUPERSCRIPT_RISE * 0.352778)); // Convert pt to mm
            let sup_size = self.settings.body_font_size * SUPERSCRIPT_RATIO;

            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text_builtin(annotation, sup_size, sup_x, sup_y, self.font);
        }
    }

    /// Render a single call and return the width used
    fn render_call(&self, layer: &mut LayerBuilder, call: &Call, pos: (Mm, Mm)) -> f32 {
        let (x, y) = pos;
        let measurer = self.get_measurer();

        match call {
            Call::Pass => {
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin("Pass", self.settings.body_font_size, x, y, self.font);
                measurer.measure_width_mm("Pass", self.settings.body_font_size)
            }
            Call::Double => {
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin("Dbl", self.settings.body_font_size, x, y, self.font);
                measurer.measure_width_mm("Dbl", self.settings.body_font_size)
            }
            Call::Redouble => {
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin("Rdbl", self.settings.body_font_size, x, y, self.font);
                measurer.measure_width_mm("Rdbl", self.settings.body_font_size)
            }
            Call::Bid { level, strain: suit } => {
                // Render level
                layer.set_fill_color(Color::Rgb(BLACK));
                let level_str = level.to_string();
                layer.use_text_builtin(&level_str, self.settings.body_font_size, x, y, self.font);

                // Render suit symbol immediately after level (no gap)
                let level_width =
                    measurer.measure_width_mm(&level_str, self.settings.body_font_size);
                let suit_x = Mm(x.0 + level_width);
                let suit_width = self.render_bid_suit(layer, *suit, (suit_x, y));
                level_width + suit_width
            }
            Call::Continue => {
                // "+" in PBN becomes "?" in display (student fills in next bid)
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin("?", self.settings.body_font_size, x, y, self.font);
                measurer.measure_width_mm("?", self.settings.body_font_size)
            }
            Call::Blank => {
                // Underscore sequences in PBN become a horizontal line for students to write answers
                // Draw a line approximately 8mm wide at the text baseline
                let line_width = 8.0; // mm
                let line_thickness = 0.3; // mm
                let baseline_offset = self.settings.body_font_size * 0.08 * 0.352778; // Slightly below baseline

                layer.set_outline_color(Color::Rgb(BLACK));
                layer.set_outline_thickness(line_thickness);
                layer.add_line(x, Mm(y.0 - baseline_offset), Mm(x.0 + line_width), Mm(y.0 - baseline_offset));
                line_width
            }
        }
    }

    /// Render notes below the bidding table
    fn render_notes(&self, layer: &mut LayerBuilder, auction: &Auction, origin: (Mm, Mm)) -> f32 {
        let (ox, oy) = origin;
        let note_font_size = self.settings.body_font_size * 0.85; // Slightly smaller for notes
        let line_height = note_font_size * 1.3 * 0.352778; // Convert pt to mm

        // Get sorted note numbers
        let mut note_nums: Vec<&u8> = auction.notes.keys().collect();
        note_nums.sort();

        let mut current_y = oy.0 - line_height; // Start below the origin with some spacing

        for num in note_nums {
            if let Some(text) = auction.notes.get(num) {
                let note_text = format!("{}. {}", num, text);
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text_builtin(&note_text, note_font_size, ox, Mm(current_y), self.font);
                current_y -= line_height;
            }
        }

        // Return total height used
        (oy.0 - current_y).max(0.0)
    }

    /// Render a bid suit symbol and return width used
    fn render_bid_suit(&self, layer: &mut LayerBuilder, suit: BidSuit, pos: (Mm, Mm)) -> f32 {
        let (x, y) = pos;
        let measurer = self.get_measurer();

        let (text, is_red, use_symbol_font) = match suit {
            BidSuit::Clubs => ("♣", false, true),
            BidSuit::Diamonds => ("♦", true, true),
            BidSuit::Hearts => ("♥", true, true),
            BidSuit::Spades => ("♠", false, true),
            BidSuit::NoTrump => ("NT", false, false),
        };

        if is_red {
            layer.set_fill_color(Color::Rgb(self.colors.hearts.clone()));
        } else {
            layer.set_fill_color(Color::Rgb(BLACK));
        }

        // Use symbol font for suit symbols, regular font for "NT"
        if use_symbol_font {
            layer.use_text(text, self.settings.body_font_size, x, y, self.symbol_font);
        } else {
            layer.use_text_builtin(text, self.settings.body_font_size, x, y, self.font);
        }

        // Measure width (use sans for symbols, serif for NT)
        if use_symbol_font {
            let sans_measurer = text_metrics::get_measurer();
            sans_measurer.measure_width_mm(text, self.settings.body_font_size)
        } else {
            measurer.measure_width_mm(text, self.settings.body_font_size)
        }
    }
}
