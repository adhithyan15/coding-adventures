// AztecCode.swift — ISO/IEC 24778:2008 Aztec Code encoder
//
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Overview
// ============================================================================
//
// Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
// published as a patent-free format. It is named after the central bullseye
// finder pattern, which resembles the stepped pyramid on the Aztec calendar.
//
// Unlike QR Code (which uses three square finder patterns in three corners),
// Aztec Code places a **single bullseye finder at the center**. The scanner
// finds the center first, then reads outward in a clockwise spiral. This
// means:
//
//   1. No large quiet zone is needed — the symbol can fill a label edge-to-edge.
//   2. Any of the four 90° rotations is supported; orientation is decoded
//      from the mode message after locating the bullseye.
//   3. Compact symbols fit in as few as 15×15 modules.
//
// ## Where Aztec Code is used today
//
//   - IATA boarding passes — every airline boarding pass
//   - Eurostar and Amtrak rail tickets
//   - PostNL, Deutsche Post, La Poste — European postal routing
//   - US military ID cards
//
// ## Symbol variants
//
// ```
// Compact: 1–4 layers,  size = 11 + 4*layers  (15×15 to 27×27)
// Full:    1–32 layers, size = 15 + 4*layers  (19×19 to 143×143)
// ```
//
// ## Encoding pipeline (v0.1.0 — byte-mode only)
//
// ```
// input string / bytes
//   → Binary-Shift codewords from Upper mode (byte-only, v0.1.0 simplification)
//   → symbol size selection (smallest compact then full at 23% ECC)
//   → pad to exact codeword count
//   → GF(256)/0x12D Reed-Solomon ECC (b=1 convention, same as Data Matrix)
//   → bit stuffing (insert complement after 4 consecutive identical bits)
//   → GF(16)/0x13 mode message (layers + codeword count + 5 or 6 RS nibbles)
//   → ModuleGrid: bullseye → orientation marks → mode msg → data spiral
// ```
//
// ## v0.1.0 simplifications (documented per spec)
//
//   1. Byte-mode only — all input encoded via Binary-Shift from Upper mode.
//      Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimisation is v0.2.0.
//   2. 8-bit codewords → GF(256)/0x12D RS (same polynomial as Data Matrix).
//      GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.
//   3. Default ECC = 23%.  No ECC percentage knob exposed in v0.1.0.
//   4. Auto-select compact vs full.  No force-compact option in v0.1.0.
//
// ============================================================================

import Barcode2D
import PaintInstructions

// ============================================================================
// MARK: - Package version
// ============================================================================

/// Current package version.
public let aztecCodeVersion = "0.1.0"

// ============================================================================
// MARK: - Public errors
// ============================================================================

/// Base error type for Aztec Code encoding failures.
public enum AztecError: Error, Equatable, CustomStringConvertible {
    /// The input is too long to fit in any 32-layer full Aztec Code symbol.
    case inputTooLong(String)

    /// An internal state inconsistency (should never occur in correct usage).
    case internalError(String)

    public var description: String {
        switch self {
        case .inputTooLong(let msg):   return "AztecCode input too long: \(msg)"
        case .internalError(let msg):  return "AztecCode internal error: \(msg)"
        }
    }
}

// ============================================================================
// MARK: - Public API
// ============================================================================

/// Options for Aztec Code encoding.
public struct AztecOptions: Sendable {
    /// Minimum error-correction percentage (default: 23, range: 10–90).
    ///
    /// Higher values produce larger symbols but can recover from more damage.
    /// 23% is the Aztec Code standard default — it lets the scanner recover
    /// from approximately 11.5% corrupted codewords.
    public var minEccPercent: Int

    public init(minEccPercent: Int = 23) {
        self.minEccPercent = min(90, max(10, minEccPercent))
    }
}

/// Encode `data` as an Aztec Code symbol.
///
/// Returns a `ModuleGrid` where `modules[row][col] == true` means a dark module.
/// The grid origin `(0, 0)` is the top-left corner.
///
/// - Parameters:
///   - data:    The string to encode. Encoded as UTF-8 via Binary-Shift.
///   - options: Encoding options (ECC percentage, etc.).
///
/// - Throws: `AztecError.inputTooLong` if the data exceeds the capacity of
///           a 32-layer full Aztec symbol.
///
/// Example:
/// ```swift
/// let grid = try AztecCode.encode("Hello, World!")
/// print(grid.rows)  // → 15 (compact 1-layer for short strings)
/// ```
public enum AztecCode {
    public static func encode(_ data: String, options: AztecOptions = AztecOptions()) throws -> [[Bool]] {
        let bytes = Array(data.utf8)
        let grid = try encodeBytes(bytes, options: options)
        return grid.modules
    }

    public static func encodeData(_ bytes: [UInt8], options: AztecOptions = AztecOptions()) throws -> [[Bool]] {
        let grid = try encodeBytes(Array(bytes), options: options)
        return grid.modules
    }

    public static func encodeToGrid(_ data: String, options: AztecOptions = AztecOptions()) throws -> ModuleGrid {
        let bytes = Array(data.utf8)
        return try encodeBytes(bytes, options: options)
    }
}

// ============================================================================
// MARK: - GF(16) arithmetic — mode message Reed-Solomon
// ============================================================================
//
// GF(16) is the finite field with 16 elements, constructed from the primitive
// polynomial:
//
//   p(x) = x^4 + x + 1   (binary 10011 = 0x13)
//
// The field has exactly 15 non-zero elements, each a power of the primitive
// element α. Because p(x) is primitive, α^15 = α^0 = 1 — so the powers cycle
// with period 15.
//
// Computing α^i iteratively starting from α^0 = 1:
//
//   i   α^i  binary   meaning
//   ─   ───  ──────   ─────────────────────────────────────────────
//   0   1    0001     x^4 = x + 1 is the reduction rule
//   1   2    0010
//   2   4    0100
//   3   8    1000
//   4   3    0011     (8 << 1 = 16 = 0b10000; 0b10000 XOR 0b10011 = 0b00011 = 3)
//   5   6    0110
//   6  12    1100
//   7  11    1011
//   8   5    0101
//   9  10    1010
//  10   7    0111
//  11  14    1110
//  12  15    1111
//  13  13    1101
//  14   9    1001
//  (15 = 0 wraps back to 1)
//
// GF16_ALOG[i] = α^i (antilogarithm / exponentiation table).
// GF16_LOG[e]  = i   where α^i = e   (discrete logarithm table).

/// GF(16) antilogarithm: `GF16_ALOG[i]` = α^i in GF(16)/0x13.
private let GF16_ALOG: [UInt8] = [
     1,  2,  4,  8,  3,  6, 12, 11,  5, 10,  7, 14, 15, 13,  9, 1,
]

/// GF(16) discrete logarithm: `GF16_LOG[e]` = i where α^i = e.
/// GF16_LOG[0] is unused (log(0) is undefined); set to 0xFF as sentinel.
private let GF16_LOG: [UInt8] = [
    0xFF,  // log(0)  = undefined
    0,     // log(1)  = 0   (α^0 = 1)
    1,     // log(2)  = 1
    4,     // log(3)  = 4
    2,     // log(4)  = 2
    8,     // log(5)  = 8
    5,     // log(6)  = 5
   10,     // log(7)  = 10
    3,     // log(8)  = 3
   14,     // log(9)  = 14
    9,     // log(10) = 9
    7,     // log(11) = 7
    6,     // log(12) = 6
   13,     // log(13) = 13
   11,     // log(14) = 11
   12,     // log(15) = 12
]

/// Multiply two GF(16) elements using log/antilog tables.
///
/// - Returns: `a * b` in GF(16).  Returns 0 if either operand is 0.
///
/// Formula: a * b = ALOG[(LOG[a] + LOG[b]) mod 15].
/// Division: a / b = ALOG[(LOG[a] - LOG[b] + 15) mod 15].
@inline(__always)
private func gf16Mul(_ a: UInt8, _ b: UInt8) -> UInt8 {
    if a == 0 || b == 0 { return 0 }
    let logSum = Int(GF16_LOG[Int(a)]) + Int(GF16_LOG[Int(b)])
    return GF16_ALOG[logSum % 15]
}

/// Build the GF(16) Reed-Solomon generator polynomial with roots α^1..α^n.
///
/// The generator polynomial is:
///   g(x) = (x + α^1)(x + α^2)...(x + α^n)
///
/// Coefficients are returned in increasing degree order: `[g_0, g_1, ..., g_n]`
/// where `g_n = 1` (the polynomial is monic).
///
/// In GF(2^m), addition and subtraction are both XOR, so the signs do not
/// matter — `(x + α^i)` is the same as `(x - α^i)`.
private func buildGF16Generator(_ n: Int) -> [UInt8] {
    var g: [UInt8] = [1]  // start with the constant polynomial 1
    for i in 1...n {
        let ai = GF16_ALOG[i % 15]    // α^i
        var next = [UInt8](repeating: 0, count: g.count + 1)
        for j in 0..<g.count {
            next[j + 1] ^= g[j]               // multiply by x
            next[j] ^= gf16Mul(ai, g[j])       // multiply by α^i (the root)
        }
        g = next
    }
    return g
}

/// Compute `n` GF(16) RS check nibbles for the given data nibbles.
///
/// Uses the LFSR (linear-feedback shift-register) polynomial division algorithm,
/// which is equivalent to computing the remainder of `data(x) * x^n` divided by
/// `g(x)`.
///
/// - Parameters:
///   - data:  Data nibbles (each in 0..15).
///   - n:     Number of ECC nibbles to produce.
/// - Returns: The `n` check nibbles appended after the data.
private func gf16RSEncode(_ data: [UInt8], checkCount n: Int) -> [UInt8] {
    let g = buildGF16Generator(n)
    var rem = [UInt8](repeating: 0, count: n)
    for byte in data {
        let fb = byte ^ rem[0]
        for i in 0..<(n - 1) {
            rem[i] = rem[i + 1] ^ gf16Mul(g[i + 1], fb)
        }
        rem[n - 1] = gf16Mul(g[n], fb)
    }
    return rem
}

// ============================================================================
// MARK: - GF(256)/0x12D arithmetic — 8-bit data codewords
// ============================================================================
//
// Aztec Code uses GF(256) with primitive polynomial:
//   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
//
// This is the SAME polynomial as Data Matrix ECC200, and DIFFERENT from
// QR Code (which uses 0x11D).  The two fields have the same number of elements
// but different multiplication tables — tables from QR Code cannot be reused.
//
// We implement GF(256)/0x12D inline rather than using the repo's GF256 package
// because that package uses 0x11D.  A two-table approach (EXP + LOG, each 256
// entries) makes multiplication O(1).
//
// Generator convention: b=1, roots α^1..α^n.  This is the MA02 convention
// used by Data Matrix — the same convention lets Aztec share the same RS logic.

/// GF(256)/0x12D exponentiation table (EXP_12D[i] = α^i), doubled to avoid
/// modulo in multiplication.  Entries 0..254 are α^0..α^254; entry 255 = α^0.
/// Entries 256..510 duplicate 0..254 so that EXP_12D[a+b] works for a,b<255.
///
/// This is a `let` constant — the table is built once at module load time and
/// never mutated, making it safe to read from any Swift concurrency context.
private let EXP_12D: [UInt8] = {
    var table = [UInt8](repeating: 0, count: 512)
    var x: Int = 1
    for i in 0..<255 {
        table[i] = UInt8(x)
        table[i + 255] = UInt8(x)
        var next = x << 1
        if next & 0x100 != 0 { next ^= 0x12D }
        x = next & 0xFF
    }
    table[255] = 1
    return table
}()

/// GF(256)/0x12D discrete logarithm table (LOG_12D[e] = i where α^i = e).
/// LOG_12D[0] is undefined — set to 0 but never used in correct code.
///
/// Also a `let` constant — read-only after initialisation.
private let LOG_12D: [UInt8] = {
    var table = [UInt8](repeating: 0, count: 256)
    var x: Int = 1
    for i in 0..<255 {
        table[x] = UInt8(i)
        var next = x << 1
        if next & 0x100 != 0 { next ^= 0x12D }
        x = next & 0xFF
    }
    return table
}()

/// Multiply two GF(256)/0x12D elements using log/antilog tables.
@inline(__always)
private func gf256Mul(_ a: UInt8, _ b: UInt8) -> UInt8 {
    if a == 0 || b == 0 { return 0 }
    return EXP_12D[Int(LOG_12D[Int(a)]) + Int(LOG_12D[Int(b)])]
}

/// Build the GF(256)/0x12D RS generator polynomial g(x) with roots α^1..α^n.
///
/// Returns big-endian coefficients (highest degree first): `[g_n, ..., g_1, g_0]`
/// where `g_n = 1`.
private func buildGF256Generator(_ n: Int) -> [UInt8] {
    var g: [UInt8] = [1]
    for i in 1...n {
        let ai = EXP_12D[i]     // α^i
        var next = [UInt8](repeating: 0, count: g.count + 1)
        for j in 0..<g.count {
            next[j] ^= g[j]
            next[j + 1] ^= gf256Mul(g[j], ai)
        }
        g = next
    }
    return g
}

/// Compute `nCheck` GF(256)/0x12D RS check bytes for the given data bytes.
///
/// Uses LFSR polynomial division (systematic encoding — same algorithm as
/// the Data Matrix MA02 Reed-Solomon encoder).
///
/// - Parameters:
///   - data:   Data bytes to protect.
///   - nCheck: Number of ECC bytes to produce.
/// - Returns:  `nCheck` ECC bytes.
private func gf256RSEncode(_ data: [UInt8], checkCount nCheck: Int) -> [UInt8] {
    let g = buildGF256Generator(nCheck)
    let n = g.count - 1
    var rem = [UInt8](repeating: 0, count: n)
    for byte in data {
        let fb = byte ^ rem[0]
        for i in 0..<(n - 1) {
            rem[i] = rem[i + 1] ^ gf256Mul(g[i + 1], fb)
        }
        rem[n - 1] = gf256Mul(g[n], fb)
    }
    return rem
}

// ============================================================================
// MARK: - Capacity tables
// ============================================================================
//
// These tables are derived from ISO/IEC 24778:2008 Table 1.  Each entry gives:
//
//   totalBits:  total available data+ECC bit positions in the symbol's data
//               layers (excluding bullseye, mode message band, orientation
//               marks, and reference grid lines).
//
//   maxBytes8:  maximum number of 8-bit codewords (totalBits / 8, rounded down).
//               Used for byte-mode capacity calculation in v0.1.0.
//
// Compact symbols have a 28-bit mode message ring; full symbols have 40 bits.
// These are already excluded from the totalBits figure.
//
// Index 0 in each table is unused (there is no "0-layer" symbol).

private struct LayerCapacity {
    let totalBits: Int    // total data+ECC bit slots
    let maxBytes8: Int    // floor(totalBits / 8) — for 8-bit codeword mode
}

private let COMPACT_CAPACITY: [LayerCapacity] = [
    LayerCapacity(totalBits:   0, maxBytes8:   0),  // [0] unused
    LayerCapacity(totalBits:  72, maxBytes8:   9),  // [1] 1 layer  15×15
    LayerCapacity(totalBits: 200, maxBytes8:  25),  // [2] 2 layers 19×19
    LayerCapacity(totalBits: 392, maxBytes8:  49),  // [3] 3 layers 23×23
    LayerCapacity(totalBits: 648, maxBytes8:  81),  // [4] 4 layers 27×27
]

private let FULL_CAPACITY: [LayerCapacity] = [
    LayerCapacity(totalBits:      0, maxBytes8:    0),  // [0]  unused
    LayerCapacity(totalBits:     88, maxBytes8:   11),  // [1]  1 layer
    LayerCapacity(totalBits:    216, maxBytes8:   27),  // [2]  2 layers
    LayerCapacity(totalBits:    360, maxBytes8:   45),  // [3]  3 layers
    LayerCapacity(totalBits:    520, maxBytes8:   65),  // [4]  4 layers
    LayerCapacity(totalBits:    696, maxBytes8:   87),  // [5]  5 layers
    LayerCapacity(totalBits:    888, maxBytes8:  111),  // [6]  6 layers
    LayerCapacity(totalBits:   1096, maxBytes8:  137),  // [7]  7 layers
    LayerCapacity(totalBits:   1320, maxBytes8:  165),  // [8]  8 layers
    LayerCapacity(totalBits:   1560, maxBytes8:  195),  // [9]  9 layers
    LayerCapacity(totalBits:   1816, maxBytes8:  227),  // [10] 10 layers
    LayerCapacity(totalBits:   2088, maxBytes8:  261),  // [11] 11 layers
    LayerCapacity(totalBits:   2376, maxBytes8:  297),  // [12] 12 layers
    LayerCapacity(totalBits:   2680, maxBytes8:  335),  // [13] 13 layers
    LayerCapacity(totalBits:   3000, maxBytes8:  375),  // [14] 14 layers
    LayerCapacity(totalBits:   3336, maxBytes8:  417),  // [15] 15 layers
    LayerCapacity(totalBits:   3688, maxBytes8:  461),  // [16] 16 layers
    LayerCapacity(totalBits:   4056, maxBytes8:  507),  // [17] 17 layers
    LayerCapacity(totalBits:   4440, maxBytes8:  555),  // [18] 18 layers
    LayerCapacity(totalBits:   4840, maxBytes8:  605),  // [19] 19 layers
    LayerCapacity(totalBits:   5256, maxBytes8:  657),  // [20] 20 layers
    LayerCapacity(totalBits:   5688, maxBytes8:  711),  // [21] 21 layers
    LayerCapacity(totalBits:   6136, maxBytes8:  767),  // [22] 22 layers
    LayerCapacity(totalBits:   6600, maxBytes8:  825),  // [23] 23 layers
    LayerCapacity(totalBits:   7080, maxBytes8:  885),  // [24] 24 layers
    LayerCapacity(totalBits:   7576, maxBytes8:  947),  // [25] 25 layers
    LayerCapacity(totalBits:   8088, maxBytes8: 1011),  // [26] 26 layers
    LayerCapacity(totalBits:   8616, maxBytes8: 1077),  // [27] 27 layers
    LayerCapacity(totalBits:   9160, maxBytes8: 1145),  // [28] 28 layers
    LayerCapacity(totalBits:   9720, maxBytes8: 1215),  // [29] 29 layers
    LayerCapacity(totalBits:  10296, maxBytes8: 1287),  // [30] 30 layers
    LayerCapacity(totalBits:  10888, maxBytes8: 1361),  // [31] 31 layers
    LayerCapacity(totalBits:  11496, maxBytes8: 1437),  // [32] 32 layers
]

// ============================================================================
// MARK: - Symbol specification
// ============================================================================

/// Describes a chosen Aztec Code symbol configuration.
private struct SymbolSpec {
    let compact:    Bool   // true = compact (≤4 layers); false = full (≤32 layers)
    let layers:     Int    // number of data layers
    let dataCwCount: Int   // number of 8-bit data codewords (excludes ECC)
    let eccCwCount:  Int   // number of 8-bit ECC codewords
    let totalBits:   Int   // total data+ECC bit capacity of the symbol
}

// ============================================================================
// MARK: - Data encoding: Binary-Shift from Upper mode
// ============================================================================
//
// All input is encoded as one Binary-Shift block from Upper mode.
//
// v0.1.0 note: This is the simplest valid encoding path — it works for
// any byte sequence.  Future versions will add true multi-mode encoding
// (Digit, Upper, Lower, Mixed, Punct) for smaller symbols.
//
// Binary-Shift encoding layout (bit-stream, MSB first):
//
//   1. Binary-Shift escape: 5 bits = 0b11111 (value 31 in Upper mode)
//   2. Length field:
//      - If byteCount ≤ 31: 5 bits for the count
//      - If byteCount >  31: 5 bits of 0b00000, then 11 bits for the count
//   3. Each byte: 8 bits, MSB first

/// Encode `input` bytes as a flat bit array using the Binary-Shift escape.
///
/// Returns an array of `UInt8` values that are always 0 or 1 (individual bits).
private func encodeBytesAsBits(_ input: [UInt8]) -> [UInt8] {
    var bits: [UInt8] = []
    bits.reserveCapacity(16 + input.count * 8)

    // Helper: write `count` bits from `value`, MSB first.
    func writeBits(_ value: Int, count: Int) {
        for shift in stride(from: count - 1, through: 0, by: -1) {
            bits.append(UInt8((value >> shift) & 1))
        }
    }

    let len = input.count
    writeBits(31, count: 5)   // Binary-Shift escape codeword in Upper mode

    if len <= 31 {
        writeBits(len, count: 5)
    } else {
        writeBits(0,   count: 5)    // 00000 signals "long length follows"
        writeBits(len, count: 11)
    }

    for byte in input {
        writeBits(Int(byte), count: 8)
    }

    return bits
}

// ============================================================================
// MARK: - Symbol size selection
// ============================================================================
//
// We select the smallest symbol (compact first, then full) whose capacity can
// hold the data bits plus the required ECC bits at the requested ECC level.
//
// A conservative 20% overhead is added to `dataBitCount` before comparison to
// account for bit-stuffing expansion (stuffing can expand the bit stream by at
// most 25% in the worst case; 20% is a safe margin for typical data).
//
// ECC ratio formula (target = minEccPct / 100):
//   eccCwCount = ceil(target * totalCwCount)
//   dataCwCount = totalCwCount - eccCwCount

/// Select the smallest Aztec Code symbol configuration for the given data.
///
/// - Parameters:
///   - dataBitCount: Number of raw data bits (before padding and stuffing).
///   - minEccPct:    Minimum error-correction percentage (10–90).
/// - Returns: The chosen `SymbolSpec`.
/// - Throws:  `AztecError.inputTooLong` if even a 32-layer full symbol cannot fit.
private func selectSymbol(dataBitCount: Int, minEccPct: Int) throws -> SymbolSpec {
    // Conservative stuffing overhead: multiply by 1.2 to reserve room for
    // the extra bits that bit-stuffing will insert after the RS step.
    let stuffedEstimate = Int((Double(dataBitCount) * 1.2).rounded(.up))

    func tryCapacity(compact: Bool, layers: Int, cap: LayerCapacity) -> SymbolSpec? {
        let totalCw = cap.maxBytes8
        guard totalCw > 0 else { return nil }
        let eccCw = Int((Double(minEccPct) / 100.0 * Double(totalCw)).rounded(.up))
        let dataCw = totalCw - eccCw
        guard dataCw > 0 else { return nil }
        // The stuffed estimate is in bits; divide by 8 (ceiling) to get bytes.
        let neededBytes = Int((Double(stuffedEstimate) / 8.0).rounded(.up))
        guard neededBytes <= dataCw else { return nil }
        return SymbolSpec(
            compact:     compact,
            layers:      layers,
            dataCwCount: dataCw,
            eccCwCount:  eccCw,
            totalBits:   cap.totalBits
        )
    }

    // Try compact layers 1–4 first (smallest symbols).
    for layers in 1...4 {
        let cap = COMPACT_CAPACITY[layers]
        if let spec = tryCapacity(compact: true, layers: layers, cap: cap) {
            return spec
        }
    }

    // Try full layers 1–32.
    for layers in 1...32 {
        let cap = FULL_CAPACITY[layers]
        if let spec = tryCapacity(compact: false, layers: layers, cap: cap) {
            return spec
        }
    }

    throw AztecError.inputTooLong(
        "\(dataBitCount) data bits cannot fit in any 32-layer full Aztec symbol"
    )
}

// ============================================================================
// MARK: - Padding
// ============================================================================
//
// After encoding the data bits, we pad out to exactly `targetBytes` 8-bit
// codewords:
//
//   1. Round the bit stream up to a multiple of 8 by appending 0 bits.
//   2. Append zero bytes until we reach `targetBytes`.
//   3. Truncate to exactly `targetBytes` bytes.
//
// All-zero codeword avoidance: if the LAST codeword is 0x00, replace it
// with 0xFF to prevent Reed-Solomon complications with zero-valued symbols.

/// Pad the bit stream to exactly `targetBytes` 8-bit codewords.
///
/// Returns the padded bytes as a `[UInt8]` array.
private func padToBytes(_ bits: [UInt8], targetBytes: Int) -> [UInt8] {
    var out = bits
    // Round up to byte boundary.
    while out.count % 8 != 0 { out.append(0) }
    // Extend to required byte count.
    while out.count < targetBytes * 8 { out.append(0) }
    // Truncate to exact size.
    out = Array(out.prefix(targetBytes * 8))

    // Convert bit array to byte array.
    var bytes = [UInt8](repeating: 0, count: targetBytes)
    for i in 0..<targetBytes {
        var byte: UInt8 = 0
        for b in 0..<8 {
            byte = (byte << 1) | (out[i * 8 + b] & 1)
        }
        bytes[i] = byte
    }

    // All-zero last codeword avoidance (see spec §3, step 3).
    if !bytes.isEmpty && bytes.last == 0 {
        bytes[bytes.count - 1] = 0xFF
    }

    return bytes
}

// ============================================================================
// MARK: - Bit stuffing
// ============================================================================
//
// Aztec Code applies a bit-stuffing rule to the final data+ECC bit stream
// BEFORE laying bits into the symbol grid.  This prevents degenerate runs of
// identical bits that could confuse the scanner's reference-grid detector.
//
// Rule: after every 4 consecutive identical bits, insert one complement bit.
//
// The complement bit resets the run counter, so the next group of 4 starts
// fresh.  Example:
//
//   Input:  0 0 0 0 1 1 1 1 1 0 1
//                ↑           ↑
//           run=4 → insert 1  run=4 (ones after stuff) → no extra run of 4 here
//   After 4×0: insert 1 → [0,0,0,0,1]  (run restarts from the stuffed 1)
//   After 4×1: insert 0 → [0,0,0,0,1, 1,1,1,1,0]  ...and so on
//
// Bit stuffing does NOT apply to the bullseye, orientation marks, mode message
// band, or reference grid — only to the data+ECC payload.
//
// The decoder reverses stuffing by removing the bit after every group of 4
// identical bits before running Reed-Solomon.

/// Apply Aztec Code bit stuffing to the data+ECC bit stream.
///
/// - Parameter bits: Array of bits (each 0 or 1) representing the data+ECC.
/// - Returns: Stuffed bit array (may be up to 25% longer in worst case).
private func stuffBits(_ bits: [UInt8]) -> [UInt8] {
    var stuffed: [UInt8] = []
    stuffed.reserveCapacity(Int(Double(bits.count) * 1.1))

    var runVal: Int = -1   // -1 means "no run started yet"
    var runLen: Int = 0

    for bit in bits {
        let bitInt = Int(bit)
        if bitInt == runVal {
            runLen += 1
        } else {
            runVal = bitInt
            runLen = 1
        }

        stuffed.append(bit)

        if runLen == 4 {
            // Insert complement bit and restart the run from it.
            let stuffBit = UInt8(1 - bitInt)
            stuffed.append(stuffBit)
            runVal = Int(stuffBit)
            runLen = 1
        }
    }

    return stuffed
}

// ============================================================================
// MARK: - Mode message encoding
// ============================================================================
//
// The mode message is Aztec Code's equivalent of QR Code's format information.
// It encodes the layer count and data codeword count, protected by GF(16) RS.
//
// Compact (28 bits = 7 nibbles):
//   Combined value m = ((layers - 1) << 6) | (dataCwCount - 1)  [8 bits]
//   Pack as 2 nibbles LSB-first, then compute 5 GF(16) ECC nibbles.
//   Flatten 7 nibbles to 28 bits, MSB-first per nibble.
//
// Full (40 bits = 10 nibbles):
//   Combined value m = ((layers - 1) << 11) | (dataCwCount - 1)  [16 bits]
//   Pack as 4 nibbles LSB-first, then compute 6 GF(16) ECC nibbles.
//   Flatten 10 nibbles to 40 bits.
//
// "LSB-first packing" means nibble[0] = m & 0xF, nibble[1] = (m >> 4) & 0xF, etc.
// Each nibble is then written MSB-first (bit 3 down to bit 0) into the bit stream.
//
// The 4 orientation mark corners in the mode message ring are fixed DARK — they
// are not part of the mode message content.

/// Encode the mode message into a flat bit array.
///
/// - Returns: 28-bit array for compact, 40-bit array for full.
private func encodeModeMessage(compact: Bool, layers: Int, dataCwCount: Int) -> [UInt8] {
    var dataNibbles: [UInt8]
    let numEcc: Int

    if compact {
        // 8-bit combined value packed into 2 nibbles.
        let m = ((layers - 1) << 6) | (dataCwCount - 1)
        dataNibbles = [
            UInt8(m & 0xF),
            UInt8((m >> 4) & 0xF),
        ]
        numEcc = 5
    } else {
        // 16-bit combined value packed into 4 nibbles.
        let m = ((layers - 1) << 11) | (dataCwCount - 1)
        dataNibbles = [
            UInt8(m & 0xF),
            UInt8((m >> 4) & 0xF),
            UInt8((m >> 8) & 0xF),
            UInt8((m >> 12) & 0xF),
        ]
        numEcc = 6
    }

    let eccNibbles = gf16RSEncode(dataNibbles, checkCount: numEcc)
    let allNibbles = dataNibbles + eccNibbles

    // Flatten each nibble to 4 bits, MSB first.
    var bits: [UInt8] = []
    bits.reserveCapacity(allNibbles.count * 4)
    for nibble in allNibbles {
        for shift in stride(from: 3, through: 0, by: -1) {
            bits.append(UInt8((Int(nibble) >> shift) & 1))
        }
    }

    return bits
}

// ============================================================================
// MARK: - Grid construction helpers
// ============================================================================

/// Compute the symbol size (side length in modules).
///
/// ```
/// compact: size = 11 + 4 * layers
/// full:    size = 15 + 4 * layers
/// ```
private func symbolSize(compact: Bool, layers: Int) -> Int {
    compact ? 11 + 4 * layers : 15 + 4 * layers
}

/// Compute the bullseye radius.
///
/// ```
/// compact bullseye: 11×11 modules → radius 5
/// full bullseye:    15×15 modules → radius 7
/// ```
private func bullseyeRadius(compact: Bool) -> Int {
    compact ? 5 : 7
}

// ============================================================================
// MARK: - Bullseye finder pattern
// ============================================================================
//
// The bullseye is a set of concentric square rings centered at (cx, cy).
// Each module's color is determined by its Chebyshev (L∞) distance from center:
//
//   d = max(|col - cx|, |row - cy|)
//
//   d == 0: DARK   (center pixel)
//   d == 1: DARK   (inner 3×3 core — rings 0 and 1 are both DARK)
//   d == 2: LIGHT
//   d == 3: DARK
//   d == 4: LIGHT
//   d == 5: DARK   ← outermost ring of compact bullseye
//   d == 6: LIGHT  ← (full only)
//   d == 7: DARK   ← outermost ring of full bullseye
//
// The scanner reads the bullseye by casting scan lines through the center.
// The pattern produces a 1:1:1:1:1 module-width ratio along any such scan line,
// which is a uniquely identifiable feature regardless of scale and rotation.
//
// Compact bullseye (11×11):
//
//   D D D D D D D D D D D
//   D L L L L L L L L L D
//   D L D D D D D D D L D
//   D L D L L L L L D L D
//   D L D L D D D L D L D
//   D L D L D D D L D L D   ← center row (d=0 at col 5)
//   D L D L D D D L D L D
//   D L D L L L L L D L D
//   D L D D D D D D D L D
//   D L L L L L L L L L D
//   D D D D D D D D D D D

/// Draw the bullseye finder pattern into `modules` and mark modules as reserved.
///
/// - Parameters:
///   - modules:  The grid to write to.
///   - reserved: Reservation map (true = module is not available for data).
///   - cx, cy:   Center coordinates.
///   - compact:  True for compact variant (radius 5), false for full (radius 7).
private func drawBullseye(
    modules: inout [[Bool]],
    reserved: inout [[Bool]],
    cx: Int, cy: Int,
    compact: Bool
) {
    let br = bullseyeRadius(compact: compact)
    for row in (cy - br)...(cy + br) {
        for col in (cx - br)...(cx + br) {
            let d = max(abs(col - cx), abs(row - cy))
            // Rings 0 and 1 are both DARK; rings 2, 4, 6 are LIGHT; rings 3, 5, 7 are DARK.
            let dark = d <= 1 || d % 2 == 1
            modules[row][col] = dark
            reserved[row][col] = true
        }
    }
}

// ============================================================================
// MARK: - Reference grid (full Aztec only)
// ============================================================================
//
// Full Aztec symbols include a reference grid — lines of alternating dark/light
// modules spaced every 16 modules from the center row and center column.  The
// grid helps scanners correct for severe perspective distortion by providing
// known calibration points throughout the symbol.
//
// Reference grid lines exist at:
//   rows: cy, cy±16, cy±32, cy±48, cy±64, ... (within symbol bounds)
//   cols: cx, cx±16, cx±32, cx±48, cx±64, ...
//
// Module value at (row, col) on a reference line:
//
//   Both on a horizontal AND vertical line → DARK (intersection)
//   Only on a horizontal line → alternates with (cx - col) mod 2 == 0 → DARK
//   Only on a vertical line   → alternates with (cy - row) mod 2 == 0 → DARK
//
// Compact symbols have no reference grid.

/// Draw reference grid lines for full Aztec symbols.
private func drawReferenceGrid(
    modules: inout [[Bool]],
    reserved: inout [[Bool]],
    cx: Int, cy: Int,
    size: Int
) {
    for row in 0..<size {
        for col in 0..<size {
            let onH = (cy - row) % 16 == 0   // on a horizontal reference line
            let onV = (cx - col) % 16 == 0   // on a vertical reference line
            guard onH || onV else { continue }

            let dark: Bool
            if onH && onV {
                dark = true                           // intersection → always DARK
            } else if onH {
                dark = (cx - col) % 2 == 0           // horizontal line alternates from center col
            } else {
                dark = (cy - row) % 2 == 0           // vertical line alternates from center row
            }

            modules[row][col] = dark
            reserved[row][col] = true
        }
    }
}

// ============================================================================
// MARK: - Orientation marks and mode message ring
// ============================================================================
//
// The ring immediately outside the bullseye (at Chebyshev radius bullseyeRadius+1)
// is called the "mode message ring".  It carries:
//
//   - 4 orientation marks: the four CORNER positions of this ring are always DARK.
//     These break the rotational symmetry and let the scanner determine which of
//     the four 90° orientations the symbol is in.
//
//   - Mode message bits: the remaining non-corner perimeter positions carry the
//     28-bit (compact) or 40-bit (full) mode message, written clockwise starting
//     just right of the top-left corner.
//
//   - Leading data bits: positions after the mode message are filled by the start
//     of the data bit stream.  The mode ring and data layers share this ring.
//
// Perimeter of a (2r+1)×(2r+1) square (excluding corners):
//   4 × (2r - 1) non-corner edge positions
//   + 4 corners (orientation marks)
//   = 4 × (2r) total perimeter positions
//
// Compact: r = 6, non-corner = 44, mode msg = 28 bits, remaining = 16 data bits.
// Full:    r = 8, non-corner = 60, mode msg = 40 bits, remaining = 20 data bits.
//
// Clockwise order starting from (cx - r + 1, cy - r) (first non-corner on top edge):
//   top edge:    col from (cx-r+1) to (cx+r-1), row = cy-r
//   right edge:  row from (cy-r+1) to (cy+r-1), col = cx+r
//   bottom edge: col from (cx+r-1) to (cx-r+1), row = cy+r   (right-to-left)
//   left edge:   row from (cy+r-1) to (cy-r+1), col = cx-r   (bottom-to-top)

/// Draw orientation marks and place mode message bits.
///
/// Returns the (col, row) pairs of positions in the mode ring that remain
/// after placing the mode message — these will be filled by the first few bits
/// of the data stream.
@discardableResult
private func drawOrientationAndModeMessage(
    modules: inout [[Bool]],
    reserved: inout [[Bool]],
    cx: Int, cy: Int,
    compact: Bool,
    modeMessageBits: [UInt8]
) -> [(col: Int, row: Int)] {
    let r = bullseyeRadius(compact: compact) + 1

    // Enumerate non-corner perimeter positions clockwise.
    var nonCorner: [(col: Int, row: Int)] = []

    // Top edge (excluding corners): left → right
    for col in (cx - r + 1)...(cx + r - 1) {
        nonCorner.append((col: col, row: cy - r))
    }
    // Right edge (excluding corners): top → bottom
    for row in (cy - r + 1)...(cy + r - 1) {
        nonCorner.append((col: cx + r, row: row))
    }
    // Bottom edge (excluding corners): right → left
    for col in stride(from: cx + r - 1, through: cx - r + 1, by: -1) {
        nonCorner.append((col: col, row: cy + r))
    }
    // Left edge (excluding corners): bottom → top
    for row in stride(from: cy + r - 1, through: cy - r + 1, by: -1) {
        nonCorner.append((col: cx - r, row: row))
    }

    // Place orientation marks (4 corners, always DARK).
    let corners: [(col: Int, row: Int)] = [
        (col: cx - r, row: cy - r),  // top-left
        (col: cx + r, row: cy - r),  // top-right
        (col: cx + r, row: cy + r),  // bottom-right
        (col: cx - r, row: cy + r),  // bottom-left
    ]
    for pos in corners {
        modules[pos.row][pos.col] = true
        reserved[pos.row][pos.col] = true
    }

    // Place mode message bits into the first `modeMessageBits.count` non-corner positions.
    for (i, pos) in nonCorner.prefix(modeMessageBits.count).enumerated() {
        modules[pos.row][pos.col] = modeMessageBits[i] == 1
        reserved[pos.row][pos.col] = true
    }

    // Return the remaining positions for data bits.
    return Array(nonCorner.dropFirst(modeMessageBits.count))
}

// ============================================================================
// MARK: - Data layer spiral placement
// ============================================================================
//
// After the mode message ring, bits are placed in a clockwise spiral through
// the data layers, from innermost to outermost.  Each layer is a 2-module-wide
// band.
//
// Layer geometry:
//   Compact: first data layer (L=1) has inner radius d_inner = bullseyeRadius + 2 = 7
//   Full:    first data layer (L=1) has inner radius d_inner = bullseyeRadius + 2 = 9
//
//   Each successive layer L adds 2 to d_inner: d_inner(L) = d_inner(1) + 2*(L-1)
//   The outer radius of each layer: d_outer = d_inner + 1
//
// Within a single layer at (d_inner, d_outer), bits are placed in clockwise order,
// with pairs: outer then inner.
//
// Top edge (left → right, skipping the top-left corner):
//   for col in (cx - d_inner + 1)...(cx + d_inner):
//     place (col, cy - d_outer)   outer row
//     place (col, cy - d_inner)   inner row
//
// Right edge (top → bottom, skipping the top-right and bottom-right corners):
//   for row in (cy - d_inner + 1)...(cy + d_inner):
//     place (cx + d_outer, row)   outer col
//     place (cx + d_inner, row)   inner col
//
// Bottom edge (right → left):
//   for col in stride(cx + d_inner, cx - d_inner + 1, -1):
//     place (col, cy + d_outer)   outer row
//     place (col, cy + d_inner)   inner row
//
// Left edge (bottom → top):
//   for row in stride(cy + d_inner, cy - d_inner + 1, -1):
//     place (cx - d_outer, row)   outer col
//     place (cx - d_inner, row)   inner col
//
// Modules that are already reserved (bullseye, mode message, reference grid)
// are skipped; the bit index advances only when a bit is actually written.

/// Place all data+ECC bits using the clockwise layer spiral.
///
/// - Parameters:
///   - modules:      The grid to write to.
///   - reserved:     Reservation map (modules already occupied by structural data).
///   - bits:         Stuffed data+ECC bit stream.
///   - cx, cy:       Center coordinates.
///   - compact:      True for compact, false for full.
///   - layers:       Number of data layers.
///   - modeRingRemainder: Positions in the mode ring after the mode message.
private func placeDataBits(
    modules: inout [[Bool]],
    reserved: inout [[Bool]],
    bits: [UInt8],
    cx: Int, cy: Int,
    compact: Bool,
    layers: Int,
    modeRingRemainder: [(col: Int, row: Int)]
) {
    let size = modules.count
    var bitIndex = 0

    // Place a single bit at (col, row), skipping reserved cells.
    // The bit index only advances when a cell is written.
    func placeBit(col: Int, row: Int) {
        guard row >= 0, row < size, col >= 0, col < size else { return }
        guard !reserved[row][col] else { return }
        modules[row][col] = bitIndex < bits.count ? bits[bitIndex] == 1 : false
        bitIndex += 1
    }

    // Fill the mode ring remainder first (data bits 0..<modeRingRemainder.count).
    for pos in modeRingRemainder {
        guard pos.row >= 0, pos.row < size, pos.col >= 0, pos.col < size else { continue }
        modules[pos.row][pos.col] = bitIndex < bits.count ? bits[bitIndex] == 1 : false
        bitIndex += 1
    }

    // Spiral through data layers, innermost first.
    let br = bullseyeRadius(compact: compact)
    let dStart = br + 2   // mode message ring is at br+1; first data layer starts at br+2

    for L in 0..<layers {
        let dI = dStart + 2 * L   // inner radius of this layer
        let dO = dI + 1            // outer radius of this layer

        // Top edge: left to right (skip the two corner positions handled by adjacent edges)
        for col in (cx - dI + 1)...(cx + dI) {
            placeBit(col: col, row: cy - dO)   // outer row
            placeBit(col: col, row: cy - dI)   // inner row
        }
        // Right edge: top to bottom
        for row in (cy - dI + 1)...(cy + dI) {
            placeBit(col: cx + dO, row: row)   // outer col
            placeBit(col: cx + dI, row: row)   // inner col
        }
        // Bottom edge: right to left
        for col in stride(from: cx + dI, through: cx - dI + 1, by: -1) {
            placeBit(col: col, row: cy + dO)   // outer row
            placeBit(col: col, row: cy + dI)   // inner row
        }
        // Left edge: bottom to top
        for row in stride(from: cy + dI, through: cy - dI + 1, by: -1) {
            placeBit(col: cx - dO, row: row)   // outer col
            placeBit(col: cx - dI, row: row)   // inner col
        }
    }
}

// ============================================================================
// MARK: - Main encode function (internal)
// ============================================================================

// ============================================================================
// MARK: - Internal test bridges
// ============================================================================
//
// These internal functions expose private implementation details so unit tests
// can verify each layer of the encoding pipeline independently.  They use the
// `internal` access level (the default in Swift) so that `@testable import`
// makes them visible to test targets, while still being hidden from external
// package consumers.

/// Test bridge: encode mode message bits (wraps `encodeModeMessage`).
func _encodeModeMessageBridge(compact: Bool, layers: Int, dataCwCount: Int) -> [UInt8] {
    return encodeModeMessage(compact: compact, layers: layers, dataCwCount: dataCwCount)
}

/// Test bridge: GF(256)/0x12D RS encoding (wraps `gf256RSEncode`).
func _gf256RSEncodeBridge(_ data: [UInt8], checkCount: Int) -> [UInt8] {
    return gf256RSEncode(data, checkCount: checkCount)
}

/// Test bridge: bit stuffing (wraps `stuffBits`).
func _stuffBitsBridge(_ bits: [UInt8]) -> [UInt8] {
    return stuffBits(bits)
}

// ============================================================================
// MARK: - Main encode function (internal)
// ============================================================================

/// Internal encode implementation that returns a `ModuleGrid`.
private func encodeBytes(_ input: [UInt8], options: AztecOptions) throws -> ModuleGrid {
    let minEccPct = options.minEccPercent

    // ─── Step 1: Encode data as Binary-Shift bits ─────────────────────────
    //
    // All input is wrapped in a Binary-Shift block from Upper mode.
    // This always produces valid Aztec codewords, though not minimal-size
    // for text that could be encoded in Digit or Upper mode.  v0.2.0 will
    // add true multi-mode optimisation.

    let dataBits = encodeBytesAsBits(input)

    // ─── Step 2: Select the smallest symbol ───────────────────────────────

    let spec = try selectSymbol(dataBitCount: dataBits.count, minEccPct: minEccPct)

    // ─── Step 3: Pad data codewords ────────────────────────────────────────
    //
    // Pad to exactly spec.dataCwCount bytes (zero-pad bit stream, then
    // convert to bytes, with all-zero last byte avoidance).

    let dataBytes = padToBytes(dataBits, targetBytes: spec.dataCwCount)

    // ─── Step 4: Compute GF(256)/0x12D Reed-Solomon ECC ───────────────────

    let eccBytes = gf256RSEncode(dataBytes, checkCount: spec.eccCwCount)

    // ─── Step 5: Build bit stream and apply bit stuffing ──────────────────

    var rawBits: [UInt8] = []
    rawBits.reserveCapacity((dataBytes.count + eccBytes.count) * 8)
    for byte in dataBytes + eccBytes {
        for shift in stride(from: 7, through: 0, by: -1) {
            rawBits.append(UInt8((Int(byte) >> shift) & 1))
        }
    }
    let stuffedBits = stuffBits(rawBits)

    // ─── Step 6: Compute mode message ─────────────────────────────────────

    let modeMsg = encodeModeMessage(
        compact:     spec.compact,
        layers:      spec.layers,
        dataCwCount: spec.dataCwCount
    )

    // ─── Step 7: Initialise the symbol grid ───────────────────────────────

    let size = symbolSize(compact: spec.compact, layers: spec.layers)
    let cx   = size / 2    // center column (size is always odd)
    let cy   = size / 2    // center row

    var modules  = [[Bool]](repeating: [Bool](repeating: false, count: size), count: size)
    var reserved = [[Bool]](repeating: [Bool](repeating: false, count: size), count: size)

    // Reference grid FIRST for full symbols (bullseye will overwrite the center).
    if !spec.compact {
        drawReferenceGrid(modules: &modules, reserved: &reserved, cx: cx, cy: cy, size: size)
    }

    // Bullseye overwrites the reference grid center (correct — bullseye is master).
    drawBullseye(modules: &modules, reserved: &reserved, cx: cx, cy: cy, compact: spec.compact)

    // Orientation marks + mode message.
    let modeRingRemainder = drawOrientationAndModeMessage(
        modules: &modules, reserved: &reserved,
        cx: cx, cy: cy,
        compact: spec.compact,
        modeMessageBits: modeMsg
    )

    // ─── Step 8: Place data+ECC bits in the clockwise spiral ──────────────

    placeDataBits(
        modules:  &modules,
        reserved: &reserved,
        bits:     stuffedBits,
        cx: cx, cy: cy,
        compact:  spec.compact,
        layers:   spec.layers,
        modeRingRemainder: modeRingRemainder
    )

    // ─── Build ModuleGrid ──────────────────────────────────────────────────

    return ModuleGrid(
        cols:        size,
        rows:        size,
        modules:     modules,
        moduleShape: .square
    )
}
