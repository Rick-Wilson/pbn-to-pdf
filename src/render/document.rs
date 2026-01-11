use crate::config::Settings;
use crate::error::RenderError;
use crate::model::{BidSuit, Board};
use printpdf::path::PaintMode;
use printpdf::{Color, IndirectFontRef, Mm, PdfDocument, PdfLayerReference, Rect, Rgb};

use super::bidding_table::BiddingTableRenderer;
use super::colors::{SuitColors, BLACK};
use super::commentary::{CommentaryRenderer, FloatLayout};
use super::fonts::FontManager;
use super::hand_diagram::HandDiagramRenderer;
use super::page::PageManager;
use super::text_metrics::get_measurer;

/// Light gray color for debug boxes
const DEBUG_BOX_COLOR: Rgb = Rgb { r: 0.7, g: 0.7, b: 0.7, icc_profile: None };
const DEBUG_BOXES: bool = false;

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

        let (doc, page_idx, layer_idx) = PdfDocument::new(
            title,
            Mm(self.settings.page_width),
            Mm(self.settings.page_height),
            "Layer 1",
        );

        let fonts = FontManager::new(&doc)?;
        let page_manager = PageManager::new(&self.settings);

        let mut current_layer = doc.get_page(page_idx).get_layer(layer_idx);
        let mut page_count = 0;

        for (i, board) in boards.iter().enumerate() {
            // Create new page if needed (skip first board, already have a page)
            if i > 0 && page_manager.needs_new_page(i) {
                let (_, layer) = page_manager.create_page(&doc, page_count + 1);
                current_layer = layer;
                page_count += 1;
            }

            self.render_board(&current_layer, board, &fonts);
        }

        // Save to bytes
        let bytes = doc.save_to_bytes().map_err(|e| {
            RenderError::PdfGeneration(format!("Failed to save PDF: {:?}", e))
        })?;

        Ok(bytes)
    }

    /// Draw a debug outline box
    fn draw_debug_box(&self, layer: &printpdf::PdfLayerReference, x: f32, y: f32, w: f32, h: f32) {
        if !DEBUG_BOXES {
            return;
        }
        // y is top of box, draw from bottom-left to top-right
        let rect = Rect::new(Mm(x), Mm(y - h), Mm(x + w), Mm(y))
            .with_mode(PaintMode::Stroke);
        layer.set_outline_color(Color::Rgb(DEBUG_BOX_COLOR));
        layer.set_outline_thickness(0.25);
        layer.add_rect(rect);
    }

    /// Render a single board - Bridge Composer style layout
    fn render_board(
        &self,
        layer: &printpdf::PdfLayerReference,
        board: &Board,
        fonts: &FontManager,
    ) {
        let margin_left = self.settings.margin_left;
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
        let font_size = self.settings.body_font_size;
        let mut title_lines: Vec<String> = Vec::new();

        if let Some(num) = board.number {
            title_lines.push(format!("Deal {}", num));  // Changed from "Board" to "Deal"
        }
        if let Some(dealer) = board.dealer {
            title_lines.push(format!("{} Deals", dealer));
        }
        title_lines.push(board.vulnerable.to_string());

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

        // Render title text with cap-height offset
        let first_baseline = title_start_y - cap_height;
        let mut current_line = 0;

        layer.set_fill_color(Color::Rgb(BLACK));

        // Line 1: Deal number (bold italic) - use hand_record font
        if let Some(num) = board.number {
            let y = first_baseline - (current_line as f32 * line_height);
            layer.use_text(
                format!("Deal {}", num),
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

        // Diagram origin: same Y as page_top (North aligns with "Board 1")
        // The diagram renderer will place North to the right (after hand_width gap for title)
        let diagram_x = margin_left;
        let diagram_y = page_top;  // Start at same level as title

        let hand_renderer = HandDiagramRenderer::new(
            layer,
            &diagram_fonts.regular,
            &diagram_fonts.bold,
            &card_table_fonts.regular,  // Compass uses CardTable font
            &fonts.sans.regular,  // DejaVu Sans for suit symbols
            &self.settings,
        );
        let diagram_height = hand_renderer.render_deal(&board.deal, (Mm(diagram_x), Mm(diagram_y)));

        // Content below diagram
        let mut content_y = Mm(diagram_y - diagram_height - 5.0);

        // Render bidding table if present
        if self.settings.show_bidding {
            if let Some(ref auction) = board.auction {
                let bidding_renderer = BiddingTableRenderer::new(
                    layer,
                    &hand_record_fonts.regular,
                    &hand_record_fonts.bold,
                    &hand_record_fonts.italic,
                    &fonts.sans.regular,  // DejaVu Sans for suit symbols
                    &self.settings,
                );
                let table_height = bidding_renderer.render(auction, (Mm(margin_left), content_y));
                content_y = Mm(content_y.0 - table_height - 2.0);

                // Render contract below auction (no label)
                if let Some(contract) = auction.final_contract() {
                    let colors = SuitColors::new(self.settings.black_color, self.settings.red_color);
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
                            let colors = SuitColors::new(self.settings.black_color, self.settings.red_color);
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
                layer,
                &commentary_fonts.regular,
                &commentary_fonts.bold,
                &commentary_fonts.italic,
                &fonts.sans.regular,  // DejaVu Sans for suit symbols
                &self.settings,
            );

            // Calculate floating layout:
            // - Commentary starts at page_top, on the right half of the page
            // - Float until we clear the deal info (content_y is below diagram + bidding + contract + lead)
            // - Then switch to full width

            let full_width = self.settings.content_width();
            let page_center = margin_left + full_width / 2.0;
            let float_width = full_width / 2.0 - 2.0;  // Small gap from center

            // The float_until_y is where the deal content ends (current content_y)
            let float_until_y = content_y.0;

            let float_layout = FloatLayout {
                float_until_y,
                float_left: page_center + 2.0,  // Start just right of center
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
                            block,
                            (Mm(float_layout.float_left), Mm(commentary_y)),
                            &float_layout,
                        );
                        commentary_y = result.final_y - 3.0;
                    } else {
                        // Below float zone, use full width
                        let height = commentary_renderer.render(
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
    fn render_contract(
        &self,
        layer: &PdfLayerReference,
        contract: &crate::model::Contract,
        x: Mm,
        y: Mm,
        text_font: &IndirectFontRef,
        symbol_font: &IndirectFontRef,
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

        let font = if use_symbol_font { symbol_font } else { text_font };
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
    fn render_lead(
        &self,
        layer: &PdfLayerReference,
        card: &crate::model::Card,
        x: Mm,
        y: Mm,
        text_font: &IndirectFontRef,
        symbol_font: &IndirectFontRef,
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
