// MicroQR.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Micro QR Code Encoder
// ============================================================================
//
// Micro QR Code is a compact variant of QR Code standardised in
// ISO/IEC 18004:2015 Annex E. It is designed for applications where even the
// smallest standard QR Code (21×21, version 1) is too large — surface-mount
// electronic component labels, circuit board markings, miniature industrial
// tags.
//
// ## Key differences from regular QR Code
//
//   - Single finder pattern (top-left only), not three.
//   - Timing patterns run along ROW 0 and COLUMN 0, not row 6 / col 6.
//   - Only 4 mask patterns instead of 8.
//   - Format XOR mask is 0x4445 (not 0x5412).
//   - Only ONE copy of format info (regular QR has two).
//   - 2-module quiet zone (regular QR needs 4).
//   - Narrower mode indicators: 0–3 bits (regular QR always uses 4).
//   - Single ECC block — no interleaving.
//
// ## Symbol sizes
//
//   M1: 11×11    M2: 13×13    M3: 15×15    M4: 17×17
//   formula: size = 2 × version_number + 9
//
// ## Encoding pipeline
//
//   input string
//     → auto-select smallest symbol (M1..M4) and mode
//     → build bit stream (mode indicator + char count + data + terminator + pad)
//     → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
//     → initialize grid (finder, L-shaped separator, timing at row 0/col 0)
//     → zigzag data placement (two-column snake from bottom-right)
//     → evaluate 4 mask patterns, pick lowest penalty
//     → write format information (15 bits, single copy, XOR 0x4445)
//     → ModuleGrid
//
// ============================================================================

import Barcode2D
import GF256
import PaintInstructions

// ============================================================================
// MARK: - Version
// ============================================================================

/// Current package version.
public let version = "0.1.0"

// ============================================================================
// MARK: - MicroQRVersion
// ============================================================================

/// Micro QR Code symbol designator.
///
/// Each step up in version adds 2 rows and 2 columns:
///
/// ```
/// M1 → 11×11   (version_number = 1, size = 2×1 + 9 = 11)
/// M2 → 13×13   (version_number = 2, size = 2×2 + 9 = 13)
/// M3 → 15×15   (version_number = 3, size = 2×3 + 9 = 15)
/// M4 → 17×17   (version_number = 4, size = 2×4 + 9 = 17)
/// ```
///
/// Larger symbols carry more data and offer stronger error correction, but at
/// the cost of a physically larger barcode. The encoder always selects the
/// smallest symbol that can hold the input at the requested ECC level.
public enum MicroQRVersion: Int, CaseIterable, Comparable, Sendable {
    case M1 = 1
    case M2 = 2
    case M3 = 3
    case M4 = 4

    /// Side length of this symbol in modules.
    ///
    /// The formula `2 × version_number + 9` comes from the ISO standard:
    ///   - The finder pattern needs 7 rows/cols.
    ///   - The separator needs 1 row/col.
    ///   - The timing row/col overlaps with the finder, adding 1 more.
    ///   - Each data "layer" adds 2 rows and 2 columns.
    var size: Int { 2 * rawValue + 9 }

    public static func < (lhs: MicroQRVersion, rhs: MicroQRVersion) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

// ============================================================================
// MARK: - MicroQREccLevel
// ============================================================================

/// Error correction level for Micro QR Code.
///
/// | Level     | Available in  | Recovery                     |
/// |-----------|---------------|------------------------------|
/// | detection | M1 only       | Detects errors; no correction |
/// | L (Low)   | M2, M3, M4    | ~7% of codewords             |
/// | M (Medium)| M2, M3, M4    | ~15% of codewords            |
/// | Q (Quarter)| M4 only      | ~25% of codewords            |
///
/// Level H (High, ~30%) is not available in any Micro QR symbol — the
/// symbols are too small to spare 30% of capacity for redundancy.
///
/// M1 only supports **error detection** (not correction). Its two ECC
/// codewords act as a checksum: a scanner can tell if the symbol is
/// damaged but cannot recover the original data.
public enum MicroQREccLevel: Sendable {
    case detection   // M1 only
    case L
    case M
    case Q
}

// ============================================================================
// MARK: - MicroQRError
// ============================================================================

/// Errors thrown by the Micro QR encoder.
public enum MicroQRError: Error {
    /// Input is too long to fit in any M1–M4 symbol at any ECC level.
    ///
    /// Maximum capacity is 35 numeric characters in an M4-L symbol.
    /// For longer inputs, use regular QR Code (ISO/IEC 18004 main standard).
    case inputTooLong(String)

    /// The requested ECC level is not available for the chosen symbol version.
    ///
    /// Common causes:
    ///   - Requesting `.detection` for M2+ (only M1 uses detection-only).
    ///   - Requesting `.Q` for M1/M2/M3 (only M4 supports Q).
    ///   - Requesting `.L` for M1 (M1 only supports detection).
    case eccNotAvailable(String)

    /// The requested encoding mode is not supported by the selected symbol.
    ///
    /// For example: byte mode is not available in M1 or M2; alphanumeric is
    /// not available in M1.
    case unsupportedMode(String)

    /// A character in the input cannot be encoded in the selected mode.
    ///
    /// For example: a lowercase letter in alphanumeric mode (which only
    /// accepts uppercase A–Z and the 35 special characters).
    case invalidCharacter(String)

    /// Layout/render error from the barcode-2d dependency.
    case layoutError(String)
}

// ============================================================================
// MARK: - Symbol Configurations
// ============================================================================
//
// All 8 valid (version, ECC) combinations are captured here as compile-time
// constant structs. Having them in a single table makes the encoding pipeline
// easy to read: every lookup is a simple array access with no conditional
// logic scattered across functions.

/// All compile-time constants for one (version, ECC) combination.
///
/// There are exactly 8 valid combinations:
///   M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
///
/// The ISO standard defines different field widths, capacities, and ECC
/// codeword counts for each. Embedding them here avoids any runtime
/// computation and makes the relationship between version and capacity
/// immediately visible.
struct SymbolConfig {
    let version: MicroQRVersion
    let ecc: MicroQREccLevel

    /// 3-bit symbol indicator encoded in the format information word.
    ///
    /// The symbol indicator uniquely identifies both the version AND the ECC
    /// level — unlike regular QR where they are encoded separately.
    /// Values 0–7 map to: M1, M2-L, M2-M, M3-L, M3-M, M4-L, M4-M, M4-Q.
    let symbolIndicator: Int

    /// Symbol side length in modules (11, 13, 15, or 17).
    let size: Int

    /// Number of data codewords (full 8-bit bytes, except M1 which has 2.5).
    ///
    /// M1 is special: it has 3 "codewords" but the last one is only 4 bits.
    /// Total M1 data capacity = 20 bits = 2 full bytes + 1 nibble.
    let dataCW: Int

    /// Number of ECC codewords appended after the data codewords.
    let eccCW: Int

    /// Maximum numeric characters (0 = mode not supported).
    let numericCap: Int

    /// Maximum alphanumeric characters (0 = mode not supported).
    let alphaCap: Int

    /// Maximum byte-mode characters (0 = mode not supported).
    let byteCap: Int

    /// Terminator bit count (3 for M1, 5 for M2, 7 for M3, 9 for M4).
    ///
    /// The terminator is a sequence of zero bits appended after the data
    /// to signal end-of-data and help fill the last codeword. Unlike regular
    /// QR (always 4 bits), Micro QR uses different lengths per symbol to
    /// accommodate the varying codeword structures.
    let terminatorBits: Int

    /// Width of the mode indicator field (0=M1, 1=M2, 2=M3, 3=M4).
    ///
    /// M1 has 0 bits because it only supports one mode (numeric).
    /// M2 uses 1 bit (0 = numeric, 1 = alphanumeric).
    /// M3 uses 2 bits (00=numeric, 01=alpha, 10=byte).
    /// M4 uses 3 bits (000=numeric, 001=alpha, 010=byte, 011=kanji).
    let modeIndicatorBits: Int

    /// Character count field width for numeric mode.
    let ccBitsNumeric: Int

    /// Character count field width for alphanumeric mode (0 = not supported).
    let ccBitsAlpha: Int

    /// Character count field width for byte mode (0 = not supported).
    let ccBitsByte: Int

    /// True only for M1: the last data "codeword" is 4 bits, not 8.
    let m1HalfCW: Bool
}

// ============================================================================
// All 8 valid symbol configurations from ISO 18004:2015 Annex E.
// ============================================================================
//
// Reading order: M1 (detection) first, then M2-L, M2-M, M3-L, M3-M,
// M4-L, M4-M, M4-Q. This matches the symbol_indicator ordering 0..7.

private let SYMBOL_CONFIGS: [SymbolConfig] = [
    // ── M1 / Detection ─────────────────────────────────────────────────────
    // Smallest symbol. Numeric only, 5-digit max. Detection-only ECC.
    // The 20-bit data capacity (3 bytes − 4 bits) is unique to M1.
    SymbolConfig(
        version: .M1, ecc: .detection,
        symbolIndicator: 0, size: 11,
        dataCW: 3, eccCW: 2,
        numericCap: 5, alphaCap: 0, byteCap: 0,
        terminatorBits: 3, modeIndicatorBits: 0,
        ccBitsNumeric: 3, ccBitsAlpha: 0, ccBitsByte: 0,
        m1HalfCW: true
    ),
    // ── M2 / L ─────────────────────────────────────────────────────────────
    // First symbol to support alphanumeric and byte modes.
    // 5 ECC codewords provide ~7% error recovery.
    SymbolConfig(
        version: .M2, ecc: .L,
        symbolIndicator: 1, size: 13,
        dataCW: 5, eccCW: 5,
        numericCap: 10, alphaCap: 6, byteCap: 4,
        terminatorBits: 5, modeIndicatorBits: 1,
        ccBitsNumeric: 4, ccBitsAlpha: 3, ccBitsByte: 4,
        m1HalfCW: false
    ),
    // ── M2 / M ─────────────────────────────────────────────────────────────
    // Same 13×13 grid as M2-L but 1 less data codeword, 1 more ECC.
    // Trading capacity for ~15% error recovery.
    SymbolConfig(
        version: .M2, ecc: .M,
        symbolIndicator: 2, size: 13,
        dataCW: 4, eccCW: 6,
        numericCap: 8, alphaCap: 5, byteCap: 3,
        terminatorBits: 5, modeIndicatorBits: 1,
        ccBitsNumeric: 4, ccBitsAlpha: 3, ccBitsByte: 4,
        m1HalfCW: false
    ),
    // ── M3 / L ─────────────────────────────────────────────────────────────
    // 15×15 grid. First to support true 2-bit mode indicator.
    // 11 data codewords allow up to 23 numeric or 9 byte characters.
    SymbolConfig(
        version: .M3, ecc: .L,
        symbolIndicator: 3, size: 15,
        dataCW: 11, eccCW: 6,
        numericCap: 23, alphaCap: 14, byteCap: 9,
        terminatorBits: 7, modeIndicatorBits: 2,
        ccBitsNumeric: 5, ccBitsAlpha: 4, ccBitsByte: 4,
        m1HalfCW: false
    ),
    // ── M3 / M ─────────────────────────────────────────────────────────────
    SymbolConfig(
        version: .M3, ecc: .M,
        symbolIndicator: 4, size: 15,
        dataCW: 9, eccCW: 8,
        numericCap: 18, alphaCap: 11, byteCap: 7,
        terminatorBits: 7, modeIndicatorBits: 2,
        ccBitsNumeric: 5, ccBitsAlpha: 4, ccBitsByte: 4,
        m1HalfCW: false
    ),
    // ── M4 / L ─────────────────────────────────────────────────────────────
    // Largest symbol. 17×17 grid. Up to 35 numeric or 15 byte characters.
    // 3-bit mode indicator supports numeric, alpha, byte, AND kanji.
    SymbolConfig(
        version: .M4, ecc: .L,
        symbolIndicator: 5, size: 17,
        dataCW: 16, eccCW: 8,
        numericCap: 35, alphaCap: 21, byteCap: 15,
        terminatorBits: 9, modeIndicatorBits: 3,
        ccBitsNumeric: 6, ccBitsAlpha: 5, ccBitsByte: 5,
        m1HalfCW: false
    ),
    // ── M4 / M ─────────────────────────────────────────────────────────────
    SymbolConfig(
        version: .M4, ecc: .M,
        symbolIndicator: 6, size: 17,
        dataCW: 14, eccCW: 10,
        numericCap: 30, alphaCap: 18, byteCap: 13,
        terminatorBits: 9, modeIndicatorBits: 3,
        ccBitsNumeric: 6, ccBitsAlpha: 5, ccBitsByte: 5,
        m1HalfCW: false
    ),
    // ── M4 / Q ─────────────────────────────────────────────────────────────
    // Highest ECC level in Micro QR. 14 ECC codewords recover ~25% damage.
    // Trade-off: only 10 data codewords → up to 21 numeric or 9 byte chars.
    SymbolConfig(
        version: .M4, ecc: .Q,
        symbolIndicator: 7, size: 17,
        dataCW: 10, eccCW: 14,
        numericCap: 21, alphaCap: 13, byteCap: 9,
        terminatorBits: 9, modeIndicatorBits: 3,
        ccBitsNumeric: 6, ccBitsAlpha: 5, ccBitsByte: 5,
        m1HalfCW: false
    ),
]

// ============================================================================
// MARK: - Pre-computed format information table
// ============================================================================
//
// The 15-bit format word encodes:
//   [symbol_indicator (3 bits)][mask_pattern (2 bits)][BCH-10 remainder (10 bits)]
//
// XOR-masked with 0x4445 (Micro QR specific — differs from regular QR's 0x5412
// to prevent a Micro QR symbol from being misread as a regular QR symbol).
//
// All 32 values (8 symbol indicators × 4 mask patterns) are pre-computed.
// Indexed as FORMAT_TABLE[symbol_indicator][mask_pattern].
//
// Verified against ISO 18004:2015 Annex E Table E.1.

private let FORMAT_TABLE: [[UInt16]] = [
    [0x4445, 0x4172, 0x4E2B, 0x4B1C],  // M1       (symbol_indicator = 0)
    [0x5528, 0x501F, 0x5F46, 0x5A71],  // M2-L     (symbol_indicator = 1)
    [0x6649, 0x637E, 0x6C27, 0x6910],  // M2-M     (symbol_indicator = 2)
    [0x7764, 0x7253, 0x7D0A, 0x783D],  // M3-L     (symbol_indicator = 3)
    [0x06DE, 0x03E9, 0x0CB0, 0x0987],  // M3-M     (symbol_indicator = 4)
    [0x17F3, 0x12C4, 0x1D9D, 0x18AA],  // M4-L     (symbol_indicator = 5)
    [0x24B2, 0x2185, 0x2EDC, 0x2BEB],  // M4-M     (symbol_indicator = 6)
    [0x359F, 0x30A8, 0x3FF1, 0x3AC6],  // M4-Q     (symbol_indicator = 7)
]

// ============================================================================
// MARK: - RS generator polynomials
// ============================================================================
//
// Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
//
//   g(x) = (x + α^0)(x + α^1) · · · (x + α^{n-1})
//
// where n is the number of ECC codewords and α = 2 (the generator of GF(256)).
//
// The first element is the leading coefficient 0x01 (monic polynomial).
// Only the counts {2, 5, 6, 8, 10, 14} are used by Micro QR.
//
// These are the same polynomials used by regular QR Code for matching ECC
// counts. We embed them as constants rather than computing at runtime.

private func generator(for eccCount: Int) -> [UInt8] {
    switch eccCount {
    case 2:
        // (x + α^0)(x + α^1) = x^2 + 3x + 2
        return [0x01, 0x03, 0x02]
    case 5:
        // Used by M2-L
        return [0x01, 0x1F, 0xF6, 0x44, 0xD9, 0x68]
    case 6:
        // Used by M2-M and M3-L
        return [0x01, 0x3F, 0x4E, 0x17, 0x9B, 0x05, 0x37]
    case 8:
        // Used by M3-M and M4-L
        return [0x01, 0x63, 0x0D, 0x60, 0x6D, 0x5B, 0x10, 0xA2, 0xA3]
    case 10:
        // Used by M4-M
        return [0x01, 0xF6, 0x75, 0xA8, 0xD0, 0xC3, 0xE3, 0x36, 0xE1, 0x3C, 0x45]
    case 14:
        // Used by M4-Q
        return [0x01, 0xF6, 0x9A, 0x60, 0x97, 0x8A, 0xF1, 0xA4, 0xA1, 0x8E, 0xFC, 0x7A, 0x52, 0xAD, 0xAC]
    default:
        // This should never be reached with valid symbol configurations.
        fatalError("MicroQR: no generator polynomial for eccCount=\(eccCount)")
    }
}

// ============================================================================
// MARK: - Encoding mode
// ============================================================================

/// The encoding mode determines how input characters are packed into bits.
///
/// Selection priority (most compact first):
///   1. Numeric    — digits 0–9 only, 3 digits per 10 bits
///   2. Alphanumeric — 45-char set, 2 chars per 11 bits
///   3. Byte       — raw UTF-8 bytes, 1 byte per 8 bits
///
/// Mode availability depends on the symbol version (see `SymbolConfig`).
enum EncodingMode {
    case numeric
    case alphanumeric
    case byte
}

/// The 45-character alphanumeric set shared with regular QR Code.
///
/// ```
/// 0–9  → indices 0–9
/// A–Z  → indices 10–35
/// SP   → 36    $  → 37    %  → 38    *  → 39
/// +  → 40    -  → 41    .  → 42    /  → 43    :  → 44
/// ```
///
/// Notice: NO lowercase letters, no parentheses, no ampersand, etc.
/// Any input containing such characters must use byte mode.
private let ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:"

/// Select the most compact encoding mode supported by the given config.
///
/// - Priority: numeric > alphanumeric > byte
/// - If no supported mode can encode the input, throws `unsupportedMode`.
private func selectMode(input: String, cfg: SymbolConfig) throws -> EncodingMode {
    // Numeric: all characters must be ASCII digits
    let isNumeric = input.isEmpty || input.unicodeScalars.allSatisfy {
        $0.value >= 0x30 && $0.value <= 0x39  // '0'...'9'
    }
    if isNumeric && cfg.ccBitsNumeric > 0 {
        return .numeric
    }

    // Alphanumeric: all characters must be in the 45-char set
    let isAlpha = input.unicodeScalars.allSatisfy { scalar in
        guard let c = Unicode.Scalar(scalar.value).map(Character.init) else { return false }
        return ALPHANUM_CHARS.contains(c)
    }
    if isAlpha && cfg.alphaCap > 0 {
        return .alphanumeric
    }

    // Byte: raw bytes (UTF-8). Works for any string, but only if symbol supports it.
    if cfg.byteCap > 0 {
        return .byte
    }

    throw MicroQRError.unsupportedMode(
        "Input cannot be encoded in any mode supported by \(cfg.version)-\(cfg.ecc)"
    )
}

/// Return the mode indicator bits (0..7) for the given mode and symbol config.
///
/// The indicator width is 0 bits for M1 (no indicator since there is only one
/// mode), 1 bit for M2, 2 bits for M3, 3 bits for M4.
private func modeIndicatorValue(mode: EncodingMode, cfg: SymbolConfig) -> UInt32 {
    switch cfg.modeIndicatorBits {
    case 0: return 0   // M1: single mode, no indicator needed
    case 1:            // M2: 0 = numeric, 1 = alphanumeric
        return mode == .numeric ? 0 : 1
    case 2:            // M3: 00 = numeric, 01 = alpha, 10 = byte
        switch mode {
        case .numeric:      return 0b00
        case .alphanumeric: return 0b01
        case .byte:         return 0b10
        }
    default:           // M4: 000 = numeric, 001 = alpha, 010 = byte, 011 = kanji
        switch mode {
        case .numeric:      return 0b000
        case .alphanumeric: return 0b001
        case .byte:         return 0b010
        }
    }
}

/// Character count field width for the given mode and symbol config.
private func charCountBits(mode: EncodingMode, cfg: SymbolConfig) -> Int {
    switch mode {
    case .numeric:      return cfg.ccBitsNumeric
    case .alphanumeric: return cfg.ccBitsAlpha
    case .byte:         return cfg.ccBitsByte
    }
}

// ============================================================================
// MARK: - Bit writer
// ============================================================================
//
// The bit writer accumulates individual bits (MSB-first per value) and
// converts them to bytes on demand. This matches the QR/Micro-QR convention
// of big-endian bit ordering within each codeword.
//
// Example: write(value: 0b101, count: 3) appends [1, 0, 1].

/// Accumulates bits MSB-first and flushes to bytes.
///
/// Each `write(_:count:)` call appends the `count` least-significant bits of
/// `value` to the bit stream, most-significant bit first.
///
/// Example:
///   write(5, 3)  →  appends bits [1, 0, 1]  (5 = 0b101, MSB first)
class BitWriter {
    private var bits: [UInt8] = []  // each element is 0 or 1

    /// Append the `count` LSBs of `value`, MSB-first.
    func write(_ value: UInt32, count: Int) {
        guard count > 0 else { return }
        for i in stride(from: count - 1, through: 0, by: -1) {
            bits.append(UInt8((value >> i) & 1))
        }
    }

    /// Number of bits written so far.
    var bitLen: Int { bits.count }

    /// Pack bits into bytes (0-padded on the right for incomplete final byte).
    func toBytes() -> [UInt8] {
        var result: [UInt8] = []
        var i = 0
        while i < bits.count {
            var byte: UInt8 = 0
            for j in 0..<8 {
                byte = (byte << 1) | (i + j < bits.count ? bits[i + j] : 0)
            }
            result.append(byte)
            i += 8
        }
        return result
    }

    /// Return the raw bit array (each element is 0 or 1).
    func toBitVec() -> [UInt8] { bits }
}

// ============================================================================
// MARK: - Data encoding helpers
// ============================================================================

/// Encode a numeric string into the bit writer.
///
/// Groups digits into triples (10 bits), pairs (7 bits), or singles (4 bits):
///
/// ```
/// "012"  → decimal 12  → 10 bits: 0000001100
/// "45"   → decimal 45  → 7 bits:  0101101
/// "6"    → decimal 6   → 4 bits:  0110
/// ```
///
/// This packs 3 digits into 10 bits rather than 24 bits (3 × 8), achieving
/// roughly 3.3 digits per byte — much better than raw ASCII.
private func encodeNumeric(_ input: String, writer: BitWriter) {
    let digits = input.unicodeScalars.map { UInt32($0.value - 0x30) }
    var i = 0
    while i + 2 < digits.count {
        writer.write(digits[i] * 100 + digits[i + 1] * 10 + digits[i + 2], count: 10)
        i += 3
    }
    if i + 1 < digits.count {
        writer.write(digits[i] * 10 + digits[i + 1], count: 7)
        i += 2
    }
    if i < digits.count {
        writer.write(digits[i], count: 4)
    }
}

/// Encode an alphanumeric string into the bit writer.
///
/// Pairs of characters are packed into 11 bits: `first × 45 + second`.
/// A trailing single character uses 6 bits.
///
/// ```
/// "AC"  → index(A)=10, index(C)=12 → 10×45+12 = 462 → 11 bits
/// "-"   → index(-)=41 → 6 bits: 101001
/// ```
///
/// Packs 2 characters into 11 bits vs. 16 bits raw — ~31% space saving.
private func encodeAlphanumeric(_ input: String, writer: BitWriter) {
    let indices: [UInt32] = input.unicodeScalars.map { scalar in
        let c = Character(scalar)
        return UInt32(ALPHANUM_CHARS.firstIndex(of: c)!.utf16Offset(in: ALPHANUM_CHARS))
    }
    var i = 0
    while i + 1 < indices.count {
        writer.write(indices[i] * 45 + indices[i + 1], count: 11)
        i += 2
    }
    if i < indices.count {
        writer.write(indices[i], count: 6)
    }
}

/// Encode byte mode: each UTF-8 byte as an 8-bit value.
///
/// This is the least efficient mode but the most general — it can encode
/// any byte sequence, including multi-byte UTF-8 characters.
private func encodeByte(_ input: String, writer: BitWriter) {
    for byte in input.utf8 {
        writer.write(UInt32(byte), count: 8)
    }
}

// ============================================================================
// MARK: - Reed-Solomon encoder
// ============================================================================
//
// Reed-Solomon error correction encodes data as a polynomial D(x) and
// computes the remainder of D(x)·x^n mod G(x), where G(x) is the
// generator polynomial.
//
// The remainder is the ECC, appended to the data. A scanner that reads
// D(x) with up to floor(n/2) corrupted codewords can recover the original.
//
// This uses the b=0 convention (first root is α^0 = 1), which is the
// same convention used by regular QR Code and is handled naturally by the
// GF256.multiply() function.

/// Compute ECC bytes using LFSR polynomial division over GF(256)/0x11D.
///
/// Returns the n-byte remainder of `data(x) · x^n mod generator(x)`.
///
/// The algorithm processes one data byte at a time using a feedback shift
/// register (LFSR). The "feedback" is the XOR of the leading data byte
/// with the current leading ECC byte, which drives the polynomial division.
///
/// This is identical to the regular QR Code RS encoder.
private func rsEncode(data: [UInt8], generator: [UInt8]) -> [UInt8] {
    let n = generator.count - 1  // degree of generator = number of ECC bytes

    // Start with all-zero remainder (the "shift register")
    var rem = [UInt8](repeating: 0, count: n)

    for b in data {
        // Feedback term: the XOR of the incoming data byte with the leading
        // ECC byte. This is the key step in polynomial long division over GF(2^8).
        let fb = b ^ rem[0]

        // Shift the register left (drop the leading byte, make room at end)
        rem.removeFirst()
        rem.append(0)

        // If feedback is non-zero, XOR each ECC byte with generator[i+1] × feedback.
        // This is the "multiply and accumulate" step of GF polynomial division.
        if fb != 0 {
            for i in 0..<n {
                rem[i] ^= GF256.multiply(generator[i + 1], fb)
            }
        }
    }

    return rem  // The computed ECC codewords
}

// ============================================================================
// MARK: - Data codeword assembly
// ============================================================================

/// Build the complete data codeword byte sequence for the given input.
///
/// The bit stream structure for non-M1 symbols:
///
/// ```
/// [mode indicator (0/1/2/3 bits)]
/// [character count (width from table)]
/// [encoded data bits]
/// [terminator (3/5/7/9 zero bits, truncated if capacity exhausted)]
/// [zero bits to reach next byte boundary]
/// [padding codewords: 0xEC 0x11 0xEC 0x11 ... to fill data codewords]
/// ```
///
/// M1 is special (20-bit capacity = 2 full bytes + nibble):
///
/// ```
/// [3-bit char count]
/// [encoded data bits]
/// [3-bit terminator, truncated]
/// [zero-padded to 20 bits]
/// → 3 bytes: byte[2] has data in upper nibble, lower nibble = 0
/// ```
private func buildDataCodewords(input: String, cfg: SymbolConfig, mode: EncodingMode) -> [UInt8] {
    // Total usable data bit capacity
    // M1: 3×8 − 4 = 20 bits (the last "codeword" is only a nibble)
    let totalBits = cfg.m1HalfCW ? cfg.dataCW * 8 - 4 : cfg.dataCW * 8

    let w = BitWriter()

    // ── Mode indicator ──────────────────────────────────────────────────────
    // M1 has no mode indicator (implicitly numeric-only).
    if cfg.modeIndicatorBits > 0 {
        w.write(modeIndicatorValue(mode: mode, cfg: cfg),
                count: cfg.modeIndicatorBits)
    }

    // ── Character count ─────────────────────────────────────────────────────
    // Byte mode counts bytes (not Unicode characters); all others count chars.
    let charCount: Int = (mode == .byte) ? input.utf8.count : input.unicodeScalars.count
    w.write(UInt32(charCount), count: charCountBits(mode: mode, cfg: cfg))

    // ── Encoded data bits ────────────────────────────────────────────────────
    switch mode {
    case .numeric:      encodeNumeric(input, writer: w)
    case .alphanumeric: encodeAlphanumeric(input, writer: w)
    case .byte:         encodeByte(input, writer: w)
    }

    // ── Terminator ──────────────────────────────────────────────────────────
    // Append up to terminatorBits zero bits. Truncate if capacity is full.
    let remaining = totalBits - w.bitLen
    if remaining > 0 {
        let termLen = min(cfg.terminatorBits, remaining)
        w.write(0, count: termLen)
    }

    // ── M1 special handling ─────────────────────────────────────────────────
    // Pad to exactly 20 bits; pack into 3 bytes where byte[2] uses only
    // its upper nibble.
    if cfg.m1HalfCW {
        var bits = w.toBitVec()
        bits += [UInt8](repeating: 0, count: max(0, 20 - bits.count))

        // Bits 0..7 → byte 0, bits 8..15 → byte 1, bits 16..19 → upper nibble of byte 2
        let b0 = (bits[0]  << 7) | (bits[1]  << 6) | (bits[2]  << 5) | (bits[3]  << 4)
               | (bits[4]  << 3) | (bits[5]  << 2) | (bits[6]  << 1) | bits[7]
        let b1 = (bits[8]  << 7) | (bits[9]  << 6) | (bits[10] << 5) | (bits[11] << 4)
               | (bits[12] << 3) | (bits[13] << 2) | (bits[14] << 1) | bits[15]
        let b2 = (bits[16] << 7) | (bits[17] << 6) | (bits[18] << 5) | (bits[19] << 4)
        return [b0, b1, b2]
    }

    // ── Pad to byte boundary ────────────────────────────────────────────────
    let rem = w.bitLen % 8
    if rem != 0 {
        w.write(0, count: 8 - rem)
    }

    // ── Fill remaining codewords with 0xEC / 0x11 ───────────────────────────
    // The alternating pattern is a well-known QR/Micro-QR convention.
    // 0xEC = 11101100, 0x11 = 00010001. Together they avoid long runs.
    var bytes = w.toBytes()
    var pad: UInt8 = 0xEC
    while bytes.count < cfg.dataCW {
        bytes.append(pad)
        pad = (pad == 0xEC) ? 0x11 : 0xEC
    }
    return bytes
}

// ============================================================================
// MARK: - Symbol selection
// ============================================================================

/// Find the smallest SymbolConfig that can hold the given input.
///
/// Scans configs in the order M1, M2-L, M2-M, M3-L, M3-M, M4-L, M4-M, M4-Q
/// (smallest first). For each candidate that matches the requested version
/// and/or ECC filter, tries the most compact mode (numeric > alpha > byte)
/// and checks if the input fits within that mode's capacity.
///
/// Returns the first (smallest) config where the input fits.
private func selectConfig(
    input: String,
    version: MicroQRVersion?,
    ecc: MicroQREccLevel?
) throws -> SymbolConfig {
    let candidates = SYMBOL_CONFIGS.filter { cfg in
        if let v = version, cfg.version != v { return false }
        if let e = ecc,     !eccMatches(e, cfg.ecc) { return false }
        return true
    }

    guard !candidates.isEmpty else {
        throw MicroQRError.eccNotAvailable(
            "No symbol configuration matches version=\(String(describing: version)) ecc=\(String(describing: ecc))"
        )
    }

    for cfg in candidates {
        if let mode = try? selectMode(input: input, cfg: cfg) {
            let inputLen = (mode == .byte) ? input.utf8.count : input.unicodeScalars.count
            let cap: Int
            switch mode {
            case .numeric:      cap = cfg.numericCap
            case .alphanumeric: cap = cfg.alphaCap
            case .byte:         cap = cfg.byteCap
            }
            if cap > 0 && inputLen <= cap {
                return cfg
            }
        }
    }

    throw MicroQRError.inputTooLong(
        "Input (length \(input.utf8.count)) does not fit in any Micro QR symbol "
        + "(version=\(String(describing: version)), ecc=\(String(describing: ecc))). "
        + "Maximum is 35 numeric chars in M4-L."
    )
}

/// Check whether a user-requested ECC level is compatible with a config's level.
///
/// We allow a nil ecc to match anything (used for auto-selection).
private func eccMatches(_ requested: MicroQREccLevel, _ configEcc: MicroQREccLevel) -> Bool {
    return requested == configEcc
}

// ============================================================================
// MARK: - Working grid
// ============================================================================
//
// The working grid tracks two layers:
//   1. modules[row][col]  — the actual dark/light value of each module
//   2. reserved[row][col] — whether the module is a structural/format module
//
// Reserved modules are never overwritten during data placement or masking.
// This two-layer design cleanly separates structural layout from data placement.

/// Mutable working grid used during the encoding pipeline.
///
/// Unlike `ModuleGrid` (which is an immutable value type from barcode-2d),
/// `WorkGrid` uses a reference-type wrapper for performance during in-place
/// modification (finder pattern placement, bit writing, masking).
private class WorkGrid {
    let size: Int
    var modules: [[Bool]]    // true = dark, false = light
    var reserved: [[Bool]]   // true = structural (never modified by data/mask)

    init(size: Int) {
        self.size = size
        self.modules  = [[Bool]](repeating: [Bool](repeating: false, count: size), count: size)
        self.reserved = [[Bool]](repeating: [Bool](repeating: false, count: size), count: size)
    }

    /// Set a module value and optionally mark it as reserved.
    @inline(__always)
    func set(row: Int, col: Int, dark: Bool, reserve: Bool = true) {
        modules[row][col] = dark
        if reserve { reserved[row][col] = true }
    }
}

// ============================================================================
// MARK: - Structural module placement
// ============================================================================

/// Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
///
/// The finder pattern is identical to regular QR Code:
///
/// ```
///  ■ ■ ■ ■ ■ ■ ■    (outer border: all dark)
///  ■ □ □ □ □ □ ■    (one-module-wide light ring)
///  ■ □ ■ ■ ■ □ ■    (3×3 dark center)
///  ■ □ ■ ■ ■ □ ■
///  ■ □ ■ ■ ■ □ ■
///  ■ □ □ □ □ □ ■
///  ■ ■ ■ ■ ■ ■ ■
/// ```
///
/// The 1:1:3:1:1 ratio of the pattern is what scanners detect. Because there
/// is only one finder, a scanner immediately knows the top-left corner.
private func placeFinder(_ g: WorkGrid) {
    for dr in 0..<7 {
        for dc in 0..<7 {
            // The finder has three regions:
            //   outer border (outermost row/col) → dark
            //   inner border (second outermost)  → light
            //   3×3 core                          → dark
            let onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6
            let inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4
            g.set(row: dr, col: dc, dark: onBorder || inCore)
        }
    }
}

/// Place the L-shaped separator (light modules bordering the finder).
///
/// Regular QR Code surrounds ALL THREE finders with a 1-module separator.
/// Micro QR only needs the bottom and right sides — the top and left are
/// the symbol's own physical boundary.
///
/// ```
/// Row 7, cols 0–7  →  8 light modules  (bottom of finder + corner)
/// Col 7, rows 0–7  →  8 light modules  (right of finder + same corner)
/// (corner at row 7, col 7 is included in both — counted once)
/// ```
private func placeSeparator(_ g: WorkGrid) {
    for i in 0...7 {
        g.set(row: 7, col: i, dark: false)  // bottom separator row
        g.set(row: i, col: 7, dark: false)  // right separator column
    }
}

/// Place timing pattern extensions along row 0 and col 0.
///
/// In regular QR, timing runs along row 6 and col 6 (fixed regardless of
/// version). In Micro QR, timing runs along the **outer edges** (row 0,
/// col 0), extending from the finder/separator boundary to the symbol edge.
///
/// Positions 0–6: already determined by the finder pattern.
/// Position 7:    the separator (always light).
/// Position 8+:   timing extension — dark if even, light if odd.
///
/// The even/odd rule is consistent with the finder: col 0 of row 0
/// (position 0, even) is dark, which matches the finder's outer border.
private func placeTiming(_ g: WorkGrid) {
    let sz = g.size
    for c in 8..<sz {
        g.set(row: 0, col: c, dark: c % 2 == 0)
    }
    for r in 8..<sz {
        g.set(row: r, col: 0, dark: r % 2 == 0)
    }
}

/// Reserve the 15 format information module positions (filled in later).
///
/// The L-shaped format info strip occupies:
///   Row 8, cols 1–8  →  8 modules  (bits f14 down to f7, MSB-first)
///   Col 8, rows 1–7  →  7 modules  (bits f6 down to f0)
///
/// They are reserved now (set to light = 0) and overwritten after mask
/// selection with the actual format bits.
private func reserveFormatInfo(_ g: WorkGrid) {
    for c in 1...8 { g.set(row: 8, col: c, dark: false) }
    for r in 1...7 { g.set(row: r, col: 8, dark: false) }
}

/// Write the 15-bit format word into the reserved format positions.
///
/// Bit layout (f14 = MSB, f0 = LSB):
///
/// ```
///   Row 8, col 1 ← f14  (MSB, placed first)
///   Row 8, col 2 ← f13
///   ...
///   Row 8, col 8 ← f7
///   Col 8, row 7 ← f6
///   Col 8, row 6 ← f5
///   ...
///   Col 8, row 1 ← f0   (LSB, placed last)
/// ```
///
/// This single copy differs from regular QR, which writes format info twice
/// (for redundancy). Micro QR omits the second copy to save space.
private func writeFormatInfo(_ g: WorkGrid, fmt: UInt16) {
    // Row 8, cols 1–8: bits f14 (MSB) down to f7
    for i: UInt16 in 0..<8 {
        g.modules[8][Int(1 + i)] = ((fmt >> (14 - i)) & 1) == 1
    }
    // Col 8, rows 7 down to 1: bits f6 down to f0 (LSB)
    for i: UInt16 in 0..<7 {
        g.modules[Int(7 - i)][8] = ((fmt >> (6 - i)) & 1) == 1
    }
}

/// Initialize the grid with all structural modules.
private func buildGrid(cfg: SymbolConfig) -> WorkGrid {
    let g = WorkGrid(size: cfg.size)
    placeFinder(g)
    placeSeparator(g)
    placeTiming(g)
    reserveFormatInfo(g)
    return g
}

// ============================================================================
// MARK: - Data placement (two-column zigzag)
// ============================================================================
//
// The data placement algorithm scans the grid in a two-column zigzag pattern,
// starting from the bottom-right corner and moving left two columns at a time,
// alternating between upward and downward directions.
//
// ```
// col = size - 1  (start at rightmost column)
// dir = upward
//
// while col >= 1:
//   for each row in the current direction:
//     for sub_col in [col, col-1]:
//       if not reserved: place next bit
//   flip direction; col -= 2
// ```
//
// Note: unlike regular QR, there is NO timing column at col 6 to skip around.
// Micro QR's timing is at col 0, which is reserved and auto-skipped.

/// Place data bits into the grid via two-column zigzag.
///
/// Reserved modules (finder, separator, timing, format) are skipped
/// automatically. Remaining bits after all data+ECC are placed are zero
/// (remainder bits). M1 has 4 remainder bits; all others have 0.
private func placeBits(g: WorkGrid, bits: [Bool]) {
    let sz = g.size
    var bitIdx = 0
    var goUp = true  // direction flag

    var col = sz - 1
    while col >= 1 {
        for vi in 0..<sz {
            let row = goUp ? (sz - 1 - vi) : vi
            for dc in 0...1 {
                let c = col - dc
                // Skip reserved (structural/format) modules
                if g.reserved[row][c] { continue }
                // Place next bit (or zero for remainder)
                g.modules[row][c] = (bitIdx < bits.count) ? bits[bitIdx] : false
                bitIdx += 1
            }
        }
        goUp = !goUp
        col -= 2
    }
}

// ============================================================================
// MARK: - Masking
// ============================================================================
//
// Masking flips non-reserved module values to avoid patterns that confuse
// scanners: long runs, 2×2 same-color blocks, finder-like sequences.
//
// Micro QR uses only 4 mask patterns (not 8 as in regular QR):
//
//   Pattern 0: flip if (row + col) mod 2 == 0
//   Pattern 1: flip if  row        mod 2 == 0
//   Pattern 2: flip if        col  mod 3 == 0
//   Pattern 3: flip if (row + col) mod 3 == 0

/// Test whether mask pattern `maskIdx` applies to module at (row, col).
///
/// Returns `true` if the module value should be flipped.
private func maskCondition(_ maskIdx: Int, row: Int, col: Int) -> Bool {
    switch maskIdx {
    case 0: return (row + col) % 2 == 0
    case 1: return row % 2 == 0
    case 2: return col % 3 == 0
    case 3: return (row + col) % 3 == 0
    default: return false
    }
}

/// Apply mask pattern to all non-reserved modules, returning a new module grid.
///
/// The mask XORs (flips) each unreserved module value if the mask condition
/// holds. Structural modules are never masked.
private func applyMask(
    modules: [[Bool]],
    reserved: [[Bool]],
    size: Int,
    maskIdx: Int
) -> [[Bool]] {
    var result = modules
    for r in 0..<size {
        for c in 0..<size {
            if !reserved[r][c] {
                result[r][c] = modules[r][c] != maskCondition(maskIdx, row: r, col: c)
            }
        }
    }
    return result
}

// ============================================================================
// MARK: - Penalty scoring
// ============================================================================
//
// All four mask candidates are scored with the same four penalty rules as
// regular QR Code. The mask with the LOWEST total penalty is selected.
// Ties go to the lower-numbered mask pattern.

/// Compute the 4-rule penalty score for a masked module grid.
///
/// **Rule 1 — Adjacent same-color runs:**
///   Scan every row and column for runs of ≥ 5 consecutive same-color modules.
///   Score += (run_length − 2) for each qualifying run.
///   Example: a run of 5 → +3; a run of 7 → +5.
///
/// **Rule 2 — 2×2 same-color blocks:**
///   For every 2×2 square of identical color, score += 3.
///
/// **Rule 3 — Finder-like sequences:**
///   Scan for the 11-module patterns `1011101000` and its mirror.
///   Each occurrence → score += 40.
///
/// **Rule 4 — Dark-module proportion:**
///   dark_pct = dark_count * 100 / total
///   prev5 = largest multiple of 5 ≤ dark_pct
///   next5 = prev5 + 5
///   score += min(|prev5 − 50|, |next5 − 50|) / 5 × 10
private func computePenalty(modules: [[Bool]], size: Int) -> Int {
    var penalty = 0

    // ── Rule 1: runs of ≥ 5 ────────────────────────────────────────────────
    for a in 0..<size {
        for isHoriz in [true, false] {
            var run = 1
            var prev = isHoriz ? modules[a][0] : modules[0][a]
            for i in 1..<size {
                let cur = isHoriz ? modules[a][i] : modules[i][a]
                if cur == prev {
                    run += 1
                } else {
                    if run >= 5 { penalty += run - 2 }
                    run = 1
                    prev = cur
                }
            }
            if run >= 5 { penalty += run - 2 }
        }
    }

    // ── Rule 2: 2×2 same-color blocks ─────────────────────────────────────
    for r in 0..<(size - 1) {
        for c in 0..<(size - 1) {
            let d = modules[r][c]
            if d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1] {
                penalty += 3
            }
        }
    }

    // ── Rule 3: finder-like sequences ──────────────────────────────────────
    let p1: [UInt8] = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0]
    let p2: [UInt8] = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1]
    for a in 0..<size {
        let limit = size >= 11 ? size - 11 : 0
        for b in 0...limit {
            var mh1 = true, mh2 = true, mv1 = true, mv2 = true
            for k in 0..<11 {
                let bh: UInt8 = modules[a][b + k] ? 1 : 0
                let bv: UInt8 = modules[b + k][a] ? 1 : 0
                if bh != p1[k] { mh1 = false }
                if bh != p2[k] { mh2 = false }
                if bv != p1[k] { mv1 = false }
                if bv != p2[k] { mv2 = false }
            }
            if mh1 { penalty += 40 }
            if mh2 { penalty += 40 }
            if mv1 { penalty += 40 }
            if mv2 { penalty += 40 }
        }
    }

    // ── Rule 4: dark proportion ─────────────────────────────────────────────
    let dark = modules.joined().filter { $0 }.count
    let total = size * size
    let darkPct = (dark * 100) / total
    let prev5 = (darkPct / 5) * 5
    let next5 = prev5 + 5
    let r4 = min(abs(prev5 - 50), abs(next5 - 50))
    penalty += (r4 / 5) * 10

    return penalty
}

// ============================================================================
// MARK: - Public API
// ============================================================================

/// Encode a string to a Micro QR Code `ModuleGrid`.
///
/// Automatically selects the smallest symbol (M1..M4) and ECC level that
/// can hold the input. Pass `version` and/or `ecc` to override the defaults.
///
/// Auto-selection order (smallest first):
///   M1/Detection → M2/L → M2/M → M3/L → M3/M → M4/L → M4/M → M4/Q
///
/// - Parameters:
///   - input: The string to encode.
///   - version: Optional: force a specific symbol version (M1–M4).
///   - ecc: Optional: force a specific ECC level. Defaults to nil (auto).
///
/// - Returns: A `ModuleGrid` ready for rendering via `barcode-2d`.
///
/// - Throws:
///   - `MicroQRError.inputTooLong` if input exceeds M4 capacity.
///   - `MicroQRError.eccNotAvailable` if the version+ECC combination is invalid.
///   - `MicroQRError.unsupportedMode` if no mode can encode the input.
///
/// # Example
///
/// ```swift
/// let grid = try encode("HELLO")
/// // grid.rows == 13  (M2 symbol, auto-selected)
///
/// let m4 = try encode("https://a.b", version: .M4, ecc: .L)
/// // m4.rows == 17
/// ```
public func encode(
    _ input: String,
    version: MicroQRVersion? = nil,
    ecc: MicroQREccLevel? = nil
) throws -> ModuleGrid {
    let cfg = try selectConfig(input: input, version: version, ecc: ecc)
    let mode = try selectMode(input: input, cfg: cfg)

    // ── 1. Build data codewords ──────────────────────────────────────────────
    let dataCW = buildDataCodewords(input: input, cfg: cfg, mode: mode)

    // ── 2. Compute RS ECC ────────────────────────────────────────────────────
    let gen = generator(for: cfg.eccCW)
    let eccCW = rsEncode(data: dataCW, generator: gen)

    // ── 3. Flatten codewords to bit stream ──────────────────────────────────
    // For M1: the final data "codeword" only contributes 4 bits (upper nibble).
    // All other codewords contribute all 8 bits.
    let finalCW = dataCW + eccCW
    var bits: [Bool] = []
    for (cwIdx, cw) in finalCW.enumerated() {
        // M1's last data codeword (index = dataCW - 1) is 4 bits, not 8.
        let bitsInCW = (cfg.m1HalfCW && cwIdx == cfg.dataCW - 1) ? 4 : 8
        for b in stride(from: bitsInCW - 1, through: 0, by: -1) {
            // Shift right by b+offset to extract MSB-first
            let shift = b + (8 - bitsInCW)
            bits.append(((cw >> shift) & 1) == 1)
        }
    }

    // ── 4. Build grid with structural modules ────────────────────────────────
    let g = buildGrid(cfg: cfg)

    // ── 5. Place data bits ───────────────────────────────────────────────────
    placeBits(g: g, bits: bits)

    // ── 6. Evaluate all 4 masks, pick lowest penalty ─────────────────────────
    var bestMask = 0
    var bestPenalty = Int.max

    for m in 0..<4 {
        let masked = applyMask(modules: g.modules, reserved: g.reserved, size: cfg.size, maskIdx: m)

        // Build a temporary copy with format info to score the full grid
        var tmpModules = masked
        let fmt = FORMAT_TABLE[cfg.symbolIndicator][m]

        // Write format info bits inline (mirrors writeFormatInfo logic)
        for i: UInt16 in 0..<8 {
            tmpModules[8][Int(1 + i)] = ((fmt >> (14 - i)) & 1) == 1
        }
        for i: UInt16 in 0..<7 {
            tmpModules[Int(7 - i)][8] = ((fmt >> (6 - i)) & 1) == 1
        }

        let p = computePenalty(modules: tmpModules, size: cfg.size)
        if p < bestPenalty {
            bestPenalty = p
            bestMask = m
        }
    }

    // ── 7. Apply best mask and write final format info ───────────────────────
    let finalModules = applyMask(
        modules: g.modules,
        reserved: g.reserved,
        size: cfg.size,
        maskIdx: bestMask
    )

    // Overwrite the work-grid modules with the masked version, then stamp
    // the real format info on top.
    for r in 0..<cfg.size {
        g.modules[r] = finalModules[r]
    }
    writeFormatInfo(g, fmt: FORMAT_TABLE[cfg.symbolIndicator][bestMask])

    // ── 8. Return as immutable ModuleGrid ────────────────────────────────────
    return ModuleGrid(
        cols: cfg.size,
        rows: cfg.size,
        modules: g.modules,
        moduleShape: .square
    )
}

/// Encode at a specific symbol version and ECC level.
///
/// - Parameters:
///   - input: The string to encode.
///   - version: The exact symbol version to use.
///   - ecc: The exact ECC level to use.
///
/// - Throws: `MicroQRError.inputTooLong` if input does not fit in the
///   requested version+ECC combination.
///
/// # Example
///
/// ```swift
/// let grid = try encodeAt("123", version: .M1, ecc: .detection)
/// // grid.rows == 11  (M1 = 11×11)
/// ```
public func encodeAt(
    _ input: String,
    version: MicroQRVersion,
    ecc: MicroQREccLevel
) throws -> ModuleGrid {
    try encode(input, version: version, ecc: ecc)
}

/// Convert a `ModuleGrid` to a `PaintScene` via `barcode-2d::layout()`.
///
/// Sets `quietZoneModules = 2` (Micro QR minimum) unless overridden in `config`.
///
/// - Parameters:
///   - grid: The Micro QR `ModuleGrid` to render.
///   - config: Optional layout configuration. If nil, uses defaults with
///             `quietZoneModules = 2`.
///
/// - Returns: A `PaintScene` ready for a paint-vm backend.
///
/// - Throws: `MicroQRError.layoutError` if layout configuration is invalid.
public func layoutGrid(
    _ grid: ModuleGrid,
    config: Barcode2DLayoutConfig? = nil
) throws -> PaintScene {
    var cfg = config ?? Barcode2DLayoutConfig()
    if config == nil {
        cfg.quietZoneModules = 2  // Micro QR uses 2-module quiet zone (not 4)
    }
    do {
        return try layout(grid: grid, config: cfg)
    } catch {
        throw MicroQRError.layoutError(String(describing: error))
    }
}
