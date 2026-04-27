/// Barcode2D — Shared 2D barcode abstraction layer.
///
/// This package provides the two building blocks every 2D barcode format needs:
///
///   1. `ModuleGrid` — the universal intermediate representation produced by
///      every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
///      It is just a 2D boolean grid: true = dark module, false = light module.
///
///   2. `layout(grid:config:)` — the single function that converts abstract
///      module coordinates into pixel-level `PaintScene` instructions ready for
///      the PaintVM (P2D01) to render.
///
/// ## Where this fits in the pipeline
///
/// ```
/// Input data
///   → format encoder (qr-code, data-matrix, aztec…)
///   → ModuleGrid          ← produced by the encoder
///   → layout()            ← THIS PACKAGE converts to pixels
///   → PaintScene          ← consumed by paint-vm (P2D01)
///   → backend (SVG, Metal, Canvas, terminal…)
/// ```
///
/// All coordinates before `layout()` are measured in "module units" — abstract
/// grid steps. Only `layout()` multiplies by `moduleSizePx` to produce real
/// pixel coordinates. This means encoders never need to know anything about
/// screen resolution or output format.
///
/// ## Supported module shapes
///
/// - **Square** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
///   Each module becomes a `PaintRect`.
///
/// - **Hex** (flat-top hexagons): used by MaxiCode. Each module is approximated
///   with a square `PaintRect` of the same bounding width. The Swift
///   `PaintInstructions` layer only supports `PaintRect`, which matches the
///   spec requirement that the layout step use only P2D00 primitives.

import PaintInstructions

// MARK: - Version

/// Current package version.
public let version = "0.1.0"

// MARK: - ModuleShape

/// The shape of each module in the grid.
///
/// - `square`: used by QR Code, Data Matrix, Aztec Code, PDF417. The
///   overwhelmingly common shape. Each module renders as a filled square.
///
/// - `hex`: used by MaxiCode (ISO/IEC 16023). MaxiCode uses flat-top
///   hexagons arranged in an offset-row grid. Each module renders as a
///   `PaintRect` approximating the hexagon's bounding box.
///
/// The shape is stored on `ModuleGrid` so that `layout()` can pick the right
/// rendering path without the caller having to specify it again.
public enum ModuleShape: Equatable, Sendable {
    case square
    case hex
}

// MARK: - ModuleRole

/// The structural role of a module within its barcode symbol.
///
/// These roles are generic — they apply across all 2D barcode formats. Each
/// role corresponds to a distinct part of the symbol's anatomy:
///
/// - `finder`: a locator pattern that helps scanners detect and orient the
///   symbol. QR Code uses three 7×7 corner finder patterns.
///
/// - `separator`: a quiet-zone strip between a finder pattern and the data
///   area. Always light (false) in correctly encoded symbols.
///
/// - `timing`: an alternating dark/light calibration strip. Enables the
///   scanner to measure module size and compensate for perspective distortion.
///
/// - `alignment`: secondary locator patterns placed in high-version QR Code
///   symbols to correct for lens distortion.
///
/// - `format`: encodes ECC level + mask indicator (QR), layer count +
///   error mode (Aztec), or other symbol-level metadata.
///
/// - `data`: one bit of an encoded codeword. The message lives here.
///
/// - `ecc`: one bit of an error correction codeword (Reed-Solomon or
///   other ECC).
///
/// - `padding`: remainder/filler bits used to fill the grid when the
///   message is shorter than the symbol's capacity.
public enum ModuleRole: Equatable, Sendable {
    case finder
    case separator
    case timing
    case alignment
    case format
    case data
    case ecc
    case padding
}

// MARK: - ModuleGrid

/// The universal intermediate representation produced by every 2D barcode
/// encoder. It is a 2D boolean grid:
///
/// ```
/// modules[row][col] == true   →  dark module (ink / filled)
/// modules[row][col] == false  →  light module (background / empty)
/// ```
///
/// Row 0 is the top row. Column 0 is the leftmost column. This matches the
/// natural reading order used in every 2D barcode standard.
///
/// ### Immutability
///
/// `ModuleGrid` is intentionally a value type (struct). Use `setModule()`
/// to produce a new grid with one module changed, rather than mutating in
/// place. This makes encoders easy to test and compose.
public struct ModuleGrid: Equatable, Sendable {
    /// Width of the grid in modules.
    public let cols: Int
    /// Height of the grid in modules.
    public let rows: Int
    /// Two-dimensional boolean grid. Access with `modules[row][col]`.
    /// `true` = dark module, `false` = light module.
    public let modules: [[Bool]]
    /// The shape of each module. Must match the format being rendered.
    public let moduleShape: ModuleShape

    public init(cols: Int, rows: Int, modules: [[Bool]], moduleShape: ModuleShape) {
        self.cols = cols
        self.rows = rows
        self.modules = modules
        self.moduleShape = moduleShape
    }
}

// MARK: - ModuleAnnotation

/// Per-module role annotation, used by visualizers to colour-code the symbol.
///
/// Annotations are entirely optional. The renderer (`layout()`) only reads
/// `ModuleGrid.modules`; it never looks at annotations.
///
/// ### codewordIndex and bitIndex
///
/// For `data` and `ecc` modules, these identify exactly which bit in which
/// codeword this module encodes. Useful for visualizers that highlight one
/// codeword at a time.
///
/// - `codewordIndex`: zero-based index into the final interleaved codeword stream.
/// - `bitIndex`: zero-based bit index within that codeword, 0 = MSB.
///
/// For structural modules (`finder`, `timing`, etc.) these are nil.
///
/// ### metadata
///
/// An escape hatch for format-specific annotations. For example:
/// - QR Code dark module: `["format_role": "qr:dark-module"]`
/// - Aztec mode message: `["format_role": "aztec:mode-message"]`
public struct ModuleAnnotation: Equatable, Sendable {
    public let role: ModuleRole
    public let dark: Bool
    /// Zero-based codeword index (nil for structural modules).
    public let codewordIndex: Int?
    /// Zero-based bit index within the codeword (nil for structural modules).
    public let bitIndex: Int?
    /// Arbitrary format-specific key/value pairs.
    public let metadata: [String: String]

    public init(
        role: ModuleRole,
        dark: Bool,
        codewordIndex: Int? = nil,
        bitIndex: Int? = nil,
        metadata: [String: String] = [:]
    ) {
        self.role = role
        self.dark = dark
        self.codewordIndex = codewordIndex
        self.bitIndex = bitIndex
        self.metadata = metadata
    }
}

// MARK: - AnnotatedModuleGrid

/// A `ModuleGrid` extended with per-module role annotations.
///
/// Used by visualizers to render colour-coded diagrams. The `annotations`
/// array mirrors `modules` exactly in size: `annotations[row][col]`
/// corresponds to `modules[row][col]`.
///
/// A `nil` annotation means "no annotation for this module" — this can
/// happen when an encoder only annotates some modules and leaves structural
/// modules un-annotated.
///
/// This type is NOT required for rendering. `layout()` accepts a plain
/// `ModuleGrid` and works identically whether or not annotations are present.
public struct AnnotatedModuleGrid: Equatable, Sendable {
    public let grid: ModuleGrid
    public let annotations: [[ModuleAnnotation?]]

    public init(grid: ModuleGrid, annotations: [[ModuleAnnotation?]]) {
        self.grid = grid
        self.annotations = annotations
    }
}

// MARK: - Barcode2DLayoutConfig

/// Configuration for `layout()`.
///
/// All fields have sensible defaults — just use `Barcode2DLayoutConfig()` and
/// override only what you need.
///
/// | Field              | Default     | Why                                   |
/// |--------------------|-------------|---------------------------------------|
/// | moduleSizePx       | 10.0        | Produces a readable QR at 210×210 px  |
/// | quietZoneModules   | 4           | QR Code minimum per ISO/IEC 18004     |
/// | foreground         | "#000000"   | Black ink on white paper              |
/// | background         | "#ffffff"   | White paper                           |
/// | showAnnotations    | false       | Off by default; opt-in for visualizers|
/// | moduleShape        | .square     | The overwhelmingly common case        |
///
/// ### moduleSizePx
///
/// The size of one module in pixels. For square modules this is both width and
/// height. Must be > 0.
///
/// ### quietZoneModules
///
/// The number of module-width quiet-zone units added on each side of the grid.
/// QR Code requires a minimum of 4 modules. Data Matrix requires 1. Must be >= 0.
///
/// ### moduleShape
///
/// Must match `ModuleGrid.moduleShape`. If they disagree, `layout()` throws
/// `Barcode2DError.invalidConfig`. This double-check prevents accidentally
/// rendering a MaxiCode hex grid with square modules.
public struct Barcode2DLayoutConfig: Equatable, Sendable {
    public var moduleSizePx: Double
    public var quietZoneModules: Int
    public var foreground: String
    public var background: String
    public var showAnnotations: Bool
    public var moduleShape: ModuleShape

    public init(
        moduleSizePx: Double = 10.0,
        quietZoneModules: Int = 4,
        foreground: String = "#000000",
        background: String = "#ffffff",
        showAnnotations: Bool = false,
        moduleShape: ModuleShape = .square
    ) {
        self.moduleSizePx = moduleSizePx
        self.quietZoneModules = quietZoneModules
        self.foreground = foreground
        self.background = background
        self.showAnnotations = showAnnotations
        self.moduleShape = moduleShape
    }
}

// MARK: - Barcode2DError

/// Errors thrown by Barcode2D functions.
public enum Barcode2DError: Error, Equatable {
    /// The configuration is invalid — for example:
    /// - `moduleSizePx <= 0`
    /// - `quietZoneModules < 0`
    /// - `config.moduleShape` does not match `grid.moduleShape`
    case invalidConfig(String)
}

// MARK: - makeModuleGrid

/// Create a new `ModuleGrid` of the given dimensions, with every module set
/// to `false` (light).
///
/// This is the starting point for every 2D barcode encoder. The encoder calls
/// `makeModuleGrid(rows:cols:)` and then uses `setModule()` to paint dark
/// modules one by one as it places finder patterns, timing strips, data bits,
/// and error correction bits.
///
/// ### Example — start a 21×21 QR Code v1 grid
///
/// ```swift
/// var grid = makeModuleGrid(rows: 21, cols: 21)
/// // grid.modules[0][0] == false  (all light)
/// // grid.rows == 21
/// // grid.cols == 21
/// ```
///
/// - Parameters:
///   - rows: Number of rows (height of the grid).
///   - cols: Number of columns (width of the grid).
///   - moduleShape: Shape of each module. Defaults to `.square`.
public func makeModuleGrid(
    rows: Int,
    cols: Int,
    moduleShape: ModuleShape = .square
) -> ModuleGrid {
    // Build a 2D array of `false` values. Each row is an independent array
    // so that `setModule()` can replace individual rows without copying the
    // entire grid.
    let modules = [[Bool]](
        repeating: [Bool](repeating: false, count: cols),
        count: rows
    )
    return ModuleGrid(cols: cols, rows: rows, modules: modules, moduleShape: moduleShape)
}

// MARK: - setModule

/// Return a new `ModuleGrid` identical to `grid` except that the module at
/// `(row, col)` is set to `dark`.
///
/// This function is **pure and immutable** — it never modifies the input grid.
/// The original grid remains valid and unchanged.
///
/// ### Why immutability matters
///
/// Barcode encoders often need to backtrack (e.g. trying different QR mask
/// patterns). Immutable grids make this trivial — save the grid before trying
/// a mask, evaluate it, discard if the score is worse, keep the old one if it
/// is better. No undo stack needed.
///
/// ### Out-of-bounds
///
/// Throws `Barcode2DError.invalidConfig` if `row` or `col` is outside the
/// grid dimensions.
///
/// ### Example
///
/// ```swift
/// let g = makeModuleGrid(rows: 3, cols: 3)
/// let g2 = try setModule(grid: g, row: 1, col: 1, dark: true)
/// // g.modules[1][1] == false  (original unchanged)
/// // g2.modules[1][1] == true
/// ```
///
/// - Parameters:
///   - grid: The source grid to copy from.
///   - row: Zero-based row index.
///   - col: Zero-based column index.
///   - dark: `true` to set a dark module, `false` for a light module.
/// - Returns: A new `ModuleGrid` with the specified module updated.
/// - Throws: `Barcode2DError.invalidConfig` if `row` or `col` is out of bounds.
public func setModule(grid: ModuleGrid, row: Int, col: Int, dark: Bool) throws -> ModuleGrid {
    guard row >= 0 && row < grid.rows else {
        throw Barcode2DError.invalidConfig(
            "setModule: row \(row) out of range [0, \(grid.rows - 1)]"
        )
    }
    guard col >= 0 && col < grid.cols else {
        throw Barcode2DError.invalidConfig(
            "setModule: col \(col) out of range [0, \(grid.cols - 1)]"
        )
    }

    // Copy only the affected row; all other rows are shared (value-copy semantics
    // for arrays in Swift handles this efficiently for small changes).
    var newModules = grid.modules
    newModules[row][col] = dark

    return ModuleGrid(
        cols: grid.cols,
        rows: grid.rows,
        modules: newModules,
        moduleShape: grid.moduleShape
    )
}

// MARK: - layout

/// Convert a `ModuleGrid` into a `PaintScene` ready for the PaintVM.
///
/// This is the **only** function in the entire 2D barcode stack that knows
/// about pixels. Everything above this step works in abstract module units.
/// Everything below this step is handled by the paint backend.
///
/// ### Square modules (the common case)
///
/// Each dark module at `(row, col)` becomes one `PaintRect`:
///
/// ```
/// quietZonePx = quietZoneModules * moduleSizePx
/// x = quietZonePx + col * moduleSizePx
/// y = quietZonePx + row * moduleSizePx
/// ```
///
/// Total symbol size (including quiet zone on all four sides):
///
/// ```
/// totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
/// totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
/// ```
///
/// The scene always starts with one background `PaintRect` covering the full
/// symbol. This ensures the quiet zone and light modules are filled with the
/// background color even if the backend has a transparent default.
///
/// ### Hex modules (MaxiCode)
///
/// For MaxiCode-style hex grids, each dark module at `(row, col)` becomes
/// one `PaintRect`. The rect is positioned using hexagonal tiling geometry,
/// but rendered as a rectangle — an appropriate approximation within the
/// P2D00 rect-only instruction set:
///
/// ```
/// hexWidth  = moduleSizePx
/// hexHeight = moduleSizePx * (√3 / 2)
/// cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2)
/// cy = quietZonePx + row * hexHeight
/// ```
///
/// Odd-numbered rows are offset by `hexWidth / 2` to produce standard
/// hexagonal tiling spacing.
///
/// ### Validation
///
/// Throws `Barcode2DError.invalidConfig` if:
/// - `moduleSizePx <= 0`
/// - `quietZoneModules < 0`
/// - `config.moduleShape != grid.moduleShape`
///
/// - Parameters:
///   - grid: The module grid to render.
///   - config: Layout configuration. Uses sensible defaults if omitted.
/// - Returns: A `PaintScene` ready for the PaintVM.
/// - Throws: `Barcode2DError.invalidConfig` if any configuration is invalid.
public func layout(
    grid: ModuleGrid,
    config: Barcode2DLayoutConfig = Barcode2DLayoutConfig()
) throws -> PaintScene {
    // ── Validation ──────────────────────────────────────────────────────────
    guard config.moduleSizePx > 0 else {
        throw Barcode2DError.invalidConfig(
            "moduleSizePx must be > 0, got \(config.moduleSizePx)"
        )
    }
    guard config.quietZoneModules >= 0 else {
        throw Barcode2DError.invalidConfig(
            "quietZoneModules must be >= 0, got \(config.quietZoneModules)"
        )
    }
    guard config.moduleShape == grid.moduleShape else {
        throw Barcode2DError.invalidConfig(
            "config.moduleShape \"\(config.moduleShape)\" does not match grid.moduleShape \"\(grid.moduleShape)\""
        )
    }

    // Dispatch to the correct rendering path based on module shape.
    switch config.moduleShape {
    case .square:
        return layoutSquare(grid: grid, config: config)
    case .hex:
        return layoutHex(grid: grid, config: config)
    }
}

// MARK: - layoutSquare (internal)

/// Render a square-module `ModuleGrid` into a `PaintScene`.
///
/// Called only by `layout()` after validation.
///
/// The algorithm is straightforward:
///
/// 1. Compute total pixel dimensions including quiet zone.
/// 2. Emit one background `PaintRect` covering the entire symbol.
/// 3. For each dark module, emit one filled `PaintRect`.
///
/// Light modules are implicitly covered by the background rect — no explicit
/// light rects are emitted. This keeps the instruction count proportional to
/// the number of dark modules rather than the total grid size.
private func layoutSquare(grid: ModuleGrid, config: Barcode2DLayoutConfig) -> PaintScene {
    let moduleSizePx = config.moduleSizePx
    let quietZoneModules = config.quietZoneModules

    // Quiet zone in pixels on each side.
    let quietZonePx = Double(quietZoneModules) * moduleSizePx

    // Total canvas dimensions including quiet zone on all four sides.
    //
    // Example: a 21×21 QR Code with moduleSizePx=10 and quietZoneModules=4:
    //   totalWidth = (21 + 2*4) * 10 = 290 pixels
    let totalWidth = Double(grid.cols + 2 * quietZoneModules) * moduleSizePx
    let totalHeight = Double(grid.rows + 2 * quietZoneModules) * moduleSizePx

    var instructions: [PaintInstruction] = []

    // 1. Background: a single rect covering the entire symbol including quiet
    //    zone. This ensures light modules and the quiet zone are always filled,
    //    even when the backend default is transparent.
    instructions.append(paintRect(
        x: 0,
        y: 0,
        width: Int(totalWidth.rounded()),
        height: Int(totalHeight.rounded()),
        fill: config.background
    ))

    // 2. One PaintRect per dark module.
    let modulePx = Int(moduleSizePx.rounded())
    for row in 0..<grid.rows {
        for col in 0..<grid.cols {
            if grid.modules[row][col] {
                // Pixel origin of this module (top-left corner of its square).
                let x = Int((quietZonePx + Double(col) * moduleSizePx).rounded())
                let y = Int((quietZonePx + Double(row) * moduleSizePx).rounded())

                instructions.append(paintRect(
                    x: x,
                    y: y,
                    width: modulePx,
                    height: modulePx,
                    fill: config.foreground
                ))
            }
        }
    }

    return paintScene(
        width: Int(totalWidth.rounded()),
        height: Int(totalHeight.rounded()),
        instructions: instructions,
        background: config.background
    )
}

// MARK: - layoutHex (internal)

/// Render a hex-module `ModuleGrid` into a `PaintScene`.
///
/// Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
/// offset-row grid. Odd rows are shifted right by half a hexagon width.
///
/// ### Flat-top hexagon geometry
///
/// A "flat-top" hexagon has two flat edges at the top and bottom:
///
/// ```
///    ___
///   /   \      ← two vertices at top
///  |     |
///   \___/      ← two vertices at bottom
/// ```
///
/// For a flat-top hexagon with width = `moduleSizePx`:
///
/// ```
/// hexWidth  = moduleSizePx             (flat-to-flat distance)
/// hexHeight = moduleSizePx * (√3 / 2)  (vertical distance between row centers)
/// ```
///
/// ### Tiling
///
/// Hex grids tile by setting `hexHeight = moduleSizePx * √3/2`. Odd rows are
/// offset by `hexWidth / 2` to interlock with even rows:
///
/// ```
/// Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
/// Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
/// Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
/// ```
///
/// ### PaintRect approximation
///
/// The `PaintInstructions` layer (P2D00) uses only `PaintRect`. Rather than
/// path commands for actual hexagons, each dark module is represented as a
/// square rect at the hexagon's center position. This matches the P2D00
/// spec's requirement to use only rect primitives.
private func layoutHex(grid: ModuleGrid, config: Barcode2DLayoutConfig) -> PaintScene {
    let moduleSizePx = config.moduleSizePx
    let quietZoneModules = config.quietZoneModules

    // Hex geometry:
    //   hexWidth  = one module width (flat-to-flat = side length for regular hex)
    //   hexHeight = vertical distance between row centers
    let hexWidth = moduleSizePx
    let hexHeight = moduleSizePx * (3.0.squareRoot() / 2.0)

    let quietZonePx = Double(quietZoneModules) * moduleSizePx

    // Total canvas size. The +hexWidth/2 accounts for the odd-row offset so
    // the rightmost modules on odd rows don't clip outside the canvas.
    let totalWidth = Double(grid.cols + 2 * quietZoneModules) * hexWidth + hexWidth / 2.0
    let totalHeight = Double(grid.rows + 2 * quietZoneModules) * hexHeight

    var instructions: [PaintInstruction] = []

    // Background rect.
    instructions.append(paintRect(
        x: 0,
        y: 0,
        width: Int(totalWidth.rounded()),
        height: Int(totalHeight.rounded()),
        fill: config.background
    ))

    let modulePx = Int(moduleSizePx.rounded())

    // One PaintRect per dark module, positioned at the hexagon's center with
    // the hexagonal tiling offset applied.
    for row in 0..<grid.rows {
        for col in 0..<grid.cols {
            if grid.modules[row][col] {
                // Center of this hexagon in pixel space.
                // Odd rows shift right by hexWidth/2 to interlock with even rows.
                let cx = quietZonePx + Double(col) * hexWidth + (Double(row % 2)) * (hexWidth / 2.0)
                let cy = quietZonePx + Double(row) * hexHeight

                // Top-left corner of the bounding rect for this hexagon.
                let x = Int((cx - hexWidth / 2.0).rounded())
                let y = Int((cy - hexHeight / 2.0).rounded())

                instructions.append(paintRect(
                    x: x,
                    y: y,
                    width: modulePx,
                    height: Int((hexHeight).rounded()),
                    fill: config.foreground
                ))
            }
        }
    }

    return paintScene(
        width: Int(totalWidth.rounded()),
        height: Int(totalHeight.rounded()),
        instructions: instructions,
        background: config.background
    )
}
