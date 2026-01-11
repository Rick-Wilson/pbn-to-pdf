/// Layout settings parsed from PBN % header directives
#[derive(Debug, Clone, Default)]
pub struct LayoutSettings {
    pub boards_per_page: Option<u8>,
    pub margins: Option<Margins>,
    pub paper_size: Option<PaperSize>,
    pub show_hcp: bool,
    pub show_card_table: bool,
    pub show_board_labels: bool,
    pub justify: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Margins {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            left: 15.0,
            right: 15.0,
            top: 15.0,
            bottom: 15.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PaperSize {
    Letter,
    A4,
    Legal,
}

impl PaperSize {
    pub fn dimensions_mm(&self) -> (f32, f32) {
        match self {
            PaperSize::Letter => (215.9, 279.4),
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::Legal => (215.9, 355.6),
        }
    }
}

impl Default for PaperSize {
    fn default() -> Self {
        PaperSize::Letter
    }
}

/// Single font specification from PBN
#[derive(Debug, Clone)]
pub struct FontSpec {
    pub family: String,
    pub size: f32,
    pub weight: u16,  // 400 = normal, 700 = bold
    pub italic: bool,
}

impl FontSpec {
    pub fn is_bold(&self) -> bool {
        self.weight >= 700
    }
}

/// Font settings parsed from PBN header
#[derive(Debug, Clone, Default)]
pub struct FontSettings {
    pub card_table: Option<FontSpec>,
    pub commentary: Option<FontSpec>,
    pub diagram: Option<FontSpec>,
    pub event: Option<FontSpec>,
    pub fixed_pitch: Option<FontSpec>,
    pub hand_record: Option<FontSpec>,
}

impl FontSettings {
    /// Get the card table font size (for compass text)
    pub fn card_table_size(&self) -> f32 {
        self.card_table.as_ref().map(|f| f.size).unwrap_or(11.0)
    }

    /// Get the commentary font size
    pub fn commentary_size(&self) -> f32 {
        self.commentary.as_ref().map(|f| f.size).unwrap_or(12.0)
    }

    /// Get the diagram font size (for hand cards)
    pub fn diagram_size(&self) -> f32 {
        self.diagram.as_ref().map(|f| f.size).unwrap_or(12.0)
    }

    /// Get the event/title font size
    pub fn event_size(&self) -> f32 {
        self.event.as_ref().map(|f| f.size).unwrap_or(20.0)
    }

    /// Check if event font should be bold
    pub fn event_is_bold(&self) -> bool {
        self.event.as_ref().map(|f| f.is_bold()).unwrap_or(true)
    }

    /// Get hand record font size (for board info like "Deal 1", dealer, vuln)
    pub fn hand_record_size(&self) -> f32 {
        self.hand_record.as_ref().map(|f| f.size).unwrap_or(11.0)
    }
}

/// Color settings for suits
#[derive(Debug, Clone)]
pub struct ColorSettings {
    pub spades: (u8, u8, u8),
    pub hearts: (u8, u8, u8),
    pub diamonds: (u8, u8, u8),
    pub clubs: (u8, u8, u8),
}

impl Default for ColorSettings {
    fn default() -> Self {
        Self {
            spades: (0, 0, 0),       // Black
            hearts: (255, 0, 0),     // Red
            diamonds: (255, 0, 0),   // Red
            clubs: (0, 0, 0),        // Black
        }
    }
}

/// Complete PBN file metadata
#[derive(Debug, Clone, Default)]
pub struct PbnMetadata {
    pub version: Option<String>,
    pub creator: Option<String>,
    pub created: Option<String>,
    pub title_event: Option<String>,
    pub title_date: Option<String>,
    pub layout: LayoutSettings,
    pub fonts: FontSettings,
    pub colors: ColorSettings,
}
