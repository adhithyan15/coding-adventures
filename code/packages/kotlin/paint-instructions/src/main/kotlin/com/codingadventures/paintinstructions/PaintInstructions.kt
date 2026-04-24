/**
 * paint-instructions — backend-neutral 2D paint scene model.
 *
 * This package defines the lightweight intermediate representation that sits
 * between a high-level drawing abstraction (like a barcode layout engine) and
 * a concrete rendering backend (SVG, Canvas, Metal, terminal, etc.).
 *
 * ## Why an intermediate representation?
 *
 * Without a neutral IR, every barcode encoder would need to know how to draw
 * rectangles in SVG, or hexagons on a terminal, or pixels in Metal.  That
 * coupling explodes the number of combinations: N encoders × M backends.
 *
 * With an IR we only need N encoder → IR adapters and M IR → backend adapters.
 * The encoders and backends never know about each other.
 *
 * ```
 * QR encoder  ─┐
 * DataMatrix  ─┼──→  PaintScene  ──→  SVG backend
 * MaxiCode    ─┘                  ──→  Canvas backend
 *                                  ──→  Terminal backend
 * ```
 *
 * ## What is a PaintScene?
 *
 * A [PaintScene] is a canvas with a background colour plus an ordered list of
 * [PaintInstruction] objects.  Instructions are rendered front-to-back in the
 * order they appear in the list.
 *
 * ## Instruction types
 *
 * - [PaintInstruction.PaintRect] — a filled axis-aligned rectangle.
 *   Used by square-module barcodes (QR Code, Data Matrix, Aztec, PDF417).
 *
 * - [PaintInstruction.PaintPath] — a filled closed polygon described by a
 *   sequence of [PathCommand] objects.  Used by hex-module barcodes (MaxiCode).
 *
 * ## Immutability
 *
 * All types in this package are immutable data classes or sealed hierarchies.
 * This makes scenes easy to cache, share, and test: there is no mutation,
 * no defensive copying, and no race conditions.
 *
 * ## Relationship to the Go reference
 *
 * The Go reference (code/packages/go/paint-instructions/paint_instructions.go)
 * defines the same types.  This Kotlin port uses idiomatic sealed classes and
 * data classes in place of Go interfaces and structs, but the semantics are
 * identical.
 *
 * Spec: P2D01 paint-instructions spec.
 */
package com.codingadventures.paintinstructions

/** Package version.  Follows Semantic Versioning 2.0. */
const val VERSION = "0.1.0"

// ============================================================================
// Metadata — arbitrary key/value annotations on scenes and instructions
// ============================================================================

/**
 * Arbitrary metadata attached to a scene or instruction.
 *
 * Metadata is an escape hatch for consumers that need to carry format-specific
 * information through the pipeline without modifying the core types.  For
 * example, a QR Code encoder might attach `"qr:version" → "3"` to the scene's
 * metadata so that a debugging visualizer can display it.
 *
 * Metadata values are always [String] to keep serialisation trivial.
 *
 * ### Example
 *
 * ```kotlin
 * val meta = mapOf("source" to "qr-encoder", "version" to "3")
 * ```
 */
typealias Metadata = Map<String, String>

// ============================================================================
// PaintColorRGBA8 — 32-bit RGBA colour
// ============================================================================

/**
 * A 32-bit RGBA colour with 8 bits per channel.
 *
 * Channels are in the range 0–255:
 * - [r] = red   (0 = no red,    255 = full red)
 * - [g] = green (0 = no green,  255 = full green)
 * - [b] = blue  (0 = no blue,   255 = full blue)
 * - [a] = alpha (0 = fully transparent, 255 = fully opaque)
 *
 * ### Parsing from hex strings
 *
 * Use [parseColorRGBA8] to convert CSS hex strings (#rgb, #rgba, #rrggbb,
 * #rrggbbaa) into a [PaintColorRGBA8].
 *
 * ### Example
 *
 * ```kotlin
 * val red   = PaintColorRGBA8(r = 255, g = 0,   b = 0,   a = 255)
 * val white = PaintColorRGBA8(r = 255, g = 255, b = 255, a = 255)
 * ```
 */
data class PaintColorRGBA8(
    /** Red channel, 0–255. */
    val r: Int,
    /** Green channel, 0–255. */
    val g: Int,
    /** Blue channel, 0–255. */
    val b: Int,
    /** Alpha channel, 0–255. 255 = fully opaque. */
    val a: Int,
)

// ============================================================================
// PathCommand — a single drawing command within a vector path
// ============================================================================

/**
 * A single command in a vector path.
 *
 * Paths are sequences of commands that together describe a closed polygon.
 * The typical pattern for a hexagon is:
 *
 * ```
 * MoveTo(x0, y0) → LineTo(x1, y1) → LineTo(x2, y2) → … → ClosePath
 * ```
 *
 * Each subtype carries the geometry needed for that command:
 *
 * - [MoveTo] — lift the pen and move to `(x, y)`.  Every path starts here.
 * - [LineTo] — draw a straight line from the current position to `(x, y)`.
 * - [ClosePath] — close the current sub-path with a straight line back to the
 *   most recent [MoveTo] point.
 *
 * ### Why a sealed class?
 *
 * Using a sealed class means Kotlin's `when` expression is exhaustive: the
 * compiler will warn if you forget to handle one of the command types.  This
 * makes backend rendering code much safer than the string-tagged approach used
 * in the Go reference.
 *
 * ### Example — equilateral triangle
 *
 * ```kotlin
 * val triangle = listOf(
 *     PathCommand.MoveTo(0.0, 0.0),
 *     PathCommand.LineTo(10.0, 0.0),
 *     PathCommand.LineTo(5.0, 8.66),
 *     PathCommand.ClosePath,
 * )
 * ```
 */
sealed class PathCommand {

    /**
     * Lift the pen and move to `(x, y)` without drawing.
     *
     * This is always the first command in a path (or the first command after a
     * [ClosePath] if multiple sub-paths are needed).
     *
     * @param x Horizontal position in pixels.
     * @param y Vertical position in pixels.
     */
    data class MoveTo(val x: Double, val y: Double) : PathCommand()

    /**
     * Draw a straight line from the current pen position to `(x, y)`.
     *
     * @param x Horizontal position in pixels.
     * @param y Vertical position in pixels.
     */
    data class LineTo(val x: Double, val y: Double) : PathCommand()

    /**
     * Close the current sub-path.
     *
     * Draws a straight line from the current position back to the point
     * specified in the most recent [MoveTo].  For a convex polygon the result
     * is a closed filled shape.
     */
    data object ClosePath : PathCommand()
}

// ============================================================================
// PaintInstruction — a single drawing instruction
// ============================================================================

/**
 * A single drawing instruction inside a [PaintScene].
 *
 * Instructions are polymorphic via a sealed class.  The two concrete subtypes
 * cover the two shapes needed by all current 2D barcode standards:
 *
 * - [PaintRect] for square-module barcodes (QR, Data Matrix, Aztec, PDF417).
 * - [PaintPath] for hex-module barcodes (MaxiCode).
 *
 * ### Sealed class benefits
 *
 * The sealed hierarchy guarantees exhaustive `when` coverage.  If a new
 * instruction type is ever added (e.g. `PaintCircle` for a bullseye ring), the
 * compiler immediately flags every `when` block that needs updating.
 */
sealed class PaintInstruction {

    /**
     * A filled axis-aligned rectangle.
     *
     * Coordinates use the top-left corner as origin with the x axis pointing
     * right and the y axis pointing down — the standard 2D raster convention.
     *
     * ```
     * (x, y) ─────────────────────┐
     *   │                         │
     *   │   PaintRect             │  height
     *   │                         │
     *   └─────────────────────────┘
     *              width
     * ```
     *
     * @param x        Left edge of the rectangle in pixels.
     * @param y        Top edge of the rectangle in pixels.
     * @param width    Width of the rectangle in pixels.  Must be ≥ 0.
     * @param height   Height of the rectangle in pixels.  Must be ≥ 0.
     * @param fill     CSS colour string for the fill, e.g. `"#000000"`.
     * @param metadata Optional key/value annotations.
     */
    data class PaintRect(
        val x: Int,
        val y: Int,
        val width: Int,
        val height: Int,
        val fill: String,
        val metadata: Metadata = emptyMap(),
    ) : PaintInstruction()

    /**
     * A filled closed polygon described by a list of [PathCommand]s.
     *
     * The commands must form a closed shape: they should start with a [PathCommand.MoveTo]
     * and end with [PathCommand.ClosePath].  The resulting polygon is filled
     * with the [fill] colour.
     *
     * ### Typical use — flat-top hexagon
     *
     * MaxiCode uses flat-top hexagons arranged in an offset-row grid.  Each
     * dark module in a MaxiCode grid becomes one [PaintPath] instruction whose
     * six vertices are computed from the module's (row, col) position.
     *
     * ```kotlin
     * // Flat-top hex centered at (cx, cy) with circumradius R:
     * val commands = (0 until 6).flatMap { i ->
     *     val angle = Math.toRadians(i * 60.0)
     *     val vx = cx + R * cos(angle)
     *     val vy = cy + R * sin(angle)
     *     if (i == 0) listOf(PathCommand.MoveTo(vx, vy))
     *     else        listOf(PathCommand.LineTo(vx, vy))
     * } + listOf(PathCommand.ClosePath)
     * ```
     *
     * @param commands Ordered list of path commands describing the polygon.
     * @param fill     CSS colour string for the fill, e.g. `"#000000"`.
     * @param metadata Optional key/value annotations.
     */
    data class PaintPath(
        val commands: List<PathCommand>,
        val fill: String,
        val metadata: Metadata = emptyMap(),
    ) : PaintInstruction()
}

// ============================================================================
// PaintScene — the complete rendering description
// ============================================================================

/**
 * A complete rendering description: canvas dimensions, background colour, and
 * an ordered list of paint instructions.
 *
 * A [PaintScene] is the final product of a layout engine.  It is passed to a
 * backend renderer that interprets the instructions to produce pixels, SVG
 * paths, terminal characters, or whatever output the backend supports.
 *
 * ### Rendering order
 *
 * Instructions are rendered front-to-back in the order they appear in
 * [instructions].  The background [background] colour fills the entire canvas
 * before any instructions are applied.  Typically the first instruction is also
 * a full-canvas background rectangle to ensure the quiet zone is always visible
 * even when the backend has a transparent default.
 *
 * ### Example
 *
 * ```kotlin
 * val scene = createScene(
 *     width = 210,
 *     height = 210,
 *     background = "#ffffff",
 *     instructions = listOf(
 *         paintRect(0, 0, 210, 210),        // white background
 *         paintRect(10, 10, 10, 10),         // top-left finder module
 *     ),
 * )
 * ```
 *
 * @param width        Canvas width in pixels.
 * @param height       Canvas height in pixels.
 * @param background   CSS colour string for the background, e.g. `"#ffffff"`.
 * @param instructions Ordered list of paint instructions to render.
 * @param metadata     Optional key/value annotations for the whole scene.
 */
data class PaintScene(
    val width: Int,
    val height: Int,
    val background: String,
    val instructions: List<PaintInstruction>,
    val metadata: Metadata = emptyMap(),
)

// ============================================================================
// Helper constructors — paintRect, paintPath, createScene
// ============================================================================

/**
 * Build a [PaintInstruction.PaintRect].
 *
 * This helper function matches the API of the Go reference's `PaintRect()`
 * function.  It sets sensible defaults: if [fill] is empty or blank it
 * defaults to `"#000000"` (black).
 *
 * @param x        Left edge in pixels.
 * @param y        Top edge in pixels.
 * @param width    Width in pixels.
 * @param height   Height in pixels.
 * @param fill     CSS fill colour.  Defaults to `"#000000"` if blank.
 * @param metadata Optional annotations.
 * @return A ready-to-use [PaintInstruction.PaintRect].
 *
 * ### Example
 *
 * ```kotlin
 * val darkModule = paintRect(x = 10, y = 20, width = 10, height = 10)
 * ```
 */
fun paintRect(
    x: Int,
    y: Int,
    width: Int,
    height: Int,
    fill: String = "#000000",
    metadata: Metadata = emptyMap(),
): PaintInstruction.PaintRect =
    PaintInstruction.PaintRect(
        x = x,
        y = y,
        width = width,
        height = height,
        fill = fill.ifBlank { "#000000" },
        metadata = metadata,
    )

/**
 * Build a [PaintInstruction.PaintPath].
 *
 * This helper function matches the API of the Go reference's `PaintPath()`
 * function.  If [fill] is empty or blank it defaults to `"#000000"` (black).
 *
 * @param commands Ordered path commands describing the polygon.
 * @param fill     CSS fill colour.  Defaults to `"#000000"` if blank.
 * @param metadata Optional annotations.
 * @return A ready-to-use [PaintInstruction.PaintPath].
 *
 * ### Example — flat-top hexagon
 *
 * ```kotlin
 * val hex = paintPath(
 *     commands = buildHexCommands(cx = 50.0, cy = 50.0, r = 6.0),
 *     fill = "#1a1a1a",
 * )
 * ```
 */
fun paintPath(
    commands: List<PathCommand>,
    fill: String = "#000000",
    metadata: Metadata = emptyMap(),
): PaintInstruction.PaintPath =
    PaintInstruction.PaintPath(
        commands = commands,
        fill = fill.ifBlank { "#000000" },
        metadata = metadata,
    )

/**
 * Build a [PaintScene].
 *
 * This helper function matches the API of the Go reference's `CreateScene()`
 * function.  Defaults:
 * - [background] defaults to `"#ffffff"` (white) when blank.
 *
 * @param width        Canvas width in pixels.
 * @param height       Canvas height in pixels.
 * @param background   CSS background colour.  Defaults to `"#ffffff"` if blank.
 * @param instructions Ordered list of paint instructions.
 * @param metadata     Optional annotations for the whole scene.
 * @return A ready-to-use [PaintScene].
 *
 * ### Example
 *
 * ```kotlin
 * val scene = createScene(
 *     width = 100,
 *     height = 100,
 *     instructions = listOf(paintRect(10, 10, 5, 5)),
 * )
 * ```
 */
fun createScene(
    width: Int,
    height: Int,
    background: String = "#ffffff",
    instructions: List<PaintInstruction> = emptyList(),
    metadata: Metadata = emptyMap(),
): PaintScene =
    PaintScene(
        width = width,
        height = height,
        background = background.ifBlank { "#ffffff" },
        instructions = instructions,
        metadata = metadata,
    )

// ============================================================================
// parseColorRGBA8 — CSS hex string parser
// ============================================================================

/**
 * Parse a CSS hex colour string into a [PaintColorRGBA8].
 *
 * Accepted formats (case-insensitive, leading `#` required):
 *
 * | Format     | Example     | Expansion rule                         |
 * |------------|-------------|----------------------------------------|
 * | `#rgb`     | `#f0a`      | Each nibble doubled; alpha = FF        |
 * | `#rgba`    | `#f0a8`     | Each nibble doubled                    |
 * | `#rrggbb`  | `#ff00aa`   | As-is; alpha = FF                      |
 * | `#rrggbbaa`| `#ff00aa80` | As-is                                  |
 *
 * ### Why these formats?
 *
 * CSS and SVG both use `#rrggbb`.  Many design tools also output `#rgb` for
 * shorter notation.  The `#rgba` and `#rrggbbaa` forms carry transparency
 * which is useful for semi-transparent overlays in rendering backends.
 *
 * @param value CSS hex colour string.
 * @return Parsed [PaintColorRGBA8].
 * @throws IllegalArgumentException if the string is not a recognised format.
 *
 * ### Examples
 *
 * ```kotlin
 * parseColorRGBA8("#000")    // PaintColorRGBA8(0, 0, 0, 255)
 * parseColorRGBA8("#ffffff") // PaintColorRGBA8(255, 255, 255, 255)
 * parseColorRGBA8("#ff000080") // PaintColorRGBA8(255, 0, 0, 128)
 * ```
 */
fun parseColorRGBA8(value: String): PaintColorRGBA8 {
    val trimmed = value.trim()
    require(trimmed.startsWith("#")) {
        "paint color must start with '#', got: $trimmed"
    }

    // Strip the leading '#' and normalise to exactly 8 hex digits (rrggbbaa).
    val hex = when (val raw = trimmed.drop(1)) {
        // #rgb → #rrggbbff
        in Regex("^[0-9a-fA-F]{3}$") ->
            "${raw[0]}${raw[0]}${raw[1]}${raw[1]}${raw[2]}${raw[2]}ff"
        // #rgba → #rrggbbaa
        in Regex("^[0-9a-fA-F]{4}$") ->
            "${raw[0]}${raw[0]}${raw[1]}${raw[1]}${raw[2]}${raw[2]}${raw[3]}${raw[3]}"
        // #rrggbb → #rrggbbff
        in Regex("^[0-9a-fA-F]{6}$") ->
            "${raw}ff"
        // #rrggbbaa — already the canonical form
        in Regex("^[0-9a-fA-F]{8}$") ->
            raw
        else ->
            throw IllegalArgumentException(
                "paint color must be #rgb, #rgba, #rrggbb, or #rrggbbaa, got: $trimmed"
            )
    }

    // Parse each 2-hex-digit channel.
    fun channel(offset: Int): Int =
        hex.substring(offset, offset + 2).toInt(16)

    return PaintColorRGBA8(
        r = channel(0),
        g = channel(2),
        b = channel(4),
        a = channel(6),
    )
}

// ============================================================================
// Regex extension helper
// ============================================================================

/**
 * Allow `string in Regex(...)` syntax for concise pattern matching.
 *
 * This private extension makes the `when` expression in [parseColorRGBA8]
 * readable without requiring full `matches()` calls.
 */
private operator fun Regex.contains(value: String): Boolean = matches(value)
