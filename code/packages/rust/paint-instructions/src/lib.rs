//! # paint-instructions
//!
//! Universal 2D paint intermediate representation (IR) for Rust.
//!
//! This crate is the Rust counterpart of the TypeScript `@coding-adventures/paint-instructions`
//! package. It defines the complete type system for the P2D00 spec: the shared vocabulary
//! between scene producers (barcodes, charts, diagrams) and rendering backends (Metal,
//! SVG, terminal).
//!
//! ## Architecture
//!
//! ```text
//! Producer (barcode, chart, mermaid diagram)
//!   → PaintScene / PaintInstruction          ← this crate
//!   → PaintVM (P2D01, paint-metal etc.)
//!   → Backend output (PixelContainer, SVG string, window frame)
//!
//! PixelContainer
//!   → ImageCodec::encode()                   ← paint-codec-png / paint-codec-webp
//!   → PNG / WebP / JPEG bytes
//! ```
//!
//! ## Design principles
//!
//! - **Pure types** — no rendering logic lives here. Zero runtime dependencies.
//! - **Acyclic** — no VM imports from a codec, no codec imports from a VM.
//!   This crate is the shared contract both sides depend on.
//! - **Composable** — every type is designed to snap into a pipeline step.
//!
//! ## Pipeline example
//!
//! ```text
//! let scene = barcode_2d::layout(&grid, &config);   // → PaintScene
//! let pixels = paint_metal::render(&scene);           // PaintScene → PixelContainer
//! let png_bytes = paint_codec_png::encode(&pixels);  // PixelContainer → Vec<u8>
//! std::fs::write("qr.png", png_bytes).unwrap();
//! ```

pub const VERSION: &str = "0.1.0";

// ============================================================================
// PixelContainer and ImageCodec — re-exported from pixel-container (IC00)
// ============================================================================
//
// These types are defined in the standalone `pixel-container` crate so that
// image codecs (BMP, PPM, QOI, PNG, JPEG…) can depend only on that crate
// without pulling in the full paint IR. Re-exporting them here preserves the
// existing `paint_instructions::{PixelContainer, ImageCodec}` import path so
// all downstream crates (paint-codec-png, paint-metal, etc.) compile unchanged.

pub use pixel_container::{ImageCodec, PixelContainer};

// ============================================================================
// PathCommand — pen-plotter commands inside a PaintPath
// ============================================================================

/// A single drawing command inside a [`PaintPath`].
///
/// Think of it as one step for a pen plotter:
///
/// ```text
/// MoveTo  → lift pen and move without drawing
/// LineTo  → draw a straight line
/// QuadTo  → draw a quadratic Bézier curve (one control point)
/// CubicTo → draw a cubic Bézier curve (two control points)
/// ArcTo   → draw an elliptical arc (SVG arc semantics)
/// Close   → straight line back to the last MoveTo, closes the subpath
/// ```
///
/// ## Example — house outline
///
/// ```text
/// [MoveTo(60,120), LineTo(60,60), LineTo(100,20), LineTo(140,60), LineTo(140,120), Close]
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum PathCommand {
    /// Lift pen and move to `(x, y)` without drawing.
    MoveTo { x: f64, y: f64 },
    /// Draw a straight line to `(x, y)`.
    LineTo { x: f64, y: f64 },
    /// Draw a quadratic Bézier curve.  `(cx, cy)` is the control point.
    QuadTo { cx: f64, cy: f64, x: f64, y: f64 },
    /// Draw a cubic Bézier curve.  `(cx1, cy1)` and `(cx2, cy2)` are the two control points.
    CubicTo {
        cx1: f64,
        cy1: f64,
        cx2: f64,
        cy2: f64,
        x: f64,
        y: f64,
    },
    /// Draw an elliptical arc with SVG arc semantics.
    ArcTo {
        rx: f64,
        ry: f64,
        /// X-axis rotation in degrees.
        x_rotation: f64,
        large_arc: bool,
        /// `true` = clockwise, `false` = counter-clockwise.
        sweep: bool,
        x: f64,
        y: f64,
    },
    /// Straight line back to the last `MoveTo` point; closes the subpath.
    Close,
}

// ============================================================================
// Transform2D — affine transform matrix
// ============================================================================

/// A 2D affine transform as six `f64` values `[a, b, c, d, e, f]`.
///
/// The 3×3 homogeneous matrix is:
///
/// ```text
/// | a  c  e |
/// | b  d  f |
/// | 0  0  1 |
/// ```
///
/// Maps `(x, y)` to:
///
/// ```text
/// x' = a*x + c*y + e
/// y' = b*x + d*y + f
/// ```
///
/// Common transforms:
///
/// ```text
/// Identity:     [1, 0, 0, 1, 0, 0]
/// Translate tx,ty:  [1, 0, 0, 1, tx, ty]
/// Scale sx,sy:      [sx, 0, 0, sy, 0, 0]
/// Rotate θ:     [cos θ, sin θ, -sin θ, cos θ, 0, 0]
/// ```
///
/// This matches `CanvasRenderingContext2D.transform(a,b,c,d,e,f)` argument order.
pub type Transform2D = [f64; 6];

/// The identity transform — no scaling, rotation, or translation.
pub const IDENTITY_TRANSFORM: Transform2D = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

// ============================================================================
// BlendMode
// ============================================================================

/// How a `PaintLayer`'s offscreen buffer is composited back into the parent surface.
///
/// Separable modes (operate per colour channel independently):
///
/// | Mode         | Effect                                    |
/// |--------------|-------------------------------------------|
/// | Normal       | Standard alpha compositing (the default)  |
/// | Multiply     | Multiply src × dst. Darkens.              |
/// | Screen       | Invert, multiply, invert. Lightens.       |
/// | Overlay      | Multiply for darks, Screen for lights.    |
/// | Darken       | min(src, dst) per channel.                |
/// | Lighten      | max(src, dst) per channel.                |
/// | ColorDodge   | Divide dst by (1 − src). Brightens.       |
/// | ColorBurn    | Invert dst, divide by src, invert.        |
/// | HardLight    | Overlay with src/dst swapped.             |
/// | SoftLight    | Softer version of HardLight.              |
/// | Difference   | |src − dst|. High contrast at edges.      |
/// | Exclusion    | Like Difference but lower contrast.       |
///
/// Non-separable modes (operate on combined HSL representation):
///
/// | Mode         | Effect                                    |
/// |--------------|-------------------------------------------|
/// | Hue          | src hue + dst saturation + dst luminosity |
/// | Saturation   | dst hue + src saturation + dst luminosity |
/// | Color        | src hue + src saturation + dst luminosity |
/// | Luminosity   | dst hue + dst saturation + src luminosity |
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

// ============================================================================
// FilterEffect
// ============================================================================

/// An image filter effect applied to a [`PaintLayer`]'s composited buffer.
///
/// Filters are applied in array order — each filter receives the output of the
/// previous one. They operate on the full composited layer as a whole, after all
/// children have been rendered to the layer's offscreen buffer.
///
/// ## Why filters are on PaintLayer, not PaintGroup
///
/// A `PaintGroup` renders directly into the parent surface — there is no separate
/// buffer to filter. A `PaintLayer` allocates an offscreen buffer, renders children
/// into it, applies filters to the whole buffer, then composites the result.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterEffect {
    /// Gaussian blur. `radius` is in user-space units.
    Blur { radius: f64 },
    /// Drop shadow — offset by `(dx, dy)`, blurred by `blur` radius, filled with `color`.
    DropShadow {
        dx: f64,
        dy: f64,
        blur: f64,
        color: String,
    },
    /// 4×5 colour matrix (20 values, row-major).  Maps `[R,G,B,A,1]` to `[R',G',B',A']`.
    ColorMatrix { matrix: Vec<f64> },
    /// Multiply luminance. `1.0` = unchanged, `0.0` = black.
    Brightness { amount: f64 },
    /// Adjust contrast. `1.0` = unchanged, `0.0` = flat grey.
    Contrast { amount: f64 },
    /// Adjust saturation. `0.0` = greyscale, `1.0` = unchanged, `2.0` = vivid.
    Saturate { amount: f64 },
    /// Rotate hue by `angle` degrees.  `180` = complement.
    HueRotate { angle: f64 },
    /// Invert colours. `0.0` = no change, `1.0` = fully inverted.
    Invert { amount: f64 },
    /// Premultiplied opacity. `0.0` = transparent, `1.0` = opaque.
    Opacity { amount: f64 },
}

// ============================================================================
// PaintInstruction types
// ============================================================================

/// Shared optional fields on every instruction.
///
/// `id` — stable opaque identity used by a future `PaintVM::patch()` for
///   diffing scene versions. UUID v4 or short stable strings work well.
///
/// `metadata` — arbitrary key/value pairs carried through unchanged.
///   The VM ignores them; backends may expose them for dev-tools or accessibility.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintBase {
    pub id: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

// ─── PaintRect ────────────────────────────────────────────────────────────────

/// Filled and/or stroked rectangle.
///
/// `x`, `y` are the top-left corner in the current coordinate system.
/// `fill` and `stroke` use CSS colour syntax (named colours, hex, rgba).
/// A rect with neither fill nor stroke renders nothing visible.
/// `corner_radius` applies uniformly to all four corners.
///
/// ## Example — a blue card with a white rounded border
///
/// ```text
/// PaintRect { x: 10.0, y: 10.0, width: 200.0, height: 120.0,
///   fill: Some("#2563eb".into()), stroke: Some("#ffffff".into()),
///   stroke_width: Some(2.0), corner_radius: Some(8.0), .. }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PaintRect {
    pub base: PaintBase,
    pub x: f64,
    pub y: f64,
    /// Must be ≥ 0.
    pub width: f64,
    /// Must be ≥ 0.
    pub height: f64,
    /// CSS colour; `None` = no fill.
    pub fill: Option<String>,
    /// CSS colour; `None` = no stroke.
    pub stroke: Option<String>,
    /// User-space units; default 1.0.
    pub stroke_width: Option<f64>,
    /// 0.0 or `None` = sharp corners.
    pub corner_radius: Option<f64>,
    /// Dash pattern for the stroke, e.g. `[6.0, 3.0]` = 6px dash, 3px gap.
    /// `None` = solid stroke.
    pub stroke_dash: Option<Vec<f64>>,
    /// Phase offset into the dash pattern. `None` = 0.0.
    pub stroke_dash_offset: Option<f64>,
}

impl PaintRect {
    /// Convenience constructor for a filled rectangle with no stroke.
    pub fn filled(x: f64, y: f64, width: f64, height: f64, fill: &str) -> Self {
        Self {
            base: PaintBase::default(),
            x,
            y,
            width,
            height,
            fill: Some(fill.to_string()),
            stroke: None,
            stroke_width: None,
            corner_radius: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        }
    }
}

// ─── PaintEllipse ─────────────────────────────────────────────────────────────

/// Filled and/or stroked ellipse or circle.
///
/// `cx`, `cy` is the geometric center (not the bounding-box origin).
/// `rx`, `ry` are the x-radius and y-radius. A circle has `rx == ry`.
///
/// ```text
///        (cx, cy−ry)          ← top
///             │
/// (cx−rx, cy)─┼─(cx+rx, cy)  ← left and right extremes
///             │
///        (cx, cy+ry)          ← bottom
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PaintEllipse {
    pub base: PaintBase,
    /// Center x.
    pub cx: f64,
    /// Center y.
    pub cy: f64,
    /// X radius (half-width).
    pub rx: f64,
    /// Y radius (half-height).
    pub ry: f64,
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    /// Dash pattern for the stroke. `None` = solid.
    pub stroke_dash: Option<Vec<f64>>,
    /// Phase offset into the dash pattern. `None` = 0.0.
    pub stroke_dash_offset: Option<f64>,
}

// ─── PaintPath ────────────────────────────────────────────────────────────────

/// Arbitrary vector path built from [`PathCommand`]s.
///
/// This is the most expressive instruction. Any shape expressible as an SVG
/// `<path d="...">` can be expressed here.
///
/// `fill_rule` controls how overlapping subpaths are filled:
/// - `"nonzero"` (default) — inside if winding number is nonzero (most shapes)
/// - `"evenodd"` — inside if crossing count is odd (donuts, stars, letters)
#[derive(Clone, Debug, PartialEq)]
pub struct PaintPath {
    pub base: PaintBase,
    pub commands: Vec<PathCommand>,
    pub fill: Option<String>,
    pub fill_rule: Option<FillRule>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_cap: Option<StrokeCap>,
    pub stroke_join: Option<StrokeJoin>,
    /// Dash pattern for the stroke. `None` = solid.
    pub stroke_dash: Option<Vec<f64>>,
    /// Phase offset into the dash pattern. `None` = 0.0.
    pub stroke_dash_offset: Option<f64>,
}

/// Fill rule for overlapping subpaths.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// How line endpoints are drawn.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum StrokeCap {
    #[default]
    Butt,
    Round,
    Square,
}

/// How corners between path segments are drawn.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum StrokeJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}

// ─── PaintText ────────────────────────────────────────────────────────────────

/// Simple text instruction — let the backend handle shaping.
///
/// `PaintText` is a higher-level alternative to [`PaintGlyphRun`] for backends
/// that have built-in text shaping (CoreText on Metal, SVG `<text>`, Canvas
/// `fillText`). The backend is responsible for font loading, glyph selection,
/// and kerning — the producer just supplies the string and position.
///
/// Use `PaintText` when:
/// - You are targeting a backend with native text support (Metal, SVG, Canvas).
/// - You do not have pre-shaped glyph IDs (e.g. diagram-to-paint).
///
/// Use `PaintGlyphRun` when you have already shaped the text with a font engine
/// (e.g. through the FNT00 font-parser pipeline) and want pixel-perfect control.
///
/// ## font_ref format
///
/// `font_ref` is an opaque string passed to the backend unchanged.  Backends
/// may interpret it however suits them.  The recommended encoding is:
///
/// ```text
/// "canvas:<family>@<size>:<weight>"   e.g. "canvas:system-ui@14:400"
/// ```
///
/// If `font_ref` is `None`, backends should use the system UI font at `font_size`.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintText {
    pub base: PaintBase,
    /// Horizontal position in scene coordinates (meaning depends on `text_align`).
    pub x: f64,
    /// Vertical position — the text baseline in scene coordinates.
    pub y: f64,
    /// The string to render.
    pub text: String,
    /// Opaque font reference (see struct doc). `None` = backend default.
    pub font_ref: Option<String>,
    /// Font size in user-space units (pixels for bitmap renderers).
    pub font_size: f64,
    /// Fill colour for the glyphs. `None` defaults to black.
    pub fill: Option<String>,
    /// Horizontal alignment relative to `x`. Default: `Left`.
    pub text_align: Option<TextAlign>,
}

/// Horizontal text alignment relative to the `x` coordinate in [`PaintText`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

// ─── PaintGlyphRun ────────────────────────────────────────────────────────────

/// Pre-positioned glyphs from a font, after shaping and layout.
///
/// A `PaintGlyphRun` represents text that has already been shaped and positioned.
/// The producer has resolved the font, computed glyph IDs, applied kerning, and
/// calculated `(x, y)` for each glyph. The VM just paints them.
///
/// ## Why not always use PaintText?
///
/// Because text rendering requires font loading, shaping, line breaking, and
/// bidirectional text support — none of which the PaintVM should know about.
/// The `PaintGlyphRun` is what you get **after** all that work is done by the
/// font-parser (FNT00) and layout layers.  Use [`PaintText`] when the backend
/// can handle shaping natively (see its struct doc).
#[derive(Clone, Debug, PartialEq)]
pub struct PaintGlyphRun {
    pub base: PaintBase,
    pub glyphs: Vec<GlyphPosition>,
    /// Opaque font reference for the backend (CSS font name, file path, or pre-loaded handle).
    pub font_ref: String,
    /// Font size in user-space units.
    pub font_size: f64,
    /// Glyph fill colour. `None` defaults to black.
    pub fill: Option<String>,
}

/// A single glyph position within a [`PaintGlyphRun`].
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphPosition {
    /// Numeric glyph ID from the font's cmap table.
    pub glyph_id: u32,
    /// X origin for this glyph in scene coordinates.
    pub x: f64,
    /// Y origin (baseline position) in scene coordinates.
    pub y: f64,
}

// ─── PaintGroup ───────────────────────────────────────────────────────────────

/// Logical container for transform and state inheritance.
///
/// A group renders directly into the parent surface — no separate buffer is
/// allocated. Use it for:
/// - Applying a transform to a set of instructions as a unit
/// - Applying an opacity to a set of instructions
/// - Logical grouping for `patch()` id stability
///
/// For filters or blend modes, use [`PaintLayer`] instead. Layers allocate a
/// separate offscreen buffer so filters can operate on the composited result.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintGroup {
    pub base: PaintBase,
    pub children: Vec<PaintInstruction>,
    /// 6-element affine matrix; `None` = identity.
    pub transform: Option<Transform2D>,
    /// 0.0–1.0; `None` = 1.0 (fully opaque).
    pub opacity: Option<f64>,
}

// ─── PaintLayer ───────────────────────────────────────────────────────────────

/// Isolated offscreen compositing surface.
///
/// A `PaintLayer` is fundamentally different from a [`PaintGroup`]:
///
/// - `PaintGroup`: renders children **directly** into the parent surface.
///   Fast. No offscreen allocation. Cannot apply filters.
///
/// - `PaintLayer`: allocates a **separate offscreen buffer**, renders children
///   into it, applies filters to the composited result, then composites the
///   entire buffer back into the parent using `blend_mode`.
///
/// This is the same model as Photoshop layers, CSS `filter + mix-blend-mode`,
/// and SVG `<filter>` elements.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintLayer {
    pub base: PaintBase,
    pub children: Vec<PaintInstruction>,
    pub filters: Option<Vec<FilterEffect>>,
    pub blend_mode: Option<BlendMode>,
    /// 0.0–1.0; applied after filters.
    pub opacity: Option<f64>,
    pub transform: Option<Transform2D>,
}

// ─── PaintLine ────────────────────────────────────────────────────────────────

/// A straight line segment between two points.
///
/// For multiple connected lines, use [`PaintPath`] with `LineTo` commands.
/// `stroke` is required — a line with no colour is invisible.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintLine {
    pub base: PaintBase,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    /// Required — CSS colour.
    pub stroke: String,
    /// Default 1.0.
    pub stroke_width: Option<f64>,
    pub stroke_cap: Option<StrokeCap>,
    /// Dash pattern for the stroke. `None` = solid.
    pub stroke_dash: Option<Vec<f64>>,
    /// Phase offset into the dash pattern. `None` = 0.0.
    pub stroke_dash_offset: Option<f64>,
}

// ─── PaintClip ────────────────────────────────────────────────────────────────

/// Rectangular clip mask for child instructions.
///
/// Children are rendered clipped to the given rectangle. Pixels outside the
/// clip rect are not drawn. The clip affects only the `children` — instructions
/// outside `PaintClip` are unaffected.
///
/// This is currently a rectangular clip. Arbitrary path clipping is deferred
/// to `PaintMask` (P2D08).
#[derive(Clone, Debug, PartialEq)]
pub struct PaintClip {
    pub base: PaintBase,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub children: Vec<PaintInstruction>,
}

// ─── PaintGradient ────────────────────────────────────────────────────────────

/// Linear or radial colour gradient.
///
/// A `PaintGradient` is referenced by id from a `PaintRect`, `PaintEllipse`, or
/// `PaintPath` fill field: `fill: "url(#my-gradient)"`.
///
/// ## Linear gradient
///
/// The gradient runs from `(x1, y1)` to `(x2, y2)` in user space.
///
/// ```text
/// gradient axis → → → → →
/// (x1,y1)           (x2,y2)
/// stop0  stop1  stop2  stop3
/// ```
///
/// ## Radial gradient
///
/// Radiates from center `(cx, cy)` outward to radius `r`.
/// Innermost colour at `(cx, cy)`, outermost at radius `r`.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintGradient {
    pub base: PaintBase,
    pub kind: GradientKind,
    pub stops: Vec<GradientStop>,
}

/// Linear vs radial gradient geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum GradientKind {
    Linear { x1: f64, y1: f64, x2: f64, y2: f64 },
    Radial { cx: f64, cy: f64, r: f64 },
}

/// A colour stop at a position along the gradient axis.
#[derive(Clone, Debug, PartialEq)]
pub struct GradientStop {
    /// 0.0 (start) to 1.0 (end).
    pub offset: f64,
    /// CSS colour at this stop.
    pub color: String,
}

// ─── PaintImage ───────────────────────────────────────────────────────────────

/// Raster or decoded pixel image rendered into a rectangle.
///
/// `src` accepts two forms:
///
/// - **`ImageSrc::Uri`** — a URI string the backend resolves at render time
///   (`"file:///assets/logo.png"`, `"data:image/png;base64,…"`).
///   The backend is responsible for decoding. The IR does not fetch or validate URIs.
///
/// - **`ImageSrc::Pixels`** — already-decoded [`PixelContainer`]. The VM paints
///   them directly with no decoding step. This is the zero-copy path when you
///   already have decoded pixels (e.g. the output of a previous render fed into
///   another scene for picture-in-picture or thumbnail strips).
#[derive(Clone, Debug, PartialEq)]
pub struct PaintImage {
    pub base: PaintBase,
    /// Top-left x of the rendered rectangle.
    pub x: f64,
    /// Top-left y of the rendered rectangle.
    pub y: f64,
    /// Rendered width (may differ from intrinsic image width).
    pub width: f64,
    /// Rendered height (may differ from intrinsic image height).
    pub height: f64,
    pub src: ImageSrc,
    /// 0.0–1.0; `None` = 1.0 (fully opaque).
    pub opacity: Option<f64>,
}

/// Source for a [`PaintImage`] — either a URI or decoded pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum ImageSrc {
    /// URI string resolved by the backend at render time.
    Uri(String),
    /// Already-decoded pixels — zero-copy path.
    Pixels(PixelContainer),
}

// ============================================================================
// PaintInstruction — the union of all instruction types
// ============================================================================

/// The complete union of all paint instruction types.
///
/// Every variant maps to one instruction type. The `kind` naming in the
/// TypeScript version becomes Rust enum variants:
///
/// ```text
/// TypeScript kind   Rust variant
/// "rect"         →  Rect(PaintRect)
/// "ellipse"      →  Ellipse(PaintEllipse)
/// "path"         →  Path(PaintPath)
/// "glyph_run"    →  GlyphRun(PaintGlyphRun)
/// "group"        →  Group(PaintGroup)
/// "layer"        →  Layer(PaintLayer)
/// "line"         →  Line(PaintLine)
/// "clip"         →  Clip(PaintClip)
/// "gradient"     →  Gradient(PaintGradient)
/// "image"        →  Image(PaintImage)
/// ```
///
/// ## Dispatch pattern
///
/// ```rust
/// use paint_instructions::PaintInstruction;
///
/// fn describe(instr: &PaintInstruction) -> &str {
///     match instr {
///         PaintInstruction::Rect(_)     => "rect",
///         PaintInstruction::Ellipse(_)  => "ellipse",
///         PaintInstruction::Path(_)     => "path",
///         PaintInstruction::Text(_)     => "text",
///         PaintInstruction::GlyphRun(_) => "glyph_run",
///         PaintInstruction::Group(_)    => "group",
///         PaintInstruction::Layer(_)    => "layer",
///         PaintInstruction::Line(_)     => "line",
///         PaintInstruction::Clip(_)     => "clip",
///         PaintInstruction::Gradient(_) => "gradient",
///         PaintInstruction::Image(_)    => "image",
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum PaintInstruction {
    Rect(PaintRect),
    Ellipse(PaintEllipse),
    Path(PaintPath),
    /// Simple string text — backend handles shaping. See [`PaintText`].
    Text(PaintText),
    GlyphRun(PaintGlyphRun),
    Group(PaintGroup),
    Layer(PaintLayer),
    Line(PaintLine),
    Clip(PaintClip),
    Gradient(PaintGradient),
    Image(PaintImage),
}

// ============================================================================
// PaintScene — the top-level container
// ============================================================================

/// The top-level value passed to a paint VM or renderer.
///
/// It defines the viewport dimensions, background colour, and the ordered list
/// of instructions. Instructions are rendered **back-to-front** (painter's
/// algorithm): the first instruction is painted first (furthest back).
///
/// ## `background`
///
/// A CSS colour painted before all instructions:
/// - `"#ffffff"` — white background
/// - `"#000000"` — black background
/// - `"transparent"` — no background fill (useful for compositing)
///
/// ## Example — a 400×300 white scene with a red square
///
/// ```rust
/// use paint_instructions::*;
///
/// let scene = PaintScene {
///     width: 400.0,
///     height: 300.0,
///     background: "#ffffff".to_string(),
///     instructions: vec![
///         PaintInstruction::Rect(PaintRect::filled(50.0, 50.0, 100.0, 100.0, "#ff0000")),
///     ],
///     id: None,
///     metadata: None,
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct PaintScene {
    /// Viewport width in user-space units (pixels for bitmap renderers).
    pub width: f64,
    /// Viewport height in user-space units.
    pub height: f64,
    /// Background colour — painted before all instructions.
    pub background: String,
    /// Instructions rendered back-to-front (painter's algorithm).
    pub instructions: Vec<PaintInstruction>,
    /// Stable scene identity for future `patch()` support.
    pub id: Option<String>,
    /// Arbitrary scene-level metadata for producers and debuggers.
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

impl PaintScene {
    /// Convenience constructor for a blank scene with a white background.
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            background: "#ffffff".to_string(),
            instructions: Vec::new(),
            id: None,
            metadata: None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ─── PixelContainer tests ────────────────────────────────────────────────

    #[test]
    fn pixel_container_new_creates_zeroed_buffer() {
        let pc = PixelContainer::new(4, 3);
        assert_eq!(pc.width, 4);
        assert_eq!(pc.height, 3);
        assert_eq!(pc.data.len(), 4 * 3 * 4);
        assert!(pc.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn pixel_container_from_data() {
        let data = vec![255u8; 2 * 2 * 4];
        let pc = PixelContainer::from_data(2, 2, data);
        assert_eq!(pc.pixel_count(), 4);
        assert_eq!(pc.byte_count(), 16);
    }

    #[test]
    #[should_panic]
    fn pixel_container_from_data_wrong_size_panics() {
        // pixel-container panics when data.len() != width*height*4.
        PixelContainer::from_data(2, 2, vec![0u8; 10]);
    }

    #[test]
    fn pixel_container_set_and_read_pixel() {
        let mut pc = PixelContainer::new(3, 3);
        pc.set_pixel(1, 2, 10, 20, 30, 40);
        assert_eq!(pc.pixel_at(1, 2), (10, 20, 30, 40));
        // Surrounding pixels untouched
        assert_eq!(pc.pixel_at(0, 0), (0, 0, 0, 0));
    }

    #[test]
    fn pixel_container_row_major_layout() {
        // For a 3×2 image, pixel (1, 0) is at byte offset 4
        let mut pc = PixelContainer::new(3, 2);
        pc.set_pixel(1, 0, 1, 2, 3, 4);
        assert_eq!(pc.data[4..8], [1, 2, 3, 4]);
        // Pixel (0, 1) is at byte offset 3*4 = 12
        pc.set_pixel(0, 1, 10, 20, 30, 40);
        assert_eq!(pc.data[12..16], [10, 20, 30, 40]);
    }

    #[test]
    fn pixel_container_zero_size() {
        let pc = PixelContainer::new(0, 0);
        assert_eq!(pc.pixel_count(), 0);
        assert_eq!(pc.byte_count(), 0);
    }

    #[test]
    fn pixel_container_x_out_of_bounds_returns_zero() {
        // pixel-container returns (0,0,0,0) for OOB coordinates rather than panicking.
        assert_eq!(PixelContainer::new(3, 3).pixel_at(3, 0), (0, 0, 0, 0));
    }

    #[test]
    fn pixel_container_y_out_of_bounds_returns_zero() {
        assert_eq!(PixelContainer::new(3, 3).pixel_at(0, 3), (0, 0, 0, 0));
    }

    // ─── PaintRect tests ─────────────────────────────────────────────────────

    #[test]
    fn paint_rect_filled_constructor() {
        let r = PaintRect::filled(10.0, 20.0, 100.0, 50.0, "#ff0000");
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
        assert_eq!(r.fill, Some("#ff0000".to_string()));
        assert_eq!(r.stroke, None);
    }

    // ─── PaintScene tests ────────────────────────────────────────────────────

    #[test]
    fn paint_scene_new_has_white_background() {
        let scene = PaintScene::new(800.0, 600.0);
        assert_eq!(scene.width, 800.0);
        assert_eq!(scene.height, 600.0);
        assert_eq!(scene.background, "#ffffff");
        assert!(scene.instructions.is_empty());
        assert!(scene.metadata.is_none());
    }

    #[test]
    fn paint_scene_can_hold_instructions() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene
            .instructions
            .push(PaintInstruction::Rect(PaintRect::filled(
                0.0, 0.0, 100.0, 100.0, "#000000",
            )));
        assert_eq!(scene.instructions.len(), 1);
    }

    #[test]
    fn paint_scene_can_hold_metadata() {
        let mut scene = PaintScene::new(100.0, 100.0);
        scene.metadata = Some(std::collections::HashMap::from([(
            "label".to_string(),
            "Demo scene".to_string(),
        )]));
        assert_eq!(
            scene
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("label")),
            Some(&"Demo scene".to_string())
        );
    }

    // ─── PaintInstruction dispatch ───────────────────────────────────────────

    #[test]
    fn paint_instruction_variants_match() {
        let line = PaintInstruction::Line(PaintLine {
            base: PaintBase::default(),
            x1: 0.0,
            y1: 0.0,
            x2: 10.0,
            y2: 10.0,
            stroke: "#000000".to_string(),
            stroke_width: None,
            stroke_cap: None,
            stroke_dash: None,
            stroke_dash_offset: None,
        });

        match &line {
            PaintInstruction::Line(l) => {
                assert_eq!(l.stroke, "#000000");
                assert_eq!(l.x2, 10.0);
            }
            _ => panic!("expected Line variant"),
        }
    }

    #[test]
    fn blend_mode_default_is_normal() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }

    #[test]
    fn identity_transform_values() {
        assert_eq!(IDENTITY_TRANSFORM, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn paint_group_can_nest_children() {
        let group = PaintInstruction::Group(PaintGroup {
            base: PaintBase::default(),
            children: vec![
                PaintInstruction::Rect(PaintRect::filled(0.0, 0.0, 50.0, 50.0, "#ff0000")),
                PaintInstruction::Rect(PaintRect::filled(50.0, 0.0, 50.0, 50.0, "#00ff00")),
            ],
            transform: None,
            opacity: None,
        });

        match &group {
            PaintInstruction::Group(g) => assert_eq!(g.children.len(), 2),
            _ => panic!("expected Group"),
        }
    }

    #[test]
    fn image_src_pixels_carries_pixel_container() {
        let pc = PixelContainer::new(4, 4);
        let src = ImageSrc::Pixels(pc.clone());
        match src {
            ImageSrc::Pixels(p) => assert_eq!(p.width, 4),
            _ => panic!("expected Pixels"),
        }
    }

    // ─── PaintText tests ─────────────────────────────────────────────────────

    #[test]
    fn paint_text_fields_round_trip() {
        let t = PaintText {
            base: PaintBase::default(),
            x: 100.0,
            y: 50.0,
            text: "Hello".to_string(),
            font_ref: Some("canvas:system-ui@14:400".to_string()),
            font_size: 14.0,
            fill: Some("#111827".to_string()),
            text_align: Some(TextAlign::Center),
        };
        assert_eq!(t.text, "Hello");
        assert_eq!(t.font_size, 14.0);
        assert_eq!(t.text_align, Some(TextAlign::Center));
    }

    #[test]
    fn text_align_default_is_left() {
        assert_eq!(TextAlign::default(), TextAlign::Left);
    }

    #[test]
    fn paint_instruction_text_variant() {
        let instr = PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: 0.0,
            y: 0.0,
            text: "diagram label".to_string(),
            font_ref: None,
            font_size: 12.0,
            fill: None,
            text_align: None,
        });
        match &instr {
            PaintInstruction::Text(t) => assert_eq!(t.text, "diagram label"),
            _ => panic!("expected Text variant"),
        }
    }
}
