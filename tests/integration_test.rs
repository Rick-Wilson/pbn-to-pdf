use std::fs;
use std::path::PathBuf;
use std::process::Command;

use pbn_to_pdf::config::Settings;
use pbn_to_pdf::model::{Card, Hand, Holding, Rank, Suit};
use pbn_to_pdf::parser::parse_pbn;
use pbn_to_pdf::render::components::{
    DeclarersPlanSmallRenderer, DummyRenderer, FanRenderer, LosersTableRenderer,
    WinnersTableRenderer,
};
use pbn_to_pdf::render::generate_pdf;
use pbn_to_pdf::render::helpers::{colors::SuitColors, FontManager};
use pbn_to_pdf::render::helpers::{CardAssets, LayerBuilder};
use printpdf::{Mm, PdfDocument, PdfPage, PdfSaveOptions, PdfWarnMsg};

fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn test_parse_abs2_practice_deals() {
    let pbn_path = fixtures_path().join("ABS2-2 Promotion and Length practice deals.pbn");
    let content = fs::read_to_string(&pbn_path).expect("Failed to read PBN file");

    let pbn_file = parse_pbn(&content).expect("Failed to parse PBN");

    // Should have 4 boards
    assert_eq!(pbn_file.boards.len(), 4);

    // Check first board
    let board1 = &pbn_file.boards[0];
    assert_eq!(board1.number, Some(1));
    assert!(board1.dealer.is_some());
    assert!(board1.auction.is_some());

    // Check that all boards have deals
    for board in &pbn_file.boards {
        assert!(board.deal.north.card_count() > 0);
        assert!(board.deal.south.card_count() > 0);
        assert!(board.deal.east.card_count() > 0);
        assert!(board.deal.west.card_count() > 0);
    }
}

#[test]
fn test_generate_pdf_from_abs2() {
    let pbn_path = fixtures_path().join("ABS2-2 Promotion and Length practice deals.pbn");
    let content = fs::read_to_string(&pbn_path).expect("Failed to read PBN file");

    let pbn_file = parse_pbn(&content).expect("Failed to parse PBN");
    let settings = Settings::default().with_metadata(&pbn_file.metadata);

    let pdf_bytes = generate_pdf(&pbn_file.boards, &settings).expect("Failed to generate PDF");

    // PDF should be non-empty
    assert!(!pdf_bytes.is_empty());

    // PDF should start with %PDF header
    assert!(pdf_bytes.starts_with(b"%PDF"));

    // Should be a reasonable size (at least 10KB for 4 boards)
    assert!(pdf_bytes.len() > 10_000);
}

#[test]
fn test_board_metadata_extraction() {
    let pbn_path = fixtures_path().join("ABS2-2 Promotion and Length practice deals.pbn");
    let content = fs::read_to_string(&pbn_path).expect("Failed to read PBN file");

    let pbn_file = parse_pbn(&content).expect("Failed to parse PBN");

    // Check metadata was extracted
    assert!(pbn_file.metadata.version.is_some());
    assert!(pbn_file.metadata.creator.is_some());

    // Check layout settings
    assert!(pbn_file.metadata.layout.boards_per_page.is_some());
}

#[test]
fn test_commentary_parsing() {
    let pbn_path = fixtures_path().join("ABS2-2 Promotion and Length practice deals.pbn");
    let content = fs::read_to_string(&pbn_path).expect("Failed to read PBN file");

    let pbn_file = parse_pbn(&content).expect("Failed to parse PBN");

    // At least some boards should have commentary
    let boards_with_commentary = pbn_file
        .boards
        .iter()
        .filter(|b| !b.commentary.is_empty())
        .count();

    assert!(
        boards_with_commentary > 0,
        "Expected some boards to have commentary"
    );
}

#[test]
fn test_auction_parsing() {
    let pbn_path = fixtures_path().join("ABS2-2 Promotion and Length practice deals.pbn");
    let content = fs::read_to_string(&pbn_path).expect("Failed to read PBN file");

    let pbn_file = parse_pbn(&content).expect("Failed to parse PBN");

    for board in &pbn_file.boards {
        if let Some(ref auction) = board.auction {
            // Auctions should have at least some calls
            assert!(
                !auction.calls.is_empty(),
                "Board {} has empty auction",
                board.number.unwrap_or(0)
            );

            // Should be able to determine final contract
            let contract = auction.final_contract();
            assert!(
                contract.is_some() || auction.is_passed_out,
                "Board {} should have contract or be passed out",
                board.number.unwrap_or(0)
            );
        }
    }
}

fn output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/output")
}

#[test]
fn test_generate_all_pdfs() {
    // Ensure output directory exists
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Get all PBN files in fixtures
    let fixtures = fixtures_path();
    let pbn_files: Vec<_> = fs::read_dir(&fixtures)
        .expect("Failed to read fixtures directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "pbn")
                .unwrap_or(false)
        })
        .collect();

    assert!(!pbn_files.is_empty(), "No PBN files found in fixtures");

    // Build the binary first
    let build_status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to build project");
    assert!(build_status.success(), "Failed to build project");

    let binary = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/pbn-to-pdf");

    for entry in pbn_files {
        let pbn_path = entry.path();
        let stem = pbn_path.file_stem().unwrap().to_string_lossy();

        // Generate analysis PDF (default layout)
        let analysis_output = output_dir.join(format!("{}.pdf", stem));
        let status = Command::new(&binary)
            .args([
                "--layout",
                "analysis",
                pbn_path.to_str().unwrap(),
                "-o",
                analysis_output.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to run pbn-to-pdf for analysis");
        assert!(
            status.success(),
            "Failed to generate analysis PDF for {}",
            stem
        );
        assert!(
            analysis_output.exists(),
            "Analysis PDF not created for {}",
            stem
        );

        // Generate bidding sheets PDF
        let bidding_output = output_dir.join(format!("{} - Bidding Sheets.pdf", stem));
        let status = Command::new(&binary)
            .args([
                "--layout",
                "bidding-sheets",
                pbn_path.to_str().unwrap(),
                "-o",
                bidding_output.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to run pbn-to-pdf for bidding-sheets");
        assert!(
            status.success(),
            "Failed to generate bidding sheets PDF for {}",
            stem
        );
        assert!(
            bidding_output.exists(),
            "Bidding sheets PDF not created for {}",
            stem
        );

        // Generate declarer's plan PDF
        let declarers_output = output_dir.join(format!("{} - Declarers Plan.pdf", stem));
        let status = Command::new(&binary)
            .args([
                "--layout",
                "declarers-plan",
                pbn_path.to_str().unwrap(),
                "-o",
                declarers_output.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to run pbn-to-pdf for declarers-plan");
        assert!(
            status.success(),
            "Failed to generate declarer's plan PDF for {}",
            stem
        );
        assert!(
            declarers_output.exists(),
            "Declarer's plan PDF not created for {}",
            stem
        );

        println!("Generated PDFs for: {}", stem);
    }
}

/// Create a test hand with known cards
fn create_test_hand() -> Hand {
    // AKQ of spades, KQJ2 of hearts, A of diamonds, QJT98 of clubs
    Hand::from_holdings(
        Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]),
        Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack, Rank::Two]),
        Holding::from_ranks([Rank::Ace]),
        Holding::from_ranks([Rank::Queen, Rank::Jack, Rank::Ten, Rank::Nine, Rank::Eight]),
    )
}

/// Create a hand with void suits for testing edge cases
fn create_hand_with_voids() -> Hand {
    // 7 spades, 0 hearts, 3 diamonds, 3 clubs
    Hand::from_holdings(
        Holding::from_ranks([
            Rank::Ace,
            Rank::King,
            Rank::Queen,
            Rank::Jack,
            Rank::Ten,
            Rank::Nine,
            Rank::Eight,
        ]),
        Holding::new(), // void in hearts
        Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]),
        Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]),
    )
}

#[test]
fn test_dummy_renderer_generates_pdf() {
    // Create output directory
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create test hand
    let hand = create_test_hand();

    // Create PDF document
    let mut doc = PdfDocument::new("Dummy Renderer Test");

    // Load card assets
    let card_assets = CardAssets::load(&mut doc).expect("Failed to load card assets");

    // Create renderer and layer - spades first with alternating colors, 20% overlap
    let renderer = DummyRenderer::with_overlap(&card_assets, 0.5, 0.20).first_suit(Suit::Spades);
    let mut layer = LayerBuilder::new();

    // Render the hand
    let height = renderer.render(&mut layer, &hand, (Mm(50.0), Mm(250.0)));

    // Create page with the rendered content
    let page = PdfPage::new(Mm(210.0), Mm(297.0), layer.into_ops());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(pdf_bytes.len() > 1000, "PDF should have reasonable size");
    assert!(height > 0.0, "Rendered height should be positive");

    // Write to output for visual verification
    let output_path = output_dir.join("dummy_test.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
    println!("Dummy renderer test PDF written to: {:?}", output_path);
}

#[test]
fn test_fan_renderer_generates_pdf() {
    // Create output directory
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create test hand
    let hand = create_test_hand();

    // Create PDF document
    let mut doc = PdfDocument::new("Fan Renderer Test");

    // Load card assets
    let card_assets = CardAssets::load(&mut doc).expect("Failed to load card assets");

    // Create renderer and layer - with 45 degree arc for natural hand appearance
    let renderer = FanRenderer::new(&card_assets, 0.4).arc(45.0);
    let mut layer = LayerBuilder::new();

    // Render the hand
    let width = renderer.render(&mut layer, &hand, (Mm(20.0), Mm(180.0)));

    // Create page with the rendered content
    let page = PdfPage::new(Mm(297.0), Mm(210.0), layer.into_ops()); // Landscape for fan
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(pdf_bytes.len() > 1000, "PDF should have reasonable size");
    assert!(width > 0.0, "Rendered width should be positive");

    // Write to output for visual verification
    let output_path = output_dir.join("fan_test.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
    println!("Fan renderer test PDF written to: {:?}", output_path);
}

#[test]
fn test_dummy_with_void_suits() {
    // Create test hand with voids
    let hand = create_hand_with_voids();

    // Create PDF document
    let mut doc = PdfDocument::new("Dummy Void Test");

    // Load card assets
    let card_assets = CardAssets::load(&mut doc).expect("Failed to load card assets");

    // Create renderer and layer - clubs first with alternating colors
    let renderer = DummyRenderer::new(&card_assets, 0.5).first_suit(Suit::Clubs);
    let mut layer = LayerBuilder::new();

    // Render the hand (should handle void suit gracefully)
    let height = renderer.render(&mut layer, &hand, (Mm(50.0), Mm(250.0)));

    // Create page with the rendered content
    let page = PdfPage::new(Mm(210.0), Mm(297.0), layer.into_ops());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(
        height > 0.0,
        "Rendered height should be positive even with voids"
    );

    // Write to output for visual verification
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");
    let output_path = output_dir.join("dummy_void_test.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
}

#[test]
fn test_card_renderer_dimensions() {
    // Create PDF document just to load assets
    let mut doc = PdfDocument::new("Dimensions Test");
    let card_assets = CardAssets::load(&mut doc).expect("Failed to load card assets");

    let hand = create_test_hand();

    // Test DummyRenderer dimensions
    let dummy_renderer = DummyRenderer::new(&card_assets, 0.5);
    let (dummy_width, dummy_height) = dummy_renderer.dimensions(&hand);
    assert!(dummy_width > 0.0, "Dummy width should be positive");
    assert!(dummy_height > 0.0, "Dummy height should be positive");

    // Test FanRenderer dimensions
    let fan_renderer = FanRenderer::new(&card_assets, 0.4);
    let (fan_width, fan_height) = fan_renderer.dimensions(&hand);
    assert!(fan_width > 0.0, "Fan width should be positive");
    assert!(fan_height > 0.0, "Fan height should be positive");

    // Fan should be wider than tall, dummy should be taller than wide
    assert!(
        fan_width > fan_height,
        "Fan layout should be wider than tall"
    );
    // Note: dummy may or may not be taller than wide depending on hand shape
}

/// Create a full deal (4 hands with all 52 cards distributed)
fn create_full_deal() -> (Hand, Hand, Hand, Hand) {
    // North: A-K of each suit (8 cards) + Q-J-T of spades (3 cards) + 9-8 of hearts (2 cards) = 13 cards
    let north = Hand::from_holdings(
        Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen, Rank::Jack, Rank::Ten]),
        Holding::from_ranks([Rank::Ace, Rank::King, Rank::Nine, Rank::Eight]),
        Holding::from_ranks([Rank::Ace, Rank::King]),
        Holding::from_ranks([Rank::Ace, Rank::King]),
    );

    // East: 9-8-7 of spades, Q-J-T of hearts, Q-J-T-9-8 of diamonds, Q-J of clubs = 13 cards
    let east = Hand::from_holdings(
        Holding::from_ranks([Rank::Nine, Rank::Eight, Rank::Seven]),
        Holding::from_ranks([Rank::Queen, Rank::Jack, Rank::Ten]),
        Holding::from_ranks([Rank::Queen, Rank::Jack, Rank::Ten, Rank::Nine, Rank::Eight]),
        Holding::from_ranks([Rank::Queen, Rank::Jack]),
    );

    // South: 6-5-4 of spades, 7-6-5-4 of hearts, 7-6 of diamonds, T-9-8-7 of clubs = 13 cards
    let south = Hand::from_holdings(
        Holding::from_ranks([Rank::Six, Rank::Five, Rank::Four]),
        Holding::from_ranks([Rank::Seven, Rank::Six, Rank::Five, Rank::Four]),
        Holding::from_ranks([Rank::Seven, Rank::Six]),
        Holding::from_ranks([Rank::Ten, Rank::Nine, Rank::Eight, Rank::Seven]),
    );

    // West: 3-2 of spades, 3-2 of hearts, 5-4-3-2 of diamonds, 6-5-4-3-2 of clubs = 13 cards
    let west = Hand::from_holdings(
        Holding::from_ranks([Rank::Three, Rank::Two]),
        Holding::from_ranks([Rank::Three, Rank::Two]),
        Holding::from_ranks([Rank::Five, Rank::Four, Rank::Three, Rank::Two]),
        Holding::from_ranks([Rank::Six, Rank::Five, Rank::Four, Rank::Three, Rank::Two]),
    );

    (north, east, south, west)
}

#[test]
fn test_full_deck_compass_layout() {
    // Create output directory
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create the four hands
    let (north, east, south, west) = create_full_deal();

    // Verify we have exactly 52 cards
    let total_cards =
        north.card_count() + east.card_count() + south.card_count() + west.card_count();
    assert_eq!(total_cards, 52, "Should have exactly 52 cards in the deal");

    // Create PDF document (landscape A4 for better layout)
    let mut doc = PdfDocument::new("Full Deck Compass Layout");

    // Load card assets
    let card_assets = CardAssets::load(&mut doc).expect("Failed to load card assets");

    // Create layer
    let mut layer = LayerBuilder::new();

    // Page dimensions (landscape A4)
    let page_width = 297.0;
    let page_height = 210.0;
    let center_x = page_width / 2.0;
    let center_y = page_height / 2.0;

    let dummy_scale = 0.35;
    let fan_scale = 0.42; // 20% larger than dummy
    let arc = 30.0;

    // Card dimensions for positioning calculations (at fan scale)
    let card_width = 58.94 * fan_scale; // CARD_WIDTH_MM * scale
    let card_height = 85.61 * fan_scale; // CARD_HEIGHT_MM * scale

    // Suit symbol width is roughly 1/6 of card width
    let suit_symbol_width = card_width / 6.0;

    // North: Dummy style at top center
    let dummy_renderer = DummyRenderer::with_overlap(&card_assets, dummy_scale, 0.20)
        .first_suit(Suit::Spades)
        .show_bounds(true);
    let (dummy_width, _) = dummy_renderer.dimensions(&north);
    let north_x = center_x - dummy_width / 2.0;
    let north_y = page_height - 15.0; // Near top
    dummy_renderer.render(&mut layer, &north, (Mm(north_x), Mm(north_y)));

    // South: Fan style at bottom center (facing up, like holding cards)
    // Scale the south fan to match the dummy width
    let temp_south_renderer = FanRenderer::new(&card_assets, 1.0).arc(arc);
    let (temp_south_width, _) = temp_south_renderer.dimensions(&south);
    let south_scale = dummy_width / temp_south_width;
    let south_renderer = FanRenderer::new(&card_assets, south_scale)
        .arc(arc)
        .show_bounds(true);
    let (south_width, south_height) = south_renderer.dimensions(&south);
    let south_x = center_x - south_width / 2.0;
    let south_y = 15.0 + south_height + 4.0 * suit_symbol_width; // Move up by 4 suit symbol widths
    south_renderer.render(&mut layer, &south, (Mm(south_x), Mm(south_y)));

    // West: Fan style rotated 90° clockwise (-90° CCW), on the left
    // Origin is now the CENTER of the rotated fan
    let west_renderer = FanRenderer::new(&card_assets, fan_scale)
        .arc(arc)
        .rotation(-90.0)
        .show_bounds(true);
    let west_x = 10.0 + 2.0 * card_height;
    let west_y = center_y + 2.0 * suit_symbol_width; // Move up by 2 suit symbol widths
    west_renderer.render(&mut layer, &west, (Mm(west_x), Mm(west_y)));

    // East: Fan style rotated 90° counter-clockwise (90° CCW), on the right
    // Origin is now the CENTER of the rotated fan - same Y as West for alignment
    let east_renderer = FanRenderer::new(&card_assets, fan_scale)
        .arc(arc)
        .rotation(90.0)
        .show_bounds(true);
    let east_x = page_width - 10.0 - 2.0 * card_height;
    let east_y = center_y + 2.0 * suit_symbol_width; // Move up by 2 suit symbol widths
    east_renderer.render(&mut layer, &east, (Mm(east_x), Mm(east_y)));

    // Create page with the rendered content (landscape A4)
    let page = PdfPage::new(Mm(page_width), Mm(page_height), layer.into_ops());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(pdf_bytes.len() > 5000, "PDF should have reasonable size");

    // Write to output for visual verification
    let output_path = output_dir.join("full_deck_compass.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
    println!(
        "Full deck compass layout test PDF written to: {:?}",
        output_path
    );
}

#[test]
fn test_losers_table_generates_pdf() {
    // Create output directory
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create PDF document
    let mut doc = PdfDocument::new("Losers Table Test");

    // Load fonts using FontManager
    let fonts = FontManager::new(&mut doc).expect("Failed to load fonts");

    // Create renderer with default colors
    let colors = SuitColors::default();
    let renderer = LosersTableRenderer::new(
        &fonts.serif.regular,
        &fonts.serif.bold,
        &fonts.sans.regular, // Sans for suit symbols
        colors,
    );

    // Create layer and render
    let mut layer = LayerBuilder::new();
    let height = renderer.render(&mut layer, (Mm(50.0), Mm(250.0)));

    // Create page with the rendered content
    let page = PdfPage::new(Mm(210.0), Mm(297.0), layer.into_ops());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(pdf_bytes.len() > 1000, "PDF should have reasonable size");
    assert!(height > 0.0, "Rendered height should be positive");

    // Check dimensions
    let (width, expected_height) = renderer.dimensions();
    assert!(width > 0.0, "Table width should be positive");
    assert!(expected_height > 0.0, "Table height should be positive");

    // Write to output for visual verification
    let output_path = output_dir.join("losers_table_test.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
    println!("Losers table test PDF written to: {:?}", output_path);
}

#[test]
fn test_winners_table_generates_pdf() {
    // Create output directory
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create PDF document
    let mut doc = PdfDocument::new("Winners Table Test");

    // Load fonts using FontManager
    let fonts = FontManager::new(&mut doc).expect("Failed to load fonts");

    // Create renderer with default colors
    let colors = SuitColors::default();
    let renderer = WinnersTableRenderer::new(
        &fonts.serif.regular,
        &fonts.serif.bold,
        &fonts.sans.regular, // Sans for suit symbols
        colors,
    );

    // Create layer and render
    let mut layer = LayerBuilder::new();
    let height = renderer.render(&mut layer, (Mm(50.0), Mm(250.0)));

    // Create page with the rendered content
    let page = PdfPage::new(Mm(210.0), Mm(297.0), layer.into_ops());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(pdf_bytes.len() > 1000, "PDF should have reasonable size");
    assert!(height > 0.0, "Rendered height should be positive");

    // Check dimensions
    let (width, expected_height) = renderer.dimensions();
    assert!(width > 0.0, "Table width should be positive");
    assert!(expected_height > 0.0, "Table height should be positive");

    // Write to output for visual verification
    let output_path = output_dir.join("winners_table_test.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
    println!("Winners table test PDF written to: {:?}", output_path);
}

#[test]
fn test_declarers_plan_small_generates_pdf() {
    // Create output directory
    let output_dir = output_path();
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Create test hands (North dummy, South declarer)
    let north = Hand::from_holdings(
        Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen, Rank::Jack]),
        Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack]),
        Holding::from_ranks([Rank::Ace, Rank::Queen]),
        Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack, Rank::Ten]),
    );

    let south = Hand::from_holdings(
        Holding::from_ranks([Rank::Ten, Rank::Nine, Rank::Eight]),
        Holding::from_ranks([Rank::Ace, Rank::Ten, Rank::Nine, Rank::Eight]),
        Holding::from_ranks([Rank::King, Rank::Jack, Rank::Ten]),
        Holding::from_ranks([Rank::Ace, Rank::Nine, Rank::Eight]),
    );

    // Create PDF document
    let mut doc = PdfDocument::new("Declarer's Plan Small Test");

    // Load card assets and fonts
    let card_assets = CardAssets::load(&mut doc).expect("Failed to load card assets");
    let fonts = FontManager::new(&mut doc).expect("Failed to load fonts");

    // Create renderer with default colors
    let colors = SuitColors::default();
    let renderer = DeclarersPlanSmallRenderer::new(
        &card_assets,
        &fonts.serif.regular,
        &fonts.serif.bold,
        &fonts.sans.regular,
        colors.clone(),
    )
    .show_bounds(true);

    // Create layer
    let mut layer = LayerBuilder::new();

    // Test 1: Suit contract with opening lead (losers table)
    let opening_lead = Some(Card::new(Suit::Hearts, Rank::King));
    let height1 = renderer.render_with_info(
        &mut layer,
        &north,
        &south,
        false, // Suit contract
        opening_lead,
        Some(1),       // Deal 1
        Some("4♥ South"), // Contract
        (Mm(15.0), Mm(280.0)),
    );

    // Test 2: NT contract without opening lead (winners table) - top right
    let height2 = renderer.render_with_info(
        &mut layer,
        &north,
        &south,
        true, // NT contract
        None, // No opening lead
        Some(2),        // Deal 2
        Some("3NT South"), // Contract
        (Mm(115.0), Mm(280.0)),
    );

    // Test 3: Suit contract with opening lead (losers table) - bottom left
    let opening_lead3 = Some(Card::new(Suit::Spades, Rank::Ace));
    let height3 = renderer.render_with_info(
        &mut layer,
        &north,
        &south,
        false, // Suit contract
        opening_lead3,
        Some(3),       // Deal 3
        Some("4♠ South"), // Contract
        (Mm(15.0), Mm(140.0)),
    );

    // Test 4: NT contract with opening lead (winners table) - bottom right
    let opening_lead4 = Some(Card::new(Suit::Diamonds, Rank::Queen));
    let height4 = renderer.render_with_info(
        &mut layer,
        &north,
        &south,
        true, // NT contract
        opening_lead4,
        Some(4),        // Deal 4
        Some("1NT North"), // Contract
        (Mm(115.0), Mm(140.0)),
    );

    // Create page with the rendered content
    let page = PdfPage::new(Mm(210.0), Mm(297.0), layer.into_ops());
    let mut warnings: Vec<PdfWarnMsg> = Vec::new();
    let pdf_bytes = doc
        .with_pages(vec![page])
        .save(&PdfSaveOptions::default(), &mut warnings);

    // Verify PDF is valid
    assert!(
        pdf_bytes.starts_with(b"%PDF"),
        "PDF should start with %PDF header"
    );
    assert!(pdf_bytes.len() > 1000, "PDF should have reasonable size");
    assert!(
        height1 > 0.0,
        "Rendered height should be positive for deal 1"
    );
    assert!(
        height2 > 0.0,
        "Rendered height should be positive for deal 2"
    );
    assert!(
        height3 > 0.0,
        "Rendered height should be positive for deal 3"
    );
    assert!(
        height4 > 0.0,
        "Rendered height should be positive for deal 4"
    );

    // Write to output for visual verification
    let output_path = output_dir.join("declarers_plan_small_test.pdf");
    fs::write(&output_path, &pdf_bytes).expect("Failed to write test PDF");
    println!(
        "Declarer's plan small test PDF written to: {:?}",
        output_path
    );
}
