//! Debug tool to visualize layout positioning
//! Run with: cargo run --bin layout_debug

use pbn_to_pdf::config::Settings;
use pbn_to_pdf::render::get_measurer;
use printpdf::path::PaintMode;
use printpdf::{Color, Mm, PdfDocument, Rect, Rgb};
use std::fs::File;
use std::io::BufWriter;

// Font constants for embedded fonts
const DEJAVU_SANS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

fn main() {
    let settings = Settings::default();

    // Get actual font metrics
    let measurer = get_measurer();
    let font_size = settings.card_font_size; // 11pt
    let cap_height = measurer.cap_height_mm(font_size);

    // Create PDF
    let (doc, page_idx, layer_idx) = PdfDocument::new(
        "Layout Debug",
        Mm(settings.page_width),
        Mm(settings.page_height),
        "Layer 1",
    );

    // Load font
    let font = doc.add_external_font(DEJAVU_SANS).unwrap();

    let layer = doc.get_page(page_idx).get_layer(layer_idx);

    // Layout constants (same as hand_diagram.rs)
    let margin = settings.margin;
    let page_top = settings.page_height - margin;
    let hand_w = settings.hand_width;
    let hand_h = settings.hand_height;
    let line_height = settings.line_height;
    let compass_size = 18.0;

    let ox = margin;
    let oy = page_top;

    // Colors for debug boxes
    let red = Rgb::new(1.0, 0.0, 0.0, None);
    let blue = Rgb::new(0.0, 0.0, 1.0, None);
    let green = Rgb::new(0.0, 0.8, 0.0, None);
    let orange = Rgb::new(1.0, 0.5, 0.0, None);
    let purple = Rgb::new(0.5, 0.0, 0.5, None);
    let cyan = Rgb::new(0.0, 0.8, 0.8, None);
    let gray = Rgb::new(0.5, 0.5, 0.5, None);
    let black = Rgb::new(0.0, 0.0, 0.0, None);

    // Calculate positions (same logic as hand_diagram.rs)
    let north_x = ox + hand_w + (compass_size - hand_w) / 2.0;
    let north_y = oy;

    let row2_y = north_y - hand_h; // No extra gap

    let west_x = ox;

    let compass_center_x = north_x + 2.5;
    let compass_y = row2_y - hand_h / 2.0; // Center vertically with West/East

    let east_x = compass_center_x + compass_size / 2.0 + 2.0;

    let hcp_row_y = row2_y - hand_h;

    let south_y = hcp_row_y - line_height;

    // Print positions
    println!("=== Layout Debug (with actual font metrics) ===");
    println!(
        "Page: {}x{} mm, margin: {} mm",
        settings.page_width, settings.page_height, margin
    );
    println!("Hand dimensions: {}x{} mm", hand_w, hand_h);
    println!("Line height: {} mm", line_height);
    println!("Font size: {} pt", font_size);
    println!("  Cap height: {:.2} mm (from font metrics)", cap_height);
    println!("  Ascender: {:.2} mm", measurer.ascender_mm(font_size));
    println!("  Descender: {:.2} mm", measurer.descender_mm(font_size));
    println!("Compass size: {} mm", compass_size);
    println!();
    println!("Origin (ox, oy): ({}, {})", ox, oy);
    println!();
    println!(
        "North hand: top-left=({:.1}, {:.1}), bottom-right=({:.1}, {:.1})",
        north_x,
        north_y,
        north_x + hand_w,
        north_y - hand_h
    );
    let first_baseline = north_y - cap_height;
    println!("  First baseline (top - cap_height): {:.1}", first_baseline);
    println!(
        "  Text baselines at Y: {:.1}, {:.1}, {:.1}, {:.1}",
        first_baseline,
        first_baseline - line_height,
        first_baseline - 2.0 * line_height,
        first_baseline - 3.0 * line_height
    );
    println!();
    println!("Row 2 Y (West/East top): {:.1}", row2_y);
    println!(
        "  Gap from North bottom to Row2: {:.1}",
        (north_y - hand_h) - row2_y
    );
    println!();
    println!(
        "West hand: top-left=({:.1}, {:.1}), bottom-right=({:.1}, {:.1})",
        west_x,
        row2_y,
        west_x + hand_w,
        row2_y - hand_h
    );
    println!();
    println!(
        "Compass: center=({:.1}, {:.1}), size={}",
        compass_center_x, compass_y, compass_size
    );
    println!(
        "  Top edge: {:.1}, Bottom edge: {:.1}",
        compass_y + compass_size / 2.0,
        compass_y - compass_size / 2.0
    );
    println!();
    println!(
        "East hand: top-left=({:.1}, {:.1}), bottom-right=({:.1}, {:.1})",
        east_x,
        row2_y,
        east_x + hand_w,
        row2_y - hand_h
    );
    println!();
    println!("HCP row Y: {:.1}", hcp_row_y);
    println!(
        "  Gap from West/East bottom to HCP: {:.1}",
        (row2_y - hand_h) - hcp_row_y
    );
    println!();
    println!(
        "South hand: top-left=({:.1}, {:.1}), bottom-right=({:.1}, {:.1})",
        north_x,
        south_y,
        north_x + hand_w,
        south_y - hand_h
    );
    println!("  Gap from HCP to South: {:.1}", hcp_row_y - south_y);

    // Draw debug rectangles (bounding boxes as calculated)

    // North hand box (red)
    draw_rect(&layer, north_x, north_y, hand_w, hand_h, &red, "North");

    // West hand box (blue)
    draw_rect(&layer, west_x, row2_y, hand_w, hand_h, &blue, "West");

    // Compass box (green)
    let compass_left = compass_center_x - compass_size / 2.0;
    let compass_top = compass_y + compass_size / 2.0;
    draw_rect(
        &layer,
        compass_left,
        compass_top,
        compass_size,
        compass_size,
        &green,
        "Compass",
    );

    // East hand box (orange)
    draw_rect(&layer, east_x, row2_y, hand_w, hand_h, &orange, "East");

    // HCP row (purple line)
    layer.set_outline_color(Color::Rgb(purple.clone()));
    layer.set_outline_thickness(0.5);
    let hcp_line = printpdf::Line {
        points: vec![
            (printpdf::Point::new(Mm(ox), Mm(hcp_row_y)), false),
            (
                printpdf::Point::new(Mm(ox + hand_w * 2.0 + compass_size), Mm(hcp_row_y)),
                false,
            ),
        ],
        is_closed: false,
    };
    layer.add_line(hcp_line);

    // South hand box (cyan)
    draw_rect(&layer, north_x, south_y, hand_w, hand_h, &cyan, "South");

    // Now render ACTUAL text at the same positions to see where it really goes
    // Using cap_height offset to align text tops with bounding box tops
    layer.set_fill_color(Color::Rgb(black.clone()));

    // North hand - render actual text
    let suits = ["♠ AKQ", "♥ JT9", "♦ 876", "♣ 5432"];
    let north_first_baseline = north_y - cap_height;
    for (i, suit_text) in suits.iter().enumerate() {
        let y = north_first_baseline - (i as f32 * line_height);
        layer.use_text(*suit_text, font_size, Mm(north_x), Mm(y), &font);

        // Draw a small marker at the baseline position
        draw_baseline_marker(&layer, north_x - 2.0, y, &gray);
    }

    // West hand
    let west_first_baseline = row2_y - cap_height;
    for (i, suit_text) in suits.iter().enumerate() {
        let y = west_first_baseline - (i as f32 * line_height);
        layer.use_text(*suit_text, font_size, Mm(west_x), Mm(y), &font);
        draw_baseline_marker(&layer, west_x - 2.0, y, &gray);
    }

    // East hand
    let east_first_baseline = row2_y - cap_height;
    for (i, suit_text) in suits.iter().enumerate() {
        let y = east_first_baseline - (i as f32 * line_height);
        layer.use_text(*suit_text, font_size, Mm(east_x), Mm(y), &font);
        draw_baseline_marker(&layer, east_x - 2.0, y, &gray);
    }

    // South hand
    let south_first_baseline = south_y - cap_height;
    for (i, suit_text) in suits.iter().enumerate() {
        let y = south_first_baseline - (i as f32 * line_height);
        layer.use_text(*suit_text, font_size, Mm(north_x), Mm(y), &font);
        draw_baseline_marker(&layer, north_x - 2.0, y, &gray);
    }

    // Save
    let file = File::create("/tmp/layout_debug.pdf").unwrap();
    doc.save(&mut BufWriter::new(file)).unwrap();
    println!();
    println!("Saved to /tmp/layout_debug.pdf");
}

fn draw_rect(
    layer: &printpdf::PdfLayerReference,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: &Rgb,
    _label: &str,
) {
    // Draw outline rectangle (y is top, so bottom = y - h)
    let rect = Rect::new(Mm(x), Mm(y - h), Mm(x + w), Mm(y)).with_mode(PaintMode::Stroke);

    layer.set_outline_color(Color::Rgb(color.clone()));
    layer.set_outline_thickness(0.5);
    layer.add_rect(rect);
}

fn draw_baseline_marker(layer: &printpdf::PdfLayerReference, x: f32, y: f32, color: &Rgb) {
    // Draw a small horizontal line to mark the baseline
    layer.set_outline_color(Color::Rgb(color.clone()));
    layer.set_outline_thickness(0.3);
    let line = printpdf::Line {
        points: vec![
            (printpdf::Point::new(Mm(x), Mm(y)), false),
            (printpdf::Point::new(Mm(x + 1.5), Mm(y)), false),
        ],
        is_closed: false,
    };
    layer.add_line(line);
}
