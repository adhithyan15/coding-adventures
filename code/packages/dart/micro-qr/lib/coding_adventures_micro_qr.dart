/// Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.
///
/// Encodes strings to Micro QR Code symbols (M1–M4, 11×11 to 17×17 modules).
/// Supports numeric, alphanumeric, and byte encoding modes with automatic
/// symbol-version and ECC-level selection.
///
/// ## Quick start
///
/// ```dart
/// import 'package:coding_adventures_micro_qr/coding_adventures_micro_qr.dart';
///
/// // Auto-select smallest symbol for "HELLO" → M2-L (13×13)
/// final grid = encode('HELLO');
/// print(grid.rows);  // 13
///
/// // Force M4-L (17×17) with low ECC for maximum capacity
/// final big = encode('https://example.com', version: MicroQRVersion.m4, ecc: MicroQREccLevel.l);
/// print(big.rows);   // 17
///
/// // Encode + lay out in one call
/// final scene = encodeAndLayout('12345');
/// ```
///
/// ## Exported types
///
/// | Type              | Description                                   |
/// |-------------------|-----------------------------------------------|
/// | [encode]          | Main encoding function → [ModuleGrid]         |
/// | [encodeAt]        | Encode to specific version+ECC                |
/// | [layoutGrid]      | [ModuleGrid] → [PaintScene]                   |
/// | [encodeAndLayout] | Encode + layout in one step                   |
/// | [MicroQRVersion]  | Symbol version enum (m1, m2, m3, m4)         |
/// | [MicroQREccLevel] | ECC level enum (detection, l, m, q)           |
/// | [MicroQRError]    | Base class for all encoder errors             |
/// | [InputTooLong]    | Input exceeds maximum capacity                |
/// | [UnsupportedMode] | Mode not available for the chosen symbol      |
/// | [ECCNotAvailable] | ECC level not available for the chosen symbol |
/// | [microQrVersion]  | Package version string                        |
library coding_adventures_micro_qr;

export 'src/micro_qr.dart';
