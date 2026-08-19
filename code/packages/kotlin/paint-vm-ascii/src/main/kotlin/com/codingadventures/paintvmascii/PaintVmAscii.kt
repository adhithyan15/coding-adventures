/**
 * A small, pure terminal backend for [PaintScene] values.
 *
 * Implements the full `P2D02-paint-vm-ascii.md` contract: filled/stroked
 * rectangles, lines, glyph runs, and plain (untransformed, unfiltered,
 * fully opaque) groups/clips/layers. Scene coordinates are divided by a
 * configurable horizontal and vertical scale to obtain character-cell
 * coordinates.
 *
 * The buffer is a [Map] from `(row, col)` to a [Cell], rather than a
 * mutable 2D array — scenes rendered by this backend are small
 * (terminal-sized, capped by [MAX_AXIS_CELLS]), so the simplicity of a
 * sparse map outweighs any performance concern, and it keeps the
 * box-drawing merge logic (two strokes sharing a corner combine into one
 * character) expressible without a pre-sized grid.
 *
 * Spec: P2D02 paint-vm-ascii.
 */
package com.codingadventures.paintvmascii

import com.codingadventures.paintinstructions.PaintGlyphPlacement
import com.codingadventures.paintinstructions.PaintInstruction
import com.codingadventures.paintinstructions.PaintScene
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min

/** Package version, shared with the other language implementations. */
const val VERSION = "0.1.0"

/** How scene coordinates map to terminal character cells. */
data class AsciiOptions(val scaleX: Int, val scaleY: Int) {
    companion object {
        /** The cross-language default: cells eight scene units wide, sixteen tall. */
        val DEFAULT = AsciiOptions(8, 16)
    }
}

/**
 * Errors this backend can report without throwing or returning a partial
 * rendering.
 */
sealed interface PaintVmAsciiError {
    /** [AsciiOptions.scaleX] was not a positive integer. */
    data class InvalidScaleX(val scaleX: Int) : PaintVmAsciiError

    /** [AsciiOptions.scaleY] was not a positive integer. */
    data class InvalidScaleY(val scaleY: Int) : PaintVmAsciiError

    /** The scene's width or height was negative. */
    data class InvalidSceneDimensions(val width: Int, val height: Int) : PaintVmAsciiError

    /**
     * The scene's cell-grid size (width/scaleX by height/scaleY) exceeds
     * the bound this backend is willing to materialize. Checked both
     * per-axis and by total cell count — a product-only check can be
     * bypassed by a zero-width, huge-height (or vice versa) scene.
     */
    data class SceneTooLarge(val width: Int, val height: Int) : PaintVmAsciiError

    /** A [PaintInstruction.PaintRect]'s width or height was negative. */
    data class InvalidRectangleGeometry(val x: Int, val y: Int, val width: Int, val height: Int) : PaintVmAsciiError

    /** A [PaintInstruction.PaintLine]'s coordinates included a NaN or infinite value. */
    data class InvalidLineGeometry(val x1: Double, val y1: Double, val x2: Double, val y2: Double) : PaintVmAsciiError

    /**
     * A [PaintInstruction.PaintClip]'s coordinates were non-finite, either
     * directly or via the `x+width`/`y+height` extent (two individually
     * finite values can sum to infinity).
     */
    data class InvalidClipGeometry(val x: Double, val y: Double, val width: Double, val height: Double) : PaintVmAsciiError

    /**
     * A [PaintInstruction.PaintGroup] or [PaintInstruction.PaintLayer] used
     * a feature this text-mode backend cannot represent (non-identity
     * transform, non-default opacity, filters, non-normal blend mode), or
     * the instruction is a [PaintInstruction.PaintPath] (this backend
     * renders no vector geometry).
     */
    data class UnsupportedInstruction(val reason: String) : PaintVmAsciiError
}

/** The outcome of [render]: either rendered text, or an error. */
sealed interface PaintVmAsciiResult {
    data class Ok(val text: String) : PaintVmAsciiResult

    data class Err(val error: PaintVmAsciiError) : PaintVmAsciiResult
}

// ---------------------------------------------------------------------------
// Buffer
// ---------------------------------------------------------------------------

private const val FLAG_UP = 1
private const val FLAG_RIGHT = 2
private const val FLAG_DOWN = 4
private const val FLAG_LEFT = 8
private const val FLAG_FILL = 16

private val BOX_CHARACTERS: Map<Int, Char> = mapOf(
    (FLAG_LEFT or FLAG_RIGHT) to '─',
    (FLAG_UP or FLAG_DOWN) to '│',
    (FLAG_DOWN or FLAG_RIGHT) to '┌',
    (FLAG_DOWN or FLAG_LEFT) to '┐',
    (FLAG_UP or FLAG_RIGHT) to '└',
    (FLAG_UP or FLAG_LEFT) to '┘',
    (FLAG_LEFT or FLAG_RIGHT or FLAG_DOWN) to '┬',
    (FLAG_LEFT or FLAG_RIGHT or FLAG_UP) to '┴',
    (FLAG_UP or FLAG_DOWN or FLAG_RIGHT) to '├',
    (FLAG_UP or FLAG_DOWN or FLAG_LEFT) to '┤',
    (FLAG_UP or FLAG_DOWN or FLAG_LEFT or FLAG_RIGHT) to '┼',
    FLAG_RIGHT to '─',
    FLAG_LEFT to '─',
    FLAG_UP to '│',
    FLAG_DOWN to '│',
)

private const val FILL_CHAR = '█'

/** One character cell. [CellText] always wins over [CellTag] — literal text is never overwritten. */
private sealed interface Cell {
    data class CellTag(val flags: Int) : Cell
    data class CellText(val ch: Char) : Cell
}

private data class Point(val row: Int, val col: Int)

private data class ClipBounds(val minCol: Int, val minRow: Int, val maxCol: Int, val maxRow: Int) {
    fun inside(row: Int, col: Int): Boolean = row >= minRow && row < maxRow && col >= minCol && col < maxCol

    /**
     * Clamp a cell coordinate into this clip's bounds. Used before building
     * any range that iterates between two cell coordinates (rect
     * fill/stroke, line endpoints), so a caller-supplied geometry with a
     * huge (but valid) extent can't force iteration/recursion far beyond
     * the actual clipped surface — bounded by the clip's own size instead
     * of by caller input.
     */
    fun clampCol(value: Int): Int = max(minCol, min(value, maxCol - 1))

    fun clampRow(value: Int): Int = max(minRow, min(value, maxRow - 1))

    fun intersect(child: ClipBounds): ClipBounds = ClipBounds(
        minCol = max(minCol, child.minCol),
        minRow = max(minRow, child.minRow),
        maxCol = min(maxCol, child.maxCol),
        maxRow = min(maxRow, child.maxRow),
    )
}

private fun writeTag(clip: ClipBounds, row: Int, col: Int, flags: Int, buffer: MutableMap<Point, Cell>) {
    if (!clip.inside(row, col)) return
    val p = Point(row, col)
    when (val existing = buffer[p]) {
        is Cell.CellText -> return
        is Cell.CellTag -> buffer[p] = Cell.CellTag(existing.flags or flags)
        null -> buffer[p] = Cell.CellTag(flags)
    }
}

private fun writeChar(clip: ClipBounds, row: Int, col: Int, ch: Char, buffer: MutableMap<Point, Cell>) {
    if (!clip.inside(row, col)) return
    buffer[Point(row, col)] = Cell.CellText(ch)
}

private fun resolveCell(cell: Cell): Char = when (cell) {
    is Cell.CellText -> cell.ch
    is Cell.CellTag -> {
        val directions = cell.flags and (FLAG_UP or FLAG_RIGHT or FLAG_DOWN or FLAG_LEFT)
        val boxChar = if (directions != 0) BOX_CHARACTERS[directions] else null
        when {
            boxChar != null -> boxChar
            (cell.flags and FLAG_FILL) != 0 -> FILL_CHAR
            else -> '+'
        }
    }
}

private fun bufferToText(rows: Int, columns: Int, buffer: Map<Point, Cell>): String {
    val lines = (0 until rows).map { row ->
        val line = (0 until columns).map { col -> buffer[Point(row, col)]?.let(::resolveCell) ?: ' ' }
        line.joinToString("").trimEnd(' ')
    }
    var lastNonBlank = lines.size
    while (lastNonBlank > 0 && lines[lastNonBlank - 1].isEmpty()) {
        lastNonBlank -= 1
    }
    return lines.subList(0, lastNonBlank).joinToString("\n")
}

// ---------------------------------------------------------------------------
// Coordinate conversion
// ---------------------------------------------------------------------------

/**
 * Cell-coordinate values are saturated to this bound (rather than left as
 * a raw rounded result) so a large-but-ordinary finite [Double] can never
 * land on exactly [Int.MIN_VALUE]/[Int.MAX_VALUE]. Without this, a clip
 * extent rounding to an extreme value could defeat
 * [ClipBounds.clampCol]/[ClipBounds.clampRow] downstream via integer
 * overflow in the `maxCol - 1` they compute, un-clamping any shape nested
 * in that clip and reopening the unbounded-iteration DoS the clip
 * clamping exists to prevent. A billion cells in either direction is far
 * beyond any real rendered scene (scenes are additionally capped at
 * [MAX_AXIS_CELLS] per axis) while leaving enormous headroom below
 * [Int]'s actual bounds for `clampCol`/`clampRow`'s arithmetic to stay
 * overflow-free.
 */
private const val CELL_BOUND = 1_000_000_000

private fun toCell(coordinate: Double, scale: Int): Int {
    val scaled = coordinate / scale
    return when {
        scaled.isNaN() -> 0
        scaled >= CELL_BOUND -> CELL_BOUND
        scaled <= -CELL_BOUND -> -CELL_BOUND
        else -> Math.round(scaled).toInt()
    }
}

/**
 * Converts an [Int]-valued coordinate ([PaintInstruction.PaintRect] uses
 * [Int], not [Double]) via the same saturating [toCell] path. Callers that
 * need to sum two [Int]s first (e.g. `x + width`) must do that addition in
 * [Long] before calling this, so two large [Int]s summing can't silently
 * wrap through [Int] overflow before ever reaching the saturation check.
 */
private fun toCellFromLong(coordinate: Long, scale: Int): Int = toCell(coordinate.toDouble(), scale)

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

private fun isFinite(value: Double): Boolean = !value.isNaN() && !value.isInfinite()

private fun validRectangle(r: PaintInstruction.PaintRect): Boolean = r.width >= 0 && r.height >= 0

private fun validLine(l: PaintInstruction.PaintLine): Boolean =
    isFinite(l.x1) && isFinite(l.y1) && isFinite(l.x2) && isFinite(l.y2)

/**
 * Validates the individual fields *and* the `x+width`/`y+height` extents
 * used by [clipBoundsOf] — two individually-finite values near
 * [Double.MAX_VALUE] can still sum to +Infinity under IEEE-754
 * arithmetic, so checking the fields alone isn't sufficient to guarantee
 * [toCell] never sees a non-finite input.
 */
private fun validClip(c: PaintInstruction.PaintClip): Boolean =
    isFinite(c.x) && isFinite(c.y) && isFinite(c.width) && isFinite(c.height) &&
        c.width >= 0 && c.height >= 0 &&
        isFinite(c.x + c.width) && isFinite(c.y + c.height)

private fun isIdentityTransform(transform: com.codingadventures.paintinstructions.Transform2D?): Boolean =
    transform == null || transform.isIdentity()

private fun assertPlainGroup(group: PaintInstruction.PaintGroup): PaintVmAsciiError? {
    if (!isIdentityTransform(group.transform)) {
        return PaintVmAsciiError.UnsupportedInstruction("group with a non-identity transform")
    }
    if (group.opacity != null && group.opacity != 1.0) {
        return PaintVmAsciiError.UnsupportedInstruction("group with non-default opacity")
    }
    return null
}

private fun assertPlainLayer(layer: PaintInstruction.PaintLayer): PaintVmAsciiError? {
    if (!isIdentityTransform(layer.transform)) {
        return PaintVmAsciiError.UnsupportedInstruction("layer with a non-identity transform")
    }
    if (layer.opacity != null && layer.opacity != 1.0) {
        return PaintVmAsciiError.UnsupportedInstruction("layer with non-default opacity")
    }
    if (layer.hasFilters) {
        return PaintVmAsciiError.UnsupportedInstruction("layer with filters")
    }
    if (layer.blendMode != null && layer.blendMode != "normal") {
        return PaintVmAsciiError.UnsupportedInstruction("layer with a non-normal blend mode")
    }
    return null
}

private fun visiblePaint(paint: String): Boolean {
    val trimmed = paint.trim()
    return trimmed.isNotEmpty() && trimmed != "transparent" && trimmed != "none"
}

// ---------------------------------------------------------------------------
// Top-level render
// ---------------------------------------------------------------------------

/**
 * Upper bound on the number of character cells a rendered scene may
 * occupy, both in total and per axis. Scene dimensions are otherwise only
 * checked for being non-negative, so without this a caller-supplied
 * width/height of e.g. one billion would force [bufferToText] to iterate
 * on an enormous number of cells even with zero drawing instructions — a
 * denial-of-service unrelated to (and not fixed by) the per-instruction
 * clip clamping. The per-axis bound is required in addition to the
 * product bound: a zero-width, huge-height scene has a product of zero
 * (passing a product-only check) while still forcing an unbounded
 * traversal along the surviving axis. 2000x2000 (a generous terminal-sized
 * canvas) is cheap to fully materialize either way.
 */
private const val MAX_AXIS_CELLS = 2000L
private const val MAX_BUFFER_CELLS = MAX_AXIS_CELLS * MAX_AXIS_CELLS

/** Render with [AsciiOptions.DEFAULT]. */
fun renderDefault(scene: PaintScene): PaintVmAsciiResult = render(scene, AsciiOptions.DEFAULT)

/** Render a scene as terminal-friendly text. */
fun render(scene: PaintScene, options: AsciiOptions): PaintVmAsciiResult {
    if (options.scaleX <= 0) return PaintVmAsciiResult.Err(PaintVmAsciiError.InvalidScaleX(options.scaleX))
    if (options.scaleY <= 0) return PaintVmAsciiResult.Err(PaintVmAsciiError.InvalidScaleY(options.scaleY))
    if (scene.width < 0 || scene.height < 0) {
        return PaintVmAsciiResult.Err(PaintVmAsciiError.InvalidSceneDimensions(scene.width, scene.height))
    }

    val columns = ceilDiv(scene.width.toLong(), options.scaleX.toLong())
    val rows = ceilDiv(scene.height.toLong(), options.scaleY.toLong())
    if (columns > MAX_AXIS_CELLS || rows > MAX_AXIS_CELLS || columns * rows > MAX_BUFFER_CELLS) {
        return PaintVmAsciiResult.Err(PaintVmAsciiError.SceneTooLarge(scene.width, scene.height))
    }

    val clip = ClipBounds(0, 0, columns.toInt(), rows.toInt())
    val buffer = mutableMapOf<Point, Cell>()
    for (instruction in scene.instructions) {
        val error = dispatch(options, clip, buffer, instruction)
        if (error != null) return PaintVmAsciiResult.Err(error)
    }
    return PaintVmAsciiResult.Ok(bufferToText(rows.toInt(), columns.toInt(), buffer))
}

private fun ceilDiv(numerator: Long, denominator: Long): Long = (numerator + denominator - 1) / denominator

/**
 * Render one instruction (recursing into group/clip/layer children),
 * mutating [buffer] in place and failing loudly on anything not in the
 * P2D02 contract. Returns `null` on success, the error otherwise.
 */
private fun dispatch(
    options: AsciiOptions,
    clip: ClipBounds,
    buffer: MutableMap<Point, Cell>,
    instruction: PaintInstruction,
): PaintVmAsciiError? = when (instruction) {
    is PaintInstruction.PaintRect -> {
        if (!validRectangle(instruction)) {
            PaintVmAsciiError.InvalidRectangleGeometry(instruction.x, instruction.y, instruction.width, instruction.height)
        } else {
            renderRectangle(options, clip, instruction, buffer)
            null
        }
    }
    is PaintInstruction.PaintLine -> {
        if (!validLine(instruction)) {
            PaintVmAsciiError.InvalidLineGeometry(instruction.x1, instruction.y1, instruction.x2, instruction.y2)
        } else {
            renderLine(options, clip, instruction, buffer)
            null
        }
    }
    is PaintInstruction.PaintGlyphRun -> {
        renderGlyphRun(options, clip, instruction, buffer)
        null
    }
    is PaintInstruction.PaintGroup -> {
        val plainCheck = assertPlainGroup(instruction)
        plainCheck ?: dispatchChildren(options, clip, buffer, instruction.children)
    }
    is PaintInstruction.PaintClip -> {
        if (!validClip(instruction)) {
            PaintVmAsciiError.InvalidClipGeometry(instruction.x, instruction.y, instruction.width, instruction.height)
        } else {
            val nextClip = clip.intersect(clipBoundsOf(options, instruction))
            dispatchChildren(options, nextClip, buffer, instruction.children)
        }
    }
    is PaintInstruction.PaintLayer -> {
        val plainCheck = assertPlainLayer(instruction)
        plainCheck ?: dispatchChildren(options, clip, buffer, instruction.children)
    }
    is PaintInstruction.PaintPath -> PaintVmAsciiError.UnsupportedInstruction("path")
}

private fun dispatchChildren(
    options: AsciiOptions,
    clip: ClipBounds,
    buffer: MutableMap<Point, Cell>,
    children: List<PaintInstruction>,
): PaintVmAsciiError? {
    for (child in children) {
        val error = dispatch(options, clip, buffer, child)
        if (error != null) return error
    }
    return null
}

private fun clipBoundsOf(options: AsciiOptions, c: PaintInstruction.PaintClip): ClipBounds = ClipBounds(
    minCol = toCell(c.x, options.scaleX),
    minRow = toCell(c.y, options.scaleY),
    maxCol = toCell(c.x + c.width, options.scaleX),
    maxRow = toCell(c.y + c.height, options.scaleY),
)

// ---------------------------------------------------------------------------
// Rect
// ---------------------------------------------------------------------------

private fun renderRectangle(
    options: AsciiOptions,
    clip: ClipBounds,
    r: PaintInstruction.PaintRect,
    buffer: MutableMap<Point, Cell>,
) {
    val c1 = clip.clampCol(toCellFromLong(r.x.toLong(), options.scaleX))
    val r1 = clip.clampRow(toCellFromLong(r.y.toLong(), options.scaleY))
    val c2 = clip.clampCol(toCellFromLong(r.x.toLong() + r.width, options.scaleX))
    val r2 = clip.clampRow(toCellFromLong(r.y.toLong() + r.height, options.scaleY))

    if (visiblePaint(r.fill)) {
        for (row in r1..r2) {
            for (col in c1..c2) {
                writeTag(clip, row, col, FLAG_FILL, buffer)
            }
        }
    }

    if (r.stroke.trim().isNotEmpty()) {
        writeTag(clip, r1, c1, FLAG_DOWN or FLAG_RIGHT, buffer)
        writeTag(clip, r1, c2, FLAG_DOWN or FLAG_LEFT, buffer)
        writeTag(clip, r2, c1, FLAG_UP or FLAG_RIGHT, buffer)
        writeTag(clip, r2, c2, FLAG_UP or FLAG_LEFT, buffer)
        for (col in (c1 + 1) until c2) {
            writeTag(clip, r1, col, FLAG_LEFT or FLAG_RIGHT, buffer)
            writeTag(clip, r2, col, FLAG_LEFT or FLAG_RIGHT, buffer)
        }
        for (row in (r1 + 1) until r2) {
            writeTag(clip, row, c1, FLAG_UP or FLAG_DOWN, buffer)
            writeTag(clip, row, c2, FLAG_UP or FLAG_DOWN, buffer)
        }
    }
}

// ---------------------------------------------------------------------------
// Line (horizontal/vertical fast paths + Bresenham for the diagonal case)
// ---------------------------------------------------------------------------

private fun renderLine(
    options: AsciiOptions,
    clip: ClipBounds,
    line: PaintInstruction.PaintLine,
    buffer: MutableMap<Point, Cell>,
) {
    // Clamped into the clip's own bounds before use — an out-of-range but
    // otherwise valid (finite) endpoint can't force iteration or
    // Bresenham recursion far beyond the actual clipped surface.
    val c1 = clip.clampCol(toCell(line.x1, options.scaleX))
    val r1 = clip.clampRow(toCell(line.y1, options.scaleY))
    val c2 = clip.clampCol(toCell(line.x2, options.scaleX))
    val r2 = clip.clampRow(toCell(line.y2, options.scaleY))

    if (r1 == r2) {
        val minCol = min(c1, c2)
        val maxCol = max(c1, c2)
        for (col in minCol..maxCol) {
            val flags = when {
                minCol == maxCol -> FLAG_LEFT or FLAG_RIGHT
                col == minCol -> FLAG_RIGHT
                col == maxCol -> FLAG_LEFT
                else -> FLAG_LEFT or FLAG_RIGHT
            }
            writeTag(clip, r1, col, flags, buffer)
        }
        return
    }

    if (c1 == c2) {
        val minRow = min(r1, r2)
        val maxRow = max(r1, r2)
        for (row in minRow..maxRow) {
            val flags = when {
                minRow == maxRow -> FLAG_UP or FLAG_DOWN
                row == minRow -> FLAG_DOWN
                row == maxRow -> FLAG_UP
                else -> FLAG_UP or FLAG_DOWN
            }
            writeTag(clip, row, c1, flags, buffer)
        }
        return
    }

    val deltaRow = abs(r2 - r1)
    val deltaCol = abs(c2 - c1)
    val stepRow = if (r1 < r2) 1 else -1
    val stepCol = if (c1 < c2) 1 else -1
    val diagonalFlags = if (deltaCol > deltaRow) FLAG_LEFT or FLAG_RIGHT else FLAG_UP or FLAG_DOWN

    var row = r1
    var col = c1
    var error = 0
    while (true) {
        writeTag(clip, row, col, diagonalFlags, buffer)
        if (row == r2 && col == c2) break
        val doubled = 2 * error
        if (doubled > -deltaRow) {
            error -= deltaRow
            col += stepCol
        }
        if (doubled < deltaCol) {
            error += deltaCol
            row += stepRow
        }
    }
}

// ---------------------------------------------------------------------------
// Glyph run
// ---------------------------------------------------------------------------

/**
 * A glyph with a non-finite position is skipped rather than passed to
 * [toCell] — unlike a malformed rect/line/clip, a single bad glyph
 * placement doesn't need to fail the whole render.
 */
private fun renderGlyphRun(
    options: AsciiOptions,
    clip: ClipBounds,
    run: PaintInstruction.PaintGlyphRun,
    buffer: MutableMap<Point, Cell>,
) {
    for (glyph: PaintGlyphPlacement in run.glyphs) {
        if (!isFinite(glyph.x) || !isFinite(glyph.y)) continue
        val row = toCell(glyph.y, options.scaleY)
        val col = toCell(glyph.x, options.scaleX)
        writeChar(clip, row, col, toSafeTerminalGlyph(glyph.glyphId), buffer)
    }
}

/**
 * ASCII-backend-specific relaxation of the general `PaintGlyphPlacement`
 * contract: `glyphId` is treated as a literal Unicode code point (no font
 * resolution happens in a terminal), per `P2D02-paint-vm-ascii.md`.
 * Control characters, bidi-control code points, and UTF-16 surrogate code
 * points are replaced with `?` so a crafted message can't inject terminal
 * escape sequences or ill-formed UTF-16. A code point requiring a
 * surrogate pair (above the Basic Multilingual Plane) is also replaced
 * with `?`, since a single Kotlin/JVM [Char] is one UTF-16 code unit and
 * cannot represent it.
 */
private fun toSafeTerminalGlyph(codePoint: Int): Char {
    if (codePoint in 0..0x10FFFF && isSafeTerminalCodePoint(codePoint) && Character.charCount(codePoint) == 1) {
        return codePoint.toChar()
    }
    return '?'
}

private fun isSafeTerminalCodePoint(codePoint: Int): Boolean {
    if (codePoint < 0x20) return false
    if (codePoint in 0x7f..0x9f) return false
    if (codePoint in 0xD800..0xDFFF) return false
    if (codePoint == 0x200e || codePoint == 0x200f || codePoint == 0x061c) return false
    if (codePoint in 0x202a..0x202e) return false
    return codePoint !in 0x2066..0x2069
}
