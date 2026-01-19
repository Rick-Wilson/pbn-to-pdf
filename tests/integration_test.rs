use std::fs;
use std::path::PathBuf;
use std::process::Command;

use pbn_to_pdf::config::Settings;
use pbn_to_pdf::parser::parse_pbn;
use pbn_to_pdf::render::generate_pdf;

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

        println!("Generated PDFs for: {}", stem);
    }
}
