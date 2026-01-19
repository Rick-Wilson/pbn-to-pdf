use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "pbn-to-pdf")]
#[command(
    author,
    version,
    about = "Convert PBN bridge files to PDF with Bridge Composer-style formatting"
)]
pub struct Args {
    /// Input PBN file path
    #[arg(required = true)]
    pub input: PathBuf,

    /// Output PDF file path (defaults to input with .pdf extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Number of boards per page (1, 2, or 4)
    #[arg(short = 'n', long, default_value = "1", value_parser = clap::value_parser!(u8).range(1..=4))]
    pub boards_per_page: u8,

    /// Page size
    #[arg(short = 's', long, value_enum, default_value = "letter")]
    pub page_size: PageSize,

    /// Page orientation
    #[arg(long, value_enum, default_value = "portrait")]
    pub orientation: Orientation,

    /// Output layout style
    #[arg(short = 'l', long, value_enum, default_value = "analysis")]
    pub layout: Layout,

    /// Hide bidding table
    #[arg(long)]
    pub no_bidding: bool,

    /// Hide play sequence
    #[arg(long)]
    pub no_play: bool,

    /// Hide commentary text
    #[arg(long)]
    pub no_commentary: bool,

    /// Hide HCP point counts
    #[arg(long)]
    pub no_hcp: bool,

    /// Board range to include (e.g., "1-16" or "5,8,12")
    #[arg(short = 'b', long)]
    pub boards: Option<String>,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum PageSize {
    Letter,
    A4,
    Legal,
}

impl PageSize {
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match self {
            PageSize::Letter => (215.9, 279.4),
            PageSize::A4 => (210.0, 297.0),
            PageSize::Legal => (215.9, 355.6),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Orientation {
    Portrait,
    Landscape,
}

/// Output layout style
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, Default)]
pub enum Layout {
    /// Standard analysis layout with hand diagram, bidding, and commentary
    #[default]
    Analysis,
    /// Bidding practice sheets for face-to-face practice
    BiddingSheets,
}

impl Args {
    /// Get the output path, defaulting to input with .pdf extension
    pub fn output_path(&self) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| self.input.with_extension("pdf"))
    }

    /// Get page dimensions in mm (width, height) accounting for orientation
    pub fn page_dimensions(&self) -> (f32, f32) {
        let (w, h) = self.page_size.dimensions_mm();
        match self.orientation {
            Orientation::Portrait => (w, h),
            Orientation::Landscape => (h, w),
        }
    }

    /// Check if bidding should be shown
    pub fn show_bidding(&self) -> bool {
        !self.no_bidding
    }

    /// Check if play should be shown
    pub fn show_play(&self) -> bool {
        !self.no_play
    }

    /// Check if commentary should be shown
    pub fn show_commentary(&self) -> bool {
        !self.no_commentary
    }

    /// Check if HCP should be shown
    pub fn show_hcp(&self) -> bool {
        !self.no_hcp
    }
}

/// Parse a board range specification
pub fn parse_board_range(spec: &str) -> Result<Vec<u32>, String> {
    let mut boards = Vec::new();

    for part in spec.split(',') {
        let part = part.trim();

        if part.contains('-') {
            // Range: "1-16"
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() != 2 {
                return Err(format!("Invalid range: {}", part));
            }

            let start: u32 = parts[0]
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number: {}", parts[0]))?;
            let end: u32 = parts[1]
                .trim()
                .parse()
                .map_err(|_| format!("Invalid number: {}", parts[1]))?;

            if start > end {
                return Err(format!("Invalid range: {} > {}", start, end));
            }

            for i in start..=end {
                boards.push(i);
            }
        } else {
            // Single number
            let num: u32 = part
                .parse()
                .map_err(|_| format!("Invalid number: {}", part))?;
            boards.push(num);
        }
    }

    Ok(boards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_board() {
        let result = parse_board_range("5").unwrap();
        assert_eq!(result, vec![5]);
    }

    #[test]
    fn test_parse_range() {
        let result = parse_board_range("1-4").unwrap();
        assert_eq!(result, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_parse_mixed() {
        let result = parse_board_range("1-3, 7, 10-12").unwrap();
        assert_eq!(result, vec![1, 2, 3, 7, 10, 11, 12]);
    }

    #[test]
    fn test_page_dimensions() {
        let args = Args {
            input: PathBuf::from("test.pbn"),
            output: None,
            boards_per_page: 1,
            page_size: PageSize::Letter,
            orientation: Orientation::Portrait,
            layout: Layout::Analysis,
            no_bidding: false,
            no_play: false,
            no_commentary: false,
            no_hcp: false,
            boards: None,
            verbose: 0,
        };

        let (w, h) = args.page_dimensions();
        assert!((w - 215.9).abs() < 0.1);
        assert!((h - 279.4).abs() < 0.1);
    }
}
