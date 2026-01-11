use std::env;
use std::fs;
use std::path::Path;
use subsetter::GlyphRemapper;

/// Characters needed for bridge documents:
/// - ASCII printable (space through tilde) for commentary text
/// - Unicode suit symbols (♠♥♦♣)
/// - Common punctuation and special characters
const GLYPH_SET: &str = concat!(
    // ASCII printable characters (space through tilde)
    " !\"#$%&'()*+,-./0123456789:;<=>?@",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`",
    "abcdefghijklmnopqrstuvwxyz{|}~",
    // Unicode suit symbols
    "\u{2660}\u{2665}\u{2666}\u{2663}", // ♠♥♦♣
    // Extended characters that might appear in bridge commentary
    "\u{2014}\u{2013}\u{2018}\u{2019}\u{201C}\u{201D}\u{2026}\u{00B0}\u{00D7}\u{00F7}\u{00B1}", // —–''""…°×÷±
    // Common accented characters for international names
    "\u{00E0}\u{00E1}\u{00E2}\u{00E3}\u{00E4}\u{00E5}\u{00E6}\u{00E7}", // àáâãäåæç
    "\u{00E8}\u{00E9}\u{00EA}\u{00EB}\u{00EC}\u{00ED}\u{00EE}\u{00EF}", // èéêëìíîï
    "\u{00F1}\u{00F2}\u{00F3}\u{00F4}\u{00F5}\u{00F6}\u{00F9}\u{00FA}\u{00FB}\u{00FC}\u{00FD}\u{00FF}", // ñòóôõöùúûüýÿ
    "\u{00C0}\u{00C1}\u{00C2}\u{00C3}\u{00C4}\u{00C5}\u{00C6}\u{00C7}", // ÀÁÂÃÄÅÆÇ
    "\u{00C8}\u{00C9}\u{00CA}\u{00CB}\u{00CC}\u{00CD}\u{00CE}\u{00CF}", // ÈÉÊËÌÍÎÏ
    "\u{00D1}\u{00D2}\u{00D3}\u{00D4}\u{00D5}\u{00D6}\u{00D9}\u{00DA}\u{00DB}\u{00DC}\u{00DD}", // ÑÒÓÔÕÖÙÚÛÜÝ
);

fn subset_font(input_path: &Path, output_path: &Path, chars: &str) -> Result<(), String> {
    let font_data = fs::read(input_path)
        .map_err(|e| format!("Failed to read font {}: {}", input_path.display(), e))?;

    // Parse the font to get glyph IDs for our characters
    let face = rustybuzz::Face::from_slice(&font_data, 0)
        .ok_or_else(|| format!("Failed to parse font: {}", input_path.display()))?;

    let mut remapper = GlyphRemapper::new();

    // Always include glyph 0 (notdef)
    remapper.remap(0);

    // Map each character to its glyph ID and add to remapper
    for c in chars.chars() {
        if let Some(glyph_id) = face.glyph_index(c) {
            remapper.remap(glyph_id.0);
        }
    }

    // Perform the subsetting
    let subsetted = subsetter::subset(&font_data, 0, &remapper)
        .map_err(|e| format!("Failed to subset font {}: {:?}", input_path.display(), e))?;

    fs::write(output_path, &subsetted).map_err(|e| {
        format!(
            "Failed to write subsetted font {}: {}",
            output_path.display(),
            e
        )
    })?;

    Ok(())
}

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    let fonts = [
        ("assets/fonts/DejaVuSans.ttf", "DejaVuSans-subset.ttf"),
        (
            "assets/fonts/DejaVuSans-Bold.ttf",
            "DejaVuSans-Bold-subset.ttf",
        ),
        (
            "assets/fonts/DejaVuSans-Oblique.ttf",
            "DejaVuSans-Oblique-subset.ttf",
        ),
        (
            "assets/fonts/texgyretermes-regular.ttf",
            "texgyretermes-regular-subset.ttf",
        ),
        (
            "assets/fonts/texgyretermes-bold.ttf",
            "texgyretermes-bold-subset.ttf",
        ),
        (
            "assets/fonts/texgyretermes-italic.ttf",
            "texgyretermes-italic-subset.ttf",
        ),
        (
            "assets/fonts/texgyretermes-bolditalic.ttf",
            "texgyretermes-bolditalic-subset.ttf",
        ),
    ];

    for (input, output) in &fonts {
        let input_path = Path::new(input);
        let output_path = out_path.join(output);

        if let Err(e) = subset_font(input_path, &output_path, GLYPH_SET) {
            panic!("Font subsetting failed: {}", e);
        }

        // Tell cargo to rerun if the source font changes
        println!("cargo:rerun-if-changed={}", input);
    }
}
