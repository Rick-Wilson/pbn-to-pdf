use std::collections::BTreeSet;
use std::fmt;

use super::card::{Rank, Suit};

/// Cards held in a single suit
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Holding {
    pub ranks: BTreeSet<Rank>,
}

impl Holding {
    pub fn new() -> Self {
        Self {
            ranks: BTreeSet::new(),
        }
    }

    pub fn from_ranks(ranks: impl IntoIterator<Item = Rank>) -> Self {
        Self {
            ranks: ranks.into_iter().collect(),
        }
    }

    pub fn add(&mut self, rank: Rank) {
        self.ranks.insert(rank);
    }

    pub fn len(&self) -> usize {
        self.ranks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ranks.is_empty()
    }

    pub fn hcp(&self) -> u8 {
        self.ranks.iter().map(|r| r.hcp_value()).sum()
    }

    pub fn is_void(&self) -> bool {
        self.ranks.is_empty()
    }
}

impl fmt::Display for Holding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ranks.is_empty() {
            write!(f, "-")
        } else {
            for rank in &self.ranks {
                write!(f, "{}", rank.to_char())?;
            }
            Ok(())
        }
    }
}

/// A player's 13-card hand
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hand {
    pub spades: Holding,
    pub hearts: Holding,
    pub diamonds: Holding,
    pub clubs: Holding,
}

impl Hand {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_holdings(
        spades: Holding,
        hearts: Holding,
        diamonds: Holding,
        clubs: Holding,
    ) -> Self {
        Self {
            spades,
            hearts,
            diamonds,
            clubs,
        }
    }

    pub fn holding(&self, suit: Suit) -> &Holding {
        match suit {
            Suit::Spades => &self.spades,
            Suit::Hearts => &self.hearts,
            Suit::Diamonds => &self.diamonds,
            Suit::Clubs => &self.clubs,
        }
    }

    pub fn holding_mut(&mut self, suit: Suit) -> &mut Holding {
        match suit {
            Suit::Spades => &mut self.spades,
            Suit::Hearts => &mut self.hearts,
            Suit::Diamonds => &mut self.diamonds,
            Suit::Clubs => &mut self.clubs,
        }
    }

    pub fn total_hcp(&self) -> u8 {
        self.spades.hcp() + self.hearts.hcp() + self.diamonds.hcp() + self.clubs.hcp()
    }

    pub fn shape(&self) -> [u8; 4] {
        [
            self.spades.len() as u8,
            self.hearts.len() as u8,
            self.diamonds.len() as u8,
            self.clubs.len() as u8,
        ]
    }

    pub fn card_count(&self) -> usize {
        self.spades.len() + self.hearts.len() + self.diamonds.len() + self.clubs.len()
    }
}

impl fmt::Display for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {} {} {} {} {}",
            Suit::Spades.symbol(),
            self.spades,
            Suit::Hearts.symbol(),
            self.hearts,
            Suit::Diamonds.symbol(),
            self.diamonds,
            Suit::Clubs.symbol(),
            self.clubs
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holding_hcp() {
        let holding = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);
        assert_eq!(holding.hcp(), 9);
    }

    #[test]
    fn test_empty_holding() {
        let holding = Holding::new();
        assert!(holding.is_void());
        assert_eq!(holding.to_string(), "-");
    }

    #[test]
    fn test_hand_shape() {
        let mut hand = Hand::new();
        hand.spades =
            Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen, Rank::Jack, Rank::Ten]);
        hand.hearts = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);
        hand.diamonds = Holding::from_ranks([Rank::Ace, Rank::King]);
        hand.clubs = Holding::from_ranks([Rank::Ace, Rank::King, Rank::Queen]);

        assert_eq!(hand.shape(), [5, 3, 2, 3]);
        assert_eq!(hand.card_count(), 13);
    }
}
