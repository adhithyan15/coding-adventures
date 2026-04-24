/**
 * @coding-adventures/barcode-2d
 *
 * Shared 2D barcode abstraction layer.
 *
 * This package provides the two building blocks every 2D barcode format needs:
 *
 *   1. `ModuleGrid` — the universal intermediate representation produced by
 *      every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
 *      It is just a 2D boolean grid: true = dark module, false = light module.
 *
 *   2. `layout()` — the single function that converts abstract module
 *      coordinates into pixel-level `PaintScene` instructions ready for the
 *      PaintVM (P2D01) to render.
 *
 * ## Where this fits in the pipeline
 *
 * ```
 * Input data
 *   → format encoder (qr-code, data-matrix, aztec…)
 *   → ModuleGrid          ← produced by the encoder
 *   → layout()            ← THIS PACKAGE converts to pixels
 *   → PaintScene          ← consumed by paint-vm (P2D01)
 *   → backend (SVG, Metal, Canvas, terminal…)
 * ```
 *
 * All coordinates before `layout()` are measured in "module units" — abstract
 * grid steps. Only `layout()` multiplies by `moduleSizePx` to produce real
 * pixel coordinates. This means encoders never need to know anything about
 * screen resolution or output format.
 *
 * ## Supported module shapes
 *
 * - **Square** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
 *   Each module becomes a `PaintRect`.
 *
 * - **Hex** (flat-top hexagons): used by MaxiCode. Each module becomes a
 *   `PaintPath` tracing six vertices.
 *
 * ## Annotations
 *
 * The optional `AnnotatedModuleGrid` adds per-module role information useful
 * for visualizers (highlighting finder patterns, data codewords, etc.).
 * Annotations are never required for rendering — the renderer only looks at
 * the `modules` boolean grid.
 */
export const VERSION = "0.1.0";

import {
  type PaintScene,
  type PaintInstruction,
  type PathCommand,
  paintScene,
  paintRect,
  paintPath,
} from "@coding-adventures/paint-instructions";

// ============================================================================
// ModuleShape — square vs. hex
// ============================================================================

/**
 * The shape of each module in the grid.
 *
 * - `"square"` — used by QR Code, Data Matrix, Aztec Code, PDF417. The
 *   overwhelmingly common shape. Each module renders as a filled square.
 *
 * - `"hex"` — used by MaxiCode (ISO/IEC 16023). MaxiCode uses flat-top
 *   hexagons arranged in an offset-row grid. Each module renders as a filled
 *   hexagon drawn with a `PaintPath`.
 *
 * The shape is stored on `ModuleGrid` so that `layout()` can pick the right
 * rendering path without the caller having to specify it again.
 */
export type ModuleShape = "square" | "hex";

// ============================================================================
// ModuleGrid — the universal output of every 2D barcode encoder
// ============================================================================

/**
 * The universal intermediate representation produced by every 2D barcode
 * encoder. It is a 2D boolean grid:
 *
 * ```
 * modules[row][col] === true   →  dark module (ink / filled)
 * modules[row][col] === false  →  light module (background / empty)
 * ```
 *
 * Row 0 is the top row. Column 0 is the leftmost column. This matches the
 * natural reading order used in every 2D barcode standard.
 *
 * ### MaxiCode fixed size
 *
 * MaxiCode grids are always 33 rows × 30 columns with `moduleShape: "hex"`.
 * The outer bullseye rings are placed at the geometric center.
 * Physical MaxiCode symbols are always approximately 1 inch × 1 inch.
 *
 * ### Immutability
 *
 * `ModuleGrid` is intentionally read-only. Use `setModule()` to produce a new
 * grid with one module changed, rather than mutating in place. This makes
 * encoders easy to test and compose.
 */
export interface ModuleGrid {
  readonly cols: number;
  readonly rows: number;
  /**
   * Two-dimensional boolean grid. Access with `modules[row][col]`.
   * `true` = dark module, `false` = light module.
   */
  readonly modules: ReadonlyArray<ReadonlyArray<boolean>>;
  readonly moduleShape: ModuleShape;
}

// ============================================================================
// ModuleRole — what a module structurally represents
// ============================================================================

/**
 * The structural role of a module within its barcode symbol.
 *
 * These roles are generic — they apply across all 2D barcode formats. Each
 * role corresponds to a distinct part of the symbol's anatomy:
 *
 * - `"finder"` — a locator pattern that helps scanners detect and orient the
 *   symbol. QR Code uses three 7×7 corner finder patterns. Data Matrix uses
 *   the L-shaped solid border (finder bar). Aztec Code uses the central
 *   bullseye rings.
 *
 * - `"separator"` — a quiet-zone strip between a finder pattern and the data
 *   area. Always light (false) in correctly encoded symbols.
 *
 * - `"timing"` — an alternating dark/light calibration strip. Enables the
 *   scanner to measure module size and compensate for perspective distortion.
 *   QR Code has two timing strips (horizontal and vertical).
 *
 * - `"alignment"` — secondary locator patterns placed in high-version QR Code
 *   symbols to correct for lens distortion over large symbols. Not used by
 *   all formats.
 *
 * - `"format"` — encodes ECC level + mask indicator (QR), layer count +
 *   error mode (Aztec), or other symbol-level metadata. Scanned before the
 *   data region so the decoder knows how to interpret the rest.
 *
 * - `"data"` — one bit of an encoded codeword. The message (after encoding)
 *   lives in these modules.
 *
 * - `"ecc"` — one bit of an error correction codeword (Reed-Solomon or
 *   other ECC). These modules let scanners recover the message even if part
 *   of the symbol is damaged.
 *
 * - `"padding"` — remainder/filler bits used to fill the grid when the
 *   message is shorter than the symbol's capacity. Always a fixed alternating
 *   pattern (0xEC / 0x11 for QR Code).
 *
 * ### Format-specific roles
 *
 * Roles that only exist in one format (e.g. QR Code's "dark module",
 * Aztec's "mode message", PDF417's row indicators) are stored in
 * `ModuleAnnotation.metadata.format_role` as strings like `"qr:dark-module"`.
 * This avoids polluting the generic enum with format-specific noise.
 */
export type ModuleRole =
  | "finder"
  | "separator"
  | "timing"
  | "alignment"
  | "format"
  | "data"
  | "ecc"
  | "padding";

// ============================================================================
// ModuleAnnotation — per-module role metadata for visualizers
// ============================================================================

/**
 * Per-module role annotation, used by visualizers to colour-code the symbol.
 *
 * Annotations are entirely optional. The renderer (`layout()`) only reads
 * `ModuleGrid.modules`; it never looks at annotations unless
 * `showAnnotations: true` is set in the layout config.
 *
 * ### codewordIndex and bitIndex
 *
 * For `"data"` and `"ecc"` modules, these identify exactly which bit in which
 * codeword this module encodes. This is useful for visualizers that highlight
 * one codeword at a time to show how data is placed across the grid.
 *
 * - `codewordIndex` — zero-based index into the final interleaved codeword
 *   stream.
 * - `bitIndex` — zero-based bit index within that codeword, 0 = MSB.
 *
 * For structural modules (`"finder"`, `"timing"`, etc.) these are undefined.
 *
 * ### metadata
 *
 * An escape hatch for format-specific annotations. For example:
 *
 * - QR Code dark module: `{ format_role: "qr:dark-module" }`
 * - QR Code masked: `{ format_role: "qr:masked", mask_pattern: "3" }`
 * - Aztec mode message: `{ format_role: "aztec:mode-message" }`
 * - PDF417 row indicator: `{ format_role: "pdf417:row-indicator", row: "4" }`
 */
export interface ModuleAnnotation {
  readonly role: ModuleRole;
  readonly dark: boolean;
  readonly codewordIndex?: number;
  readonly bitIndex?: number;
  /**
   * Arbitrary format-specific key/value pairs. The key `format_role` holds a
   * namespaced role string such as `"qr:dark-module"` or `"aztec:mode-message"`.
   */
  readonly metadata?: Record<string, string>;
}

// ============================================================================
// AnnotatedModuleGrid — ModuleGrid with per-module role annotations
// ============================================================================

/**
 * A `ModuleGrid` extended with per-module role annotations.
 *
 * Used by visualizers to render colour-coded diagrams that teach how the
 * barcode is structured. The `annotations` array mirrors `modules` exactly
 * in size: `annotations[row][col]` corresponds to `modules[row][col]`.
 *
 * A `null` annotation means "no annotation for this module" — this can happen
 * when an encoder only annotates some modules (e.g. only the data region)
 * and leaves structural modules un-annotated.
 *
 * This type is NOT required for rendering. `layout()` accepts a plain
 * `ModuleGrid` and works identically whether or not annotations are present.
 */
export interface AnnotatedModuleGrid extends ModuleGrid {
  readonly annotations: ReadonlyArray<ReadonlyArray<ModuleAnnotation | null>>;
}

// ============================================================================
// Barcode2DLayoutConfig — pixel-level rendering options
// ============================================================================

/**
 * Configuration for `layout()`.
 *
 * All fields are optional — pass a `Partial<Barcode2DLayoutConfig>` and the
 * defaults from `DEFAULT_BARCODE_2D_LAYOUT_CONFIG` fill in any gaps.
 *
 * ### moduleSizePx
 *
 * The size of one module in pixels. For square modules this is both width and
 * height. For hex modules it is the hexagon's width (flat-to-flat, also equal
 * to the side length for a regular hexagon).
 *
 * Must be > 0.
 *
 * ### quietZoneModules
 *
 * The number of module-width quiet-zone units added on each side of the grid.
 * QR Code requires a minimum of 4 modules. Data Matrix requires 1. MaxiCode
 * requires 1.
 *
 * Must be ≥ 0.
 *
 * ### moduleShape
 *
 * Must match `ModuleGrid.moduleShape`. If they disagree, `layout()` throws
 * `InvalidBarcode2DConfigError`. This double-check prevents accidentally
 * rendering a MaxiCode hex grid with square modules.
 */
export interface Barcode2DLayoutConfig {
  readonly moduleSizePx: number;
  readonly quietZoneModules: number;
  readonly foreground: string;
  readonly background: string;
  readonly showAnnotations: boolean;
  readonly moduleShape: ModuleShape;
}

/**
 * Sensible defaults for `layout()`.
 *
 * | Field              | Default     | Why                                   |
 * |--------------------|-------------|---------------------------------------|
 * | moduleSizePx       | 10          | Produces a readable QR at 210×210 px  |
 * | quietZoneModules   | 4           | QR Code minimum per ISO/IEC 18004     |
 * | foreground         | "#000000"   | Black ink on white paper              |
 * | background         | "#ffffff"   | White paper                           |
 * | showAnnotations    | false       | Off by default; opt-in for visualizers|
 * | moduleShape        | "square"    | The overwhelmingly common case        |
 */
export const DEFAULT_BARCODE_2D_LAYOUT_CONFIG: Barcode2DLayoutConfig = {
  moduleSizePx: 10,
  quietZoneModules: 4,
  foreground: "#000000",
  background: "#ffffff",
  showAnnotations: false,
  moduleShape: "square",
};

// ============================================================================
// Error types
// ============================================================================

/**
 * Base class for all barcode-2d errors.
 */
export class Barcode2DError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "Barcode2DError";
  }
}

/**
 * Thrown by `layout()` when the configuration is invalid — for example:
 * - `moduleSizePx <= 0`
 * - `quietZoneModules < 0`
 * - `config.moduleShape` does not match `grid.moduleShape`
 */
export class InvalidBarcode2DConfigError extends Barcode2DError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidBarcode2DConfigError";
  }
}

// ============================================================================
// makeModuleGrid — create an all-light grid
// ============================================================================

/**
 * Create a new `ModuleGrid` of the given dimensions, with every module set
 * to `false` (light).
 *
 * This is the starting point for every 2D barcode encoder. The encoder calls
 * `makeModuleGrid(rows, cols)` and then uses `setModule()` to paint dark
 * modules one by one as it places finder patterns, timing strips, data bits,
 * and error correction bits.
 *
 * ### Example — start a 21×21 QR Code v1 grid
 *
 * ```typescript
 * let grid = makeModuleGrid(21, 21);
 * // grid.modules[0][0] === false  (all light)
 * // grid.rows === 21
 * // grid.cols === 21
 * ```
 *
 * @param rows     Number of rows (height of the grid).
 * @param cols     Number of columns (width of the grid).
 * @param moduleShape  Shape of each module. Defaults to `"square"`.
 */
export function makeModuleGrid(
  rows: number,
  cols: number,
  moduleShape: ModuleShape = "square",
): ModuleGrid {
  // Build a 2D array of `false` values. Each row is an independent array so
  // that setModule() can replace individual rows without copying the entire
  // grid.
  const modules: ReadonlyArray<boolean>[] = [];
  for (let r = 0; r < rows; r++) {
    modules.push(new Array<boolean>(cols).fill(false));
  }
  return { rows, cols, modules, moduleShape };
}

// ============================================================================
// setModule — immutable single-module update
// ============================================================================

/**
 * Return a new `ModuleGrid` identical to `grid` except that module at
 * `(row, col)` is set to `dark`.
 *
 * This function is **pure and immutable** — it never modifies the input grid.
 * The original grid remains valid and unchanged. Only the affected row is
 * re-allocated; all other rows are shared between old and new grids.
 *
 * ### Why immutability matters
 *
 * Barcode encoders often need to backtrack (e.g. trying different QR mask
 * patterns). Immutable grids make this trivial — save the grid before trying
 * a mask, evaluate it, discard if the score is worse, keep the old one if it
 * is better. No undo stack needed.
 *
 * ### Out-of-bounds
 *
 * Throws `RangeError` if `row` or `col` is outside the grid dimensions. This
 * is a programming error in the encoder, not a user-facing error.
 *
 * ### Example
 *
 * ```typescript
 * let g = makeModuleGrid(3, 3);
 * let g2 = setModule(g, 1, 1, true);
 * // g.modules[1][1] === false  (original unchanged)
 * // g2.modules[1][1] === true
 * // g !== g2                   (new object)
 * ```
 */
export function setModule(
  grid: ModuleGrid,
  row: number,
  col: number,
  dark: boolean,
): ModuleGrid {
  if (row < 0 || row >= grid.rows) {
    throw new RangeError(
      `setModule: row ${row} out of range [0, ${grid.rows - 1}]`,
    );
  }
  if (col < 0 || col >= grid.cols) {
    throw new RangeError(
      `setModule: col ${col} out of range [0, ${grid.cols - 1}]`,
    );
  }

  // Copy only the affected row; all other rows are shared (shallow copy).
  const newRow = [...grid.modules[row]] as boolean[];
  newRow[col] = dark;

  const newModules = grid.modules.map((r, i) =>
    i === row ? newRow : r,
  ) as ReadonlyArray<ReadonlyArray<boolean>>;

  return { ...grid, modules: newModules };
}

// ============================================================================
// layout — ModuleGrid → PaintScene
// ============================================================================

/**
 * Convert a `ModuleGrid` into a `PaintScene` ready for the PaintVM.
 *
 * This is the **only** function in the entire 2D barcode stack that knows
 * about pixels. Everything above this step works in abstract module units.
 * Everything below this step is handled by the paint backend.
 *
 * ### Square modules (the common case)
 *
 * Each dark module at `(row, col)` becomes one `PaintRect`:
 *
 * ```
 * quietZonePx = quietZoneModules * moduleSizePx
 * x = quietZonePx + col * moduleSizePx
 * y = quietZonePx + row * moduleSizePx
 * ```
 *
 * Total symbol size (including quiet zone on all four sides):
 *
 * ```
 * totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
 * totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
 * ```
 *
 * The scene always starts with one background `PaintRect` covering the full
 * symbol. This ensures the quiet zone and light modules are filled with the
 * background color even if the backend has a transparent default.
 *
 * ### Hex modules (MaxiCode)
 *
 * Each dark module at `(row, col)` becomes one `PaintPath` tracing a
 * flat-top regular hexagon. Odd-numbered rows are offset by half a hexagon
 * width to produce the standard hexagonal tiling:
 *
 * ```
 * Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
 * Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
 * Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
 * ```
 *
 * Geometry for a flat-top hexagon centered at `(cx, cy)` with circumradius R
 * (center to vertex distance):
 *
 * ```
 * hexWidth  = moduleSizePx                    (flat-to-flat = side length)
 * hexHeight = moduleSizePx * (√3 / 2)        (point-to-point across short axis)
 * circumR   = moduleSizePx / √3              (center to vertex)
 *
 * Vertex angles: 0°, 60°, 120°, 180°, 240°, 300°  (measured from positive X)
 * Vertex i:  ( cx + circumR * cos(i * 60°),
 *              cy + circumR * sin(i * 60°) )
 * ```
 *
 * Center coordinates including quiet zone offset:
 *
 * ```
 * cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2)
 * cy = quietZonePx + row * hexHeight
 * ```
 *
 * ### Validation
 *
 * Throws `InvalidBarcode2DConfigError` if:
 * - `moduleSizePx <= 0`
 * - `quietZoneModules < 0`
 * - `config.moduleShape !== grid.moduleShape`
 *
 * @param grid   The module grid to render.
 * @param config Partial layout config; unset fields use defaults.
 * @returns      A `PaintScene` ready for the PaintVM.
 */
export function layout(
  grid: ModuleGrid,
  config?: Partial<Barcode2DLayoutConfig>,
): PaintScene {
  // Merge partial config with defaults.
  const cfg: Barcode2DLayoutConfig = {
    ...DEFAULT_BARCODE_2D_LAYOUT_CONFIG,
    ...config,
  };

  // ── Validation ──────────────────────────────────────────────────────────
  if (cfg.moduleSizePx <= 0) {
    throw new InvalidBarcode2DConfigError(
      `moduleSizePx must be > 0, got ${cfg.moduleSizePx}`,
    );
  }
  if (cfg.quietZoneModules < 0) {
    throw new InvalidBarcode2DConfigError(
      `quietZoneModules must be >= 0, got ${cfg.quietZoneModules}`,
    );
  }
  if (cfg.moduleShape !== grid.moduleShape) {
    throw new InvalidBarcode2DConfigError(
      `config.moduleShape "${cfg.moduleShape}" does not match grid.moduleShape "${grid.moduleShape}"`,
    );
  }

  // Dispatch to the correct rendering path.
  if (cfg.moduleShape === "square") {
    return layoutSquare(grid, cfg);
  } else {
    return layoutHex(grid, cfg);
  }
}

// ============================================================================
// layoutSquare — internal helper for square-module grids
// ============================================================================

/**
 * Render a square-module `ModuleGrid` into a `PaintScene`.
 *
 * Called only by `layout()` after validation. Not exported because callers
 * should always go through `layout()` to ensure the config is validated.
 *
 * The algorithm is straightforward:
 *
 * 1. Compute total pixel dimensions including quiet zone.
 * 2. Emit one background `PaintRect` covering the entire symbol.
 * 3. For each dark module, emit one filled `PaintRect`.
 *
 * Light modules are implicitly covered by the background rect — no explicit
 * light rects are emitted. This keeps the instruction count proportional to
 * the number of dark modules rather than the total grid size.
 */
function layoutSquare(grid: ModuleGrid, cfg: Barcode2DLayoutConfig): PaintScene {
  const { moduleSizePx, quietZoneModules, foreground, background } = cfg;

  // Quiet zone in pixels on each side.
  const quietZonePx = quietZoneModules * moduleSizePx;

  // Total canvas dimensions including quiet zone on all four sides.
  const totalWidth = (grid.cols + 2 * quietZoneModules) * moduleSizePx;
  const totalHeight = (grid.rows + 2 * quietZoneModules) * moduleSizePx;

  const instructions: PaintInstruction[] = [];

  // 1. Background: a single rect covering the entire symbol including quiet
  //    zone. This ensures light modules and the quiet zone are always filled,
  //    even when the backend default is transparent.
  instructions.push(paintRect(0, 0, totalWidth, totalHeight, { fill: background }));

  // 2. One PaintRect per dark module.
  for (let row = 0; row < grid.rows; row++) {
    for (let col = 0; col < grid.cols; col++) {
      if (grid.modules[row][col]) {
        // Pixel origin of this module (top-left corner of its square).
        const x = quietZonePx + col * moduleSizePx;
        const y = quietZonePx + row * moduleSizePx;

        instructions.push(
          paintRect(x, y, moduleSizePx, moduleSizePx, { fill: foreground }),
        );
      }
    }
  }

  return paintScene(totalWidth, totalHeight, background, instructions);
}

// ============================================================================
// layoutHex — internal helper for hex-module grids (MaxiCode)
// ============================================================================

/**
 * Render a hex-module `ModuleGrid` into a `PaintScene`.
 *
 * Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
 * offset-row grid. Odd rows are shifted right by half a hexagon width.
 *
 * ### Flat-top hexagon geometry reminder
 *
 * A "flat-top" hexagon has two flat edges at the top and bottom:
 *
 * ```
 *    ___
 *   /   \      ← two vertices at top
 *  |     |
 *   \___/      ← two vertices at bottom
 * ```
 *
 * Contrast with "pointy-top" which has a vertex at the top. MaxiCode and
 * most industrial standards use flat-top.
 *
 * For a flat-top hexagon centered at `(cx, cy)` with circumradius `R`:
 *
 * ```
 * Vertices at angles 0°, 60°, 120°, 180°, 240°, 300°:
 *
 *   angle  cos    sin    role
 *     0°    1      0     right midpoint
 *    60°   0.5   √3/2   bottom-right
 *   120°  -0.5   √3/2   bottom-left
 *   180°  -1      0     left midpoint
 *   240°  -0.5  -√3/2   top-left
 *   300°   0.5  -√3/2   top-right
 * ```
 *
 * ### Tiling
 *
 * Hex grids tile by setting:
 *   hexWidth  = moduleSizePx
 *   hexHeight = moduleSizePx * (√3 / 2)   ← vertical distance between row centers
 *
 * Odd rows are offset by hexWidth / 2 to interlock with even rows:
 *
 * ```
 * Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
 * Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
 * Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no offset)
 * ```
 */
function layoutHex(grid: ModuleGrid, cfg: Barcode2DLayoutConfig): PaintScene {
  const { moduleSizePx, quietZoneModules, foreground, background } = cfg;

  // Hex geometry:
  //   hexWidth  = one module width (flat-to-flat = side length for regular hex)
  //   hexHeight = vertical distance between row centers
  //   circumR   = center-to-vertex distance (circumscribed circle radius)
  //
  // For a regular hexagon where side length = s:
  //   flat-to-flat distance = s  →  hexWidth = moduleSizePx
  //   point-to-point (short axis) = s * √3  →  but we want hexHeight = row step
  //   row step = s * (√3 / 2) = hexWidth * (√3 / 2)
  //   circumR = s / √3 = hexWidth / √3
  const hexWidth = moduleSizePx;
  const hexHeight = moduleSizePx * (Math.sqrt(3) / 2);
  const circumR = moduleSizePx / Math.sqrt(3);

  const quietZonePx = quietZoneModules * moduleSizePx;

  // Total canvas size. The +hexWidth/2 accounts for the odd-row offset so
  // the rightmost modules on odd rows don't clip outside the canvas.
  const totalWidth =
    (grid.cols + 2 * quietZoneModules) * hexWidth + hexWidth / 2;
  const totalHeight = (grid.rows + 2 * quietZoneModules) * hexHeight;

  const instructions: PaintInstruction[] = [];

  // Background rect.
  instructions.push(paintRect(0, 0, totalWidth, totalHeight, { fill: background }));

  // One PaintPath per dark module.
  for (let row = 0; row < grid.rows; row++) {
    for (let col = 0; col < grid.cols; col++) {
      if (grid.modules[row][col]) {
        // Center of this hexagon in pixel space.
        // Odd rows shift right by hexWidth/2.
        const cx =
          quietZonePx +
          col * hexWidth +
          (row % 2) * (hexWidth / 2);
        const cy = quietZonePx + row * hexHeight;

        instructions.push(
          paintPath(buildFlatTopHexPath(cx, cy, circumR), { fill: foreground }),
        );
      }
    }
  }

  return paintScene(totalWidth, totalHeight, background, instructions);
}

// ============================================================================
// buildFlatTopHexPath — geometry helper
// ============================================================================

/**
 * Build the six `PathCommand`s for a flat-top regular hexagon.
 *
 * The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
 * from the center `(cx, cy)` at circumradius `R`:
 *
 * ```
 * vertex_i = ( cx + R * cos(i * 60°),
 *              cy + R * sin(i * 60°) )
 * ```
 *
 * The path starts with a `move_to` to vertex 0, then five `line_to` commands
 * to vertices 1–5, then a `close` to return to vertex 0.
 *
 * @param cx       Center x in pixels.
 * @param cy       Center y in pixels.
 * @param circumR  Circumscribed circle radius (center to vertex) in pixels.
 */
function buildFlatTopHexPath(
  cx: number,
  cy: number,
  circumR: number,
): PathCommand[] {
  const commands: PathCommand[] = [];
  const DEG_TO_RAD = Math.PI / 180;

  // First vertex: move_to
  const angle0 = 0 * 60 * DEG_TO_RAD;
  commands.push({
    kind: "move_to",
    x: cx + circumR * Math.cos(angle0),
    y: cy + circumR * Math.sin(angle0),
  });

  // Remaining 5 vertices: line_to
  for (let i = 1; i <= 5; i++) {
    const angle = i * 60 * DEG_TO_RAD;
    commands.push({
      kind: "line_to",
      x: cx + circumR * Math.cos(angle),
      y: cy + circumR * Math.sin(angle),
    });
  }

  // Close back to vertex 0.
  commands.push({ kind: "close" });

  return commands;
}

// Re-export PaintScene so callers can type the return value of layout()
// without needing to import paint-instructions themselves.
export type { PaintScene };
