//! # barcode-2d
//!
//! Shared 2D barcode abstraction layer.
//!
//! This crate provides the two building blocks every 2D barcode format needs:
//!
//! 1. [`ModuleGrid`] — the universal intermediate representation produced by
//!    every 2D barcode encoder (QR Code, Data Matrix, Aztec Code, PDF417,
//!    MaxiCode). It is a 2D boolean grid: `true` = dark module, `false` = light.
//!
//! 2. [`layout()`] — the single function that converts abstract module
//!    coordinates into pixel-level [`PaintScene`] instructions ready for the
//!    PaintVM (P2D01) to render.
//!
//! ## Pipeline
//!
//! ```text
//! Input data
//!   → format encoder (qr-code, data-matrix, aztec…)
//!   → ModuleGrid                 ← produced by the encoder
//!   → layout()                   ← THIS CRATE converts to pixels
//!   → PaintScene                 ← consumed by paint-vm (P2D01)
//!   → backend (SVG, Metal, Canvas, terminal…)
//! ```
//!
//! All coordinates before `layout()` are in abstract module units.
//! Only `layout()` multiplies by `module_size_px` to produce real pixels.
//!
//! ## Supported module shapes
//!
//! - **Square** (default): QR Code, Data Matrix, Aztec Code, PDF417.
//!   Each dark module becomes a [`PaintRect`].
//!
//! - **Hex** (flat-top hexagons): MaxiCode (ISO/IEC 16023).
//!   Each dark module becomes a [`PaintPath`] tracing six vertices.

pub const VERSION: &str = "0.1.0";

use paint_instructions::{
    PaintBase, PaintInstruction, PaintPath, PaintRect, PaintScene, PathCommand,
};
use std::f64::consts::PI;

// ============================================================================
// ModuleShape — square vs. hex
// ============================================================================

/// The shape of each module in the grid.
///
/// - [`Square`] — used by QR Code, Data Matrix, Aztec Code, PDF417.
///   Each module renders as a filled square (`PaintRect`).
///
/// - [`Hex`] — used by MaxiCode (ISO/IEC 16023). MaxiCode uses flat-top
///   hexagons arranged in an offset-row grid. Each module renders as a
///   filled hexagon drawn with a `PaintPath`.
///
/// The shape is stored on [`ModuleGrid`] so that [`layout()`] can pick the
/// right rendering path without the caller specifying it again.
///
/// [`Square`]: ModuleShape::Square
/// [`Hex`]: ModuleShape::Hex
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleShape {
    /// Standard square modules — QR Code, Data Matrix, Aztec, PDF417.
    Square,
    /// Flat-top hexagonal modules — MaxiCode.
    Hex,
}

impl Default for ModuleShape {
    fn default() -> Self {
        Self::Square
    }
}

// ============================================================================
// ModuleGrid — the universal output of every 2D barcode encoder
// ============================================================================

/// The universal intermediate representation produced by every 2D barcode
/// encoder. It is a 2D boolean grid.
///
/// ```text
/// modules[row][col] == true   →  dark module (ink / filled)
/// modules[row][col] == false  →  light module (background / empty)
/// ```
///
/// Row 0 is the top row. Column 0 is the leftmost column. This matches the
/// natural reading order used in every 2D barcode standard.
///
/// ## MaxiCode fixed size
///
/// MaxiCode grids are always 33 rows × 30 columns with `module_shape: Hex`.
/// Physical MaxiCode symbols are always approximately 1 inch × 1 inch.
///
/// ## Ownership
///
/// [`ModuleGrid`] is owned (not borrowed). Use [`set_module()`] to produce a
/// new grid with one module changed rather than mutating in place. Rust's
/// move semantics make this efficient: only the affected row is re-allocated.
pub struct ModuleGrid {
    /// Number of columns (width of the grid).
    pub cols: u32,
    /// Number of rows (height of the grid).
    pub rows: u32,
    /// Two-dimensional boolean grid. Access with `modules[row][col]`.
    /// Outer `Vec` is rows, inner `Vec` is columns.
    pub modules: Vec<Vec<bool>>,
    /// Shape of each module — square (default) or hex (MaxiCode).
    pub module_shape: ModuleShape,
}

// ============================================================================
// ModuleRole — what a module structurally represents
// ============================================================================

/// The structural role of a module within its barcode symbol.
///
/// These roles are generic — they apply across all 2D barcode formats.
///
/// Format-specific roles (e.g. QR Code's "dark module", Aztec's "mode
/// message") are stored in [`ModuleAnnotation::metadata`] under the key
/// `"format_role"` as strings like `"qr:dark-module"`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleRole {
    /// Locator pattern (QR: 7×7 corner ring, Data Matrix: L-finder, Aztec: bullseye).
    Finder,
    /// Quiet zone isolating a finder from the data area.
    Separator,
    /// Alternating dark/light calibration strip.
    Timing,
    /// Secondary locator for distortion correction (QR Code high versions).
    Alignment,
    /// ECC level + mask/layer info bits.
    Format,
    /// One bit of an encoded message codeword.
    Data,
    /// One bit of an error correction codeword.
    Ecc,
    /// Filler bits to complete the grid capacity.
    Padding,
}

// ============================================================================
// ModuleAnnotation — per-module role metadata for visualizers
// ============================================================================

/// Per-module role annotation, used by visualizers to colour-code the symbol.
///
/// Annotations are entirely optional. [`layout()`] only reads
/// [`ModuleGrid::modules`]; it never looks at annotations.
pub struct ModuleAnnotation {
    /// The structural role of this module.
    pub role: ModuleRole,
    /// Whether this module is dark (`true`) or light (`false`).
    pub dark: bool,
    /// Zero-based index into the interleaved codeword stream (data/ecc only).
    pub codeword_index: Option<u32>,
    /// Zero-based bit index within the codeword (0 = MSB) (data/ecc only).
    pub bit_index: Option<u32>,
    /// Format-specific key/value pairs. Use `"format_role"` for namespaced
    /// role strings like `"qr:dark-module"` or `"aztec:mode-message"`.
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

// ============================================================================
// AnnotatedModuleGrid — ModuleGrid with per-module role annotations
// ============================================================================

/// A [`ModuleGrid`] extended with per-module role annotations.
///
/// The `annotations` outer `Vec` is rows, inner `Vec` is columns.
/// `annotations[row][col]` mirrors `modules[row][col]`.
/// `None` means "no annotation for this module".
pub struct AnnotatedModuleGrid {
    pub grid: ModuleGrid,
    pub annotations: Vec<Vec<Option<ModuleAnnotation>>>,
}

// ============================================================================
// Barcode2DLayoutConfig — pixel-level rendering options
// ============================================================================

/// Configuration for [`layout()`].
///
/// Use `Default::default()` for sensible defaults, or build your own struct.
///
/// # Validation
///
/// [`layout()`] validates the config and returns `Err(Barcode2DError::InvalidConfig(…))`
/// if:
/// - `module_size_px <= 0.0`
/// - `quiet_zone_modules > u32::MAX` (cannot happen via the `u32` type itself)
/// - `module_shape` does not match `grid.module_shape`
pub struct Barcode2DLayoutConfig {
    /// Size of one module in pixels. Must be > 0.
    /// Default: 10.0
    pub module_size_px: f64,
    /// Number of module-width quiet-zone units added on each side.
    /// QR Code minimum = 4, Data Matrix minimum = 1.
    /// Default: 4
    pub quiet_zone_modules: u32,
    /// Dark module color (CSS color string). Default: "#000000"
    pub foreground: String,
    /// Light module / background color (CSS color string). Default: "#ffffff"
    pub background: String,
    /// Whether to embed role metadata in the PaintScene. Default: false
    pub show_annotations: bool,
    /// Shape of modules — must match the `ModuleGrid`'s `module_shape`.
    /// Default: `ModuleShape::Square`
    pub module_shape: ModuleShape,
}

impl Default for Barcode2DLayoutConfig {
    /// Sensible defaults matching QR Code's minimum quiet-zone requirement.
    ///
    /// | Field              | Default     |
    /// |--------------------|-------------|
    /// | module_size_px     | 10.0        |
    /// | quiet_zone_modules | 4           |
    /// | foreground         | "#000000"   |
    /// | background         | "#ffffff"   |
    /// | show_annotations   | false       |
    /// | module_shape       | Square      |
    fn default() -> Self {
        Self {
            module_size_px: 10.0,
            quiet_zone_modules: 4,
            foreground: "#000000".to_string(),
            background: "#ffffff".to_string(),
            show_annotations: false,
            module_shape: ModuleShape::Square,
        }
    }
}

// ============================================================================
// Error types
// ============================================================================

/// Errors produced by the barcode-2d crate.
#[derive(Debug, PartialEq, Eq)]
pub enum Barcode2DError {
    /// The layout configuration is invalid. The message describes why.
    InvalidConfig(String),
    /// Grid dimensions are internally inconsistent (never happens via the
    /// public API, but useful for defensive defensive checks in encoders).
    DimensionMismatch(String),
}

impl std::fmt::Display for Barcode2DError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(msg) => write!(f, "InvalidConfig: {}", msg),
            Self::DimensionMismatch(msg) => write!(f, "DimensionMismatch: {}", msg),
        }
    }
}

impl std::error::Error for Barcode2DError {}

// ============================================================================
// make_module_grid — create an all-light grid
// ============================================================================

/// Create a new [`ModuleGrid`] of the given dimensions, with every module
/// set to `false` (light).
///
/// This is the starting point for every 2D barcode encoder. The encoder calls
/// `make_module_grid(rows, cols, ModuleShape::Square)` and then uses
/// [`set_module()`] to paint dark modules one by one.
///
/// ## Example — start a 21×21 QR Code v1 grid
///
/// ```text
/// let grid = make_module_grid(21, 21, ModuleShape::Square);
/// assert_eq!(grid.rows, 21);
/// assert_eq!(grid.cols, 21);
/// assert!(!grid.modules[0][0]); // all light
/// ```
pub fn make_module_grid(rows: u32, cols: u32, module_shape: ModuleShape) -> ModuleGrid {
    let modules: Vec<Vec<bool>> = (0..rows)
        .map(|_| vec![false; cols as usize])
        .collect();
    ModuleGrid { cols, rows, modules, module_shape }
}

// ============================================================================
// set_module — immutable single-module update
// ============================================================================

/// Return a new [`ModuleGrid`] identical to `grid` except that module at
/// `(row, col)` is set to `dark`.
///
/// This function is **pure and produces a new grid** — it never modifies the
/// input. Only the affected row is re-allocated; all other rows are cloned
/// by reference (cheap for small grids, and grids are small by barcode
/// standards: at most ~177×177 for QR v40).
///
/// ## Panics
///
/// Panics if `row >= grid.rows` or `col >= grid.cols`. This is a programming
/// error in the encoder.
///
/// ## Example
///
/// ```text
/// let g = make_module_grid(3, 3, ModuleShape::Square);
/// let g2 = set_module(&g, 1, 1, true);
/// assert!(!g.modules[1][1]);   // original unchanged
/// assert!(g2.modules[1][1]);   // new grid updated
/// ```
pub fn set_module(grid: &ModuleGrid, row: u32, col: u32, dark: bool) -> ModuleGrid {
    assert!(
        row < grid.rows,
        "set_module: row {} out of range [0, {}]",
        row,
        grid.rows - 1
    );
    assert!(
        col < grid.cols,
        "set_module: col {} out of range [0, {}]",
        col,
        grid.cols - 1
    );

    // Clone only the affected row; other rows are cheap Vec clones.
    let mut new_modules: Vec<Vec<bool>> = grid.modules.clone();
    new_modules[row as usize][col as usize] = dark;

    ModuleGrid {
        cols: grid.cols,
        rows: grid.rows,
        modules: new_modules,
        module_shape: grid.module_shape.clone(),
    }
}

// ============================================================================
// layout — ModuleGrid → PaintScene
// ============================================================================

/// Convert a [`ModuleGrid`] into a [`PaintScene`] ready for the PaintVM.
///
/// This is the **only** function in the barcode-2d crate that knows about
/// pixels. Everything above this step works in abstract module units.
/// Everything below this step is handled by the paint backend.
///
/// ## Square modules (the common case)
///
/// Each dark module at `(row, col)` becomes one [`PaintRect`]:
///
/// ```text
/// quiet_zone_px = quiet_zone_modules * module_size_px
/// x = quiet_zone_px + col * module_size_px
/// y = quiet_zone_px + row * module_size_px
///
/// total_width  = (cols + 2 * quiet_zone_modules) * module_size_px
/// total_height = (rows + 2 * quiet_zone_modules) * module_size_px
/// ```
///
/// The scene always starts with a background [`PaintRect`] covering the
/// entire symbol including the quiet zone.
///
/// ## Hex modules (MaxiCode)
///
/// Each dark module at `(row, col)` becomes one [`PaintPath`] tracing a
/// flat-top regular hexagon. Odd-numbered rows are shifted right by half a
/// hexagon width (offset-row tiling).
///
/// ## Errors
///
/// Returns `Err(Barcode2DError::InvalidConfig(…))` if:
/// - `config.module_size_px <= 0.0`
/// - `config.module_shape != grid.module_shape`
pub fn layout(
    grid: &ModuleGrid,
    config: &Barcode2DLayoutConfig,
) -> Result<PaintScene, Barcode2DError> {
    // ── Validation ──────────────────────────────────────────────────────────
    if config.module_size_px <= 0.0 {
        return Err(Barcode2DError::InvalidConfig(format!(
            "module_size_px must be > 0, got {}",
            config.module_size_px
        )));
    }
    if config.module_shape != grid.module_shape {
        return Err(Barcode2DError::InvalidConfig(format!(
            "config.module_shape ({:?}) does not match grid.module_shape ({:?})",
            config.module_shape, grid.module_shape
        )));
    }

    // Dispatch to the correct rendering path.
    match config.module_shape {
        ModuleShape::Square => Ok(layout_square(grid, config)),
        ModuleShape::Hex => Ok(layout_hex(grid, config)),
    }
}

// ============================================================================
// layout_square — internal helper for square-module grids
// ============================================================================

/// Render a square-module [`ModuleGrid`] into a [`PaintScene`].
///
/// Called only by [`layout()`] after validation.
///
/// Algorithm:
/// 1. Compute total pixel dimensions including quiet zone.
/// 2. Emit one background [`PaintRect`] covering the entire symbol.
/// 3. For each dark module, emit one filled [`PaintRect`].
///
/// Light modules are implicitly covered by the background rect — no separate
/// light rects are emitted. This keeps the instruction count proportional to
/// the number of dark modules rather than the total grid size.
fn layout_square(grid: &ModuleGrid, config: &Barcode2DLayoutConfig) -> PaintScene {
    let module_size = config.module_size_px;
    let quiet = config.quiet_zone_modules as f64;

    // Quiet zone in pixels on each side.
    let quiet_zone_px = quiet * module_size;

    // Total canvas dimensions including quiet zone on all four sides.
    let total_width = (grid.cols as f64 + 2.0 * quiet) * module_size;
    let total_height = (grid.rows as f64 + 2.0 * quiet) * module_size;

    let mut instructions: Vec<PaintInstruction> = Vec::new();

    // 1. Background rect — covers the entire symbol including quiet zone.
    instructions.push(PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(),
        x: 0.0,
        y: 0.0,
        width: total_width,
        height: total_height,
        fill: Some(config.background.clone()),
        stroke: None,
        stroke_width: None,
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }));

    // 2. One PaintRect per dark module.
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            if grid.modules[row as usize][col as usize] {
                let x = quiet_zone_px + col as f64 * module_size;
                let y = quiet_zone_px + row as f64 * module_size;

                instructions.push(PaintInstruction::Rect(PaintRect {
                    base: PaintBase::default(),
                    x,
                    y,
                    width: module_size,
                    height: module_size,
                    fill: Some(config.foreground.clone()),
                    stroke: None,
                    stroke_width: None,
                    corner_radius: None,
                    stroke_dash: None,
                    stroke_dash_offset: None,
                }));
            }
        }
    }

    PaintScene {
        width: total_width,
        height: total_height,
        background: config.background.clone(),
        instructions,
        id: None,
        metadata: None,
    }
}

// ============================================================================
// layout_hex — internal helper for hex-module grids (MaxiCode)
// ============================================================================

/// Render a hex-module [`ModuleGrid`] into a [`PaintScene`].
///
/// Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
/// offset-row grid. Odd rows are shifted right by half a hexagon width.
///
/// ## Flat-top hexagon geometry
///
/// For a flat-top hexagon centered at `(cx, cy)` with circumradius `R`:
///
/// ```text
/// hex_width  = module_size_px                (flat-to-flat = side length)
/// hex_height = module_size_px * (√3 / 2)    (vertical row step)
/// circum_r   = module_size_px / √3           (center to vertex)
///
/// Vertices at angles 0°, 60°, 120°, 180°, 240°, 300°:
///   vertex_i = ( cx + R * cos(i * 60°),
///                cy + R * sin(i * 60°) )
/// ```
///
/// Offset-row tiling:
///
/// ```text
/// Row 0:  ⬡ ⬡ ⬡ ⬡   (no offset)
/// Row 1:   ⬡ ⬡ ⬡ ⬡  (shifted right by hex_width / 2)
/// Row 2:  ⬡ ⬡ ⬡ ⬡   (no offset)
/// ```
fn layout_hex(grid: &ModuleGrid, config: &Barcode2DLayoutConfig) -> PaintScene {
    let module_size = config.module_size_px;
    let quiet = config.quiet_zone_modules as f64;

    // Hex geometry.
    let hex_width = module_size;
    let hex_height = module_size * (3.0_f64.sqrt() / 2.0);
    let circum_r = module_size / 3.0_f64.sqrt();

    let quiet_zone_px = quiet * module_size;

    // Total canvas size. The + hex_width/2 accommodates the odd-row offset.
    let total_width = (grid.cols as f64 + 2.0 * quiet) * hex_width + hex_width / 2.0;
    let total_height = (grid.rows as f64 + 2.0 * quiet) * hex_height;

    let mut instructions: Vec<PaintInstruction> = Vec::new();

    // Background rect.
    instructions.push(PaintInstruction::Rect(PaintRect {
        base: PaintBase::default(),
        x: 0.0,
        y: 0.0,
        width: total_width,
        height: total_height,
        fill: Some(config.background.clone()),
        stroke: None,
        stroke_width: None,
        corner_radius: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }));

    // One PaintPath per dark module.
    for row in 0..grid.rows {
        for col in 0..grid.cols {
            if grid.modules[row as usize][col as usize] {
                // Center of this hexagon in pixel space.
                // Odd rows shift right by hex_width / 2.
                let cx = quiet_zone_px
                    + col as f64 * hex_width
                    + if row % 2 == 1 { hex_width / 2.0 } else { 0.0 };
                let cy = quiet_zone_px + row as f64 * hex_height;

                instructions.push(PaintInstruction::Path(build_flat_top_hex_path(
                    cx,
                    cy,
                    circum_r,
                    &config.foreground,
                )));
            }
        }
    }

    PaintScene {
        width: total_width,
        height: total_height,
        background: config.background.clone(),
        instructions,
        id: None,
        metadata: None,
    }
}

// ============================================================================
// build_flat_top_hex_path — geometry helper
// ============================================================================

/// Build a [`PaintPath`] for a flat-top regular hexagon centered at `(cx, cy)`
/// with circumradius `r`.
///
/// The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
/// from the center:
///
/// ```text
/// vertex_i = ( cx + R * cos(i * 60°),
///              cy + R * sin(i * 60°) )
/// ```
///
/// The path is: `MoveTo(vertex_0)`, `LineTo(vertex_1..5)`, `Close`.
fn build_flat_top_hex_path(cx: f64, cy: f64, r: f64, fill: &str) -> PaintPath {
    let mut commands: Vec<PathCommand> = Vec::with_capacity(7);

    // Move to vertex 0.
    let angle0 = 0.0_f64 * (PI / 180.0);
    commands.push(PathCommand::MoveTo {
        x: cx + r * angle0.cos(),
        y: cy + r * angle0.sin(),
    });

    // Line to vertices 1–5.
    for i in 1..=5 {
        let angle = (i as f64) * 60.0 * (PI / 180.0);
        commands.push(PathCommand::LineTo {
            x: cx + r * angle.cos(),
            y: cy + r * angle.sin(),
        });
    }

    // Close back to vertex 0.
    commands.push(PathCommand::Close);

    PaintPath {
        base: PaintBase::default(),
        commands,
        fill: Some(fill.to_string()),
        fill_rule: None,
        stroke: None,
        stroke_width: None,
        stroke_cap: None,
        stroke_join: None,
        stroke_dash: None,
        stroke_dash_offset: None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── VERSION ──────────────────────────────────────────────────────────────

    #[test]
    fn version_is_correct() {
        assert_eq!(VERSION, "0.1.0");
    }

    // ── make_module_grid ─────────────────────────────────────────────────────

    #[test]
    fn make_module_grid_stores_correct_dimensions() {
        let g = make_module_grid(7, 11, ModuleShape::Square);
        assert_eq!(g.rows, 7);
        assert_eq!(g.cols, 11);
    }

    #[test]
    fn make_module_grid_all_light() {
        let g = make_module_grid(5, 5, ModuleShape::Square);
        for row in 0..g.rows as usize {
            for col in 0..g.cols as usize {
                assert!(!g.modules[row][col]);
            }
        }
    }

    #[test]
    fn make_module_grid_default_square_shape() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        assert_eq!(g.module_shape, ModuleShape::Square);
    }

    #[test]
    fn make_module_grid_hex_shape() {
        let g = make_module_grid(33, 30, ModuleShape::Hex);
        assert_eq!(g.module_shape, ModuleShape::Hex);
        assert_eq!(g.rows, 33);
        assert_eq!(g.cols, 30);
    }

    // ── set_module ───────────────────────────────────────────────────────────

    #[test]
    fn set_module_returns_new_grid() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let g2 = set_module(&g, 0, 0, true);
        // They are separate allocations (just check the value differs).
        assert!(!g.modules[0][0]);
        assert!(g2.modules[0][0]);
    }

    #[test]
    fn set_module_sets_target_to_true() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let g2 = set_module(&g, 1, 2, true);
        assert!(g2.modules[1][2]);
    }

    #[test]
    fn set_module_leaves_original_unchanged() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let _g2 = set_module(&g, 1, 2, true);
        assert!(!g.modules[1][2]);
    }

    #[test]
    fn set_module_clears_dark_module() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let g1 = set_module(&g, 0, 0, true);
        let g2 = set_module(&g1, 0, 0, false);
        assert!(!g2.modules[0][0]);
    }

    #[test]
    fn set_module_does_not_affect_other_modules() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let g2 = set_module(&g, 1, 1, true);
        assert!(!g2.modules[0][0]);
        assert!(!g2.modules[2][2]);
        assert!(!g2.modules[1][0]);
        assert!(!g2.modules[1][2]);
    }

    #[test]
    fn set_module_preserves_dimensions_and_shape() {
        let g = make_module_grid(5, 7, ModuleShape::Hex);
        let g2 = set_module(&g, 2, 3, true);
        assert_eq!(g2.rows, 5);
        assert_eq!(g2.cols, 7);
        assert_eq!(g2.module_shape, ModuleShape::Hex);
    }

    #[test]
    #[should_panic(expected = "row 3 out of range")]
    fn set_module_panics_row_out_of_bounds() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        set_module(&g, 3, 0, true);
    }

    #[test]
    #[should_panic(expected = "col 3 out of range")]
    fn set_module_panics_col_out_of_bounds() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        set_module(&g, 0, 3, true);
    }

    // ── layout (square) ──────────────────────────────────────────────────────

    #[test]
    fn layout_square_all_light_has_only_background_rect() {
        let g = make_module_grid(1, 1, ModuleShape::Square);
        let config = Barcode2DLayoutConfig::default();
        let scene = layout(&g, &config).unwrap();
        assert_eq!(scene.instructions.len(), 1);
        assert!(matches!(scene.instructions[0], PaintInstruction::Rect(_)));
    }

    #[test]
    fn layout_square_dark_module_produces_two_rects() {
        let g = make_module_grid(1, 1, ModuleShape::Square);
        let g = set_module(&g, 0, 0, true);
        let config = Barcode2DLayoutConfig {
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // Background + 1 dark module = 2 rects.
        let rect_count = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::Rect(_)))
            .count();
        assert_eq!(rect_count, 2);
    }

    #[test]
    fn layout_square_dark_module_at_0_0_has_correct_coordinates() {
        let g = make_module_grid(5, 5, ModuleShape::Square);
        let g = set_module(&g, 0, 0, true);
        let config = Barcode2DLayoutConfig {
            module_size_px: 10.0,
            quiet_zone_modules: 4,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // Second instruction is the dark module rect.
        if let PaintInstruction::Rect(r) = &scene.instructions[1] {
            // quiet_zone_px = 4 * 10 = 40
            assert!((r.x - 40.0).abs() < 1e-9);
            assert!((r.y - 40.0).abs() < 1e-9);
        } else {
            panic!("expected Rect");
        }
    }

    #[test]
    fn layout_square_dark_module_pixel_coordinates() {
        let g = make_module_grid(10, 10, ModuleShape::Square);
        let g = set_module(&g, 2, 3, true);
        let config = Barcode2DLayoutConfig {
            module_size_px: 8.0,
            quiet_zone_modules: 2,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // quiet_zone_px = 2 * 8 = 16, x = 16 + 3*8 = 40, y = 16 + 2*8 = 32
        if let PaintInstruction::Rect(r) = &scene.instructions[1] {
            assert!((r.x - 40.0).abs() < 1e-9);
            assert!((r.y - 32.0).abs() < 1e-9);
            assert!((r.width - 8.0).abs() < 1e-9);
            assert!((r.height - 8.0).abs() < 1e-9);
        } else {
            panic!("expected Rect");
        }
    }

    #[test]
    fn layout_square_qr_v1_canvas_size() {
        let g = make_module_grid(21, 21, ModuleShape::Square);
        let config = Barcode2DLayoutConfig {
            module_size_px: 10.0,
            quiet_zone_modules: 4,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // total = (21 + 8) * 10 = 290
        assert!((scene.width - 290.0).abs() < 1e-9);
        assert!((scene.height - 290.0).abs() < 1e-9);
    }

    #[test]
    fn layout_square_background_color_applied() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let config = Barcode2DLayoutConfig {
            background: "#aabbcc".to_string(),
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        assert_eq!(scene.background, "#aabbcc");
        if let PaintInstruction::Rect(r) = &scene.instructions[0] {
            assert_eq!(r.fill.as_deref(), Some("#aabbcc"));
        }
    }

    #[test]
    fn layout_square_foreground_color_applied_to_dark_rects() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let g = set_module(&g, 0, 0, true);
        let g = set_module(&g, 2, 2, true);
        let config = Barcode2DLayoutConfig {
            foreground: "#ff0000".to_string(),
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // Skip background rect (index 0).
        for instr in &scene.instructions[1..] {
            if let PaintInstruction::Rect(r) = instr {
                assert_eq!(r.fill.as_deref(), Some("#ff0000"));
            }
        }
    }

    #[test]
    fn layout_square_produces_no_path_instructions() {
        let g = make_module_grid(5, 5, ModuleShape::Square);
        let g = set_module(&g, 2, 2, true);
        let config = Barcode2DLayoutConfig::default();
        let scene = layout(&g, &config).unwrap();
        let path_count = scene
            .instructions
            .iter()
            .filter(|i| matches!(i, PaintInstruction::Path(_)))
            .count();
        assert_eq!(path_count, 0);
    }

    // ── layout (hex) ─────────────────────────────────────────────────────────

    #[test]
    fn layout_hex_dark_module_produces_path_not_rect() {
        let g = make_module_grid(5, 5, ModuleShape::Hex);
        let g = set_module(&g, 0, 0, true);
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // Background is still a rect.
        assert!(matches!(scene.instructions[0], PaintInstruction::Rect(_)));
        // Dark module is a path.
        assert!(matches!(scene.instructions[1], PaintInstruction::Path(_)));
    }

    #[test]
    fn layout_hex_path_has_seven_commands() {
        let g = make_module_grid(3, 3, ModuleShape::Hex);
        let g = set_module(&g, 0, 0, true);
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        if let PaintInstruction::Path(p) = &scene.instructions[1] {
            assert_eq!(p.commands.len(), 7);
            assert!(matches!(p.commands[0], PathCommand::MoveTo { .. }));
            assert!(matches!(p.commands[1], PathCommand::LineTo { .. }));
            assert!(matches!(p.commands[6], PathCommand::Close));
        } else {
            panic!("expected Path");
        }
    }

    #[test]
    fn layout_hex_even_row_no_x_offset() {
        let g = make_module_grid(3, 3, ModuleShape::Hex);
        let g = set_module(&g, 0, 0, true); // row 0 = even
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            module_size_px: 10.0,
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        if let PaintInstruction::Path(p) = &scene.instructions[1] {
            // For row=0, col=0, even row: cx = 0, vertex at 0° is (circumR, 0)
            let circum_r = 10.0 / 3.0_f64.sqrt();
            if let PathCommand::MoveTo { x, y } = p.commands[0] {
                assert!((x - circum_r).abs() < 1e-9);
                assert!((y - 0.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn layout_hex_odd_row_has_x_offset() {
        let g = make_module_grid(3, 3, ModuleShape::Hex);
        let g = set_module(&g, 1, 0, true); // row 1 = odd
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            module_size_px: 10.0,
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        if let PaintInstruction::Path(p) = &scene.instructions[1] {
            // For row=1, col=0, odd row: cx = 0 + 10/2 = 5
            let hex_width = 10.0_f64;
            let hex_height = 10.0 * (3.0_f64.sqrt() / 2.0);
            let circum_r = hex_width / 3.0_f64.sqrt();
            let expected_cx = hex_width / 2.0;
            let expected_cy = hex_height; // row 1
            if let PathCommand::MoveTo { x, y } = p.commands[0] {
                assert!((x - (expected_cx + circum_r)).abs() < 1e-9);
                assert!((y - expected_cy).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn layout_hex_all_vertices_at_circum_r_from_center() {
        let g = make_module_grid(3, 3, ModuleShape::Hex);
        let g = set_module(&g, 0, 1, true); // col=1, row=0 (even)
        let module_size_px = 12.0_f64;
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            module_size_px,
            quiet_zone_modules: 0,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        if let PaintInstruction::Path(p) = &scene.instructions[1] {
            let circum_r = module_size_px / 3.0_f64.sqrt();
            // Center: cx = 1 * 12 = 12, cy = 0
            let cx = 12.0_f64;
            let cy = 0.0_f64;
            // Check the 6 vertex commands (not the Close).
            for cmd in &p.commands[0..6] {
                let (vx, vy) = match cmd {
                    PathCommand::MoveTo { x, y } => (*x, *y),
                    PathCommand::LineTo { x, y } => (*x, *y),
                    _ => panic!("unexpected command"),
                };
                let dist = ((vx - cx).powi(2) + (vy - cy).powi(2)).sqrt();
                assert!((dist - circum_r).abs() < 1e-9, "dist={dist} circum_r={circum_r}");
            }
        }
    }

    #[test]
    fn layout_hex_total_width_includes_half_hex_offset() {
        let g = make_module_grid(4, 6, ModuleShape::Hex);
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            module_size_px: 10.0,
            quiet_zone_modules: 2,
            ..Default::default()
        };
        let scene = layout(&g, &config).unwrap();
        // total_width = (6 + 4) * 10 + 5 = 105
        assert!((scene.width - 105.0).abs() < 1e-9);
    }

    // ── Validation ───────────────────────────────────────────────────────────

    #[test]
    fn layout_invalid_module_size_zero() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let config = Barcode2DLayoutConfig {
            module_size_px: 0.0,
            ..Default::default()
        };
        assert!(matches!(
            layout(&g, &config),
            Err(Barcode2DError::InvalidConfig(_))
        ));
    }

    #[test]
    fn layout_invalid_module_size_negative() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let config = Barcode2DLayoutConfig {
            module_size_px: -5.0,
            ..Default::default()
        };
        assert!(matches!(
            layout(&g, &config),
            Err(Barcode2DError::InvalidConfig(_))
        ));
    }

    #[test]
    fn layout_module_shape_mismatch_square_grid_hex_config() {
        let g = make_module_grid(3, 3, ModuleShape::Square);
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Hex,
            ..Default::default()
        };
        assert!(matches!(
            layout(&g, &config),
            Err(Barcode2DError::InvalidConfig(_))
        ));
    }

    #[test]
    fn layout_module_shape_mismatch_hex_grid_square_config() {
        let g = make_module_grid(3, 3, ModuleShape::Hex);
        let config = Barcode2DLayoutConfig {
            module_shape: ModuleShape::Square,
            ..Default::default()
        };
        assert!(matches!(
            layout(&g, &config),
            Err(Barcode2DError::InvalidConfig(_))
        ));
    }

    #[test]
    fn barcode_2d_error_implements_display() {
        let e = Barcode2DError::InvalidConfig("test error".to_string());
        assert!(e.to_string().contains("test error"));
    }

    // ── Default config ───────────────────────────────────────────────────────

    #[test]
    fn default_config_module_size() {
        let cfg = Barcode2DLayoutConfig::default();
        assert!((cfg.module_size_px - 10.0).abs() < 1e-9);
    }

    #[test]
    fn default_config_quiet_zone() {
        let cfg = Barcode2DLayoutConfig::default();
        assert_eq!(cfg.quiet_zone_modules, 4);
    }

    #[test]
    fn default_config_foreground() {
        let cfg = Barcode2DLayoutConfig::default();
        assert_eq!(cfg.foreground, "#000000");
    }

    #[test]
    fn default_config_background() {
        let cfg = Barcode2DLayoutConfig::default();
        assert_eq!(cfg.background, "#ffffff");
    }

    #[test]
    fn default_config_show_annotations_false() {
        let cfg = Barcode2DLayoutConfig::default();
        assert!(!cfg.show_annotations);
    }

    #[test]
    fn default_config_module_shape_square() {
        let cfg = Barcode2DLayoutConfig::default();
        assert_eq!(cfg.module_shape, ModuleShape::Square);
    }
}
