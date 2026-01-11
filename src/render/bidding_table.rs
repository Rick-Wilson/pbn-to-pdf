use crate::config::Settings;
use crate::model::{Auction, BidSuit, Call, Direction};
use printpdf::{Color, IndirectFontRef, Mm, PdfLayerReference};

use super::colors::{SuitColors, BLACK};

/// Renderer for bidding tables
pub struct BiddingTableRenderer<'a> {
    layer: &'a PdfLayerReference,
    font: &'a IndirectFontRef,
    bold_font: &'a IndirectFontRef,
    italic_font: &'a IndirectFontRef,
    symbol_font: &'a IndirectFontRef,  // Font with Unicode suit symbols (DejaVu Sans)
    colors: SuitColors,
    settings: &'a Settings,
}

impl<'a> BiddingTableRenderer<'a> {
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

    /// Render the bidding table and return the height used
    pub fn render(&self, auction: &Auction, origin: (Mm, Mm)) -> f32 {
        let (ox, oy) = origin;
        let col_width = self.settings.bid_column_width;
        let row_height = self.settings.bid_row_height;

        // Render header row with spelled-out, italicized direction names
        self.layer.set_fill_color(Color::Rgb(BLACK));
        for (i, dir) in [Direction::West, Direction::North, Direction::East, Direction::South]
            .iter()
            .enumerate()
        {
            let x = ox.0 + (i as f32 * col_width);
            self.layer.use_text(
                &format!("{}", dir),  // Use Display trait for full name
                self.settings.header_font_size,
                Mm(x),
                oy,
                self.italic_font,  // Use italic font
            );
        }

        let calls = &auction.calls;

        // Check if auction is passed out (exactly 4 passes, no bids)
        let is_passed_out = calls.len() == 4
            && calls.iter().all(|a| a.call == Call::Pass);

        // Check if auction ends with 3+ passes after bidding (for "All Pass" rendering)
        let trailing_passes = calls.iter().rev().take_while(|a| a.call == Call::Pass).count();
        let show_all_pass = !is_passed_out && trailing_passes >= 3;
        let calls_to_render = if is_passed_out || show_all_pass {
            calls.len() - trailing_passes.min(calls.len())
        } else {
            calls.len()
        };

        // Determine starting column based on dealer
        let start_col = auction.dealer.table_position();
        let mut row = 1;
        let mut col = start_col;

        // Handle passed out auction
        if is_passed_out {
            let x = ox.0 + (col as f32 * col_width);
            let y = oy.0 - (row as f32 * row_height);

            self.layer.set_fill_color(Color::Rgb(BLACK));
            self.layer.use_text(
                "Passed Out",
                self.settings.body_font_size,
                Mm(x),
                Mm(y),
                self.font,
            );
            row += 1;
        } else {
            // Render regular calls (excluding trailing passes if we'll show "All Pass")
            for annotated in calls.iter().take(calls_to_render) {
                let x = ox.0 + (col as f32 * col_width);
                let y = oy.0 - (row as f32 * row_height);

                self.render_call(&annotated.call, (Mm(x), Mm(y)));

                col += 1;
                if col >= 4 {
                    col = 0;
                    row += 1;
                }
            }

            // Render "All Pass" in place of trailing passes
            if show_all_pass {
                let x = ox.0 + (col as f32 * col_width);
                let y = oy.0 - (row as f32 * row_height);

                self.layer.set_fill_color(Color::Rgb(BLACK));
                self.layer.use_text(
                    "All Pass",
                    self.settings.body_font_size,
                    Mm(x),
                    Mm(y),
                    self.font,
                );
                row += 1;
            }
        }

        // Return total height used
        ((row + 1) as f32) * row_height
    }

    /// Render a single call
    fn render_call(&self, call: &Call, pos: (Mm, Mm)) {
        let (x, y) = pos;

        match call {
            Call::Pass => {
                self.layer.set_fill_color(Color::Rgb(BLACK));
                self.layer.use_text(
                    "Pass",
                    self.settings.body_font_size,
                    x,
                    y,
                    self.font,
                );
            }
            Call::Double => {
                self.layer.set_fill_color(Color::Rgb(BLACK));
                self.layer.use_text(
                    "Dbl",
                    self.settings.body_font_size,
                    x,
                    y,
                    self.font,
                );
            }
            Call::Redouble => {
                self.layer.set_fill_color(Color::Rgb(BLACK));
                self.layer.use_text(
                    "Rdbl",
                    self.settings.body_font_size,
                    x,
                    y,
                    self.font,
                );
            }
            Call::Bid { level, suit } => {
                // Render level
                self.layer.set_fill_color(Color::Rgb(BLACK));
                self.layer.use_text(
                    &level.to_string(),
                    self.settings.body_font_size,
                    x,
                    y,
                    self.font,
                );

                // Render suit symbol with appropriate color
                let suit_x = Mm(x.0 + 3.5);
                self.render_bid_suit(*suit, (suit_x, y));
            }
        }
    }

    /// Render a bid suit symbol
    fn render_bid_suit(&self, suit: BidSuit, pos: (Mm, Mm)) {
        let (x, y) = pos;

        let (text, is_red, use_symbol_font) = match suit {
            BidSuit::Clubs => ("♣", false, true),
            BidSuit::Diamonds => ("♦", true, true),
            BidSuit::Hearts => ("♥", true, true),
            BidSuit::Spades => ("♠", false, true),
            BidSuit::NoTrump => ("NT", false, false),
        };

        if is_red {
            self.layer.set_fill_color(Color::Rgb(
                self.colors.hearts.clone(),
            ));
        } else {
            self.layer.set_fill_color(Color::Rgb(BLACK));
        }

        // Use symbol font for suit symbols, regular font for "NT"
        let font = if use_symbol_font { self.symbol_font } else { self.font };
        self.layer.use_text(
            text,
            self.settings.body_font_size,
            x,
            y,
            font,
        );
    }
}
