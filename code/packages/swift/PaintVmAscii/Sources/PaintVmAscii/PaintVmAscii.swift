import Foundation
import PaintInstructions

// A small, pure terminal backend for `PaintScene` values.
//
// Implements the full `P2D02-paint-vm-ascii.md` contract: filled/stroked
// rectangles, lines, glyph runs, and plain (untransformed, unfiltered,
// fully opaque) groups/clips/layers. Scene coordinates are divided by a
// configurable horizontal and vertical scale to obtain character-cell
// coordinates.
//
// The buffer is a `Dictionary` from a `Point` (row, col) to a `Cell`,
// rather than a mutable 2D array — scenes rendered by this backend are
// small (terminal-sized, capped by `maxAxisCells`), so the simplicity of a
// sparse map outweighs any performance concern, and it keeps the
// box-drawing merge logic (two strokes sharing a corner combine into one
// character) expressible without a pre-sized grid.
//
// Spec: P2D02 paint-vm-ascii.

/// Package version, shared with the other language implementations.
public let version = "0.1.0"

// MARK: - Options

/// How scene coordinates map to terminal character cells.
public struct AsciiOptions: Equatable, Sendable {
    public let scaleX: Int
    public let scaleY: Int

    public init(scaleX: Int, scaleY: Int) {
        self.scaleX = scaleX
        self.scaleY = scaleY
    }

    /// The cross-language default: cells eight scene units wide, sixteen tall.
    public static let defaultOptions = AsciiOptions(scaleX: 8, scaleY: 16)
}

// MARK: - Errors

/// Errors this backend can report without throwing to a partial rendering
/// — every error carries enough context to explain exactly what about the
/// scene was rejected.
public enum PaintVmAsciiError: Error, Equatable, Sendable {
    /// `AsciiOptions.scaleX` was not a positive integer.
    case invalidScaleX(Int)
    /// `AsciiOptions.scaleY` was not a positive integer.
    case invalidScaleY(Int)
    /// The scene's width or height was negative.
    case invalidSceneDimensions(width: Int, height: Int)
    /// The scene's cell-grid size (width/scaleX by height/scaleY) exceeds
    /// the bound this backend is willing to materialize. Checked both
    /// per-axis and by total cell count — a product-only check can be
    /// bypassed by a zero-width, huge-height (or vice versa) scene.
    case sceneTooLarge(width: Int, height: Int)
    /// A `PaintRectInstruction`'s width or height was negative.
    case invalidRectangleGeometry(x: Int, y: Int, width: Int, height: Int)
    /// A `PaintLineInstruction`'s coordinates included a NaN or infinite value.
    case invalidLineGeometry(x1: Double, y1: Double, x2: Double, y2: Double)
    /// A `PaintClipInstruction`'s coordinates were non-finite, either
    /// directly or via the `x+width`/`y+height` extent (two individually
    /// finite values can sum to infinity).
    case invalidClipGeometry(x: Double, y: Double, width: Double, height: Double)
    /// A `PaintGroupInstruction`/`PaintLayerInstruction` used a feature
    /// this text-mode backend cannot represent (non-identity transform,
    /// non-default opacity, filters, non-normal blend mode).
    case unsupportedInstruction(reason: String)
    /// A `group`/`clip`/`layer` nested more than `maxNestingDepth` levels deep.
    case sceneTooDeep(depth: Int)
}

// MARK: - Buffer

private let flagUp = 1
private let flagRight = 2
private let flagDown = 4
private let flagLeft = 8
private let flagFill = 16

private let boxCharacters: [Int: String] = [
    (flagLeft | flagRight): "\u{2500}",
    (flagUp | flagDown): "\u{2502}",
    (flagDown | flagRight): "\u{250C}",
    (flagDown | flagLeft): "\u{2510}",
    (flagUp | flagRight): "\u{2514}",
    (flagUp | flagLeft): "\u{2518}",
    (flagLeft | flagRight | flagDown): "\u{252C}",
    (flagLeft | flagRight | flagUp): "\u{2534}",
    (flagUp | flagDown | flagRight): "\u{251C}",
    (flagUp | flagDown | flagLeft): "\u{2524}",
    (flagUp | flagDown | flagLeft | flagRight): "\u{253C}",
    flagRight: "\u{2500}",
    flagLeft: "\u{2500}",
    flagUp: "\u{2502}",
    flagDown: "\u{2502}",
]

private let fillChar = "\u{2588}"

/// One character cell. `.text` always wins over `.tag` — literal text is
/// never overwritten.
private enum Cell {
    case tag(Int)
    case text(String)
}

private struct Point: Hashable {
    let row: Int
    let col: Int
}

private struct ClipBounds {
    let minCol: Int
    let minRow: Int
    let maxCol: Int
    let maxRow: Int

    func inside(row: Int, col: Int) -> Bool {
        row >= minRow && row < maxRow && col >= minCol && col < maxCol
    }

    /// Clamp a cell coordinate into this clip's bounds. Used before
    /// building any range that iterates between two cell coordinates (rect
    /// fill/stroke, line endpoints), so a caller-supplied geometry with a
    /// huge (but valid) extent can't force iteration/recursion far beyond
    /// the actual clipped surface — bounded by the clip's own size instead
    /// of by caller input.
    func clampCol(_ value: Int) -> Int { max(minCol, min(value, maxCol - 1)) }
    func clampRow(_ value: Int) -> Int { max(minRow, min(value, maxRow - 1)) }

    func intersect(_ child: ClipBounds) -> ClipBounds {
        ClipBounds(
            minCol: max(minCol, child.minCol),
            minRow: max(minRow, child.minRow),
            maxCol: min(maxCol, child.maxCol),
            maxRow: min(maxRow, child.maxRow)
        )
    }
}

private func writeTag(_ clip: ClipBounds, _ row: Int, _ col: Int, _ flags: Int, _ buffer: inout [Point: Cell]) {
    guard clip.inside(row: row, col: col) else { return }
    let p = Point(row: row, col: col)
    switch buffer[p] {
    case .text:
        return
    case .tag(let existing):
        buffer[p] = .tag(existing | flags)
    case nil:
        buffer[p] = .tag(flags)
    }
}

private func writeChar(_ clip: ClipBounds, _ row: Int, _ col: Int, _ text: String, _ buffer: inout [Point: Cell]) {
    guard clip.inside(row: row, col: col) else { return }
    buffer[Point(row: row, col: col)] = .text(text)
}

private func resolveCell(_ cell: Cell) -> String {
    switch cell {
    case .text(let text):
        return text
    case .tag(let flags):
        let directions = flags & (flagUp | flagRight | flagDown | flagLeft)
        if directions != 0, let boxChar = boxCharacters[directions] {
            return boxChar
        }
        if (flags & flagFill) != 0 {
            return fillChar
        }
        return "+"
    }
}

private func bufferToText(_ rows: Int, _ columns: Int, _ buffer: [Point: Cell]) -> String {
    var lines: [String] = []
    lines.reserveCapacity(rows)
    for row in 0..<rows {
        var line = ""
        for col in 0..<columns {
            if let cell = buffer[Point(row: row, col: col)] {
                line += resolveCell(cell)
            } else {
                line += " "
            }
        }
        while line.hasSuffix(" ") {
            line.removeLast()
        }
        lines.append(line)
    }
    var lastNonBlank = lines.count
    while lastNonBlank > 0 && lines[lastNonBlank - 1].isEmpty {
        lastNonBlank -= 1
    }
    return lines[0..<lastNonBlank].joined(separator: "\n")
}

// MARK: - Coordinate conversion

/// Cell-coordinate values are saturated to this bound (rather than left as
/// a raw rounded result) so a large-but-ordinary finite `Double` can never
/// land on an extreme `Int` value. Without this, a clip extent rounding to
/// an extreme value could defeat `ClipBounds.clampCol`/`clampRow`
/// downstream via integer overflow in the `maxCol - 1` they compute,
/// un-clamping any shape nested in that clip and reopening the
/// unbounded-iteration DoS the clip clamping exists to prevent. A billion
/// cells in either direction is far beyond any real rendered scene (scenes
/// are additionally capped at `maxAxisCells` per axis) while leaving
/// enormous headroom below 64-bit `Int`'s actual bounds for
/// `clampCol`/`clampRow`'s arithmetic to stay overflow-free.
private let cellBound = 1_000_000_000

private func toCell(_ coordinate: Double, _ scale: Int) -> Int {
    let scaled = coordinate / Double(scale)
    if scaled.isNaN { return 0 }
    if scaled >= Double(cellBound) { return cellBound }
    if scaled <= -Double(cellBound) { return -cellBound }
    return Int(scaled.rounded())
}

// MARK: - Validation

private func isFinite(_ value: Double) -> Bool { !value.isNaN && !value.isInfinite }

private func validRectangle(_ r: PaintRectInstruction) -> Bool { r.width >= 0 && r.height >= 0 }

private func validLine(_ l: PaintLineInstruction) -> Bool {
    isFinite(l.x1) && isFinite(l.y1) && isFinite(l.x2) && isFinite(l.y2)
}

/// Validates the individual fields *and* the `x+width`/`y+height` extents
/// used by `clipBoundsOf` — two individually-finite values near
/// `Double.greatestFiniteMagnitude` can still sum to `+Infinity` under
/// IEEE-754 arithmetic, so checking the fields alone isn't sufficient to
/// guarantee `toCell` never sees a non-finite input.
private func validClip(_ c: PaintClipInstruction) -> Bool {
    isFinite(c.x) && isFinite(c.y) && isFinite(c.width) && isFinite(c.height)
        && c.width >= 0 && c.height >= 0
        && isFinite(c.x + c.width) && isFinite(c.y + c.height)
}

private func isIdentityTransform(_ transform: Transform2D?) -> Bool {
    transform == nil || transform!.isIdentity
}

private func assertPlainGroup(_ group: PaintGroupInstruction) -> PaintVmAsciiError? {
    if !isIdentityTransform(group.transform) {
        return .unsupportedInstruction(reason: "group with a non-identity transform")
    }
    if let opacity = group.opacity, opacity != 1.0 {
        return .unsupportedInstruction(reason: "group with non-default opacity")
    }
    return nil
}

private func assertPlainLayer(_ layer: PaintLayerInstruction) -> PaintVmAsciiError? {
    if !isIdentityTransform(layer.transform) {
        return .unsupportedInstruction(reason: "layer with a non-identity transform")
    }
    if let opacity = layer.opacity, opacity != 1.0 {
        return .unsupportedInstruction(reason: "layer with non-default opacity")
    }
    if layer.hasFilters {
        return .unsupportedInstruction(reason: "layer with filters")
    }
    if let blendMode = layer.blendMode, blendMode != "normal" {
        return .unsupportedInstruction(reason: "layer with a non-normal blend mode")
    }
    return nil
}

private func visiblePaint(_ paint: String) -> Bool {
    let trimmed = paint.trimmingCharacters(in: .whitespaces)
    return !trimmed.isEmpty && trimmed != "transparent" && trimmed != "none"
}

// MARK: - Top-level render

/// Upper bound on the number of character cells a rendered scene may
/// occupy, both in total and per axis. Scene dimensions are otherwise only
/// checked for being non-negative, so without this a caller-supplied
/// width/height of e.g. one billion would force `bufferToText` to iterate
/// on an enormous number of cells even with zero drawing instructions — a
/// denial-of-service unrelated to (and not fixed by) the per-instruction
/// clip clamping. The per-axis bound is required in addition to the
/// product bound: a zero-width, huge-height scene has a product of zero
/// (passing a product-only check) while still forcing an unbounded
/// traversal along the surviving axis. 2000x2000 (a generous terminal-sized
/// canvas) is cheap to fully materialize either way.
private let maxAxisCells = 2000
private let maxBufferCells = maxAxisCells * maxAxisCells

/// Upper bound on how deeply `group`/`clip`/`layer` children may nest.
/// `dispatch` recurses one call frame per nesting level with no other
/// bound on depth, so a scene built from deeply nested wrapper
/// instructions (each with a single child) could otherwise exhaust the
/// call stack. 64 levels is far beyond any real scene (this package's own
/// scenes are always flat) while stopping a pathological scene long before
/// it threatens the stack.
private let maxNestingDepth = 64

/// Render with `AsciiOptions.defaultOptions`.
public func renderDefault(_ scene: PaintScene) throws -> String {
    try render(scene, AsciiOptions.defaultOptions)
}

/// Render a scene as terminal-friendly text.
public func render(_ scene: PaintScene, _ options: AsciiOptions) throws -> String {
    if options.scaleX <= 0 { throw PaintVmAsciiError.invalidScaleX(options.scaleX) }
    if options.scaleY <= 0 { throw PaintVmAsciiError.invalidScaleY(options.scaleY) }
    if scene.width < 0 || scene.height < 0 {
        throw PaintVmAsciiError.invalidSceneDimensions(width: scene.width, height: scene.height)
    }

    let columns = ceilDiv(scene.width, options.scaleX)
    let rows = ceilDiv(scene.height, options.scaleY)
    if columns > maxAxisCells || rows > maxAxisCells || columns * rows > maxBufferCells {
        throw PaintVmAsciiError.sceneTooLarge(width: scene.width, height: scene.height)
    }

    let clip = ClipBounds(minCol: 0, minRow: 0, maxCol: columns, maxRow: rows)
    var buffer: [Point: Cell] = [:]
    for instruction in scene.instructions {
        try dispatch(options, clip, &buffer, instruction, 0)
    }
    return bufferToText(rows, columns, buffer)
}

/// Computes `ceil(numerator / denominator)` for `numerator >= 0`,
/// `denominator > 0` (both already validated by `render` before this is
/// called). Deliberately NOT the usual `(numerator + denominator - 1) /
/// denominator` formula: unlike Dart/Kotlin/Java's wrapping 64-bit
/// arithmetic, Swift's `Int` `+` TRAPS on overflow, and `scene.width`/
/// `scene.height` have no upper bound check before reaching here — a
/// caller-supplied width near `Int.max` would crash the process instead of
/// being caught by the `sceneTooLarge` check a few lines later. Rewritten
/// as `(numerator - 1) / denominator + 1` (for `numerator > 0`), which
/// cannot overflow for any valid non-negative `Int` input.
private func ceilDiv(_ numerator: Int, _ denominator: Int) -> Int {
    guard numerator > 0 else { return 0 }
    return (numerator - 1) / denominator + 1
}

/// Render one instruction (recursing into group/clip/layer children),
/// mutating `buffer` in place and failing loudly on anything not in the
/// P2D02 contract. `depth` is the current nesting depth (0 for a top-level
/// scene instruction), checked against `maxNestingDepth` before recursing
/// into any container's children.
private func dispatch(
    _ options: AsciiOptions,
    _ clip: ClipBounds,
    _ buffer: inout [Point: Cell],
    _ instruction: PaintInstruction,
    _ depth: Int
) throws {
    switch instruction {
    case .rect(let r):
        guard validRectangle(r) else {
            throw PaintVmAsciiError.invalidRectangleGeometry(x: r.x, y: r.y, width: r.width, height: r.height)
        }
        renderRectangle(options, clip, r, &buffer)
    case .line(let l):
        guard validLine(l) else {
            throw PaintVmAsciiError.invalidLineGeometry(x1: l.x1, y1: l.y1, x2: l.x2, y2: l.y2)
        }
        renderLine(options, clip, l, &buffer)
    case .glyphRun(let run):
        renderGlyphRun(options, clip, run, &buffer)
    case .group(let group):
        if let error = assertPlainGroup(group) { throw error }
        try dispatchChildren(options, clip, &buffer, group.children, depth)
    case .clip(let c):
        guard validClip(c) else {
            throw PaintVmAsciiError.invalidClipGeometry(x: c.x, y: c.y, width: c.width, height: c.height)
        }
        let nextClip = clip.intersect(clipBoundsOf(options, c))
        try dispatchChildren(options, nextClip, &buffer, c.children, depth)
    case .layer(let layer):
        if let error = assertPlainLayer(layer) { throw error }
        try dispatchChildren(options, clip, &buffer, layer.children, depth)
    }
}

private func dispatchChildren(
    _ options: AsciiOptions,
    _ clip: ClipBounds,
    _ buffer: inout [Point: Cell],
    _ children: [PaintInstruction],
    _ depth: Int
) throws {
    let nextDepth = depth + 1
    if nextDepth > maxNestingDepth { throw PaintVmAsciiError.sceneTooDeep(depth: nextDepth) }
    for child in children {
        try dispatch(options, clip, &buffer, child, nextDepth)
    }
}

private func clipBoundsOf(_ options: AsciiOptions, _ c: PaintClipInstruction) -> ClipBounds {
    ClipBounds(
        minCol: toCell(c.x, options.scaleX),
        minRow: toCell(c.y, options.scaleY),
        maxCol: toCell(c.x + c.width, options.scaleX),
        maxRow: toCell(c.y + c.height, options.scaleY)
    )
}

// MARK: - Rect

private func renderRectangle(_ options: AsciiOptions, _ clip: ClipBounds, _ r: PaintRectInstruction, _ buffer: inout [Point: Cell]) {
    // Summed in Double, not Int: unlike Dart/Kotlin/Java's 64-bit
    // arithmetic (which wraps silently), Swift's `Int` `+` TRAPS on
    // overflow — `r.x + r.width` on an adversarial Int.max-ish input would
    // crash the whole process instead of producing a catchable error.
    // Double addition of two in-range Int64 values can never itself
    // overflow to infinity (its max representable magnitude is far below
    // Double's), so this sidesteps the trap entirely while still letting
    // `toCell`'s saturation handle the (now merely large, not infinite)
    // result.
    let c1 = clip.clampCol(toCell(Double(r.x), options.scaleX))
    let r1 = clip.clampRow(toCell(Double(r.y), options.scaleY))
    let c2 = clip.clampCol(toCell(Double(r.x) + Double(r.width), options.scaleX))
    let r2 = clip.clampRow(toCell(Double(r.y) + Double(r.height), options.scaleY))

    if visiblePaint(r.fill) {
        if r1 <= r2 && c1 <= c2 {
            for row in r1...r2 {
                for col in c1...c2 {
                    writeTag(clip, row, col, flagFill, &buffer)
                }
            }
        }
    }

    if !r.stroke.trimmingCharacters(in: .whitespaces).isEmpty {
        writeTag(clip, r1, c1, flagDown | flagRight, &buffer)
        writeTag(clip, r1, c2, flagDown | flagLeft, &buffer)
        writeTag(clip, r2, c1, flagUp | flagRight, &buffer)
        writeTag(clip, r2, c2, flagUp | flagLeft, &buffer)
        if c1 + 1 < c2 {
            for col in (c1 + 1)..<c2 {
                writeTag(clip, r1, col, flagLeft | flagRight, &buffer)
                writeTag(clip, r2, col, flagLeft | flagRight, &buffer)
            }
        }
        if r1 + 1 < r2 {
            for row in (r1 + 1)..<r2 {
                writeTag(clip, row, c1, flagUp | flagDown, &buffer)
                writeTag(clip, row, c2, flagUp | flagDown, &buffer)
            }
        }
    }
}

// MARK: - Line (horizontal/vertical fast paths + Bresenham for the diagonal case)

private func renderLine(_ options: AsciiOptions, _ clip: ClipBounds, _ line: PaintLineInstruction, _ buffer: inout [Point: Cell]) {
    // Clamped into the clip's own bounds before use — an out-of-range but
    // otherwise valid (finite) endpoint can't force iteration or Bresenham
    // recursion far beyond the actual clipped surface.
    let c1 = clip.clampCol(toCell(line.x1, options.scaleX))
    let r1 = clip.clampRow(toCell(line.y1, options.scaleY))
    let c2 = clip.clampCol(toCell(line.x2, options.scaleX))
    let r2 = clip.clampRow(toCell(line.y2, options.scaleY))

    if r1 == r2 {
        let minCol = min(c1, c2)
        let maxCol = max(c1, c2)
        for col in minCol...maxCol {
            let flags: Int
            if minCol == maxCol {
                flags = flagLeft | flagRight
            } else if col == minCol {
                flags = flagRight
            } else if col == maxCol {
                flags = flagLeft
            } else {
                flags = flagLeft | flagRight
            }
            writeTag(clip, r1, col, flags, &buffer)
        }
        return
    }

    if c1 == c2 {
        let minRow = min(r1, r2)
        let maxRow = max(r1, r2)
        for row in minRow...maxRow {
            let flags: Int
            if minRow == maxRow {
                flags = flagUp | flagDown
            } else if row == minRow {
                flags = flagDown
            } else if row == maxRow {
                flags = flagUp
            } else {
                flags = flagUp | flagDown
            }
            writeTag(clip, row, c1, flags, &buffer)
        }
        return
    }

    let deltaRow = abs(r2 - r1)
    let deltaCol = abs(c2 - c1)
    let stepRow = r1 < r2 ? 1 : -1
    let stepCol = c1 < c2 ? 1 : -1
    let diagonalFlags = deltaCol > deltaRow ? (flagLeft | flagRight) : (flagUp | flagDown)

    // The error term is seeded to deltaCol - deltaRow (the standard
    // Bresenham initialization), not 0 — starting from 0 lets `row`
    // overshoot `r2` for some slopes (e.g. deltaRow=1, deltaCol=3) without
    // the loop's break condition (row == r2 && col == c2) ever becoming
    // true again, hanging forever. This exact bug (error seeded to 0) was
    // found in this package's Haskell/Java/Kotlin siblings — see
    // https://github.com/adhithyan15/coding-adventures/issues/12093.
    var row = r1
    var col = c1
    var error = deltaCol - deltaRow
    while true {
        writeTag(clip, row, col, diagonalFlags, &buffer)
        if row == r2 && col == c2 { break }
        let doubled = 2 * error
        if doubled > -deltaRow {
            error -= deltaRow
            col += stepCol
        }
        if doubled < deltaCol {
            error += deltaCol
            row += stepRow
        }
    }
}

// MARK: - Glyph run

/// A glyph with a non-finite position is skipped rather than passed to
/// `toCell` — unlike a malformed rect/line/clip, a single bad glyph
/// placement doesn't need to fail the whole render.
private func renderGlyphRun(_ options: AsciiOptions, _ clip: ClipBounds, _ run: PaintGlyphRunInstruction, _ buffer: inout [Point: Cell]) {
    for glyph in run.glyphs {
        guard isFinite(glyph.x), isFinite(glyph.y) else { continue }
        let row = toCell(glyph.y, options.scaleY)
        let col = toCell(glyph.x, options.scaleX)
        writeChar(clip, row, col, toSafeTerminalGlyph(glyph.glyphId), &buffer)
    }
}

/// ASCII-backend-specific relaxation of the general `PaintGlyphPlacement`
/// contract: `glyphId` is treated as a literal Unicode code point (no font
/// resolution happens in a terminal), per `P2D02-paint-vm-ascii.md`.
/// Control characters, bidi-control code points, and UTF-16 surrogate code
/// points are replaced with `?` so a crafted message can't inject terminal
/// escape sequences or ill-formed UTF-16. Swift's `String` naturally
/// represents any valid Unicode scalar value (including supplementary-plane
/// code points) directly — no BMP-only limitation like the JVM ports.
private func toSafeTerminalGlyph(_ codePoint: Int) -> String {
    if codePoint >= 0, codePoint <= 0x10FFFF, isSafeTerminalCodePoint(codePoint),
       let scalar = Unicode.Scalar(codePoint) {
        return String(Character(scalar))
    }
    return "?"
}

private func isSafeTerminalCodePoint(_ codePoint: Int) -> Bool {
    if codePoint < 0x20 { return false }
    if codePoint >= 0x7f && codePoint <= 0x9f { return false }
    if codePoint >= 0xD800 && codePoint <= 0xDFFF { return false }
    if codePoint == 0x200e || codePoint == 0x200f || codePoint == 0x061c { return false }
    if codePoint >= 0x202a && codePoint <= 0x202e { return false }
    return !(codePoint >= 0x2066 && codePoint <= 0x2069)
}
