use crate::config::Settings;
use crate::model::{Deal, Hand, Suit};
use printpdf::{Color, FontId, Mm, PaintMode, Rgb};

use crate::render::helpers::colors::{self, SuitColors};
use crate::render::helpers::layer::LayerBuilder;
use crate::render::helpers::text_metrics;

/// Light gray color for debug boxes
const DEBUG_BOX_COLOR: Rgb = Rgb {
    r: 0.7,
    g: 0.7,
    b: 0.7,
    icc_profile: None,
};

/// Renderer for hand diagrams
pub struct HandDiagramRenderer<'a> {
    font: &'a FontId,
    bold_font: &'a FontId,
    compass_font: &'a FontId,
    symbol_font: &'a FontId, // Font with Unicode suit symbols (DejaVu Sans)
    colors: SuitColors,
    settings: &'a Settings,
    debug_boxes: bool,
}

impl<'a> HandDiagramRenderer<'a> {
    pub fn new(
        font: &'a FontId,
        bold_font: &'a FontId,
        compass_font: &'a FontId,
        symbol_font: &'a FontId,
        settings: &'a Settings,
    ) -> Self {
        Self {
            font,
            bold_font,
            compass_font,
            symbol_font,
            colors: SuitColors::new(settings.black_color, settings.red_color),
            settings,
            debug_boxes: false, // Disable debug boxes for production
        }
    }

    /// Draw a debug outline box
    fn draw_debug_box(&self, layer: &mut LayerBuilder, x: f32, y: f32, w: f32, h: f32) {
        if !self.debug_boxes {
            return;
        }
        // y is top of box, draw from bottom-left to top-right
        layer.set_outline_color(Color::Rgb(DEBUG_BOX_COLOR));
        layer.set_outline_thickness(0.25);
        layer.add_rect(Mm(x), Mm(y - h), Mm(x + w), Mm(y), PaintMode::Stroke);
    }

    /// Calculate the actual height of a hand block based on font metrics
    fn actual_hand_height(&self) -> f32 {
        let measurer = text_metrics::get_measurer();
        let line_height = self.settings.line_height;
        let cap_height = measurer.cap_height_mm(self.settings.card_font_size);
        let descender = measurer.descender_mm(self.settings.card_font_size);

        // 4 lines of text:
        // - cap_height: from top of box to first baseline
        // - 3 * line_height: 3 gaps between the 4 baselines
        // - descender: from last baseline to bottom of descenders
        cap_height + 3.0 * line_height + descender
    }

    /// Calculate the actual width of a hand by measuring all suit lines
    fn actual_hand_width(&self, hand: &Hand) -> f32 {
        let measurer = text_metrics::get_measurer();
        let font_size = self.settings.card_font_size;

        Suit::all()
            .iter()
            .map(|suit| {
                let holding = hand.holding(*suit);
                let cards_str = if holding.is_void() {
                    "-".to_string()
                } else {
                    holding
                        .ranks
                        .iter()
                        .map(|r| r.to_char().to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                // Full line: "♠ A K Q J T 9 8 7 6 5" (symbol + space + spaced cards)
                let line = format!("{} {}", suit.symbol(), cards_str);
                measurer.measure_width_mm(&line, font_size)
            })
            .fold(0.0_f32, |max, w| max.max(w))
    }

    /// Render a complete deal with compass rose - Bridge Composer style
    /// Returns the height used by the diagram
    pub fn render_deal(&self, layer: &mut LayerBuilder, deal: &Deal, origin: (Mm, Mm)) -> f32 {
        let (ox, oy) = origin;

        // Layout constants
        let hand_w = self.settings.hand_width; // Used for positioning
        let hand_h = self.actual_hand_height(); // Use actual calculated height
        let compass_size = self.compass_box_size(); // Dynamic size based on font

        // Calculate actual widths for each hand
        let north_w = self.actual_hand_width(&deal.north);
        let south_w = self.actual_hand_width(&deal.south);
        let east_w = self.actual_hand_width(&deal.east);
        let west_w = self.actual_hand_width(&deal.west);

        // Row 1: North hand (centered above compass)
        let north_x = ox.0 + hand_w + (compass_size - hand_w) / 2.0;
        let north_y = oy.0;
        self.draw_debug_box(layer, north_x, north_y, north_w, hand_h);
        self.render_hand_cards(layer, &deal.north, (Mm(north_x), Mm(north_y)));

        // Row 2: West hand | Compass | East hand (immediately below North)
        let row2_y = north_y - hand_h; // No extra gap

        // West hand - left side
        let west_x = ox.0;
        self.draw_debug_box(layer, west_x, row2_y, west_w, hand_h);
        self.render_hand_cards(layer, &deal.west, (Mm(west_x), Mm(row2_y)));

        // Compass rose - vertically centered with West/East hands
        // Left edge of compass aligns with right edge of suit symbols (suit symbols are ~5mm wide)
        let suit_symbol_width = 5.0;
        let half_char_adjust = 1.5; // Fine-tune alignment
        let compass_center_x = north_x + suit_symbol_width + compass_size / 2.0 - half_char_adjust;
        let compass_y = row2_y - hand_h / 2.0; // Center vertically with West/East
                                               // Debug box for compass (centered)
        self.draw_debug_box(
            layer,
            compass_center_x - compass_size / 2.0,
            compass_y + compass_size / 2.0,
            compass_size,
            compass_size,
        );
        self.render_compass(layer, (Mm(compass_center_x), Mm(compass_y)));

        // East hand - to the right of compass
        let east_x = compass_center_x + compass_size / 2.0 + 3.5;
        self.draw_debug_box(layer, east_x, row2_y, east_w, hand_h);
        self.render_hand_cards(layer, &deal.east, (Mm(east_x), Mm(row2_y)));

        // Row 3: HCP box (below West) and South hand (next to HCP box)
        let hcp_box_size = compass_size;
        let hcp_box_x = west_x;
        let hcp_box_y = row2_y - hand_h - 2.0; // Small gap below West hand

        if self.settings.show_hcp {
            self.render_hcp_box(layer, deal, (Mm(hcp_box_x), Mm(hcp_box_y)), hcp_box_size);
        }

        // South hand - positioned next to HCP box, at same Y level
        let south_y = hcp_box_y;
        self.draw_debug_box(layer, north_x, south_y, south_w, hand_h);
        self.render_hand_cards(layer, &deal.south, (Mm(north_x), Mm(south_y)));

        // Return total height used
        oy.0 - (south_y - hand_h)
    }

    /// Render a single hand (used for backward compatibility)
    pub fn render_hand(
        &self,
        layer: &mut LayerBuilder,
        hand: &Hand,
        origin: (Mm, Mm),
        _show_hcp: bool,
    ) {
        self.render_hand_cards(layer, hand, origin);
    }

    /// Render hand cards only (no HCP)
    /// Origin is the top-left of the visual bounding box
    fn render_hand_cards(&self, layer: &mut LayerBuilder, hand: &Hand, origin: (Mm, Mm)) {
        let (ox, oy) = origin;
        let line_height = self.settings.line_height;

        // Use actual font metrics to get cap-height
        let measurer = text_metrics::get_measurer();
        let cap_height = measurer.cap_height_mm(self.settings.card_font_size);

        // First baseline is below the top by cap-height
        // This aligns the top of capital letters with the bounding box top
        let first_baseline = oy.0 - cap_height;

        // Render each suit
        for (i, suit) in Suit::all().iter().enumerate() {
            let y = first_baseline - (i as f32 * line_height);
            self.render_suit_line(layer, *suit, hand.holding(*suit), (Mm(ox.0), Mm(y)));
        }
    }

    /// Render a single suit line (symbol + cards)
    fn render_suit_line(
        &self,
        layer: &mut LayerBuilder,
        suit: Suit,
        holding: &crate::model::Holding,
        origin: (Mm, Mm),
    ) {
        let (ox, oy) = origin;

        // Set color based on suit
        let color = self.colors.for_suit(&suit);
        layer.set_fill_color(Color::Rgb(color.clone()));

        // Render suit symbol using symbol font (DejaVu Sans has suit glyphs)
        let symbol = suit.symbol().to_string();
        layer.use_text(
            &symbol,
            self.settings.card_font_size,
            ox,
            oy,
            self.symbol_font,
        );

        // Render cards (in black) using regular font
        layer.set_fill_color(Color::Rgb(colors::BLACK));

        let cards_str = if holding.is_void() {
            "-".to_string()
        } else {
            holding
                .ranks
                .iter()
                .map(|r| r.to_char().to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };

        // Offset for cards (after suit symbol)
        let cards_x = Mm(ox.0 + 5.0);
        layer.use_text(
            &cards_str,
            self.settings.card_font_size,
            cards_x,
            oy,
            self.font,
        );
    }

    /// Calculate compass box size based on font metrics
    fn compass_box_size(&self) -> f32 {
        let measurer = text_metrics::get_measurer();
        let font_size = self.settings.compass_font_size;

        // Measure the widest letter (W is typically widest)
        let w_width = measurer.measure_width_mm("W", font_size);
        let cap_height = measurer.cap_height_mm(font_size);

        // Box needs to fit: letter on each side + padding
        // Width: W on left + gap + W on right + padding on edges
        // Height: N on top + gap + S on bottom + padding on edges
        let letter_size = w_width.max(cap_height);
        let padding = 1.5; // Small padding around letters at edges
        let inner_gap = letter_size * 1.6; // Gap between letters - proportional to letter size

        // Total: padding + letter + gap + letter + padding
        (padding * 2.0) + (letter_size * 2.0) + inner_gap
    }

    /// Render compass rose with green filled box and white letters
    fn render_compass(&self, layer: &mut LayerBuilder, center: (Mm, Mm)) {
        let (cx, cy) = center;
        let measurer = text_metrics::get_measurer();
        let font_size = self.settings.compass_font_size;

        let box_size = self.compass_box_size();
        let half_box = box_size / 2.0;

        // Get font metrics for positioning
        let cap_height = measurer.cap_height_mm(font_size);
        let n_width = measurer.measure_width_mm("N", font_size);
        let s_width = measurer.measure_width_mm("S", font_size);
        let e_width = measurer.measure_width_mm("E", font_size);

        // Draw filled green rectangle
        layer.set_fill_color(Color::Rgb(colors::GREEN));
        layer.add_rect(
            Mm(cx.0 - half_box),
            Mm(cy.0 - half_box),
            Mm(cx.0 + half_box),
            Mm(cy.0 + half_box),
            PaintMode::Fill,
        );

        // Draw white letters using compass font size
        layer.set_fill_color(Color::Rgb(colors::WHITE));

        let padding = 1.5;

        // N (top center) - baseline positioned so cap-height reaches near top edge
        layer.use_text(
            "N",
            font_size,
            Mm(cx.0 - n_width / 2.0),
            Mm(cy.0 + half_box - padding - cap_height),
            self.compass_font,
        );

        // S (bottom center) - baseline near bottom edge
        layer.use_text(
            "S",
            font_size,
            Mm(cx.0 - s_width / 2.0),
            Mm(cy.0 - half_box + padding),
            self.compass_font,
        );

        // W (left center) - vertically centered
        layer.use_text(
            "W",
            font_size,
            Mm(cx.0 - half_box + padding),
            Mm(cy.0 - cap_height / 2.0),
            self.compass_font,
        );

        // E (right center) - vertically centered
        layer.use_text(
            "E",
            font_size,
            Mm(cx.0 + half_box - padding - e_width),
            Mm(cy.0 - cap_height / 2.0),
            self.compass_font,
        );
    }

    /// Render HCP box with all four hands' point counts
    /// Origin is top-left of the box
    fn render_hcp_box(
        &self,
        layer: &mut LayerBuilder,
        deal: &Deal,
        origin: (Mm, Mm),
        box_size: f32,
    ) {
        let (ox, oy) = origin;
        let half_box = box_size / 2.0;
        let center_x = ox.0 + half_box;
        let center_y = oy.0 - half_box;

        // Draw debug box (same style as hands)
        self.draw_debug_box(layer, ox.0, oy.0, box_size, box_size);

        // Draw HCP values in compass positions
        layer.set_fill_color(Color::Rgb(colors::BLACK));
        let font_size = self.settings.card_font_size - 1.0;

        // Get HCP values
        let north_hcp = deal.north.total_hcp();
        let south_hcp = deal.south.total_hcp();
        let east_hcp = deal.east.total_hcp();
        let west_hcp = deal.west.total_hcp();

        // Use bold measurer for HCP values
        let bold_measurer = text_metrics::get_serif_bold_measurer();

        // N (top center)
        let n_text = format!("{}", north_hcp);
        let n_width = bold_measurer.measure_width_mm(&n_text, font_size);
        layer.use_text(
            &n_text,
            font_size,
            Mm(center_x - n_width / 2.0),
            Mm(center_y + half_box - 5.0),
            self.bold_font,
        );

        // S (bottom center)
        let s_text = format!("{}", south_hcp);
        let s_width = bold_measurer.measure_width_mm(&s_text, font_size);
        layer.use_text(
            &s_text,
            font_size,
            Mm(center_x - s_width / 2.0),
            Mm(center_y - half_box + 2.0),
            self.bold_font,
        );

        // W (left center)
        let w_text = format!("{}", west_hcp);
        layer.use_text(
            &w_text,
            font_size,
            Mm(ox.0 + 2.0),
            Mm(center_y - 1.5),
            self.bold_font,
        );

        // E (right center)
        let e_text = format!("{}", east_hcp);
        let e_width = bold_measurer.measure_width_mm(&e_text, font_size);
        layer.use_text(
            &e_text,
            font_size,
            Mm(ox.0 + box_size - e_width - 2.0),
            Mm(center_y - 1.5),
            self.bold_font,
        );
    }
}
