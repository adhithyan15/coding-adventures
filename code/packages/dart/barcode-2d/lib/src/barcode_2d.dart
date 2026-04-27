/// Shared 2D barcode abstraction layer.
///
/// ## Overview
///
/// This library answers one question: "Given an abstract grid of dark/light
/// modules, how do I produce pixel-level paint instructions?"
///
/// Two concepts are involved:
///
///   - **ModuleGrid** — an immutable 2D boolean grid. `true` = dark module
///     (ink), `false` = light module (background). Every 2D barcode encoder
///     produces one of these.
///
///   - **layout()** — the only place in the entire barcode stack that knows
///     about pixels. It reads `moduleSizePx` and `quietZoneModules` from a
///     config, multiplies, and emits a `PaintScene`.
///
/// ## Supported module shapes
///
/// | Shape    | Standard(s)               | Instruction type |
/// |----------|---------------------------|------------------|
/// | `square` | QR, Data Matrix, Aztec,   | `PaintRect`      |
/// |          | PDF417 (all rectangular)  |                  |
/// | `hex`    | MaxiCode (ISO/IEC 16023)  | `PaintPath`      |
///
/// ## Immutability and the encoder pattern
///
/// All grids are immutable. Encoders call [makeModuleGrid] once, then
/// repeatedly call [setModule] to build up the grid one dark module at a time.
/// Each [setModule] call returns a **new** grid; the original is unchanged.
///
/// This pattern makes encoders easy to test (snapshot grids at any step) and
/// supports backtracking (e.g. QR mask evaluation: try each mask, score it,
/// keep the best one — no undo stack needed).
///
/// ## Hex grid geometry quick reference
///
/// MaxiCode uses flat-top hexagons in an offset-row tiling:
///
/// ```
/// Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡    (no offset)
/// Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡   (shifted right by hexWidth/2)
/// Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡    (no offset)
/// ```
///
/// For `moduleSizePx = s`:
///   hexWidth  = s
///   hexHeight = s × (√3 / 2)   (vertical distance between row centres)
///   circumR   = s / √3          (centre to vertex)
library barcode_2d;

import 'dart:math' as math;

import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

// Re-export PaintScene so callers can type the return value of layout()
// without needing to import paint-instructions themselves.
export 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart'
    show PaintScene;

// ============================================================================
// Package version
// ============================================================================

/// Package version following Semantic Versioning 2.0.
const String version = '0.1.0';

// ============================================================================
// ModuleShape
// ============================================================================

/// The shape used to render each module in a [ModuleGrid].
///
/// - `square` — the overwhelming majority of 2D barcode formats use
///   rectangular modules (QR Code, Data Matrix, Aztec Code, PDF417).
///   Each module becomes a `PaintRect`.
///
/// - `hex` — MaxiCode (ISO/IEC 16023) uses flat-top hexagons arranged in an
///   offset-row tiling. Each module becomes a `PaintPath` with six vertices.
///
/// The shape is stored on the [ModuleGrid] so [layout] can pick the right
/// rendering path without the caller having to specify it again.
enum ModuleShape {
  /// Axis-aligned square modules (used by QR, Data Matrix, Aztec, PDF417).
  square,

  /// Flat-top hexagonal modules (used by MaxiCode).
  hex,
}

// ============================================================================
// ModuleRole
// ============================================================================

/// The structural role of a module within its barcode symbol.
///
/// Used by [ModuleAnnotation] for visualizer colour-coding. Roles are generic
/// across all 2D barcode formats. Format-specific roles (e.g. QR dark module,
/// Aztec mode message) are stored in [ModuleAnnotation.metadata] as strings
/// like `"qr:dark-module"`.
///
/// | Role        | Description                                              |
/// |-------------|----------------------------------------------------------|
/// | finder      | Locator patterns used for symbol orientation detection   |
/// | separator   | Quiet strip between finder and data (always light)       |
/// | timing      | Alternating strip for module size calibration            |
/// | alignment   | Secondary locators in high-version QR symbols            |
/// | format      | Encodes ECC level + mask / layer count + error mode      |
/// | data        | One bit of an encoded message codeword                   |
/// | ecc         | One bit of an error correction codeword                  |
/// | padding     | Filler bits to complete the grid                         |
enum ModuleRole {
  finder,
  separator,
  timing,
  alignment,
  format,
  data,
  ecc,
  padding,
}

// ============================================================================
// ModuleAnnotation
// ============================================================================

/// Per-module role annotation used by visualizers to colour-code symbols.
///
/// Annotations are entirely optional — [layout] never reads them. They exist
/// purely for educational visualizers that want to highlight which part of the
/// grid is a finder pattern vs. a data codeword.
///
/// ### codewordIndex and bitIndex
///
/// For `data` and `ecc` modules these identify exactly which bit of which
/// codeword is stored here:
///   - `codewordIndex` — zero-based index into the interleaved codeword stream.
///   - `bitIndex` — zero-based bit index within that codeword (0 = MSB).
///
/// ### metadata
///
/// Escape hatch for format-specific annotations:
///   - `{"format_role": "qr:dark-module"}`
///   - `{"format_role": "aztec:mode-message"}`
///   - `{"format_role": "pdf417:row-indicator", "row": "4"}`
final class ModuleAnnotation {
  /// The structural role of this module.
  final ModuleRole role;

  /// Whether this module is dark at the time of annotation.
  ///
  /// This can differ from the grid's boolean value after masking — the
  /// annotation records the logical role, not the visual state.
  final bool dark;

  /// Zero-based codeword index (only set for `data` and `ecc` roles).
  final int? codewordIndex;

  /// Zero-based bit index within the codeword (only set for `data`/`ecc`).
  final int? bitIndex;

  /// Format-specific metadata (e.g. `{"format_role": "qr:dark-module"}`).
  final Map<String, String>? metadata;

  const ModuleAnnotation({
    required this.role,
    required this.dark,
    this.codewordIndex,
    this.bitIndex,
    this.metadata,
  });
}

// ============================================================================
// ModuleGrid
// ============================================================================

/// The universal intermediate representation of a 2D barcode symbol.
///
/// A `ModuleGrid` is an immutable 2D boolean matrix:
/// ```
/// modules[row][col] == true   →  dark module (ink / filled)
/// modules[row][col] == false  →  light module (background / empty)
/// ```
///
/// Row 0 is the top row. Column 0 is the leftmost column. This matches the
/// natural reading order of every 2D barcode standard.
///
/// ### Immutability
///
/// `ModuleGrid` is intentionally immutable. Use [setModule] to produce a new
/// grid with one module changed, rather than mutating in place. This keeps
/// encoder logic pure and supports backtracking (e.g. QR mask evaluation).
///
/// ### Creating grids
///
/// ```dart
/// // All-light 21×21 grid (QR Code v1 size):
/// final grid = makeModuleGrid(rows: 21, cols: 21);
///
/// // Set top-left corner dark:
/// final grid2 = setModule(grid, row: 0, col: 0, dark: true);
/// ```
///
/// ### MaxiCode note
///
/// MaxiCode grids are always 33 rows × 30 columns with
/// `moduleShape: ModuleShape.hex`. Physical MaxiCode symbols are always
/// approximately 1 inch × 1 inch.
final class ModuleGrid {
  /// Number of rows (height of the grid in modules).
  final int rows;

  /// Number of columns (width of the grid in modules).
  final int cols;

  /// The 2D boolean grid. Access with `modules[row][col]`.
  ///
  /// `true` = dark module, `false` = light module.
  ///
  /// The outer list is indexed by row; each inner list is indexed by column.
  /// Both lists are unmodifiable — attempting to mutate them throws
  /// [UnsupportedError].
  final List<List<bool>> modules;

  /// The shape used to render each module.
  final ModuleShape moduleShape;

  /// Create a grid directly. Prefer [makeModuleGrid] instead.
  ///
  /// The `modules` list must have exactly `rows` rows, each with `cols` booleans.
  const ModuleGrid({
    required this.rows,
    required this.cols,
    required this.modules,
    required this.moduleShape,
  });
}

// ============================================================================
// AnnotatedModuleGrid
// ============================================================================

/// A [ModuleGrid] extended with per-module role annotations.
///
/// Used by visualizers to render colour-coded diagrams of barcode structure.
/// The `annotations` list mirrors `modules` exactly:
/// `annotations[row][col]` corresponds to `modules[row][col]`.
///
/// A `null` annotation means "no annotation for this module" — this happens
/// when an encoder annotates only part of the grid (e.g. only the data region).
///
/// [layout] never reads annotations — it works identically on plain
/// [ModuleGrid] and [AnnotatedModuleGrid].
final class AnnotatedModuleGrid extends ModuleGrid {
  /// Per-module annotations. `null` = no annotation for that module.
  final List<List<ModuleAnnotation?>> annotations;

  const AnnotatedModuleGrid({
    required super.rows,
    required super.cols,
    required super.modules,
    required super.moduleShape,
    required this.annotations,
  });
}

// ============================================================================
// Barcode2DLayoutConfig
// ============================================================================

/// Configuration for [layout].
///
/// All fields are optional — pass named parameters and the defaults from
/// [defaultBarcode2DLayoutConfig] fill any gaps you omit.
///
/// | Field            | Default     | Why                                    |
/// |------------------|-------------|----------------------------------------|
/// | moduleSizePx     | 10          | Produces a readable QR at ~210×210 px  |
/// | quietZoneModules | 4           | QR Code minimum per ISO/IEC 18004      |
/// | foreground       | `"#000000"` | Black ink on white paper               |
/// | background       | `"#ffffff"` | White paper                            |
/// | moduleShape      | `square`    | The overwhelmingly common case         |
///
/// ### moduleSizePx
///
/// The side length of one module in pixels. For square modules this is both
/// width and height. For hex modules this is the flat-to-flat diameter
/// (which equals the side length of a regular hexagon).
///
/// Must be > 0.
///
/// ### quietZoneModules
///
/// The number of module-width quiet-zone units added on each side of the grid.
/// QR Code requires ≥ 4. Data Matrix requires ≥ 1. MaxiCode requires ≥ 1.
///
/// Must be ≥ 0.
///
/// ### moduleShape
///
/// Must match [ModuleGrid.moduleShape]. If they disagree, [layout] throws
/// [InvalidBarcode2DConfigError]. This double-check prevents accidentally
/// rendering a MaxiCode hex grid with square modules.
final class Barcode2DLayoutConfig {
  /// Size of one module in pixels. Must be > 0.
  final int moduleSizePx;

  /// Number of quiet-zone module-widths added on each side. Must be ≥ 0.
  final int quietZoneModules;

  /// CSS hex fill colour for dark modules. Default `"#000000"`.
  final String foreground;

  /// CSS hex fill colour for the background / light modules. Default `"#ffffff"`.
  final String background;

  /// The module shape. Must match the grid's [ModuleGrid.moduleShape].
  final ModuleShape moduleShape;

  const Barcode2DLayoutConfig({
    required this.moduleSizePx,
    required this.quietZoneModules,
    required this.foreground,
    required this.background,
    required this.moduleShape,
  });

  /// Produce a new config identical to this one but with the given field(s)
  /// replaced. Useful for one-off overrides without repeating every field.
  ///
  /// ```dart
  /// final big = defaultBarcode2DLayoutConfig.copyWith(moduleSizePx: 20);
  /// ```
  Barcode2DLayoutConfig copyWith({
    int? moduleSizePx,
    int? quietZoneModules,
    String? foreground,
    String? background,
    ModuleShape? moduleShape,
  }) {
    return Barcode2DLayoutConfig(
      moduleSizePx: moduleSizePx ?? this.moduleSizePx,
      quietZoneModules: quietZoneModules ?? this.quietZoneModules,
      foreground: foreground ?? this.foreground,
      background: background ?? this.background,
      moduleShape: moduleShape ?? this.moduleShape,
    );
  }
}

/// Sensible defaults for [layout].
///
/// `moduleSizePx=10` produces a 210×210 px scene for a standard 21-module
/// QR Code v1 with the required 4-module quiet zone on each side.
const Barcode2DLayoutConfig defaultBarcode2DLayoutConfig = Barcode2DLayoutConfig(
  moduleSizePx: 10,
  quietZoneModules: 4,
  foreground: '#000000',
  background: '#ffffff',
  moduleShape: ModuleShape.square,
);

// ============================================================================
// Error types
// ============================================================================

/// Base class for all barcode-2d errors.
///
/// Catch this type to handle any error from this library without caring about
/// the specific subtype.
final class Barcode2DError implements Exception {
  /// Human-readable description of the error.
  final String message;

  const Barcode2DError(this.message);

  @override
  String toString() => 'Barcode2DError: $message';
}

/// Thrown by [layout] when the configuration is invalid.
///
/// Possible causes:
/// - `moduleSizePx <= 0`
/// - `quietZoneModules < 0`
/// - `config.moduleShape` does not match `grid.moduleShape`
final class InvalidBarcode2DConfigError extends Barcode2DError {
  const InvalidBarcode2DConfigError(super.message);

  @override
  String toString() => 'InvalidBarcode2DConfigError: $message';
}

// ============================================================================
// makeModuleGrid — create an all-light grid
// ============================================================================

/// Create a new [ModuleGrid] with every module set to `false` (light).
///
/// This is the starting point for every 2D barcode encoder. The encoder calls
/// [makeModuleGrid] once, then uses [setModule] to paint dark modules one by
/// one as it places finder patterns, timing strips, data bits, and ECC bits.
///
/// ### Example — start a 21×21 QR Code v1 grid
///
/// ```dart
/// final grid = makeModuleGrid(rows: 21, cols: 21);
/// // grid.modules[0][0] == false  (all light)
/// // grid.rows == 21
/// // grid.cols == 21
/// ```
///
/// @param rows        Number of rows (height of the grid in modules).
/// @param cols        Number of columns (width of the grid in modules).
/// @param moduleShape Shape of each module. Defaults to [ModuleShape.square].
ModuleGrid makeModuleGrid({
  required int rows,
  required int cols,
  ModuleShape moduleShape = ModuleShape.square,
}) {
  // Build a 2D list of `false` values.
  //
  // Each row is an independent List<bool> so that setModule() can replace a
  // single row without copying the entire grid (structural sharing).
  final modules = List.generate(
    rows,
    (_) => List<bool>.filled(cols, false),
    growable: false,
  );
  return ModuleGrid(
    rows: rows,
    cols: cols,
    modules: modules,
    moduleShape: moduleShape,
  );
}

// ============================================================================
// setModule — immutable single-module update
// ============================================================================

/// Return a new [ModuleGrid] identical to `grid` except that the module at
/// (`row`, `col`) is set to `dark`.
///
/// This function is **pure and immutable** — it never modifies the input grid.
/// The original grid remains valid and unchanged. Only the affected row is
/// re-allocated; all other rows are shared between old and new grids (structural
/// sharing keeps memory usage proportional to the number of updates, not total
/// grid size).
///
/// ### Why immutability matters
///
/// QR Code encoders try all eight mask patterns and keep the best-scoring one.
/// With mutable grids you'd need an undo stack or would copy the whole grid
/// before each trial. With immutable grids you simply save the reference before
/// masking and discard it if the score is worse.
///
/// ### Out-of-bounds
///
/// Throws [RangeError] if `row` or `col` is outside the grid dimensions. This
/// is always a programming error in the encoder, not a user-facing condition.
///
/// ### Example
///
/// ```dart
/// final g1 = makeModuleGrid(rows: 3, cols: 3);
/// final g2 = setModule(g1, row: 1, col: 1, dark: true);
/// // g1.modules[1][1] == false  (original unchanged)
/// // g2.modules[1][1] == true   (new grid)
/// // identical(g1, g2) == false (different objects)
/// ```
ModuleGrid setModule(
  ModuleGrid grid, {
  required int row,
  required int col,
  required bool dark,
}) {
  // Bounds check: catch encoder bugs early with a clear error message.
  if (row < 0 || row >= grid.rows) {
    throw RangeError(
      'setModule: row $row is out of range [0, ${grid.rows - 1}]',
    );
  }
  if (col < 0 || col >= grid.cols) {
    throw RangeError(
      'setModule: col $col is out of range [0, ${grid.cols - 1}]',
    );
  }

  // Copy only the affected row; all other rows are structurally shared.
  final newRow = List<bool>.from(grid.modules[row]);
  newRow[col] = dark;

  // Build a new modules list: reuse existing rows for unchanged rows.
  final newModules = List<List<bool>>.generate(
    grid.rows,
    (i) => i == row ? newRow : grid.modules[i],
    growable: false,
  );

  return ModuleGrid(
    rows: grid.rows,
    cols: grid.cols,
    modules: newModules,
    moduleShape: grid.moduleShape,
  );
}

// ============================================================================
// layout — ModuleGrid → PaintScene
// ============================================================================

/// Convert a [ModuleGrid] into a [PaintScene] ready for a paint backend.
///
/// This is the **only** function in the entire 2D barcode stack that knows
/// about pixels. Everything above this step works in abstract module units.
/// Everything below is handled by the paint backend (SVG, Canvas, Metal, …).
///
/// ### Square modules (the common case)
///
/// Each dark module at `(row, col)` becomes one `PaintRect`:
///
/// ```
/// quietZonePx = quietZoneModules × moduleSizePx
/// x = quietZonePx + col × moduleSizePx
/// y = quietZonePx + row × moduleSizePx
/// ```
///
/// Total canvas size (including quiet zone on all four sides):
///
/// ```
/// totalWidth  = (cols + 2 × quietZoneModules) × moduleSizePx
/// totalHeight = (rows + 2 × quietZoneModules) × moduleSizePx
/// ```
///
/// The scene always starts with one background `PaintRect` covering the full
/// canvas, so the quiet zone and light modules are always filled even when the
/// backend defaults to transparent.
///
/// ### Hex modules (MaxiCode)
///
/// Each dark module at `(row, col)` becomes one `PaintPath` tracing a flat-top
/// regular hexagon. Odd-numbered rows are offset right by half a hexagon width
/// to produce the standard hexagonal tiling.
///
/// Geometry (`s = moduleSizePx`):
///
/// ```
/// hexWidth  = s                    (flat-to-flat = side length)
/// hexHeight = s × (√3 / 2)        (vertical distance between row centres)
/// circumR   = s / √3              (centre-to-vertex = circumscribed radius)
///
/// Centre of module (row, col):
///   cx = quietZonePx + col × hexWidth + (row % 2) × (hexWidth / 2)
///   cy = quietZonePx + row × hexHeight
///
/// Vertex i (i = 0..5):
///   angle = i × 60°
///   vx = cx + circumR × cos(angle)
///   vy = cy + circumR × sin(angle)
/// ```
///
/// ### Validation
///
/// Throws [InvalidBarcode2DConfigError] if:
/// - `moduleSizePx <= 0`
/// - `quietZoneModules < 0`
/// - `config.moduleShape != grid.moduleShape`
///
/// @param grid    The module grid to render.
/// @param config  Optional layout config; unset fields use defaults from
///                [defaultBarcode2DLayoutConfig].
PaintScene layout(
  ModuleGrid grid, {
  Barcode2DLayoutConfig? config,
}) {
  // Merge supplied config with defaults.
  final cfg = config ?? defaultBarcode2DLayoutConfig;

  // ── Validation ────────────────────────────────────────────────────────────

  if (cfg.moduleSizePx <= 0) {
    throw InvalidBarcode2DConfigError(
      'moduleSizePx must be > 0, got ${cfg.moduleSizePx}',
    );
  }
  if (cfg.quietZoneModules < 0) {
    throw InvalidBarcode2DConfigError(
      'quietZoneModules must be >= 0, got ${cfg.quietZoneModules}',
    );
  }
  if (cfg.moduleShape != grid.moduleShape) {
    throw InvalidBarcode2DConfigError(
      'config.moduleShape "${cfg.moduleShape.name}" does not match '
      'grid.moduleShape "${grid.moduleShape.name}"',
    );
  }

  // Dispatch to the correct rendering path.
  return switch (cfg.moduleShape) {
    ModuleShape.square => _layoutSquare(grid, cfg),
    ModuleShape.hex => _layoutHex(grid, cfg),
  };
}

// ============================================================================
// _layoutSquare — internal helper
// ============================================================================

/// Render a square-module [ModuleGrid] into a [PaintScene].
///
/// Called only by [layout] after validation. Not exported — callers should
/// always go through [layout] to ensure validation runs first.
///
/// Algorithm:
///   1. Compute total canvas size including the quiet zone on all four sides.
///   2. Emit one background [PaintRect] covering the entire canvas.
///   3. Scan every module: emit one foreground [PaintRect] for each dark module.
///
/// Light modules are covered implicitly by the background rect. This keeps
/// instruction count proportional to dark modules, not total grid area.
PaintScene _layoutSquare(ModuleGrid grid, Barcode2DLayoutConfig cfg) {
  final quietZonePx = cfg.quietZoneModules * cfg.moduleSizePx;

  // Total canvas dimensions: grid + quiet zone on all four sides.
  final totalWidth = (grid.cols + 2 * cfg.quietZoneModules) * cfg.moduleSizePx;
  final totalHeight = (grid.rows + 2 * cfg.quietZoneModules) * cfg.moduleSizePx;

  final instructions = <PaintInstruction>[];

  // ── Step 1: Background ───────────────────────────────────────────────────
  //
  // A single rect fills the entire canvas with the background colour.
  // This handles the quiet zone and every light module in one shot.
  instructions.add(
    paintRect(
      x: 0,
      y: 0,
      width: totalWidth,
      height: totalHeight,
      fill: cfg.background,
    ),
  );

  // ── Step 2: Dark modules ─────────────────────────────────────────────────
  //
  // Scan row by row, left to right. Emit one PaintRect per dark module.
  for (var row = 0; row < grid.rows; row++) {
    for (var col = 0; col < grid.cols; col++) {
      if (grid.modules[row][col]) {
        // The pixel origin of this module's top-left corner.
        final x = quietZonePx + col * cfg.moduleSizePx;
        final y = quietZonePx + row * cfg.moduleSizePx;

        instructions.add(
          paintRect(
            x: x,
            y: y,
            width: cfg.moduleSizePx,
            height: cfg.moduleSizePx,
            fill: cfg.foreground,
          ),
        );
      }
    }
  }

  return createScene(
    width: totalWidth,
    height: totalHeight,
    background: cfg.background,
    instructions: instructions,
  );
}

// ============================================================================
// _layoutHex — internal helper
// ============================================================================

/// Render a hex-module [ModuleGrid] into a [PaintScene].
///
/// Used for MaxiCode (ISO/IEC 16023), which tiles flat-top hexagons in an
/// offset-row grid.
///
/// ### Flat-top hexagon geometry
///
/// A "flat-top" hexagon has flat edges at the top and bottom:
///
/// ```
///    ___
///   /   \     ← two vertices at the top
///  |     |
///   \___/     ← two vertices at the bottom
/// ```
///
/// For a flat-top hexagon with circumscribed radius R (centre-to-vertex):
///
/// ```
///   Vertex i (i = 0..5):
///     angle = i × 60° (measured from positive X axis)
///     vx = cx + R × cos(angle)
///     vy = cy + R × sin(angle)
///
///   angle  role
///     0°   right midpoint
///    60°   bottom-right
///   120°   bottom-left
///   180°   left midpoint
///   240°   top-left
///   300°   top-right
/// ```
///
/// ### Tiling
///
/// `hexWidth = moduleSizePx` (flat-to-flat distance = side length for a
/// regular hex). `hexHeight = moduleSizePx × (√3/2)` is the vertical step
/// between row centres. Odd rows shift right by `hexWidth/2` to interlock:
///
/// ```
/// Row 0:  ⬡ ⬡ ⬡    (cx starts at quietZonePx)
/// Row 1:   ⬡ ⬡ ⬡   (cx starts at quietZonePx + hexWidth/2)
/// Row 2:  ⬡ ⬡ ⬡    (cx starts at quietZonePx)
/// ```
PaintScene _layoutHex(ModuleGrid grid, Barcode2DLayoutConfig cfg) {
  final s = cfg.moduleSizePx.toDouble();

  // Hex geometry constants:
  //   hexWidth  = s       (flat-to-flat = side length for a regular hexagon)
  //   hexHeight = s×(√3/2) (vertical step between row centres)
  //   circumR   = s/√3    (circumscribed radius = centre-to-vertex)
  final hexWidth = s;
  final hexHeight = s * (math.sqrt(3) / 2);
  final circumR = s / math.sqrt(3);

  final quietZonePx = cfg.quietZoneModules * s;

  // Canvas size:
  //   Width:  cols × hexWidth + 2 × quietZonePx
  //           + hexWidth/2 for the odd-row offset (so modules don't clip)
  //   Height: rows × hexHeight + 2 × quietZonePx
  final totalWidth =
      ((grid.cols + 2 * cfg.quietZoneModules) * hexWidth + hexWidth / 2).round();
  final totalHeight =
      ((grid.rows + 2 * cfg.quietZoneModules) * hexHeight).round();

  final instructions = <PaintInstruction>[];

  // Background rect covering the entire canvas.
  instructions.add(
    paintRect(
      x: 0,
      y: 0,
      width: totalWidth,
      height: totalHeight,
      fill: cfg.background,
    ),
  );

  // One PaintPath per dark module.
  for (var row = 0; row < grid.rows; row++) {
    for (var col = 0; col < grid.cols; col++) {
      if (grid.modules[row][col]) {
        // Pixel centre of this hexagon.
        // Odd rows shift right by hexWidth/2 to produce the interlocked tiling.
        final cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2);
        final cy = quietZonePx + row * hexHeight;

        instructions.add(
          paintPath(
            commands: _buildFlatTopHexPath(cx, cy, circumR),
            fill: cfg.foreground,
          ),
        );
      }
    }
  }

  return createScene(
    width: totalWidth,
    height: totalHeight,
    background: cfg.background,
    instructions: instructions,
  );
}

// ============================================================================
// _buildFlatTopHexPath — geometry helper
// ============================================================================

/// Build the six [PathCommand]s that trace a flat-top regular hexagon.
///
/// The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
/// from the centre `(cx, cy)` at circumradius `R`:
///
/// ```
/// vertex i = ( cx + R × cos(i × 60°),
///              cy + R × sin(i × 60°) )
/// ```
///
/// The path is:
///   - `move_to` vertex 0
///   - `line_to` vertices 1–5
///   - `close`
///
/// @param cx       Centre x in pixels.
/// @param cy       Centre y in pixels.
/// @param circumR  Circumscribed radius (centre to vertex) in pixels.
List<PathCommand> _buildFlatTopHexPath(double cx, double cy, double circumR) {
  final commands = <PathCommand>[];
  const degToRad = math.pi / 180.0;

  // Vertex 0: move_to (start of sub-path, no line drawn).
  final angle0 = 0.0 * 60.0 * degToRad;
  commands.add(PathCommand.moveTo(
    cx + circumR * math.cos(angle0),
    cy + circumR * math.sin(angle0),
  ));

  // Vertices 1–5: line_to (extend the outline).
  for (var i = 1; i <= 5; i++) {
    final angle = i * 60.0 * degToRad;
    commands.add(PathCommand.lineTo(
      cx + circumR * math.cos(angle),
      cy + circumR * math.sin(angle),
    ));
  }

  // Close: draw back to vertex 0 and fill the enclosed area.
  commands.add(PathCommand.close());

  return commands;
}
