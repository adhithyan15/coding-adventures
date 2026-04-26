// Zstd.swift — CMP07: Zstandard lossless compression (RFC 8878).
//
// Zstandard (RFC 8878, 2021) is a high-ratio, high-speed compression algorithm
// created by Yann Collet at Facebook (now Meta) in 2015.  It supersedes DEFLATE
// (ZIP/gzip) in most scenarios, achieving better ratios at higher throughput by
// combining two classical ideas in a modern way:
//
//   LZ77 back-references   — the same "copy from earlier in the output" trick
//                            used by DEFLATE, LZSS, LZW, and Brotli, but with
//                            a much larger window (up to 3.75 GB in theory;
//                            we use 32 KB for simplicity).
//
//   FSE (Finite State Entropy) — an asymmetric numeral system invented by
//                            Jarek Duda (2013).  It encodes a sequence of
//                            symbols at their Shannon-entropy cost in one pass,
//                            reaching closer to the theoretical limit than
//                            Huffman while remaining faster.
//
// Frame layout (RFC 8878 §3):
//
//   ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
//   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
//   │ 4B LE  │ 1B  │ 1/2/4/8 B (LE)      │  ...   │ 4B (optional)    │
//   └────────┴─────┴──────────────────────┴────────┴──────────────────┘
//
// Each block has a 3-byte header:
//   bit  0        = Last_Block flag
//   bits [2:1]    = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
//   bits [23:3]   = Block_Size
//
// Compression strategy in this implementation:
//   1. Split input into 128 KB blocks.
//   2. For each block, try in order:
//      a. RLE block       — all bytes identical → 5 bytes total.
//      b. Compressed block — LZ77+FSE; emit only if smaller than input.
//      c. Raw block        — verbatim fallback.
//
// Series context:
//   CMP00 (LZ77)    — Sliding-window back-references
//   CMP01 (LZ78)    — Explicit dictionary (trie)
//   CMP02 (LZSS)    — LZ77 + flag bits                ← we depend on this
//   CMP03 (LZW)     — LZ78 + pre-initialised alphabet; GIF
//   CMP04 (Huffman) — Entropy coding
//   CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
//   CMP06 (Brotli)  — DEFLATE + context modelling + static dict
//   CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed ← this package

import LZSS

// ============================================================================
// MARK: - Constants
// ============================================================================

/// ZStd magic number: 0xFD2FB528 (little-endian bytes: 28 B5 2F FD).
///
/// Every valid ZStd frame starts with these 4 bytes.  The value was chosen to
/// be unlikely to appear at the start of plain-text files — a useful invariant
/// for format-detection heuristics.
private let magic: UInt32 = 0xFD2FB528

/// Maximum block size: 128 KiB.
///
/// The ZStd spec allows blocks up to min(WindowSize, 128 KiB).  We use 128 KiB
/// for all blocks, which is both spec-compliant and a good trade-off between
/// compression ratio (larger blocks give the LZ77 engine more context) and
/// memory usage.
private let maxBlockSize = 128 * 1024

/// Decompression output cap: 256 MiB.
///
/// Prevents "decompression bomb" attacks where a tiny input expands to gigabytes
/// of output, exhausting memory.  256 MiB is generous for most real payloads.
private let maxOutput = 256 * 1024 * 1024

// ============================================================================
// MARK: - LL / ML / OF code tables (RFC 8878 §3.1.1.3)
// ============================================================================
//
// ZStd sequences carry three fields: literal length (LL), match length (ML),
// and match offset (OF).  Each field is encoded as a short FSE symbol (the
// "code") plus a fixed number of extra bits for precision.
//
// For example:
//   LL code 17 covers literal lengths 18 and 19: baseline=18, extraBits=1.
//   Reading 1 extra bit from the bitstream gives the exact length.
//
// This two-level scheme (FSE code + raw extra bits) is the same idea used in
// DEFLATE's length/distance tables, but generalised via the FSE state machine.

/// Literal Length code table: (baseline, extraBits) for codes 0..35.
///
/// Codes 0-15 each cover exactly one literal length (0 extra bits).
/// Codes 16+ cover increasingly wide ranges.
private let llCodes: [(UInt32, UInt8)] = [
    // Codes 0-15: individual values 0..15
    (0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0),
    (6, 0), (7, 0), (8, 0), (9, 0), (10, 0), (11, 0),
    (12, 0), (13, 0), (14, 0), (15, 0),
    // Codes 16-19: pairs (1 extra bit each)
    (16, 1), (18, 1), (20, 1), (22, 1),
    // Codes 20-21: quads (2 extra bits each)
    (24, 2), (28, 2),
    // Codes 22-23: octets (3 extra bits each)
    (32, 3), (40, 3),
    // Codes 24+: increasingly large ranges
    (48, 4), (64, 6),
    (128, 7), (256, 8), (512, 9), (1024, 10), (2048, 11), (4096, 12),
    (8192, 13), (16384, 14), (32768, 15), (65536, 16),
]

/// Match Length code table: (baseline, extraBits) for codes 0..52.
///
/// Minimum match length in ZStd is 3 (not 0).  Code 0 = match of 3 bytes.
private let mlCodes: [(UInt32, UInt8)] = [
    // Codes 0-31: individual values 3..34 (0 extra bits each)
    (3, 0),  (4, 0),  (5, 0),  (6, 0),  (7, 0),  (8, 0),
    (9, 0),  (10, 0), (11, 0), (12, 0), (13, 0), (14, 0),
    (15, 0), (16, 0), (17, 0), (18, 0), (19, 0), (20, 0),
    (21, 0), (22, 0), (23, 0), (24, 0), (25, 0), (26, 0),
    (27, 0), (28, 0), (29, 0), (30, 0), (31, 0), (32, 0),
    (33, 0), (34, 0),
    // Codes 32-35: pairs (1 extra bit each)
    (35, 1), (37, 1), (39, 1), (41, 1),
    // Codes 36-37: quads (2 extra bits)
    (43, 2), (47, 2),
    // Codes 38-39: octets (3 extra bits)
    (51, 3), (59, 3),
    // Codes 40-41 (4 extra bits)
    (67, 4), (83, 4),
    // Codes 42-43 (5 / 7 extra bits)
    (99, 5), (131, 7),
    // Codes 44-52: large ranges
    (259, 8), (515, 9), (1027, 10), (2051, 11),
    (4099, 12), (8195, 13), (16387, 14), (32771, 15), (65539, 16),
]

// ============================================================================
// MARK: - FSE predefined distributions (RFC 8878 Appendix B)
// ============================================================================
//
// "Predefined_Mode" means the decoder builds its FSE table from a
// specification-mandated fixed distribution — no per-frame table description
// is transmitted.  This saves header bytes for short frames.
//
// An entry of -1 means "probability 1/table_size".  Such symbols get exactly
// one slot in the decode table.  Their FSE state after decoding cycles through
// all states, ensuring the state machine always has a valid next state.

/// Predefined normalised distribution for Literal Length FSE.
/// accuracy_log = 6 → 64-slot table.
private let llNorm: [Int16] = [
     4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
    -1, -1, -1, -1,
]
private let llAccLog: UInt8 = 6  // table_size = 1 << 6 = 64

/// Predefined normalised distribution for Match Length FSE.
/// accuracy_log = 6 → 64-slot table.
private let mlNorm: [Int16] = [
     1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
    -1, -1, -1, -1, -1,
]
private let mlAccLog: UInt8 = 6

/// Predefined normalised distribution for Offset FSE.
/// accuracy_log = 5 → 32-slot table.
private let ofNorm: [Int16] = [
     1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
]
private let ofAccLog: UInt8 = 5  // table_size = 1 << 5 = 32

// ============================================================================
// MARK: - FSE decode table entry
// ============================================================================

/// One cell in the FSE decode table.
///
/// To decode a symbol from encoder state S:
///   1. `sym` is the output symbol.
///   2. Read `nb` bits from the bitstream as `bits`.
///   3. New state = `base + bits`.
///
/// The pair (nb, base) is designed so that new_state lands in the range
/// [sz, 2*sz), which is the valid encoder state range.  Encoder and decoder
/// are thus mirrors of each other.
private struct FseDe {
    var sym: UInt8   // decoded symbol (index into llCodes / mlCodes / ofNorm)
    var nb: UInt8    // extra bits to read for the next state
    var base: UInt16 // base value for next-state computation
}

// ============================================================================
// MARK: - FSE encode table entry
// ============================================================================

/// Encode transform for one symbol.
///
/// Given encoder state S for symbol `sym`:
///   nbOut  = (S + deltaNb) >> 16      (bits to emit)
///   emit the low `nbOut` bits of S
///   newS   = stateTbl[(S >> nbOut) + deltaFs]
///
/// The precomputed (deltaNb, deltaFs) fields make the hot encode loop a
/// fixed-cost sequence of arithmetic, bit-emit, and table-lookup operations.
private struct FseEe {
    /// `(maxBitsOut << 16) - (count << maxBitsOut)`
    /// Used as: `nbOut = (state &+ deltaNb) >> 16`
    var deltaNb: UInt32
    /// `cumulativeCountBeforeSym - count`  (can be negative → Int32)
    /// Used as: `newState = stateTbl[(state >> nbOut) + Int(deltaFs)]`
    var deltaFs: Int32
}

// ============================================================================
// MARK: - FSE decode table builder
// ============================================================================

/// Build an FSE decode table from a normalised probability distribution.
///
/// The algorithm has three phases, matching the ZStd reference decoder:
///
/// Phase 1 — rare symbols (probability = -1) are placed at the TOP of the
///           table (highest indices), one slot each.  These symbols are the
///           rarest; putting them at the top avoids disturbing the spreading
///           pattern of common symbols.
///
/// Phase 2 — remaining symbols are spread using the deterministic step
///           function `step = (sz >> 1) + (sz >> 3) + 3`.  This value is
///           always co-prime to `sz` (a power of two), so the walk visits
///           every free slot exactly once.  Symbols with count > 1 are spread
///           first, then count == 1 symbols, to match the reference order.
///
/// Phase 3 — for each slot, assign `nb` (state bits to read) and `base`
///           (next-state base).  The j-th slot (in index order) for symbol
///           `s` uses `ns = count[s] + j`:
///             nb   = accLog - floor(log2(ns))
///             base = ns * (1 << nb) - sz
///
/// - Parameters:
///   - norm:   Normalised probability distribution (sum = sz, -1 = 1/sz).
///   - accLog: Accuracy log; table size = 1 << accLog.
/// - Returns: Decode table of size 1 << accLog.
private func buildDecodeTable(norm: [Int16], accLog: UInt8) -> [FseDe] {
    let sz = 1 << Int(accLog)
    let step = (sz >> 1) + (sz >> 3) + 3

    // Initialise the table; FseDe values will be overwritten in phases 1-3.
    var tbl = [FseDe](repeating: FseDe(sym: 0, nb: 0, base: 0), count: sz)

    // symNext[s] tracks the next FSE "occurrence index" for symbol s.
    // After phase 1 and 2 it starts at count[s]; phase 3 increments it.
    var symNext = [UInt16](repeating: 0, count: norm.count)

    // ── Phase 1: probability -1 symbols at the top ────────────────────────
    // Rare symbols each get exactly 1 slot, placed from the end downward.
    var high = sz - 1
    for (s, c) in norm.enumerated() {
        if c == -1 {
            tbl[high].sym = UInt8(s)
            if high > 0 { high -= 1 }
            symNext[s] = 1
        }
    }

    // ── Phase 2: spread remaining symbols ────────────────────────────────
    // The two-pass approach (count > 1 first, then count == 1) mirrors the
    // ZStd reference implementation, producing a deterministic, reproducible
    // table layout.
    var pos = 0
    for pass in 0..<2 {
        for (s, c) in norm.enumerated() {
            guard c > 0 else { continue }
            let cnt = Int(c)
            // pass 0 handles cnt > 1 first; pass 1 handles cnt == 1.
            if (pass == 0) != (cnt > 1) { continue }
            symNext[s] = UInt16(cnt)
            for _ in 0..<cnt {
                tbl[pos].sym = UInt8(s)
                pos = (pos + step) & (sz - 1)
                while pos > high {
                    pos = (pos + step) & (sz - 1)
                }
            }
        }
    }

    // ── Phase 3: assign nb and base for each slot ─────────────────────────
    // We iterate over ALL slots in index order, using symNext as a running
    // counter of how many times we've seen each symbol so far.
    var sn = symNext  // local copy that we increment
    for i in 0..<sz {
        let s = Int(tbl[i].sym)
        let ns = UInt32(sn[s])
        sn[s] += 1
        // floor(log2(ns)) = 31 - leading zeros of ns  (ns is always ≥ 1 here)
        let nb = accLog - UInt8(31 - ns.leadingZeroBitCount)
        // base = ns * (1 << nb) - sz
        // We use wrapping arithmetic because the subtraction is intentionally
        // modular (base can be 0 when ns * (1<<nb) == sz).
        let base = UInt16((ns << nb) &- UInt32(sz))
        tbl[i].nb = nb
        tbl[i].base = base
    }

    return tbl
}

// ============================================================================
// MARK: - FSE encode table builder
// ============================================================================

/// Build FSE encode tables from a normalised distribution.
///
/// Returns:
///   - `ee[sym]`:   The FseEe transform for each symbol.
///   - `st[slot]`:  The encoder state table (slot → output state in [sz, 2*sz)).
///
/// Encode/decode symmetry:
///   The decoder visits slots in ascending INDEX order to assign (sym, nb, base).
///   The encoder must use the same ordering: the j-th occurrence of symbol `s`
///   (in ascending slot index) maps to encoder slot `cumul[s] + j`.
///   After encoding, the new encoder state = (decode table index) + sz.
///
/// This means: if the decoder, at index `i`, reads `nb` bits and computes
/// new_state = base + bits, the encoder at that new_state will emit exactly
/// those `nb` bits and land back at decode index `i`.  Perfect symmetry.
private func buildEncodeTable(norm: [Int16], accLog: UInt8) -> ([FseEe], [UInt16]) {
    let sz = UInt32(1) << accLog

    // ── Step 1: cumulative sums ───────────────────────────────────────────
    // cumul[s] = sum of counts for symbols 0..<s.
    // This is the starting encoder "slot" for symbol s.
    var cumul = [UInt32](repeating: 0, count: norm.count)
    var total: UInt32 = 0
    for (s, c) in norm.enumerated() {
        cumul[s] = total
        let cnt: UInt32 = c == -1 ? 1 : (c > 0 ? UInt32(c) : 0)
        total += cnt
    }

    // ── Step 2: rebuild the spread table ─────────────────────────────────
    // Same spreading algorithm as buildDecodeTable so the index ordering is
    // identical.
    let step = (sz >> 1) + (sz >> 3) + 3
    var spread = [UInt8](repeating: 0, count: Int(sz))
    var idxHigh = Int(sz) - 1

    for (s, c) in norm.enumerated() {
        if c == -1 {
            spread[idxHigh] = UInt8(s)
            if idxHigh > 0 { idxHigh -= 1 }
        }
    }
    let idxLimit = idxHigh

    var pos2 = 0
    for pass in 0..<2 {
        for (s, c) in norm.enumerated() {
            guard c > 0 else { continue }
            let cnt = Int(c)
            if (pass == 0) != (cnt > 1) { continue }
            for _ in 0..<cnt {
                spread[pos2] = UInt8(s)
                pos2 = (pos2 + Int(step)) & (Int(sz) - 1)
                while pos2 > idxLimit {
                    pos2 = (pos2 + Int(step)) & (Int(sz) - 1)
                }
            }
        }
    }

    // ── Step 3: build the state table ────────────────────────────────────
    // For each table index i (ascending), determine which occurrence j of
    // symbol s = spread[i] this is (using symOcc[s] as counter).
    // Encoder slot = cumul[s] + j; output state = i + sz.
    var symOcc = [UInt32](repeating: 0, count: norm.count)
    var st = [UInt16](repeating: 0, count: Int(sz))

    for i in 0..<Int(sz) {
        let s = Int(spread[i])
        let j = symOcc[s]
        symOcc[s] += 1
        let slot = Int(cumul[s]) + Int(j)
        st[slot] = UInt16(i) &+ UInt16(sz)  // output state = decode index + sz
    }

    // ── Step 4: compute FseEe entries ────────────────────────────────────
    // For symbol s with count c:
    //   maxBitsOut (mbo) = accLog - floor(log2(c))  [or accLog if c==1]
    //   deltaNb = (mbo << 16) - (c << mbo)
    //   deltaFs = cumul[s] - c
    var ee = [FseEe](repeating: FseEe(deltaNb: 0, deltaFs: 0), count: norm.count)
    for (s, c) in norm.enumerated() {
        let cnt: UInt32 = c == -1 ? 1 : (c > 0 ? UInt32(c) : 0)
        guard cnt > 0 else { continue }
        let mbo: UInt32 = cnt == 1 ? UInt32(accLog) : UInt32(accLog) - UInt32(31 - cnt.leadingZeroBitCount)
        ee[s].deltaNb = (mbo << 16) &- (cnt << mbo)
        ee[s].deltaFs = Int32(cumul[s]) - Int32(cnt)
    }

    return (ee, st)
}

// ============================================================================
// MARK: - Reverse bit-writer
// ============================================================================
//
// ZStd's sequence bitstream is written BACKWARDS relative to the data flow:
// the encoder writes the bits that the decoder reads LAST, first.  This
// allows the decoder to process a forward-only stream while decoding sequences
// in natural order.
//
// Byte layout: [byte0, byte1, ..., byteN]
//   byteN (last) contains a SENTINEL BIT — the highest set bit in that byte.
//   The decoder initialises by finding the sentinel, then reads backward.
//
// Bit layout within each byte: bit 0 (LSB) = earliest written bit.
//
// Example: write bits 1, 0, 1, 1  (4 bits), then flush:
//   reg = 0b1011  (bits accumulate from LSB)
//   flush: sentinel at position 4 → last byte = 0b0001_1011 = 0x1B
//   buf = [0x1B]
// Decoder reads: find MSB at bit 4 (the sentinel), discard it, then read
//   bits 3..0 = 0b1011 — the original 4 bits, in original order.

private final class RevBitWriter {
    private var buf: [UInt8] = []
    private var reg: UInt64 = 0   // accumulation register (fills from LSB)
    private var bits: Int = 0     // number of valid bits in reg

    /// Append `nb` low-order bits of `val` to the stream.
    func addBits(_ val: UInt64, _ nb: Int) {
        guard nb > 0 else { return }
        let mask: UInt64 = nb == 64 ? UInt64.max : (1 << nb) - 1
        reg |= (val & mask) << bits
        bits += nb
        while bits >= 8 {
            buf.append(UInt8(reg & 0xFF))
            reg >>= 8
            bits -= 8
        }
    }

    /// Flush any remaining bits with a sentinel and seal the stream.
    ///
    /// The sentinel is a `1` bit placed just ABOVE all remaining data bits.
    /// The decoder locates it via `leadingZeroBitCount` on the last byte.
    func flush() {
        let sentinel: UInt8 = 1 << bits   // 1 at position `bits` (just above data)
        buf.append(UInt8(reg & 0xFF) | sentinel)
        reg = 0
        bits = 0
    }

    /// Return the accumulated bytes (call after `flush()`).
    func finish() -> [UInt8] { buf }
}

// ============================================================================
// MARK: - Reverse bit-reader
// ============================================================================
//
// Mirrors RevBitWriter: reads bits from the END of the buffer going backward.
// The LAST bytes written by the encoder (highest FSE state bits, written last)
// appear at the END of the byte buffer (the sentinel-containing last byte).
// The decoder initialises from the last byte and reads toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// readBits(n) extracts the top n bits and shifts the register left by n.
//
// Why left-aligned?  The writer accumulates bits LSB-first.  Within a flushed
// byte, bit 0 = earliest written, bit N = latest written.  Reading from the
// TOP of the register gives the highest-position bits first — the reverse of
// write order, which is exactly what the decoder needs.

private final class RevBitReader {
    private var data: [UInt8]
    private var reg: UInt64 = 0  // valid bits are LEFT-ALIGNED (MSB side)
    private var bits: Int = 0    // count of valid bits loaded
    private var pos: Int         // index of next byte to load (counts down)

    init(data: [UInt8]) throws {
        guard !data.isEmpty else { throw ZstdError.bitstreamEmpty }
        let last = data.last!
        guard last != 0 else { throw ZstdError.bitstreamNoSentinel }

        // Locate the sentinel: the highest set bit in the last byte.
        // sentinelPos (0-indexed from LSB) = 7 - leadingZeroBitCount.
        let sentinelPos = Int(last.bitWidth) - 1 - last.leadingZeroBitCount
        // validBits = data bits below the sentinel
        let validBits = sentinelPos

        // Place the valid bits of the sentinel byte at the TOP of the register.
        // Example: last = 0b0001_1110 (0x1E), sentinelPos = 4, validBits = 4.
        //   data bits = last & 0b1111 = 0b1110.
        //   After shifting to top: reg bits 63-60 = 1,1,1,0.
        let mask: UInt64 = validBits > 0 ? (1 << validBits) - 1 : 0
        self.reg = validBits > 0 ? (UInt64(last) & mask) << (64 - validBits) : 0
        self.bits = validBits
        self.pos = data.count - 1   // sentinel byte already consumed; reload from pos-1
        self.data = data
        reload()
    }

    /// Load more bytes into the left-aligned register, reading backward through `data`.
    ///
    /// Each new byte goes BELOW the already-loaded bits:
    ///   current top `bits` bits are occupied; new byte goes at position (64 - bits - 8).
    private func reload() {
        while bits <= 56 && pos > 0 {
            pos -= 1
            let shift = 64 - bits - 8
            reg |= UInt64(data[pos]) << shift
            bits += 8
        }
    }

    /// Read `nb` bits from the top of the register (returns 0 if nb == 0).
    ///
    /// Returns the most-recently-written bits first (highest stream positions first),
    /// mirroring the encoder's backward write order.
    func readBits(_ nb: Int) -> UInt64 {
        guard nb > 0 else { return 0 }
        let val = reg >> (64 - nb)
        reg = nb == 64 ? 0 : reg << nb
        bits = max(0, bits - nb)
        if bits < 24 { reload() }
        return val
    }
}

// ============================================================================
// MARK: - FSE encode/decode helpers
// ============================================================================

/// Encode one symbol into the backward bitstream, advancing the FSE state.
///
/// Encode step:
///   1. nbOut = (state &+ ee.deltaNb) >> 16   (bits to flush)
///   2. Write low nbOut bits of state.
///   3. newState = st[(state >> nbOut) + ee.deltaFs]
///
/// After encoding all symbols, the caller writes the final state (minus sz)
/// as accLog bits so the decoder can initialise.
private func fseEncodeSym(
    state: inout UInt32,
    sym: UInt8,
    ee: [FseEe],
    st: [UInt16],
    bw: RevBitWriter
) {
    let e = ee[Int(sym)]
    let nb = Int((state &+ e.deltaNb) >> 16)
    bw.addBits(UInt64(state), nb)
    let slotIdx = Int(state >> nb) + Int(e.deltaFs)
    state = UInt32(st[slotIdx])
}

/// Decode one symbol from the backward bitstream, advancing the FSE state.
///
/// Decode step:
///   1. Look up de[state] → (sym, nb, base).
///   2. newState = base + readBits(nb).
private func fseDecodeSym(state: inout UInt16, de: [FseDe], br: RevBitReader) -> UInt8 {
    let e = de[Int(state)]
    let sym = e.sym
    state = e.base &+ UInt16(br.readBits(Int(e.nb)))
    return sym
}

// ============================================================================
// MARK: - LL / ML code-number helpers
// ============================================================================

/// Map a literal length to its LL code number (0..35).
///
/// We scan the llCodes table from the beginning, keeping the last entry whose
/// baseline ≤ ll.  Since entries are in increasing baseline order, this gives
/// the largest code that does not exceed ll.
private func llToCode(_ ll: UInt32) -> Int {
    var code = 0
    for (i, entry) in llCodes.enumerated() {
        if entry.0 <= ll { code = i } else { break }
    }
    return code
}

/// Map a match length to its ML code number (0..52).
private func mlToCode(_ ml: UInt32) -> Int {
    var code = 0
    for (i, entry) in mlCodes.enumerated() {
        if entry.0 <= ml { code = i } else { break }
    }
    return code
}

// ============================================================================
// MARK: - Sequence struct
// ============================================================================

/// One ZStd sequence: literal_length, match_length, match_offset.
///
/// Processing one sequence means:
///   1. Copy `ll` literal bytes from the literals section.
///   2. Copy `ml` bytes starting `off` positions back in the output buffer.
///
/// After all sequences, any remaining literals are appended verbatim.
private struct Seq {
    let ll: UInt32   // literal length
    let ml: UInt32   // match length (minimum 3 in ZStd)
    let off: UInt32  // match offset (1-indexed; 1 = last byte written)
}

/// Convert LZSS tokens into ZStd sequences + a flat literals buffer.
///
/// LZSS produces a stream of `.literal(byte)` and `.match(offset, length)`.
/// ZStd groups consecutive literals before each match into a single Seq.
/// Trailing literals (after the last match) are appended to `lits` without
/// creating a sequence entry.
private func tokensToSeqs(_ tokens: [Token]) -> ([UInt8], [Seq]) {
    var lits: [UInt8] = []
    var seqs: [Seq] = []
    var litRun: UInt32 = 0

    for tok in tokens {
        switch tok {
        case .literal(let b):
            lits.append(b)
            litRun += 1
        case .match(let offset, let length):
            seqs.append(Seq(ll: litRun, ml: UInt32(length), off: UInt32(offset)))
            litRun = 0
        }
    }
    // Trailing literals stay in lits; no Seq for them.
    return (lits, seqs)
}

// ============================================================================
// MARK: - Literals section
// ============================================================================
//
// ZStd literals can be Huffman-coded or stored raw.  We always use Raw_Literals
// (type = 0), the simplest option: no Huffman table, bytes stored verbatim.
//
// Header encoding (RFC 8878 §3.1.1.2.1):
//   Bottom 2 bits = Literals_Block_Type = 00 (Raw)
//   Next  2 bits  = Size_Format:
//     0b00 or 0b10 → 1-byte header: (size << 3) | 0b000
//     0b01          → 2-byte header: (size << 4) | 0b0100
//     0b11          → 3-byte header: (size << 4) | 0b1100

/// Encode a raw literals section, prepending the appropriate length header.
private func encodeLiteralsSection(_ lits: [UInt8]) -> [UInt8] {
    let n = lits.count
    var out: [UInt8] = []
    out.reserveCapacity(n + 3)

    if n <= 31 {
        // 1-byte header: size_format=00, type=00
        // Header = (n << 3) | 0b000  (low 3 bits are 0)
        out.append(UInt8(n << 3))
    } else if n <= 4095 {
        // 2-byte header: size_format=01, type=00 → trailer bits = 0b0100
        let hdr = UInt32(n) << 4 | 0b0100
        out.append(UInt8(hdr & 0xFF))
        out.append(UInt8((hdr >> 8) & 0xFF))
    } else {
        // 3-byte header: size_format=11, type=00 → trailer bits = 0b1100
        let hdr = UInt32(n) << 4 | 0b1100
        out.append(UInt8(hdr & 0xFF))
        out.append(UInt8((hdr >> 8) & 0xFF))
        out.append(UInt8((hdr >> 16) & 0xFF))
    }

    out.append(contentsOf: lits)
    return out
}

/// Decode a raw literals section, returning (literals, bytesConsumed).
///
/// - Parameter data: Bytes starting at the beginning of the literals section.
/// - Throws: `ZstdError.decodingError` for truncated or unsupported input.
private func decodeLiteralsSection(_ data: [UInt8]) throws -> ([UInt8], Int) {
    guard !data.isEmpty else {
        throw ZstdError.decodingError("empty literals section")
    }

    let b0 = data[0]
    let ltype = b0 & 0b11  // Literals_Block_Type: 0=Raw, 2/3=Huffman

    // We only produce Raw_Literals (type 0); Huffman (types 2, 3) from other
    // encoders would require a full Huffman decoder, which is out of scope here.
    guard ltype == 0 else {
        throw ZstdError.decodingError("unsupported literals type \(ltype) (only Raw=0 is supported)")
    }

    let sizeFormat = (b0 >> 2) & 0b11

    let n: Int
    let headerBytes: Int
    switch sizeFormat {
    case 0, 2:
        // 1-byte header: size in bits [7:3] (5 bits, values 0..31)
        n = Int(b0 >> 3)
        headerBytes = 1
    case 1:
        // 2-byte header: 12-bit size
        guard data.count >= 2 else {
            throw ZstdError.decodingError("truncated 2-byte literals header")
        }
        n = Int(b0 >> 4) | (Int(data[1]) << 4)
        headerBytes = 2
    default:  // 3
        // 3-byte header: 20-bit size (enough for blocks up to 1 MB)
        guard data.count >= 3 else {
            throw ZstdError.decodingError("truncated 3-byte literals header")
        }
        n = Int(b0 >> 4) | (Int(data[1]) << 4) | (Int(data[2]) << 12)
        headerBytes = 3
    }

    let start = headerBytes
    let end = start + n
    guard end <= data.count else {
        throw ZstdError.decodingError("literals data truncated: need \(end), have \(data.count)")
    }

    return (Array(data[start..<end]), end)
}

// ============================================================================
// MARK: - Sequences section
// ============================================================================
//
// Layout:
//   [sequence_count: 1-3 bytes]
//   [symbol_compression_modes: 1 byte]  — always 0x00 (all Predefined) here
//   [FSE bitstream: variable]
//
// Symbol compression modes byte:
//   bits [7:6] = LL mode   (0=Predefined, 1=RLE, 2=FSE_Compressed, 3=Repeat)
//   bits [5:4] = OF mode
//   bits [3:2] = ML mode
//   bits [1:0] = reserved (0)
// We always emit 0x00 (all Predefined), so no per-frame table is sent.
//
// FSE bitstream is a BACKWARD bit-stream:
//   Sequences are encoded in REVERSE ORDER (last sequence first).
//   For each sequence (in reverse):
//       OF extra bits, ML extra bits, LL extra bits  (in this order)
//       FSE symbol for OF, then ML, then LL
//   After all sequences, flush final FSE states:
//       (state_of - sz_of) as OF_ACC_LOG bits
//       (state_ml - sz_ml) as ML_ACC_LOG bits
//       (state_ll - sz_ll) as LL_ACC_LOG bits
//   Then flush the sentinel byte.
//
// Decoder reverses this:
//   1. Read LL_ACC_LOG bits → initial state_ll
//   2. Read ML_ACC_LOG bits → initial state_ml
//   3. Read OF_ACC_LOG bits → initial state_of
//   4. For each sequence (in forward order):
//       decode LL symbol (state transition: updates state, returns symbol)
//       decode OF symbol
//       decode ML symbol
//       read LL extra bits
//       read ML extra bits
//       read OF extra bits
//   5. Apply sequence to output.

/// Encode the sequence count field (1-3 bytes).
///
/// ZStd sequence count encoding (RFC 8878 §3.1.1.3.3):
///
///   count == 0          → [0x00]                        (1 byte)
///   1 ≤ count < 128     → [count]                       (1 byte, bit 7 = 0)
///   128 ≤ count < 32512 → 2-byte big-endian-ish:
///                           byte_0 = 0x80 + (count >> 8)
///                           byte_1 = count & 0xFF
///                         byte_0 is in [0x80, 0xFE] (high bit set, ≠ 0xFF).
///   count ≥ 32512       → [0xFF, lo, hi] where lo+(hi<<8) = count - 0x7F00
///
/// The 2-byte form stores the count as a 15-bit value by setting the high bit
/// of byte_0 and encoding the remaining 7 bits of the high byte there plus
/// all 8 bits of the low byte in byte_1.  This guarantees byte_0 ∈ [0x80, 0xFE].
private func encodeSeqCount(_ count: Int) -> [UInt8] {
    if count == 0 { return [0] }
    if count < 128 { return [UInt8(count)] }
    if count < 0x7F00 {
        // 2-byte form: byte_0 = 0x80 | (count >> 8), byte_1 = count & 0xFF.
        // byte_0 ∈ [0x80, 0xFE] because count >> 8 ∈ [0, 0x7E] for count < 0x7F00.
        let b0 = UInt8(0x80 | (count >> 8))
        let b1 = UInt8(count & 0xFF)
        return [b0, b1]
    }
    // 3-byte form: sentinel 0xFF + (count - 0x7F00) as LE u16.
    let r = count - 0x7F00
    return [0xFF, UInt8(r & 0xFF), UInt8((r >> 8) & 0xFF)]
}

/// Decode the sequence count field, returning (count, bytesConsumed).
///
/// Mirrors `encodeSeqCount`:
///   b0 < 0x80          → count = b0                           (1 byte)
///   0x80 ≤ b0 < 0xFF   → count = ((b0 - 0x80) << 8) | b1     (2 bytes)
///   b0 == 0xFF         → count = 0x7F00 + b1 + (b2 << 8)     (3 bytes)
private func decodeSeqCount(_ data: [UInt8]) throws -> (Int, Int) {
    guard !data.isEmpty else {
        throw ZstdError.decodingError("empty sequence count")
    }
    let b0 = data[0]
    if b0 < 128 {
        return (Int(b0), 1)
    } else if b0 < 0xFF {
        guard data.count >= 2 else {
            throw ZstdError.decodingError("truncated sequence count (2-byte)")
        }
        let count = (Int(b0 - 0x80) << 8) | Int(data[1])
        return (count, 2)
    } else {
        guard data.count >= 3 else {
            throw ZstdError.decodingError("truncated sequence count (3-byte)")
        }
        let count = 0x7F00 + Int(data[1]) + (Int(data[2]) << 8)
        return (count, 3)
    }
}

/// Encode the sequences section (count + modes byte + FSE bitstream).
private func encodeSequencesSection(_ seqs: [Seq]) -> [UInt8] {
    // Build encode tables from the predefined distributions.
    let (eeLl, stLl) = buildEncodeTable(norm: llNorm, accLog: llAccLog)
    let (eeMl, stMl) = buildEncodeTable(norm: mlNorm, accLog: mlAccLog)
    let (eeOf, stOf) = buildEncodeTable(norm: ofNorm, accLog: ofAccLog)

    let szLl = UInt32(1) << llAccLog
    let szMl = UInt32(1) << mlAccLog
    let szOf = UInt32(1) << ofAccLog

    // FSE encoder states start at sz (the midpoint of the valid range [sz, 2*sz)).
    var stateLl = szLl
    var stateMl = szMl
    var stateOf = szOf

    let bw = RevBitWriter()

    // Encode sequences in REVERSE ORDER so the decoder sees them forward.
    for seq in seqs.reversed() {
        let llCode = llToCode(seq.ll)
        let mlCode = mlToCode(seq.ml)

        // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1).
        // code = floor(log2(raw)); extra = raw - (1 << code).
        let rawOff = seq.off + 3
        let ofCode: UInt8 = rawOff <= 1 ? 0 : UInt8(31 - rawOff.leadingZeroBitCount)
        let ofExtra = rawOff - (UInt32(1) << Int(ofCode))

        // Write extra bits in OF, ML, LL order (backwards stream).
        bw.addBits(UInt64(ofExtra), Int(ofCode))
        let mlExtra = seq.ml - mlCodes[mlCode].0
        bw.addBits(UInt64(mlExtra), Int(mlCodes[mlCode].1))
        let llExtra = seq.ll - llCodes[llCode].0
        bw.addBits(UInt64(llExtra), Int(llCodes[llCode].1))

        // FSE encode symbols.
        // Decode order: LL, OF, ML.
        // Since the bitstream is reversed, we WRITE in the opposite order: ML → OF → LL.
        // That way when the decoder reads forward, it sees LL first.
        fseEncodeSym(state: &stateMl, sym: UInt8(mlCode), ee: eeMl, st: stMl, bw: bw)
        fseEncodeSym(state: &stateOf, sym: ofCode,          ee: eeOf, st: stOf, bw: bw)
        fseEncodeSym(state: &stateLl, sym: UInt8(llCode),   ee: eeLl, st: stLl, bw: bw)
    }

    // Flush final FSE states as raw bits (OF first, then ML, then LL).
    bw.addBits(UInt64(stateOf - szOf), Int(ofAccLog))
    bw.addBits(UInt64(stateMl - szMl), Int(mlAccLog))
    bw.addBits(UInt64(stateLl - szLl), Int(llAccLog))
    bw.flush()

    return bw.finish()
}

// ============================================================================
// MARK: - Block-level compress
// ============================================================================

/// Compress one 128 KB block using LZ77 + FSE (ZStd compressed block format).
///
/// Returns `nil` if the compressed form would be larger than the input — in that
/// case the caller falls back to a Raw block.
private func compressBlock(_ block: [UInt8]) -> [UInt8]? {
    // LZSS provides the LZ77 match engine.  ZStd recommends a 32 KB window;
    // we use a 32 KB window with maximum match length 255 and minimum 3.
    let tokens = LZSS.encode(block, windowSize: 32768, maxMatch: 255, minMatch: 3)

    let (lits, seqs) = tokensToSeqs(tokens)

    // If no sequences were found, LZ77 had nothing to compress.  A compressed
    // block with 0 sequences still has overhead vs. a raw block, so fall back.
    guard !seqs.isEmpty else { return nil }

    var out: [UInt8] = []

    // Literals section (Raw_Literals format).
    out.append(contentsOf: encodeLiteralsSection(lits))

    // Sequence count + modes byte + FSE bitstream.
    out.append(contentsOf: encodeSeqCount(seqs.count))
    out.append(0x00)  // Symbol_Compression_Modes = all Predefined
    out.append(contentsOf: encodeSequencesSection(seqs))

    // Only return the compressed form if it is actually smaller.
    return out.count < block.count ? out : nil
}

/// Decompress one ZStd compressed block into `out`.
///
/// Parses the literals section, sequences section, and applies each sequence
/// to reconstruct the original bytes.
///
/// - Parameters:
///   - data: The block payload (after the 3-byte block header).
///   - out:  Output buffer; decoded bytes are appended here.
/// - Throws: `ZstdError` on malformed input.
private func decompressBlock(_ data: [UInt8], out: inout [UInt8]) throws {
    // ── Literals section ─────────────────────────────────────────────────
    let (lits, litConsumed) = try decodeLiteralsSection(data)
    var pos = litConsumed

    // ── Sequence count ───────────────────────────────────────────────────
    // A block may contain only literals (pos == data.count means no sequences).
    guard pos < data.count else {
        out.append(contentsOf: lits)
        return
    }

    let (nSeqs, scBytes) = try decodeSeqCount(Array(data[pos...]))
    pos += scBytes

    guard nSeqs > 0 else {
        // Zero sequences — all content is in literals.
        out.append(contentsOf: lits)
        return
    }

    // ── Symbol compression modes ─────────────────────────────────────────
    guard pos < data.count else {
        throw ZstdError.decodingError("missing symbol compression modes byte")
    }
    let modesByte = data[pos]
    pos += 1

    let llMode = (modesByte >> 6) & 3
    let ofMode = (modesByte >> 4) & 3
    let mlMode = (modesByte >> 2) & 3
    guard llMode == 0, ofMode == 0, mlMode == 0 else {
        throw ZstdError.unsupportedFSEModes
    }

    // ── FSE bitstream ────────────────────────────────────────────────────
    let bitstream = Array(data[pos...])
    let br = try RevBitReader(data: bitstream)

    // Build decode tables from predefined distributions.
    let dtLl = buildDecodeTable(norm: llNorm, accLog: llAccLog)
    let dtMl = buildDecodeTable(norm: mlNorm, accLog: mlAccLog)
    let dtOf = buildDecodeTable(norm: ofNorm, accLog: ofAccLog)

    // Initialise FSE states.
    // The encoder wrote (in this order): stateLl, stateMl, stateOf.
    // The decoder reads them in the same order (because flush() reversed the bits).
    var stateLl = UInt16(br.readBits(Int(llAccLog)))
    var stateMl = UInt16(br.readBits(Int(mlAccLog)))
    var stateOf = UInt16(br.readBits(Int(ofAccLog)))

    var litPos = 0

    for _ in 0..<nSeqs {
        // Decode symbols (state transitions).  Order: LL, OF, ML.
        let llCode = fseDecodeSym(state: &stateLl, de: dtLl, br: br)
        let ofCode = fseDecodeSym(state: &stateOf, de: dtOf, br: br)
        let mlCode = fseDecodeSym(state: &stateMl, de: dtMl, br: br)

        // Validate code indices.
        guard Int(llCode) < llCodes.count else {
            throw ZstdError.decodingError("invalid LL code \(llCode)")
        }
        guard Int(mlCode) < mlCodes.count else {
            throw ZstdError.decodingError("invalid ML code \(mlCode)")
        }

        let llInfo = llCodes[Int(llCode)]
        let mlInfo = mlCodes[Int(mlCode)]

        // Read extra bits.
        let ll = llInfo.0 + UInt32(br.readBits(Int(llInfo.1)))
        let ml = mlInfo.0 + UInt32(br.readBits(Int(mlInfo.1)))

        // Offset: raw = (1 << ofCode) | extra; offset = raw - 3.
        let ofExtra = br.readBits(Int(ofCode))
        let ofRaw = (UInt32(1) << Int(ofCode)) | UInt32(ofExtra)
        guard ofRaw >= 3 else {
            throw ZstdError.decodingError("decoded offset underflow: ofRaw=\(ofRaw)")
        }
        let offset = ofRaw - 3

        // Emit `ll` literal bytes from the literals buffer.
        let litEnd = litPos + Int(ll)
        guard litEnd <= lits.count else {
            throw ZstdError.decodingError(
                "literal run \(ll) overflows literals buffer (pos=\(litPos) len=\(lits.count))")
        }
        out.append(contentsOf: lits[litPos..<litEnd])
        litPos = litEnd

        // Copy `ml` bytes from `offset` positions back in the output.
        // offset == 0 or offset > out.count would be a corrupt back-reference.
        guard offset > 0, Int(offset) <= out.count else {
            throw ZstdError.invalidOffset(offset, out.count)
        }
        let copyStart = out.count - Int(offset)
        for i in 0..<Int(ml) {
            out.append(out[copyStart + i])
        }
    }

    // Append any remaining literals after the last sequence.
    out.append(contentsOf: lits[litPos...])
}

// ============================================================================
// MARK: - Public API
// ============================================================================

/// Errors that can be thrown by `decompress`.
public enum ZstdError: Error {
    /// The input is shorter than a minimal ZStd frame header.
    case frameTooShort
    /// The first 4 bytes do not match the ZStd magic number.
    case badMagic(UInt32)
    /// A block header claims more bytes than are available.
    case blockTruncated
    /// Block type 3 (Reserved) was encountered.
    case reservedBlockType
    /// The decompressed output would exceed the 256 MiB safety cap.
    case outputLimitExceeded
    /// The FSE bitstream is empty.
    case bitstreamEmpty
    /// The last byte of the FSE bitstream is zero (no sentinel bit found).
    case bitstreamNoSentinel
    /// The sequences section uses a non-Predefined FSE mode.
    case unsupportedFSEModes
    /// A back-reference offset is 0 or larger than the current output.
    case invalidOffset(UInt32, Int)
    /// A generic decoding error with a human-readable message.
    case decodingError(String)
}

/// Compress `data` to a ZStd frame (RFC 8878).
///
/// The output is a standards-compliant ZStd frame decompressible by the
/// `zstd` CLI tool or any conforming implementation.
///
/// Strategy:
///   - Empty input → one empty raw block.
///   - All-identical block → RLE block (4 bytes total).
///   - Other blocks → try LZ77+FSE compressed; fall back to raw.
///
/// ```swift
/// let data = Array("hello, world!".utf8)
/// let compressed = compress(data)
/// let recovered = try! decompress(compressed)
/// assert(recovered == data)
/// ```
public func compress(_ data: [UInt8]) -> [UInt8] {
    var out: [UInt8] = []

    // ── ZStd frame header ─────────────────────────────────────────────────
    // Magic number (4 bytes, little-endian).
    let m = magic
    out.append(UInt8(m & 0xFF))
    out.append(UInt8((m >> 8) & 0xFF))
    out.append(UInt8((m >> 16) & 0xFF))
    out.append(UInt8((m >> 24) & 0xFF))

    // Frame Header Descriptor (FHD):
    //   bits [7:6] = FCS_Field_Size = 11 → 8-byte FCS
    //   bit  [5]   = Single_Segment_Flag = 1 (no Window_Descriptor)
    //   bit  [4]   = Content_Checksum_Flag = 0
    //   bits [3:2] = reserved = 0
    //   bits [1:0] = Dict_ID_Flag = 0
    // = 0b1110_0000 = 0xE0
    out.append(0xE0)

    // Frame_Content_Size (8 bytes, little-endian) — the uncompressed size.
    let fcs = UInt64(data.count)
    for shift in [0, 8, 16, 24, 32, 40, 48, 56] as [UInt64] {
        out.append(UInt8((fcs >> shift) & 0xFF))
    }

    // ── Blocks ────────────────────────────────────────────────────────────
    // Special case: empty input → one empty raw block (last=1, type=raw, size=0).
    if data.isEmpty {
        let hdr: UInt32 = 0b001  // last=1, type=00 (raw), size=0
        out.append(UInt8(hdr & 0xFF))
        out.append(UInt8((hdr >> 8) & 0xFF))
        out.append(UInt8((hdr >> 16) & 0xFF))
        return out
    }

    var offset = 0
    while offset < data.count {
        let end = min(offset + maxBlockSize, data.count)
        let block = Array(data[offset..<end])
        let last = end == data.count

        // ── RLE block: all bytes identical ────────────────────────────────
        // ZStd RLE blocks store a single byte and repeat it `bsize` times.
        // Cost: 3-byte header + 1 byte = 4 bytes total (vs. bsize for raw).
        if !block.isEmpty && block.allSatisfy({ $0 == block[0] }) {
            let hdr = (UInt32(block.count) << 3) | (0b01 << 1) | (last ? 1 : 0)
            out.append(UInt8(hdr & 0xFF))
            out.append(UInt8((hdr >> 8) & 0xFF))
            out.append(UInt8((hdr >> 16) & 0xFF))
            out.append(block[0])
        } else if let compressed = compressBlock(block) {
            // ── Compressed block: LZ77 + FSE ──────────────────────────────
            let hdr = (UInt32(compressed.count) << 3) | (0b10 << 1) | (last ? 1 : 0)
            out.append(UInt8(hdr & 0xFF))
            out.append(UInt8((hdr >> 8) & 0xFF))
            out.append(UInt8((hdr >> 16) & 0xFF))
            out.append(contentsOf: compressed)
        } else {
            // ── Raw block: verbatim fallback ──────────────────────────────
            let hdr = (UInt32(block.count) << 3) | (0b00 << 1) | (last ? 1 : 0)
            out.append(UInt8(hdr & 0xFF))
            out.append(UInt8((hdr >> 8) & 0xFF))
            out.append(UInt8((hdr >> 16) & 0xFF))
            out.append(contentsOf: block)
        }

        offset = end
    }

    return out
}

/// Decompress a ZStd frame, returning the original data.
///
/// Accepts any valid ZStd frame with:
///   - Single-segment or multi-segment layout
///   - Raw, RLE, or Compressed blocks
///   - Predefined FSE modes (no per-frame table description)
///
/// Output is capped at 256 MiB to guard against decompression bombs.
///
/// ```swift
/// let original = Array("hello, ZStd!".utf8)
/// let recovered = try decompress(compress(original))
/// assert(recovered == original)
/// ```
///
/// - Throws: `ZstdError` on malformed, truncated, or unsupported input.
public func decompress(_ data: [UInt8]) throws -> [UInt8] {
    // Minimum valid frame: 4 (magic) + 1 (FHD) = 5 bytes.
    guard data.count >= 5 else { throw ZstdError.frameTooShort }

    // ── Validate magic ────────────────────────────────────────────────────
    let gotMagic = UInt32(data[0])
        | (UInt32(data[1]) << 8)
        | (UInt32(data[2]) << 16)
        | (UInt32(data[3]) << 24)
    guard gotMagic == magic else { throw ZstdError.badMagic(gotMagic) }

    var pos = 4

    // ── Parse Frame Header Descriptor ────────────────────────────────────
    let fhd = data[pos]
    pos += 1

    // FCS_Field_Size: bits [7:6]
    //   00 → 0 bytes (or 1 byte if Single_Segment = 1)
    //   01 → 2 bytes  (value + 256)
    //   10 → 4 bytes
    //   11 → 8 bytes
    let fcsFlag = (fhd >> 6) & 3

    // Single_Segment_Flag: bit 5.  When set, the window descriptor is omitted.
    let singleSeg = (fhd >> 5) & 1

    // Dict_ID_Flag: bits [1:0].  Indicates dict-ID byte count.
    let dictFlag = fhd & 3

    // ── Window Descriptor ─────────────────────────────────────────────────
    // Present only if Single_Segment_Flag = 0.  We skip it; we don't enforce
    // a window-size limit in this implementation.
    if singleSeg == 0 { pos += 1 }

    // ── Dict ID ───────────────────────────────────────────────────────────
    let dictIdBytes = [0, 1, 2, 4][Int(dictFlag)]
    pos += dictIdBytes  // skip dict ID (custom dicts not supported)

    // ── Frame Content Size ────────────────────────────────────────────────
    let fcsBytes: Int = {
        switch fcsFlag {
        case 0: return singleSeg == 1 ? 1 : 0
        case 1: return 2
        case 2: return 4
        default: return 8
        }
    }()
    pos += fcsBytes  // skip FCS (we trust the blocks to be correct)

    // ── Blocks ────────────────────────────────────────────────────────────
    var out: [UInt8] = []

    while true {
        guard pos + 3 <= data.count else { throw ZstdError.blockTruncated }

        // 3-byte little-endian block header.
        let hdr = UInt32(data[pos])
            | (UInt32(data[pos + 1]) << 8)
            | (UInt32(data[pos + 2]) << 16)
        pos += 3

        let lastBlock = (hdr & 1) != 0
        let btype = (hdr >> 1) & 3
        let bsize = Int(hdr >> 3)

        switch btype {
        case 0:
            // ── Raw block: bsize bytes verbatim ───────────────────────────
            guard pos + bsize <= data.count else { throw ZstdError.blockTruncated }
            guard out.count + bsize <= maxOutput else { throw ZstdError.outputLimitExceeded }
            out.append(contentsOf: data[pos..<(pos + bsize)])
            pos += bsize

        case 1:
            // ── RLE block: 1 byte repeated bsize times ────────────────────
            guard pos < data.count else { throw ZstdError.blockTruncated }
            guard out.count + bsize <= maxOutput else { throw ZstdError.outputLimitExceeded }
            let byte = data[pos]
            pos += 1
            out.append(contentsOf: repeatElement(byte, count: bsize))

        case 2:
            // ── Compressed block: LZ77 + FSE ──────────────────────────────
            guard pos + bsize <= data.count else { throw ZstdError.blockTruncated }
            let blockData = Array(data[pos..<(pos + bsize)])
            pos += bsize
            try decompressBlock(blockData, out: &out)
            guard out.count <= maxOutput else { throw ZstdError.outputLimitExceeded }

        default:  // 3 = Reserved
            throw ZstdError.reservedBlockType
        }

        if lastBlock { break }
    }

    return out
}
