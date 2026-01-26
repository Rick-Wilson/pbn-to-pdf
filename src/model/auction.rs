//! Auction types for bridge bidding.
//!
//! Re-exports core types from bridge-types with display-oriented extensions.

use std::fmt;

use super::deal::Direction;

// Re-export core types from bridge-types
pub use bridge_types::{AnnotatedCall, Call, FinalContract, Strain};

// Type alias for backward compatibility
pub type BidSuit = Strain;

/// Extension trait for Call to add pbn-to-pdf specific functionality
pub trait CallExt {
    fn from_pbn_ext(s: &str) -> Option<Call>;
}

impl CallExt for Call {
    /// Parse a call from PBN notation, with special handling for "AP" (All Pass)
    fn from_pbn_ext(s: &str) -> Option<Call> {
        let s = s.trim();
        if s.to_uppercase() == "AP" {
            return None; // All Pass is handled specially
        }
        Call::from_pbn(s)
    }
}

/// A complete auction (bidding sequence)
#[derive(Debug, Clone)]
pub struct Auction {
    pub dealer: Direction,
    pub calls: Vec<AnnotatedCall>,
    pub is_passed_out: bool,
    /// Notes/alerts referenced by =N= in the auction
    pub notes: std::collections::HashMap<u8, String>,
}

impl Auction {
    pub fn new(dealer: Direction) -> Self {
        Self {
            dealer,
            calls: Vec::new(),
            is_passed_out: false,
            notes: std::collections::HashMap::new(),
        }
    }

    pub fn add_note(&mut self, number: u8, text: String) {
        self.notes.insert(number, text);
    }

    pub fn add_call(&mut self, call: Call) {
        self.calls.push(AnnotatedCall::new(call));
    }

    pub fn add_annotated_call(&mut self, call: Call, annotation: Option<String>) {
        if let Some(ann) = annotation {
            self.calls.push(AnnotatedCall::with_annotation(call, ann));
        } else {
            self.calls.push(AnnotatedCall::new(call));
        }
    }

    /// Returns true if this is an uncontested auction (one pair only bids, opponents only pass)
    /// Returns the bidding pair: Some((Direction, Direction)) for the pair that bids
    pub fn uncontested_pair(&self) -> Option<(Direction, Direction)> {
        let mut ns_bid = false;
        let mut ew_bid = false;

        let mut current = self.dealer;
        for annotated in &self.calls {
            let dominated = matches!(annotated.call, Call::Pass | Call::Continue);
            if !dominated {
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
            _ => None,
        }
    }

    pub fn final_contract(&self) -> Option<Contract> {
        let mut last_bid: Option<(u8, Strain, Direction)> = None;
        let mut doubled = false;
        let mut redoubled = false;
        let mut current_player = self.dealer;

        for annotated in &self.calls {
            match &annotated.call {
                Call::Bid { level, strain } => {
                    last_bid = Some((*level, *strain, current_player));
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
                Call::Pass | Call::Continue | Call::Blank => {}
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

/// The contract resulting from an auction
#[derive(Debug, Clone)]
pub struct Contract {
    pub level: u8,
    pub suit: Strain,
    pub doubled: bool,
    pub redoubled: bool,
    pub declarer: Direction,
}

impl Contract {
    /// Parse a contract string like "1NT", "4S", "4HX", "3NTXX"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        let level = s.chars().next()?.to_digit(10)? as u8;
        if !(1..=7).contains(&level) {
            return None;
        }

        let rest = &s[1..];
        let (suit_part, doubled, redoubled) = if let Some(stripped) = rest.strip_suffix("XX") {
            (stripped, false, true)
        } else if let Some(stripped) = rest.strip_suffix('X') {
            (stripped, true, false)
        } else {
            (rest, false, false)
        };

        let suit = Strain::from_str(suit_part)?;

        Some(Contract {
            level,
            suit,
            doubled,
            redoubled,
            declarer: Direction::South, // Default, should be set from Declarer tag
        })
    }

    /// Convert to bridge_types::FinalContract
    pub fn to_final_contract(&self) -> FinalContract {
        let mut fc = FinalContract::new(self.level, self.suit, self.declarer);
        if self.redoubled {
            fc = fc.redoubled();
        } else if self.doubled {
            fc = fc.doubled();
        }
        fc
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
                strain: Strain::Clubs
            })
        );
        assert_eq!(
            Call::from_pbn("3NT"),
            Some(Call::Bid {
                level: 3,
                strain: Strain::NoTrump
            })
        );
        assert_eq!(Call::from_pbn("Pass"), Some(Call::Pass));
        assert_eq!(Call::from_pbn("X"), Some(Call::Double));
        assert_eq!(Call::from_pbn("XX"), Some(Call::Redouble));
        assert_eq!(Call::from_pbn("+"), Some(Call::Continue));
    }

    #[test]
    fn test_bid_display() {
        assert_eq!(
            Call::Bid {
                level: 1,
                strain: Strain::NoTrump
            }
            .to_string(),
            "1NT"
        );
    }

    #[test]
    fn test_contract_display() {
        let contract = Contract {
            level: 4,
            suit: Strain::Spades,
            doubled: true,
            redoubled: false,
            declarer: Direction::South,
        };
        assert_eq!(contract.to_string(), "4♠X by South");
    }

    #[test]
    fn test_bidsuit_alias() {
        // BidSuit is now an alias for Strain
        let suit: BidSuit = Strain::Hearts;
        assert!(suit.is_red());
        assert_eq!(suit.symbol(), "♥");
    }
}
