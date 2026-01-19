# Roadmap

Future enhancements planned for pbn-to-pdf.

## Completed Features

- [x] Hand record layout (analysis layout)
- [x] Bidding sheets layout
- [x] Reduce PDF file size via font subsetting (implemented: 3.1 MB → 167 KB using runtime subsetting with `subsetter` crate)

## In Progress

- [ ] Only remove "(Practice Page)" from headers when title is long (currently always removed when title present)

## Planned Features

### Layouts
- [ ] Add Dealer Summary layout
- [ ] Add Winners and Losers layout
- [ ] Add two-column layout

### Bidding Sheets Enhancements
- [ ] Add pre-rotation to N/S for bidding sheets (rotate hands so practicing player is always South)

### Distribution & Deployment
- [ ] Add GitHub Actions to generate binaries for Windows and Linux
- [ ] Add auto-update mechanism

### Web Interface
- [ ] Add web interface, hosted on Harmonicsystems.com

### Other
- [ ] Append PBN to PDF
- [ ] Compare page sizes with Bridge Composer-generated PDF
- [ ] Compare page sizes with Windows-printed PDF
- [ ] Add optional double-dummy panel
