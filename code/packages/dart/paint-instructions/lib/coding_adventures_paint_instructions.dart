/// Backend-neutral paint scene model.
///
/// This library defines the tiny intermediate representation that sits between
/// abstract data (barcode grids, vector graphics) and concrete pixel backends
/// (SVG, Canvas, Metal, Direct2D, ASCII art, …).
///
/// The model has deliberately few types:
///
///   - [PathCommand] — a single drawing verb (move, line, close)
///   - [PaintInstruction] — a sealed supertype for one rendered shape:
///       * [PaintRect]  — an axis-aligned filled rectangle
///       * [PaintPath]  — a filled polygon described by [PathCommand]s
///   - [PaintScene]  — a complete frame: dimensions, background, and a list
///                     of instructions that are painted in order (painter's
///                     algorithm, first instruction at the back)
///   - [PaintColorRGBA8] — a parsed RGBA color extracted from a hex string
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';
///
/// void main() {
///   final scene = createScene(
///     width: 100,
///     height: 100,
///     instructions: [
///       paintRect(x: 10, y: 10, width: 80, height: 80),
///     ],
///   );
///   print(scene.instructions.length); // 1
/// }
/// ```
library coding_adventures_paint_instructions;

export 'src/paint_instructions.dart';
