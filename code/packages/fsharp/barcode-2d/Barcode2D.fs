/// Barcode2D.fs — Universal 2D barcode module-grid abstraction for F#
///
/// This module provides the two building blocks that every 2D barcode format
/// needs:
///
///   1. ``ModuleGrid`` — the universal intermediate representation produced by
///      every 2D barcode encoder (QR Code, Data Matrix, Aztec, PDF417,
///      MaxiCode). It is simply a 2-D boolean grid:
///        - ``true``  = dark module (ink / filled)
///        - ``false`` = light module (background / empty)
///
///   2. ``layout`` — the single function that converts abstract module
///      coordinates into pixel-level ``PaintScene`` instructions ready for the
///      PaintVM (P2D01) to render.
///
/// ## Where this fits in the pipeline
///
///  Input data
///    → format encoder (qr-code, data-matrix, aztec…)
///    → ModuleGrid          ← produced by the encoder
///    → layout()            ← THIS MODULE converts to pixels
///    → PaintScene          ← consumed by paint-vm (P2D01)
///    → backend (SVG, Metal, Canvas, terminal…)
///
/// All coordinates before ``layout`` are measured in "module units" — abstract
/// grid steps.  Only ``layout`` multiplies by ``moduleSizePx`` to produce real
/// pixel coordinates. This means encoders never need to know anything about
/// screen resolution or output format.
///
/// ## Supported module shapes
///
///   - ``Square`` (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
///     Each module becomes a ``PaintRect``.
///
///   - ``Hex`` (flat-top hexagons): used by MaxiCode (ISO/IEC 16023). Each
///     module becomes a ``PaintPath`` tracing six vertices.
///
/// ## Annotations
///
/// The ``AnnotatedModuleGrid`` record extends ``ModuleGrid`` with per-module
/// role strings useful for visualizers (highlighting finder patterns, data
/// codewords, etc.). Annotations are never required for rendering — the
/// renderer only looks at the boolean ``modules`` grid.

namespace CodingAdventures.Barcode2D

open CodingAdventures.PaintInstructions
open System

// ============================================================================
// ModuleShape discriminated union
// ============================================================================

/// The shape of each module in the grid.
///
///   - ``Square`` — the overwhelmingly common shape used by QR Code, Data
///     Matrix, Aztec Code, and PDF417. Each module renders as a filled square.
///
///   - ``Hex`` — flat-top hexagons used by MaxiCode (ISO/IEC 16023). MaxiCode
///     arranges modules in an offset-row grid and renders each as a filled
///     hexagon drawn with a ``PaintPath``.
///
/// The shape is stored on ``ModuleGrid`` so that ``Barcode2D.layout`` can pick
/// the right rendering path without the caller having to specify it again.
type ModuleShape =
    /// Square modules — used by QR Code, Data Matrix, Aztec Code, PDF417.
    | Square
    /// Flat-top hexagonal modules — used by MaxiCode (ISO/IEC 16023).
    | Hex

// ============================================================================
// ModuleGrid record
// ============================================================================

/// The universal intermediate representation produced by every 2D barcode
/// encoder. It is a 2-D boolean grid:
///
///   modules.[row].[col] = true   →  dark module (ink / filled)
///   modules.[row].[col] = false  →  light module (background / empty)
///
/// Row 0 is the top row. Column 0 is the leftmost column. This matches the
/// natural reading order used in every 2D barcode standard.
///
/// ### MaxiCode fixed size
///
/// MaxiCode grids are always 33 rows × 30 columns with ``moduleShape = Hex``.
/// Physical MaxiCode symbols are always approximately 1 inch × 1 inch.
///
/// ### Immutability
///
/// ``ModuleGrid`` is intentionally immutable. Use ``Barcode2D.setModule`` to
/// produce a new grid with one module changed, rather than mutating in place.
/// This makes encoders easy to test and compose.
type ModuleGrid =
    {
        /// Number of rows (height) in the grid.
        Rows: int
        /// Number of columns (width) in the grid.
        Cols: int
        /// Two-dimensional boolean grid. Access with ``modules.[row].[col]``.
        /// ``true`` = dark module, ``false`` = light module.
        Modules: bool[] array
        /// The shape used to render each module in pixel space.
        ModuleShape: ModuleShape
    }

// ============================================================================
// AnnotatedModuleGrid record
// ============================================================================

/// A ``ModuleGrid`` extended with per-module role annotations.
///
/// Used by visualizers to render colour-coded diagrams that teach how the
/// barcode is structured. The ``Roles`` array mirrors ``Modules`` exactly in
/// size: ``roles.[row].[col]`` corresponds to ``modules.[row].[col]``.
///
/// A ``None`` role means "no annotation for this module" — this can happen
/// when an encoder only annotates some modules (e.g. only the data region)
/// and leaves structural modules unannotated.
///
/// This type is NOT required for rendering. ``Barcode2D.layout`` accepts a
/// plain ``ModuleGrid`` and works identically whether or not annotations are
/// present.
type AnnotatedModuleGrid =
    {
        /// Underlying module grid (rows, cols, modules, moduleShape).
        Grid: ModuleGrid
        /// Per-module role strings. ``roles.[row].[col]`` is ``Some "finder"``,
        /// ``Some "data"``, etc., or ``None`` if unannotated.
        Roles: string option[] array
    }

// ============================================================================
// Barcode2DLayoutConfig record
// ============================================================================

/// Configuration for ``Barcode2D.layout``.
///
/// | Field              | Default     | Why                                       |
/// |--------------------|-------------|-------------------------------------------|
/// | ModuleSizePx       | 10.0        | Produces a readable QR at 210×210 px      |
/// | QuietZoneModules   | 4           | QR Code minimum per ISO/IEC 18004         |
/// | DarkColor          | "#000000"   | Black ink on white paper                  |
/// | LightColor         | "#ffffff"   | White paper                               |
///
/// ### ModuleSizePx
///
/// The size of one module in pixels. For square modules this is both width and
/// height. For hex modules it is the hexagon width (flat-to-flat, which equals
/// the side length for a regular hexagon).  Must be > 0.
///
/// ### QuietZoneModules
///
/// The number of module-width quiet-zone units added on each side of the grid.
/// QR Code requires a minimum of 4 modules. Data Matrix requires 1. MaxiCode
/// requires 1.  Must be >= 0.
type Barcode2DLayoutConfig =
    {
        /// Size of one module in pixels. Must be > 0.
        ModuleSizePx: float
        /// Quiet-zone width in module units on each side. Must be >= 0.
        QuietZoneModules: int
        /// Foreground (dark module) colour — any CSS colour string.
        DarkColor: string
        /// Background (light module / quiet zone) colour — any CSS colour string.
        LightColor: string
    }

// ============================================================================
// Barcode2D module
// ============================================================================

/// Functions for creating, updating, and rendering 2D barcode module grids.
///
/// Use ``[<RequireQualifiedAccess>]`` — all calls must be prefixed:
///   ``Barcode2D.makeModuleGrid``, ``Barcode2D.setModule``, ``Barcode2D.layout``.
[<RequireQualifiedAccess>]
module Barcode2D =

    // -----------------------------------------------------------------------
    // Version
    // -----------------------------------------------------------------------

    /// Package version, used by dependency smoke tests.
    [<Literal>]
    let VERSION = "0.1.0"

    // -----------------------------------------------------------------------
    // Default layout config
    // -----------------------------------------------------------------------

    /// Sensible defaults for ``layout``.
    ///
    /// These values are merged with any caller-supplied partial overrides. You
    /// can pass a record expression using ``{ Barcode2D.defaultConfig with ... }``
    /// to override only the fields you care about.
    let defaultConfig : Barcode2DLayoutConfig =
        {
            ModuleSizePx     = 10.0
            QuietZoneModules = 4
            DarkColor        = "#000000"
            LightColor       = "#ffffff"
        }

    // -----------------------------------------------------------------------
    // makeModuleGrid
    // -----------------------------------------------------------------------

    /// Create a new ``ModuleGrid`` of the given dimensions, with every module
    /// set to ``false`` (light).
    ///
    /// This is the starting point for every 2D barcode encoder. The encoder
    /// calls ``Barcode2D.makeModuleGrid`` and then uses ``Barcode2D.setModule``
    /// to paint dark modules one by one as it places finder patterns, timing
    /// strips, data bits, and error correction bits.
    ///
    /// ### Example — start a 21×21 QR Code v1 grid
    ///
    ///   let grid = Barcode2D.makeModuleGrid 21 21 Square
    ///   // grid.Modules.[0].[0] = false  (all light)
    ///   // grid.Rows = 21
    ///   // grid.Cols = 21
    ///
    /// Parameters:
    ///   rows        — number of rows (height of the grid)
    ///   cols        — number of columns (width of the grid)
    ///   moduleShape — shape of each module (``Square`` or ``Hex``)
    let makeModuleGrid (rows: int) (cols: int) (moduleShape: ModuleShape) : ModuleGrid =
        // Build a 2-D array of ``false`` values. Each row is an independent
        // array so that setModule can replace individual rows without copying
        // the entire grid.
        let modules = Array.init rows (fun _ -> Array.create cols false)
        { Rows = rows; Cols = cols; Modules = modules; ModuleShape = moduleShape }

    // -----------------------------------------------------------------------
    // setModule
    // -----------------------------------------------------------------------

    /// Return a new ``ModuleGrid`` identical to ``grid`` except that module at
    /// ``(row, col)`` is set to ``dark``.
    ///
    /// This function is **pure and immutable** — it never modifies the input
    /// grid. The original grid remains valid and unchanged. Only the affected
    /// row is re-allocated; all other rows are shared between old and new grids.
    ///
    /// ### Why immutability matters
    ///
    /// Barcode encoders often need to backtrack (e.g. trying different QR mask
    /// patterns). Immutable grids make this trivial — save the grid before
    /// trying a mask, evaluate it, discard if the score is worse, keep the old
    /// one if it is better. No undo stack needed.
    ///
    /// ### Out-of-bounds
    ///
    /// Raises ``ArgumentOutOfRangeException`` if ``row`` or ``col`` is outside
    /// the grid dimensions. This is a programming error in the encoder, not a
    /// user-facing error.
    ///
    /// ### Example
    ///
    ///   let g  = Barcode2D.makeModuleGrid 3 3 Square
    ///   let g2 = Barcode2D.setModule g 1 1 true
    ///   // g.Modules.[1].[1]  = false  (original unchanged)
    ///   // g2.Modules.[1].[1] = true
    ///   // g <> g2             (new record object)
    let setModule (grid: ModuleGrid) (row: int) (col: int) (dark: bool) : ModuleGrid =
        if row < 0 || row >= grid.Rows then
            raise (ArgumentOutOfRangeException("row", sprintf "setModule: row %d out of range [0, %d]" row (grid.Rows - 1)))
        if col < 0 || col >= grid.Cols then
            raise (ArgumentOutOfRangeException("col", sprintf "setModule: col %d out of range [0, %d]" col (grid.Cols - 1)))

        // Copy only the affected row; all other rows are shared (structural sharing).
        // This is the standard functional "copy-on-write" approach — cheap for
        // encoders that set modules one at a time.
        let newRow = Array.copy grid.Modules.[row]
        newRow.[col] <- dark

        let newModules =
            Array.init grid.Rows (fun i ->
                if i = row then newRow else grid.Modules.[i])

        { grid with Modules = newModules }

    // -----------------------------------------------------------------------
    // Internal helper: build a flat-top hexagon PathCommand list
    // -----------------------------------------------------------------------

    /// Build the six ``PathCommand`` values for a flat-top regular hexagon.
    ///
    /// A "flat-top" hexagon has two flat edges at the top and bottom:
    ///
    ///      ___
    ///     /   \       ← two vertices at top
    ///    |     |
    ///     \___/       ← two vertices at bottom
    ///
    /// For a flat-top hexagon centred at ``(cx, cy)`` with circumradius ``R``
    /// (centre to vertex):
    ///
    ///   Vertex i is at angle  i × 60°  from positive X:
    ///     xi = cx + R × cos(i × 60°)
    ///     yi = cy + R × sin(i × 60°)
    ///
    ///   Vertex table:
    ///     angle  cos    sin    role
    ///       0°    1      0     right midpoint
    ///      60°   0.5   √3/2   bottom-right
    ///     120°  -0.5   √3/2   bottom-left
    ///     180°  -1      0     left midpoint
    ///     240°  -0.5  -√3/2   top-left
    ///     300°   0.5  -√3/2   top-right
    ///
    /// The path starts with a ``MoveTo`` to vertex 0, followed by five
    /// ``LineTo`` commands to vertices 1–5, then a ``Close`` to return to
    /// vertex 0.
    let private buildFlatTopHexPath (cx: float) (cy: float) (circumR: float) : PathCommand list =
        let degToRad = Math.PI / 180.0

        // Vertex 0 — start the path with MoveTo.
        let angle0 = 0.0 * 60.0 * degToRad
        let v0 = MoveTo(cx + circumR * Math.Cos(angle0), cy + circumR * Math.Sin(angle0))

        // Vertices 1–5 — continue with LineTo for each.
        let rest =
            [ 1..5 ]
            |> List.map (fun i ->
                let angle = float i * 60.0 * degToRad
                LineTo(cx + circumR * Math.Cos(angle), cy + circumR * Math.Sin(angle)))

        // Close the path back to vertex 0.
        [ v0 ] @ rest @ [ Close ]

    // -----------------------------------------------------------------------
    // Internal: layoutSquare
    // -----------------------------------------------------------------------

    /// Render a square-module ``ModuleGrid`` into a ``PaintScene``.
    ///
    /// Algorithm:
    ///   1. Compute total pixel dimensions including quiet zone on all four
    ///      sides.
    ///   2. Emit one background ``Rect`` covering the entire symbol. This
    ///      ensures the quiet zone and light modules are filled with the
    ///      background colour even when the backend default is transparent.
    ///   3. For each dark module, emit one filled ``Rect``.
    ///
    /// Light modules are implicitly covered by the background rect — no
    /// explicit light rects are emitted. This keeps the instruction count
    /// proportional to the number of dark modules rather than the total grid
    /// size.
    let private layoutSquare (grid: ModuleGrid) (cfg: Barcode2DLayoutConfig) : PaintScene =
        let sz  = cfg.ModuleSizePx
        let qz  = float cfg.QuietZoneModules * sz

        // Total canvas dimensions including quiet zone on all four sides.
        let totalWidth  = float (grid.Cols + 2 * cfg.QuietZoneModules) * sz
        let totalHeight = float (grid.Rows + 2 * cfg.QuietZoneModules) * sz

        // 1. Background rect — covers the full symbol including quiet zone.
        let bgOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some cfg.LightColor }
        let bgRect    = PaintInstructions.paintRectWith bgOptions 0.0 0.0 totalWidth totalHeight

        // 2. One rect per dark module.
        let darkRects =
            [ for row in 0 .. grid.Rows - 1 do
                for col in 0 .. grid.Cols - 1 do
                    if grid.Modules.[row].[col] then
                        // Pixel origin of this module (top-left corner of its square).
                        let x = qz + float col * sz
                        let y = qz + float row * sz
                        let opts = { PaintInstructions.defaultPaintRectOptions with Fill = Some cfg.DarkColor }
                        yield PaintInstructions.paintRectWith opts x y sz sz ]

        let instructions = bgRect :: darkRects
        PaintInstructions.paintScene totalWidth totalHeight cfg.LightColor instructions

    // -----------------------------------------------------------------------
    // Internal: layoutHex
    // -----------------------------------------------------------------------

    /// Render a hex-module ``ModuleGrid`` into a ``PaintScene``.
    ///
    /// Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
    /// offset-row grid. Odd rows are shifted right by half a hexagon width.
    ///
    /// ### Flat-top hexagon geometry
    ///
    /// For a flat-top hexagon where side length = s:
    ///   hexWidth  = s            (flat-to-flat distance = side length)
    ///   hexHeight = s × (√3/2)  (vertical distance between row centres)
    ///   circumR   = s / √3      (centre-to-vertex circumscribed circle radius)
    ///
    /// Setting ``moduleSizePx = s`` means:
    ///   hexWidth  = moduleSizePx
    ///   hexHeight = moduleSizePx × (√3/2)
    ///   circumR   = moduleSizePx / √3
    ///
    /// ### Offset-row tiling
    ///
    ///   Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
    ///   Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
    ///   Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
    ///
    /// Odd rows are offset by ``hexWidth / 2`` to interlock with even rows.
    let private layoutHex (grid: ModuleGrid) (cfg: Barcode2DLayoutConfig) : PaintScene =
        let sz = cfg.ModuleSizePx

        // Hex geometry (see docstring above):
        let hexWidth  = sz
        let hexHeight = sz * (Math.Sqrt 3.0 / 2.0)
        let circumR   = sz / Math.Sqrt 3.0

        let qz = float cfg.QuietZoneModules * sz

        // Total canvas size. The ``+ hexWidth/2`` accounts for the odd-row
        // offset so the rightmost modules on odd rows don't clip outside the
        // canvas.
        let totalWidth  = float (grid.Cols + 2 * cfg.QuietZoneModules) * hexWidth + hexWidth / 2.0
        let totalHeight = float (grid.Rows + 2 * cfg.QuietZoneModules) * hexHeight

        // Background rect.
        let bgOptions = { PaintInstructions.defaultPaintRectOptions with Fill = Some cfg.LightColor }
        let bgRect    = PaintInstructions.paintRectWith bgOptions 0.0 0.0 totalWidth totalHeight

        // One PaintPath per dark module.
        let hexPaths =
            [ for row in 0 .. grid.Rows - 1 do
                for col in 0 .. grid.Cols - 1 do
                    if grid.Modules.[row].[col] then
                        // Centre of this hexagon in pixel space.
                        // Odd rows shift right by hexWidth/2.
                        let cx = qz + float col * hexWidth + float (row % 2) * (hexWidth / 2.0)
                        let cy = qz + float row * hexHeight
                        let commands = buildFlatTopHexPath cx cy circumR
                        let pathOpts = { PaintInstructions.defaultPaintPathOptions with Fill = Some cfg.DarkColor }
                        yield PaintInstructions.paintPathWith pathOpts commands ]

        let instructions = bgRect :: hexPaths
        PaintInstructions.paintScene totalWidth totalHeight cfg.LightColor instructions

    // -----------------------------------------------------------------------
    // layout — the main public API
    // -----------------------------------------------------------------------

    /// Convert a ``ModuleGrid`` into a ``PaintScene`` ready for the PaintVM.
    ///
    /// This is the **only** function in the entire 2D barcode stack that knows
    /// about pixels. Everything above this step works in abstract module units.
    /// Everything below this step is handled by the paint backend.
    ///
    /// ### Square modules (the common case)
    ///
    /// Each dark module at ``(row, col)`` becomes one ``Rect``:
    ///
    ///   quietZonePx = quietZoneModules × moduleSizePx
    ///   x = quietZonePx + col × moduleSizePx
    ///   y = quietZonePx + row × moduleSizePx
    ///
    /// Total symbol size (including quiet zone on all four sides):
    ///
    ///   totalWidth  = (cols + 2 × quietZoneModules) × moduleSizePx
    ///   totalHeight = (rows + 2 × quietZoneModules) × moduleSizePx
    ///
    /// ### Hex modules (MaxiCode)
    ///
    /// Each dark module at ``(row, col)`` becomes one ``Path`` tracing a
    /// flat-top regular hexagon.  Odd-numbered rows are offset by half a
    /// hexagon width to produce the standard hexagonal tiling.
    ///
    /// ### Validation
    ///
    /// Raises ``ArgumentException`` if:
    ///   - ``config.ModuleSizePx <= 0``
    ///   - ``config.QuietZoneModules < 0``
    ///   - ``config.ModuleShape`` (derived from ``grid.ModuleShape``) is
    ///     inconsistent with the grid (currently validated via the internal
    ///     dispatch; the grid's own shape drives which renderer is chosen)
    ///
    /// Parameters:
    ///   grid   — the module grid to render
    ///   config — layout configuration (use ``Barcode2D.defaultConfig`` as base)
    let layout (grid: ModuleGrid) (config: Barcode2DLayoutConfig) : PaintScene =
        // Validate config fields before any work is done.
        if config.ModuleSizePx <= 0.0 then
            raise (ArgumentException(sprintf "ModuleSizePx must be > 0, got %g" config.ModuleSizePx, "config"))
        if config.QuietZoneModules < 0 then
            raise (ArgumentException(sprintf "QuietZoneModules must be >= 0, got %d" config.QuietZoneModules, "config"))

        // Dispatch to the correct rendering path based on the grid's own shape.
        // This is the single decision point — no shape ambiguity is possible.
        match grid.ModuleShape with
        | Square -> layoutSquare grid config
        | Hex    -> layoutHex    grid config
