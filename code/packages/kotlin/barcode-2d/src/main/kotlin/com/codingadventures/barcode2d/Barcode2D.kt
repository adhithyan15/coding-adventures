/**
 * barcode-2d — shared 2D barcode abstraction layer.
 *
 * This package provides the two building blocks every 2D barcode format needs:
 *
 *   1. [ModuleGrid] — the universal intermediate representation produced by
 *      every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
 *      It is just a 2D boolean grid: `true` = dark module, `false` = light.
 *
 *   2. [layout] — the single function that converts abstract module coordinates
 *      into pixel-level [PaintScene] instructions ready for the PaintVM.
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
 * All coordinates before [layout] are measured in "module units" — abstract
 * grid steps. Only [layout] multiplies by [Barcode2DLayoutConfig.moduleSizePx]
 * to produce real pixel coordinates. This means encoders never need to know
 * anything about screen resolution or output format.
 *
 * ## Supported module shapes
 *
 * - **Square** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
 *   Each module becomes a [com.codingadventures.paintinstructions.PaintInstruction.PaintRect].
 *
 * - **Hex** (flat-top hexagons): used by MaxiCode. Each module becomes a
 *   [com.codingadventures.paintinstructions.PaintInstruction.PaintPath] tracing
 *   six vertices.
 *
 * ## Relationship to the TypeScript reference
 *
 * This package mirrors `code/packages/typescript/barcode-2d/src/index.ts`.
 * TypeScript uses interfaces and discriminated union types; Kotlin uses sealed
 * classes, data classes, and enum classes.  The public API names and semantics
 * are kept as consistent as the two type systems allow.
 *
 * Spec: DT2D01 barcode-2d spec.
 */
package com.codingadventures.barcode2d

import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PaintScene
import com.codingadventures.paintinstructions.PathCommand
import com.codingadventures.paintinstructions.createScene
import com.codingadventures.paintinstructions.paintPath
import com.codingadventures.paintinstructions.paintRect
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt

/** Package version.  Follows Semantic Versioning 2.0. */
const val VERSION = "0.1.0"

// ============================================================================
// ModuleShape — square vs. hex
// ============================================================================

/**
 * The shape of each module in the grid.
 *
 * - [SQUARE] — used by QR Code, Data Matrix, Aztec Code, PDF417.  The
 *   overwhelmingly common shape.  Each module renders as a filled square.
 *
 * - [HEX] — used by MaxiCode (ISO/IEC 16023).  MaxiCode uses flat-top
 *   hexagons arranged in an offset-row grid.  Each module renders as a filled
 *   hexagon drawn with a [PaintInstruction.PaintPath].
 *
 * The shape is stored on [ModuleGrid] so that [layout] can pick the right
 * rendering path without the caller having to specify it again.
 */
enum class ModuleShape {
    /**
     * Square modules — QR Code, Data Matrix, Aztec, PDF417.
     * Each dark module → one [PaintInstruction.PaintRect].
     */
    SQUARE,

    /**
     * Flat-top hexagonal modules — MaxiCode (ISO/IEC 16023).
     * Each dark module → one [PaintInstruction.PaintPath] with 6 vertices.
     *
     * MaxiCode grids are always 33 rows × 30 columns.
     * Odd-numbered rows are shifted right by half a hexagon width to produce
     * the standard hexagonal tiling pattern:
     *
     * ```
     * Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no horizontal offset)
     * Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
     * Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no horizontal offset)
     * ```
     */
    HEX,
}

// ============================================================================
// ModuleGrid — the universal output of every 2D barcode encoder
// ============================================================================

/**
 * The universal intermediate representation produced by every 2D barcode
 * encoder.  It is a 2D boolean grid:
 *
 * ```
 * modules[row][col] == true   →  dark module (ink / filled)
 * modules[row][col] == false  →  light module (background / empty)
 * ```
 *
 * Row 0 is the top row.  Column 0 is the leftmost column.  This matches the
 * natural reading order used in every 2D barcode standard.
 *
 * ### MaxiCode fixed size
 *
 * MaxiCode grids are always 33 rows × 30 columns with [ModuleShape.HEX].
 * Physical MaxiCode symbols are always approximately 1 inch × 1 inch.
 *
 * ### Immutability
 *
 * [ModuleGrid] is intentionally immutable.  Use [setModule] to produce a new
 * grid with one module changed, rather than mutating in place.  This makes
 * encoders easy to test and compose without undo stacks or defensive copies.
 *
 * @param rows         Number of rows (height of the grid).
 * @param cols         Number of columns (width of the grid).
 * @param modules      Immutable 2D boolean grid.  Access with `modules[row][col]`.
 * @param moduleShape  Shape of each module.
 */
data class ModuleGrid(
    val rows: Int,
    val cols: Int,
    /**
     * Two-dimensional boolean grid.  Access with `modules[row][col]`.
     * `true` = dark module, `false` = light module.
     *
     * Each inner list is a complete row; the outer list contains all rows in
     * top-to-bottom order.
     */
    val modules: List<List<Boolean>>,
    val moduleShape: ModuleShape = ModuleShape.SQUARE,
)

// ============================================================================
// ModuleRole — what a module structurally represents (for annotated grids)
// ============================================================================

/**
 * The structural role of a module within its barcode symbol.
 *
 * These roles are generic — they apply across all 2D barcode formats.
 *
 * - [FINDER]    — locator pattern (QR corner squares, Data Matrix border,
 *                 Aztec bullseye rings).
 * - [SEPARATOR] — quiet-zone strip between finder and data area.
 * - [TIMING]    — alternating dark/light calibration strip.
 * - [ALIGNMENT] — secondary locator (high-version QR only).
 * - [FORMAT]    — encodes ECC level + mask / layer count.
 * - [DATA]      — one bit of an encoded codeword.
 * - [ECC]       — one bit of an error correction codeword.
 * - [PADDING]   — filler bits (e.g. 0xEC/0x11 in QR).
 */
enum class ModuleRole {
    FINDER, SEPARATOR, TIMING, ALIGNMENT, FORMAT, DATA, ECC, PADDING
}

// ============================================================================
// ModuleAnnotation — per-module role metadata for visualizers
// ============================================================================

/**
 * Per-module role annotation, used by visualizers to colour-code the symbol.
 *
 * Annotations are entirely optional.  The renderer ([layout]) only reads
 * [ModuleGrid.modules]; it never looks at annotations.
 *
 * @param role           Structural role of this module.
 * @param dark           Whether the module is dark (mirrors [ModuleGrid.modules]).
 * @param codewordIndex  Zero-based codeword index (data/ecc modules only).
 * @param bitIndex       Zero-based bit index within the codeword (0 = MSB).
 * @param metadata       Format-specific metadata.  Key `format_role` holds a
 *                       namespaced string like `"qr:dark-module"`.
 */
data class ModuleAnnotation(
    val role: ModuleRole,
    val dark: Boolean,
    val codewordIndex: Int? = null,
    val bitIndex: Int? = null,
    val metadata: Map<String, String> = emptyMap(),
)

// ============================================================================
// AnnotatedModuleGrid — ModuleGrid + per-module role annotations
// ============================================================================

/**
 * A [ModuleGrid] extended with per-module role annotations.
 *
 * Used by visualizers to render colour-coded diagrams.  The [annotations]
 * list mirrors [ModuleGrid.modules] exactly in size:
 * `annotations[row][col]` corresponds to `modules[row][col]`.
 *
 * A `null` annotation means "no annotation for this module".
 *
 * @param grid        The base module grid.
 * @param annotations 2D grid of nullable annotations, same size as [grid].
 */
data class AnnotatedModuleGrid(
    val grid: ModuleGrid,
    val annotations: List<List<ModuleAnnotation?>>,
)

// ============================================================================
// Barcode2DLayoutConfig — pixel-level rendering options
// ============================================================================

/**
 * Configuration for the [layout] function.
 *
 * All fields have sensible defaults; typically you only need to change
 * [moduleSizePx] or [foreground]/[background] for custom styling.
 *
 * ### moduleSizePx
 *
 * The size of one module in pixels.  For square modules this is both width and
 * height.  For hex modules it is the hexagon's flat-to-flat width.
 *
 * Must be > 0.
 *
 * ### quietZoneModules
 *
 * The number of module-width quiet-zone units added on each side of the grid.
 * QR Code requires a minimum of 4 modules.  Data Matrix requires 1.
 * MaxiCode requires 1.
 *
 * Must be ≥ 0.
 *
 * ### moduleShape
 *
 * Must match [ModuleGrid.moduleShape].  If they disagree, [layout] throws
 * [InvalidBarcode2DConfigException].  This double-check prevents accidentally
 * rendering a MaxiCode hex grid with square modules.
 *
 * @param moduleSizePx      Pixels per module side.  Default: 10.
 * @param quietZoneModules  Quiet-zone width in module units per side.  Default: 4.
 * @param foreground        CSS fill colour for dark modules.  Default: `"#000000"`.
 * @param background        CSS fill colour for light modules / quiet zone.  Default: `"#ffffff"`.
 * @param showAnnotations   Whether to colour-code by role (visualizers only).  Default: `false`.
 * @param moduleShape       Expected shape of the modules.  Default: [ModuleShape.SQUARE].
 */
data class Barcode2DLayoutConfig(
    val moduleSizePx: Int = 10,
    val quietZoneModules: Int = 4,
    val foreground: String = "#000000",
    val background: String = "#ffffff",
    val showAnnotations: Boolean = false,
    val moduleShape: ModuleShape = ModuleShape.SQUARE,
)

// ============================================================================
// Error types
// ============================================================================

/**
 * Base class for all barcode-2d errors.
 *
 * Using a dedicated exception hierarchy lets callers catch barcode-specific
 * errors with `catch (e: Barcode2DException)` without accidentally swallowing
 * general [RuntimeException]s from the JVM or other libraries.
 */
open class Barcode2DException(message: String) : Exception(message)

/**
 * Thrown by [layout] when the configuration is invalid.
 *
 * Specific causes:
 * - [Barcode2DLayoutConfig.moduleSizePx] ≤ 0
 * - [Barcode2DLayoutConfig.quietZoneModules] < 0
 * - [Barcode2DLayoutConfig.moduleShape] does not match [ModuleGrid.moduleShape]
 */
class InvalidBarcode2DConfigException(message: String) : Barcode2DException(message)

// ============================================================================
// makeModuleGrid — create an all-light grid
// ============================================================================

/**
 * Create a new [ModuleGrid] of the given dimensions, with every module set
 * to `false` (light).
 *
 * This is the starting point for every 2D barcode encoder.  The encoder calls
 * `makeModuleGrid(rows, cols)` and then uses [setModule] to paint dark modules
 * one by one as it places finder patterns, timing strips, data bits, and error
 * correction bits.
 *
 * ### Example — start a 21×21 QR Code v1 grid
 *
 * ```kotlin
 * var grid = makeModuleGrid(rows = 21, cols = 21)
 * // grid.modules[0][0] == false  (all light)
 * // grid.rows == 21, grid.cols == 21
 * ```
 *
 * @param rows        Number of rows (height of the grid in module units).
 * @param cols        Number of columns (width of the grid in module units).
 * @param moduleShape Shape of each module.  Defaults to [ModuleShape.SQUARE].
 * @return A new [ModuleGrid] with every module set to `false`.
 */
fun makeModuleGrid(
    rows: Int,
    cols: Int,
    moduleShape: ModuleShape = ModuleShape.SQUARE,
): ModuleGrid {
    // Build a 2D list of `false` values.  Each row is an independent list so
    // that setModule() can replace individual rows without copying the whole grid.
    val modules = List(rows) { List(cols) { false } }
    return ModuleGrid(rows = rows, cols = cols, modules = modules, moduleShape = moduleShape)
}

// ============================================================================
// setModule — immutable single-module update
// ============================================================================

/**
 * Return a new [ModuleGrid] identical to [grid] except that module at
 * `(row, col)` is set to [dark].
 *
 * This function is **pure and immutable** — it never modifies the input grid.
 * The original grid remains valid and unchanged.  Only the affected row is
 * re-allocated; all other rows are shared between old and new grids.
 *
 * ### Why immutability matters
 *
 * Barcode encoders often need to backtrack (e.g. trying different QR mask
 * patterns).  Immutable grids make this trivial — save the grid before trying
 * a mask, evaluate it, discard if the score is worse, keep the old one if
 * it is better.  No undo stack needed.
 *
 * ### Out-of-bounds
 *
 * Throws [IndexOutOfBoundsException] if [row] or [col] is outside the grid
 * dimensions.  This is a programming error in the encoder, not a user error.
 *
 * ### Example
 *
 * ```kotlin
 * val g  = makeModuleGrid(3, 3)
 * val g2 = setModule(g, row = 1, col = 1, dark = true)
 * // g.modules[1][1]  == false   (original unchanged)
 * // g2.modules[1][1] == true
 * // g !== g2                     (new object)
 * ```
 *
 * @param grid The source grid.
 * @param row  Row index (0 = top row).
 * @param col  Column index (0 = leftmost column).
 * @param dark `true` to make the module dark; `false` to make it light.
 * @return A new [ModuleGrid] with the specified module changed.
 * @throws IndexOutOfBoundsException if [row] or [col] is out of bounds.
 */
fun setModule(
    grid: ModuleGrid,
    row: Int,
    col: Int,
    dark: Boolean,
): ModuleGrid {
    if (row < 0 || row >= grid.rows) {
        throw IndexOutOfBoundsException(
            "setModule: row $row out of range [0, ${grid.rows - 1}]"
        )
    }
    if (col < 0 || col >= grid.cols) {
        throw IndexOutOfBoundsException(
            "setModule: col $col out of range [0, ${grid.cols - 1}]"
        )
    }

    // Copy only the affected row; all other rows are re-used as-is.
    val newRow: List<Boolean> = grid.modules[row].toMutableList().also { it[col] = dark }

    val newModules = grid.modules.mapIndexed { r, existingRow ->
        if (r == row) newRow else existingRow
    }

    return grid.copy(modules = newModules)
}

// ============================================================================
// layout — ModuleGrid → PaintScene
// ============================================================================

/**
 * Convert a [ModuleGrid] into a [PaintScene] ready for the PaintVM.
 *
 * This is the **only** function in the entire 2D barcode stack that knows
 * about pixels.  Everything above this step works in abstract module units.
 * Everything below this step is handled by the paint backend.
 *
 * ### Square modules (the common case)
 *
 * Each dark module at `(row, col)` becomes one [PaintInstruction.PaintRect]:
 *
 * ```
 * quietZonePx = quietZoneModules * moduleSizePx
 * x           = quietZonePx + col * moduleSizePx
 * y           = quietZonePx + row * moduleSizePx
 * ```
 *
 * Total symbol size including quiet zone on all four sides:
 *
 * ```
 * totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
 * totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
 * ```
 *
 * The scene always starts with one background [PaintInstruction.PaintRect]
 * covering the full symbol.  This ensures the quiet zone and light modules
 * are filled with the background colour even if the backend has a transparent
 * default.
 *
 * ### Hex modules (MaxiCode)
 *
 * Each dark module at `(row, col)` becomes one [PaintInstruction.PaintPath]
 * tracing a flat-top regular hexagon.  Odd-numbered rows are offset by half
 * a hexagon width to produce the standard hexagonal tiling:
 *
 * ```
 * Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
 * Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
 * Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
 * ```
 *
 * Center of hexagon at `(row, col)`:
 *
 * ```
 * hexWidth  = moduleSizePx
 * hexHeight = moduleSizePx * (√3 / 2)     ← vertical row step
 * circumR   = moduleSizePx / √3           ← center to vertex
 *
 * cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2)
 * cy = quietZonePx + row * hexHeight
 * ```
 *
 * Vertices at angles 0°, 60°, 120°, 180°, 240°, 300° from center.
 *
 * ### Validation
 *
 * Throws [InvalidBarcode2DConfigException] if:
 * - [Barcode2DLayoutConfig.moduleSizePx] ≤ 0
 * - [Barcode2DLayoutConfig.quietZoneModules] < 0
 * - [Barcode2DLayoutConfig.moduleShape] ≠ [ModuleGrid.moduleShape]
 *
 * @param grid   The module grid to render.
 * @param config Layout configuration.  All fields have defaults; pass
 *               `Barcode2DLayoutConfig()` for a fully-default config.
 * @return A [PaintScene] ready for the PaintVM.
 * @throws [InvalidBarcode2DConfigException] on invalid configuration.
 */
fun layout(
    grid: ModuleGrid,
    config: Barcode2DLayoutConfig = Barcode2DLayoutConfig(),
): PaintScene {
    // ── Validation ────────────────────────────────────────────────────────────
    if (config.moduleSizePx <= 0) {
        throw InvalidBarcode2DConfigException(
            "moduleSizePx must be > 0, got ${config.moduleSizePx}"
        )
    }
    if (config.quietZoneModules < 0) {
        throw InvalidBarcode2DConfigException(
            "quietZoneModules must be >= 0, got ${config.quietZoneModules}"
        )
    }
    if (config.moduleShape != grid.moduleShape) {
        throw InvalidBarcode2DConfigException(
            "config.moduleShape ${config.moduleShape} does not match " +
                    "grid.moduleShape ${grid.moduleShape}"
        )
    }

    // ── Dispatch to the correct rendering path ────────────────────────────────
    return when (config.moduleShape) {
        ModuleShape.SQUARE -> layoutSquare(grid, config)
        ModuleShape.HEX -> layoutHex(grid, config)
    }
}

// ============================================================================
// layoutSquare — internal helper for square-module grids
// ============================================================================

/**
 * Render a square-module [ModuleGrid] into a [PaintScene].
 *
 * Called only by [layout] after validation.  Not exported because callers
 * should always go through [layout] to ensure the config is validated.
 *
 * The algorithm is straightforward:
 *
 * 1. Compute total pixel dimensions including quiet zone.
 * 2. Emit one background [PaintInstruction.PaintRect] covering the entire symbol.
 * 3. For each dark module, emit one filled [PaintInstruction.PaintRect].
 *
 * Light modules are implicitly covered by the background rect — no explicit
 * light rects are emitted.  This keeps the instruction count proportional to
 * the number of dark modules rather than the total grid size.
 */
private fun layoutSquare(grid: ModuleGrid, config: Barcode2DLayoutConfig): PaintScene {
    val moduleSizePx = config.moduleSizePx
    val quietZoneModules = config.quietZoneModules

    // Quiet zone in pixels on each side.
    val quietZonePx = quietZoneModules * moduleSizePx

    // Total canvas dimensions including quiet zone on all four sides.
    val totalWidth = (grid.cols + 2 * quietZoneModules) * moduleSizePx
    val totalHeight = (grid.rows + 2 * quietZoneModules) * moduleSizePx

    val instructions = mutableListOf<PaintInstruction>()

    // 1. Background: a single rect covering the entire symbol including quiet
    //    zone.  This ensures light modules and the quiet zone are always filled,
    //    even when the backend default is transparent.
    instructions.add(
        paintRect(
            x = 0, y = 0,
            width = totalWidth, height = totalHeight,
            fill = config.background,
        )
    )

    // 2. One PaintRect per dark module.
    for (row in 0 until grid.rows) {
        for (col in 0 until grid.cols) {
            if (grid.modules[row][col]) {
                // Pixel origin of this module (top-left corner of its square).
                val x = quietZonePx + col * moduleSizePx
                val y = quietZonePx + row * moduleSizePx

                instructions.add(
                    paintRect(
                        x = x, y = y,
                        width = moduleSizePx, height = moduleSizePx,
                        fill = config.foreground,
                    )
                )
            }
        }
    }

    return createScene(
        width = totalWidth,
        height = totalHeight,
        background = config.background,
        instructions = instructions,
    )
}

// ============================================================================
// layoutHex — internal helper for hex-module grids (MaxiCode)
// ============================================================================

/**
 * Render a hex-module [ModuleGrid] into a [PaintScene].
 *
 * Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
 * offset-row grid.  Odd rows are shifted right by half a hexagon width.
 *
 * ### Flat-top hexagon geometry
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
 * ```
 * hexWidth  = moduleSizePx
 * hexHeight = moduleSizePx * (√3 / 2)   ← vertical distance between row centres
 * circumR   = moduleSizePx / √3         ← center to vertex
 *
 * cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2.0)
 * cy = quietZonePx + row * hexHeight
 * ```
 */
private fun layoutHex(grid: ModuleGrid, config: Barcode2DLayoutConfig): PaintScene {
    val moduleSizePx = config.moduleSizePx.toDouble()
    val quietZoneModules = config.quietZoneModules

    // Hex geometry:
    //   hexWidth  = one module width (flat-to-flat = side length for regular hex)
    //   hexHeight = vertical distance between row centres
    //   circumR   = circumscribed circle radius (centre-to-vertex distance)
    val hexWidth = moduleSizePx
    val hexHeight = moduleSizePx * (sqrt(3.0) / 2.0)
    val circumR = moduleSizePx / sqrt(3.0)

    val quietZonePx = quietZoneModules * moduleSizePx

    // Total canvas size.  The +hexWidth/2 accounts for the odd-row offset so
    // the rightmost modules on odd rows don't clip outside the canvas.
    val totalWidth = ((grid.cols + 2 * quietZoneModules) * hexWidth + hexWidth / 2.0).toInt()
    val totalHeight = ((grid.rows + 2 * quietZoneModules) * hexHeight).toInt()

    val instructions = mutableListOf<PaintInstruction>()

    // Background rect.
    instructions.add(
        paintRect(
            x = 0, y = 0,
            width = totalWidth, height = totalHeight,
            fill = config.background,
        )
    )

    // One PaintPath per dark module.
    for (row in 0 until grid.rows) {
        for (col in 0 until grid.cols) {
            if (grid.modules[row][col]) {
                // Centre of this hexagon in pixel space.
                // Odd rows shift right by hexWidth/2.
                val cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2.0)
                val cy = quietZonePx + row * hexHeight

                instructions.add(
                    paintPath(
                        commands = buildFlatTopHexPath(cx, cy, circumR),
                        fill = config.foreground,
                    )
                )
            }
        }
    }

    return createScene(
        width = totalWidth,
        height = totalHeight,
        background = config.background,
        instructions = instructions,
    )
}

// ============================================================================
// buildFlatTopHexPath — geometry helper
// ============================================================================

/**
 * Build the seven [PathCommand]s for a flat-top regular hexagon: six vertex
 * commands plus one [PathCommand.ClosePath].
 *
 * The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300° from
 * the centre `(cx, cy)` at circumradius `circumR`:
 *
 * ```
 * vertex_i = ( cx + circumR * cos(i * 60°),
 *              cy + circumR * sin(i * 60°) )
 * ```
 *
 * The path starts with a [PathCommand.MoveTo] to vertex 0, then five
 * [PathCommand.LineTo] commands to vertices 1–5, then [PathCommand.ClosePath]
 * to return to vertex 0.
 *
 * @param cx       Centre x coordinate in pixels.
 * @param cy       Centre y coordinate in pixels.
 * @param circumR  Circumscribed circle radius in pixels (centre to vertex).
 * @return List of 7 path commands describing the hexagon.
 */
internal fun buildFlatTopHexPath(cx: Double, cy: Double, circumR: Double): List<PathCommand> {
    val commands = mutableListOf<PathCommand>()
    val degToRad = Math.PI / 180.0

    // Vertex 0 → MoveTo; vertices 1..5 → LineTo.
    for (i in 0..5) {
        val angle = i * 60.0 * degToRad
        val vx = cx + circumR * cos(angle)
        val vy = cy + circumR * sin(angle)
        if (i == 0) {
            commands.add(PathCommand.MoveTo(vx, vy))
        } else {
            commands.add(PathCommand.LineTo(vx, vy))
        }
    }

    // Close back to vertex 0.
    commands.add(PathCommand.ClosePath)

    return commands
}
