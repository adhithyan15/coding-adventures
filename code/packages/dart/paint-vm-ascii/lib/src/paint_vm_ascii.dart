/// A small, pure terminal backend for [PaintScene] values.
///
/// Implements the full `P2D02-paint-vm-ascii.md` contract: filled/stroked
/// rectangles, lines, glyph runs, and plain (untransformed, unfiltered,
/// fully opaque) groups/clips/layers. Scene coordinates are divided by a
/// configurable horizontal and vertical scale to obtain character-cell
/// coordinates.
///
/// The buffer is a [Map] from `(row, col)` record to a [_Cell], rather than
/// a mutable 2D array — scenes rendered by this backend are small
/// (terminal-sized, capped by [_maxAxisCells]), so the simplicity of a
/// sparse map outweighs any performance concern, and it keeps the
/// box-drawing merge logic (two strokes sharing a corner combine into one
/// character) expressible without a pre-sized grid.
///
/// Spec: P2D02 paint-vm-ascii.
library paint_vm_ascii;

import 'dart:math' as math;

import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';

/// Package version, shared with the other language implementations.
const String version = '0.1.0';

// ============================================================================
// Options
// ============================================================================

/// How scene coordinates map to terminal character cells.
final class AsciiOptions {
  final int scaleX;
  final int scaleY;

  const AsciiOptions({required this.scaleX, required this.scaleY});

  /// The cross-language default: cells eight scene units wide, sixteen tall.
  static const defaultOptions = AsciiOptions(scaleX: 8, scaleY: 16);

  @override
  String toString() => 'AsciiOptions(scaleX=$scaleX, scaleY=$scaleY)';
}

// ============================================================================
// Errors and result
// ============================================================================

/// Errors this backend can report without throwing or returning a partial
/// rendering.
sealed class PaintVmAsciiError {
  const PaintVmAsciiError._();
}

/// [AsciiOptions.scaleX] was not a positive integer.
final class InvalidScaleX extends PaintVmAsciiError {
  final int scaleX;
  const InvalidScaleX(this.scaleX) : super._();
  @override
  String toString() => 'InvalidScaleX(scaleX=$scaleX)';
}

/// [AsciiOptions.scaleY] was not a positive integer.
final class InvalidScaleY extends PaintVmAsciiError {
  final int scaleY;
  const InvalidScaleY(this.scaleY) : super._();
  @override
  String toString() => 'InvalidScaleY(scaleY=$scaleY)';
}

/// The scene's width or height was negative.
final class InvalidSceneDimensions extends PaintVmAsciiError {
  final int width;
  final int height;
  const InvalidSceneDimensions(this.width, this.height) : super._();
  @override
  String toString() => 'InvalidSceneDimensions(width=$width, height=$height)';
}

/// The scene's cell-grid size (width/scaleX by height/scaleY) exceeds the
/// bound this backend is willing to materialize. Checked both per-axis and
/// by total cell count — a product-only check can be bypassed by a
/// zero-width, huge-height (or vice versa) scene.
final class SceneTooLarge extends PaintVmAsciiError {
  final int width;
  final int height;
  const SceneTooLarge(this.width, this.height) : super._();
  @override
  String toString() => 'SceneTooLarge(width=$width, height=$height)';
}

/// A [PaintRect]'s width or height was negative.
final class InvalidRectangleGeometry extends PaintVmAsciiError {
  final int x;
  final int y;
  final int width;
  final int height;
  const InvalidRectangleGeometry(this.x, this.y, this.width, this.height)
      : super._();
  @override
  String toString() =>
      'InvalidRectangleGeometry(x=$x, y=$y, width=$width, height=$height)';
}

/// A [PaintLine]'s coordinates included a NaN or infinite value.
final class InvalidLineGeometry extends PaintVmAsciiError {
  final double x1;
  final double y1;
  final double x2;
  final double y2;
  const InvalidLineGeometry(this.x1, this.y1, this.x2, this.y2) : super._();
  @override
  String toString() =>
      'InvalidLineGeometry(x1=$x1, y1=$y1, x2=$x2, y2=$y2)';
}

/// A [PaintClip]'s coordinates were non-finite, either directly or via the
/// `x+width`/`y+height` extent (two individually finite values can sum to
/// infinity).
final class InvalidClipGeometry extends PaintVmAsciiError {
  final double x;
  final double y;
  final double width;
  final double height;
  const InvalidClipGeometry(this.x, this.y, this.width, this.height)
      : super._();
  @override
  String toString() =>
      'InvalidClipGeometry(x=$x, y=$y, width=$width, height=$height)';
}

/// A [PaintGroup] or [PaintLayer] used a feature this text-mode backend
/// cannot represent (non-identity transform, non-default opacity, filters,
/// non-normal blend mode), or the instruction is a [PaintPath] (this
/// backend renders no vector geometry).
final class UnsupportedInstruction extends PaintVmAsciiError {
  final String reason;
  const UnsupportedInstruction(this.reason) : super._();
  @override
  String toString() => 'UnsupportedInstruction(reason=$reason)';
}

/// A [PaintGroup], [PaintClip], or [PaintLayer] nested more than
/// [_maxNestingDepth] levels deep.
final class SceneTooDeep extends PaintVmAsciiError {
  final int depth;
  const SceneTooDeep(this.depth) : super._();
  @override
  String toString() => 'SceneTooDeep(depth=$depth)';
}

/// The outcome of [render]: either rendered text, or an error.
sealed class PaintVmAsciiResult {
  const PaintVmAsciiResult._();
}

final class PaintVmAsciiOk extends PaintVmAsciiResult {
  final String text;
  const PaintVmAsciiOk(this.text) : super._();
  @override
  String toString() => 'PaintVmAsciiOk(text.length=${text.length})';
}

final class PaintVmAsciiErr extends PaintVmAsciiResult {
  final PaintVmAsciiError error;
  const PaintVmAsciiErr(this.error) : super._();
  @override
  String toString() => 'PaintVmAsciiErr(error=$error)';
}

// ============================================================================
// Buffer
// ============================================================================

const int _flagUp = 1;
const int _flagRight = 2;
const int _flagDown = 4;
const int _flagLeft = 8;
const int _flagFill = 16;

const Map<int, String> _boxCharacters = {
  (_flagLeft | _flagRight): '─',
  (_flagUp | _flagDown): '│',
  (_flagDown | _flagRight): '┌',
  (_flagDown | _flagLeft): '┐',
  (_flagUp | _flagRight): '└',
  (_flagUp | _flagLeft): '┘',
  (_flagLeft | _flagRight | _flagDown): '┬',
  (_flagLeft | _flagRight | _flagUp): '┴',
  (_flagUp | _flagDown | _flagRight): '├',
  (_flagUp | _flagDown | _flagLeft): '┤',
  (_flagUp | _flagDown | _flagLeft | _flagRight): '┼',
  _flagRight: '─',
  _flagLeft: '─',
  _flagUp: '│',
  _flagDown: '│',
};

const String _fillChar = '█';

/// One character cell. [_CellText] always wins over [_CellTag] — literal
/// text is never overwritten.
sealed class _Cell {}

final class _CellTag extends _Cell {
  final int flags;
  _CellTag(this.flags);
}

final class _CellText extends _Cell {
  final String text;
  _CellText(this.text);
}

/// A cell coordinate within the render buffer. Dart records give this
/// structural equality/hashCode for free, so it doubles as a [Map] key.
typedef _Point = (int row, int col);

final class _ClipBounds {
  final int minCol;
  final int minRow;
  final int maxCol;
  final int maxRow;

  const _ClipBounds(this.minCol, this.minRow, this.maxCol, this.maxRow);

  bool inside(int row, int col) =>
      row >= minRow && row < maxRow && col >= minCol && col < maxCol;

  /// Clamp a cell coordinate into this clip's bounds. Used before building
  /// any range that iterates between two cell coordinates (rect
  /// fill/stroke, line endpoints), so a caller-supplied geometry with a
  /// huge (but valid) extent can't force iteration/recursion far beyond the
  /// actual clipped surface — bounded by the clip's own size instead of by
  /// caller input.
  int clampCol(int value) => math.max(minCol, math.min(value, maxCol - 1));

  int clampRow(int value) => math.max(minRow, math.min(value, maxRow - 1));

  _ClipBounds intersect(_ClipBounds child) => _ClipBounds(
        math.max(minCol, child.minCol),
        math.max(minRow, child.minRow),
        math.min(maxCol, child.maxCol),
        math.min(maxRow, child.maxRow),
      );
}

void _writeTag(_ClipBounds clip, int row, int col, int flags,
    Map<_Point, _Cell> buffer) {
  if (!clip.inside(row, col)) return;
  final p = (row, col);
  final existing = buffer[p];
  if (existing is _CellText) return;
  if (existing is _CellTag) {
    buffer[p] = _CellTag(existing.flags | flags);
  } else {
    buffer[p] = _CellTag(flags);
  }
}

void _writeChar(
    _ClipBounds clip, int row, int col, String text, Map<_Point, _Cell> buffer) {
  if (!clip.inside(row, col)) return;
  buffer[(row, col)] = _CellText(text);
}

String _resolveCell(_Cell cell) {
  switch (cell) {
    case _CellText(:final text):
      return text;
    case _CellTag(:final flags):
      final directions = flags & (_flagUp | _flagRight | _flagDown | _flagLeft);
      final boxChar = directions != 0 ? _boxCharacters[directions] : null;
      if (boxChar != null) return boxChar;
      if ((flags & _flagFill) != 0) return _fillChar;
      return '+';
  }
}

String _bufferToText(int rows, int columns, Map<_Point, _Cell> buffer) {
  final lines = List<String>.generate(rows, (row) {
    final sb = StringBuffer();
    for (var col = 0; col < columns; col++) {
      final cell = buffer[(row, col)];
      sb.write(cell == null ? ' ' : _resolveCell(cell));
    }
    return sb.toString().replaceAll(RegExp(r' +$'), '');
  });
  var lastNonBlank = lines.length;
  while (lastNonBlank > 0 && lines[lastNonBlank - 1].isEmpty) {
    lastNonBlank -= 1;
  }
  return lines.sublist(0, lastNonBlank).join('\n');
}

// ============================================================================
// Coordinate conversion
// ============================================================================

/// Cell-coordinate values are saturated to this bound (rather than left as
/// a raw rounded result) so a large-but-ordinary finite [double] can never
/// land on an extreme [int] value. Without this, a clip extent rounding to
/// an extreme value could defeat [_ClipBounds.clampCol]/[_ClipBounds.clampRow]
/// downstream via integer overflow in the `maxCol - 1` they compute,
/// un-clamping any shape nested in that clip and reopening the
/// unbounded-iteration DoS the clip clamping exists to prevent. A billion
/// cells in either direction is far beyond any real rendered scene (scenes
/// are additionally capped at [_maxAxisCells] per axis) while leaving
/// enormous headroom below 64-bit `int`'s actual bounds for
/// `clampCol`/`clampRow`'s arithmetic to stay overflow-free.
const int _cellBound = 1000000000;

int _toCell(double coordinate, int scale) {
  final scaled = coordinate / scale;
  if (scaled.isNaN) return 0;
  if (scaled >= _cellBound) return _cellBound;
  if (scaled <= -_cellBound) return -_cellBound;
  return scaled.round();
}

// ============================================================================
// Validation
// ============================================================================

bool _isFinite(double value) => !value.isNaN && !value.isInfinite;

bool _validRectangle(PaintRect r) => r.width >= 0 && r.height >= 0;

bool _validLine(PaintLine l) =>
    _isFinite(l.x1) && _isFinite(l.y1) && _isFinite(l.x2) && _isFinite(l.y2);

/// Validates the individual fields *and* the `x+width`/`y+height` extents
/// used by [_clipBoundsOf] — two individually-finite values near
/// [double.maxFinite] can still sum to +Infinity under IEEE-754 arithmetic,
/// so checking the fields alone isn't sufficient to guarantee [_toCell]
/// never sees a non-finite input.
bool _validClip(PaintClip c) =>
    _isFinite(c.x) &&
    _isFinite(c.y) &&
    _isFinite(c.width) &&
    _isFinite(c.height) &&
    c.width >= 0 &&
    c.height >= 0 &&
    _isFinite(c.x + c.width) &&
    _isFinite(c.y + c.height);

bool _isIdentityTransform(Transform2D? transform) =>
    transform == null || transform.isIdentity;

PaintVmAsciiError? _assertPlainGroup(PaintGroup group) {
  if (!_isIdentityTransform(group.transform)) {
    return const UnsupportedInstruction('group with a non-identity transform');
  }
  if (group.opacity != null && group.opacity != 1.0) {
    return const UnsupportedInstruction('group with non-default opacity');
  }
  return null;
}

PaintVmAsciiError? _assertPlainLayer(PaintLayer layer) {
  if (!_isIdentityTransform(layer.transform)) {
    return const UnsupportedInstruction('layer with a non-identity transform');
  }
  if (layer.opacity != null && layer.opacity != 1.0) {
    return const UnsupportedInstruction('layer with non-default opacity');
  }
  if (layer.hasFilters) {
    return const UnsupportedInstruction('layer with filters');
  }
  if (layer.blendMode != null && layer.blendMode != 'normal') {
    return const UnsupportedInstruction('layer with a non-normal blend mode');
  }
  return null;
}

bool _visiblePaint(String paint) {
  final trimmed = paint.trim();
  return trimmed.isNotEmpty && trimmed != 'transparent' && trimmed != 'none';
}

// ============================================================================
// Top-level render
// ============================================================================

/// Upper bound on the number of character cells a rendered scene may
/// occupy, both in total and per axis. Scene dimensions are otherwise only
/// checked for being non-negative, so without this a caller-supplied
/// width/height of e.g. one billion would force [_bufferToText] to iterate
/// on an enormous number of cells even with zero drawing instructions — a
/// denial-of-service unrelated to (and not fixed by) the per-instruction
/// clip clamping. The per-axis bound is required in addition to the
/// product bound: a zero-width, huge-height scene has a product of zero
/// (passing a product-only check) while still forcing an unbounded
/// traversal along the surviving axis. 2000x2000 (a generous terminal-sized
/// canvas) is cheap to fully materialize either way.
const int _maxAxisCells = 2000;
const int _maxBufferCells = _maxAxisCells * _maxAxisCells;

/// Upper bound on how deeply [PaintGroup]/[PaintClip]/[PaintLayer] children
/// may nest. [_dispatch] recurses one call frame per nesting level with no
/// other bound on depth, so a scene built from deeply nested wrapper
/// instructions (each with a single child) could otherwise exhaust the call
/// stack — a `StackOverflowError` that, unlike every other error this
/// backend reports, can't be caught and returned as a normal
/// [PaintVmAsciiErr]. 64 levels is far beyond any real scene (this
/// package's own scenes are always flat: one `glyph_run`/`rect`/`line` per
/// instruction, no nesting) while stopping a pathological scene long before
/// it threatens the stack.
const int _maxNestingDepth = 64;

/// Render with [AsciiOptions.defaultOptions].
PaintVmAsciiResult renderDefault(PaintScene scene) =>
    render(scene, AsciiOptions.defaultOptions);

/// Render a scene as terminal-friendly text.
PaintVmAsciiResult render(PaintScene scene, AsciiOptions options) {
  if (options.scaleX <= 0) {
    return PaintVmAsciiErr(InvalidScaleX(options.scaleX));
  }
  if (options.scaleY <= 0) {
    return PaintVmAsciiErr(InvalidScaleY(options.scaleY));
  }
  if (scene.width < 0 || scene.height < 0) {
    return PaintVmAsciiErr(InvalidSceneDimensions(scene.width, scene.height));
  }

  final columns = _ceilDiv(scene.width, options.scaleX);
  final rows = _ceilDiv(scene.height, options.scaleY);
  if (columns > _maxAxisCells ||
      rows > _maxAxisCells ||
      columns * rows > _maxBufferCells) {
    return PaintVmAsciiErr(SceneTooLarge(scene.width, scene.height));
  }

  final clip = _ClipBounds(0, 0, columns, rows);
  final buffer = <_Point, _Cell>{};
  for (final instruction in scene.instructions) {
    final error = _dispatch(options, clip, buffer, instruction, 0);
    if (error != null) return PaintVmAsciiErr(error);
  }
  return PaintVmAsciiOk(_bufferToText(rows, columns, buffer));
}

int _ceilDiv(int numerator, int denominator) =>
    (numerator + denominator - 1) ~/ denominator;

/// Render one instruction (recursing into group/clip/layer children),
/// mutating [buffer] in place and failing loudly on anything not in the
/// P2D02 contract. Returns `null` on success, the error otherwise. [depth]
/// is the current nesting depth (0 for a top-level scene instruction),
/// checked against [_maxNestingDepth] before recursing into any container's
/// children.
PaintVmAsciiError? _dispatch(AsciiOptions options, _ClipBounds clip,
    Map<_Point, _Cell> buffer, PaintInstruction instruction, int depth) {
  switch (instruction) {
    case PaintRect():
      if (!_validRectangle(instruction)) {
        return InvalidRectangleGeometry(
            instruction.x, instruction.y, instruction.width, instruction.height);
      }
      _renderRectangle(options, clip, instruction, buffer);
      return null;
    case PaintLine():
      if (!_validLine(instruction)) {
        return InvalidLineGeometry(
            instruction.x1, instruction.y1, instruction.x2, instruction.y2);
      }
      _renderLine(options, clip, instruction, buffer);
      return null;
    case PaintGlyphRun():
      _renderGlyphRun(options, clip, instruction, buffer);
      return null;
    case PaintGroup():
      final plainCheck = _assertPlainGroup(instruction);
      if (plainCheck != null) return plainCheck;
      return _dispatchChildren(options, clip, buffer, instruction.children, depth);
    case PaintClip():
      if (!_validClip(instruction)) {
        return InvalidClipGeometry(
            instruction.x, instruction.y, instruction.width, instruction.height);
      }
      final nextClip = clip.intersect(_clipBoundsOf(options, instruction));
      return _dispatchChildren(options, nextClip, buffer, instruction.children, depth);
    case PaintLayer():
      final plainCheck = _assertPlainLayer(instruction);
      if (plainCheck != null) return plainCheck;
      return _dispatchChildren(options, clip, buffer, instruction.children, depth);
    case PaintPath():
      return const UnsupportedInstruction('path');
  }
}

PaintVmAsciiError? _dispatchChildren(AsciiOptions options, _ClipBounds clip,
    Map<_Point, _Cell> buffer, List<PaintInstruction> children, int depth) {
  final nextDepth = depth + 1;
  if (nextDepth > _maxNestingDepth) return SceneTooDeep(nextDepth);
  for (final child in children) {
    final error = _dispatch(options, clip, buffer, child, nextDepth);
    if (error != null) return error;
  }
  return null;
}

_ClipBounds _clipBoundsOf(AsciiOptions options, PaintClip c) => _ClipBounds(
      _toCell(c.x, options.scaleX),
      _toCell(c.y, options.scaleY),
      _toCell(c.x + c.width, options.scaleX),
      _toCell(c.y + c.height, options.scaleY),
    );

// ============================================================================
// Rect
// ============================================================================

void _renderRectangle(AsciiOptions options, _ClipBounds clip, PaintRect r,
    Map<_Point, _Cell> buffer) {
  final c1 = clip.clampCol(_toCell(r.x.toDouble(), options.scaleX));
  final r1 = clip.clampRow(_toCell(r.y.toDouble(), options.scaleY));
  final c2 = clip.clampCol(_toCell((r.x + r.width).toDouble(), options.scaleX));
  final r2 = clip.clampRow(_toCell((r.y + r.height).toDouble(), options.scaleY));

  if (_visiblePaint(r.fill)) {
    for (var row = r1; row <= r2; row++) {
      for (var col = c1; col <= c2; col++) {
        _writeTag(clip, row, col, _flagFill, buffer);
      }
    }
  }

  if (r.stroke.trim().isNotEmpty) {
    _writeTag(clip, r1, c1, _flagDown | _flagRight, buffer);
    _writeTag(clip, r1, c2, _flagDown | _flagLeft, buffer);
    _writeTag(clip, r2, c1, _flagUp | _flagRight, buffer);
    _writeTag(clip, r2, c2, _flagUp | _flagLeft, buffer);
    for (var col = c1 + 1; col < c2; col++) {
      _writeTag(clip, r1, col, _flagLeft | _flagRight, buffer);
      _writeTag(clip, r2, col, _flagLeft | _flagRight, buffer);
    }
    for (var row = r1 + 1; row < r2; row++) {
      _writeTag(clip, row, c1, _flagUp | _flagDown, buffer);
      _writeTag(clip, row, c2, _flagUp | _flagDown, buffer);
    }
  }
}

// ============================================================================
// Line (horizontal/vertical fast paths + Bresenham for the diagonal case)
// ============================================================================

void _renderLine(AsciiOptions options, _ClipBounds clip, PaintLine line,
    Map<_Point, _Cell> buffer) {
  // Clamped into the clip's own bounds before use — an out-of-range but
  // otherwise valid (finite) endpoint can't force iteration or Bresenham
  // recursion far beyond the actual clipped surface.
  final c1 = clip.clampCol(_toCell(line.x1, options.scaleX));
  final r1 = clip.clampRow(_toCell(line.y1, options.scaleY));
  final c2 = clip.clampCol(_toCell(line.x2, options.scaleX));
  final r2 = clip.clampRow(_toCell(line.y2, options.scaleY));

  if (r1 == r2) {
    final minCol = math.min(c1, c2);
    final maxCol = math.max(c1, c2);
    for (var col = minCol; col <= maxCol; col++) {
      final flags = minCol == maxCol
          ? (_flagLeft | _flagRight)
          : col == minCol
              ? _flagRight
              : col == maxCol
                  ? _flagLeft
                  : (_flagLeft | _flagRight);
      _writeTag(clip, r1, col, flags, buffer);
    }
    return;
  }

  if (c1 == c2) {
    final minRow = math.min(r1, r2);
    final maxRow = math.max(r1, r2);
    for (var row = minRow; row <= maxRow; row++) {
      final flags = minRow == maxRow
          ? (_flagUp | _flagDown)
          : row == minRow
              ? _flagDown
              : row == maxRow
                  ? _flagUp
                  : (_flagUp | _flagDown);
      _writeTag(clip, row, c1, flags, buffer);
    }
    return;
  }

  final deltaRow = (r2 - r1).abs();
  final deltaCol = (c2 - c1).abs();
  final stepRow = r1 < r2 ? 1 : -1;
  final stepCol = c1 < c2 ? 1 : -1;
  final diagonalFlags =
      deltaCol > deltaRow ? (_flagLeft | _flagRight) : (_flagUp | _flagDown);

  // The error term is seeded to deltaCol - deltaRow (the standard
  // Bresenham initialization), not 0 — starting from 0 lets `row` overshoot
  // `r2` for some slopes (e.g. deltaRow=1, deltaCol=3) without the loop's
  // break condition (row == r2 && col == c2) ever becoming true again,
  // hanging forever. Verified against several (deltaRow, deltaCol) ratios
  // before relying on it.
  var row = r1;
  var col = c1;
  var error = deltaCol - deltaRow;
  while (true) {
    _writeTag(clip, row, col, diagonalFlags, buffer);
    if (row == r2 && col == c2) break;
    final doubled = 2 * error;
    if (doubled > -deltaRow) {
      error -= deltaRow;
      col += stepCol;
    }
    if (doubled < deltaCol) {
      error += deltaCol;
      row += stepRow;
    }
  }
}

// ============================================================================
// Glyph run
// ============================================================================

/// A glyph with a non-finite position is skipped rather than passed to
/// [_toCell] — unlike a malformed rect/line/clip, a single bad glyph
/// placement doesn't need to fail the whole render.
void _renderGlyphRun(AsciiOptions options, _ClipBounds clip, PaintGlyphRun run,
    Map<_Point, _Cell> buffer) {
  for (final glyph in run.glyphs) {
    if (!_isFinite(glyph.x) || !_isFinite(glyph.y)) continue;
    final row = _toCell(glyph.y, options.scaleY);
    final col = _toCell(glyph.x, options.scaleX);
    _writeChar(clip, row, col, _toSafeTerminalGlyph(glyph.glyphId), buffer);
  }
}

/// ASCII-backend-specific relaxation of the general `PaintGlyphPlacement`
/// contract: `glyphId` is treated as a literal Unicode code point (no font
/// resolution happens in a terminal), per `P2D02-paint-vm-ascii.md`.
/// Control characters, bidi-control code points, and UTF-16 surrogate code
/// points are replaced with `?` so a crafted message can't inject terminal
/// escape sequences or ill-formed UTF-16. Unlike the JVM ports (Java,
/// Kotlin), Dart's [String] is not limited to a single UTF-16 code unit —
/// [String.fromCharCode] builds the correct surrogate pair for a
/// supplementary-plane code point — so this backend accepts the full valid
/// Unicode scalar-value range rather than only the Basic Multilingual
/// Plane.
String _toSafeTerminalGlyph(int codePoint) {
  if (codePoint >= 0 &&
      codePoint <= 0x10FFFF &&
      _isSafeTerminalCodePoint(codePoint)) {
    return String.fromCharCode(codePoint);
  }
  return '?';
}

bool _isSafeTerminalCodePoint(int codePoint) {
  if (codePoint < 0x20) return false;
  if (codePoint >= 0x7f && codePoint <= 0x9f) return false;
  if (codePoint >= 0xD800 && codePoint <= 0xDFFF) return false;
  if (codePoint == 0x200e || codePoint == 0x200f || codePoint == 0x061c) {
    return false;
  }
  if (codePoint >= 0x202a && codePoint <= 0x202e) return false;
  return !(codePoint >= 0x2066 && codePoint <= 0x2069);
}
