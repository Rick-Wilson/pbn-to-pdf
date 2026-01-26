//! PDF compression helper
//!
//! Uses lopdf to compress PDF streams after printpdf generates uncompressed output.

use std::io::Cursor;

/// Compress PDF streams to reduce file size.
///
/// This is a post-processing step needed because printpdf doesn't compress
/// its output streams. We parse the PDF bytes with lopdf, compress all
/// streams, and re-save.
pub fn compress_pdf(uncompressed: Vec<u8>) -> Result<Vec<u8>, String> {
    // Parse the uncompressed PDF
    let mut doc = lopdf::Document::load_mem(&uncompressed)
        .map_err(|e| format!("Failed to parse PDF for compression: {}", e))?;

    // Compress all streams
    doc.compress();

    // Save to bytes
    let mut output = Cursor::new(Vec::new());
    doc.save_to(&mut output)
        .map_err(|e| format!("Failed to save compressed PDF: {}", e))?;

    Ok(output.into_inner())
}
