# pbn-to-pdf Design Document

## Overview

`pbn-to-pdf` is a Rust CLI tool that converts PBN (Portable Bridge Notation) files to PDF documents with Bridge Composer-style formatting. The tool is designed to produce professional-quality bridge hand diagrams suitable for teaching materials and publications.

## Requirements

### Functional Requirements

1. **PBN Parsing**
   - Parse PBN 2.1 format files
   - Extract hand distributions from Deal notation
   - Parse bidding auctions with annotations
   - Parse play sequences
   - Extract commentary with embedded formatting
   - Read Bridge Composer header directives for layout hints

2. **PDF Generation**
   - Render hands in compass-rose layout (N at top, S at bottom, E/W on sides)
   - Display Unicode suit symbols with appropriate colors (black for ♠♣, red for ♥♦)
   - Show HCP (High Card Points) for each hand
   - Render bidding table in 4-column format (W/N/E/S)
   - Render formatted commentary text
   - Support multiple boards per page (1, 2, or 4)

3. **CLI Interface**
   - Accept input PBN file path
   - Allow output path specification
   - Provide options to hide/show various elements
   - Support board range filtering
   - Allow page size and orientation selection

### Non-Functional Requirements

- Fast parsing and rendering
- Minimal dependencies
- Cross-platform compatibility (macOS, Linux, Windows)
- Readable, maintainable code structure

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           CLI Layer                              │
│                         (src/cli/)                               │
│  - Argument parsing (clap)                                       │
│  - Board range filtering                                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Parser Layer                             │
│                        (src/parser/)                             │
│  - PBN file parsing                                              │
│  - Tag pairs, deal notation, auctions, play, commentary          │
│  - Header directive parsing                                      │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Model Layer                              │
│                        (src/model/)                              │
│  - Card, Rank, Suit                                              │
│  - Hand, Holding, Deal                                           │
│  - Auction, Call, Contract                                       │
│  - Board, Vulnerability                                          │
│  - FormattedText, Commentary                                     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Render Layer                              │
│                        (src/render/)                             │
│  - PDF document generation (printpdf)                            │
│  - Hand diagram rendering                                        │
│  - Bidding table rendering                                       │
│  - Commentary text rendering                                     │
│  - Layout calculations                                           │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Config Layer                              │
│                        (src/config/)                             │
│  - Default values                                                │
│  - Settings from CLI + PBN metadata                              │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs              # Entry point, orchestration
├── lib.rs               # Library exports
├── error.rs             # Error types (PbnError, RenderError)
│
├── bin/
│   └── layout_debug.rs  # Debug utility for layout testing
│
├── cli/
│   ├── mod.rs
│   └── args.rs          # CLI argument definitions (clap)
│
├── parser/
│   ├── mod.rs
│   ├── pbn.rs           # Main PBN file parser
│   ├── tags.rs          # [Name "Value"] tag parsing
│   ├── deal.rs          # N:AKQ.JT9.876.5432 parsing
│   ├── auction.rs       # Bidding sequence parsing
│   ├── play.rs          # Play sequence parsing
│   ├── commentary.rs    # {<b>text</b> \S} parsing
│   └── header.rs        # % directive parsing (incl. BCOptions)
│
├── model/
│   ├── mod.rs
│   ├── card.rs          # Suit, Rank, Card
│   ├── hand.rs          # Holding, Hand
│   ├── deal.rs          # Deal, Direction
│   ├── auction.rs       # Call, Auction, Contract
│   ├── play.rs          # Trick, PlaySequence
│   ├── commentary.rs    # TextSpan, FormattedText
│   ├── board.rs         # Board, Vulnerability
│   └── metadata.rs      # PbnMetadata, LayoutSettings, FontSettings
│
├── render/
│   ├── mod.rs
│   ├── document.rs      # PDF document orchestration
│   ├── page.rs          # Page management
│   ├── fonts.rs         # Font loading (DejaVu Sans, TeX Gyre Termes)
│   ├── colors.rs        # Suit color definitions
│   ├── layout.rs        # Position calculations
│   ├── text_metrics.rs  # Text measurement with rustybuzz
│   ├── hand_diagram.rs  # Compass-rose hand rendering
│   ├── bidding_table.rs # Auction table rendering
│   └── commentary.rs    # Formatted text rendering with justification
│
└── config/
    ├── mod.rs
    ├── defaults.rs      # Default layout values
    └── settings.rs      # Runtime configuration
```

## Key Data Structures

### Card Representation

```rust
enum Suit { Spades, Hearts, Diamonds, Clubs }
enum Rank { Ace, King, Queen, Jack, Ten, Nine, ..., Two }
struct Card { suit: Suit, rank: Rank }
```

### Hand and Deal

```rust
struct Holding { ranks: BTreeSet<Rank> }  // Cards in one suit
struct Hand { spades, hearts, diamonds, clubs: Holding }
struct Deal { north, east, south, west: Hand }
enum Direction { North, East, South, West }
```

### Auction

```rust
enum BidSuit { Clubs, Diamonds, Hearts, Spades, NoTrump }
enum Call { Pass, Bid { level: u8, suit: BidSuit }, Double, Redouble }
struct Auction { dealer: Direction, calls: Vec<AnnotatedCall> }
struct Contract { level: u8, suit: BidSuit, doubled: bool, redoubled: bool, declarer: Direction }
```

### Commentary

```rust
enum TextSpan {
    Plain(String),
    Bold(String),
    Italic(String),
    SuitSymbol(Suit),
    CardRef { suit: Suit, rank: Rank },
    LineBreak,
}
struct FormattedText { spans: Vec<TextSpan> }
```

### Board (Complete Record)

```rust
struct Board {
    number: Option<u32>,
    dealer: Option<Direction>,
    vulnerable: Vulnerability,
    deal: Deal,
    auction: Option<Auction>,
    contract: Option<Contract>,
    declarer: Option<Direction>,
    play: Option<PlaySequence>,
    commentary: Vec<CommentaryBlock>,
}
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `printpdf` | Low-level PDF generation |
| `nom` | Parser combinator framework for PBN parsing |
| `thiserror` | Error type definitions |
| `anyhow` | Application-level error handling |
| `log` + `env_logger` | Logging |
| `rustybuzz` | Text shaping and measurement for accurate layout |

## PBN Format Reference

### Deal Notation

```
[Deal "N:AKQ.JT9.876.5432 QJ.AK.QT9.87654 ..."]
```

Format: `FirstPlayer:Hand1 Hand2 Hand3 Hand4`

Each hand: `Spades.Hearts.Diamonds.Clubs`

Cards: A, K, Q, J, T (ten), 9-2

### Auction Notation

```
[Auction "N"]
1D 1S X Pass
1NT AP
```

- Bids: `1C`, `2H`, `3NT`, etc.
- Pass: `Pass` or `P`
- Double: `X`
- Redouble: `XX`
- All Pass: `AP` (adds 3 passes)

### Commentary Formatting

```
{<b>Bidding.</b> Open 1\D with this hand. Partner responds 1\S...}
```

- `<b>...</b>` - Bold text
- `<i>...</i>` - Italic text
- `\S`, `\H`, `\D`, `\C` - Suit symbols
- `\SQ`, `\HA` - Card references (♠Q, ♥A)

### Bridge Composer Header Directives

```
% PBN 2.1
%Creator: BridgeComposer Version 5.109
%BoardsPerPage fit,1
%Margins 1000,1000,500,750
%PipColors #000000,#ff0000,#ff0000,#000000
%Font:CardTable "Arial",11,400,0
%BCOptions Float Justify ShowHCP
```

- `BCOptions` flags: `Float` (floating commentary), `Justify` (full justification), `ShowHCP`

## Layout Design

### Compass-Rose Hand Diagram

```
           NORTH
         ♠ A K Q
         ♥ J T 9
         ♦ 8 7 6
         ♣ 5 4 3 2

WEST       N       EAST
♠ x x    W   E    ♠ x x
♥ x x      S      ♥ x x
♦ x x             ♦ x x
♣ x x             ♣ x x

           SOUTH
         ♠ x x
         ♥ x x
         ♦ x x
         ♣ x x
```

### Bidding Table

```
   W    N    E    S
        1♦   1♠   X
  Pass 1NT  Pass Pass
  Pass
```

## Embedded Fonts

The tool embeds two font families for consistent rendering across platforms:

- **DejaVu Sans** - Sans-serif font with full Unicode suit symbol support (♠♥♦♣)
- **TeX Gyre Termes** - High-quality Times New Roman clone for professional serif typography

Font selection is automatic based on PBN font specifications (Arial → DejaVu Sans, Times → TeX Gyre Termes).

## Future Enhancements

1. ~~**Font Embedding** - Embed custom fonts for better Unicode support~~ ✓ Done
2. **Custom Themes** - Allow color scheme customization
3. **Play Diagram** - Visual trick-by-trick play display
4. **Hand Records** - Generate multi-page hand record sheets
5. **Duplicate Scoring** - Calculate and display match point scores
