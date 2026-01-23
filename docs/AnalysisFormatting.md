# Analysis Layout Formatting Reference

This document describes the formatting controls available for the analysis (two-column) layout in pbn-to-pdf.

## Special Board Names

pbn-to-pdf recognizes special board names that control page and column layout. These provide a simple way to force layout breaks without relying on pixel-perfect spacing.

### Column Break

A board with the name `column-break` (case-insensitive) forces the layout to move to the next column:

```
[Board "column-break"]
[Deal ""]
```

- In the left column: moves content to the right column
- In the right column: moves content to a new page (left column)
- No content is rendered for the board itself
- No separator line is drawn

**Legacy support:** The board name `spacer` is treated as `column-break` for backward compatibility with Bridge Composer workarounds.

### Page Break

A board with the name `page-break` (case-insensitive) forces the layout to start a new page:

```
[Board "page-break"]
[Deal ""]
```

- Immediately ends the current page
- The next board starts at the top of a new page (left column)
- No content is rendered for the board itself
- No separator line is drawn

### Example Usage

```pbn
[Board "1"]
[Deal "N:AKQ.JT9.876.5432 ..."]
{ Content for board 1 }

[Board "column-break"]
[Deal ""]

[Board "2"]
[Deal "N:JT9.AKQ.5432.876 ..."]
{ Content for board 2 - will start in right column }

[Board "page-break"]
[Deal ""]

[Board "3"]
[Deal "N:876.5432.AKQ.JT9 ..."]
{ Content for board 3 - will start on new page }
```

---

## BCFlags Tag Reference

The `[BCFlags]` tag is a Bridge Composer custom field that controls the display of various board elements. The value is a hexadecimal bitmask where each bit controls a specific display option.

### Implementation Status

The following table shows which BCFlags are currently implemented in pbn-to-pdf:

| Flag | Status | Notes |
|------|--------|-------|
| Show Diagram (0x08) | ✅ Implemented | Controls hand diagram visibility |
| Show Auction (0x10) | ✅ Implemented | Controls bidding table visibility |
| Show Event Commentary (0x20) | ✅ Implemented | Controls commentary visibility |
| Show Final Commentary (0x04) | ✅ Implemented | Controls commentary visibility |
| Hide Board (0x100000) | ✅ Implemented | Hides "Deal N" text |
| Hide Dealer (0x200000) | ✅ Implemented | Hides "X Deals" text |
| Hide Vulnerable (0x400000) | ✅ Implemented | Hides vulnerability text |
| Page break (0x4000000) | 🔲 Planned | Use `[Board "page-break"]` instead |
| Column break (0x2000000) | 🔲 Planned | Use `[Board "column-break"]` instead |
| All other flags | 🔲 Future | Not yet implemented |

### Format

```
[BCFlags "XXXXXXXX"]
```

Where `XXXXXXXX` is a hexadecimal value (e.g., `"600023"`, `"17"`, `"60001b"`).

### Bit Definitions

| Bit Value    | Hex      | Description                                      |
|--------------|----------|--------------------------------------------------|
| 0x00000001   | 1        | Show the Play section                            |
| 0x00000002   | 2        | Show the Results section                         |
| 0x00000004   | 4        | Show the Final Commentary                        |
| 0x00000008   | 8        | Show the Diagram section                         |
| 0x00000010   | 10       | Show the Auction section                         |
| 0x00000020   | 20       | Show the Event Commentary                        |
| 0x00000040   | 40       | Show the Diagram Commentary                      |
| 0x00000080   | 80       | Show the Auction Commentary                      |
| 0x00000400   | 400      | Board is an Import Score Data recap listing      |
| 0x00000800   | 800      | "View→Slide Highlighting" flag                   |
| 0x00001000   | 1000     | Event Commentary preformatted flag               |
| 0x00002000   | 2000     | Diagram Commentary preformatted flag             |
| 0x00004000   | 4000     | Auction Commentary preformatted flag             |
| 0x00008000   | 8000     | Final Commentary preformatted flag               |
| 0x00010000   | 10000    | Single suit show one pip flag                    |
| 0x00020000   | 20000    | Single suit show all pips flag                   |
| 0x00040000   | 40000    | Event Commentary is an Import Score Data recap   |
| 0x00080000   | 80000    | Double-dummy data has been verified              |
| 0x00100000   | 100000   | Hide the Board field                             |
| 0x00200000   | 200000   | Hide the Dealer field                            |
| 0x00400000   | 400000   | Hide the Vulnerable field                        |
| 0x00800000   | 800000   | Hide the Total Score Table                       |
| 0x02000000   | 2000000  | Column break before                              |
| 0x04000000   | 4000000  | Page break before                                |
| 0x08000000   | 8000000  | Hide the Contract and Declarer fields            |
| 0x10000000   | 10000000 | Hide "Notes"                                     |
| 0x20000000   | 20000000 | Show space for hidden hands                      |

### Common Values

#### `17` (0x00000017)
- Show Play (0x01)
- Show Results (0x02)
- Show Final Commentary (0x04)
- Show Auction (0x10)

Typically used for commentary-only boards with no diagram.

#### `60001b` (0x0060001B)
- Show Play (0x01)
- Show Results (0x02)
- Show Diagram (0x08)
- Show Auction (0x10)
- Hide Board field (0x100000)
- Hide Dealer field (0x200000)
- Hide Vulnerable field (0x400000)

Used for sub-boards (e.g., "1-1", "1-2") in exercise layouts where the main board shows the header info.

#### `600023` (0x00600023)
- Show Play (0x01)
- Show Results (0x02)
- Show Event Commentary (0x20)
- Hide Board field (0x100000)
- Hide Dealer field (0x200000)
- Hide Vulnerable field (0x400000)

Used for exercise header boards that contain instructional text.

#### `60000b` (0x0060000B)
- Show Play (0x01)
- Show Results (0x02)
- Show Diagram (0x08)
- Hide Board field (0x100000)
- Hide Dealer field (0x200000)
- Hide Vulnerable field (0x400000)

Used for boards showing only the diagram without auction.

---

## Implied Hiding Rules

In addition to explicit BCFlags, pbn-to-pdf applies implicit hiding rules based on deal content. These rules are applied in `render/layouts/analysis.rs` before rendering.

### Empty Deal Hides Board Metadata

When `deal.is_empty()` returns true (no cards in any suit across all hands), the following are automatically hidden regardless of BCFlags:

| Element | Condition |
|---------|-----------|
| Board number | Hidden when deal is empty |
| Dealer line | Hidden when deal is empty |
| Vulnerability | Hidden when deal is empty |
| Diagram | Hidden when deal is empty |

### Only North Visible Hides Compass

When the `[Hidden]` PBN tag specifies that East, South, and West are hidden (e.g., `[Hidden "ESW"]`), leaving only North visible:

- The compass box is not rendered
- North's cards are centered where the compass would have been
- Cards appear on the same line as the board title

### Single Suit Fragment Hides Suit Symbol

When only one suit has cards across the entire deal (a "fragment"):

- The suit symbol prefix is not shown (e.g., just "3" instead of "♠ 3")
- This provides cleaner display for answer keys showing single cards

### Visibility Decision Flow

The final visibility for each element combines BCFlags with implied rules:

```
show_board = !deal_is_empty AND !BCFlags.hide_board()
show_dealer = !deal_is_empty AND !BCFlags.hide_dealer()
show_vulnerable = !deal_is_empty AND !BCFlags.hide_vulnerable()
show_diagram = !deal_is_empty AND BCFlags.show_diagram()
show_auction = BCFlags.show_auction() AND settings.show_bidding
show_commentary = settings.show_commentary AND has_commentary AND BCFlags.show_*_commentary()
```

---

## Implementation Notes

When rendering boards, check the following in order:

1. **Special board names**: Check for `column-break`, `page-break`, or `spacer` first.

2. **What to show**: Use the "Show" flags (bits 0-7) to determine which sections to render.

3. **What to hide**: Use the "Hide" flags (bits 20-28) to suppress specific header fields.

4. **Preformatted text**: Bits 12-15 indicate if commentary should preserve whitespace formatting.

5. **Implied rules**: Apply empty-deal and hidden-hands rules before BCFlags checks.
