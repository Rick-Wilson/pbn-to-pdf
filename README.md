# pbn-to-pdf

A Rust CLI tool that converts PBN (Portable Bridge Notation) files to PDF with Bridge Composer-style formatting.

## Features

- Full table layout with 4 hands arranged around a compass rose
- Unicode suit symbols (♠♥♦♣) with red/black coloring
- Bidding table with West/North/East/South columns
- Commentary text with formatting (bold, italic, inline suit symbols)
- HCP (High Card Points) display for each hand
- Configurable page layout (1, 2, or 4 boards per page)
- Support for Letter, A4, and Legal paper sizes

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/pbn-to-pdf`.

## Usage

```
pbn-to-pdf [OPTIONS] <INPUT>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Input PBN file path (required) |

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <OUTPUT>` | Output PDF file path (defaults to input with .pdf extension) |
| `-l, --layout <LAYOUT>` | Output layout style: analysis, bidding-sheets (default: analysis) |
| `-n, --boards-per-page <N>` | Number of boards per page: 1, 2, or 4 (default: 1) |
| `-s, --page-size <SIZE>` | Page size: letter, a4, legal (default: letter) |
| `--orientation <O>` | Page orientation: portrait, landscape (default: portrait) |
| `-m, --margins <PRESET>` | Page margins: narrow (1/4"), standard (1/2"), wide (1") |
| `--no-bidding` | Hide bidding table |
| `--no-play` | Hide play sequence |
| `--no-commentary` | Hide commentary text |
| `--no-hcp` | Hide HCP point counts |
| `-b, --boards <RANGE>` | Board range to include (e.g., "1-16" or "5,8,12") |
| `-t, --title [TITLE]` | Title for bidding sheets banner (overrides %HRTitleEvent; use with no value to hide) |
| `--debug-boxes` | Draw debug boxes around layout regions |
| `-v, --verbose` | Increase verbosity (-v, -vv, -vvv) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

### Examples

```bash
# Basic conversion (creates input.pdf)
pbn-to-pdf hands.pbn

# Specify output file
pbn-to-pdf hands.pbn -o output.pdf

# Two boards per page on A4 paper
pbn-to-pdf hands.pbn -n 2 -s a4

# Landscape orientation, 4 boards per page
pbn-to-pdf hands.pbn -n 4 --orientation landscape

# Only boards 1-8, no commentary
pbn-to-pdf hands.pbn -b 1-8 --no-commentary

# Specific boards only
pbn-to-pdf hands.pbn -b "1,5,9,13"

# Verbose output for debugging
pbn-to-pdf hands.pbn -vv

# Generate bidding practice sheets
pbn-to-pdf hands.pbn -l bidding-sheets -o practice.pdf

# Bidding sheets with wide margins
pbn-to-pdf hands.pbn -l bidding-sheets -m wide

# Bidding sheets with custom title
pbn-to-pdf hands.pbn -l bidding-sheets -t "My Practice Session"

# Bidding sheets with no title
pbn-to-pdf hands.pbn -l bidding-sheets -t
```

## PBN Format Support

The tool supports PBN 2.1 format including:

- Standard tags: `[Event]`, `[Board]`, `[Dealer]`, `[Vulnerable]`, `[Deal]`, etc.
- Auction section with bids, doubles, redoubles, and "AP" (All Pass)
- Play section with card notation
- Commentary in braces `{...}` with formatting:
  - `<b>Bold text</b>`
  - `<i>Italic text</i>`
  - `\S` `\H` `\D` `\C` for suit symbols
  - `\SQ` `\HA` etc. for card references
- Bridge Composer header directives (`%BoardsPerPage`, `%Margins`, `%PipColors`, etc.)

## License

This project is released under the Unlicense (public domain).
