# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`pbn-to-pdf` is a Rust CLI tool that converts PBN (Portable Bridge Notation) files to PDF documents with Bridge Composer-style formatting. It produces professional-quality bridge hand diagrams suitable for teaching materials and publications.

## Build and Test Commands

```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Run with a PBN file
cargo run -- path/to/file.pbn

# Run with output path
cargo run -- input.pbn -o output.pdf

# Check for clippy warnings
cargo clippy

# Format code
cargo fmt
```

## Architecture

The codebase follows a layered architecture:

```
CLI (src/cli/) → Parser (src/parser/) → Model (src/model/) → Render (src/render/)
                                                                    ↓
                                                            Config (src/config/)
```

### Key Modules

- **src/parser/** - PBN file parsing using nom combinators
  - `pbn.rs` - Main file parser
  - `deal.rs` - Hand distribution parsing (N:AKQ.JT9.876.5432)
  - `auction.rs` - Bidding sequence parsing
  - `commentary.rs` - Formatted text with `<b>`, `<i>`, `\S`, `\H`, `\D`, `\C` codes
  - `header.rs` - Bridge Composer `%` directives including BCOptions

- **src/model/** - Data structures for bridge concepts
  - `card.rs` - Suit, Rank, Card with Unicode symbols (♠♥♦♣)
  - `hand.rs` - Holding, Hand with HCP calculation
  - `auction.rs` - Call, Auction, Contract
  - `board.rs` - Complete game record

- **src/render/** - PDF generation using printpdf
  - `document.rs` - PDF orchestration and board layout
  - `hand_diagram.rs` - Compass-rose hand rendering
  - `bidding_table.rs` - Auction table in W/N/E/S columns
  - `commentary.rs` - Text rendering with justification and floating layout
  - `fonts.rs` - Embedded fonts (DejaVu Sans, TeX Gyre Termes)
  - `text_metrics.rs` - Text measurement with rustybuzz

- **src/config/** - Runtime settings from CLI args and PBN metadata

## Embedded Fonts

Two font families are embedded for cross-platform consistency:
- **DejaVu Sans** - Sans-serif with Unicode suit symbols
- **TeX Gyre Termes** - Serif (Times New Roman clone) for professional typography

Font selection is automatic based on PBN font specifications.

## PBN Format Notes

Key PBN elements the parser handles:
- Tag pairs: `[Name "Value"]`
- Deal notation: `N:AKQ.JT9.876.5432 ...` (Spades.Hearts.Diamonds.Clubs)
- Auction: `1D 1S X Pass 1NT AP`
- Commentary: `{<b>Bidding.</b> Open 1\D with this hand...}`
- Header directives: `%BCOptions Float Justify ShowHCP`

## Code Style

- Use rustfmt for formatting
- Run clippy before committing
- Prefer editing existing files over creating new ones
- Keep functions focused and reasonably sized
- Use descriptive variable names for bridge concepts
