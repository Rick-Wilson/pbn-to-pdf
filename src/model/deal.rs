use std::fmt;

use super::card::Suit;
use super::hand::Hand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_uppercase() {
            'N' => Some(Direction::North),
            'E' => Some(Direction::East),
            'S' => Some(Direction::South),
            'W' => Some(Direction::West),
            _ => None,
        }
    }

    pub fn to_char(&self) -> char {
        match self {
            Direction::North => 'N',
            Direction::East => 'E',
            Direction::South => 'S',
            Direction::West => 'W',
        }
    }

    pub fn next(&self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    pub fn partner(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    /// Returns the table position (0-3) for bidding display (West=0, North=1, East=2, South=3)
    pub fn table_position(&self) -> usize {
        match self {
            Direction::West => 0,
            Direction::North => 1,
            Direction::East => 2,
            Direction::South => 3,
        }
    }

    pub fn all() -> [Direction; 4] {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::North => write!(f, "North"),
            Direction::East => write!(f, "East"),
            Direction::South => write!(f, "South"),
            Direction::West => write!(f, "West"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Deal {
    pub north: Hand,
    pub east: Hand,
    pub south: Hand,
    pub west: Hand,
}

impl Deal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hand(&self, direction: Direction) -> &Hand {
        match direction {
            Direction::North => &self.north,
            Direction::East => &self.east,
            Direction::South => &self.south,
            Direction::West => &self.west,
        }
    }

    pub fn hand_mut(&mut self, direction: Direction) -> &mut Hand {
        match direction {
            Direction::North => &mut self.north,
            Direction::East => &mut self.east,
            Direction::South => &mut self.south,
            Direction::West => &mut self.west,
        }
    }

    pub fn set_hand(&mut self, direction: Direction, hand: Hand) {
        match direction {
            Direction::North => self.north = hand,
            Direction::East => self.east = hand,
            Direction::South => self.south = hand,
            Direction::West => self.west = hand,
        }
    }

    /// Returns which suits have at least one card across all four hands.
    /// Used to detect hand fragments that only show certain suits.
    pub fn suits_present(&self) -> Vec<Suit> {
        Suit::all()
            .into_iter()
            .filter(|suit| {
                !self.north.holding(*suit).is_void()
                    || !self.east.holding(*suit).is_void()
                    || !self.south.holding(*suit).is_void()
                    || !self.west.holding(*suit).is_void()
            })
            .collect()
    }

    /// Returns true if this is a hand fragment (not all suits have cards)
    pub fn is_fragment(&self) -> bool {
        self.suits_present().len() < 4
    }

    /// Returns true if this deal has no cards at all (empty deal)
    pub fn is_empty(&self) -> bool {
        self.suits_present().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_from_char() {
        assert_eq!(Direction::from_char('N'), Some(Direction::North));
        assert_eq!(Direction::from_char('e'), Some(Direction::East));
        assert_eq!(Direction::from_char('X'), None);
    }

    #[test]
    fn test_direction_next() {
        assert_eq!(Direction::North.next(), Direction::East);
        assert_eq!(Direction::East.next(), Direction::South);
        assert_eq!(Direction::South.next(), Direction::West);
        assert_eq!(Direction::West.next(), Direction::North);
    }

    #[test]
    fn test_direction_partner() {
        assert_eq!(Direction::North.partner(), Direction::South);
        assert_eq!(Direction::East.partner(), Direction::West);
    }

    #[test]
    fn test_table_position() {
        assert_eq!(Direction::West.table_position(), 0);
        assert_eq!(Direction::North.table_position(), 1);
        assert_eq!(Direction::East.table_position(), 2);
        assert_eq!(Direction::South.table_position(), 3);
    }

    #[test]
    fn test_suits_present_full_deal() {
        use super::super::hand::Holding;
        use super::super::card::Rank;

        let mut deal = Deal::new();
        deal.north.spades = Holding::from_ranks([Rank::Ace, Rank::King]);
        deal.north.hearts = Holding::from_ranks([Rank::Ace]);
        deal.south.diamonds = Holding::from_ranks([Rank::Ace]);
        deal.west.clubs = Holding::from_ranks([Rank::Ace]);

        let suits = deal.suits_present();
        assert_eq!(suits.len(), 4);
        assert!(!deal.is_fragment());
    }

    #[test]
    fn test_suits_present_spades_only() {
        use super::super::hand::Holding;
        use super::super::card::Rank;

        let mut deal = Deal::new();
        deal.north.spades = Holding::from_ranks([Rank::King, Rank::Queen, Rank::Jack, Rank::Ten]);
        deal.east.spades = Holding::from_ranks([Rank::Seven, Rank::Six, Rank::Four, Rank::Two]);

        let suits = deal.suits_present();
        assert_eq!(suits.len(), 1);
        assert_eq!(suits[0], Suit::Spades);
        assert!(deal.is_fragment());
    }

    #[test]
    fn test_suits_present_empty_deal() {
        let deal = Deal::new();
        let suits = deal.suits_present();
        assert_eq!(suits.len(), 0);
        assert!(deal.is_fragment());
        assert!(deal.is_empty());
    }

    #[test]
    fn test_is_empty_with_cards() {
        use super::super::card::Rank;
        use super::super::hand::Holding;

        let mut deal = Deal::new();
        deal.north.spades = Holding::from_ranks([Rank::Ace]);

        assert!(!deal.is_empty());
    }
}
