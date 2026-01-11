use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub fn symbol(&self) -> char {
        match self {
            Suit::Spades => '\u{2660}',   // ♠
            Suit::Hearts => '\u{2665}',   // ♥
            Suit::Diamonds => '\u{2666}', // ♦
            Suit::Clubs => '\u{2663}',    // ♣
        }
    }

    pub fn is_red(&self) -> bool {
        matches!(self, Suit::Hearts | Suit::Diamonds)
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'S' => Some(Suit::Spades),
            'H' => Some(Suit::Hearts),
            'D' => Some(Suit::Diamonds),
            'C' => Some(Suit::Clubs),
            _ => None,
        }
    }

    pub fn all() -> [Suit; 4] {
        [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs]
    }
}

impl fmt::Display for Suit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Ace,
    King,
    Queen,
    Jack,
    Ten,
    Nine,
    Eight,
    Seven,
    Six,
    Five,
    Four,
    Three,
    Two,
}

impl Rank {
    pub fn from_pbn_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'A' => Some(Rank::Ace),
            'K' => Some(Rank::King),
            'Q' => Some(Rank::Queen),
            'J' => Some(Rank::Jack),
            'T' => Some(Rank::Ten),
            '9' => Some(Rank::Nine),
            '8' => Some(Rank::Eight),
            '7' => Some(Rank::Seven),
            '6' => Some(Rank::Six),
            '5' => Some(Rank::Five),
            '4' => Some(Rank::Four),
            '3' => Some(Rank::Three),
            '2' => Some(Rank::Two),
            _ => None,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            Rank::Ace => 'A',
            Rank::King => 'K',
            Rank::Queen => 'Q',
            Rank::Jack => 'J',
            Rank::Ten => 'T',
            Rank::Nine => '9',
            Rank::Eight => '8',
            Rank::Seven => '7',
            Rank::Six => '6',
            Rank::Five => '5',
            Rank::Four => '4',
            Rank::Three => '3',
            Rank::Two => '2',
        }
    }

    pub fn hcp_value(&self) -> u8 {
        match self {
            Rank::Ace => 4,
            Rank::King => 3,
            Rank::Queen => 2,
            Rank::Jack => 1,
            _ => 0,
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.suit.symbol(), self.rank.to_char())
    }
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
}
