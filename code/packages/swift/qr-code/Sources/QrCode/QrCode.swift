// QrCode.swift — ISO/IEC 18004:2015 QR Code encoder
//
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Overview
// ============================================================================
//
// QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in
// 1994 to track automotive parts. It encodes text (or binary data) as a 2D
// grid of black and white squares that any camera can scan and decode.
//
// ## Encoding pipeline (bottom to top)
//
// ```
// input string
//   → mode selection    (numeric / alphanumeric / byte)
//   → version selection (smallest version that fits at chosen ECC level)
//   → bit stream        (mode indicator + char count + data + padding)
//   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
//   → interleave        (data CWs interleaved, then ECC CWs)
//   → grid init         (finder, separator, timing, alignment, format, dark)
//   → zigzag placement  (two-column snake from bottom-right)
//   → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
//   → finalize          (format info + version info v7+)
//   → ModuleGrid        (boolean grid, true = dark)
// ```
//
// ## Reed-Solomon convention
//
// The QR Code standard uses the b=0 convention: the generator polynomial's
// roots are α^0, α^1, …, α^{n-1}. This differs from the coding-adventures
// ReedSolomon package (which uses b=1). We embed our own minimal LFSR-based
// RS encoder directly in this file to avoid the mismatch.
//
// ## Key parameters
//
// Symbol size: (4V + 17) × (4V + 17) modules, V ∈ {1..40}.
// V1 = 21×21, V40 = 177×177.
//
// ============================================================================

import Barcode2D
import GF256

// ============================================================================
// MARK: - Public errors
// ============================================================================

/// Any error produced by the QR Code encoder.
///
/// The hierarchy is flat (two cases), keeping error handling simple for callers
/// that only want to catch QR-specific failures.
public enum QrCodeError: Error, Equatable {

    /// The input string is too long for any QR Code version at the chosen ECC
    /// level. Version 40 at level L holds at most 7,089 numeric characters or
    /// 2,953 bytes.
    case inputTooLong(String)

    /// An internal encoding precondition was violated. Should never be thrown
    /// by valid UTF-8 input — if it is, it indicates a bug in the encoder.
    case encodingError(String)
}

// ============================================================================
// MARK: - Error correction level
// ============================================================================

/// The four QR Code error correction levels.
///
/// Higher levels add more redundancy, shrinking data capacity but making the
/// symbol more resilient to damage or occlusion.
///
/// | Level     | Recovery | Typical use case                          |
/// |-----------|----------|-------------------------------------------|
/// | `.low`    | ~7%      | Maximum data density, low damage risk     |
/// | `.medium` | ~15%     | General purpose — the common default      |
/// | `.quartile`| ~25%   | Moderate damage / decorative overlays     |
/// | `.high`   | ~30%     | Heavy damage risk, logo printed on top    |
///
/// - Note: "Quartile" is the official ISO name for 25% recovery, not a
///   standard English word for 25%.
public enum ErrorCorrectionLevel: Sendable, Equatable {
    case low       /// L — ~7% recovery
    case medium    /// M — ~15% recovery
    case quartile  /// Q — ~25% recovery
    case high      /// H — ~30% recovery

    /// 2-bit ECC level indicator written into format information.
    ///
    /// ISO 18004 Table 12 — deliberately NOT alphabetical:
    ///   L=01, M=00, Q=11, H=10
    var formatBits: Int {
        switch self {
        case .low:      return 0b01
        case .medium:   return 0b00
        case .quartile: return 0b11
        case .high:     return 0b10
        }
    }

    /// Index into the [L, M, Q, H] lookup table arrays.
    var index: Int {
        switch self {
        case .low:      return 0
        case .medium:   return 1
        case .quartile: return 2
        case .high:     return 3
        }
    }
}

// ============================================================================
// MARK: - Grid geometry helpers
// ============================================================================

/// Symbol side length in modules: (4V + 17).
///
/// Version 1 → 21 modules. Version 40 → 177 modules.
func symbolSize(_ version: Int) -> Int {
    return 4 * version + 17
}

/// Total raw bits available in the symbol (data + ECC combined).
///
/// Derived by subtracting all function-module areas from the raw bit count.
/// Formula from Nayuki's reference implementation (public domain).
///
/// Function modules that are NOT data:
///   • Finder patterns + separators
///   • Timing strips
///   • Alignment patterns
///   • Format information (15-bit × 2 copies)
///   • Version information (18-bit × 2 copies, v7+)
///   • Always-dark module
func numRawDataModules(_ version: Int) -> Int {
    var result = (16 * version + 128) * version + 64
    if version >= 2 {
        let numAlign = version / 7 + 2
        result -= (25 * numAlign - 10) * numAlign - 55
        if version >= 7 { result -= 36 }
    }
    return result
}

/// Total DATA codewords (no ECC) for a given version/ECC combination.
///
/// rawBits / 8, minus the ECC codewords that the RS encoder will add.
func numDataCodewords(_ version: Int, _ ecc: ErrorCorrectionLevel) -> Int {
    let e = ecc.index
    return numRawDataModules(version) / 8
        - NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version]
}

/// Remainder bits appended after the interleaved codeword stream.
///
/// Some versions have raw-bit-count that is not a multiple of 8. The leftover
/// positions are filled with zero bits (they don't carry data or ECC).
func numRemainderBits(_ version: Int) -> Int {
    return numRawDataModules(version) % 8
}

// ============================================================================
// MARK: - Reed-Solomon (b=0 convention)
// ============================================================================
//
// QR Code uses the b=0 root convention: the generator polynomial's roots are
// α^0=1, α^1=2, …, α^{n-1}.
//
// g(x) = ∏_{i=0}^{n-1} (x + α^i)
//
// This differs from the repo's ReedSolomon package (which uses b=1, roots
// α^1, α^2, …). We embed a minimal standalone RS encoder here rather than
// wrapping the existing package incorrectly.
//
// The encoder is LFSR-based (shift register): it computes the check bytes as
// the remainder of D(x)·x^n mod G(x) without explicit polynomial division.

/// Build the monic RS generator of degree n (b=0 convention).
///
/// Start with g = [1], then for each i in 0..n-1 multiply by (x + α^i):
///
///   g_new[j] = g[j-1] XOR (α^i · g[j])
///
/// Result is big-endian: g[0] is the degree-n coefficient (always 1).
///
/// # Example (n=2)
///
/// Start: [1]
/// i=0, α^0=1: multiply by (x+1) → [1, 1]
/// i=1, α^1=2: multiply by (x+2) → [1, 1^2, 1·2] = [1, 3, 2]
/// ↑ monic: g[0]=1 is the x^2 coefficient.
func buildGenerator(_ n: Int) -> [UInt8] {
    // We accumulate in a big-endian array: index 0 = highest degree.
    var g: [UInt8] = [1]  // g(x) = 1

    for i in 0..<n {
        // alpha_i = α^i in GF(256)
        let ai = GF256.power(2, UInt32(i))

        // Multiply g(x) by (x + α^i).
        // Convolution: new[j] = g[j-1] + α^i·g[j]
        // (Indexing is careful: new has one more element than g.)
        var next = [UInt8](repeating: 0, count: g.count + 1)
        for j in 0..<g.count {
            next[j] ^= g[j]                           // g[j] → next[j]   (coefficient of x^{...} from x term)
            next[j + 1] ^= GF256.multiply(g[j], ai)  // α^i·g[j] → next[j+1]
        }
        g = next
    }
    return g
}

// Cache of generators keyed by degree — build each only once.
// nonisolated(unsafe) because the cache is populated before any concurrent
// access and never mutated after that.
nonisolated(unsafe) private var generatorCache: [Int: [UInt8]] = {
    var cache: [Int: [UInt8]] = [:]
    // Pre-build the degrees used across all 40 QR versions.
    for n in [7, 10, 13, 15, 16, 17, 18, 20, 22, 24, 26, 28, 30] {
        cache[n] = buildGenerator(n)
    }
    return cache
}()

/// Return the cached generator polynomial of degree n, building if needed.
func getGenerator(_ n: Int) -> [UInt8] {
    if let g = generatorCache[n] { return g }
    let g = buildGenerator(n)
    generatorCache[n] = g
    return g
}

/// Compute ECC bytes via LFSR (shift register) polynomial division.
///
/// Computes R(x) = D(x)·x^n mod G(x) where n = degree(G).
///
/// The LFSR approach:
///   - rem is the n-byte shift register, initialised to zero.
///   - For each data byte b:
///       feedback = b XOR rem[0]
///       shift rem left (rem[i] ← rem[i+1])
///       for each coefficient g[i+1] of G: rem[i] ^= g[i+1] · feedback
///
/// After processing all data bytes, rem holds the check bytes.
/// Generator is BIG-ENDIAN (g[0]=1 is the leading coefficient of degree n).
func rsEncode(_ data: [UInt8], _ generator: [UInt8]) -> [UInt8] {
    let n = generator.count - 1  // degree of generator = number of ECC bytes
    var rem = [UInt8](repeating: 0, count: n)

    for b in data {
        let fb = b ^ rem[0]  // feedback byte

        // Shift register left: rem[0] ← rem[1], rem[1] ← rem[2], …
        for i in 0..<(n - 1) {
            rem[i] = rem[i + 1]
        }
        rem[n - 1] = 0

        // XOR feedback × each generator coefficient into register.
        // generator[0] = 1 (monic leading coeff, already consumed by feedback)
        // generator[1..n] drive the feedback path.
        if fb != 0 {
            for i in 0..<n {
                rem[i] ^= GF256.multiply(generator[i + 1], fb)
            }
        }
    }
    return rem
}

// ============================================================================
// MARK: - Data encoding modes
// ============================================================================
//
// QR Code supports three modes for encoding text. The encoder picks the most
// compact mode that can represent the entire input.
//
// Compact mode selection:
//   numeric     → only decimal digits 0-9
//   alphanumeric → digits + uppercase + ' $%*+-./:' (45 chars total)
//   byte        → any UTF-8 byte sequence

/// The 45-character QR alphanumeric alphabet, in value order.
///
/// A character's value is its position in this string (0-44).
/// Pairs encode as: (value0 × 45 + value1) in 11 bits.
/// Trailing single character encodes in 6 bits.
let ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

/// 4-bit mode indicator written before each segment.
///
/// ISO 18004 Table 2.
enum EncodingMode: Int {
    case numeric     = 0b0001
    case alphanumeric = 0b0010
    case byte        = 0b0100
}

/// Select the most compact mode that can represent the entire input.
///
/// Priority: numeric > alphanumeric > byte.
/// A single non-digit forces at least alphanumeric; a non-alphanumeric char
/// forces byte mode.
func selectMode(_ input: String) -> EncodingMode {
    // Check for pure numeric
    if input.unicodeScalars.allSatisfy({ $0.value >= 48 && $0.value <= 57 }) {
        return .numeric
    }
    // Check for pure alphanumeric
    if input.unicodeScalars.allSatisfy({ ALPHANUM_CHARS.contains(Character($0)) }) {
        return .alphanumeric
    }
    return .byte
}

/// Width of the character-count field (depends on mode AND version range).
///
/// ISO 18004 Table 3.
///
/// | Mode         | v1-9 | v10-26 | v27-40 |
/// |--------------|------|--------|--------|
/// | numeric      |  10  |   12   |   14   |
/// | alphanumeric |   9  |   11   |   13   |
/// | byte         |   8  |   16   |   16   |
func charCountBits(_ mode: EncodingMode, _ version: Int) -> Int {
    switch mode {
    case .numeric:
        return version <= 9 ? 10 : version <= 26 ? 12 : 14
    case .alphanumeric:
        return version <= 9 ? 9 : version <= 26 ? 11 : 13
    case .byte:
        return version <= 9 ? 8 : 16
    }
}

// ============================================================================
// MARK: - Bit writer
// ============================================================================
//
// The bit writer accumulates individual bits (MSB first per value) and packs
// them into a byte array. It is used to build the raw data codeword stream.
//
// Bit ordering within each value: the most significant bit is written first.
// Example: write(0b1011, count: 4) appends bits [1, 0, 1, 1].

/// Accumulates bits MSB-first and converts to bytes on demand.
final class BitWriter {
    private var bits: [Bool] = []

    /// Append the `count` most-significant bits of `value`.
    ///
    /// `count` must be ≤ 32. Values are truncated to `count` bits.
    ///
    /// Example: write(0b1011, count: 4) → [1, 0, 1, 1]
    func write(_ value: Int, count: Int) {
        for i in stride(from: count - 1, through: 0, by: -1) {
            bits.append(((value >> i) & 1) == 1)
        }
    }

    /// Total number of bits accumulated so far.
    var bitLength: Int { bits.count }

    /// Pack accumulated bits into bytes (MSB first, zero-padded on the right).
    ///
    /// Bits are grouped in 8. If the total is not a multiple of 8, the last
    /// byte is right-padded with zeros. This padding is harmless because the
    /// caller always pads to a full byte boundary before calling toBytes().
    func toBytes() -> [UInt8] {
        var bytes: [UInt8] = []
        var i = 0
        while i < bits.count {
            var byte: UInt8 = 0
            for j in 0..<8 {
                if (i + j) < bits.count && bits[i + j] {
                    byte |= UInt8(1 << (7 - j))
                }
            }
            bytes.append(byte)
            i += 8
        }
        return bytes
    }
}

// ============================================================================
// MARK: - Mode-specific encoding
// ============================================================================

/// Encode a numeric string into the bit writer.
///
/// Groups of 3 digits → 10 bits.
/// Groups of 2 digits → 7 bits.
/// Single digit      → 4 bits.
///
/// Example: "01234" → write(012, 10) write(34, 7) = 17 bits total.
///
/// Why groups? Each decimal digit needs log2(10) ≈ 3.32 bits. Packing 3 digits
/// into 10 bits gives 3.33 bits/digit — near-optimal.
func encodeNumeric(_ input: String, _ w: BitWriter) {
    let chars = Array(input)
    var i = 0
    while i + 2 < chars.count {
        let val = Int(String(chars[i...i+2]))!
        w.write(val, count: 10)
        i += 3
    }
    if i + 1 < chars.count {
        let val = Int(String(chars[i...i+1]))!
        w.write(val, count: 7)
        i += 2
    }
    if i < chars.count {
        let val = Int(String(chars[i]))!
        w.write(val, count: 4)
    }
}

/// Encode an alphanumeric string into the bit writer.
///
/// Pairs of chars → 11 bits: (value0 × 45 + value1).
/// Trailing single → 6 bits: value0.
///
/// Why 45? There are 45 symbols in the QR alphanumeric set (0-9, A-Z, 7 more).
/// A pair covers 45×45=2025 combinations → ceil(log2(2025)) = 11 bits.
///
/// Pre-condition: every character must be in ALPHANUM_CHARS. (selectMode()
/// already verified this before calling encodeAlphanumeric.)
func encodeAlphanumeric(_ input: String, _ w: BitWriter) throws {
    let chars = Array(input)
    var i = 0
    while i + 1 < chars.count {
        guard let idx0 = ALPHANUM_CHARS.firstIndex(of: chars[i]),
              let idx1 = ALPHANUM_CHARS.firstIndex(of: chars[i + 1]) else {
            throw QrCodeError.encodingError("character '\(chars[i])' not in QR alphanumeric set")
        }
        let v0 = ALPHANUM_CHARS.distance(from: ALPHANUM_CHARS.startIndex, to: idx0)
        let v1 = ALPHANUM_CHARS.distance(from: ALPHANUM_CHARS.startIndex, to: idx1)
        w.write(v0 * 45 + v1, count: 11)
        i += 2
    }
    if i < chars.count {
        guard let idx = ALPHANUM_CHARS.firstIndex(of: chars[i]) else {
            throw QrCodeError.encodingError("character '\(chars[i])' not in QR alphanumeric set")
        }
        let v = ALPHANUM_CHARS.distance(from: ALPHANUM_CHARS.startIndex, to: idx)
        w.write(v, count: 6)
    }
}

/// Encode a string as raw UTF-8 bytes.
///
/// Each byte is written as 8 bits, MSB first. No transformation; the byte
/// values are placed directly into the bit stream.
func encodeByte(_ input: String, _ w: BitWriter) {
    for byte in input.utf8 {
        w.write(Int(byte), count: 8)
    }
}

// ============================================================================
// MARK: - Data codeword assembly
// ============================================================================

/// Assemble the complete data codeword sequence for a given input/version/ECC.
///
/// The output is exactly `numDataCodewords(version, ecc)` bytes.
///
/// ## Stream layout
///
/// ```
/// [4b mode][charCountBits(mode,v) b count][data bits]
///   [≤4b terminator][pad to byte boundary][0xEC/0x11 fill bytes]
/// ```
///
/// The terminator is up to 4 zero bits, shortened if the capacity is already
/// reached. The EC/0x11 alternating padding pattern was chosen by the standard
/// so that the fill bytes don't accidentally create finder-pattern-like data.
func buildDataCodewords(_ input: String, _ version: Int, _ ecc: ErrorCorrectionLevel) throws -> [UInt8] {
    let mode = selectMode(input)
    let capacity = numDataCodewords(version, ecc)
    let w = BitWriter()

    // Mode indicator (4 bits).
    w.write(mode.rawValue, count: 4)

    // Character count field (width varies by mode and version).
    let charCount: Int
    if mode == .byte {
        charCount = input.utf8.count
    } else {
        charCount = input.count
    }
    w.write(charCount, count: charCountBits(mode, version))

    // Data payload.
    switch mode {
    case .numeric:
        encodeNumeric(input, w)
    case .alphanumeric:
        try encodeAlphanumeric(input, w)
    case .byte:
        encodeByte(input, w)
    }

    // Terminator: up to 4 zero bits.
    let termLen = min(4, capacity * 8 - w.bitLength)
    if termLen > 0 { w.write(0, count: termLen) }

    // Pad to byte boundary.
    let rem = w.bitLength % 8
    if rem != 0 { w.write(0, count: 8 - rem) }

    // Byte-level padding with alternating 0xEC / 0x11.
    var bytes = w.toBytes()
    var pad: UInt8 = 0xEC
    while bytes.count < capacity {
        bytes.append(pad)
        pad = (pad == 0xEC) ? 0x11 : 0xEC
    }
    return bytes
}

// ============================================================================
// MARK: - Block processing
// ============================================================================

/// A single RS error-correction block: data codewords + ECC codewords.
struct Block {
    let data: [UInt8]
    let ecc: [UInt8]
}

/// Split the data stream into blocks and compute RS ECC for each.
///
/// ## Why multiple blocks?
///
/// A single RS codeword can be at most 255 bytes (GF(256) limit). Large QR
/// symbols hold much more data than that, so the standard splits data into
/// independent blocks. Each block is encoded with its own RS computation.
///
/// ## Short vs. long blocks
///
/// When the total data doesn't divide evenly, some blocks ("long") get one
/// extra data byte:
///
///   g1Count = totalBlocks - numLong   (these have shortLen bytes each)
///   numLong = totalData % totalBlocks (these have shortLen+1 bytes each)
///
/// This guarantees the total data exactly equals totalData.
func computeBlocks(_ data: [UInt8], _ version: Int, _ ecc: ErrorCorrectionLevel) -> [Block] {
    let e = ecc.index
    let totalBlocks = NUM_BLOCKS[e][version]
    let eccLen = ECC_CODEWORDS_PER_BLOCK[e][version]
    let totalData = numDataCodewords(version, ecc)
    let shortLen = totalData / totalBlocks
    let numLong = totalData % totalBlocks
    let gen = getGenerator(eccLen)
    var blocks: [Block] = []
    var offset = 0

    let g1Count = totalBlocks - numLong
    for _ in 0..<g1Count {
        let d = Array(data[offset..<offset + shortLen])
        blocks.append(Block(data: d, ecc: rsEncode(d, gen)))
        offset += shortLen
    }
    for _ in 0..<numLong {
        let d = Array(data[offset..<offset + shortLen + 1])
        blocks.append(Block(data: d, ecc: rsEncode(d, gen)))
        offset += shortLen + 1
    }
    return blocks
}

/// Interleave codewords across all blocks.
///
/// The interleaved stream is read column-by-column from the block table:
///
///   data[0][0], data[1][0], …, data[k][0],
///   data[0][1], data[1][1], …, data[k][1],
///   …
///   ecc[0][0],  ecc[1][0],  …, ecc[k][0],
///   …
///
/// Why interleave? A burst error (scratch, fold) destroys consecutive bytes.
/// Interleaving spreads consecutive bytes across all blocks, so each block
/// only loses a few scattered bytes — well within its RS correction capacity.
func interleaveBlocks(_ blocks: [Block]) -> [UInt8] {
    var result: [UInt8] = []
    let maxData = blocks.map(\.data.count).max() ?? 0
    let maxEcc  = blocks.map(\.ecc.count).max()  ?? 0
    for i in 0..<maxData {
        for b in blocks where i < b.data.count { result.append(b.data[i]) }
    }
    for i in 0..<maxEcc {
        for b in blocks where i < b.ecc.count { result.append(b.ecc[i]) }
    }
    return result
}

// ============================================================================
// MARK: - Work grid
// ============================================================================
//
// The work grid is a mutable 2D array used internally during encoding.
// It tracks two layers:
//   modules:  the current dark/light state of each module
//   reserved: true for "function" modules (finder, timing, format, etc.)
//             that must not be touched during data placement or masking
//
// The final ModuleGrid (from Barcode2D) is immutable; we produce it only at
// the very end by extracting the modules layer.

struct WorkGrid {
    var size: Int
    var modules:  [[Bool]]   // true = dark
    var reserved: [[Bool]]   // true = structural (skip during data/mask)

    init(size: Int) {
        self.size = size
        self.modules  = [[Bool]](repeating: [Bool](repeating: false, count: size), count: size)
        self.reserved = [[Bool]](repeating: [Bool](repeating: false, count: size), count: size)
    }

    /// Set a module at (row, col). If reserve=true, also mark it reserved.
    mutating func set(_ row: Int, _ col: Int, dark: Bool, reserve: Bool = false) {
        modules[row][col] = dark
        if reserve { reserved[row][col] = true }
    }
}

// ============================================================================
// MARK: - Function pattern placement
// ============================================================================

/// Place the 7×7 finder pattern with top-left corner at (topRow, topCol).
///
/// ```
/// ■ ■ ■ ■ ■ ■ ■    ← row 0 (outer border)
/// ■ □ □ □ □ □ ■
/// ■ □ ■ ■ ■ □ ■    ← rows 2-4: inner 3×3 dark square
/// ■ □ ■ ■ ■ □ ■
/// ■ □ ■ ■ ■ □ ■
/// ■ □ □ □ □ □ ■
/// ■ ■ ■ ■ ■ ■ ■    ← row 6 (outer border)
/// ```
///
/// The 1:1:3:1:1 ratio (dark:light:dark:light:dark) in every scan direction
/// lets decoders locate and orient the symbol even under heavy rotation or
/// partial occlusion.
func placeFinder(_ g: inout WorkGrid, topRow: Int, topCol: Int) {
    for dr in 0..<7 {
        for dc in 0..<7 {
            let onBorder = (dr == 0 || dr == 6 || dc == 0 || dc == 6)
            let inCore   = (dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4)
            g.set(topRow + dr, topCol + dc, dark: onBorder || inCore, reserve: true)
        }
    }
}

/// Place the 5×5 alignment pattern centred at (row, col).
///
/// ```
/// ■ ■ ■ ■ ■
/// ■ □ □ □ ■
/// ■ □ ■ □ ■    ← centre module always dark
/// ■ □ □ □ ■
/// ■ ■ ■ ■ ■
/// ```
///
/// Used in versions 2+. Multiple alignment patterns in large symbols give
/// the decoder interior reference points to correct perspective distortion.
func placeAlignment(_ g: inout WorkGrid, row: Int, col: Int) {
    for dr in -2...2 {
        for dc in -2...2 {
            let onBorder = (abs(dr) == 2 || abs(dc) == 2)
            let isCenter = (dr == 0 && dc == 0)
            g.set(row + dr, col + dc, dark: onBorder || isCenter, reserve: true)
        }
    }
}

/// Place all alignment patterns for the given version.
///
/// All cross-product pairs of ALIGNMENT_POSITIONS[version-1] are considered.
/// Pairs whose centre lands on an already-reserved module (finder, separator,
/// timing) are skipped — this naturally excludes the three finder overlaps.
func placeAllAlignments(_ g: inout WorkGrid, version: Int) {
    let positions = ALIGNMENT_POSITIONS[version - 1]
    for row in positions {
        for col in positions {
            if g.reserved[row][col] { continue }  // overlaps finder/timing
            placeAlignment(&g, row: row, col: col)
        }
    }
}

/// Place the horizontal and vertical timing strips.
///
/// Row 6 (horizontal): cols 8 … size-9, dark when col is even.
/// Col 6 (vertical):   rows 8 … size-9, dark when row is even.
///
/// The alternating dark/light pattern gives the decoder a ruler to measure
/// module size and correct for non-uniform scaling.
func placeTimingStrips(_ g: inout WorkGrid) {
    let sz = g.size
    for c in 8...(sz - 9) {
        g.set(6, c, dark: c % 2 == 0, reserve: true)
    }
    for r in 8...(sz - 9) {
        g.set(r, 6, dark: r % 2 == 0, reserve: true)
    }
}

/// Reserve all format information module positions (write false; actual bits
/// are filled in after mask selection).
///
/// Copy 1 — adjacent to the top-left finder:
///   Row 8, cols 0..5 and 7..8.
///   Col 8, rows 0..5 and 7..8.
///
/// Copy 2 — adjacent to the other two finders:
///   Col 8, rows size-7 … size-1.
///   Row 8, cols size-8 … size-1.
func reserveFormatInfo(_ g: inout WorkGrid) {
    let sz = g.size
    for c in 0...8 where c != 6 { g.reserved[8][c] = true }
    for r in 0...8 where r != 6 { g.reserved[r][8] = true }
    for r in (sz - 7)..<sz { g.reserved[r][8] = true }
    for c in (sz - 8)..<sz { g.reserved[8][c] = true }
}

/// Reserve version information positions (v7+): two 6×3 blocks.
///
/// Top-right block:    rows 0..5, cols size-11..size-9.
/// Bottom-left block:  rows size-11..size-9, cols 0..5.
///
/// Version information encodes the version number (7-40) as an 18-bit BCH
/// codeword. Without it, decoders would have to try every possible version to
/// find the format information — prohibitively slow.
func reserveVersionInfo(_ g: inout WorkGrid, version: Int) {
    guard version >= 7 else { return }
    let sz = g.size
    for r in 0..<6 { for dc in 0..<3 { g.reserved[r][sz - 11 + dc] = true } }
    for dr in 0..<3 { for c in 0..<6 { g.reserved[sz - 11 + dr][c] = true } }
}

/// Place the always-dark module at (4V+9, 8).
///
/// This module is always dark in every valid QR symbol. It is not part of the
/// data, format information, or timing strips — it just happens to fall at this
/// fixed position. Decoders can use it as a quick sanity check.
func placeDarkModule(_ g: inout WorkGrid, version: Int) {
    g.set(4 * version + 9, 8, dark: true, reserve: true)
}

// ============================================================================
// MARK: - Format information
// ============================================================================

/// Compute the 15-bit format information string.
///
/// ## Steps
///
/// 1. 5-bit data: [ECC bits (2b)][mask (3b)]
/// 2. BCH(15,5) error correction: append the remainder of
///    (data × x^10) mod G(x), where G(x) = 0x537.
/// 3. XOR with 0x5412 to prevent all-zero format information.
///
/// G(x) = x^10 + x^8 + x^5 + x^4 + x^2 + x + 1 = 0x537.
///
/// The XOR mask 0x5412 was chosen so that no valid ECC/mask combination
/// produces an all-zero format info string (which could be confused with an
/// unformatted symbol).
func computeFormatBits(_ ecc: ErrorCorrectionLevel, mask: Int) -> Int {
    let data = (ecc.formatBits << 3) | mask
    var rem = data << 10
    for i in stride(from: 14, through: 10, by: -1) {
        if ((rem >> i) & 1) != 0 { rem ^= (0x537 << (i - 10)) }
    }
    return ((data << 10) | (rem & 0x3FF)) ^ 0x5412
}

/// Write the 15-bit format information into both copy locations.
///
/// ## Copy 1 (adjacent to top-left finder)
///
/// Bits 0-5 → (8, 0..5)   [left of top-left finder, horizontal]
/// Bit  6   → (8, 7)       [skip col 6 = timing]
/// Bit  7   → (8, 8)       [corner]
/// Bit  8   → (7, 8)       [skip row 6 = timing]
/// Bits 9-14 → (5..0, 8)  [above top-left finder, vertical, reversed]
///
/// ## Copy 2 (adjacent to top-right and bottom-left finders)
///
/// Bits 0-6  → (size-1..size-7, 8)   [bottom-left area]
/// Bits 7-14 → (8, size-8..size-1)   [top-right area]
func writeFormatInfo(_ g: inout WorkGrid, fmtBits: Int) {
    let sz = g.size

    // Copy 1
    for i in 0...5 { g.modules[8][i] = ((fmtBits >> i) & 1) == 1 }
    g.modules[8][7] = ((fmtBits >> 6) & 1) == 1
    g.modules[8][8] = ((fmtBits >> 7) & 1) == 1
    g.modules[7][8] = ((fmtBits >> 8) & 1) == 1
    for i in 9...14 { g.modules[14 - i][8] = ((fmtBits >> i) & 1) == 1 }

    // Copy 2
    for i in 0...6 { g.modules[sz - 1 - i][8] = ((fmtBits >> i) & 1) == 1 }
    for i in 7...14 { g.modules[8][sz - 15 + i] = ((fmtBits >> i) & 1) == 1 }
}

// ============================================================================
// MARK: - Version information (v7+)
// ============================================================================

/// Compute the 18-bit version information codeword (v7+).
///
/// ## Steps
///
/// 1. 6-bit version number (7..40).
/// 2. BCH(18,6): append the remainder of (version × x^12) mod G(x).
///    G(x) = x^12 + x^11 + x^10 + x^9 + x^8 + x^5 + x^2 + 1 = 0x1F25.
///
/// The 18-bit result is placed in two 6×3 blocks so decoders can reliably
/// read the version number even if part of the symbol is damaged.
func computeVersionBits(_ version: Int) -> Int {
    var rem = version << 12
    for i in stride(from: 17, through: 12, by: -1) {
        if ((rem >> i) & 1) != 0 { rem ^= (0x1F25 << (i - 12)) }
    }
    return (version << 12) | (rem & 0xFFF)
}

/// Write version information into both 6×3 blocks (v7+).
///
/// Top-right block (rows 0..5, cols size-11..size-9):
///   bit i → (5 − ⌊i/3⌋, size−9−(i%3))
///
/// Bottom-left block (rows size-11..size-9, cols 0..5):
///   bit i → (size−9−(i%3), 5−⌊i/3⌋)   ← transposed
func writeVersionInfo(_ g: inout WorkGrid, version: Int) {
    guard version >= 7 else { return }
    let sz = g.size
    let bits = computeVersionBits(version)
    for i in 0..<18 {
        let dark = ((bits >> i) & 1) == 1
        let a = 5 - (i / 3)
        let b = sz - 9 - (i % 3)
        g.modules[a][b] = dark     // top-right block
        g.modules[b][a] = dark     // bottom-left block (transposed)
    }
}

// ============================================================================
// MARK: - Grid construction
// ============================================================================

/// Build and reserve all structural (non-data) modules for a given version.
///
/// After this call the work grid contains:
///   • Three finder patterns in the three corners.
///   • 1-module separator (light) strips around each finder.
///   • Timing strips on row 6 and col 6.
///   • All alignment patterns (version 2+).
///   • Reserved format info positions (15 per copy × 2 copies).
///   • Reserved version info positions (v7+).
///   • The always-dark module.
///
/// Data bits are placed into the remaining non-reserved modules by `placeBits`.
func buildGrid(version: Int) -> WorkGrid {
    let sz = symbolSize(version)
    var g = WorkGrid(size: sz)

    // Three finder patterns, one in each corner.
    placeFinder(&g, topRow: 0,       topCol: 0)       // top-left
    placeFinder(&g, topRow: 0,       topCol: sz - 7)  // top-right
    placeFinder(&g, topRow: sz - 7,  topCol: 0)       // bottom-left

    // Separator strips: 1-module light border just outside each finder.
    //
    // Top-left: row 7 (cols 0..7) + col 7 (rows 0..7)
    for i in 0...7 { g.set(7, i, dark: false, reserve: true) }
    for i in 0...7 { g.set(i, 7, dark: false, reserve: true) }
    // Top-right: row 7 (cols sz-8..sz-1) + col sz-8 (rows 0..7)
    for i in 0...7 { g.set(7, sz - 1 - i, dark: false, reserve: true) }
    for i in 0...7 { g.set(i, sz - 8,     dark: false, reserve: true) }
    // Bottom-left: row sz-8 (cols 0..7) + col 7 (rows sz-7..sz-1)
    for i in 0...7 { g.set(sz - 8, i, dark: false, reserve: true) }
    for i in 0...7 { g.set(sz - 1 - i, 7, dark: false, reserve: true) }

    // Timing strips (must precede alignment patterns: they set row/col 6 reserved)
    placeTimingStrips(&g)

    // Alignment patterns (for version 2+)
    placeAllAlignments(&g, version: version)

    // Format and version info reservations
    reserveFormatInfo(&g)
    reserveVersionInfo(&g, version: version)

    // Always-dark module
    placeDarkModule(&g, version: version)

    return g
}

// ============================================================================
// MARK: - Data placement (zigzag scan)
// ============================================================================

/// Place the interleaved codeword stream using the two-column zigzag scan.
///
/// ## Scan pattern
///
/// The QR standard scans from the bottom-right corner leftward in 2-column
/// strips. Each strip alternates direction (up/down):
///
/// ```
/// col pair: ...  9,8    7,6*   5,4    3,2    1,0
///                 ↑      ↑      ↓      ↑      ↓    (alternates)
/// ```
///
/// (*) Column 6 is the vertical timing strip and is always skipped. When the
///     scanner reaches col 7 and would move to col 6, it jumps to col 5.
///
/// ## Reserved modules
///
/// Every module marked `reserved` is skipped. Data bits only fill the
/// non-reserved positions.
///
/// ## Remainder bits
///
/// After all codeword bits are placed, `numRemainderBits(version)` additional
/// zero bits fill the remaining positions in the last column pair.
func placeBits(_ g: inout WorkGrid, codewords: [UInt8], version: Int) {
    let sz = g.size

    // Flatten codewords to a bit array (MSB first per byte).
    var bits: [Bool] = []
    bits.reserveCapacity(codewords.count * 8 + numRemainderBits(version))
    for cw in codewords {
        for b in stride(from: 7, through: 0, by: -1) {
            bits.append(((cw >> b) & 1) == 1)
        }
    }
    for _ in 0..<numRemainderBits(version) { bits.append(false) }

    var bitIdx = 0
    var up = true        // true = scan bottom→top
    var col = sz - 1     // leading column (right of each 2-column pair)

    while col >= 1 {
        for vi in 0..<sz {
            let row = up ? sz - 1 - vi : vi
            for dc in 0...1 {
                let c = col - dc
                if c == 6 { continue }          // skip timing column
                if g.reserved[row][c] { continue }
                g.modules[row][c] = bitIdx < bits.count ? bits[bitIdx] : false
                bitIdx += 1
            }
        }
        up = !up
        col -= 2
        if col == 6 { col = 5 }  // hop over the vertical timing strip
    }
}

// ============================================================================
// MARK: - Masking
// ============================================================================
//
// QR masking XORs data (non-reserved) modules with a repeating pattern.
// The purpose: prevent long runs of same-colour modules that could confuse
// scanner hardware. The encoder evaluates all 8 patterns and picks the one
// with the lowest penalty score.

/// Apply mask pattern `maskIdx` to the module array.
///
/// The 8 mask conditions (ISO 18004 Table 10):
///
/// | idx | condition                              |
/// |-----|----------------------------------------|
/// |  0  | (row + col) % 2 == 0                   |
/// |  1  | row % 2 == 0                           |
/// |  2  | col % 3 == 0                           |
/// |  3  | (row + col) % 3 == 0                   |
/// |  4  | (⌊row/2⌋ + ⌊col/3⌋) % 2 == 0          |
/// |  5  | (row·col)%2 + (row·col)%3 == 0         |
/// |  6  | ((row·col)%2 + (row·col)%3) % 2 == 0  |
/// |  7  | ((row+col)%2 + (row·col)%3) % 2 == 0  |
///
/// If the condition is true for a non-reserved module, its colour is flipped.
/// Reserved (structural) modules are never masked.
///
/// Returns a NEW module array — the input is not modified.
func applyMask(modules: [[Bool]], reserved: [[Bool]], size sz: Int, maskIdx: Int) -> [[Bool]] {
    var masked = modules
    for r in 0..<sz {
        for c in 0..<sz {
            if reserved[r][c] { continue }
            let flip: Bool
            switch maskIdx {
            case 0: flip = (r + c) % 2 == 0
            case 1: flip = r % 2 == 0
            case 2: flip = c % 3 == 0
            case 3: flip = (r + c) % 3 == 0
            case 4: flip = (r / 2 + c / 3) % 2 == 0
            case 5: flip = (r * c) % 2 + (r * c) % 3 == 0
            case 6: flip = ((r * c) % 2 + (r * c) % 3) % 2 == 0
            case 7: flip = ((r + c) % 2 + (r * c) % 3) % 2 == 0
            default: flip = false
            }
            if flip { masked[r][c] = !masked[r][c] }
        }
    }
    return masked
}

// ============================================================================
// MARK: - Penalty scoring (ISO 18004 Section 7.8.3)
// ============================================================================
//
// The penalty score measures how "bad" a masked pattern looks to a scanner.
// Four rules are applied; the mask with the lowest total penalty wins.

/// Compute the 4-rule penalty score for a masked module array.
///
/// ## Rule 1 — Adjacent same-colour runs
///
/// Any run of ≥ 5 identical modules in the same row or column scores
/// (run_length − 2) points. Runs of exactly 5 score 3 points.
///
/// Motivation: long same-colour runs look like solid bars, which confuse
/// scanners optimised for high-contrast transitions.
///
/// ## Rule 2 — 2×2 same-colour blocks
///
/// Every 2×2 block of identical modules scores 3 points.
///
/// Motivation: large filled areas have no transitions for scanner sync.
///
/// ## Rule 3 — Finder-like patterns
///
/// The patterns 1011101_0000 and 0000_1011101 (horizontal and vertical) each
/// score 40 points per occurrence.
///
/// Motivation: these look like finder patterns (1:1:3:1:1) preceded or
/// followed by a quiet zone. False finder detection causes catastrophic
/// misalignment.
///
/// ## Rule 4 — Dark module proportion
///
/// Compute percent dark = 100 × (darkCount / totalModules).
/// Round down to nearest multiple of 5, and round up.
/// Add 10 × min(|floor - 50|, |ceil - 50|) / 5.
///
/// Motivation: a symbol that is mostly dark or mostly light is harder to scan
/// with automatic brightness adjustment.
func computePenalty(modules: [[Bool]], size sz: Int) -> Int {
    var penalty = 0

    // ── Rule 1: adjacent same-colour runs of length ≥ 5 ─────────────────────
    for a in 0..<sz {
        for horiz in [true, false] {
            var run = 1
            var prev = horiz ? modules[a][0] : modules[0][a]
            for i in 1..<sz {
                let cur = horiz ? modules[a][i] : modules[i][a]
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

    // ── Rule 2: 2×2 same-colour blocks ──────────────────────────────────────
    for r in 0..<(sz - 1) {
        for c in 0..<(sz - 1) {
            let d = modules[r][c]
            if d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1] {
                penalty += 3
            }
        }
    }

    // ── Rule 3: finder-pattern-like sequences ────────────────────────────────
    // P1 = 1,0,1,1,1,0,1,0,0,0,0  and  P2 = 0,0,0,0,1,0,1,1,1,0,1
    let P1: [Int] = [1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0]
    let P2: [Int] = [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1]
    for a in 0..<sz {
        for b in 0...(sz - 11) {
            var mH1 = true, mH2 = true, mV1 = true, mV2 = true
            for k in 0..<11 {
                let bH = modules[a][b + k] ? 1 : 0
                let bV = modules[b + k][a] ? 1 : 0
                if bH != P1[k] { mH1 = false }
                if bH != P2[k] { mH2 = false }
                if bV != P1[k] { mV1 = false }
                if bV != P2[k] { mV2 = false }
            }
            if mH1 { penalty += 40 }
            if mH2 { penalty += 40 }
            if mV1 { penalty += 40 }
            if mV2 { penalty += 40 }
        }
    }

    // ── Rule 4: dark proportion deviation from 50% ──────────────────────────
    var darkCount = 0
    for r in 0..<sz { for c in 0..<sz { if modules[r][c] { darkCount += 1 } } }
    let total = sz * sz
    let ratio = (darkCount * 100) / total     // integer percent, truncated
    let prev5 = (ratio / 5) * 5               // floor to nearest 5%
    let lo = abs(prev5 - 50)
    let hi = abs(prev5 + 5 - 50)
    penalty += min(lo, hi) / 5 * 10

    return penalty
}

// ============================================================================
// MARK: - Version selection
// ============================================================================

/// Find the minimum QR version (1-40) whose data capacity fits the input.
///
/// Uses the exact bit count for the selected mode, including the mode indicator
/// (4 bits) and the character-count field (width varies with version).
///
/// Throws `QrCodeError.inputTooLong` if even version 40 cannot hold the input.
func selectVersion(_ input: String, ecc: ErrorCorrectionLevel) throws -> Int {
    let mode = selectMode(input)
    let byteLen = input.utf8.count

    for v in 1...40 {
        let capacity = numDataCodewords(v, ecc)
        let dataBits: Int
        switch mode {
        case .byte:
            dataBits = byteLen * 8
        case .numeric:
            // 10 bits per 3 digits (ceil), but character count field changes at v10 and v27.
            // Simpler: use ceiling division directly.
            dataBits = ((input.count * 10) + 2) / 3  // ≈ ceil(n * 10/3)
        case .alphanumeric:
            // 11 bits per 2 chars (ceil).
            dataBits = ((input.count * 11) + 1) / 2  // ≈ ceil(n * 11/2)
        }
        let bitsNeeded = 4 + charCountBits(mode, v) + dataBits
        if (bitsNeeded + 7) / 8 <= capacity { return v }
    }
    throw QrCodeError.inputTooLong(
        "Input (\(input.count) chars, ECC=\(ecc)) exceeds version-40 capacity."
    )
}

// ============================================================================
// MARK: - Public API
// ============================================================================

/// Encode a UTF-8 string into a QR Code ModuleGrid.
///
/// Returns a `(4V+17) × (4V+17)` boolean grid where `true` = dark module.
/// The version V is selected automatically as the smallest version that fits
/// the input at the chosen error correction level.
///
/// ## Usage
///
/// ```swift
/// let grid = try QrCode.encode("https://example.com")
/// // grid is a Barcode2D.ModuleGrid
/// // grid.rows == grid.cols == 29  (for this input at .medium)
/// ```
///
/// ## Error handling
///
/// - `QrCodeError.inputTooLong` if the input exceeds version-40 capacity.
///   Version 40 at level L holds at most 7,089 numeric characters or 2,953
///   bytes. Use `level: .low` and numeric/alphanumeric encoding if you need
///   maximum capacity.
///
/// - Parameter data: The UTF-8 string to encode.
/// - Parameter level: Error correction level. Defaults to `.medium`.
/// - Returns: A square `ModuleGrid` of size (4V+17).
/// - Throws: `QrCodeError.inputTooLong` or `QrCodeError.encodingError`.
public enum QrCode {

    public static func encode(
        _ data: String,
        level: ErrorCorrectionLevel = .medium
    ) throws -> ModuleGrid {
        // Guard against DoS-amplifying inputs. QR v40 numeric holds 7089 chars.
        // Without this guard, selectVersion() would TextEncoder-encode the input
        // up to 40 times before throwing.
        if data.count > 7089 {
            throw QrCodeError.inputTooLong(
                "Input length \(data.count) exceeds 7089 (QR Code v40 numeric-mode maximum)."
            )
        }

        let version = try selectVersion(data, ecc: level)
        let sz      = symbolSize(version)

        // 1. Build data codeword stream.
        let dataCW = try buildDataCodewords(data, version, level)

        // 2. Split into RS blocks and compute ECC.
        let blocks = computeBlocks(dataCW, version, level)

        // 3. Interleave blocks.
        let interleaved = interleaveBlocks(blocks)

        // 4. Initialise grid with all structural modules.
        var grid = buildGrid(version: version)

        // 5. Place data/ECC bits via zigzag scan.
        placeBits(&grid, codewords: interleaved, version: version)

        // 6. Evaluate all 8 masks; pick the one with lowest penalty.
        var bestMask = 0
        var bestPenalty = Int.max

        for m in 0..<8 {
            var masked = applyMask(modules: grid.modules, reserved: grid.reserved, size: sz, maskIdx: m)
            // Write format info into a temporary copy to score it correctly.
            let fmtBits = computeFormatBits(level, mask: m)
            var testGrid = WorkGrid(size: sz)
            testGrid.modules  = masked
            testGrid.reserved = grid.reserved
            writeFormatInfo(&testGrid, fmtBits: fmtBits)
            masked = testGrid.modules

            let p = computePenalty(modules: masked, size: sz)
            if p < bestPenalty { bestPenalty = p; bestMask = m }
        }

        // 7. Apply best mask and write final format + version info.
        let finalMods = applyMask(modules: grid.modules, reserved: grid.reserved, size: sz, maskIdx: bestMask)
        var finalGrid = WorkGrid(size: sz)
        finalGrid.modules  = finalMods
        finalGrid.reserved = grid.reserved
        writeFormatInfo(&finalGrid, fmtBits: computeFormatBits(level, mask: bestMask))
        writeVersionInfo(&finalGrid, version: version)

        return ModuleGrid(cols: sz, rows: sz, modules: finalGrid.modules, moduleShape: .square)
    }
}
