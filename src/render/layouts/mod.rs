//! Layout renderers - one per --layout option

pub mod analysis;
pub mod bidding_sheets;
pub mod dealer_summary;
pub mod declarers_plan;

pub use analysis::generate_pdf;
pub use bidding_sheets::BiddingSheetsRenderer;
pub use dealer_summary::DealerSummaryRenderer;
pub use declarers_plan::DeclarersPlanRenderer;
