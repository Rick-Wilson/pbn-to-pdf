//! Compatibility layer for printpdf 0.8
//!
//! This module provides a `LayerBuilder` that mimics the old `PdfLayerReference` API
//! but collects operations into a `Vec<Op>` for the new printpdf 0.8 API.

use printpdf::{
    Color, CurTransMat, FontId, LinePoint, Mm, Op, PaintMode, PdfFontHandle, Point, Polygon,
    PolygonRing, Pt, TextItem, WindingOrder, XObjectId, XObjectTransform,
};

/// A builder that collects PDF operations
///
/// This mimics the old `PdfLayerReference` API from printpdf 0.7
/// but internally builds a `Vec<Op>` for printpdf 0.8
#[derive(Default)]
pub struct LayerBuilder {
    ops: Vec<Op>,
}

impl LayerBuilder {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    /// Get the collected operations
    pub fn into_ops(self) -> Vec<Op> {
        self.ops
    }

    /// Get a reference to the operations (for inspection)
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// Extend with operations from another builder
    pub fn extend(&mut self, other: LayerBuilder) {
        self.ops.extend(other.ops);
    }

    /// Set the fill color
    pub fn set_fill_color(&mut self, color: Color) {
        self.ops.push(Op::SetFillColor { col: color });
    }

    /// Set the outline/stroke color
    pub fn set_outline_color(&mut self, color: Color) {
        self.ops.push(Op::SetOutlineColor { col: color });
    }

    /// Set the outline thickness
    pub fn set_outline_thickness(&mut self, thickness: f32) {
        self.ops.push(Op::SetOutlineThickness { pt: Pt(thickness) });
    }

    /// Draw text at a specific position
    ///
    /// This mimics the old `layer.use_text()` API
    pub fn use_text<S: Into<String>>(
        &mut self,
        text: S,
        font_size: f32,
        x: Mm,
        y: Mm,
        font: &FontId,
    ) {
        let text_str = text.into();
        if text_str.is_empty() {
            return;
        }

        self.ops.push(Op::StartTextSection);
        self.ops.push(Op::SetTextCursor {
            pos: Point {
                x: x.into(),
                y: y.into(),
            },
        });
        self.ops.push(Op::SetFont {
            size: Pt(font_size),
            font: PdfFontHandle::External(font.clone()),
        });
        self.ops.push(Op::ShowText {
            items: vec![TextItem::Text(text_str)],
        });
        self.ops.push(Op::EndTextSection);
    }

    /// Add a filled or stroked rectangle
    ///
    /// Takes lower-left x, y and upper-right x, y coordinates with a paint mode
    pub fn add_rect(&mut self, x1: Mm, y1: Mm, x2: Mm, y2: Mm, mode: PaintMode) {
        let ll = Point {
            x: x1.into(),
            y: y1.into(),
        };
        let lr = Point {
            x: x2.into(),
            y: y1.into(),
        };
        let ur = Point {
            x: x2.into(),
            y: y2.into(),
        };
        let ul = Point {
            x: x1.into(),
            y: y2.into(),
        };

        let points = vec![
            LinePoint {
                p: ll,
                bezier: false,
            },
            LinePoint {
                p: lr,
                bezier: false,
            },
            LinePoint {
                p: ur,
                bezier: false,
            },
            LinePoint {
                p: ul,
                bezier: false,
            },
        ];

        let polygon = Polygon {
            rings: vec![PolygonRing { points }],
            mode,
            winding_order: WindingOrder::NonZero,
        };

        self.ops.push(Op::DrawPolygon { polygon });
    }

    /// Save graphics state
    pub fn save_graphics_state(&mut self) {
        self.ops.push(Op::SaveGraphicsState);
    }

    /// Restore graphics state
    pub fn restore_graphics_state(&mut self) {
        self.ops.push(Op::RestoreGraphicsState);
    }

    /// Set the current transformation matrix
    ///
    /// This applies a transformation to all subsequent drawing operations
    /// until the graphics state is restored.
    pub fn set_transform(&mut self, matrix: CurTransMat) {
        self.ops.push(Op::SetTransformationMatrix { matrix });
    }

    /// Draw a line from (x1, y1) to (x2, y2)
    pub fn add_line(&mut self, x1: Mm, y1: Mm, x2: Mm, y2: Mm) {
        let points = vec![
            LinePoint {
                p: Point {
                    x: x1.into(),
                    y: y1.into(),
                },
                bezier: false,
            },
            LinePoint {
                p: Point {
                    x: x2.into(),
                    y: y2.into(),
                },
                bezier: false,
            },
        ];

        let polygon = Polygon {
            rings: vec![PolygonRing { points }],
            mode: PaintMode::Stroke,
            winding_order: WindingOrder::NonZero,
        };

        self.ops.push(Op::DrawPolygon { polygon });
    }

    /// Place an XObject (SVG/image) with the given transform
    ///
    /// The transform specifies position, scale, rotation, etc.
    /// Use `PdfDocument::add_xobject()` to register an SVG and get the XObjectId.
    pub fn use_xobject(&mut self, id: XObjectId, transform: XObjectTransform) {
        self.ops.push(Op::UseXobject { id, transform });
    }

    /// Begin a rectangular clipping region
    ///
    /// All drawing operations after this call will be clipped to the specified rectangle.
    /// Call `end_clip()` to restore the previous graphics state and end clipping.
    pub fn begin_clip_rect(&mut self, x: Mm, y: Mm, width: Mm, height: Mm) {
        self.save_graphics_state();

        // Create rectangular clipping path
        let points = vec![
            LinePoint {
                p: Point {
                    x: x.into(),
                    y: y.into(),
                },
                bezier: false,
            },
            LinePoint {
                p: Point {
                    x: (Mm(x.0 + width.0)).into(),
                    y: y.into(),
                },
                bezier: false,
            },
            LinePoint {
                p: Point {
                    x: (Mm(x.0 + width.0)).into(),
                    y: (Mm(y.0 + height.0)).into(),
                },
                bezier: false,
            },
            LinePoint {
                p: Point {
                    x: x.into(),
                    y: (Mm(y.0 + height.0)).into(),
                },
                bezier: false,
            },
        ];

        let polygon = Polygon {
            rings: vec![PolygonRing { points }],
            mode: PaintMode::Clip,
            winding_order: WindingOrder::NonZero,
        };

        self.ops.push(Op::DrawPolygon { polygon });
    }

    /// End a clipping region
    ///
    /// Restores the graphics state to before `begin_clip_rect()` was called.
    pub fn end_clip(&mut self) {
        self.restore_graphics_state();
    }
}
