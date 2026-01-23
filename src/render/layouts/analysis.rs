use crate::config::Settings;
use crate::error::RenderError;
use crate::model::{BidSuit, Board};
use printpdf::{Color, FontId, Mm, PaintMode, PdfDocument, PdfPage, PdfSaveOptions, Rgb};

use crate::render::components::bidding_table::BiddingTableRenderer;
use crate::render::components::commentary::{CommentaryRenderer, FloatLayout};
use crate::render::components::hand_diagram::{DiagramDisplayOptions, HandDiagramRenderer};
use crate::render::helpers::colors::{SuitColors, BLACK};
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
// Debug boxes are now controlled via settings.debug_boxes

/// Dark gray color for separator lines
const SEPARATOR_COLOR: Rgb = Rgb {
    r: 0.4,
    g: 0.4,
    b: 0.4,
    icc_profile: None,
};

/// Separator line thickness
const SEPARATOR_THICKNESS: f32 = 0.5;

/// Main document renderer
pub struct DocumentRenderer {
    settings: Settings,
}

impl DocumentRenderer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Generate a PDF from a list of boards
    pub fn render(&self, boards: &[Board]) -> Result<Vec<u8>, RenderError> {
        let title = boards
            .first()
            .and_then(|b| b.event.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Bridge Hands");

        let mut doc = PdfDocument::new(title);

        // Load fonts - printpdf 0.8 handles subsetting automatically
        let fonts = FontManager::new(&mut doc)?;

        let mut pages = Vec::new();

        if self.settings.two_column {
            // Two-column layout: fit multiple boards per page
            pages = self.render_two_column(boards, &fonts);
        } else {
            // Single board per page (original behavior)
            for board in boards {
                let mut layer = LayerBuilder::new();
                self.render_board(&mut layer, board, &fonts, self.settings.margin_left);

                let page = PdfPage::new(
                    Mm(self.settings.page_width),
                    Mm(self.settings.page_height),
                    layer.into_ops(),
                );
                pages.push(page);
            }
        }

        doc.with_pages(pages);

        // Save with auto-subsetting enabled (default)
        let mut warnings = Vec::new();
        let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);

        Ok(bytes)
    }

    /// Render boards in two-column layout with multiple boards per page
    fn render_two_column(&self, boards: &[Board], fonts: &FontManager) -> Vec<PdfPage> {
        let mut pages = Vec::new();

        let page_width = self.settings.page_width;
        let page_height = self.settings.page_height;
        let margin_left = self.settings.margin_left;
        let margin_right = self.settings.margin_right;
        let margin_top = self.settings.margin_top;
        let margin_bottom = self.settings.margin_bottom;

        let content_width = page_width - margin_left - margin_right;
        let column_width = content_width / 2.0;
        let gutter = 5.0; // Space between columns
        let usable_column_width = column_width - gutter / 2.0;

        // Calculate center x for vertical separator
        let center_x = margin_left + column_width;

        // Minimum space needed below content before starting new page
        let min_space_for_board = 25.0; // Enough for at least a small fragment

        // Process boards dynamically - fill each column until no more space
        let mut board_iter = boards.iter().peekable();

        while board_iter.peek().is_some() {
            let mut layer = LayerBuilder::new();

            // Draw vertical separator line
            layer.set_outline_color(Color::Rgb(SEPARATOR_COLOR));
            layer.set_outline_thickness(SEPARATOR_THICKNESS);
            layer.add_line(
                Mm(center_x),
                Mm(margin_bottom),
                Mm(center_x),
                Mm(page_height - margin_top),
            );

            // Track positions for both columns
            let mut left_y = page_height - margin_top;
            let mut right_y = page_height - margin_top;
            let mut left_board_count = 0;
            let mut right_board_count = 0;

            // Fill left column first
            while board_iter.peek().is_some() && left_y > margin_bottom + min_space_for_board {
                if let Some(board) = board_iter.next() {
                    // Draw horizontal separator if not at top
                    if left_board_count > 0 {
                        let sep_y = left_y + 2.0;
                        layer.set_outline_color(Color::Rgb(SEPARATOR_COLOR));
                        layer.set_outline_thickness(SEPARATOR_THICKNESS);
                        layer.add_line(
                            Mm(margin_left),
                            Mm(sep_y),
                            Mm(center_x - gutter / 2.0),
                            Mm(sep_y),
                        );
                    }

                    let board_height = self.render_board_in_column(
                        &mut layer,
                        board,
                        fonts,
                        margin_left,
                        left_y,
                        usable_column_width,
                    );

                    left_y -= board_height + 5.0;
                    left_board_count += 1;
                } else {
                    break;
                }
            }

            // Fill right column
            while board_iter.peek().is_some() && right_y > margin_bottom + min_space_for_board {
                if let Some(board) = board_iter.next() {
                    // Draw horizontal separator if not at top
                    if right_board_count > 0 {
                        let sep_y = right_y + 2.0;
                        layer.set_outline_color(Color::Rgb(SEPARATOR_COLOR));
                        layer.set_outline_thickness(SEPARATOR_THICKNESS);
                        layer.add_line(
                            Mm(center_x + gutter / 2.0),
                            Mm(sep_y),
                            Mm(page_width - margin_right),
                            Mm(sep_y),
                        );
                    }

                    let board_height = self.render_board_in_column(
                        &mut layer,
                        board,
                        fonts,
                        center_x + gutter / 2.0,
                        right_y,
                        usable_column_width,
                    );

                    right_y -= board_height + 5.0;
                    right_board_count += 1;
                } else {
                    break;
                }
            }

            let page = PdfPage::new(Mm(page_width), Mm(page_height), layer.into_ops());
            pages.push(page);
        }

        pages
    }

    /// Render a board within a column (for two-column layout)
    fn render_board_in_column(
        &self,
        layer: &mut LayerBuilder,
        board: &Board,
        fonts: &FontManager,
        column_x: f32,
        start_y: f32,
        column_width: f32,
    ) -> f32 {
        let line_height = self.settings.line_height;

        // Get font sets
        let diagram_fonts = fonts.set_for_spec(self.settings.fonts.diagram.as_ref());
        let card_table_fonts = fonts.set_for_spec(self.settings.fonts.card_table.as_ref());
        let hand_record_fonts = fonts.set_for_spec(self.settings.fonts.hand_record.as_ref());
        let commentary_fonts = fonts.set_for_spec(self.settings.fonts.commentary.as_ref());

        let measurer = get_measurer();
        let cap_height = measurer.cap_height_mm(self.settings.body_font_size);

        let mut current_y: f32;

        // Check BCFlags for visibility control
        // If deal is empty (no cards), hide board number, dealer, vulnerability, and diagram
        let flags = board.bc_flags;
        let deal_is_empty = board.deal.is_empty();
        let show_board = !deal_is_empty && flags.map(|f| !f.hide_board()).unwrap_or(true);
        let show_dealer = !deal_is_empty && flags.map(|f| !f.hide_dealer()).unwrap_or(true);
        let show_vulnerable = !deal_is_empty && flags.map(|f| !f.hide_vulnerable()).unwrap_or(true);
        let show_diagram = !deal_is_empty && flags.map(|f| f.show_diagram()).unwrap_or(true);
        let show_auction = flags.map(|f| f.show_auction()).unwrap_or(true) && self.settings.show_bidding;
        let show_commentary = self.settings.show_commentary
            && !board.commentary.is_empty()
            && flags.map(|f| f.show_event_commentary() || f.show_final_commentary()).unwrap_or(true);

        // Skip completely empty boards (nothing visible to show)
        if !show_board && !show_dealer && !show_vulnerable && !show_diagram && !show_commentary {
            return 0.0;
        }

        // Build and render title lines (Deal #, Dealer, Vulnerability)
        let font_size = self.settings.body_font_size;
        // Extra spacing before title (between separator line and title)
        let title_spacing = cap_height;
        // Title baseline: move down by title_spacing to create gap after separator
        let first_baseline = start_y - cap_height - title_spacing;
        let mut title_line = 0;

        layer.set_fill_color(Color::Rgb(BLACK));

        if show_board {
            if let Some(ref board_id) = board.board_id {
                let y = first_baseline - (title_line as f32 * line_height);
                // Use board label format from settings (e.g., "Board %" -> "Board 1", "%)" -> "1)")
                let label = self.settings.board_label_format.replace('%', board_id);
                layer.use_text(
                    label,
                    font_size,
                    Mm(column_x),
                    Mm(y),
                    &hand_record_fonts.bold_italic,
                );
                title_line += 1;
            }
        }

        if show_dealer {
            if let Some(dealer) = board.dealer {
                let y = first_baseline - (title_line as f32 * line_height);
                layer.use_text(
                    format!("{} Deals", dealer),
                    font_size,
                    Mm(column_x),
                    Mm(y),
                    &hand_record_fonts.regular,
                );
                title_line += 1;
            }
        }

        if show_vulnerable {
            let y = first_baseline - (title_line as f32 * line_height);
            layer.use_text(
                board.vulnerable.to_string(),
                font_size,
                Mm(column_x),
                Mm(y),
                &hand_record_fonts.regular,
            );
        }

        // Render hand diagram if enabled
        if show_diagram {
            let diagram_x = column_x;

            // Compute display options - all visibility decisions are made here
            let diagram_options = DiagramDisplayOptions::from_deal(&board.deal, &board.hidden);

            // Full compass: diagram starts at start_y (title already moved down by title_spacing)
            // Hidden compass: diagram starts below title lines
            let diagram_y = if diagram_options.hide_compass {
                first_baseline - (title_line as f32 * line_height)
            } else {
                start_y
            };

            let hand_renderer = HandDiagramRenderer::new(
                &diagram_fonts.regular,
                &diagram_fonts.bold,
                &card_table_fonts.regular,
                &fonts.sans.regular,
                &self.settings,
            );
            let diagram_height = hand_renderer.render_deal_with_options(
                layer,
                &board.deal,
                (Mm(diagram_x), Mm(diagram_y)),
                &diagram_options,
            );

            current_y = diagram_y - diagram_height;
        } else {
            // No diagram - content starts below title lines
            current_y = first_baseline - (title_line as f32 * line_height);
        }

        // Render bidding table if present and enabled
        if show_auction {
            if let Some(ref auction) = board.auction {
                let bidding_renderer = BiddingTableRenderer::new(
                    &hand_record_fonts.regular,
                    &hand_record_fonts.bold,
                    &hand_record_fonts.italic,
                    &fonts.sans.regular,
                    &self.settings,
                );
                let table_height =
                    bidding_renderer.render(layer, auction, (Mm(column_x), Mm(current_y)));
                current_y -= table_height + 2.0;

                // Render contract
                if let Some(contract) = auction.final_contract() {
                    let colors =
                        SuitColors::new(self.settings.black_color, self.settings.red_color);
                    self.render_contract(
                        layer,
                        &contract,
                        Mm(column_x),
                        Mm(current_y),
                        &hand_record_fonts.regular,
                        &fonts.sans.regular,
                        &colors,
                    );
                    current_y -= line_height;
                }

                // Render opening lead
                if let Some(ref play) = board.play {
                    if let Some(first_trick) = play.tricks.first() {
                        if let Some(lead_card) = first_trick.cards[0] {
                            let colors =
                                SuitColors::new(self.settings.black_color, self.settings.red_color);
                            self.render_lead(
                                layer,
                                &lead_card,
                                Mm(column_x),
                                Mm(current_y),
                                &hand_record_fonts.regular,
                                &fonts.sans.regular,
                                &colors,
                            );
                            current_y -= line_height;
                        }
                    }
                }

                current_y -= 2.0;
            }
        }

        // Render commentary - simplified for column layout (no floating)
        if show_commentary {
            let commentary_renderer = CommentaryRenderer::new(
                &commentary_fonts.regular,
                &commentary_fonts.bold,
                &commentary_fonts.italic,
                &commentary_fonts.bold_italic,
                &fonts.sans.regular,
                &self.settings,
            );

            for block in &board.commentary {
                let height =
                    commentary_renderer.render(layer, block, (Mm(column_x), Mm(current_y)), column_width);
                current_y -= height + 2.0;
            }
        }

        // Return total height used
        start_y - current_y
    }

    /// Draw a debug outline box
    fn draw_debug_box(&self, layer: &mut LayerBuilder, x: f32, y: f32, w: f32, h: f32) {
        if !self.settings.debug_boxes {
            return;
        }
        // y is top of box, draw from bottom-left to top-right
        layer.set_outline_color(Color::Rgb(DEBUG_BOX_COLOR));
        layer.set_outline_thickness(0.25);
        layer.add_rect(Mm(x), Mm(y - h), Mm(x + w), Mm(y), PaintMode::Stroke);
    }

    /// Render a single board - Bridge Composer style layout
    fn render_board(&self, layer: &mut LayerBuilder, board: &Board, fonts: &FontManager, margin_left: f32) {
        let margin_top = self.settings.margin_top;
        let page_top = self.settings.page_height - margin_top;
        let line_height = self.settings.line_height;

        // Get font sets based on PBN font specifications
        let diagram_fonts = fonts.set_for_spec(self.settings.fonts.diagram.as_ref());
        let card_table_fonts = fonts.set_for_spec(self.settings.fonts.card_table.as_ref());
        let hand_record_fonts = fonts.set_for_spec(self.settings.fonts.hand_record.as_ref());
        let commentary_fonts = fonts.set_for_spec(self.settings.fonts.commentary.as_ref());

        // Get font metrics for accurate box heights
        let measurer = get_measurer();
        let cap_height = measurer.cap_height_mm(self.settings.body_font_size);
        let descender = measurer.descender_mm(self.settings.body_font_size);

        // Title: 3 lines stacked vertically, positioned above West hand area
        let title_x = margin_left;
        let title_start_y = page_top;

        // Build title lines and measure widths
        // If deal is empty (no cards), hide board number, dealer, and vulnerability
        let font_size = self.settings.body_font_size;
        let mut title_lines: Vec<String> = Vec::new();
        let deal_is_empty = board.deal.is_empty();

        if !deal_is_empty {
            if let Some(ref board_id) = board.board_id {
                // Use board label format from settings (e.g., "Board %" -> "Board 1", "%)" -> "1)")
                let label = self.settings.board_label_format.replace('%', board_id);
                title_lines.push(label);
            }
            if let Some(dealer) = board.dealer {
                title_lines.push(format!("{} Deals", dealer));
            }
            title_lines.push(board.vulnerable.to_string());
        }

        let num_lines = title_lines.len();

        // Calculate actual width by measuring all lines
        let title_width = title_lines
            .iter()
            .map(|line| measurer.measure_width_mm(line, font_size))
            .fold(0.0_f32, |max, w| max.max(w));

        // Title box height: cap_height + (num_lines - 1) gaps + descender
        let title_height = cap_height + (num_lines - 1) as f32 * line_height + descender;

        // Draw debug box around title area
        self.draw_debug_box(layer, title_x, title_start_y, title_width, title_height);

        // Render title text with cap-height offset (only if deal has cards)
        let first_baseline = title_start_y - cap_height;
        let mut current_line = 0;

        layer.set_fill_color(Color::Rgb(BLACK));

        if !deal_is_empty {
            // Line 1: Board label (bold italic) - use hand_record font
            if let Some(ref board_id) = board.board_id {
                let y = first_baseline - (current_line as f32 * line_height);
                // Use board label format from settings (e.g., "Board %" -> "Board 1", "%)" -> "1)")
                let label = self.settings.board_label_format.replace('%', board_id);
                layer.use_text(
                    label,
                    self.settings.body_font_size,
                    Mm(title_x),
                    Mm(y),
                    &hand_record_fonts.bold_italic,
                );
                current_line += 1;
            }

            // Line 2: Dealer - use hand_record font
            if let Some(dealer) = board.dealer {
                let y = first_baseline - (current_line as f32 * line_height);
                layer.use_text(
                    format!("{} Deals", dealer),
                    self.settings.body_font_size,
                    Mm(title_x),
                    Mm(y),
                    &hand_record_fonts.regular,
                );
                current_line += 1;
            }

            // Line 3: Vulnerability - use hand_record font
            let y = first_baseline - (current_line as f32 * line_height);
            layer.use_text(
                board.vulnerable.to_string(),
                self.settings.body_font_size,
                Mm(title_x),
                Mm(y),
                &hand_record_fonts.regular,
            );
        }

        // Diagram origin: same Y as page_top (North aligns with "Board 1")
        // The diagram renderer will place North to the right (after hand_width gap for title)
        let diagram_x = margin_left;
        let diagram_y = page_top; // Start at same level as title

        // Content below diagram (or title if no diagram)
        let mut content_y;

        // Only render diagram if deal has cards
        if !deal_is_empty {
            // Compute display options - all visibility decisions are made here
            let diagram_options = DiagramDisplayOptions::from_deal(&board.deal, &board.hidden);

            let hand_renderer = HandDiagramRenderer::new(
                &diagram_fonts.regular,
                &diagram_fonts.bold,
                &card_table_fonts.regular, // Compass uses CardTable font
                &fonts.sans.regular,       // DejaVu Sans for suit symbols
                &self.settings,
            );
            let diagram_height = hand_renderer.render_deal_with_options(
                layer,
                &board.deal,
                (Mm(diagram_x), Mm(diagram_y)),
                &diagram_options,
            );

            content_y = Mm(diagram_y - diagram_height - 5.0);
        } else {
            // No diagram, content starts below any title lines
            let title_height = title_lines.len() as f32 * line_height;
            content_y = Mm(page_top - title_height - 5.0);
        }

        // Render bidding table if present
        if self.settings.show_bidding {
            if let Some(ref auction) = board.auction {
                let bidding_renderer = BiddingTableRenderer::new(
                    &hand_record_fonts.regular,
                    &hand_record_fonts.bold,
                    &hand_record_fonts.italic,
                    &fonts.sans.regular, // DejaVu Sans for suit symbols
                    &self.settings,
                );
                let table_height =
                    bidding_renderer.render(layer, auction, (Mm(margin_left), content_y));
                content_y = Mm(content_y.0 - table_height - 2.0);

                // Render contract below auction (no label)
                if let Some(contract) = auction.final_contract() {
                    let colors =
                        SuitColors::new(self.settings.black_color, self.settings.red_color);
                    let x = self.render_contract(
                        layer,
                        &contract,
                        Mm(margin_left),
                        content_y,
                        &hand_record_fonts.regular,
                        &fonts.sans.regular,
                        &colors,
                    );
                    // Continue after contract text
                    let _ = x; // Contract rendered inline
                    content_y = Mm(content_y.0 - line_height);
                }

                // Render opening lead if play sequence exists
                if let Some(ref play) = board.play {
                    if let Some(first_trick) = play.tricks.first() {
                        if let Some(lead_card) = first_trick.cards[0] {
                            let colors =
                                SuitColors::new(self.settings.black_color, self.settings.red_color);
                            self.render_lead(
                                layer,
                                &lead_card,
                                Mm(margin_left),
                                content_y,
                                &hand_record_fonts.regular,
                                &fonts.sans.regular,
                                &colors,
                            );
                            content_y = Mm(content_y.0 - line_height);
                        }
                    }
                }

                content_y = Mm(content_y.0 - 3.0);
            }
        }

        // Render commentary if present - using floating layout
        if self.settings.show_commentary && !board.commentary.is_empty() {
            let commentary_renderer = CommentaryRenderer::new(
                &commentary_fonts.regular,
                &commentary_fonts.bold,
                &commentary_fonts.italic,
                &commentary_fonts.bold_italic,
                &fonts.sans.regular, // DejaVu Sans for suit symbols
                &self.settings,
            );

            // Calculate floating layout:
            // - Commentary starts at page_top, on the right half of the page
            // - Float until we clear the deal info (content_y is below diagram + bidding + contract + lead)
            // - Then switch to full width

            let full_width = self.settings.content_width();
            let page_center = margin_left + full_width / 2.0;
            let float_width = full_width / 2.0 - 2.0; // Small gap from center

            // The float_until_y is where the deal content ends (current content_y)
            let float_until_y = content_y.0;

            let float_layout = FloatLayout {
                float_until_y,
                float_left: page_center + 2.0, // Start just right of center
                float_width,
                full_left: margin_left,
                full_width,
            };

            // Start commentary at the top of the page, using floating layout
            let mut commentary_y = page_top;
            let mut first_block = true;

            for block in &board.commentary {
                if first_block {
                    // First block uses floating layout
                    let result = commentary_renderer.render_float(
                        layer,
                        block,
                        (Mm(float_layout.float_left), Mm(commentary_y)),
                        &float_layout,
                    );
                    commentary_y = result.final_y - 3.0;
                    first_block = false;

                    // Update content_y if commentary went below the deal content
                    if commentary_y < content_y.0 {
                        content_y = Mm(commentary_y);
                    }
                } else {
                    // Subsequent blocks: check if we're still above float_until_y
                    if commentary_y > float_until_y {
                        // Still in float zone
                        let result = commentary_renderer.render_float(
                            layer,
                            block,
                            (Mm(float_layout.float_left), Mm(commentary_y)),
                            &float_layout,
                        );
                        commentary_y = result.final_y - 3.0;
                    } else {
                        // Below float zone, use full width
                        let height = commentary_renderer.render(
                            layer,
                            block,
                            (Mm(margin_left), Mm(commentary_y)),
                            full_width,
                        );
                        commentary_y -= height + 3.0;
                    }

                    if commentary_y < content_y.0 {
                        content_y = Mm(commentary_y);
                    }
                }
            }
        }
    }

    /// Render a contract with proper suit symbol font
    /// Returns the x position after the rendered text
    #[allow(clippy::too_many_arguments)]
    fn render_contract(
        &self,
        layer: &mut LayerBuilder,
        contract: &crate::model::Contract,
        x: Mm,
        y: Mm,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) -> f32 {
        let measurer = get_measurer();
        let font_size = self.settings.body_font_size;
        let mut current_x = x.0;

        // Render level
        let level_str = contract.level.to_string();
        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text(&level_str, font_size, Mm(current_x), y, text_font);
        current_x += measurer.measure_width_mm(&level_str, font_size);

        // Render suit symbol (or NT)
        let (symbol, use_symbol_font) = match contract.suit {
            BidSuit::Clubs => ("♣", true),
            BidSuit::Diamonds => ("♦", true),
            BidSuit::Hearts => ("♥", true),
            BidSuit::Spades => ("♠", true),
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
        layer.use_text(symbol, font_size, Mm(current_x), y, font);
        current_x += measurer.measure_width_mm(symbol, font_size);

        // Render doubled/redoubled
        layer.set_fill_color(Color::Rgb(BLACK));
        if contract.redoubled {
            layer.use_text("XX", font_size, Mm(current_x), y, text_font);
            current_x += measurer.measure_width_mm("XX", font_size);
        } else if contract.doubled {
            layer.use_text("X", font_size, Mm(current_x), y, text_font);
            current_x += measurer.measure_width_mm("X", font_size);
        }

        // Render " by [declarer]"
        let by_text = format!(" by {}", contract.declarer);
        layer.use_text(&by_text, font_size, Mm(current_x), y, text_font);
        current_x += measurer.measure_width_mm(&by_text, font_size);

        current_x
    }

    /// Render opening lead with proper suit symbol font
    #[allow(clippy::too_many_arguments)]
    fn render_lead(
        &self,
        layer: &mut LayerBuilder,
        card: &crate::model::Card,
        x: Mm,
        y: Mm,
        text_font: &FontId,
        symbol_font: &FontId,
        colors: &SuitColors,
    ) {
        let measurer = get_measurer();
        let font_size = self.settings.body_font_size;
        let mut current_x = x.0;

        // Render "Lead: "
        let prefix = "Lead: ";
        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text(prefix, font_size, Mm(current_x), y, text_font);
        current_x += measurer.measure_width_mm(prefix, font_size);

        // Render suit symbol with color
        let symbol = card.suit.symbol().to_string();
        let suit_color = colors.for_suit(&card.suit);
        layer.set_fill_color(Color::Rgb(suit_color));
        layer.use_text(&symbol, font_size, Mm(current_x), y, symbol_font);
        current_x += measurer.measure_width_mm(&symbol, font_size);

        // Render rank in black
        let rank = card.rank.to_char().to_string();
        layer.set_fill_color(Color::Rgb(BLACK));
        layer.use_text(&rank, font_size, Mm(current_x), y, text_font);
    }
}

/// Convenience function to generate PDF
pub fn generate_pdf(boards: &[Board], settings: &Settings) -> Result<Vec<u8>, RenderError> {
    let renderer = DocumentRenderer::new(settings.clone());
    renderer.render(boards)
}
