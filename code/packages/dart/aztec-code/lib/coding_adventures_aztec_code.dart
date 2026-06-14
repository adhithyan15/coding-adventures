/// Aztec Code 2D barcode encoder — ISO/IEC 24778:2008 compliant.
///
/// Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
/// published as a patent-free 2D matrix barcode. Unlike QR Code (which has
/// three square finder patterns at three corners), Aztec Code uses a single
/// **bullseye finder pattern at the centre** of the symbol. The scanner
/// detects the bullseye first, then reads outward in a spiral — no large
/// quiet zone is required.
///
/// ## Where Aztec Code is used today
///
/// - **IATA boarding passes** — the barcode on every airline boarding pass.
/// - **Eurostar and Amtrak rail tickets** — printed and on-screen tickets.
/// - **PostNL, Deutsche Post, La Poste** — European postal routing.
/// - **US military ID cards.**
///
/// ## Symbol variants
///
/// | Variant | Layers | Sizes (modules)        |
/// |---------|--------|------------------------|
/// | Compact | 1 – 4  | 15×15 to 27×27         |
/// | Full    | 1 – 32 | 19×19 to 143×143       |
///
/// `compactSize = 11 + 4 × layers`,
/// `fullSize    = 15 + 4 × layers`.
///
/// ## Encoding pipeline (v0.1.0 — byte-mode only)
///
/// ```
/// input string / bytes
///   → Binary-Shift bits from Upper mode
///   → smallest symbol that fits at minEccPercent
///   → pad to exact data-codeword count
///   → GF(256)/0x12D Reed-Solomon ECC (b=1, roots α^1..α^n)
///   → bit stuffing (insert complement after 4 consecutive identical bits)
///   → GF(16) mode message (layers + dataCwCount + 5 or 6 RS nibbles)
///   → ModuleGrid  (bullseye → orientation marks → mode msg → data spiral)
/// ```
///
/// ## Quick start
///
/// ```dart
/// import 'package:coding_adventures_aztec_code/coding_adventures_aztec_code.dart';
///
/// final grid = encode('IATA BP DATA');
/// print(grid.rows);  // e.g. 19 modules (compact-2)
/// print(grid.cols);  // 19
///
/// // Encode + layout in one step → PaintScene.
/// final scene = encodeAndLayout('Hello, World!');
/// ```
///
/// ## Exported API
///
/// | Symbol               | Description                                  |
/// |----------------------|----------------------------------------------|
/// | [encode]             | Encode bytes/string → [ModuleGrid]           |
/// | [encodeAndLayout]    | Encode + layout → `PaintScene`               |
/// | [explain]            | Encode → `AnnotatedModuleGrid` (stub v0.1.0) |
/// | [AztecOptions]       | Encoding options (`minEccPercent`)           |
/// | [AztecError]         | Base error class                             |
/// | [InputTooLongError]  | Input exceeds 32-layer full symbol capacity  |
/// | [aztecCodeVersion]   | Package version string                       |
library coding_adventures_aztec_code;

export 'src/aztec_code.dart';
