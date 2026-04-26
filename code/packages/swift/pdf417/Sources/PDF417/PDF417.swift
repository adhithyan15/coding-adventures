// PDF417.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - PDF417 Encoder (ISO/IEC 15438:2015)
// ============================================================================
//
// PDF417 (Portable Data File 417) is a stacked linear barcode invented by
// Ynjiun P. Wang at Symbol Technologies in 1991. The "417" in the name
// encodes its geometry: every codeword has exactly **4** bars and **4**
// spaces (8 elements) and occupies exactly **17** modules of horizontal space.
//
// ## Where PDF417 is used
//
// | Application      | Detail                                                |
// |------------------|-------------------------------------------------------|
// | AAMVA            | North American driver's licences and government IDs   |
// | IATA BCBP        | Airline boarding passes                               |
// | USPS             | Domestic shipping labels                              |
// | US immigration   | Form I-94, customs declarations                       |
// | Healthcare       | Patient wristbands, medication labels                 |
//
// ## Encoding pipeline
//
// ```
// raw bytes
//   → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
//   → length descriptor   (codeword 0 = total codewords in symbol)
//   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
//   → dimension selection (auto: roughly square symbol)
//   → padding             (codeword 900 fills unused slots)
//   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
//   → cluster table lookup (codeword → 17-module bar/space pattern)
//   → start/stop patterns (fixed per row)
//   → ModuleGrid          (abstract boolean grid)
// ```
//
// ## v0.1.0 scope — byte compaction only
//
// This Swift port matches the TypeScript reference implementation: byte
// compaction is the sole compaction mode. Text and numeric compaction modes
// are deferred to a future release. Byte mode handles **any** input
// (including ASCII text) at the cost of being less compact than the
// specialized modes — about 1.2 codewords per byte.
//
// ============================================================================

import Barcode2D
import PaintInstructions

// ============================================================================
// MARK: - Version
// ============================================================================

/// Current package version.
public let version = "0.1.0"

// ============================================================================
// MARK: - PDF417Error
// ============================================================================

/// Errors thrown by the PDF417 encoder.
///
/// All errors carry a human-readable message describing what went wrong and,
/// where possible, the offending value (e.g. the requested ECC level or
/// column count). The cases mirror the TypeScript reference's error classes:
///
/// | Case                  | TypeScript class           |
/// |-----------------------|----------------------------|
/// | `.inputTooLong`       | `InputTooLongError`        |
/// | `.invalidDimensions`  | `InvalidDimensionsError`   |
/// | `.invalidECCLevel`    | `InvalidECCLevelError`     |
/// | `.layoutError`        | (wraps Barcode2D errors)   |
public enum PDF417Error: Error, Equatable {
    /// The input data is too long to fit in any valid PDF417 symbol given
    /// the requested column count, or larger than 90 logical rows × 30 cols.
    case inputTooLong(String)

    /// User-supplied `columns` is outside the valid range 1–30.
    case invalidDimensions(String)

    /// User-supplied `eccLevel` is outside the valid range 0–8.
    case invalidECCLevel(String)

    /// Wrapping error from the Barcode2D layout layer.
    case layoutError(String)
}

// ============================================================================
// MARK: - PDF417Options
// ============================================================================

/// Options controlling how the PDF417 symbol is encoded.
///
/// Every field has a sensible default. Just pass `PDF417Options()` for the
/// defaults; override only what you need.
///
/// ```swift
/// // Default encoding — auto-selected ECC, auto-selected dimensions.
/// let g = try encode("HELLO")
///
/// // Force a specific shape and stronger ECC.
/// let g2 = try encode("HELLO", options: PDF417Options(
///     eccLevel: 4,
///     columns: 6,
///     rowHeight: 4
/// ))
/// ```
public struct PDF417Options: Equatable, Sendable {
    /// Reed-Solomon error correction level, 0–8. Higher levels use more ECC
    /// codewords (level k uses 2^(k+1) codewords). `nil` = auto-select based
    /// on data size.
    public var eccLevel: Int?

    /// Number of data columns, 1–30. `nil` = auto-select to produce a
    /// roughly square symbol.
    public var columns: Int?

    /// Module-rows per logical PDF417 row, ≥ 1. Larger values produce taller
    /// symbols. Defaults to 3 — the same as the TypeScript reference.
    public var rowHeight: Int

    public init(eccLevel: Int? = nil, columns: Int? = nil, rowHeight: Int = 3) {
        self.eccLevel = eccLevel
        self.columns = columns
        self.rowHeight = rowHeight
    }
}

// ============================================================================
// MARK: - Constants
// ============================================================================

/// GF(929) prime modulus.
let PDF417_GF929_PRIME: Int = 929

/// Generator element α = 3 (a primitive root mod 929).
let PDF417_GF929_ALPHA: Int = 3

/// Multiplicative-group order of GF(929) = PRIME − 1 = 928.
let PDF417_GF929_ORDER: Int = 928

/// Latch-to-byte-compaction codeword (alternate form, accepts any length).
let PDF417_LATCH_BYTE: Int = 924

/// Padding codeword (neutral filler; appears only after RS ECC is computed).
let PDF417_PADDING_CW: Int = 900

let PDF417_MIN_ROWS: Int = 3
let PDF417_MAX_ROWS: Int = 90
let PDF417_MIN_COLS: Int = 1
let PDF417_MAX_COLS: Int = 30

// ============================================================================
// MARK: - GF(929) arithmetic
// ============================================================================
//
// GF(929) is the integers modulo 929. Since 929 is prime, every non-zero
// element has a multiplicative inverse. We use log/antilog lookup tables for
// O(1) multiplication, built once at module load time.
//
// Memory cost: 929 entries × 2 tables × 8 bytes (Swift Int) ≈ 14.9 KB.
// Trivial compared to any barcode render time.

/// `α^i mod 929` for i in 0..<929. Index 928 wraps back to GF_EXP[0] = 1.
let PDF417_GF_EXP: [Int] = {
    // We allocate 929 slots so that `gfMul`'s `(la + lb) % 928` index can
    // never exceed 928 — the sentinel slot saves us a wraparound branch.
    var arr = [Int](repeating: 0, count: 929)
    var v = 1
    for i in 0..<PDF417_GF929_ORDER {
        arr[i] = v
        v = (v * PDF417_GF929_ALPHA) % PDF417_GF929_PRIME
    }
    arr[PDF417_GF929_ORDER] = arr[0]  // wrap-around sentinel
    return arr
}()

/// Inverse of GF_EXP: `GF_LOG[v] = i` such that `α^i ≡ v (mod 929)`.
/// `GF_LOG[0]` is unused (log of 0 is undefined).
let PDF417_GF_LOG: [Int] = {
    var arr = [Int](repeating: 0, count: 929)
    var v = 1
    for i in 0..<PDF417_GF929_ORDER {
        arr[v] = i
        v = (v * PDF417_GF929_ALPHA) % PDF417_GF929_PRIME
    }
    return arr
}()

/// Multiply two elements of GF(929) using log/antilog tables.
///
/// Returns 0 if either operand is 0 (the absorbing element).
@inline(__always)
func pdf417GFMul(_ a: Int, _ b: Int) -> Int {
    if a == 0 || b == 0 { return 0 }
    return PDF417_GF_EXP[(PDF417_GF_LOG[a] + PDF417_GF_LOG[b]) % PDF417_GF929_ORDER]
}

/// Add two elements of GF(929): `(a + b) mod 929`.
@inline(__always)
func pdf417GFAdd(_ a: Int, _ b: Int) -> Int {
    return (a + b) % PDF417_GF929_PRIME
}

// ============================================================================
// MARK: - Reed-Solomon generator polynomial
// ============================================================================
//
// For ECC level L, k = 2^(L+1) ECC codewords. The generator polynomial uses
// the b=3 convention: roots are α^3, α^4, ..., α^(k+2).
//
//   g(x) = (x − α^3)(x − α^4) ··· (x − α^(k+2))
//
// We build g iteratively by multiplying in each linear factor (x − α^j).
//
// Returns k+1 coefficients [g_k, g_{k−1}, ..., g_1, g_0] where g_k = 1
// (leading coefficient — the polynomial is monic).

/// Build the RS generator polynomial for ECC level `eccLevel` (0–8).
///
/// The returned array has length `k + 1` where `k = 2^(eccLevel + 1)`.
func pdf417BuildGenerator(eccLevel: Int) -> [Int] {
    let k = 1 << (eccLevel + 1)  // 2^(eccLevel + 1)
    var g: [Int] = [1]

    for j in 3...(k + 2) {
        // α^j (mod 929). The order is 928, so we modulo j into [0, 928).
        let root = PDF417_GF_EXP[j % PDF417_GF929_ORDER]
        // Negate inside GF(929): −x ≡ (PRIME − x).
        let negRoot = PDF417_GF929_PRIME - root

        // Multiply the running polynomial g by the linear factor (x − α^j).
        var newG = [Int](repeating: 0, count: g.count + 1)
        for i in 0..<g.count {
            newG[i] = pdf417GFAdd(newG[i], g[i])
            newG[i + 1] = pdf417GFAdd(newG[i + 1], pdf417GFMul(g[i], negRoot))
        }
        g = newG
    }

    return g
}

// ============================================================================
// MARK: - Reed-Solomon encoder
// ============================================================================
//
// Standard shift-register (LFSR) polynomial long-division algorithm. We
// produce `k = 2^(L+1)` ECC codewords from the input data.
//
// Unlike QR Code, PDF417 does NOT interleave ECC blocks — one Reed-Solomon
// encoder consumes all input data in one pass, which keeps the
// implementation refreshingly simple.

/// Compute `k` Reed-Solomon ECC codewords for `data` over GF(929) with the
/// b=3 convention.
func pdf417RSEncode(data: [Int], eccLevel: Int) -> [Int] {
    let g = pdf417BuildGenerator(eccLevel: eccLevel)
    let k = g.count - 1
    var ecc = [Int](repeating: 0, count: k)

    for d in data {
        let feedback = pdf417GFAdd(d, ecc[0])

        // Shift register left.
        for i in 0..<(k - 1) {
            ecc[i] = ecc[i + 1]
        }
        ecc[k - 1] = 0

        // Add `feedback × generator coefficient` to each cell.
        for i in 0..<k {
            ecc[i] = pdf417GFAdd(ecc[i], pdf417GFMul(g[k - i], feedback))
        }
    }

    return ecc
}

// ============================================================================
// MARK: - Byte compaction
// ============================================================================
//
// Byte compaction packs raw bytes into base-900 codewords:
//
//   - 6 bytes are treated as a 48-bit big-endian integer and converted to 5
//     base-900 codewords (most-significant codeword first).
//   - 1–5 trailing bytes are encoded directly, one byte per codeword (the
//     "alternate" sub-mode triggered by latch codeword 924).
//
// We use Swift's built-in `UInt64` for the 48-bit arithmetic — 48 bits fit
// comfortably in 64 bits with no overflow.

/// Encode raw bytes using byte compaction mode (latch codeword 924).
///
/// Returns `[924, c1, c2, ...]` where the `c_i` are byte-compacted codewords.
func pdf417ByteCompact(bytes: [UInt8]) -> [Int] {
    var codewords: [Int] = [PDF417_LATCH_BYTE]
    var i = 0
    let len = bytes.count

    // ── Process full 6-byte groups ─────────────────────────────────────────
    // 6 bytes (48 bits) → 5 base-900 codewords. UInt64 has 64 bits of
    // precision, so the multiply-by-256 chain never overflows.
    while i + 6 <= len {
        var n: UInt64 = 0
        for j in 0..<6 {
            n = n &* 256 &+ UInt64(bytes[i + j])
        }
        // Convert n to base 900 → 5 codewords, most-significant first.
        var group = [Int](repeating: 0, count: 5)
        for j in stride(from: 4, through: 0, by: -1) {
            group[j] = Int(n % 900)
            n /= 900
        }
        codewords.append(contentsOf: group)
        i += 6
    }

    // ── Remaining 1–5 bytes ───────────────────────────────────────────────
    // Direct encoding: one codeword per byte.
    while i < len {
        codewords.append(Int(bytes[i]))
        i += 1
    }

    return codewords
}

// ============================================================================
// MARK: - ECC level auto-selection
// ============================================================================

/// Select the minimum recommended ECC level based on data codeword count.
///
/// These thresholds match ISO/IEC 15438 Annex E recommendations and the
/// reference TypeScript / Java implementations.
func pdf417AutoEccLevel(dataCount: Int) -> Int {
    if dataCount <= 40 { return 2 }
    if dataCount <= 160 { return 3 }
    if dataCount <= 320 { return 4 }
    if dataCount <= 863 { return 5 }
    return 6
}

// ============================================================================
// MARK: - Dimension selection
// ============================================================================

/// Choose `(cols, rows)` for a symbol holding `total` codewords.
///
/// Heuristic: `c = ⌈√(total / 3)⌉`, clamped to 1..30. Then `r = ⌈total / c⌉`,
/// clamped to 3..90. Aims for roughly square symbols.
func pdf417ChooseDimensions(total: Int) -> (cols: Int, rows: Int) {
    // Use Double to compute the initial column estimate.
    let raw = Int((Double(total) / 3.0).squareRoot().rounded(.up))
    var c = max(PDF417_MIN_COLS, min(PDF417_MAX_COLS, raw))
    var r = max(PDF417_MIN_ROWS, Int((Double(total) / Double(c)).rounded(.up)))

    if r < PDF417_MIN_ROWS {
        r = PDF417_MIN_ROWS
        let cc = Int((Double(total) / Double(r)).rounded(.up))
        c = max(PDF417_MIN_COLS, min(PDF417_MAX_COLS, cc))
        r = max(PDF417_MIN_ROWS, Int((Double(total) / Double(c)).rounded(.up)))
    }

    r = min(PDF417_MAX_ROWS, r)
    return (c, r)
}

// ============================================================================
// MARK: - Row indicator computation
// ============================================================================
//
// Each row carries two row-indicator codewords that together encode:
//
//   R_info = (R − 1) / 3      (total rows information)
//   C_info = C − 1            (columns information)
//   L_info = 3·L + (R − 1) % 3 (ECC level + row parity)
//
// For row r (0-indexed) with `cluster = r % 3`:
//
//   Cluster 0: LRI = 30·(r/3) + R_info,  RRI = 30·(r/3) + C_info
//   Cluster 1: LRI = 30·(r/3) + L_info,  RRI = 30·(r/3) + R_info
//   Cluster 2: LRI = 30·(r/3) + C_info,  RRI = 30·(r/3) + L_info
//
// Note: the RRI mapping (Cluster 0 → C_info, Cluster 1 → R_info,
// Cluster 2 → L_info) matches the Python `pdf417` library and produces
// scannable symbols. The TypeScript reference uses the same formula.

/// Compute the Left Row Indicator codeword for row `r`.
public func pdf417ComputeLRI(r: Int, rows: Int, cols: Int, eccLevel: Int) -> Int {
    let rInfo = (rows - 1) / 3
    let cInfo = cols - 1
    let lInfo = 3 * eccLevel + (rows - 1) % 3
    let rowGroup = r / 3
    let cluster = r % 3

    if cluster == 0 { return 30 * rowGroup + rInfo }
    if cluster == 1 { return 30 * rowGroup + lInfo }
    return 30 * rowGroup + cInfo
}

/// Compute the Right Row Indicator codeword for row `r`.
public func pdf417ComputeRRI(r: Int, rows: Int, cols: Int, eccLevel: Int) -> Int {
    let rInfo = (rows - 1) / 3
    let cInfo = cols - 1
    let lInfo = 3 * eccLevel + (rows - 1) % 3
    let rowGroup = r / 3
    let cluster = r % 3

    if cluster == 0 { return 30 * rowGroup + cInfo }
    if cluster == 1 { return 30 * rowGroup + rInfo }
    return 30 * rowGroup + lInfo
}

// ============================================================================
// MARK: - Codeword → modules expansion
// ============================================================================

/// Expand a packed bar/space pattern into 17 boolean module values, appended
/// to `modules`.
///
/// The 8 element widths are stored as 4 bits each in the packed UInt32:
///
/// ```
/// bits 31..28 = b1 (bar)    bits 27..24 = s1 (space)
/// bits 23..20 = b2          bits 19..16 = s2
/// bits 15..12 = b3          bits 11..8  = s3
/// bits 7..4   = b4          bits 3..0   = s4
/// ```
///
/// Output alternates: bar (true) → space (false) → bar → space → ...
@inline(__always)
func pdf417ExpandPattern(packed: UInt32, into modules: inout [Bool]) {
    let b1 = Int((packed >> 28) & 0xF)
    let s1 = Int((packed >> 24) & 0xF)
    let b2 = Int((packed >> 20) & 0xF)
    let s2 = Int((packed >> 16) & 0xF)
    let b3 = Int((packed >> 12) & 0xF)
    let s3 = Int((packed >> 8)  & 0xF)
    let b4 = Int((packed >> 4)  & 0xF)
    let s4 = Int( packed        & 0xF)

    for _ in 0..<b1 { modules.append(true) }
    for _ in 0..<s1 { modules.append(false) }
    for _ in 0..<b2 { modules.append(true) }
    for _ in 0..<s2 { modules.append(false) }
    for _ in 0..<b3 { modules.append(true) }
    for _ in 0..<s3 { modules.append(false) }
    for _ in 0..<b4 { modules.append(true) }
    for _ in 0..<s4 { modules.append(false) }
}

/// Expand a bar/space width array into boolean module values, appended to
/// `modules`. The first element is always a bar (dark = true); subsequent
/// elements alternate.
@inline(__always)
func pdf417ExpandWidths(widths: [Int], into modules: inout [Bool]) {
    var dark = true
    for w in widths {
        for _ in 0..<w { modules.append(dark) }
        dark.toggle()
    }
}

// ============================================================================
// MARK: - Public API: encode
// ============================================================================

/// Encode `data` (UTF-8 bytes from the string) as a PDF417 symbol.
///
/// This is the convenience overload taking a `String`. It encodes the input
/// as UTF-8 and feeds the bytes through byte compaction.
///
/// - Parameters:
///   - data:    The string to encode. Encoded as UTF-8 internally.
///   - options: Optional encoding options; defaults to auto for everything.
///
/// - Returns: A `ModuleGrid` ready for rendering via `Barcode2D.layout()`.
///
/// - Throws:
///   - `PDF417Error.invalidECCLevel` if `options.eccLevel` is outside 0–8.
///   - `PDF417Error.invalidDimensions` if `options.columns` is outside 1–30.
///   - `PDF417Error.inputTooLong` if the data does not fit.
@discardableResult
public func encode(_ data: String, options: PDF417Options = PDF417Options()) throws -> ModuleGrid {
    return try encode(bytes: Array(data.utf8), options: options)
}

/// Encode raw bytes as a PDF417 symbol.
///
/// - Parameters:
///   - bytes:   Raw bytes to encode.
///   - options: Optional encoding options.
///
/// - Returns: A `ModuleGrid` ready for rendering via `Barcode2D.layout()`.
///
/// - Throws: `PDF417Error` cases for invalid options or oversized input.
@discardableResult
public func encode(bytes: [UInt8], options: PDF417Options = PDF417Options()) throws -> ModuleGrid {
    // ── 1. Validate ECC level ─────────────────────────────────────────────
    if let level = options.eccLevel, level < 0 || level > 8 {
        throw PDF417Error.invalidECCLevel(
            "ECC level must be 0–8, got \(level)"
        )
    }

    // ── 2. Byte compaction ────────────────────────────────────────────────
    let dataCwords = pdf417ByteCompact(bytes: bytes)

    // ── 3. Auto-select ECC level if not provided ──────────────────────────
    let eccLevel = options.eccLevel ?? pdf417AutoEccLevel(dataCount: dataCwords.count + 1)
    let eccCount = 1 << (eccLevel + 1)  // 2^(eccLevel + 1)

    // ── 4. Length descriptor ──────────────────────────────────────────────
    // The length descriptor is the very first codeword. It counts itself +
    // all data codewords + all ECC codewords (but NOT padding).
    let lengthDesc = 1 + dataCwords.count + eccCount

    // Full data array for RS encoding: [lengthDesc, ...dataCwords].
    var fullData: [Int] = [lengthDesc]
    fullData.append(contentsOf: dataCwords)

    // ── 5. Reed-Solomon ECC ───────────────────────────────────────────────
    let eccCwords = pdf417RSEncode(data: fullData, eccLevel: eccLevel)

    // ── 6. Choose dimensions (rows × cols) ────────────────────────────────
    let totalCwords = fullData.count + eccCwords.count

    var cols: Int
    var rows: Int

    if let userCols = options.columns {
        if userCols < PDF417_MIN_COLS || userCols > PDF417_MAX_COLS {
            throw PDF417Error.invalidDimensions(
                "columns must be 1–30, got \(userCols)"
            )
        }
        cols = userCols
        rows = max(PDF417_MIN_ROWS, Int((Double(totalCwords) / Double(cols)).rounded(.up)))
        if rows > PDF417_MAX_ROWS {
            throw PDF417Error.inputTooLong(
                "Data requires \(rows) rows (max 90) with \(cols) columns."
            )
        }
    } else {
        let dims = pdf417ChooseDimensions(total: totalCwords)
        cols = dims.cols
        rows = dims.rows
    }

    // Verify capacity. The two clauses above should already guarantee this,
    // but a defensive check makes the failure mode explicit if a future
    // change to `chooseDimensions` ever reduces capacity.
    if cols * rows < totalCwords {
        throw PDF417Error.inputTooLong(
            "Cannot fit \(totalCwords) codewords in \(rows)×\(cols) grid."
        )
    }

    // ── 7. Pad data to fill the grid exactly ──────────────────────────────
    // Padding codeword 900 is appended between fullData and ecc — the ECC
    // was computed BEFORE padding, since padding is a "neutral" codeword.
    let paddingCount = cols * rows - totalCwords
    var paddedData = fullData
    if paddingCount > 0 {
        paddedData.append(contentsOf: [Int](repeating: PDF417_PADDING_CW, count: paddingCount))
    }

    // Full codeword sequence to rasterize: [data + padding, ecc].
    var fullSequence = paddedData
    fullSequence.append(contentsOf: eccCwords)

    // ── 8. Rasterize to ModuleGrid ────────────────────────────────────────
    let rowHeight = max(1, options.rowHeight)
    return try pdf417Rasterize(
        sequence: fullSequence,
        rows: rows,
        cols: cols,
        eccLevel: eccLevel,
        rowHeight: rowHeight
    )
}

// ============================================================================
// MARK: - Rasterization
// ============================================================================

/// Convert the flat codeword sequence to a `ModuleGrid`.
///
/// Every logical PDF417 row is laid out as:
///
/// ```
/// start(17) + LRI(17) + data×cols(17 each) + RRI(17) + stop(18)
/// ```
///
/// = 17 + 17 + 17·cols + 17 + 18 = `69 + 17 · cols` modules wide.
///
/// Each logical row is then repeated `rowHeight` times vertically. The total
/// pixel-grid height is `rows × rowHeight`.
func pdf417Rasterize(
    sequence: [Int],
    rows: Int,
    cols: Int,
    eccLevel: Int,
    rowHeight: Int
) throws -> ModuleGrid {
    let moduleWidth = 69 + 17 * cols
    let moduleHeight = rows * rowHeight

    // Precompute start and stop module sequences (identical for every row).
    var startModules: [Bool] = []
    pdf417ExpandWidths(widths: PDF417_START_PATTERN, into: &startModules)

    var stopModules: [Bool] = []
    pdf417ExpandWidths(widths: PDF417_STOP_PATTERN, into: &stopModules)

    // Build a 2D grid initialized to all-light (false) and stamp each row in
    // place. We assemble the row in a [Bool] buffer, then copy it down.
    var modules = [[Bool]](
        repeating: [Bool](repeating: false, count: moduleWidth),
        count: moduleHeight
    )

    for r in 0..<rows {
        let cluster = r % 3
        let clusterTable = PDF417_CLUSTER_TABLES[cluster]

        var rowModules: [Bool] = []
        rowModules.reserveCapacity(moduleWidth)

        // 1. Start pattern (17 modules).
        rowModules.append(contentsOf: startModules)

        // 2. Left Row Indicator (17 modules).
        let lri = pdf417ComputeLRI(r: r, rows: rows, cols: cols, eccLevel: eccLevel)
        pdf417ExpandPattern(packed: clusterTable[lri], into: &rowModules)

        // 3. Data codewords (17 modules each).
        for j in 0..<cols {
            let cw = sequence[r * cols + j]
            pdf417ExpandPattern(packed: clusterTable[cw], into: &rowModules)
        }

        // 4. Right Row Indicator (17 modules).
        let rri = pdf417ComputeRRI(r: r, rows: rows, cols: cols, eccLevel: eccLevel)
        pdf417ExpandPattern(packed: clusterTable[rri], into: &rowModules)

        // 5. Stop pattern (18 modules).
        rowModules.append(contentsOf: stopModules)

        // Sanity check — guards against mis-sized cluster table entries or
        // off-by-one bugs in start/stop pattern widths.
        if rowModules.count != moduleWidth {
            throw PDF417Error.inputTooLong(
                "Internal error: row \(r) has \(rowModules.count) modules, expected \(moduleWidth)"
            )
        }

        // Stamp this module row into the grid `rowHeight` times.
        let baseRow = r * rowHeight
        for h in 0..<rowHeight {
            modules[baseRow + h] = rowModules
        }
    }

    return ModuleGrid(
        cols: moduleWidth,
        rows: moduleHeight,
        modules: modules,
        moduleShape: .square
    )
}

// ============================================================================
// MARK: - encodeAndLayout — convenience
// ============================================================================

/// Encode `data` and pass the resulting `ModuleGrid` through the
/// `Barcode2D.layout()` pipeline, producing a `PaintScene` ready for any
/// PaintVM-compatible backend.
///
/// - Parameters:
///   - data:    The string to encode.
///   - options: Optional encoding options for `encode()`.
///   - config:  Optional layout configuration. Defaults override the
///              Barcode2D defaults to `quietZoneModules = 2` (PDF417 needs
///              less quiet zone than QR Code).
///
/// - Returns: A `PaintScene` from `Barcode2D.layout()`.
///
/// - Throws: `PDF417Error` cases (including `.layoutError` if the layout
///           layer rejects the configuration).
public func encodeAndLayout(
    _ data: String,
    options: PDF417Options = PDF417Options(),
    config: Barcode2DLayoutConfig? = nil
) throws -> PaintScene {
    let grid = try encode(data, options: options)
    var cfg = config ?? Barcode2DLayoutConfig()
    if config == nil {
        cfg.quietZoneModules = 2  // PDF417 minimum quiet zone
    }
    do {
        return try layout(grid: grid, config: cfg)
    } catch {
        throw PDF417Error.layoutError(String(describing: error))
    }
}

// ============================================================================
// MARK: - PaintScene re-export type alias for convenience
// ============================================================================

/// Convenience alias so callers don't need to import PaintInstructions
/// just to spell the return type of `encodeAndLayout`.
public typealias PDF417PaintScene = PaintScene
