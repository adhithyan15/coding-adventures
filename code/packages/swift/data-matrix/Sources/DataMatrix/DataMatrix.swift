// DataMatrix.swift — ISO/IEC 16022:2006 ECC200 Data Matrix encoder
//
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Overview
// ============================================================================
//
// Data Matrix is a two-dimensional matrix barcode invented in 1989 (originally
// "DataCode") and standardised as ISO/IEC 16022:2006. ECC200 is the modern
// variant — using Reed-Solomon over GF(256) — that has displaced the older
// ECC000–ECC140 lineage.
//
// ## Where Data Matrix is used
//
//   - PCBs        — every modern board carries a tiny Data Matrix etched on the
//                   substrate for traceability through automated assembly lines.
//   - Pharma      — the US FDA DSCSA mandates Data Matrix on unit-dose packages.
//   - Aerospace   — etched / dot-peened marks survive decades of heat and
//                   abrasion that would destroy ink-printed labels.
//   - Medical     — GS1 DataMatrix on surgical instruments and implants.
//   - Postage     — USPS registered mail and customs forms.
//
// ## Key differences from QR Code
//
// ```
// ┌────────────────┬────────────────────┬───────────────────────┐
// │ Property       │ QR Code            │ Data Matrix ECC200    │
// ├────────────────┼────────────────────┼───────────────────────┤
// │ GF(256) poly   │ 0x11D              │ 0x12D                 │
// │ RS root start  │ b = 0 (α⁰..)       │ b = 1 (α¹..)          │
// │ Finder         │ three corner       │ one L-shape           │
// │                │ squares            │ (left + bottom)       │
// │ Placement      │ column zigzag      │ "Utah" diagonal       │
// │ Masking        │ 8 patterns,        │ NONE                  │
// │                │ penalty-scored     │                       │
// │ Sizes          │ 40 versions        │ 30 square + 6 rect    │
// └────────────────┴────────────────────┴───────────────────────┘
// ```
//
// ## Encoding pipeline
//
// ```
// input string
//   → ASCII encoding      (chars+1; digit pairs packed into one codeword)
//   → symbol selection    (smallest symbol whose capacity ≥ codeword count)
//   → pad to capacity     (scrambled-pad codewords fill unused slots)
//   → RS blocks + ECC     (GF(256)/0x12D, b=1 convention)
//   → interleave blocks   (data round-robin then ECC round-robin)
//   → grid init           (L-finder + timing border + alignment borders)
//   → Utah placement      (diagonal codeword placement, NO masking)
//   → ModuleGrid          (abstract boolean grid, true = dark)
// ```
//
// ============================================================================

import Barcode2D
import GF256
import PaintInstructions

// ============================================================================
// MARK: - Version
// ============================================================================

/// Current package version.
public let dataMatrixVersion = "0.1.0"

// ============================================================================
// MARK: - GF(256)/0x12D primitive polynomial constant
// ============================================================================

/// Primitive polynomial used by Data Matrix ECC200: p(x) = x⁸ + x⁵ + x⁴ + x² + x + 1.
///
/// In binary: 1_0010_1101 = 0x12D = 301 decimal.
///
/// IMPORTANT: This is DIFFERENT from QR Code's 0x11D polynomial. Both are
/// degree-8 irreducible polynomials over GF(2), but the resulting fields are
/// not interchangeable — never mix tables between QR and Data Matrix.
///
/// Why this polynomial?
/// ISO/IEC 16022:2006 mandates 0x12D as the irreducible polynomial for
/// ECC200. The choice affects every Reed-Solomon codeword and therefore the
/// bit pattern of every encoded symbol.
public let gf256Prime = 0x12D

// ============================================================================
// MARK: - SymbolShape
// ============================================================================

/// Controls which symbol shapes are considered during auto-selection.
///
/// Data Matrix ECC200 defines two families of symbols:
///
/// - **Square**: 24 sizes from 10×10 to 144×144. The overwhelmingly common
///   variant. Square symbols handle the widest range of data types and are
///   used on PCBs, pharmaceuticals, and aerospace parts.
///
/// - **Rectangle**: 6 sizes from 8×18 to 16×48. Useful when the print area
///   has a constrained aspect ratio — for example, a long thin label on a
///   cable wrap or a narrow label on a syringe.
///
/// - **Any**: considers both shapes and picks whichever produces the smallest
///   total module count for the input (ties broken by area).
public enum SymbolShape: Sendable {
    /// Consider only the 24 square symbol sizes (10×10 … 144×144).
    case square
    /// Consider only the 6 rectangular symbol sizes (8×18 … 16×48).
    case rectangle
    /// Consider all 30 symbol sizes and pick the smallest that fits.
    case any
}

// ============================================================================
// MARK: - DataMatrixOptions
// ============================================================================

/// Configuration options for the Data Matrix encoder.
///
/// All fields have sensible defaults — `DataMatrixOptions()` works for most uses.
///
/// ```swift
/// // Auto-select the smallest square symbol:
/// let grid = try encode("Hello", options: DataMatrixOptions())
///
/// // Force a specific size:
/// var opts = DataMatrixOptions()
/// opts.size = 18   // use 18×18
/// let grid = try encode("Hello", options: opts)
/// ```
public struct DataMatrixOptions: Sendable {

    /// Force a specific square symbol side-length in modules (e.g. 10, 12, 14…).
    ///
    /// When `nil` (the default), the encoder selects the smallest symbol that
    /// can hold the encoded data, filtered by `shape`.
    ///
    /// For rectangular symbols you cannot use this field directly — use
    /// `encode(_:rows:cols:)` instead.
    public var size: Int? = nil

    /// Which shape family to consider during auto-selection.
    ///
    /// Ignored when `size` is non-nil (explicit size implies the shape).
    public var shape: SymbolShape = .square

    /// Create options with default values.
    public init() {}
}

// ============================================================================
// MARK: - DataMatrixError
// ============================================================================

/// Errors thrown by the Data Matrix encoder.
///
/// Catch `DataMatrixError` to handle any encoder error regardless of subtype.
public enum DataMatrixError: Error {

    /// Input encodes to more codewords than the largest fitting symbol can hold.
    ///
    /// The largest ECC200 symbol is 144×144, holding at most 1558 data
    /// codewords. Consider splitting the data across multiple symbols
    /// (Structured Append) or switching to a different barcode format (e.g.
    /// PDF417 or Aztec Code, which handle longer payloads).
    case inputTooLong(String)

    /// The requested explicit symbol size does not match any ECC200 size.
    ///
    /// Valid square sizes: 10, 12, 14, 16, 18, 20, 22, 24, 26, 32, 36, 40,
    /// 44, 48, 52, 64, 72, 80, 88, 96, 104, 120, 132, 144.
    ///
    /// Valid rectangular sizes (rows × cols):
    /// 8×18, 8×32, 12×26, 12×36, 16×36, 16×48.
    case invalidSize(String)
}

// ============================================================================
// MARK: - Symbol size table — ISO/IEC 16022:2006 Table 7
// ============================================================================
//
// Every Data Matrix ECC200 symbol decomposes as:
//
//   symbol = outer_border + (regionRows × regionCols) data regions
//
// Each data region is (dataRegionHeight × dataRegionWidth) modules of pure
// data. Regions are separated by 2-module alignment borders, and the whole
// symbol is wrapped in a 1-module finder/timing border.
//
// The Utah placement algorithm scans the *logical* grid — the concatenation
// of all data region interiors — then we map back to physical coordinates.

/// One Data Matrix ECC200 symbol size and its capacity parameters.
///
/// The fields correspond exactly to the columns in ISO/IEC 16022:2006 Table 7.
/// Having them all in one struct makes the symbol-selection step a simple
/// linear scan with no arithmetic.
private struct SymbolEntry: Sendable {
    /// Total symbol size in modules (including the outer border).
    let symbolRows: Int
    let symbolCols: Int
    /// Number of data region rows / cols (regionRows × regionCols).
    /// Single-region symbols have `1, 1`; the largest 144×144 has `6, 6`.
    let regionRows: Int
    let regionCols: Int
    /// Interior data size per region (excludes alignment borders).
    let dataRegionHeight: Int
    let dataRegionWidth: Int
    /// Total data codeword capacity for this symbol.
    let dataCW: Int
    /// Total ECC codewords appended after data (sum across all blocks).
    let eccCW: Int
    /// Number of interleaved Reed-Solomon blocks.
    let numBlocks: Int
    /// ECC codewords per block (identical for all blocks in one symbol).
    let eccPerBlock: Int
}

// The 24 square symbol sizes from ISO/IEC 16022:2006, Table 7, ascending.
// Fields: symbolRows, symbolCols, regionRows, regionCols,
//         dataRegionHeight, dataRegionWidth, dataCW, eccCW, numBlocks, eccPerBlock
private let squareSizes: [SymbolEntry] = [
    SymbolEntry(symbolRows: 10,  symbolCols: 10,  regionRows: 1, regionCols: 1, dataRegionHeight: 8,  dataRegionWidth: 8,  dataCW: 3,    eccCW: 5,   numBlocks: 1,  eccPerBlock: 5),
    SymbolEntry(symbolRows: 12,  symbolCols: 12,  regionRows: 1, regionCols: 1, dataRegionHeight: 10, dataRegionWidth: 10, dataCW: 5,    eccCW: 7,   numBlocks: 1,  eccPerBlock: 7),
    SymbolEntry(symbolRows: 14,  symbolCols: 14,  regionRows: 1, regionCols: 1, dataRegionHeight: 12, dataRegionWidth: 12, dataCW: 8,    eccCW: 10,  numBlocks: 1,  eccPerBlock: 10),
    SymbolEntry(symbolRows: 16,  symbolCols: 16,  regionRows: 1, regionCols: 1, dataRegionHeight: 14, dataRegionWidth: 14, dataCW: 12,   eccCW: 12,  numBlocks: 1,  eccPerBlock: 12),
    SymbolEntry(symbolRows: 18,  symbolCols: 18,  regionRows: 1, regionCols: 1, dataRegionHeight: 16, dataRegionWidth: 16, dataCW: 18,   eccCW: 14,  numBlocks: 1,  eccPerBlock: 14),
    SymbolEntry(symbolRows: 20,  symbolCols: 20,  regionRows: 1, regionCols: 1, dataRegionHeight: 18, dataRegionWidth: 18, dataCW: 22,   eccCW: 18,  numBlocks: 1,  eccPerBlock: 18),
    SymbolEntry(symbolRows: 22,  symbolCols: 22,  regionRows: 1, regionCols: 1, dataRegionHeight: 20, dataRegionWidth: 20, dataCW: 30,   eccCW: 20,  numBlocks: 1,  eccPerBlock: 20),
    SymbolEntry(symbolRows: 24,  symbolCols: 24,  regionRows: 1, regionCols: 1, dataRegionHeight: 22, dataRegionWidth: 22, dataCW: 36,   eccCW: 24,  numBlocks: 1,  eccPerBlock: 24),
    SymbolEntry(symbolRows: 26,  symbolCols: 26,  regionRows: 1, regionCols: 1, dataRegionHeight: 24, dataRegionWidth: 24, dataCW: 44,   eccCW: 28,  numBlocks: 1,  eccPerBlock: 28),
    SymbolEntry(symbolRows: 32,  symbolCols: 32,  regionRows: 2, regionCols: 2, dataRegionHeight: 14, dataRegionWidth: 14, dataCW: 62,   eccCW: 36,  numBlocks: 2,  eccPerBlock: 18),
    SymbolEntry(symbolRows: 36,  symbolCols: 36,  regionRows: 2, regionCols: 2, dataRegionHeight: 16, dataRegionWidth: 16, dataCW: 86,   eccCW: 42,  numBlocks: 2,  eccPerBlock: 21),
    SymbolEntry(symbolRows: 40,  symbolCols: 40,  regionRows: 2, regionCols: 2, dataRegionHeight: 18, dataRegionWidth: 18, dataCW: 114,  eccCW: 48,  numBlocks: 2,  eccPerBlock: 24),
    SymbolEntry(symbolRows: 44,  symbolCols: 44,  regionRows: 2, regionCols: 2, dataRegionHeight: 20, dataRegionWidth: 20, dataCW: 144,  eccCW: 56,  numBlocks: 4,  eccPerBlock: 14),
    SymbolEntry(symbolRows: 48,  symbolCols: 48,  regionRows: 2, regionCols: 2, dataRegionHeight: 22, dataRegionWidth: 22, dataCW: 174,  eccCW: 68,  numBlocks: 4,  eccPerBlock: 17),
    SymbolEntry(symbolRows: 52,  symbolCols: 52,  regionRows: 2, regionCols: 2, dataRegionHeight: 24, dataRegionWidth: 24, dataCW: 204,  eccCW: 84,  numBlocks: 4,  eccPerBlock: 21),
    SymbolEntry(symbolRows: 64,  symbolCols: 64,  regionRows: 4, regionCols: 4, dataRegionHeight: 14, dataRegionWidth: 14, dataCW: 280,  eccCW: 112, numBlocks: 4,  eccPerBlock: 28),
    SymbolEntry(symbolRows: 72,  symbolCols: 72,  regionRows: 4, regionCols: 4, dataRegionHeight: 16, dataRegionWidth: 16, dataCW: 368,  eccCW: 144, numBlocks: 4,  eccPerBlock: 36),
    SymbolEntry(symbolRows: 80,  symbolCols: 80,  regionRows: 4, regionCols: 4, dataRegionHeight: 18, dataRegionWidth: 18, dataCW: 456,  eccCW: 192, numBlocks: 4,  eccPerBlock: 48),
    SymbolEntry(symbolRows: 88,  symbolCols: 88,  regionRows: 4, regionCols: 4, dataRegionHeight: 20, dataRegionWidth: 20, dataCW: 576,  eccCW: 224, numBlocks: 4,  eccPerBlock: 56),
    SymbolEntry(symbolRows: 96,  symbolCols: 96,  regionRows: 4, regionCols: 4, dataRegionHeight: 22, dataRegionWidth: 22, dataCW: 696,  eccCW: 272, numBlocks: 4,  eccPerBlock: 68),
    SymbolEntry(symbolRows: 104, symbolCols: 104, regionRows: 4, regionCols: 4, dataRegionHeight: 24, dataRegionWidth: 24, dataCW: 816,  eccCW: 336, numBlocks: 6,  eccPerBlock: 56),
    SymbolEntry(symbolRows: 120, symbolCols: 120, regionRows: 6, regionCols: 6, dataRegionHeight: 18, dataRegionWidth: 18, dataCW: 1050, eccCW: 408, numBlocks: 6,  eccPerBlock: 68),
    SymbolEntry(symbolRows: 132, symbolCols: 132, regionRows: 6, regionCols: 6, dataRegionHeight: 20, dataRegionWidth: 20, dataCW: 1304, eccCW: 496, numBlocks: 8,  eccPerBlock: 62),
    SymbolEntry(symbolRows: 144, symbolCols: 144, regionRows: 6, regionCols: 6, dataRegionHeight: 22, dataRegionWidth: 22, dataCW: 1558, eccCW: 620, numBlocks: 10, eccPerBlock: 62),
]

// The 6 rectangular symbol sizes from ISO/IEC 16022:2006, Table 7.
private let rectSizes: [SymbolEntry] = [
    SymbolEntry(symbolRows: 8,  symbolCols: 18, regionRows: 1, regionCols: 1, dataRegionHeight: 6,  dataRegionWidth: 16, dataCW: 5,  eccCW: 7,  numBlocks: 1, eccPerBlock: 7),
    SymbolEntry(symbolRows: 8,  symbolCols: 32, regionRows: 1, regionCols: 2, dataRegionHeight: 6,  dataRegionWidth: 14, dataCW: 10, eccCW: 11, numBlocks: 1, eccPerBlock: 11),
    SymbolEntry(symbolRows: 12, symbolCols: 26, regionRows: 1, regionCols: 1, dataRegionHeight: 10, dataRegionWidth: 24, dataCW: 16, eccCW: 14, numBlocks: 1, eccPerBlock: 14),
    SymbolEntry(symbolRows: 12, symbolCols: 36, regionRows: 1, regionCols: 2, dataRegionHeight: 10, dataRegionWidth: 16, dataCW: 22, eccCW: 18, numBlocks: 1, eccPerBlock: 18),
    SymbolEntry(symbolRows: 16, symbolCols: 36, regionRows: 1, regionCols: 2, dataRegionHeight: 14, dataRegionWidth: 16, dataCW: 32, eccCW: 24, numBlocks: 1, eccPerBlock: 24),
    SymbolEntry(symbolRows: 16, symbolCols: 48, regionRows: 1, regionCols: 2, dataRegionHeight: 14, dataRegionWidth: 22, dataCW: 49, eccCW: 28, numBlocks: 1, eccPerBlock: 28),
]

/// Largest data codeword capacity across all ECC200 symbols.
/// Used for error message generation.
private let maxDataCW = 1558

// ============================================================================
// MARK: - GF(256)/0x12D — Data Matrix field tables
// ============================================================================
//
// Data Matrix uses GF(256) with the primitive polynomial 0x12D:
//
//   p(x) = x⁸ + x⁵ + x⁴ + x² + x + 1  =  0x12D  =  301 decimal
//
// IMPORTANT: this is DIFFERENT from QR Code's 0x11D polynomial. Both are
// degree-8 irreducible polynomials over GF(2), but the resulting fields are
// non-isomorphic — never mix tables between QR and Data Matrix encoders.
//
// We precompute exp/log tables for fast O(1) field multiplication.
// The generator g = 2 (polynomial x) is primitive: raising it to powers
// 0..254 produces every non-zero element of the field exactly once.
//
// Algorithm for building tables:
//   Start with val = 1 (= α⁰).
//   Each step: val <<= 1 (multiply by α = x in polynomial form).
//   If bit 8 is set (val ≥ 256), XOR with 0x12D to reduce mod p(x).

/// Build the (exp, log) lookup tables for GF(256)/0x12D.
///
/// - Returns: A tuple where:
///   - `exp[i]` = αⁱ for i in 0..254, and exp[255] = 1 (wraps to α⁰)
///   - `log[v]` = k such that αᵏ = v (log[0] is unused / undefined)
private func buildDMTables() -> (exp: [Int], log: [Int]) {
    var expTable = [Int](repeating: 0, count: 256)
    var logTable = [Int](repeating: 0, count: 256)
    var val = 1
    for i in 0..<255 {
        expTable[i] = val
        logTable[val] = i
        val <<= 1
        if val & 0x100 != 0 {
            // Reduce modulo 0x12D.
            // XOR clears bit 8 and applies the polynomial reduction:
            // x⁸ → x⁵ + x⁴ + x² + x + 1 (the lower 8 bits of 0x12D).
            val ^= 0x12D
        }
    }
    // α²⁵⁵ = α⁰ = 1: the multiplicative group has order 255.
    // Storing 1 at index 255 makes inverse(1) work correctly.
    expTable[255] = expTable[0]
    return (exp: expTable, log: logTable)
}

/// Pre-built (exp, log) tables for GF(256)/0x12D.
/// Built once at module load time. Thread-safe (immutable after init).
private let dmTables: (exp: [Int], log: [Int]) = buildDMTables()

/// Multiply two elements of GF(256)/0x12D using log/antilog tables.
///
/// For a, b ≠ 0:  `a × b = exp[(log[a] + log[b]) mod 255]`
/// If either operand is 0, the product is 0 (zero absorbs multiplication).
///
/// This reduces polynomial multiplication to two table lookups and one
/// addition modulo 255 — effectively O(1).
private func gfMul(_ a: Int, _ b: Int) -> Int {
    guard a != 0 && b != 0 else { return 0 }
    return dmTables.exp[(dmTables.log[a] + dmTables.log[b]) % 255]
}

// ============================================================================
// MARK: - RS generator polynomials (GF(256)/0x12D, b=1 convention)
// ============================================================================
//
// Data Matrix uses the b=1 convention: the RS generator's roots are
// α¹, α², …, αⁿ (not α⁰, α¹, …, α^{n-1} like QR Code).
//
// The generator polynomial for n ECC bytes:
//
//   g(x) = (x + α¹)(x + α²) ··· (x + αⁿ)
//
// The set of distinct ECC block sizes across all 30 ECC200 symbols is small
// and fixed by the standard: {5, 7, 10, 11, 12, 14, 17, 18, 21, 24, 28, 36,
// 42, 48, 56, 62, 68}. We precompute all of them at startup into an immutable
// dictionary — no mutable global state, fully Swift 6 concurrency-safe.

/// Build the RS generator polynomial for `nECC` ECC bytes.
///
/// Algorithm: start with g = [1], then for each i from 1 to nECC multiply g
/// by the linear factor (x + αⁱ):
///
/// ```
/// for j, coeff in enumerate(g):
///     new_g[j]   ^= coeff          (coeff × x term)
///     new_g[j+1] ^= coeff × αⁱ    (coeff × constant term)
/// ```
///
/// Format: highest-degree coefficient first, length = nECC + 1.
private func buildGenerator(_ nECC: Int) -> [Int] {
    var g = [1]
    for i in 1...nECC {
        let ai = dmTables.exp[i]  // αⁱ
        var newG = [Int](repeating: 0, count: g.count + 1)
        for (j, coeff) in g.enumerated() {
            newG[j] ^= coeff
            newG[j + 1] ^= gfMul(coeff, ai)
        }
        g = newG
    }
    return g
}

/// Pre-built generator polynomials for all ECC block sizes used by ECC200.
///
/// Distinct eccPerBlock values across all 30 symbols (from squareSizes + rectSizes):
///   5, 7, 10, 11, 12, 14, 17, 18, 21, 24, 28, 36, 42, 48, 56, 62, 68
///
/// Building them all at module-load time (into an immutable `let`) is the
/// correct Swift 6 approach: no mutable global state, no `nonisolated` issues.
private let generatorTable: [Int: [Int]] = {
    let sizes = [5, 7, 10, 11, 12, 14, 17, 18, 21, 24, 28, 36, 42, 48, 56, 62, 68]
    var table = [Int: [Int]]()
    for n in sizes { table[n] = buildGenerator(n) }
    return table
}()

/// Return the generator polynomial for `nECC` ECC bytes.
///
/// Falls back to building on demand for any size not in the precomputed table
/// (this should never happen for valid ECC200 symbols, but is safe regardless).
private func getGenerator(_ nECC: Int) -> [Int] {
    if let prebuilt = generatorTable[nECC] { return prebuilt }
    return buildGenerator(nECC)  // fallback — should not occur for ECC200
}

// ============================================================================
// MARK: - Reed-Solomon encoding (LFSR shift-register method)
// ============================================================================

/// Compute ECC bytes for a data block using the LFSR shift-register method.
///
/// Computes `R(x) = D(x) · x^{nECC} mod G(x)` over GF(256)/0x12D.
///
/// For each input byte `d`:
///
/// ```
/// feedback = d XOR rem[0]
/// shift rem left by one position (drop rem[0], append 0)
/// for i in 0..nECC-1:
///     rem[i] ^= gen[i+1] × feedback
/// ```
///
/// This is the standard systematic Reed-Solomon encoding approach —
/// equivalent to polynomial long-division but implemented as a streaming
/// shift register. Output length = `generator.count - 1`.
private func rsEncodeBlock(data: [Int], generator: [Int]) -> [Int] {
    let nECC = generator.count - 1
    var rem = [Int](repeating: 0, count: nECC)
    for d in data {
        let fb = d ^ rem[0]
        // Shift register left: drop rem[0], push 0 at the end.
        rem = Array(rem.dropFirst()) + [0]
        if fb != 0 {
            for i in 0..<nECC {
                rem[i] ^= gfMul(generator[i + 1], fb)
            }
        }
    }
    return rem
}

// ============================================================================
// MARK: - ASCII data encoding (ISO/IEC 16022:2006 §5.2.4)
// ============================================================================
//
// ASCII mode is the default encoding mode for Data Matrix ECC200. It covers
// all 128 ASCII characters and provides a digit-pair compaction optimization
// that halves the codeword budget for numeric strings.
//
// Encoding rules:
//
//   1. Two consecutive ASCII digits (0x30–0x39) → one codeword =
//      130 + (d1 × 10 + d2). This digit-pair optimization is critical for
//      manufacturing lot codes, serial numbers, and barcodes that are mostly
//      digits.
//
//   2. Single ASCII char (0–127) → one codeword = ASCII_value + 1.
//      So 'A' (65) → 66, space (32) → 33. The +1 shift exists because
//      codeword 0 is reserved as "end of data".
//
//   3. Extended ASCII (128–255) → two codewords: 235 (UPPER_SHIFT),
//      then ASCII_value - 127. Enables Latin-1 / Windows-1252 characters
//      but is rare in practice.
//
// Examples:
//   "A"    → [66]           (65 + 1)
//   "12"   → [142]          (130 + 12, digit pair)
//   "1234" → [142, 174]     (two digit pairs)
//   "1A"   → [50, 66]       ('1' alone — next char is not a digit)
//   "00"   → [130]          (130 + 0)
//   "99"   → [229]          (130 + 99)

/// Encode a byte sequence in Data Matrix ASCII mode.
///
/// - Parameter inputBytes: Raw UTF-8 bytes of the input string.
/// - Returns: Array of Data Matrix codeword integers.
private func encodeASCII(_ inputBytes: [UInt8]) -> [Int] {
    var codewords = [Int]()
    var i = 0
    let n = inputBytes.count
    while i < n {
        let c = Int(inputBytes[i])
        // Digit pair: both current and next bytes are ASCII digits (0x30–0x39).
        if c >= 0x30 && c <= 0x39 && i + 1 < n {
            let next = Int(inputBytes[i + 1])
            if next >= 0x30 && next <= 0x39 {
                let d1 = c - 0x30
                let d2 = next - 0x30
                codewords.append(130 + d1 * 10 + d2)
                i += 2
                continue
            }
        }
        if c <= 127 {
            // Standard ASCII character: value + 1.
            codewords.append(c + 1)
        } else {
            // Extended ASCII (128–255): UPPER_SHIFT then (value - 127).
            codewords.append(235)
            codewords.append(c - 127)
        }
        i += 1
    }
    return codewords
}

// ============================================================================
// MARK: - Pad codewords (ISO/IEC 16022:2006 §5.2.3)
// ============================================================================
//
// After ASCII encoding, the codeword sequence is padded to the symbol's data
// capacity. The padding uses a scrambled pseudo-random sequence to prevent
// degenerate placement patterns in the Utah algorithm.
//
// Padding rules:
//   1. The first pad codeword is always the literal value 129 ("EOM").
//   2. Subsequent pads use a scrambled value depending on their 1-indexed
//      position k within the full codeword stream:
//
//      scrambled = 129 + (149 × k) mod 253 + 1
//      if scrambled > 254: scrambled -= 254
//
// Worked example — encoding "A" into a 10×10 symbol (dataCW = 3):
//   codewords = [66]  (length 1)
//   k=2: 129                      (first pad — always literal EOM)
//   k=3: 129 + (149×3 mod 253) + 1 = 129 + 194 + 1 = 324; 324 - 254 = 70
//   Result: [66, 129, 70]

/// Pad `codewords` to exactly `dataCW` bytes using the ECC200 scrambled rule.
///
/// - Parameters:
///   - codewords: The already-encoded data codewords.
///   - dataCW: Target capacity (must be ≥ codewords.count).
/// - Returns: A new array of exactly `dataCW` codewords.
private func padCodewords(_ codewords: [Int], to dataCW: Int) -> [Int] {
    var padded = codewords
    var isFirst = true
    // k is the 1-indexed position of the next pad byte in the full stream.
    var k = codewords.count + 1
    while padded.count < dataCW {
        if isFirst {
            padded.append(129)
            isFirst = false
        } else {
            var scrambled = 129 + (149 * k) % 253 + 1
            if scrambled > 254 { scrambled -= 254 }
            padded.append(scrambled)
        }
        k += 1
    }
    return padded
}

// ============================================================================
// MARK: - Symbol selection
// ============================================================================

/// Find the smallest symbol entry that can hold `codewordCount` codewords.
///
/// Iterates all candidates (filtered by shape) sorted by dataCW ascending —
/// ties broken by total module area — and returns the first whose
/// `dataCW ≥ codewordCount`.
///
/// - Parameters:
///   - codewordCount: Number of data codewords to fit.
///   - shape: Which symbol shape family to consider.
/// - Throws: `DataMatrixError.inputTooLong` if no symbol is large enough.
private func selectSymbol(codewordCount: Int, shape: SymbolShape) throws -> SymbolEntry {
    var candidates: [SymbolEntry]
    switch shape {
    case .square:
        candidates = squareSizes
    case .rectangle:
        candidates = rectSizes
    case .any:
        candidates = squareSizes + rectSizes
    }

    // Sort by capacity ascending, ties broken by module area.
    candidates.sort { a, b in
        if a.dataCW != b.dataCW { return a.dataCW < b.dataCW }
        return a.symbolRows * a.symbolCols < b.symbolRows * b.symbolCols
    }

    for e in candidates {
        if e.dataCW >= codewordCount { return e }
    }

    throw DataMatrixError.inputTooLong(
        "data-matrix: input too long — encoded \(codewordCount) codewords, "
        + "maximum is \(maxDataCW) (144×144 symbol)."
    )
}

/// Find the symbol entry matching an explicit (rows, cols) size.
///
/// - Throws: `DataMatrixError.invalidSize` if no match found.
private func findEntryBySize(rows: Int, cols: Int) throws -> SymbolEntry {
    for entry in squareSizes where entry.symbolRows == rows && entry.symbolCols == cols {
        return entry
    }
    for entry in rectSizes where entry.symbolRows == rows && entry.symbolCols == cols {
        return entry
    }
    throw DataMatrixError.invalidSize(
        "data-matrix: \(rows)×\(cols) is not a valid ECC200 symbol size. "
        + "Square sizes: 10×10, 12×12, …, 144×144. "
        + "Rect sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48."
    )
}

// ============================================================================
// MARK: - Block splitting, ECC computation, and interleaving
// ============================================================================
//
// Larger symbols use multiple interleaved Reed-Solomon blocks to distribute
// burst error correction. The interleaving distributes burst errors: a
// physical scratch destroying N contiguous modules affects at most
// ceil(N / numBlocks) codewords per block — far more likely to be within
// each block's correction capacity.
//
// Block splitting:
//
//   baseLen    = dataCW / numBlocks      (integer division)
//   extraBlocks = dataCW mod numBlocks
//   Blocks 0..extraBlocks-1   get baseLen + 1 data codewords.
//   Blocks extraBlocks..end-1 get baseLen     data codewords.
//
// Interleaving output order:
//   1. Data codewords round-robin: emit data[blk][pos] for each block.
//   2. ECC codewords round-robin: emit ecc[blk][pos] for each block.

/// Split `data` across RS blocks, compute ECC per block, and interleave.
///
/// - Parameters:
///   - data: Padded data codewords (length == entry.dataCW).
///   - entry: Symbol size entry providing block parameters.
/// - Returns: Interleaved codeword array ready for Utah placement.
private func computeInterleaved(data: [Int], entry: SymbolEntry) -> [Int] {
    let numBlocks = entry.numBlocks
    let eccPerBlock = entry.eccPerBlock
    let dataCW = entry.dataCW
    let gen = getGenerator(eccPerBlock)

    // ── Split data into blocks ───────────────────────────────────────────────
    // Earlier blocks get one extra codeword when dataCW is not divisible by numBlocks.
    let baseLen = dataCW / numBlocks
    let extraBlocks = dataCW % numBlocks

    var dataBlocks = [[Int]]()
    var offset = 0
    for b in 0..<numBlocks {
        let len = b < extraBlocks ? baseLen + 1 : baseLen
        dataBlocks.append(Array(data[offset..<(offset + len)]))
        offset += len
    }

    // ── Compute ECC for each block ───────────────────────────────────────────
    let eccBlocks = dataBlocks.map { rsEncodeBlock(data: $0, generator: gen) }

    // ── Interleave data round-robin ──────────────────────────────────────────
    var interleaved = [Int]()
    let maxDataLen = dataBlocks.map { $0.count }.max() ?? 0
    for pos in 0..<maxDataLen {
        for b in 0..<numBlocks {
            if pos < dataBlocks[b].count {
                interleaved.append(dataBlocks[b][pos])
            }
        }
    }

    // ── Interleave ECC round-robin ───────────────────────────────────────────
    for pos in 0..<eccPerBlock {
        for b in 0..<numBlocks {
            interleaved.append(eccBlocks[b][pos])
        }
    }

    return interleaved
}

// ============================================================================
// MARK: - Grid initialization (finder + timing + alignment borders)
// ============================================================================
//
// The physical grid has three kinds of structural elements:
//
// Outer "finder + clock" border (all symbols):
//
//   - Top row    (row 0): alternating dark/light starting dark at col 0.
//                         This is the TIMING clock for the top edge.
//   - Right col  (col C-1): alternating dark/light starting dark at row 0.
//                           This is the TIMING clock for the right edge.
//   - Left col   (col 0): ALL DARK — vertical leg of the L-finder.
//   - Bottom row (row R-1): ALL DARK — horizontal leg of the L-finder.
//
// The L-shaped solid bar tells a scanner where the symbol starts and which
// orientation it has. The alternating timing on the opposite two edges
// distinguishes all four 90° rotations.
//
// Alignment borders (multi-region symbols with regionRows×regionCols > 1):
//
//   For symbols with more than one data region, 2-module alignment borders
//   separate adjacent data regions:
//     - First row/col of the border: ALL DARK
//     - Second row/col of the border: ALTERNATING (starts dark)
//
// Writing order matters for corner pixels:
//   1. Alignment borders FIRST (so outer border can override at intersections).
//   2. Top row (timing).
//   3. Right column (timing).
//   4. Left column (L-finder) — overrides timing at (0,0).
//   5. Bottom row (L-finder) — written LAST, overrides everything.

/// Allocate the physical grid and fill in all fixed structural elements.
///
/// - Parameter entry: Symbol size entry providing dimensions and region layout.
/// - Returns: A 2D boolean grid with all structural modules set.
private func initGrid(_ entry: SymbolEntry) -> [[Bool]] {
    let R = entry.symbolRows
    let C = entry.symbolCols

    var grid = [[Bool]](repeating: [Bool](repeating: false, count: C), count: R)

    // ── Alignment borders (multi-region symbols only) ──────────────────────
    // Written FIRST so the outer border can override at intersections.
    for rr in 0..<(entry.regionRows - 1) {
        // Physical row of the first alignment border row:
        //   outer border (1) + (rr+1) * dataRegionHeight + rr * 2 (prev ABs)
        let abRow0 = 1 + (rr + 1) * entry.dataRegionHeight + rr * 2
        let abRow1 = abRow0 + 1
        for c in 0..<C {
            grid[abRow0][c] = true           // all dark
            grid[abRow1][c] = (c % 2 == 0)  // alternating, starts dark
        }
    }
    for rc in 0..<(entry.regionCols - 1) {
        let abCol0 = 1 + (rc + 1) * entry.dataRegionWidth + rc * 2
        let abCol1 = abCol0 + 1
        for r in 0..<R {
            grid[r][abCol0] = true           // all dark
            grid[r][abCol1] = (r % 2 == 0)  // alternating, starts dark
        }
    }

    // ── Top row: timing clock — alternating dark/light starting dark ────────
    for c in 0..<C {
        grid[0][c] = (c % 2 == 0)
    }

    // ── Right column: timing clock — alternating, starts dark ───────────────
    for r in 0..<R {
        grid[r][C - 1] = (r % 2 == 0)
    }

    // ── Left column: L-finder left leg — all dark ───────────────────────────
    // Written after timing to override the timing value at (0, 0).
    for r in 0..<R {
        grid[r][0] = true
    }

    // ── Bottom row: L-finder bottom leg — all dark ──────────────────────────
    // Written LAST: overrides alignment borders, right-column timing, etc.
    for c in 0..<C {
        grid[R - 1][c] = true
    }

    return grid
}

// ============================================================================
// MARK: - Utah placement algorithm
// ============================================================================
//
// The Utah placement algorithm is the most distinctive part of Data Matrix
// encoding. Its name comes from the 8-module codeword shape, which resembles
// the outline of the US state of Utah — a rectangle with a notch cut from
// the top-left corner.
//
// The algorithm scans the *logical* grid (all data region interiors
// concatenated) in a diagonal zigzag. For each codeword, 8 bits are placed
// at 8 fixed offsets relative to the current reference position (row, col).
// After each codeword the reference moves diagonally:
//   - row -= 2, col += 2  for the upward-right leg
//   - row += 2, col -= 2  for the downward-left leg
//
// Four special "corner" patterns handle positions where the standard Utah
// shape would extend outside the grid boundary.
//
// There is NO masking step after placement. The diagonal traversal naturally
// distributes bits across the symbol without the degenerate clustering that
// would otherwise require masking (as in QR Code).
//
// The standard "Utah" 8-module shape at reference position (row, col):
//
//               col-2  col-1   col
//
//    row-2 :      .   [bit1]  [bit2]
//    row-1 :   [bit3] [bit4]  [bit5]
//    row   :   [bit6] [bit7]  [bit8]
//
// Bits 1–8 are extracted from the codeword with bit 8 = MSB (placed at
// (row, col)) and bit 1 = LSB (placed at (row-2, col-1)).

/// Apply the boundary wrap rules from ISO/IEC 16022:2006 Annex F.
///
/// When the standard Utah shape extends beyond the logical grid edge,
/// these four rules fold the coordinates back into valid range.
/// Applied in order — the first matching rule wins.
///
/// - Parameters:
///   - row: Logical row coordinate (may be negative or ≥ nRows).
///   - col: Logical col coordinate (may be negative or ≥ nCols).
///   - nRows: Logical grid height.
///   - nCols: Logical grid width.
/// - Returns: Wrapped (row, col) within [0, nRows) × [0, nCols).
private func applyWrap(row: Int, col: Int, nRows: Int, nCols: Int) -> (Int, Int) {
    // Rule 1: top-left singularity
    if row < 0 && col == 0 { return (1, 3) }
    // Rule 2: wrapped past right edge
    if row < 0 && col == nCols { return (0, col - 2) }
    // Rule 3: wrap top → bottom
    if row < 0 { return (row + nRows, col - 4) }
    // Rule 4: wrap left → right
    if col < 0 { return (row - 4, col + nCols) }
    return (row, col)
}

/// Place one codeword using the standard "Utah" 8-module pattern.
///
/// Each of the 8 bits is placed at a fixed offset from the reference
/// position `(row, col)`, with wrapping applied when out of bounds.
/// Already-placed modules (used[r][c] == true) are not overwritten.
private func placeUtah(
    cw: Int, row: Int, col: Int, nRows: Int, nCols: Int,
    grid: inout [[Bool]], used: inout [[Bool]]
) {
    // (deltaRow, deltaCol, bitShift) — bitShift 7 = MSB, 0 = LSB
    let placements: [(Int, Int, Int)] = [
        (0,  0,  7),  // bit 8 (MSB) at (row,   col)
        (0, -1,  6),  // bit 7       at (row,   col-1)
        (0, -2,  5),  // bit 6       at (row,   col-2)
        (-1, 0,  4),  // bit 5       at (row-1, col)
        (-1,-1,  3),  // bit 4       at (row-1, col-1)
        (-1,-2,  2),  // bit 3       at (row-1, col-2)
        (-2, 0,  1),  // bit 2       at (row-2, col)
        (-2,-1,  0),  // bit 1 (LSB) at (row-2, col-1)
    ]
    for (dr, dc, bit) in placements {
        let (r, c) = applyWrap(row: row + dr, col: col + dc, nRows: nRows, nCols: nCols)
        if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
            grid[r][c] = ((cw >> bit) & 1) == 1
            used[r][c] = true
        }
    }
}

/// Place a codeword at explicit (row, col, bitShift) positions.
///
/// Used by the four corner patterns where the standard Utah offsets
/// do not apply. Each element of `positions` is (row, col, bitShift).
private func placeAtPositions(
    cw: Int, positions: [(Int, Int, Int)], nRows: Int, nCols: Int,
    grid: inout [[Bool]], used: inout [[Bool]]
) {
    for (r, c, bit) in positions {
        if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] {
            grid[r][c] = ((cw >> bit) & 1) == 1
            used[r][c] = true
        }
    }
}

// ── Corner patterns ──────────────────────────────────────────────────────────
//
// These four patterns replace the Utah shape at specific boundary positions.
// Each pattern places 8 bits at fixed coordinates (not offsets), allowing
// edge and corner modules to be reached without wrapping ambiguity.

/// Corner pattern 1 — triggered at the top-left boundary.
private func placeCorner1(
    cw: Int, nRows: Int, nCols: Int,
    grid: inout [[Bool]], used: inout [[Bool]]
) {
    let positions: [(Int, Int, Int)] = [
        (0,          nCols - 2, 7),
        (0,          nCols - 1, 6),
        (1,          0,          5),
        (2,          0,          4),
        (nRows - 2,  0,          3),
        (nRows - 1,  0,          2),
        (nRows - 1,  1,          1),
        (nRows - 1,  2,          0),
    ]
    placeAtPositions(cw: cw, positions: positions, nRows: nRows, nCols: nCols, grid: &grid, used: &used)
}

/// Corner pattern 2 — triggered at the top-right boundary.
private func placeCorner2(
    cw: Int, nRows: Int, nCols: Int,
    grid: inout [[Bool]], used: inout [[Bool]]
) {
    let positions: [(Int, Int, Int)] = [
        (0,          nCols - 2, 7),
        (0,          nCols - 1, 6),
        (1,          nCols - 1, 5),
        (2,          nCols - 1, 4),
        (nRows - 1,  0,          3),
        (nRows - 1,  1,          2),
        (nRows - 1,  2,          1),
        (nRows - 1,  3,          0),
    ]
    placeAtPositions(cw: cw, positions: positions, nRows: nRows, nCols: nCols, grid: &grid, used: &used)
}

/// Corner pattern 3 — triggered at the bottom-left boundary.
private func placeCorner3(
    cw: Int, nRows: Int, nCols: Int,
    grid: inout [[Bool]], used: inout [[Bool]]
) {
    let positions: [(Int, Int, Int)] = [
        (0,          nCols - 1, 7),
        (1,          0,          6),
        (2,          0,          5),
        (nRows - 2,  0,          4),
        (nRows - 1,  0,          3),
        (nRows - 1,  1,          2),
        (nRows - 1,  2,          1),
        (nRows - 1,  3,          0),
    ]
    placeAtPositions(cw: cw, positions: positions, nRows: nRows, nCols: nCols, grid: &grid, used: &used)
}

/// Corner pattern 4 — triggered when nCols mod 8 == 0.
private func placeCorner4(
    cw: Int, nRows: Int, nCols: Int,
    grid: inout [[Bool]], used: inout [[Bool]]
) {
    let positions: [(Int, Int, Int)] = [
        (nRows - 3, nCols - 1, 7),
        (nRows - 2, nCols - 1, 6),
        (nRows - 1, nCols - 3, 5),
        (nRows - 1, nCols - 2, 4),
        (nRows - 1, nCols - 1, 3),
        (0,          0,          2),
        (1,          0,          1),
        (2,          0,          0),
    ]
    placeAtPositions(cw: cw, positions: positions, nRows: nRows, nCols: nCols, grid: &grid, used: &used)
}

/// Run the Utah diagonal placement algorithm on the logical data matrix.
///
/// The reference position `(row, col)` starts at `(4, 0)` and zigzags
/// diagonally across the logical grid. Each iteration of the outer loop
/// has two legs:
///
///   1. Upward-right leg: place codewords, then move `row -= 2, col += 2`
///      until out of bounds. Then step: `row += 1, col += 3`.
///
///   2. Downward-left leg: place codewords, then move `row += 2, col -= 2`
///      until out of bounds. Then step: `row += 3, col += 1`.
///
/// Between legs, four corner patterns fire when the reference position
/// matches specific trigger conditions.
///
/// Termination: when both `row >= nRows` and `col >= nCols`, all codewords
/// have been visited. Unvisited modules receive the ISO fill pattern:
/// `(r + c) % 2 == 1` (dark) — the "right-and-bottom fill rule".
///
/// - Parameters:
///   - codewords: Full interleaved codeword array (data + ECC).
///   - nRows: Logical grid height (= regionRows × dataRegionHeight).
///   - nCols: Logical grid width  (= regionCols × dataRegionWidth).
/// - Returns: 2D boolean logical grid after placement.
private func utahPlacement(codewords: [Int], nRows: Int, nCols: Int) -> [[Bool]] {
    var grid = [[Bool]](repeating: [Bool](repeating: false, count: nCols), count: nRows)
    var used = [[Bool]](repeating: [Bool](repeating: false, count: nCols), count: nRows)

    var cwIdx = 0
    var row = 4
    var col = 0

    while true {
        // ── Corner special cases ─────────────────────────────────────────────
        // Each corner fires only once (the used[][] array prevents double-placement).

        // Corner 1: reference at (nRows, 0) when nRows or nCols ≡ 0 (mod 4).
        if row == nRows && col == 0 && (nRows % 4 == 0 || nCols % 4 == 0) {
            if cwIdx < codewords.count {
                placeCorner1(cw: codewords[cwIdx], nRows: nRows, nCols: nCols, grid: &grid, used: &used)
                cwIdx += 1
            }
        }
        // Corner 2: reference at (nRows-2, 0) when nCols mod 4 ≠ 0.
        if row == nRows - 2 && col == 0 && nCols % 4 != 0 {
            if cwIdx < codewords.count {
                placeCorner2(cw: codewords[cwIdx], nRows: nRows, nCols: nCols, grid: &grid, used: &used)
                cwIdx += 1
            }
        }
        // Corner 3: reference at (nRows-2, 0) when nCols mod 8 == 4.
        if row == nRows - 2 && col == 0 && nCols % 8 == 4 {
            if cwIdx < codewords.count {
                placeCorner3(cw: codewords[cwIdx], nRows: nRows, nCols: nCols, grid: &grid, used: &used)
                cwIdx += 1
            }
        }
        // Corner 4: reference at (nRows+4, 2) when nCols mod 8 == 0.
        if row == nRows + 4 && col == 2 && nCols % 8 == 0 {
            if cwIdx < codewords.count {
                placeCorner4(cw: codewords[cwIdx], nRows: nRows, nCols: nCols, grid: &grid, used: &used)
                cwIdx += 1
            }
        }

        // ── Upward-right diagonal leg (row -= 2, col += 2) ──────────────────
        var r = row
        var c = col
        while true {
            if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] && cwIdx < codewords.count {
                placeUtah(cw: codewords[cwIdx], row: r, col: c, nRows: nRows, nCols: nCols, grid: &grid, used: &used)
                cwIdx += 1
            }
            r -= 2
            c += 2
            if r < 0 || c >= nCols { break }
        }

        // Step to next diagonal start.
        row += 1
        col += 3

        // ── Downward-left diagonal leg (row += 2, col -= 2) ─────────────────
        r = row
        c = col
        while true {
            if r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c] && cwIdx < codewords.count {
                placeUtah(cw: codewords[cwIdx], row: r, col: c, nRows: nRows, nCols: nCols, grid: &grid, used: &used)
                cwIdx += 1
            }
            r += 2
            c -= 2
            if r >= nRows || c < 0 { break }
        }

        // Step to next diagonal start.
        row += 3
        col += 1

        // ── Termination check ────────────────────────────────────────────────
        if row >= nRows && col >= nCols { break }
        if cwIdx >= codewords.count { break }
    }

    // ── Fill remaining unset modules (ISO right-and-bottom fill rule) ────────
    // Some symbol sizes have residual modules the diagonal walk does not reach.
    // ISO/IEC 16022 §10 specifies these receive (r+c) mod 2 == 1 (dark).
    for r in 0..<nRows {
        for c in 0..<nCols {
            if !used[r][c] {
                grid[r][c] = (r + c) % 2 == 1
            }
        }
    }

    return grid
}

// ============================================================================
// MARK: - Logical → physical coordinate mapping
// ============================================================================
//
// The logical data matrix is the concatenation of all data region interiors
// treated as one flat grid. Utah placement works in this logical space.
// After placement we map back to the physical grid, which adds:
//
//   - 1-module outer border (finder + timing) on all four sides.
//   - 2-module alignment border between adjacent data regions.
//
// For a symbol with regionRows × regionCols data regions, each of size
// (rh × rw):
//
//   physRow = ⌊r / rh⌋ × (rh + 2) + (r mod rh) + 1
//   physCol = ⌊c / rw⌋ × (rw + 2) + (c mod rw) + 1
//
// The "+ 2" accounts for the 2-module alignment border between regions.
// The "+ 1" accounts for the 1-module outer border.
//
// For single-region symbols (1×1) this simplifies to:
//   physRow = r + 1, physCol = c + 1

/// Map a logical data-matrix coordinate to its physical symbol coordinate.
///
/// - Parameters:
///   - r: Logical row in [0, regionRows × dataRegionHeight).
///   - c: Logical col in [0, regionCols × dataRegionWidth).
///   - entry: Symbol size entry providing region layout.
/// - Returns: Physical (row, col) in the full symbol grid.
private func logicalToPhysical(r: Int, c: Int, entry: SymbolEntry) -> (Int, Int) {
    let rh = entry.dataRegionHeight
    let rw = entry.dataRegionWidth
    let physRow = (r / rh) * (rh + 2) + (r % rh) + 1
    let physCol = (c / rw) * (rw + 2) + (c % rw) + 1
    return (physRow, physCol)
}

// ============================================================================
// MARK: - Public encode function
// ============================================================================

/// Encode a string into a Data Matrix ECC200 `ModuleGrid`.
///
/// The smallest symbol that can hold the encoded data is selected
/// automatically. Use `DataMatrixOptions` to control shape preference or
/// force a specific symbol size.
///
/// ## Pipeline
///
///   1. ASCII-encode the input (with digit-pair compression).
///   2. Select the smallest fitting symbol (or use the forced size).
///   3. Pad to data capacity with the ECC200 scrambled-pad sequence.
///   4. Compute Reed-Solomon ECC for each block over GF(256)/0x12D.
///   5. Interleave data + ECC blocks round-robin.
///   6. Initialize the physical grid (finder + timing + alignment borders).
///   7. Run Utah diagonal placement on the logical data matrix.
///   8. Map logical → physical coordinates.
///   9. Return the `ModuleGrid`.
///
/// ## Example
///
/// ```swift
/// // Auto-select the smallest square symbol:
/// let grid = try encode("A")
/// assert(grid.rows == 10 && grid.cols == 10)
///
/// // Allow rectangular symbols:
/// var opts = DataMatrixOptions()
/// opts.shape = .any
/// let grid = try encode("Hi", options: opts)
/// ```
///
/// - Parameters:
///   - data: The string to encode. Encoded as UTF-8; bytes 128–255 use the
///           UPPER_SHIFT mechanism (two codewords each).
///   - options: Encoding options (shape, optional explicit size).
/// - Returns: A `ModuleGrid` where `true` = dark module.
/// - Throws: `DataMatrixError.inputTooLong` if the input is too long.
///           `DataMatrixError.invalidSize` if `options.size` is not valid.
public func encode(_ data: String, options: DataMatrixOptions = DataMatrixOptions()) throws -> ModuleGrid {
    // Step 1: ASCII encode (UTF-8 bytes; bytes ≥128 use UPPER_SHIFT).
    let inputBytes = Array(data.utf8)
    let codewords = encodeASCII(inputBytes)

    // Step 2: Select symbol — explicit size or auto-pick smallest.
    let entry: SymbolEntry
    if let size = options.size {
        entry = try findEntryBySize(rows: size, cols: size)
        if codewords.count > entry.dataCW {
            throw DataMatrixError.inputTooLong(
                "data-matrix: input encodes to \(codewords.count) codewords "
                + "but \(entry.symbolRows)×\(entry.symbolCols) symbol holds only \(entry.dataCW)."
            )
        }
    } else {
        entry = try selectSymbol(codewordCount: codewords.count, shape: options.shape)
    }

    // Step 3: Pad to data capacity.
    let padded = padCodewords(codewords, to: entry.dataCW)

    // Steps 4–5: Compute RS ECC and interleave blocks.
    let interleaved = computeInterleaved(data: padded, entry: entry)

    // Step 6: Initialize physical grid with all structural modules.
    var physGrid = initGrid(entry)

    // Step 7: Run Utah placement on the logical data matrix.
    let nRows = entry.regionRows * entry.dataRegionHeight
    let nCols = entry.regionCols * entry.dataRegionWidth
    let logicalGrid = utahPlacement(codewords: interleaved, nRows: nRows, nCols: nCols)

    // Step 8: Map logical → physical coordinates.
    for r in 0..<nRows {
        for c in 0..<nCols {
            let (pr, pc) = logicalToPhysical(r: r, c: c, entry: entry)
            physGrid[pr][pc] = logicalGrid[r][c]
        }
    }

    // Step 9: Build the ModuleGrid from the physical boolean grid.
    var grid = makeModuleGrid(rows: entry.symbolRows, cols: entry.symbolCols)
    for r in 0..<entry.symbolRows {
        for c in 0..<entry.symbolCols {
            if physGrid[r][c] {
                grid = try setModule(grid: grid, row: r, col: c, dark: true)
            }
        }
    }

    return grid
}

// ============================================================================
// MARK: - Convenience overloads
// ============================================================================

/// Encode `data` to a specific square symbol size (side length in modules).
///
/// Equivalent to calling `encode(_:options:)` with `options.size = side`.
///
/// Valid square side lengths: 10, 12, 14, 16, 18, 20, 22, 24, 26, 32, 36,
/// 40, 44, 48, 52, 64, 72, 80, 88, 96, 104, 120, 132, 144.
///
/// ```swift
/// let grid = try encode("HELLO", squareSize: 18)
/// assert(grid.rows == 18 && grid.cols == 18)
/// ```
///
/// - Throws: `DataMatrixError.invalidSize` if `side` is not a valid square size.
///           `DataMatrixError.inputTooLong` if the input does not fit.
public func encode(_ data: String, squareSize side: Int) throws -> ModuleGrid {
    var opts = DataMatrixOptions()
    opts.size = side
    return try encode(data, options: opts)
}

/// Encode `data` to a specific rectangular symbol size (rows × cols).
///
/// Valid rectangular sizes: 8×18, 8×32, 12×26, 12×36, 16×36, 16×48.
///
/// ```swift
/// let grid = try encode("Hi", rows: 8, cols: 18)
/// assert(grid.rows == 8 && grid.cols == 18)
/// ```
///
/// - Throws: `DataMatrixError.invalidSize` if (rows, cols) is not valid.
///           `DataMatrixError.inputTooLong` if the input does not fit.
public func encode(_ data: String, rows: Int, cols: Int) throws -> ModuleGrid {
    let entry = try findEntryBySize(rows: rows, cols: cols)
    let inputBytes = Array(data.utf8)
    let codewords = encodeASCII(inputBytes)
    if codewords.count > entry.dataCW {
        throw DataMatrixError.inputTooLong(
            "data-matrix: input encodes to \(codewords.count) codewords "
            + "but \(rows)×\(cols) symbol holds only \(entry.dataCW)."
        )
    }
    let padded = padCodewords(codewords, to: entry.dataCW)
    let interleaved = computeInterleaved(data: padded, entry: entry)
    var physGrid = initGrid(entry)
    let nRows = entry.regionRows * entry.dataRegionHeight
    let nCols = entry.regionCols * entry.dataRegionWidth
    let logicalGrid = utahPlacement(codewords: interleaved, nRows: nRows, nCols: nCols)
    for r in 0..<nRows {
        for c in 0..<nCols {
            let (pr, pc) = logicalToPhysical(r: r, c: c, entry: entry)
            physGrid[pr][pc] = logicalGrid[r][c]
        }
    }
    var grid = makeModuleGrid(rows: entry.symbolRows, cols: entry.symbolCols)
    for r in 0..<entry.symbolRows {
        for c in 0..<entry.symbolCols {
            if physGrid[r][c] {
                grid = try setModule(grid: grid, row: r, col: c, dark: true)
            }
        }
    }
    return grid
}

// ============================================================================
// MARK: - Debugging utility
// ============================================================================

/// Render a `ModuleGrid` as a multi-line '0' / '1' string.
///
/// Useful for debugging, snapshot tests, and cross-language corpus comparison.
/// Each row is one line; rows are separated by newlines; no trailing newline.
///
/// ```swift
/// let grid = try encode("A")
/// print(gridToString(grid))
/// // 1010101010
/// // 1000010001
/// // ...
/// ```
public func gridToString(_ grid: ModuleGrid) -> String {
    grid.modules.map { row in
        row.map { $0 ? "1" : "0" }.joined()
    }.joined(separator: "\n")
}
