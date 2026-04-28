/// Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.
///
/// Encodes strings to Data Matrix symbols using the ECC200 variant
/// (the modern Reed-Solomon-protected form that has replaced the older
/// ECC000–ECC140 lineage worldwide).
///
/// ## Quick start
///
/// ```dart
/// import 'package:coding_adventures_data_matrix/data_matrix.dart';
///
/// // Auto-select smallest square symbol for "A" → 10×10
/// final grid = encode('A');
/// print(grid.rows);  // 10
///
/// // Force a specific size
/// final big = encode('Hello', size: 16);
/// print(big.rows);   // 16
///
/// // Try rectangular shapes too
/// final any = encode('HELLO', shape: SymbolShape.any);
/// ```
///
/// ## Where Data Matrix is used
///
/// - **PCBs**: every modern board carries an etched Data Matrix for
///   traceability through automated assembly lines.
/// - **Pharmaceuticals**: US FDA DSCSA mandates Data Matrix on unit-dose
///   packages.
/// - **Aerospace parts**: dot-peened marks survive decades of heat and
///   abrasion that would destroy ink-printed labels.
/// - **Medical devices**: GS1 DataMatrix on surgical instruments and implants.
/// - **USPS registered mail and customs forms**.
///
/// ## Key differences from QR Code
///
/// - GF(256) primitive polynomial is **0x12D** (not QR's 0x11D).
/// - Reed-Solomon **b=1 convention** (roots α¹…αⁿ) — not QR's b=0 (α⁰…αⁿ⁻¹).
/// - **L-shaped finder** (left column + bottom row all dark) plus a clock
///   border (top row + right column alternating).
/// - **Diagonal "Utah" placement** — no separate data zigzag, no masking step.
/// - **36 symbol sizes**: 24 square (10×10 … 144×144) + 6 rectangular.
///
/// ## Exported types
///
/// | Type                  | Description                                        |
/// |-----------------------|----------------------------------------------------|
/// | [encode]              | Main encoding function → `ModuleGrid`              |
/// | [encodeAndLayout]     | Encode + barcode-2d layout in one step             |
/// | [layoutGrid]          | `ModuleGrid` → `PaintScene` with DM defaults       |
/// | [SymbolShape]         | Square / rectangular / any shape preference        |
/// | [DataMatrixError]     | Base class for all encoder errors                  |
/// | [InputTooLongError]   | Input exceeds the largest symbol's capacity        |
/// | [InvalidSizeError]    | Caller-provided `size` is not a valid DM dimension |
/// | [dataMatrixVersion]   | Package version string                             |
library data_matrix;

export 'src/data_matrix.dart';
