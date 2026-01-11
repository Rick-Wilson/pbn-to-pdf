use super::auction::{Auction, Contract};
use super::commentary::CommentaryBlock;
use super::deal::{Deal, Direction};
use super::play::PlaySequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Vulnerability {
    #[default]
    None,
    NorthSouth,
    EastWest,
    Both,
}

impl Vulnerability {
    pub fn from_pbn(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "NONE" | "-" | "LOVE" => Some(Vulnerability::None),
            "NS" | "N-S" => Some(Vulnerability::NorthSouth),
            "EW" | "E-W" => Some(Vulnerability::EastWest),
            "BOTH" | "ALL" => Some(Vulnerability::Both),
            _ => None,
        }
    }

    pub fn is_vulnerable(&self, direction: Direction) -> bool {
        match self {
            Vulnerability::None => false,
            Vulnerability::Both => true,
            Vulnerability::NorthSouth => matches!(direction, Direction::North | Direction::South),
            Vulnerability::EastWest => matches!(direction, Direction::East | Direction::West),
        }
    }
}

impl std::fmt::Display for Vulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vulnerability::None => write!(f, "None Vul"),
            Vulnerability::NorthSouth => write!(f, "N-S Vul"),
            Vulnerability::EastWest => write!(f, "E-W Vul"),
            Vulnerability::Both => write!(f, "Both Vul"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Board {
    // Identification
    pub number: Option<u32>,
    pub event: Option<String>,
    pub site: Option<String>,
    pub date: Option<String>,

    // Setup
    pub dealer: Option<Direction>,
    pub vulnerable: Vulnerability,
    pub deal: Deal,

    // Bidding
    pub auction: Option<Auction>,
    pub contract: Option<Contract>,
    pub declarer: Option<Direction>,

    // Play
    pub play: Option<PlaySequence>,
    pub result: Option<i8>,

    // Commentary
    pub commentary: Vec<CommentaryBlock>,
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_number(mut self, number: u32) -> Self {
        self.number = Some(number);
        self
    }

    pub fn with_dealer(mut self, dealer: Direction) -> Self {
        self.dealer = Some(dealer);
        self
    }

    pub fn with_vulnerability(mut self, vuln: Vulnerability) -> Self {
        self.vulnerable = vuln;
        self
    }

    pub fn with_deal(mut self, deal: Deal) -> Self {
        self.deal = deal;
        self
    }

    pub fn title(&self) -> String {
        let mut parts = Vec::new();

        if let Some(num) = self.number {
            parts.push(format!("Board {}", num));
        }

        if let Some(dealer) = self.dealer {
            parts.push(format!("{} Deals", dealer));
        }

        parts.push(self.vulnerable.to_string());

        parts.join(" • ")
    }

    pub fn has_commentary(&self) -> bool {
        !self.commentary.is_empty()
    }

    pub fn opening_lead_direction(&self) -> Option<Direction> {
        self.declarer.map(|d| d.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulnerability_parsing() {
        assert_eq!(Vulnerability::from_pbn("None"), Some(Vulnerability::None));
        assert_eq!(
            Vulnerability::from_pbn("NS"),
            Some(Vulnerability::NorthSouth)
        );
        assert_eq!(
            Vulnerability::from_pbn("E-W"),
            Some(Vulnerability::EastWest)
        );
        assert_eq!(Vulnerability::from_pbn("Both"), Some(Vulnerability::Both));
    }

    #[test]
    fn test_vulnerability_check() {
        assert!(!Vulnerability::None.is_vulnerable(Direction::North));
        assert!(Vulnerability::Both.is_vulnerable(Direction::North));
        assert!(Vulnerability::NorthSouth.is_vulnerable(Direction::South));
        assert!(!Vulnerability::NorthSouth.is_vulnerable(Direction::East));
    }

    #[test]
    fn test_board_title() {
        let board = Board::new()
            .with_number(1)
            .with_dealer(Direction::North)
            .with_vulnerability(Vulnerability::None);

        assert_eq!(board.title(), "Board 1 • North Deals • None Vul");
    }
}
