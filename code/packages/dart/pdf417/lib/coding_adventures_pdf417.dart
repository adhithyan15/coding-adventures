/// PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.
///
/// PDF417 (Portable Data File 417) is a stacked linear barcode invented by
/// Ynjiun P. Wang at Symbol Technologies in 1991. It encodes arbitrary bytes
/// into a rectangular grid of dark/light modules using:
///
/// - **Byte compaction** (codeword 924 latch): 6 bytes → 5 base-900 codewords,
///   achieving 1.2 bytes/codeword for bulk data.
/// - **GF(929) Reed-Solomon ECC**: error correction over the prime field GF(929)
///   using the b=3 convention with generator element α=3.
/// - **Three codeword clusters**: each row uses one of three distinct bar/space
///   pattern tables, cycling 0→1→2→0… so that any three consecutive rows
///   contain all row-metadata.
///
/// ## Quick start
///
/// ```dart
/// import 'package:coding_adventures_pdf417/coding_adventures_pdf417.dart';
///
/// // Encode a string (UTF-8 bytes) → ModuleGrid
/// final grid = encodeString('HELLO WORLD');
/// print(grid.rows);  // e.g. 9 rows × 3 cols = 27 module rows
/// print(grid.cols);  // 69 + 17×3 = 120 module columns
///
/// // Encode raw bytes with explicit ECC level and column count
/// final grid2 = encode(
///   [0x41, 0x42, 0x43],
///   options: const Pdf417Options(eccLevel: 2, columns: 3),
/// );
///
/// // Encode + layout in one step → PaintScene
/// final scene = encodeAndLayout('IATA BP'.codeUnits);
/// ```
///
/// ## Exported types
///
/// | Type                  | Description                                    |
/// |-----------------------|------------------------------------------------|
/// | [encode]              | Encode `List<int>` bytes → [ModuleGrid]        |
/// | [encodeString]        | Encode a UTF-8 string → [ModuleGrid]           |
/// | [encodeAndLayout]     | Encode + layout → [PaintScene]                 |
/// | [Pdf417Options]       | Encoding options (eccLevel, columns, rowHeight) |
/// | [Pdf417Error]         | Base error class                               |
/// | [InputTooLongError]   | Input exceeds symbol capacity                  |
/// | [InvalidDimensionsError] | columns out of range                        |
/// | [InvalidEccLevelError]   | ECC level outside 0–8                       |
/// | [computeLri]          | Internal: left row indicator (exported for tests)|
/// | [computeRri]          | Internal: right row indicator (exported for tests)|
/// | [pdf417Version]       | Package version string                         |
library coding_adventures_pdf417;

export 'src/pdf417.dart';
