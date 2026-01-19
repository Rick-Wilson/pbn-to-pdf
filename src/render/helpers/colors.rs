use crate::model::Suit;
use printpdf::Rgb;

/// Color provider for suit symbols
#[derive(Debug, Clone)]
pub struct SuitColors {
    pub spades: Rgb,
    pub hearts: Rgb,
    pub diamonds: Rgb,
    pub clubs: Rgb,
}

impl Default for SuitColors {
    fn default() -> Self {
        Self {
            spades: Rgb::new(0.0, 0.0, 0.0, None),   // Black
            hearts: Rgb::new(0.8, 0.0, 0.0, None),   // Red
            diamonds: Rgb::new(0.8, 0.0, 0.0, None), // Red
            clubs: Rgb::new(0.0, 0.0, 0.0, None),    // Black
        }
    }
}

impl SuitColors {
    pub fn new(black: (f32, f32, f32), red: (f32, f32, f32)) -> Self {
        Self {
            spades: Rgb::new(black.0, black.1, black.2, None),
            hearts: Rgb::new(red.0, red.1, red.2, None),
            diamonds: Rgb::new(red.0, red.1, red.2, None),
            clubs: Rgb::new(black.0, black.1, black.2, None),
        }
    }

    pub fn for_suit(&self, suit: &Suit) -> Rgb {
        match suit {
            Suit::Spades => self.spades.clone(),
            Suit::Hearts => self.hearts.clone(),
            Suit::Diamonds => self.diamonds.clone(),
            Suit::Clubs => self.clubs.clone(),
        }
    }
}

/// Standard colors
pub const BLACK: Rgb = Rgb {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    icc_profile: None,
};

pub const GRAY: Rgb = Rgb {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    icc_profile: None,
};

/// Light gray for HCP box background
pub const LIGHT_GRAY: Rgb = Rgb {
    r: 0.9,
    g: 0.9,
    b: 0.9,
    icc_profile: None,
};

/// Green color for compass rose (Bridge Composer style)
pub const GREEN: Rgb = Rgb {
    r: 0.0,
    g: 0.5,
    b: 0.0,
    icc_profile: None,
};

/// White color for compass letters
pub const WHITE: Rgb = Rgb {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    icc_profile: None,
};
