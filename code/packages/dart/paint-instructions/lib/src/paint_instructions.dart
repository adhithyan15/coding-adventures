/// Backend-neutral paint scene model.
///
/// ## The Big Picture
///
/// Imagine you want to render a QR code. You have an abstract 21×21 grid of
/// dark/light modules, but you do NOT want to hard-code "draw SVG rectangles"
/// or "draw Metal quads" inside the QR encoder — that would tie the encoder to
/// one specific backend.
///
/// Instead you write a tiny intermediate language: a list of paint instructions
/// that say "fill this rectangle with this colour", "trace this polygon", etc.
/// Any backend — SVG, Canvas 2D, terminal ASCII, native GPU — can read that
/// list and render it in its own way.
///
/// That is exactly what this library provides.
///
/// ## Types at a Glance
///
/// ```
/// PathCommand       — one drawing verb: move_to, line_to, or close
/// PaintInstruction  — sealed base: everything you can paint
///   PaintRect       — axis-aligned filled rectangle  (x, y, width, height, fill)
///   PaintPath       — filled polygon  (list of PathCommands, fill)
/// PaintScene        — complete frame  (width, height, background, instructions, metadata)
/// PaintColorRGBA8   — RGBA colour bytes decoded from a CSS hex string
/// ```
///
/// ## Painter's Algorithm
///
/// Instructions are applied in order, just like a painter layering paint:
/// earlier instructions go behind later ones. The typical pattern is:
///
/// 1. First instruction: background `PaintRect` covering the whole scene.
/// 2. Remaining instructions: foreground shapes, painted on top.
///
/// ## Colour Strings
///
/// Fill colours are CSS hex strings:
///   `"#rgb"`, `"#rgba"`, `"#rrggbb"`, `"#rrggbbaa"`
///
/// Alpha defaults to `ff` (fully opaque) when omitted.
library paint_instructions;

// ============================================================================
// Version
// ============================================================================

/// Package version following Semantic Versioning 2.0.
const String version = '0.1.0';

// ============================================================================
// Metadata
// ============================================================================

/// Arbitrary key-value metadata attached to instructions or scenes.
///
/// Metadata carries optional hints that backends or debugging tools can use
/// (e.g. `{"row": "3", "col": "7", "role": "data"}`). It has no effect on
/// rendering logic.
typedef Metadata = Map<String, Object>;

// ============================================================================
// PathCommand
// ============================================================================

/// A single drawing verb within a [PaintPath].
///
/// A path is a sequence of [PathCommand]s that together trace a polygon:
///
/// ```
/// move_to (start of sub-path)
/// line_to (extend the outline)
/// line_to
/// ...
/// close   (return to the most recent move_to)
/// ```
///
/// ### Kinds
///
/// | kind       | meaning                                              |
/// |------------|------------------------------------------------------|
/// | `move_to`  | Lift the pen and place it at (`x`, `y`). Starts a   |
/// |            | new sub-path. No line is drawn.                     |
/// | `line_to`  | Draw a straight line from the current position to    |
/// |            | (`x`, `y`).                                          |
/// | `close`    | Draw a straight line back to the last `move_to`      |
/// |            | point, closing the current sub-path. The `x` and    |
/// |            | `y` fields are unused (zero by convention).          |
///
/// ### Example — a triangle
///
/// ```dart
/// final triangle = [
///   PathCommand.moveTo(0, 0),
///   PathCommand.lineTo(50, 100),
///   PathCommand.lineTo(100, 0),
///   PathCommand.close(),
/// ];
/// ```
sealed class PathCommand {
  /// The verb: `"move_to"`, `"line_to"`, or `"close"`.
  String get kind;

  /// The target x-coordinate in pixels.
  ///
  /// For [Close], this is always `0.0` and has no semantic meaning.
  double get x;

  /// The target y-coordinate in pixels.
  ///
  /// For [Close], this is always `0.0` and has no semantic meaning.
  double get y;

  // Private constructor prevents external subclassing while still
  // allowing the sealed subtypes below.
  const PathCommand._();

  /// Convenience constructor: lift the pen and move to (`x`, `y`).
  factory PathCommand.moveTo(double x, double y) = MoveTo;

  /// Convenience constructor: draw a line to (`x`, `y`).
  factory PathCommand.lineTo(double x, double y) = LineTo;

  /// Convenience constructor: close the current sub-path.
  factory PathCommand.close() = Close;
}

/// A `move_to` command: lifts the pen and places it at (`x`, `y`).
///
/// This starts a new sub-path. No visible line is drawn for the move itself.
/// After a `MoveTo`, subsequent `LineTo` commands extend the path from this
/// new origin.
final class MoveTo extends PathCommand {
  @override
  final String kind = 'move_to';

  @override
  final double x;

  @override
  final double y;

  const MoveTo(this.x, this.y) : super._();

  @override
  String toString() => 'MoveTo($x, $y)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MoveTo && other.x == x && other.y == y;

  @override
  int get hashCode => Object.hash(kind, x, y);
}

/// A `line_to` command: draws a straight line from the current pen position
/// to (`x`, `y`).
///
/// After this command the pen is at (`x`, `y`), ready for the next command.
final class LineTo extends PathCommand {
  @override
  final String kind = 'line_to';

  @override
  final double x;

  @override
  final double y;

  const LineTo(this.x, this.y) : super._();

  @override
  String toString() => 'LineTo($x, $y)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is LineTo && other.x == x && other.y == y;

  @override
  int get hashCode => Object.hash(kind, x, y);
}

/// A `close` command: draws a straight line back to the most recent `move_to`
/// point, closing the current sub-path.
///
/// After a `Close` the sub-path is complete and filled according to the
/// instruction's fill colour. The `x` and `y` fields are always `0.0` and
/// carry no meaning.
final class Close extends PathCommand {
  @override
  final String kind = 'close';

  @override
  final double x = 0.0;

  @override
  final double y = 0.0;

  const Close() : super._();

  @override
  String toString() => 'Close()';

  @override
  bool operator ==(Object other) => other is Close;

  @override
  int get hashCode => kind.hashCode;
}

// ============================================================================
// PaintInstruction (sealed)
// ============================================================================

/// Base sealed class for all renderable paint instructions.
///
/// There are exactly two subtypes:
///   - [PaintRect]  — an axis-aligned filled rectangle
///   - [PaintPath]  — a filled polygon traced by [PathCommand]s
///
/// Both carry a `fill` colour string and optional [Metadata].
///
/// Because this class is `sealed`, the Dart compiler will produce an
/// exhaustiveness warning if a `switch` on a [PaintInstruction] misses a
/// subtype — a handy safety net when adding future instruction kinds.
sealed class PaintInstruction {
  /// The CSS hex fill colour for this instruction.
  ///
  /// Examples: `"#000000"` (opaque black), `"#ffffff"` (opaque white),
  /// `"#ff000080"` (semi-transparent red).
  String get fill;

  /// Optional per-instruction metadata (e.g. barcode role annotations).
  Metadata get metadata;

  /// The kind string used by backends and serializers.
  ///
  /// `"rect"` for [PaintRect], `"path"` for [PaintPath].
  String get instructionKind;

  const PaintInstruction._();
}

// ============================================================================
// PaintRect
// ============================================================================

/// An axis-aligned filled rectangle.
///
/// This is the most common instruction. QR Code and Data Matrix renderers
/// emit one `PaintRect` per dark module.
///
/// ### Coordinate system
///
/// Coordinates are in pixels with the origin at the **top-left** corner of
/// the scene. X increases to the right; Y increases downward (screen coords).
///
/// ```
/// (x, y) ──────────────── x + width
///    │                         │
///    │         filled          │
///    │         rectangle       │
///    │                         │
/// y + height ──────────────────┘
/// ```
///
/// ### Example
///
/// ```dart
/// // A 10×10 black square at position (20, 30).
/// final rect = paintRect(x: 20, y: 30, width: 10, height: 10);
/// ```
final class PaintRect extends PaintInstruction {
  /// Left edge of the rectangle in pixels.
  final int x;

  /// Top edge of the rectangle in pixels.
  final int y;

  /// Width of the rectangle in pixels. Must be > 0.
  final int width;

  /// Height of the rectangle in pixels. Must be > 0.
  final int height;

  @override
  final String fill;

  @override
  final Metadata metadata;

  @override
  String get instructionKind => 'rect';

  const PaintRect({
    required this.x,
    required this.y,
    required this.width,
    required this.height,
    required this.fill,
    required this.metadata,
  }) : super._();

  @override
  String toString() =>
      'PaintRect(x=$x, y=$y, w=$width, h=$height, fill=$fill)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PaintRect &&
          other.x == x &&
          other.y == y &&
          other.width == width &&
          other.height == height &&
          other.fill == fill;

  @override
  int get hashCode => Object.hash(instructionKind, x, y, width, height, fill);
}

// ============================================================================
// PaintPath
// ============================================================================

/// A filled polygon described by a sequence of [PathCommand]s.
///
/// Used for non-rectangular shapes: hexagonal MaxiCode modules, custom
/// decorations, etc. The path is closed (the fill covers the enclosed area).
///
/// ### Typical path structure
///
/// ```
/// move_to  (first vertex)
/// line_to  (second vertex)
/// line_to  (third vertex)
/// ...
/// close    (back to first vertex, shape filled)
/// ```
///
/// ### Example — a triangle
///
/// ```dart
/// final triangle = paintPath(commands: [
///   PathCommand.moveTo(0, 0),
///   PathCommand.lineTo(50, 100),
///   PathCommand.lineTo(100, 0),
///   PathCommand.close(),
/// ]);
/// ```
final class PaintPath extends PaintInstruction {
  /// The drawing commands that trace the polygon outline.
  final List<PathCommand> commands;

  @override
  final String fill;

  @override
  final Metadata metadata;

  @override
  String get instructionKind => 'path';

  const PaintPath({
    required this.commands,
    required this.fill,
    required this.metadata,
  }) : super._();

  @override
  String toString() =>
      'PaintPath(commands=${commands.length}, fill=$fill)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PaintPath &&
          other.fill == fill &&
          _commandsEqual(other.commands, commands);

  bool _commandsEqual(List<PathCommand> a, List<PathCommand> b) {
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i] != b[i]) return false;
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(instructionKind, fill, Object.hashAll(commands));
}

// ============================================================================
// PaintScene
// ============================================================================

/// A complete renderable frame.
///
/// A `PaintScene` bundles everything a backend needs to produce one image:
///
/// - **Dimensions** (`width`, `height`) in pixels.
/// - **Background** colour: the canvas is pre-filled with this before any
///   instruction is applied.
/// - **Instructions**: a list of [PaintInstruction]s applied in order using
///   the painter's algorithm (index 0 is painted first, at the back).
/// - **Metadata**: optional key-value hints for debugging or serialization.
///
/// ### Painter's algorithm
///
/// ```
/// index 0   ← painted first (furthest back)
/// index 1
/// ...
/// index n   ← painted last (on top of everything)
/// ```
///
/// ### Typical layout
///
/// ```
/// instructions[0] = PaintRect(full canvas, fill=background)  // quiet zone
/// instructions[1] = PaintRect(module at row=0,col=0)         // dark module
/// instructions[2] = PaintRect(module at row=0,col=2)
/// ...
/// ```
///
/// ### Creating a scene
///
/// Use the [createScene] helper (which applies sane defaults) rather than
/// constructing [PaintScene] directly.
final class PaintScene {
  /// Total canvas width in pixels.
  final int width;

  /// Total canvas height in pixels.
  final int height;

  /// The CSS hex string used to pre-fill the canvas before painting.
  ///
  /// Defaults to `"#ffffff"` (opaque white).
  final String background;

  /// The ordered list of paint instructions (painter's algorithm order).
  final List<PaintInstruction> instructions;

  /// Optional scene-level metadata (title, source format, encoding options…).
  final Metadata metadata;

  const PaintScene({
    required this.width,
    required this.height,
    required this.background,
    required this.instructions,
    required this.metadata,
  });

  @override
  String toString() =>
      'PaintScene(${width}x$height, bg=$background, '
      '${instructions.length} instructions)';
}

// ============================================================================
// RGBA8 colour
// ============================================================================

/// A parsed RGBA colour with one byte per channel.
///
/// Extracted from a CSS hex string by [parseColorRGBA8].
///
/// | Field | Range  | Meaning                       |
/// |-------|--------|-------------------------------|
/// | r     | 0–255  | Red channel                   |
/// | g     | 0–255  | Green channel                 |
/// | b     | 0–255  | Blue channel                  |
/// | a     | 0–255  | Alpha (255 = fully opaque)    |
///
/// ### Examples
///
/// ```dart
/// parseColorRGBA8('#ff0000');   // red:   PaintColorRGBA8(r:255, g:0,   b:0,   a:255)
/// parseColorRGBA8('#0000ff80'); // blue:  PaintColorRGBA8(r:0,   g:0,   b:255, a:128)
/// parseColorRGBA8('#fff');      // white: PaintColorRGBA8(r:255, g:255, b:255, a:255)
/// ```
final class PaintColorRGBA8 {
  /// Red channel (0–255).
  final int r;

  /// Green channel (0–255).
  final int g;

  /// Blue channel (0–255).
  final int b;

  /// Alpha channel (0 = fully transparent, 255 = fully opaque).
  final int a;

  const PaintColorRGBA8({
    required this.r,
    required this.g,
    required this.b,
    required this.a,
  });

  @override
  String toString() => 'PaintColorRGBA8(r=$r, g=$g, b=$b, a=$a)';

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PaintColorRGBA8 &&
          other.r == r &&
          other.g == g &&
          other.b == b &&
          other.a == a;

  @override
  int get hashCode => Object.hash(r, g, b, a);
}

// ============================================================================
// Helper factory functions
// ============================================================================

/// Create a [PaintRect] instruction.
///
/// Parameters default to sensible values:
///   - `fill` defaults to `"#000000"` (opaque black)
///   - `metadata` defaults to an empty map
///
/// ```dart
/// final r = paintRect(x: 0, y: 0, width: 100, height: 100);
/// ```
PaintRect paintRect({
  required int x,
  required int y,
  required int width,
  required int height,
  String fill = '#000000',
  Metadata? metadata,
}) {
  return PaintRect(
    x: x,
    y: y,
    width: width,
    height: height,
    fill: fill.isEmpty ? '#000000' : fill,
    metadata: metadata ?? {},
  );
}

/// Create a [PaintPath] instruction.
///
/// Parameters default to sensible values:
///   - `fill` defaults to `"#000000"` (opaque black)
///   - `metadata` defaults to an empty map
///
/// ```dart
/// final hex = paintPath(commands: [
///   PathCommand.moveTo(10, 0),
///   PathCommand.lineTo(20, 17),
///   PathCommand.lineTo(10, 34),
///   PathCommand.lineTo(0, 17),
///   PathCommand.close(),
/// ]);
/// ```
PaintPath paintPath({
  required List<PathCommand> commands,
  String fill = '#000000',
  Metadata? metadata,
}) {
  return PaintPath(
    commands: commands,
    fill: fill.isEmpty ? '#000000' : fill,
    metadata: metadata ?? {},
  );
}

/// Create a [PaintScene].
///
/// Parameters default to sensible values:
///   - `background` defaults to `"#ffffff"` (opaque white)
///   - `metadata` defaults to an empty map
///
/// ```dart
/// final scene = createScene(
///   width: 210,
///   height: 210,
///   instructions: [paintRect(x: 0, y: 0, width: 10, height: 10)],
/// );
/// ```
PaintScene createScene({
  required int width,
  required int height,
  required List<PaintInstruction> instructions,
  String background = '#ffffff',
  Metadata? metadata,
}) {
  return PaintScene(
    width: width,
    height: height,
    background: background.isEmpty ? '#ffffff' : background,
    instructions: instructions,
    metadata: metadata ?? {},
  );
}

// ============================================================================
// Color parsing
// ============================================================================

/// Parse a CSS hex colour string into a [PaintColorRGBA8].
///
/// Accepted formats (case-insensitive):
///
/// | Format      | Example        | Expansion                 |
/// |-------------|----------------|---------------------------|
/// | `#rgb`      | `#f00`         | `#ff0000ff`               |
/// | `#rgba`     | `#f00f`        | `#ff0000ff`               |
/// | `#rrggbb`   | `#ff0000`      | `#ff0000ff`               |
/// | `#rrggbbaa` | `#ff000080`    | as-is                     |
///
/// Throws [FormatException] if the string does not start with `#`,
/// has an unsupported length, or contains non-hex digits.
///
/// ### Short-form expansion
///
/// `#rgb` expands each digit by doubling it: `#f80` → `#ff8800ff`.
/// This matches the CSS3 Color Module specification.
///
/// ```dart
/// final c = parseColorRGBA8('#ff0000');
/// print(c.r); // 255
/// print(c.g); // 0
/// print(c.b); // 0
/// print(c.a); // 255
/// ```
PaintColorRGBA8 parseColorRGBA8(String value) {
  final trimmed = value.trim();

  if (!trimmed.startsWith('#')) {
    throw FormatException(
      'Paint colour must start with #, got: "$value"',
    );
  }

  // Strip the leading '#'.
  var hex = trimmed.substring(1).toLowerCase();

  // Expand short forms to 8-character RRGGBBAA.
  switch (hex.length) {
    case 3:
      // #rgb → #rrggbbff
      hex = '${hex[0]}${hex[0]}${hex[1]}${hex[1]}${hex[2]}${hex[2]}ff';
    case 4:
      // #rgba → #rrggbbaa
      hex = '${hex[0]}${hex[0]}${hex[1]}${hex[1]}${hex[2]}${hex[2]}${hex[3]}${hex[3]}';
    case 6:
      // #rrggbb → #rrggbbff (fully opaque)
      hex = '${hex}ff';
    case 8:
      // already canonical
      break;
    default:
      throw FormatException(
        'Paint colour must be #rgb, #rgba, #rrggbb, or #rrggbbaa, got: "$value"',
      );
  }

  // Parse each channel as a two-hex-digit unsigned byte.
  int parseChannel(int offset) {
    final slice = hex.substring(offset, offset + 2);
    final v = int.tryParse(slice, radix: 16);
    if (v == null) {
      throw FormatException(
        'Paint colour contains invalid hex digits in "$value"',
      );
    }
    return v;
  }

  return PaintColorRGBA8(
    r: parseChannel(0),
    g: parseChannel(2),
    b: parseChannel(4),
    a: parseChannel(6),
  );
}
