use super::card::Card;
use super::deal::Direction;

#[derive(Debug, Clone)]
pub struct Trick {
    pub leader: Direction,
    pub cards: [Option<Card>; 4],
    pub winner: Option<Direction>,
}

impl Trick {
    pub fn new(leader: Direction) -> Self {
        Self {
            leader,
            cards: [None, None, None, None],
            winner: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.cards.iter().all(|c| c.is_some())
    }

    pub fn set_card(&mut self, position: usize, card: Card) {
        if position < 4 {
            self.cards[position] = Some(card);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaySequence {
    pub opening_leader: Direction,
    pub tricks: Vec<Trick>,
}

impl PlaySequence {
    pub fn new(opening_leader: Direction) -> Self {
        Self {
            opening_leader,
            tricks: Vec::new(),
        }
    }

    pub fn add_trick(&mut self, trick: Trick) {
        self.tricks.push(trick);
    }

    pub fn is_complete(&self) -> bool {
        self.tricks.len() == 13 && self.tricks.iter().all(|t| t.is_complete())
    }

    pub fn tricks_played(&self) -> usize {
        self.tricks.iter().filter(|t| t.is_complete()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trick_complete() {
        let mut trick = Trick::new(Direction::West);
        assert!(!trick.is_complete());

        use super::super::card::{Rank, Suit};
        trick.set_card(0, Card::new(Suit::Spades, Rank::Ace));
        trick.set_card(1, Card::new(Suit::Spades, Rank::King));
        trick.set_card(2, Card::new(Suit::Spades, Rank::Queen));
        trick.set_card(3, Card::new(Suit::Spades, Rank::Jack));
        assert!(trick.is_complete());
    }
}
