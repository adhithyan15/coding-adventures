/// QR Code encoder — ISO/IEC 18004:2015 compliant.
///
/// This is the main entry point for the `coding_adventures_qr_code` package.
///
/// ## Quick start
///
/// ```dart
/// import 'package:coding_adventures_qr_code/coding_adventures_qr_code.dart';
///
/// // Encode a string into a QR Code module grid:
/// final grid = encode('HELLO WORLD', EccLevel.m);
/// print('Version 1: ${grid.rows}×${grid.cols}'); // 21×21
///
/// // Encode and produce pixel-resolved paint instructions:
/// final scene = encodeAndLayout('https://example.com', EccLevel.m);
/// ```
///
/// ## Exported symbols
///
/// | Symbol | Description |
/// |--------|-------------|
/// | [encode] | Main encoder: string → ModuleGrid |
/// | [encodeAndLayout] | Convenience: string → PaintScene |
/// | [EccLevel] | Error correction levels: l, m, q, h |
/// | [QRCodeError] | Base error class |
/// | [InputTooLongError] | Thrown when input exceeds v40 capacity |
/// | [QRLayoutError] | Thrown when layout config is invalid |
/// | [ModuleGrid] | 2D boolean grid (re-exported from barcode-2d) |
/// | [PaintScene] | Pixel-resolved paint instructions (re-exported) |
/// | [Barcode2DLayoutConfig] | Layout configuration (re-exported) |
/// | [defaultBarcode2DLayoutConfig] | Sensible layout defaults (re-exported) |
library coding_adventures_qr_code;

export 'src/qr_code.dart'
    show
        encode,
        encodeAndLayout,
        EccLevel,
        QRCodeError,
        InputTooLongError,
        QRLayoutError,
        version,
        // Re-exports from barcode-2d (pass-through for callers):
        ModuleGrid,
        ModuleShape,
        PaintScene,
        Barcode2DLayoutConfig,
        defaultBarcode2DLayoutConfig;
