/// Shared 2D barcode abstraction layer.
///
/// This library provides the two building blocks every 2D barcode format needs:
///
///   1. [ModuleGrid] — the universal intermediate representation produced by
///      every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
///      It is a 2D boolean grid: `true` = dark module, `false` = light module.
///
///   2. [layout] — the single function that converts abstract module
///      coordinates into pixel-level [PaintScene] instructions ready for a
///      paint backend to render.
///
/// ## Pipeline position
///
/// ```
/// Input data
///   → format encoder (qr-code, data-matrix, aztec…)
///   → ModuleGrid          ← produced by the encoder
///   → layout()            ← THIS PACKAGE converts to pixels
///   → PaintScene          ← consumed by paint-vm backend
///   → output (SVG, Canvas, terminal…)
/// ```
///
/// ## Usage
///
/// ```dart
/// import 'package:coding_adventures_barcode_2d/coding_adventures_barcode_2d.dart';
///
/// // Build a 3×3 grid and set the centre module dark.
/// var grid = makeModuleGrid(rows: 3, cols: 3);
/// grid = setModule(grid, row: 1, col: 1, dark: true);
///
/// // Convert to a PaintScene with default settings.
/// final scene = layout(grid);
/// print(scene.width);  // (3 + 2*4) * 10 = 110
/// ```
library coding_adventures_barcode_2d;

export 'src/barcode_2d.dart';
