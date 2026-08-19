/// A small, pure terminal backend for [PaintScene] values.
///
/// Implements the full `P2D02-paint-vm-ascii.md` contract: filled/stroked
/// rectangles, lines, glyph runs, and plain (untransformed, unfiltered,
/// fully opaque) groups/clips/layers.
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_paint_instructions/coding_adventures_paint_instructions.dart';
/// import 'package:coding_adventures_paint_vm_ascii/coding_adventures_paint_vm_ascii.dart';
///
/// void main() {
///   final scene = createScene(
///     width: 80,
///     height: 16,
///     instructions: [
///       paintRect(x: 0, y: 0, width: 80, height: 16, fill: '', stroke: '#000000'),
///     ],
///   );
///   final result = renderDefault(scene);
///   switch (result) {
///     case PaintVmAsciiOk(:final text):
///       print(text);
///     case PaintVmAsciiErr(:final error):
///       print('render failed: $error');
///   }
/// }
/// ```
library coding_adventures_paint_vm_ascii;

export 'src/paint_vm_ascii.dart';
