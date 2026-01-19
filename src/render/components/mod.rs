//! Rendering components for PDF generation

pub mod bidding_table;
pub mod commentary;
pub mod dummy;
pub mod fan;
pub mod hand_diagram;

pub use bidding_table::BiddingTableRenderer;
pub use commentary::CommentaryRenderer;
pub use dummy::DummyRenderer;
pub use fan::FanRenderer;
pub use hand_diagram::HandDiagramRenderer;
