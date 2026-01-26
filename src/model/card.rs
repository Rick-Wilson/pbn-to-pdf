//! Card, Suit, and Rank types for bridge.
//!
//! This module re-exports types from bridge-types and provides additional
//! display-oriented helpers for PDF rendering.

// Re-export core types from bridge-types
pub use bridge_types::{Card, Rank, Suit};

/// Suits in display order (Spades first, as shown in bridge diagrams)
pub const SUITS_DISPLAY_ORDER: [Suit; 4] =
    [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];

/// Ranks in display order (Ace first, high to low)
pub const RANKS_DISPLAY_ORDER: [Rank; 13] = [
    Rank::Ace,
    Rank::King,
    Rank::Queen,
    Rank::Jack,
    Rank::Ten,
    Rank::Nine,
    Rank::Eight,
    Rank::Seven,
    Rank::Six,
    Rank::Five,
    Rank::Four,
    Rank::Three,
    Rank::Two,
];

/// Extension trait for Suit providing display-oriented methods
pub trait SuitExt {
    fn all_display() -> [Suit; 4];
}

impl SuitExt for Suit {
    /// Returns suits in display order (Spades first)
    fn all_display() -> [Suit; 4] {
        SUITS_DISPLAY_ORDER
    }
}

/// Extension trait for Rank providing display-oriented methods
pub trait RankExt {
    fn all_display() -> [Rank; 13];
    fn from_pbn_char(c: char) -> Option<Rank>;
    fn hcp_value(&self) -> u8;
}

impl RankExt for Rank {
    /// Returns ranks in display order (Ace first)
    fn all_display() -> [Rank; 13] {
        RANKS_DISPLAY_ORDER
    }

    /// Parse rank from a PBN character (alias for from_char)
    fn from_pbn_char(c: char) -> Option<Rank> {
        Rank::from_char(c)
    }

    /// Get HCP value (alias for hcp)
    fn hcp_value(&self) -> u8 {
        self.hcp()
    }
}

/// Compare ranks in display order (Ace > King > ... > Two)
pub fn rank_display_cmp(a: &Rank, b: &Rank) -> std::cmp::Ordering {
    // In bridge-types, Two=2, Three=3, ... Ace=14
    // For display, we want reverse order: Ace > King > ... > Two
    // So we compare in reverse
    (*b as u8).cmp(&(*a as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suit_symbols() {
        assert_eq!(Suit::Spades.symbol(), '♠');
        assert_eq!(Suit::Hearts.symbol(), '♥');
        assert_eq!(Suit::Diamonds.symbol(), '♦');
        assert_eq!(Suit::Clubs.symbol(), '♣');
    }

    #[test]
    fn test_suit_colors() {
        assert!(!Suit::Spades.is_red());
        assert!(Suit::Hearts.is_red());
        assert!(Suit::Diamonds.is_red());
        assert!(!Suit::Clubs.is_red());
    }

    #[test]
    fn test_rank_parsing() {
        assert_eq!(Rank::from_pbn_char('A'), Some(Rank::Ace));
        assert_eq!(Rank::from_pbn_char('T'), Some(Rank::Ten));
        assert_eq!(Rank::from_pbn_char('2'), Some(Rank::Two));
        assert_eq!(Rank::from_pbn_char('X'), None);
    }

    #[test]
    fn test_hcp_values() {
        assert_eq!(Rank::Ace.hcp_value(), 4);
        assert_eq!(Rank::King.hcp_value(), 3);
        assert_eq!(Rank::Queen.hcp_value(), 2);
        assert_eq!(Rank::Jack.hcp_value(), 1);
        assert_eq!(Rank::Ten.hcp_value(), 0);
    }

    #[test]
    fn test_display_order() {
        // First suit in display order should be Spades
        assert_eq!(SUITS_DISPLAY_ORDER[0], Suit::Spades);
        // First rank in display order should be Ace
        assert_eq!(RANKS_DISPLAY_ORDER[0], Rank::Ace);
        // Last rank should be Two
        assert_eq!(RANKS_DISPLAY_ORDER[12], Rank::Two);
    }

    #[test]
    fn test_rank_display_ordering() {
        use std::cmp::Ordering;
        assert_eq!(rank_display_cmp(&Rank::Ace, &Rank::King), Ordering::Less); // Ace sorts before King in display
        assert_eq!(
            rank_display_cmp(&Rank::Two, &Rank::Three),
            Ordering::Greater
        ); // Two sorts after Three
    }
}
