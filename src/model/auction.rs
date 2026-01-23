use std::collections::HashMap;
use std::fmt;

use super::deal::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BidSuit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    NoTrump,
}

impl BidSuit {
    pub fn symbol(&self) -> &'static str {
        match self {
            BidSuit::Clubs => "♣",
            BidSuit::Diamonds => "♦",
            BidSuit::Hearts => "♥",
            BidSuit::Spades => "♠",
            BidSuit::NoTrump => "NT",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "C" => Some(BidSuit::Clubs),
            "D" => Some(BidSuit::Diamonds),
            "H" => Some(BidSuit::Hearts),
            "S" => Some(BidSuit::Spades),
            "N" | "NT" => Some(BidSuit::NoTrump),
            _ => None,
        }
    }

    pub fn is_red(&self) -> bool {
        matches!(self, BidSuit::Hearts | BidSuit::Diamonds)
    }
}

impl fmt::Display for BidSuit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Pass,
    Bid { level: u8, suit: BidSuit },
    Double,
    Redouble,
    /// "+" in PBN - indicates auction continues (student fills in next bid)
    /// Displayed as "?" in output
    Continue,
}

impl Call {
    pub fn from_pbn(s: &str) -> Option<Self> {
        let s = s.trim();
        match s.to_uppercase().as_str() {
            "PASS" | "P" => Some(Call::Pass),
            "X" => Some(Call::Double),
            "XX" => Some(Call::Redouble),
            "+" => Some(Call::Continue),
            "AP" => None, // All Pass is handled specially
            _ => {
                // Parse "1C", "2H", "3NT", etc.
                let mut chars = s.chars();
                let level = chars.next()?.to_digit(10)? as u8;
                if !(1..=7).contains(&level) {
                    return None;
                }
                let suit_str: String = chars.collect();
                let suit = BidSuit::parse(&suit_str)?;
                Some(Call::Bid { level, suit })
            }
        }
    }
}

impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Call::Pass => write!(f, "Pass"),
            Call::Double => write!(f, "X"),
            Call::Redouble => write!(f, "XX"),
            Call::Continue => write!(f, "?"),
            Call::Bid { level, suit } => write!(f, "{}{}", level, suit),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnnotatedCall {
    pub call: Call,
    pub annotation: Option<String>,
}

impl AnnotatedCall {
    pub fn new(call: Call) -> Self {
        Self {
            call,
            annotation: None,
        }
    }

    pub fn with_annotation(call: Call, annotation: String) -> Self {
        Self {
            call,
            annotation: Some(annotation),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Auction {
    pub dealer: Direction,
    pub calls: Vec<AnnotatedCall>,
    pub is_passed_out: bool,
    /// Notes/alerts referenced by =N= in the auction
    pub notes: HashMap<u8, String>,
}

impl Auction {
    pub fn new(dealer: Direction) -> Self {
        Self {
            dealer,
            calls: Vec::new(),
            is_passed_out: false,
            notes: HashMap::new(),
        }
    }

    pub fn add_note(&mut self, number: u8, text: String) {
        self.notes.insert(number, text);
    }

    pub fn add_call(&mut self, call: Call) {
        self.calls.push(AnnotatedCall::new(call));
    }

    pub fn add_annotated_call(&mut self, call: Call, annotation: Option<String>) {
        self.calls.push(AnnotatedCall { call, annotation });
    }

    /// Returns true if this is an uncontested auction (one pair only bids, opponents only pass)
    /// Returns the bidding pair: Some((Direction, Direction)) for the pair that bids
    /// - West/East pair if N/S only pass
    /// - North/South pair if E/W only pass
    /// Returns None if both pairs bid (contested)
    pub fn uncontested_pair(&self) -> Option<(Direction, Direction)> {
        let mut ns_bid = false;
        let mut ew_bid = false;

        let mut current = self.dealer;
        for annotated in &self.calls {
            let is_bid = !matches!(
                annotated.call,
                Call::Pass | Call::Continue
            );
            if is_bid {
                match current {
                    Direction::North | Direction::South => ns_bid = true,
                    Direction::East | Direction::West => ew_bid = true,
                }
            }
            current = current.next();
        }

        match (ns_bid, ew_bid) {
            (true, false) => Some((Direction::North, Direction::South)),
            (false, true) => Some((Direction::West, Direction::East)),
            _ => None, // Both pairs bid or neither bid
        }
    }

    pub fn final_contract(&self) -> Option<Contract> {
        let mut last_bid: Option<(u8, BidSuit, Direction)> = None;
        let mut doubled = false;
        let mut redoubled = false;
        let mut current_player = self.dealer;

        for annotated in &self.calls {
            match &annotated.call {
                Call::Bid { level, suit } => {
                    last_bid = Some((*level, *suit, current_player));
                    doubled = false;
                    redoubled = false;
                }
                Call::Double => {
                    doubled = true;
                    redoubled = false;
                }
                Call::Redouble => {
                    doubled = false;
                    redoubled = true;
                }
                Call::Pass | Call::Continue => {}
            }
            current_player = current_player.next();
        }

        last_bid.map(|(level, suit, declarer)| Contract {
            level,
            suit,
            doubled,
            redoubled,
            declarer,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Contract {
    pub level: u8,
    pub suit: BidSuit,
    pub doubled: bool,
    pub redoubled: bool,
    pub declarer: Direction,
}

impl Contract {
    /// Parse a contract string like "1NT", "4S", "4HX", "3NTXX"
    /// The declarer is not included in PBN contract strings (it's a separate tag)
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        // First character must be level 1-7
        let level = s.chars().next()?.to_digit(10)? as u8;
        if !(1..=7).contains(&level) {
            return None;
        }

        let rest = &s[1..];

        // Check for doubled/redoubled at end
        let (suit_part, doubled, redoubled) = if let Some(stripped) = rest.strip_suffix("XX") {
            (stripped, false, true)
        } else if let Some(stripped) = rest.strip_suffix('X') {
            (stripped, true, false)
        } else {
            (rest, false, false)
        };

        // Parse suit
        let suit = BidSuit::parse(suit_part)?;

        Some(Contract {
            level,
            suit,
            doubled,
            redoubled,
            declarer: Direction::South, // Default, should be set from Declarer tag
        })
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.level, self.suit)?;
        if self.redoubled {
            write!(f, "XX")?;
        } else if self.doubled {
            write!(f, "X")?;
        }
        write!(f, " by {}", self.declarer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bid_parsing() {
        assert_eq!(
            Call::from_pbn("1C"),
            Some(Call::Bid {
                level: 1,
                suit: BidSuit::Clubs
            })
        );
        assert_eq!(
            Call::from_pbn("3NT"),
            Some(Call::Bid {
                level: 3,
                suit: BidSuit::NoTrump
            })
        );
        assert_eq!(Call::from_pbn("Pass"), Some(Call::Pass));
        assert_eq!(Call::from_pbn("X"), Some(Call::Double));
        assert_eq!(Call::from_pbn("XX"), Some(Call::Redouble));
    }

    #[test]
    fn test_bid_display() {
        assert_eq!(
            Call::Bid {
                level: 1,
                suit: BidSuit::NoTrump
            }
            .to_string(),
            "1NT"
        );
    }

    #[test]
    fn test_contract_display() {
        let contract = Contract {
            level: 4,
            suit: BidSuit::Spades,
            doubled: true,
            redoubled: false,
            declarer: Direction::South,
        };
        assert_eq!(contract.to_string(), "4♠X by South");
    }
}
