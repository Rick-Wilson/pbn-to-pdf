pub mod cli;
pub mod config;
pub mod error;
pub mod model;
pub mod parser;
pub mod render;

pub use cli::Layout;
pub use config::Settings;
pub use error::{PbnError, RenderError};
pub use model::Board;
pub use parser::{parse_pbn, PbnFile};
pub use render::generate_pdf;

use parser::header::parse_headers;
use render::{BiddingSheetsRenderer, DealerSummaryRenderer, DeclarersPlanRenderer};

/// High-level API for rendering boards to PDF.
///
/// This is the recommended entry point for library consumers. It handles all
/// rendering details internally - consumers just provide boards, metadata, and
/// layout choice.
///
/// # Arguments
///
/// * `boards` - Slice of parsed Board structs to render
/// * `metadata_comments` - Raw PBN header comments (lines starting with %)
///   like `%HRTitleEvent My Event`, `%BCOptions Justify ShowHCP`, etc.
/// * `layout` - The layout style to use for rendering
///
/// # Returns
///
/// PDF file contents as bytes, or a RenderError on failure.
///
/// # Example
///
/// ```no_run
/// use pbn_to_pdf::{parse_pbn, render_boards, Layout};
///
/// let pbn_content = std::fs::read_to_string("hands.pbn").unwrap();
/// let pbn_file = parse_pbn(&pbn_content).unwrap();
///
/// // Extract metadata comments (lines starting with %)
/// let metadata_comments: Vec<String> = pbn_content
///     .lines()
///     .filter(|line| line.starts_with('%'))
///     .map(String::from)
///     .collect();
///
/// let pdf_bytes = render_boards(
///     &pbn_file.boards,
///     &metadata_comments,
///     Layout::DeclarersPlan,
/// ).unwrap();
///
/// std::fs::write("output.pdf", pdf_bytes).unwrap();
/// ```
pub fn render_boards(
    boards: &[Board],
    metadata_comments: &[String],
    layout: Layout,
) -> Result<Vec<u8>, RenderError> {
    // Parse metadata from raw comment lines
    let comment_refs: Vec<&str> = metadata_comments.iter().map(|s| s.as_str()).collect();
    let metadata = parse_headers(&comment_refs);

    // Create settings with layout-appropriate defaults, then apply metadata
    let settings = Settings::for_layout(layout).with_metadata(&metadata);

    // Route to the appropriate renderer based on layout
    match layout {
        Layout::Analysis => generate_pdf(boards, &settings),
        Layout::BiddingSheets => {
            let renderer = BiddingSheetsRenderer::new(settings);
            renderer.render(boards)
        }
        Layout::DeclarersPlan => {
            let renderer = DeclarersPlanRenderer::new(settings);
            renderer.render(boards)
        }
        Layout::DealerSummary => {
            let renderer = DealerSummaryRenderer::new(settings);
            renderer.render(boards)
        }
    }
}
