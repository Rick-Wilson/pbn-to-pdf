//! Bidding Sheets Layout Renderer
//!
//! Generates PDF documents for face-to-face bidding practice.
//! Each board set produces:
//! 1. North practice page (shows only North's hand)
//! 2. Answers page (shows both hands + auction)
//! 3. South practice page (shows only South's hand)
//! 4. Answers page (repeated for duplex printing)

use printpdf::{Color, FontId, Mm, PaintMode, PdfDocument, PdfPage, PdfSaveOptions, Rgb};

use crate::config::Settings;
use crate::error::RenderError;
use crate::model::{AnnotatedCall, Auction, BidSuit, Board, Call, Direction, Hand, Suit, Vulnerability};

use crate::render::helpers::colors::{SuitColors, BLACK, WHITE};
use crate::render::helpers::fonts::FontManager;
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics::get_measurer;

/// Light gray color for debug boxes
const DEBUG_BOX_COLOR: Rgb = Rgb {
    r: 0.7,
    g: 0.7,
    b: 0.7,
    icc_profile: None,
};
const DEBUG_BOXES: bool = true;

/// Font sizes for bidding sheets
const PRACTICE_FONT_SIZE: f32 = 14.0;
const ANSWERS_FONT_SIZE: f32 = 11.0;
const HEADER_FONT_SIZE: f32 = 16.0;

/// Line height multiplier
const LINE_HEIGHT_MULTIPLIER: f32 = 1.4;

/// Superscript ratio relative to body font
const SUPERSCRIPT_RATIO: f32 = 0.65;
/// Superscript vertical rise as fraction of font size
const SUPERSCRIPT_RISE: f32 = 0.4;

/// Banner height in mm
const BANNER_HEIGHT: f32 = 10.0;
/// Gap after banner before content
const AFTER_BANNER_GAP: f32 = 10.0;

/// Column widths (in mm)
const CONTEXT_COLUMN_WIDTH: f32 = 45.0;
const HAND_COLUMN_WIDTH: f32 = 35.0;

/// Row spacing
const ROW_GAP: f32 = 5.0;

/// Bidding sheets renderer
pub struct BiddingSheetsRenderer {
    settings: Settings,
}

impl BiddingSheetsRenderer {
    /// Draw a debug outline box
    fn draw_debug_box(&self, layer: &mut LayerBuilder, x: f32, y: f32, w: f32, h: f32) {
        if !DEBUG_BOXES {
            return;
        }
        // y is top of box, draw from bottom-left to top-right
        layer.set_outline_color(Color::Rgb(DEBUG_BOX_COLOR));
        layer.set_outline_thickness(0.25);
        layer.add_rect(Mm(x), Mm(y - h), Mm(x + w), Mm(y), PaintMode::Stroke);
    }
}

impl BiddingSheetsRenderer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Generate a PDF with bidding practice sheets
    pub fn render(&self, boards: &[Board]) -> Result<Vec<u8>, RenderError> {
        let title = boards
            .first()
            .and_then(|b| b.event.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Bidding Practice");

        let mut doc = PdfDocument::new(title);

        // Load fonts - printpdf 0.8 handles subsetting automatically
        let fonts = FontManager::new(&mut doc)?;

        let mut pages = Vec::new();

        // Group boards into sets that fit on a page
        let board_sets = self.group_boards(boards);

        for board_set in board_sets {
            // North practice page
            let mut layer = LayerBuilder::new();
            self.render_practice_page(&mut layer, board_set, Direction::North, &fonts);
            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));

            // Answers page (after North)
            let mut layer = LayerBuilder::new();
            self.render_answers_page(&mut layer, board_set, &fonts);
            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));

            // South practice page
            let mut layer = LayerBuilder::new();
            self.render_practice_page(&mut layer, board_set, Direction::South, &fonts);
            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));

            // Answers page (after South, for duplex printing)
            let mut layer = LayerBuilder::new();
            self.render_answers_page(&mut layer, board_set, &fonts);
            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));
        }

        doc.with_pages(pages);

        let mut warnings = Vec::new();
        let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

        Ok(bytes)
    }

    /// Calculate available content height on a page (after banner and gaps)
    fn available_content_height(&self) -> f32 {
        let margin_top = self.settings.margin_top;
        let margin_bottom = self.settings.margin_bottom;
        let page_height = self.settings.page_height;

        page_height - margin_top - margin_bottom - BANNER_HEIGHT - AFTER_BANNER_GAP
    }

    /// Calculate height needed for a board on a practice page
    /// This must match exactly how render_practice_page advances current_y
    fn practice_board_height(&self, _board: &Board) -> f32 {
        let line_height = PRACTICE_FONT_SIZE * LINE_HEIGHT_MULTIPLIER * 0.4;

        // Practice page uses: row_height = max of column heights, then advances by row_height + ROW_GAP
        // All columns are 4 lines
        4.0 * line_height
        // Note: ROW_GAP is between boards, not after the last one
    }

    /// Calculate height needed for a board on an answers page
    /// This must match exactly how render_answers_page advances current_y
    fn answers_board_height(&self, board: &Board) -> f32 {
        let line_height = ANSWERS_FONT_SIZE * LINE_HEIGHT_MULTIPLIER * 0.4;

        // Answers page uses: row_height = max of column heights, then advances by row_height + ROW_GAP
        // Context: 6 lines, Hands: 5 lines (label + 4 suits), Auction: variable
        let context_height = 6.0 * line_height;
        let hand_height = 5.0 * line_height;
        let auction_height = self.calculate_auction_height(board, ANSWERS_FONT_SIZE);

        context_height.max(hand_height).max(auction_height)
        // Note: ROW_GAP is between boards, not after the last one
    }

    /// Group boards into sets that fit on a page
    /// Uses the answers page height (which is larger) as the constraint
    fn group_boards<'a>(&self, boards: &'a [Board]) -> Vec<&'a [Board]> {
        let available_height = self.available_content_height();
        let mut sets = Vec::new();
        let mut start = 0;

        while start < boards.len() {
            let mut current_height = 0.0;
            let mut end = start;

            // Add boards until we run out of space
            while end < boards.len() {
                // Calculate heights for this board on all page types
                let practice_height = self.practice_board_height(&boards[end]);
                let answers_height = self.answers_board_height(&boards[end]);

                // Use the maximum height (answers page typically needs more space)
                let board_height = practice_height.max(answers_height);

                // Add ROW_GAP between boards (not before the first one)
                let height_needed = if end == start {
                    board_height
                } else {
                    board_height + ROW_GAP
                };

                if current_height + height_needed > available_height && end > start {
                    // This board won't fit, but we have at least one board
                    break;
                }

                current_height += height_needed;
                end += 1;
            }

            // Ensure we make progress (at least one board per page)
            if end == start {
                end = start + 1;
            }

            sets.push(&boards[start..end]);
            start = end;
        }

        sets
    }

    /// Render a practice page (shows only one player's hand)
    fn render_practice_page(
        &self,
        layer: &mut LayerBuilder,
        boards: &[Board],
        player: Direction,
        fonts: &FontManager,
    ) {
        let margin_left = self.settings.margin_left;
        let margin_top = self.settings.margin_top;
        let page_top = self.settings.page_height - margin_top;
        let content_width = self.settings.content_width();
        let measurer = get_measurer();

        let text_font = &fonts.serif.regular;
        let bold_font = &fonts.serif.bold;
        let sans_bold_font = &fonts.sans.bold;
        let symbol_font = &fonts.sans.regular;
        let colors = SuitColors::new(self.settings.black_color, self.settings.red_color);

        // Header banner - full width colored rectangle with white sans-serif text
        let header_text = format!("{} hands (Practice Page)", player);
        let banner_padding = 3.0; // Padding inside banner

        // Color for player identification
        let header_color = match player {
            Direction::North => Rgb::new(0.12, 0.56, 1.0, None), // DodgerBlue
            Direction::South => Rgb::new(1.0, 0.65, 0.0, None),  // Orange
            _ => Rgb::new(0.5, 0.5, 0.5, None),
        };

        // Draw filled rectangle banner
        layer.set_fill_color(Color::Rgb(header_color));
        layer.add_rect(
            Mm(margin_left),
            Mm(page_top - BANNER_HEIGHT),
            Mm(margin_left + content_width),
            Mm(page_top),
            PaintMode::Fill,
        );

        // Draw white text inside banner (sans-serif bold)
        let text_y = page_top - banner_padding - measurer.cap_height_mm(HEADER_FONT_SIZE);
        layer.set_fill_color(Color::Rgb(WHITE));
        layer.use_text(
            &header_text,
            HEADER_FONT_SIZE,
            Mm(margin_left + banner_padding),
            Mm(text_y),
            sans_bold_font,
        );

        // Start content below banner
        let mut current_y = page_top - BANNER_HEIGHT - AFTER_BANNER_GAP;

        let line_height = PRACTICE_FONT_SIZE * LINE_HEIGHT_MULTIPLIER * 0.4;

        for board in boards {
            let row_start_y = current_y;

            // Debug boxes for each column
            // Text is rendered at baseline (current_y), so box top should be at baseline + cap_height
            // Box height: cap_height (first line above baseline) + 3 * line_height (to 4th baseline)
            let cap_height = measurer.cap_height_mm(PRACTICE_FONT_SIZE);
            let box_top = current_y + cap_height;
            let row_height = cap_height + 3.0 * line_height;
            self.draw_debug_box(
                layer,
                margin_left,
                box_top,
                CONTEXT_COLUMN_WIDTH,
                row_height,
            );
            self.draw_debug_box(
                layer,
                margin_left + CONTEXT_COLUMN_WIDTH,
                box_top,
                HAND_COLUMN_WIDTH,
                row_height,
            );
            self.draw_debug_box(
                layer,
                margin_left + CONTEXT_COLUMN_WIDTH + HAND_COLUMN_WIDTH,
                box_top,
                80.0,
                row_height,
            );

            // Column 1: Board context
            let col1_x = margin_left;
            self.render_board_context(
                layer,
                board,
                Some(player),
                col1_x,
                current_y,
                PRACTICE_FONT_SIZE,
                text_font,
                bold_font,
            );

            // Column 2: Player's hand (vertical layout)
            let col2_x = margin_left + CONTEXT_COLUMN_WIDTH;
            let hand = board.deal.hand(player);
            self.render_hand_vertical(
                layer,
                hand,
                col2_x,
                current_y,
                PRACTICE_FONT_SIZE,
                text_font,
                symbol_font,
                &colors,
            );

            // Column 3: Opposition bidding + who bids first
            let col3_x = margin_left + CONTEXT_COLUMN_WIDTH + HAND_COLUMN_WIDTH;
            self.render_auction_setup(
                layer,
                board,
                player,
                col3_x,
                current_y,
                PRACTICE_FONT_SIZE,
                text_font,
                symbol_font,
                &colors,
            );

            // Calculate row height (max of all columns)
            let context_height = 4.0 * line_height; // 4 lines: Board, Dealer, Vul, HCP
            let hand_height = 4.0 * line_height; // 4 suits
            let setup_height = 4.0 * line_height; // Opposition + who bids first

            let row_height = context_height.max(hand_height).max(setup_height);
            current_y = row_start_y - row_height - ROW_GAP;
        }
    }

    /// Render an answers page (shows both hands + auction)
    fn render_answers_page(&self, layer: &mut LayerBuilder, boards: &[Board], fonts: &FontManager) {
        let margin_left = self.settings.margin_left;
        let margin_top = self.settings.margin_top;
        let page_top = self.settings.page_height - margin_top;
        let content_width = self.settings.content_width();
        let measurer = get_measurer();

        let text_font = &fonts.serif.regular;
        let bold_font = &fonts.serif.bold;
        let sans_bold_font = &fonts.sans.bold;
        let symbol_font = &fonts.sans.regular;
        let colors = SuitColors::new(self.settings.black_color, self.settings.red_color);

        // Header banner - full width dark gray rectangle with white sans-serif text
        let header_text = "Both hands (Answers Page)";
        let banner_padding = 3.0; // Padding inside banner
        let header_color = Rgb::new(0.3, 0.3, 0.3, None); // Dark gray for answers

        // Draw filled rectangle banner
        layer.set_fill_color(Color::Rgb(header_color));
        layer.add_rect(
            Mm(margin_left),
            Mm(page_top - BANNER_HEIGHT),
            Mm(margin_left + content_width),
            Mm(page_top),
            PaintMode::Fill,
        );

        // Draw white text inside banner (sans-serif bold)
        let text_y = page_top - banner_padding - measurer.cap_height_mm(HEADER_FONT_SIZE);
        layer.set_fill_color(Color::Rgb(WHITE));
        layer.use_text(
            header_text,
            HEADER_FONT_SIZE,
            Mm(margin_left + banner_padding),
            Mm(text_y),
            sans_bold_font,
        );

        // Start content below banner
        let mut current_y = page_top - BANNER_HEIGHT - AFTER_BANNER_GAP;

        let line_height = ANSWERS_FONT_SIZE * LINE_HEIGHT_MULTIPLIER * 0.4;

        for board in boards {
            let row_start_y = current_y;

            // Column 1: Board context (with both HCPs and contract)
            let col1_x = margin_left;
            self.render_board_context_full(
                layer,
                board,
                col1_x,
                current_y,
                ANSWERS_FONT_SIZE,
                text_font,
                bold_font,
                symbol_font,
                &colors,
            );

            // Column 2: North's hand
            let col2_x = margin_left + CONTEXT_COLUMN_WIDTH;
            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text(
                "North:",
                ANSWERS_FONT_SIZE,
                Mm(col2_x),
                Mm(current_y),
                bold_font,
            );
            self.render_hand_vertical(
                layer,
                &board.deal.north,
                col2_x,
                current_y - line_height,
                ANSWERS_FONT_SIZE,
                text_font,
                symbol_font,
                &colors,
            );

            // Column 3: South's hand
            let col3_x = margin_left + CONTEXT_COLUMN_WIDTH + HAND_COLUMN_WIDTH;
            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text(
                "South:",
                ANSWERS_FONT_SIZE,
                Mm(col3_x),
                Mm(current_y),
                bold_font,
            );
            self.render_hand_vertical(
                layer,
                &board.deal.south,
                col3_x,
                current_y - line_height,
                ANSWERS_FONT_SIZE,
                text_font,
                symbol_font,
                &colors,
            );

            // Column 4: Auction table - capture actual height and last line height
            let col4_x = margin_left + CONTEXT_COLUMN_WIDTH + 2.0 * HAND_COLUMN_WIDTH;
            let (auction_height, auction_last_line_height) = if let Some(ref auction) = board.auction {
                self.render_auction_table(
                    layer,
                    auction,
                    col4_x,
                    current_y,
                    ANSWERS_FONT_SIZE,
                    text_font,
                    bold_font,
                    symbol_font,
                    &colors,
                )
            } else {
                (line_height, line_height)
            };

            // Calculate row height using actual auction height
            let context_height = 6.0 * line_height; // More lines on answers page
            let hand_height = 5.0 * line_height; // Label + 4 suits
            let row_height = context_height.max(hand_height).max(auction_height);

            // Determine which column is tallest and use its last line height
            let last_line_height = if auction_height >= context_height && auction_height >= hand_height {
                auction_last_line_height
            } else {
                line_height
            };

            // Draw debug boxes using actual heights
            let cap_height = measurer.cap_height_mm(ANSWERS_FONT_SIZE);
            let descender = measurer.descender_mm(ANSWERS_FONT_SIZE);
            let box_top = row_start_y + cap_height;
            // Box height from top of first line to bottom of descenders on last line
            let box_height = cap_height + row_height - last_line_height + descender;

            self.draw_debug_box(
                layer,
                margin_left,
                box_top,
                CONTEXT_COLUMN_WIDTH,
                box_height,
            );
            self.draw_debug_box(
                layer,
                margin_left + CONTEXT_COLUMN_WIDTH,
                box_top,
                HAND_COLUMN_WIDTH,
                box_height,
            );
            self.draw_debug_box(
                layer,
                margin_left + CONTEXT_COLUMN_WIDTH + HAND_COLUMN_WIDTH,
                box_top,
                HAND_COLUMN_WIDTH,
                box_height,
            );
            self.draw_debug_box(
                layer,
                margin_left + CONTEXT_COLUMN_WIDTH + 2.0 * HAND_COLUMN_WIDTH,
                box_top,
                60.0,
                box_height,
            );

            current_y = row_start_y - row_height - ROW_GAP;
        }
    }

    /// Render board context (board number, dealer, vulnerability, HCP)
    #[allow(clippy::too_many_arguments)]
    fn render_board_context(
        &self,
        layer: &mut LayerBuilder,
        board: &Board,
        player: Option<Direction>,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        bold_font: &FontId,
    ) {
        let line_height = font_size * LINE_HEIGHT_MULTIPLIER * 0.4;
        let mut current_y = y;

        layer.set_fill_color(Color::Rgb(BLACK));

        // Board number
        if let Some(num) = board.number {
            layer.use_text(
                format!("Board: {}", num),
                font_size,
                Mm(x),
                Mm(current_y),
                bold_font,
            );
            current_y -= line_height;
        }

        // Dealer
        if let Some(dealer) = board.dealer {
            layer.use_text(
                format!("Dealer: {}", dealer),
                font_size,
                Mm(x),
                Mm(current_y),
                text_font,
            );
            current_y -= line_height;
        }

        // Vulnerability
        let vul_str = match board.vulnerable {
            Vulnerability::None => "None",
            Vulnerability::NorthSouth => "N-S",
            Vulnerability::EastWest => "E-W",
            Vulnerability::Both => "Both",
        };
        layer.use_text(
            format!("Vul: {}", vul_str),
            font_size,
            Mm(x),
            Mm(current_y),
            text_font,
        );
        current_y -= line_height;

        // HCP (for single player)
        if let Some(player) = player {
            let hcp = board.deal.hand(player).total_hcp();
            layer.use_text(
                format!("HCP: {}", hcp),
                font_size,
                Mm(x),
                Mm(current_y),
                text_font,
            );
        }
    }

    /// Render full board context for answers page (includes both HCPs and contract)
    #[allow(clippy::too_many_arguments)]
    fn render_board_context_full(
        &self,
        layer: &mut LayerBuilder,
        board: &Board,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        bold_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) {
        let line_height = font_size * LINE_HEIGHT_MULTIPLIER * 0.4;
        let mut current_y = y;
        let measurer = get_measurer();

        layer.set_fill_color(Color::Rgb(BLACK));

        // Board number
        if let Some(num) = board.number {
            layer.use_text(
                format!("Board: {}", num),
                font_size,
                Mm(x),
                Mm(current_y),
                bold_font,
            );
            current_y -= line_height;
        }

        // Dealer
        if let Some(dealer) = board.dealer {
            layer.use_text(
                format!("Dealer: {}", dealer),
                font_size,
                Mm(x),
                Mm(current_y),
                text_font,
            );
            current_y -= line_height;
        }

        // Vulnerability
        let vul_str = match board.vulnerable {
            Vulnerability::None => "None",
            Vulnerability::NorthSouth => "N-S",
            Vulnerability::EastWest => "E-W",
            Vulnerability::Both => "Both",
        };
        layer.use_text(
            format!("Vul: {}", vul_str),
            font_size,
            Mm(x),
            Mm(current_y),
            text_font,
        );
        current_y -= line_height;

        // North HCP
        let north_hcp = board.deal.north.total_hcp();
        layer.use_text(
            format!("North HCP: {}", north_hcp),
            font_size,
            Mm(x),
            Mm(current_y),
            text_font,
        );
        current_y -= line_height;

        // South HCP
        let south_hcp = board.deal.south.total_hcp();
        layer.use_text(
            format!("South HCP: {}", south_hcp),
            font_size,
            Mm(x),
            Mm(current_y),
            text_font,
        );
        current_y -= line_height;

        // Contract (if available)
        if let Some(ref auction) = board.auction {
            if let Some(contract) = auction.final_contract() {
                let prefix = "Contract: ";
                layer.use_text(prefix, font_size, Mm(x), Mm(current_y), text_font);

                let prefix_width = measurer.measure_width_mm(prefix, font_size);
                let mut contract_x = x + prefix_width;

                // Level
                let level_str = contract.level.to_string();
                layer.use_text(
                    &level_str,
                    font_size,
                    Mm(contract_x),
                    Mm(current_y),
                    text_font,
                );
                contract_x += measurer.measure_width_mm(&level_str, font_size);

                // Suit symbol
                let (symbol, use_symbol_font) = match contract.suit {
                    BidSuit::Clubs => ("\u{2663}", true),
                    BidSuit::Diamonds => ("\u{2666}", true),
                    BidSuit::Hearts => ("\u{2665}", true),
                    BidSuit::Spades => ("\u{2660}", true),
                    BidSuit::NoTrump => ("NT", false),
                };

                if contract.suit.is_red() {
                    layer.set_fill_color(Color::Rgb(colors.hearts.clone()));
                } else {
                    layer.set_fill_color(Color::Rgb(BLACK));
                }

                let font = if use_symbol_font {
                    symbol_font
                } else {
                    text_font
                };
                layer.use_text(symbol, font_size, Mm(contract_x), Mm(current_y), font);
                contract_x += measurer.measure_width_mm(symbol, font_size);

                layer.set_fill_color(Color::Rgb(BLACK));

                // Doubled/Redoubled
                if contract.redoubled {
                    layer.use_text("XX", font_size, Mm(contract_x), Mm(current_y), text_font);
                    contract_x += measurer.measure_width_mm("XX", font_size);
                } else if contract.doubled {
                    layer.use_text("X", font_size, Mm(contract_x), Mm(current_y), text_font);
                    contract_x += measurer.measure_width_mm("X", font_size);
                }

                // Declarer
                let declarer_str = format!(" {}", contract.declarer);
                layer.use_text(
                    &declarer_str,
                    font_size,
                    Mm(contract_x),
                    Mm(current_y),
                    text_font,
                );
            }
        }
    }

    /// Render a hand in vertical layout (suit per line)
    #[allow(clippy::too_many_arguments)]
    fn render_hand_vertical(
        &self,
        layer: &mut LayerBuilder,
        hand: &Hand,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) {
        let line_height = font_size * LINE_HEIGHT_MULTIPLIER * 0.4;
        let measurer = get_measurer();
        let mut current_y = y;

        let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];

        for suit in suits {
            let holding = hand.holding(suit);

            // Suit symbol
            let symbol = suit.symbol().to_string();
            let suit_color = colors.for_suit(
                &crate::model::Card {
                    suit,
                    rank: crate::model::Rank::Ace,
                }
                .suit,
            );
            layer.set_fill_color(Color::Rgb(suit_color));
            layer.use_text(&symbol, font_size, Mm(x), Mm(current_y), symbol_font);

            let symbol_width = measurer.measure_width_mm(&symbol, font_size);

            // Holding (or void dash)
            layer.set_fill_color(Color::Rgb(BLACK));
            let holding_str = if holding.is_void() {
                "\u{2014}".to_string() // Em dash for void
            } else {
                holding.to_string()
            };
            layer.use_text(
                &holding_str,
                font_size,
                Mm(x + symbol_width + 1.0),
                Mm(current_y),
                text_font,
            );

            current_y -= line_height;
        }
    }

    /// Render auction setup (opposition bidding + who bids first)
    #[allow(clippy::too_many_arguments)]
    fn render_auction_setup(
        &self,
        layer: &mut LayerBuilder,
        board: &Board,
        player: Direction,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) {
        let line_height = font_size * LINE_HEIGHT_MULTIPLIER * 0.4;
        let mut current_y = y;

        // Opposition bidding
        let opp_lines =
            self.format_opposition_bidding(board, text_font, symbol_font, colors, font_size);
        for line in &opp_lines {
            self.render_mixed_text(
                layer,
                line,
                x,
                current_y,
                font_size,
                text_font,
                symbol_font,
                colors,
            );
            current_y -= line_height;
        }

        // Add gap
        current_y -= line_height * 0.5;

        // Who bids first
        let who_first = self.who_bids_first(board, player);
        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text(&who_first, font_size, Mm(x), Mm(current_y), text_font);
    }

    /// Format opposition bidding as text lines
    fn format_opposition_bidding(
        &self,
        board: &Board,
        _text_font: &FontId,
        _symbol_font: &FontId,
        _colors: &SuitColors,
        _font_size: f32,
    ) -> Vec<MixedText> {
        let mut lines = Vec::new();

        let Some(ref auction) = board.auction else {
            lines.push(MixedText::plain("The opponents are silent."));
            return lines;
        };

        let mut current_seat = auction.dealer;
        let mut is_opening_bid = true;
        let mut prev_bid: Option<(u8, BidSuit)> = None;
        let mut found_opp_bid = false;

        for annotated in &auction.calls {
            let is_opponent = matches!(current_seat, Direction::East | Direction::West);

            if is_opponent {
                match &annotated.call {
                    Call::Bid { level, suit } => {
                        let action = if is_opening_bid { "Opens" } else { "Bids" };
                        lines.push(MixedText::bid_action(current_seat, action, *level, *suit));
                        is_opening_bid = false;
                        prev_bid = Some((*level, *suit));
                        found_opp_bid = true;
                    }
                    Call::Double => {
                        if let Some((level, suit)) = prev_bid {
                            lines.push(MixedText::double_action(current_seat, level, suit));
                        }
                        found_opp_bid = true;
                    }
                    Call::Redouble => {
                        lines.push(MixedText::plain(&format!(
                            "{} Redoubles if possible.",
                            current_seat
                        )));
                        found_opp_bid = true;
                    }
                    Call::Pass => {}
                }
            } else {
                // Track N/S bids for "doubles X" context
                if let Call::Bid { level, suit } = &annotated.call {
                    prev_bid = Some((*level, *suit));
                    is_opening_bid = false;
                }
            }

            current_seat = current_seat.next();
        }

        if !found_opp_bid {
            lines.push(MixedText::plain("The opponents are silent."));
        }

        lines
    }

    /// Determine who bids first for a given player
    fn who_bids_first(&self, board: &Board, player: Direction) -> String {
        let Some(dealer) = board.dealer else {
            return String::new();
        };

        let partner = player.partner();

        // Player bids first if dealer is player or RHO
        let rho = match player {
            Direction::North => Direction::West,
            Direction::East => Direction::North,
            Direction::South => Direction::East,
            Direction::West => Direction::South,
        };

        if dealer == player || dealer == rho {
            "You bid first.".to_string()
        } else if dealer == partner {
            "Partner bids first.".to_string()
        } else {
            // LHO deals, so RHO acts before you but partner is still before you
            "Partner bids first.".to_string()
        }
    }

    /// Render mixed text (text with embedded suit symbols)
    #[allow(clippy::too_many_arguments)]
    fn render_mixed_text(
        &self,
        layer: &mut LayerBuilder,
        text: &MixedText,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) {
        let measurer = get_measurer();
        let mut current_x = x;

        for segment in &text.segments {
            match segment {
                TextSegment::Plain(s) => {
                    layer.set_fill_color(Color::Rgb(BLACK));
                    layer.use_text(s, font_size, Mm(current_x), Mm(y), text_font);
                    current_x += measurer.measure_width_mm(s, font_size);
                }
                TextSegment::Suit(suit) => {
                    let suit_color = colors.for_bid_suit(suit);
                    layer.set_fill_color(Color::Rgb(suit_color));
                    let symbol = suit.symbol();
                    layer.use_text(symbol, font_size, Mm(current_x), Mm(y), symbol_font);
                    current_x += measurer.measure_width_mm(symbol, font_size);
                }
            }
        }
    }

    /// Render auction table in W/N/E/S columns
    /// Returns (total_height, last_line_height) so caller can correctly compute box bounds
    #[allow(clippy::too_many_arguments)]
    fn render_auction_table(
        &self,
        layer: &mut LayerBuilder,
        auction: &Auction,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        bold_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) -> (f32, f32) {
        let line_height = font_size * LINE_HEIGHT_MULTIPLIER * 0.4;
        let col_width = 12.0; // Column width for each seat
        let mut current_y = y;

        // Header row
        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text("W", font_size, Mm(x), Mm(current_y), bold_font);
        layer.use_text("N", font_size, Mm(x + col_width), Mm(current_y), bold_font);
        layer.use_text(
            "E",
            font_size,
            Mm(x + 2.0 * col_width),
            Mm(current_y),
            bold_font,
        );
        layer.use_text(
            "S",
            font_size,
            Mm(x + 3.0 * col_width),
            Mm(current_y),
            bold_font,
        );
        current_y -= line_height;

        // Determine starting column based on dealer
        let start_col = auction.dealer.table_position();

        // Render calls
        let mut col = start_col;
        let mut row_y = current_y;

        // Add dashes for positions before dealer
        for i in 0..start_col {
            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text(
                "\u{2014}",
                font_size,
                Mm(x + i as f32 * col_width),
                Mm(row_y),
                text_font,
            );
        }

        for annotated in &auction.calls {
            let col_x = x + col as f32 * col_width;

            self.render_annotated_call(
                layer,
                annotated,
                col_x,
                row_y,
                font_size,
                text_font,
                symbol_font,
                colors,
            );

            col += 1;
            if col >= 4 {
                col = 0;
                row_y -= line_height;
            }
        }

        // Track the last line height used (for correct box calculation)
        let mut last_line_height = line_height;

        // Render notes if present
        if !auction.notes.is_empty() {
            // Move to a new row after the auction
            if col > 0 {
                row_y -= line_height;
            }
            row_y -= line_height * 0.5; // Gap before notes

            let note_font_size = font_size * 0.85;
            let note_line_height = note_font_size * LINE_HEIGHT_MULTIPLIER * 0.4;

            // Get sorted note numbers
            let mut note_nums: Vec<&u8> = auction.notes.keys().collect();
            note_nums.sort();

            for num in note_nums {
                if let Some(text) = auction.notes.get(num) {
                    let note_text = format!("{}. {}", num, text);
                    layer.set_fill_color(Color::Rgb(BLACK));
                    layer.use_text(&note_text, note_font_size, Mm(x), Mm(row_y), text_font);
                    row_y -= note_line_height;
                }
            }
            // Notes use smaller line height
            last_line_height = note_line_height;
        }

        // Return (total height, last line height used)
        (y - row_y, last_line_height)
    }

    /// Render an annotated call (call with optional superscript annotation)
    #[allow(clippy::too_many_arguments)]
    fn render_annotated_call(
        &self,
        layer: &mut LayerBuilder,
        annotated: &AnnotatedCall,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) {
        let call_width = self.render_call(layer, &annotated.call, x, y, font_size, text_font, symbol_font, colors);

        // If there's an annotation, render it as superscript
        if let Some(ref annotation) = annotated.annotation {
            let sup_x = x + call_width;
            let sup_y = y + (font_size * SUPERSCRIPT_RISE * 0.352778); // Convert pt to mm
            let sup_size = font_size * SUPERSCRIPT_RATIO;

            layer.set_fill_color(Color::Rgb(BLACK));
            layer.use_text(annotation, sup_size, Mm(sup_x), Mm(sup_y), text_font);
        }
    }

    /// Render a single call and return the width used
    #[allow(clippy::too_many_arguments)]
    fn render_call(
        &self,
        layer: &mut LayerBuilder,
        call: &Call,
        x: f32,
        y: f32,
        font_size: f32,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) -> f32 {
        let measurer = get_measurer();

        match call {
            Call::Pass => {
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text("Pass", font_size, Mm(x), Mm(y), text_font);
                measurer.measure_width_mm("Pass", font_size)
            }
            Call::Double => {
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text("X", font_size, Mm(x), Mm(y), text_font);
                measurer.measure_width_mm("X", font_size)
            }
            Call::Redouble => {
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text("XX", font_size, Mm(x), Mm(y), text_font);
                measurer.measure_width_mm("XX", font_size)
            }
            Call::Bid { level, suit } => {
                // Level
                let level_str = level.to_string();
                layer.set_fill_color(Color::Rgb(BLACK));
                layer.use_text(&level_str, font_size, Mm(x), Mm(y), text_font);

                let level_width = measurer.measure_width_mm(&level_str, font_size);

                // Suit
                let (symbol, use_symbol_font) = match suit {
                    BidSuit::Clubs => ("\u{2663}", true),
                    BidSuit::Diamonds => ("\u{2666}", true),
                    BidSuit::Hearts => ("\u{2665}", true),
                    BidSuit::Spades => ("\u{2660}", true),
                    BidSuit::NoTrump => ("NT", false),
                };

                if suit.is_red() {
                    layer.set_fill_color(Color::Rgb(colors.hearts.clone()));
                } else {
                    layer.set_fill_color(Color::Rgb(BLACK));
                }

                let font = if use_symbol_font {
                    symbol_font
                } else {
                    text_font
                };
                layer.use_text(symbol, font_size, Mm(x + level_width), Mm(y), font);

                let symbol_width = measurer.measure_width_mm(symbol, font_size);
                level_width + symbol_width
            }
        }
    }

    /// Calculate auction height for layout - must match render_auction_table exactly
    fn calculate_auction_height(&self, board: &Board, font_size: f32) -> f32 {
        let line_height = font_size * LINE_HEIGHT_MULTIPLIER * 0.4;

        let Some(ref auction) = board.auction else {
            return line_height;
        };

        // Starting column based on dealer (same as render_auction_table)
        let start_col = auction.dealer.table_position();

        // Header row
        let mut height = line_height;

        // Calculate how many full rows of calls we have
        // Total positions used = start_col + calls.len()
        // Number of rows = ceil(total_positions / 4)
        let total_positions = start_col + auction.calls.len();
        let call_rows = total_positions.div_ceil(4);
        height += call_rows as f32 * line_height;

        // Account for notes if present
        if !auction.notes.is_empty() {
            // Final column position after all calls
            let final_col = total_positions % 4;
            // Extra line if last row of bids wasn't complete
            if final_col > 0 {
                height += line_height;
            }
            // Gap before notes
            height += line_height * 0.5;

            // Notes themselves (smaller font)
            let note_font_size = font_size * 0.85;
            let note_line_height = note_font_size * LINE_HEIGHT_MULTIPLIER * 0.4;
            height += auction.notes.len() as f32 * note_line_height;
        }

        height
    }
}

/// Mixed text with plain text and suit symbols
struct MixedText {
    segments: Vec<TextSegment>,
}

enum TextSegment {
    Plain(String),
    Suit(BidSuit),
}

impl MixedText {
    fn plain(s: &str) -> Self {
        Self {
            segments: vec![TextSegment::Plain(s.to_string())],
        }
    }

    fn bid_action(seat: Direction, action: &str, level: u8, suit: BidSuit) -> Self {
        Self {
            segments: vec![
                TextSegment::Plain(format!("{} {} {}", seat, action, level)),
                TextSegment::Suit(suit),
                TextSegment::Plain(".".to_string()),
            ],
        }
    }

    fn double_action(seat: Direction, level: u8, suit: BidSuit) -> Self {
        Self {
            segments: vec![
                TextSegment::Plain(format!("{} Doubles {}", seat, level)),
                TextSegment::Suit(suit),
                TextSegment::Plain(" if possible.".to_string()),
            ],
        }
    }
}

/// Extension trait for SuitColors to handle BidSuit
trait SuitColorsExt {
    fn for_bid_suit(&self, suit: &BidSuit) -> Rgb;
}

impl SuitColorsExt for SuitColors {
    fn for_bid_suit(&self, suit: &BidSuit) -> Rgb {
        match suit {
            BidSuit::Hearts | BidSuit::Diamonds => self.hearts.clone(),
            BidSuit::Spades | BidSuit::Clubs | BidSuit::NoTrump => self.spades.clone(),
        }
    }
}
