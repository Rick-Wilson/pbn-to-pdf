//! Declarer's Plan Layout Renderers (1-up, 2-up, 4-up)
//!
//! Generates PDF documents for declarer play practice.
//! Three layout variants:
//! - **1-up**: One deal per page at full size
//! - **2-up**: Two deals per page, each rotated 90° CW for landscape reading
//! - **4-up**: Four deals per page in a 2x2 grid (original layout)

use printpdf::{Color, CurTransMat, Mm, PdfDocument, PdfPage, PdfSaveOptions, Rgb};

use crate::config::Settings;
use crate::error::RenderError;
use crate::model::{BidSuit, Board, Deal, Direction, Hand};

use crate::render::components::DeclarersPlanSmallRenderer;
use crate::render::helpers::card_assets::CardAssets;
use crate::render::helpers::colors::SuitColors;
use crate::render::helpers::compress::compress_pdf;
use crate::render::helpers::fonts::FontManager;
use crate::render::helpers::layer::LayerBuilder;

/// Separator line thickness
const SEPARATOR_THICKNESS: f32 = 2.0;

/// Separator line color (dark gray)
const SEPARATOR_COLOR: Rgb = Rgb {
    r: 0.3,
    g: 0.3,
    b: 0.3,
    icc_profile: None,
};

/// Padding inside each panel
const PANEL_PADDING: f32 = 5.0;

/// mm to PDF points conversion factor
const MM_TO_PT: f32 = 2.834_645_7;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Prepared board data ready for rendering
struct PreparedBoard<'a> {
    dummy_hand: Hand,
    declarer_hand: Hand,
    is_nt: bool,
    opening_lead: Option<crate::model::Card>,
    deal_number: Option<u32>,
    contract_str: Option<String>,
    trump: Option<BidSuit>,
    _board: &'a Board,
}

fn prepare_board(board: &Board) -> PreparedBoard<'_> {
    let is_nt = board
        .contract
        .as_ref()
        .map(|c| c.suit == BidSuit::NoTrump)
        .unwrap_or(false);

    let opening_lead = board
        .play
        .as_ref()
        .and_then(|play| play.tricks.first().and_then(|trick| trick.cards[0]));

    let contract_str = board.contract.as_ref().map(|c| {
        let suit_symbol = match c.suit {
            BidSuit::Clubs => "♣",
            BidSuit::Diamonds => "♦",
            BidSuit::Hearts => "♥",
            BidSuit::Spades => "♠",
            BidSuit::NoTrump => "NT",
        };
        format!("{}{}", c.level, suit_symbol)
    });

    let declarer = board
        .contract
        .as_ref()
        .map(|c| c.declarer)
        .unwrap_or(Direction::South);
    let (dummy_hand, declarer_hand) = rotate_deal_for_declarer(&board.deal, declarer);

    let trump = board.contract.as_ref().map(|c| c.suit);

    PreparedBoard {
        dummy_hand,
        declarer_hand,
        is_nt,
        opening_lead,
        deal_number: board.number,
        contract_str,
        trump,
        _board: board,
    }
}

/// Rotate a deal so that the declarer is always South.
/// Returns (dummy_hand, declarer_hand).
fn rotate_deal_for_declarer(deal: &Deal, declarer: Direction) -> (Hand, Hand) {
    match declarer {
        Direction::South => (deal.north.clone(), deal.south.clone()),
        Direction::North => (deal.south.clone(), deal.north.clone()),
        Direction::East => (deal.west.clone(), deal.east.clone()),
        Direction::West => (deal.east.clone(), deal.west.clone()),
    }
}

/// Baseline card scale (4-up) — layout_scale is relative to this
const BASELINE_CARD_SCALE: f32 = SCALE_4UP;

/// Create the shared component renderer with given card scale
fn make_renderer<'a>(
    card_assets: &'a CardAssets,
    fonts: &'a FontManager,
    settings: &Settings,
    card_scale: f32,
) -> DeclarersPlanSmallRenderer<'a> {
    let colors = SuitColors::new(settings.black_color, settings.red_color);
    let layout_scale = card_scale / BASELINE_CARD_SCALE;
    DeclarersPlanSmallRenderer::new(
        card_assets,
        fonts.serif.regular,
        fonts.serif.bold,
        fonts.symbol_font(),
        colors,
    )
    .card_scale(card_scale)
    .layout_scale(layout_scale)
    .show_bounds(settings.debug_boxes)
}

/// Render a single prepared board into a layer
fn render_prepared(
    renderer: &DeclarersPlanSmallRenderer<'_>,
    layer: &mut LayerBuilder,
    board: &PreparedBoard<'_>,
    origin: (Mm, Mm),
) {
    renderer.render_with_info(
        layer,
        &board.dummy_hand,
        &board.declarer_hand,
        board.is_nt,
        board.opening_lead,
        board.deal_number,
        board.contract_str.as_deref(),
        board.trump,
        origin,
    );
}

/// Draw a horizontal separator line across the content area
fn draw_horizontal_separator(layer: &mut LayerBuilder, settings: &Settings, y: f32) {
    layer.set_outline_color(Color::Rgb(SEPARATOR_COLOR));
    layer.set_outline_thickness(SEPARATOR_THICKNESS);
    layer.add_line(
        Mm(settings.margin_left),
        Mm(y),
        Mm(settings.page_width - settings.margin_right),
        Mm(y),
    );
}

/// Generate the final PDF bytes from a document
fn finalize_pdf(doc: PdfDocument, pages: Vec<PdfPage>) -> Result<Vec<u8>, RenderError> {
    let mut doc = doc;
    doc.with_pages(pages);
    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    let compressed = compress_pdf(bytes.clone()).unwrap_or(bytes);
    Ok(compressed)
}

// ---------------------------------------------------------------------------
// 1-Up Renderer
// ---------------------------------------------------------------------------

/// Card scale for 1-up layout (one deal fills the page)
const SCALE_1UP: f32 = 0.55;

/// Declarer's plan 1-up renderer — one deal per page
pub struct DeclarersPlan1UpRenderer {
    settings: Settings,
}

impl DeclarersPlan1UpRenderer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    pub fn render(&self, boards: &[Board]) -> Result<Vec<u8>, RenderError> {
        let title = boards
            .first()
            .and_then(|b| b.event.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Declarer's Plan");

        let mut doc = PdfDocument::new(title);
        let fonts = FontManager::new(&mut doc)?;
        let card_assets =
            CardAssets::load(&mut doc).map_err(|e| RenderError::CardAsset(e.to_string()))?;

        let renderer = make_renderer(&card_assets, &fonts, &self.settings, SCALE_1UP);

        let mut pages = Vec::new();

        for board in boards {
            let prep = prepare_board(board);
            let mut layer = LayerBuilder::new();

            // Center the panel on the page
            let content_width =
                self.settings.page_width - self.settings.margin_left - self.settings.margin_right;
            let content_height =
                self.settings.page_height - self.settings.margin_top - self.settings.margin_bottom;

            let (panel_w, panel_h) =
                renderer.dimensions(&prep.dummy_hand, &prep.declarer_hand, prep.is_nt);

            let origin_x = self.settings.margin_left + (content_width - panel_w) / 2.0;
            let origin_y = self.settings.page_height
                - self.settings.margin_top
                - (content_height - panel_h) / 2.0;

            render_prepared(&renderer, &mut layer, &prep, (Mm(origin_x), Mm(origin_y)));

            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));
        }

        finalize_pdf(doc, pages)
    }
}

// ---------------------------------------------------------------------------
// 2-Up Renderer (rotated 90° CW)
// ---------------------------------------------------------------------------

/// Card scale for 2-up layout
const SCALE_2UP: f32 = 0.45;

/// Declarer's plan 2-up renderer — two deals per portrait page, each rotated 90° CW
///
/// Each half of the page contains one panel. The panel content is rendered in
/// portrait orientation then rotated 90° clockwise, so the reader turns the
/// page 90° counter-clockwise to read. This maximises the available vertical
/// space for the tall declarer's plan layout.
pub struct DeclarersPlan2UpRenderer {
    settings: Settings,
}

impl DeclarersPlan2UpRenderer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    pub fn render(&self, boards: &[Board]) -> Result<Vec<u8>, RenderError> {
        let title = boards
            .first()
            .and_then(|b| b.event.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Declarer's Plan");

        let mut doc = PdfDocument::new(title);
        let fonts = FontManager::new(&mut doc)?;
        let card_assets =
            CardAssets::load(&mut doc).map_err(|e| RenderError::CardAsset(e.to_string()))?;

        let renderer = make_renderer(&card_assets, &fonts, &self.settings, SCALE_2UP);

        let content_width =
            self.settings.page_width - self.settings.margin_left - self.settings.margin_right;
        let content_height =
            self.settings.page_height - self.settings.margin_top - self.settings.margin_bottom;
        let half_height = content_height / 2.0;
        let center_y = self.settings.margin_bottom + half_height;

        // Slot centers (in page coordinates)
        // Offset each slot away from the center divider for better visual balance
        let center_inset = PANEL_PADDING * 2.0;
        let slot_cx = self.settings.margin_left + content_width / 2.0;
        let top_slot_cy = center_y + half_height / 2.0 + center_inset / 2.0;
        let bottom_slot_cy = center_y - half_height / 2.0 - center_inset / 2.0;

        let mut pages = Vec::new();

        for chunk in boards.chunks(2) {
            let mut layer = LayerBuilder::new();

            // Draw horizontal separator between panels
            draw_horizontal_separator(&mut layer, &self.settings, center_y);

            let slot_centers = [(slot_cx, top_slot_cy), (slot_cx, bottom_slot_cy)];

            for (i, board) in chunk.iter().enumerate() {
                let prep = prepare_board(board);
                let (dest_cx, dest_cy) = slot_centers[i];

                // Virtual canvas: panel is rendered upright, then rotated 90° CW.
                // After rotation, the panel's width maps to slot height and vice versa.
                // Virtual canvas dimensions = (slot_height, slot_width) so that after
                // rotation the panel fits within (slot_width, slot_height).
                let canvas_w = half_height - PANEL_PADDING * 2.0;
                let canvas_h = content_width - PANEL_PADDING * 2.0;

                let (panel_w, panel_h) =
                    renderer.dimensions(&prep.dummy_hand, &prep.declarer_hand, prep.is_nt);

                // Panel origin (top-left) centered in virtual canvas
                let panel_ox = (canvas_w - panel_w) / 2.0;
                let panel_oy = canvas_h - (canvas_h - panel_h) / 2.0;

                // Canvas center in virtual coords
                let cx = canvas_w / 2.0;
                let cy = canvas_h / 2.0;

                // 90° CW rotation around canvas center, then translate to slot center.
                // CW rotation matrix: [0, -1, 1, 0, 0, 0]
                // maps (x, y) → (y, -x)
                //
                // Combined: rotate around (cx,cy) then translate to (dest_cx, dest_cy):
                //   e = dest_cx - cy     (since c=1:  c*(-cy) + e = dest_cx - cx... let me derive)
                //
                // Full derivation:
                //   (x,y) → rotate CW around (cx,cy) → translate to (dx,dy)
                //   Step 1: x' = x - cx, y' = y - cy
                //   Step 2 (CW): x'' = y', y'' = -x'  →  x'' = y-cy, y'' = -(x-cx) = cx-x
                //   Step 3: x''' = x'' + dx, y''' = y'' + dy  →  x''' = y-cy+dx, y''' = cx-x+dy
                //
                // Matrix form [a,b,c,d,e,f] where x'=ax+cy+e, y'=bx+dy+f:
                //   a=0, b=-1, c=1, d=0, e=dx-cy, f=dy+cx
                let e_mm = dest_cx - cy;
                let f_mm = dest_cy + cx;
                let e_pt = e_mm * MM_TO_PT;
                let f_pt = f_mm * MM_TO_PT;

                layer.save_graphics_state();
                layer.set_transform(CurTransMat::Raw([0.0, -1.0, 1.0, 0.0, e_pt, f_pt]));
                render_prepared(&renderer, &mut layer, &prep, (Mm(panel_ox), Mm(panel_oy)));
                layer.restore_graphics_state();
            }

            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));
        }

        finalize_pdf(doc, pages)
    }
}

// ---------------------------------------------------------------------------
// 4-Up Renderer (original)
// ---------------------------------------------------------------------------

/// Card scale for 4-up layout
const SCALE_4UP: f32 = 0.35;

/// Declarer's plan 4-up renderer — four deals per page in a 2x2 grid
pub struct DeclarersPlanRenderer {
    settings: Settings,
}

impl DeclarersPlanRenderer {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Generate a PDF with declarer's plan practice sheets (4 per page)
    pub fn render(&self, boards: &[Board]) -> Result<Vec<u8>, RenderError> {
        let title = boards
            .first()
            .and_then(|b| b.event.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Declarer's Plan Practice");

        let mut doc = PdfDocument::new(title);
        let fonts = FontManager::new(&mut doc)?;
        let card_assets =
            CardAssets::load(&mut doc).map_err(|e| RenderError::CardAsset(e.to_string()))?;

        let mut pages = Vec::new();

        for chunk in boards.chunks(4) {
            let mut layer = LayerBuilder::new();
            self.render_page(&mut layer, chunk, &fonts, &card_assets);
            pages.push(PdfPage::new(
                Mm(self.settings.page_width),
                Mm(self.settings.page_height),
                layer.into_ops(),
            ));
        }

        finalize_pdf(doc, pages)
    }

    /// Render a single page with up to 4 deals
    fn render_page(
        &self,
        layer: &mut LayerBuilder,
        boards: &[Board],
        fonts: &FontManager,
        card_assets: &CardAssets,
    ) {
        let renderer = make_renderer(card_assets, fonts, &self.settings, SCALE_4UP);

        let margin_left = self.settings.margin_left;
        let margin_right = self.settings.margin_right;
        let margin_top = self.settings.margin_top;
        let margin_bottom = self.settings.margin_bottom;
        let page_width = self.settings.page_width;
        let page_height = self.settings.page_height;

        let content_width = page_width - margin_left - margin_right;
        let content_height = page_height - margin_top - margin_bottom;

        let half_width = content_width / 2.0;
        let half_height = content_height / 2.0;

        let center_x = margin_left + half_width;
        let center_y = margin_bottom + half_height;

        // Draw separator lines
        self.draw_separator_lines(layer, center_x, center_y);

        // Origins for each quadrant (top-left corner of each, with padding)
        let positions = [
            (margin_left + PANEL_PADDING, page_height - margin_top), // Top-left
            (center_x + PANEL_PADDING, page_height - margin_top),    // Top-right
            (margin_left + PANEL_PADDING, center_y),                 // Bottom-left
            (center_x + PANEL_PADDING, center_y),                    // Bottom-right
        ];

        for (i, board) in boards.iter().enumerate() {
            if i >= 4 {
                break;
            }

            let (x, y) = positions[i];
            let prep = prepare_board(board);
            render_prepared(&renderer, layer, &prep, (Mm(x), Mm(y)));
        }
    }

    /// Draw horizontal and vertical separator lines between quadrants
    fn draw_separator_lines(&self, layer: &mut LayerBuilder, center_x: f32, center_y: f32) {
        let margin_left = self.settings.margin_left;
        let margin_right = self.settings.margin_right;
        let margin_top = self.settings.margin_top;
        let margin_bottom = self.settings.margin_bottom;
        let page_width = self.settings.page_width;
        let page_height = self.settings.page_height;

        layer.set_outline_color(Color::Rgb(SEPARATOR_COLOR));
        layer.set_outline_thickness(SEPARATOR_THICKNESS);

        // Vertical line
        layer.add_line(
            Mm(center_x),
            Mm(margin_bottom),
            Mm(center_x),
            Mm(page_height - margin_top),
        );

        // Horizontal line
        layer.add_line(
            Mm(margin_left),
            Mm(center_y),
            Mm(page_width - margin_right),
            Mm(center_y),
        );
    }
}
