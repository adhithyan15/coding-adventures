/**
 * Zstandard (ZStd) lossless compression algorithm — CMP07.
 *
 * Zstandard (RFC 8878) is a high-ratio, fast compression format created by
 * Yann Collet at Facebook (2015). It combines:
 *
 *   - **LZ77 back-references** (via LZSS token generation) to exploit
 *     repetition in the data — the same "copy from earlier in the output"
 *     trick as DEFLATE, but with a 32 KB window.
 *
 *   - **FSE (Finite State Entropy)** coding instead of Huffman for the
 *     sequence descriptor symbols. FSE is an asymmetric numeral system that
 *     approaches the Shannon entropy limit in a single pass.
 *
 *   - **Predefined decode tables** (RFC 8878 Appendix B) so short frames
 *     need no table description overhead.
 *
 * ## Frame layout (RFC 8878 §3)
 * ```
 * ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
 * │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
 * │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
 * └────────┴─────┴──────────────────────┴────────┴──────────────────┘
 * ```
 *
 * Each **block** has a 3-byte header:
 * ```
 * bit 0       = Last_Block flag
 * bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
 * bits [23:3] = Block_Size
 * ```
 *
 * ## Compression strategy (this implementation)
 *
 * 1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
 * 2. For each block, try:
 *    a. **RLE** — all bytes identical → 4 bytes total.
 *    b. **Compressed** (LZ77 + FSE) — if output < input length.
 *    c. **Raw** — verbatim copy as fallback.
 *
 * ## Series
 * ```
 * CMP00 (LZ77)     — Sliding-window back-references
 * CMP01 (LZ78)     — Explicit dictionary (trie)
 * CMP02 (LZSS)     — LZ77 + flag bits
 * CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
 * CMP04 (Huffman)  — Entropy coding
 * CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
 * CMP06 (Brotli)   — DEFLATE + context modelling + static dict
 * CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this package
 * ```
 *
 * ## Example
 * ```kotlin
 * val data = "the quick brown fox jumps over the lazy dog".encodeToByteArray()
 * val compressed = Zstd.compress(data)
 * val restored = Zstd.decompress(compressed)
 * assert(data.contentEquals(restored))
 * ```
 */
package com.codingadventures.zstd

import com.codingadventures.lzss.Lzss
import com.codingadventures.lzss.LzssToken
import java.io.IOException
import java.nio.ByteBuffer
import java.nio.ByteOrder

// ─── Constants ────────────────────────────────────────────────────────────────

/**
 * ZStd magic number: `0xFD2FB528` (little-endian bytes: 28 B5 2F FD).
 *
 * Every valid ZStd frame starts with these 4 bytes. The value was chosen to
 * be unlikely to appear at the start of plaintext files.
 */
private const val MAGIC: Long = 0xFD2FB528L

/**
 * Maximum block size: 128 KB.
 *
 * ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
 * blocks. The spec maximum is `min(WindowSize, 128 KB)`.
 */
private const val MAX_BLOCK_SIZE = 128 * 1024

// ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────────
//
// These tables map a *code number* to a (baseline, extra_bits) pair.
//
// For example, LL code 17 means literal_length = 18 + read(1 extra bit),
// so it covers literal lengths 18 and 19.
//
// The FSE state machine tracks one code number per field; extra bits are
// read directly from the bitstream after state transitions.

/**
 * Literal Length code table: `(baseline, extra_bits)` for codes 0..35.
 *
 * Literal length 0..15 each have their own code (0 extra bits).
 * Larger lengths are grouped with increasing ranges.
 */
internal val LL_CODES: Array<LongArray> = arrayOf(
    // code: value = baseline + read(extra_bits)
    longArrayOf(0, 0),  longArrayOf(1, 0),  longArrayOf(2, 0),  longArrayOf(3, 0),
    longArrayOf(4, 0),  longArrayOf(5, 0),  longArrayOf(6, 0),  longArrayOf(7, 0),
    longArrayOf(8, 0),  longArrayOf(9, 0),  longArrayOf(10, 0), longArrayOf(11, 0),
    longArrayOf(12, 0), longArrayOf(13, 0), longArrayOf(14, 0), longArrayOf(15, 0),
    // Grouped ranges start at code 16
    longArrayOf(16, 1), longArrayOf(18, 1), longArrayOf(20, 1), longArrayOf(22, 1),
    longArrayOf(24, 2), longArrayOf(28, 2),
    longArrayOf(32, 3), longArrayOf(40, 3),
    longArrayOf(48, 4), longArrayOf(64, 6),
    longArrayOf(128, 7), longArrayOf(256, 8), longArrayOf(512, 9), longArrayOf(1024, 10),
    longArrayOf(2048, 11), longArrayOf(4096, 12),
    longArrayOf(8192, 13), longArrayOf(16384, 14), longArrayOf(32768, 15), longArrayOf(65536, 16),
)

/**
 * Match Length code table: `(baseline, extra_bits)` for codes 0..52.
 *
 * Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.
 */
internal val ML_CODES: Array<LongArray> = arrayOf(
    // codes 0..31: individual values 3..34
    longArrayOf(3, 0),  longArrayOf(4, 0),  longArrayOf(5, 0),  longArrayOf(6, 0),
    longArrayOf(7, 0),  longArrayOf(8, 0),  longArrayOf(9, 0),  longArrayOf(10, 0),
    longArrayOf(11, 0), longArrayOf(12, 0), longArrayOf(13, 0), longArrayOf(14, 0),
    longArrayOf(15, 0), longArrayOf(16, 0), longArrayOf(17, 0), longArrayOf(18, 0),
    longArrayOf(19, 0), longArrayOf(20, 0), longArrayOf(21, 0), longArrayOf(22, 0),
    longArrayOf(23, 0), longArrayOf(24, 0), longArrayOf(25, 0), longArrayOf(26, 0),
    longArrayOf(27, 0), longArrayOf(28, 0), longArrayOf(29, 0), longArrayOf(30, 0),
    longArrayOf(31, 0), longArrayOf(32, 0), longArrayOf(33, 0), longArrayOf(34, 0),
    // codes 32+: grouped ranges
    longArrayOf(35, 1), longArrayOf(37, 1),  longArrayOf(39, 1),  longArrayOf(41, 1),
    longArrayOf(43, 2), longArrayOf(47, 2),
    longArrayOf(51, 3), longArrayOf(59, 3),
    longArrayOf(67, 4), longArrayOf(83, 4),
    longArrayOf(99, 5), longArrayOf(131, 7),
    longArrayOf(259, 8), longArrayOf(515, 9), longArrayOf(1027, 10), longArrayOf(2051, 11),
    longArrayOf(4099, 12), longArrayOf(8195, 13), longArrayOf(16387, 14),
    longArrayOf(32771, 15), longArrayOf(65539, 16),
)

// ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted.
// The decoder builds the same table from these fixed distributions.
//
// Entries of -1 mean "probability 1/table_size" — these symbols get one slot
// in the decode table and their encoder state never needs extra bits.

/**
 * Predefined normalised distribution for Literal Length FSE.
 * Table accuracy log = 6 → 64 slots.
 */
internal val LL_NORM = shortArrayOf(
     4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
     2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
    -1, -1, -1, -1,
)
internal const val LL_ACC_LOG = 6 // table_size = 64

/**
 * Predefined normalised distribution for Match Length FSE.
 * Table accuracy log = 6 → 64 slots.
 */
internal val ML_NORM = shortArrayOf(
     1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
    -1, -1, -1, -1, -1,
)
internal const val ML_ACC_LOG = 6

/**
 * Predefined normalised distribution for Offset FSE.
 * Table accuracy log = 5 → 32 slots.
 */
internal val OF_NORM = shortArrayOf(
     1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
     1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
)
internal const val OF_ACC_LOG = 5 // table_size = 32

// ─── FSE decode table entry ───────────────────────────────────────────────────

/**
 * One cell in the FSE decode table.
 *
 * To decode a symbol from state S:
 *   1. `sym` is the output symbol.
 *   2. Read `nb` bits from the bitstream as `bits`.
 *   3. New state = `base + bits`.
 */
internal data class FseDe(
    val sym: Int,   // decoded symbol
    val nb: Int,    // number of extra bits to read for next state
    val base: Int,  // base value for next state computation
)

/**
 * Encode transform for one symbol.
 *
 * Given encoder state S for symbol `s`:
 *   nb_out = (S + delta_nb) >> 16   (number of bits to emit)
 *   emit low nb_out bits of S
 *   new_S  = state_tbl[(S >> nb_out) + delta_fs]
 *
 * The `deltaNb` and `deltaFs` values are precomputed from the distribution
 * so the hot-path encode loop needs only arithmetic and a table lookup.
 */
internal data class FseEe(
    /** `(max_bits_out shl 16) - (count shl max_bits_out)` */
    val deltaNb: Long,
    /** `cumulative_count_before_sym - count` (may be negative) */
    val deltaFs: Int,
)

// ─── FSE table construction ───────────────────────────────────────────────────

/**
 * Build an FSE decode table from a normalised probability distribution.
 *
 * The algorithm:
 *  1. Place symbols with probability -1 (very rare) at the top of the table.
 *  2. Spread remaining symbols using a deterministic step function derived
 *     from the table size. This ensures each symbol occupies the correct
 *     fraction of slots.
 *  3. Assign `nb` (number of state bits to read) and `base` to each slot so
 *     that the decoder can reconstruct the next state.
 *
 * The step function `step = (sz shr 1) + (sz shr 3) + 3` is co-prime to `sz`
 * when `sz` is a power of two (which it always is in ZStd), ensuring that
 * the walk visits every slot exactly once.
 */
internal fun buildDecodeTable(norm: ShortArray, accLog: Int): Array<FseDe> {
    val sz = 1 shl accLog
    val step = (sz shr 1) + (sz shr 3) + 3
    val tbl = Array(sz) { FseDe(0, 0, 0) }
    val symNext = IntArray(norm.size)

    // Phase 1: symbols with probability -1 go at the top (high indices).
    // These symbols each get exactly 1 slot, and their state transition uses
    // the full accLog bits (they can go to any state).
    var high = sz - 1
    for (s in norm.indices) {
        if (norm[s].toInt() == -1) {
            tbl[high] = FseDe(s, 0, 0)
            if (high > 0) high--
            symNext[s] = 1
        }
    }

    // Phase 2: spread remaining symbols into the lower portion of the table.
    // Two-pass approach: first symbols with count > 1, then count == 1.
    // This matches the reference implementation's deterministic ordering.
    var pos = 0
    for (pass in 0 until 2) {
        for (s in norm.indices) {
            if (norm[s] <= 0) continue
            val cnt = norm[s].toInt()
            if ((pass == 0) != (cnt > 1)) continue
            symNext[s] = cnt
            repeat(cnt) {
                tbl[pos] = FseDe(s, 0, 0)
                pos = (pos + step) and (sz - 1)
                while (pos > high) {
                    pos = (pos + step) and (sz - 1)
                }
            }
        }
    }

    // Phase 3: assign nb (number of state bits to read) and base.
    //
    // For a symbol with count `cnt` occupying slots i₀, i₁, ...:
    //   The next_state counter starts at `cnt` and increments.
    //   nb = accLog - floor(log2(next_state))
    //   base = next_state * (1 shl nb) - sz
    //
    // This ensures that when we reconstruct state = base + read(nb bits),
    // we land in the range [sz, 2*sz), which is the valid encoder state range.
    val sn = symNext.clone()
    for (i in 0 until sz) {
        val s = tbl[i].sym
        val ns = sn[s]
        sn[s] += 1
        // floor(log2(ns)) = 31 - Integer.numberOfLeadingZeros(ns)
        val nb = accLog - (31 - Integer.numberOfLeadingZeros(ns))
        // base = ns * (1 shl nb) - sz
        val base = (ns shl nb) - sz
        tbl[i] = FseDe(s, nb, base)
    }

    return tbl
}

/**
 * Build FSE encode tables from a normalised distribution.
 *
 * Returns:
 * - `ee[sym]`: the [FseEe] transform for each symbol.
 * - `st[slot]`: the encoder state table (slot → output state in [sz, 2*sz)).
 *
 * The encode/decode symmetry: the FSE decoder assigns `(sym, nb, base)` to
 * each table cell in INDEX ORDER. For symbol `s`, the j-th cell (in ascending
 * index order) has:
 *   ns = count[s] + j
 *   nb = accLog - floor(log2(ns))
 *   base = ns * (1 shl nb) - sz
 *
 * The FSE encoder must use the SAME indexing: slot `cumul[s]+j` maps to the
 * j-th table cell for symbol `s` (in ascending index order).
 */
internal fun buildEncodeTable(norm: ShortArray, accLog: Int): Pair<Array<FseEe>, IntArray> {
    val sz = 1 shl accLog

    // Step 1: compute cumulative sums.
    val cumul = IntArray(norm.size)
    var total = 0
    for (s in norm.indices) {
        cumul[s] = total
        val cnt = if (norm[s].toInt() == -1) 1 else maxOf(0, norm[s].toInt())
        total += cnt
    }

    // Step 2: build the spread table (which symbol occupies each table slot).
    //
    // This uses the same spreading algorithm as buildDecodeTable, producing
    // a mapping from table index to symbol.
    val step = (sz shr 1) + (sz shr 3) + 3
    val spread = IntArray(sz)
    var idxHigh = sz - 1

    // Phase 1: probability -1 symbols at the high end
    for (s in norm.indices) {
        if (norm[s].toInt() == -1) {
            spread[idxHigh] = s
            if (idxHigh > 0) idxHigh--
        }
    }
    val idxLimit = idxHigh

    // Phase 2: spread remaining symbols using the step function
    var pos2 = 0
    for (pass in 0 until 2) {
        for (s in norm.indices) {
            if (norm[s] <= 0) continue
            val cnt = norm[s].toInt()
            if ((pass == 0) != (cnt > 1)) continue
            repeat(cnt) {
                spread[pos2] = s
                pos2 = (pos2 + step) and (sz - 1)
                while (pos2 > idxLimit) {
                    pos2 = (pos2 + step) and (sz - 1)
                }
            }
        }
    }

    // Step 3: build the state table by iterating spread in INDEX ORDER.
    //
    // For each table index `i` (in ascending order), determine which
    // occurrence of symbol `s = spread[i]` this is (j = 0, 1, 2, ...).
    // The encode slot is `cumul[s] + j`, and the encoder output state is
    // `i + sz` (so the decoder, in state `i`, will decode symbol `s`).
    val symOcc = IntArray(norm.size)
    val st = IntArray(sz)

    for (i in 0 until sz) {
        val s = spread[i]
        val j = symOcc[s]
        symOcc[s]++
        val slot = cumul[s] + j
        st[slot] = i + sz
    }

    // Step 4: build FseEe entries.
    //
    // For symbol s with count c and max_bits_out mbo:
    //   deltaNb = (mbo shl 16) - (c shl mbo)
    //   deltaFs = cumul[s] - c
    //
    // Encode step: given current encoder state E ∈ [sz, 2*sz):
    //   nb = (E + deltaNb) shr 16     (number of state bits to emit)
    //   emit low nb bits of E
    //   new_E = st[(E shr nb) + deltaFs]
    val ee = Array(norm.size) { FseEe(0L, 0) }
    for (s in norm.indices) {
        val cnt = if (norm[s].toInt() == -1) 1 else maxOf(0, norm[s].toInt())
        if (cnt == 0) continue
        val mbo: Int = if (cnt == 1) {
            accLog
        } else {
            // max_bits_out = accLog - floor(log2(cnt))
            accLog - (31 - Integer.numberOfLeadingZeros(cnt))
        }
        val deltaNb = (mbo.toLong() shl 16) - (cnt.toLong() shl mbo)
        val deltaFs = cumul[s] - cnt
        ee[s] = FseEe(deltaNb, deltaFs)
    }

    return Pair(ee, st)
}

// ─── Reverse bit-writer ───────────────────────────────────────────────────────
//
// ZStd's sequence bitstream is written *backwards* relative to the data flow:
// the encoder writes bits that the decoder will read last, first. This allows
// the decoder to read a forward-only stream while decoding sequences in order.
//
// Byte layout: `[byte0, byte1, ..., byteN]` where `byteN` is the last byte
// written, and it contains a **sentinel bit** (the highest set bit) that marks
// the end of meaningful data. The decoder initialises by finding this sentinel.
//
// Bit layout within each byte: LSB = first bit written.
//
// Example: write bits `1, 0, 1, 1` (4 bits) then flush:
//   reg = 0b1011, bits = 4
//   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
//   buf = [0x1B]
//
// The decoder reads this as: find MSB (bit 4 = sentinel), then read
// bits 3..0 = 0b1011 = the original 4 bits.

internal class RevBitWriter {
    private val buf = ArrayList<Byte>()
    private var reg: Long = 0L   // accumulation register (bits fill from LSB)
    private var bits: Int = 0    // number of valid bits in reg

    /**
     * Add `nb` low-order bits of `val` to the stream.
     */
    fun addBits(value: Long, nb: Int) {
        if (nb == 0) return
        val mask = if (nb == 64) -1L else (1L shl nb) - 1L
        reg = reg or ((value and mask) shl bits)
        bits += nb
        while (bits >= 8) {
            buf.add(reg.toByte())
            reg = reg ushr 8
            bits -= 8
        }
    }

    /**
     * Flush remaining bits with a sentinel and mark the stream end.
     *
     * The sentinel is a `1` bit placed at position `bits` in the last byte.
     * The decoder locates it with `numberOfLeadingZeros` arithmetic.
     */
    fun flush() {
        val sentinel = (1 shl bits).toByte()
        val lastByte = ((reg.toInt() and 0xFF) or sentinel.toInt()).toByte()
        buf.add(lastByte)
        reg = 0L
        bits = 0
    }

    fun finish(): ByteArray = buf.toByteArray()
}

// ─── Reverse bit-reader ───────────────────────────────────────────────────────
//
// Mirrors RevBitWriter: reads bits from the END of the buffer going backwards.
// The stream is laid out so that the LAST bits written by the encoder are at the
// END of the byte buffer (in the sentinel-containing last byte). The reader
// initialises at the last byte and reads backward toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// `readBits(n)` extracts the top n bits and shifts the register left by n.
//
// Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
// byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
// bits first (which were in the highest byte positions and in the high bits of
// each byte), we need a left-aligned register so that reading from the top
// gives the highest-position bits first.

internal class RevBitReader(private val data: ByteArray) {
    private var reg: Long = 0L   // shift register, valid bits packed at TOP (MSB side)
    private var bits: Int = 0    // how many valid bits are loaded (count from MSB)
    private var pos: Int         // index of the next byte to load (decrements toward 0)

    init {
        if (data.isEmpty()) throw IOException("empty bitstream")

        // Find the sentinel bit in the last byte.
        // The sentinel is the highest set bit; valid data bits are below it.
        val last = data.last().toInt() and 0xFF
        if (last == 0) throw IOException("bitstream last byte is zero (no sentinel)")

        // sentinel_pos = bit index (0 = LSB) of the sentinel in the last byte
        var sentinelPos = 0
        for (b in 7 downTo 0) {
            if (last and (1 shl b) != 0) {
                sentinelPos = b
                break
            }
        }
        val validBits = sentinelPos // number of data bits below the sentinel

        // Place the valid bits of the sentinel byte at the TOP of the register.
        val mask = if (validBits == 0) 0L else (1L shl validBits) - 1L
        reg = if (validBits == 0) 0L else ((last.toLong() and mask) shl (64 - validBits))

        bits = validBits
        pos = data.size - 1 // sentinel byte already consumed; load from here-1

        // Fill the register from earlier bytes.
        reload()
    }

    /**
     * Load more bytes into the register from the stream going backward.
     *
     * Each new byte is placed just BELOW the currently loaded bits (in the
     * left-aligned register, that means at position `64 - bits - 8`).
     */
    private fun reload() {
        while (bits <= 56 && pos > 0) {
            pos--
            val shift = 64 - bits - 8
            reg = reg or ((data[pos].toLong() and 0xFFL) shl shift)
            bits += 8
        }
    }

    /**
     * Read `nb` bits from the top of the register (returns 0 if nb == 0).
     *
     * This returns the most recently written bits first (highest stream
     * positions first), mirroring the encoder's backward order.
     */
    fun readBits(nb: Int): Long {
        if (nb == 0) return 0L
        val value = reg ushr (64 - nb)
        reg = if (nb == 64) 0L else (reg shl nb)
        bits = maxOf(0, bits - nb)
        if (bits < 24) reload()
        return value
    }
}

// ─── FSE encode/decode helpers ────────────────────────────────────────────────

/**
 * Encode one symbol into the backward bitstream, updating the FSE state.
 *
 * The encoder maintains state in `[sz, 2*sz)`. To emit symbol `sym`:
 * 1. Compute how many bits to flush: `nb = (state + deltaNb) shr 16`
 * 2. Write the low `nb` bits of `state` to the bitstream.
 * 3. New state = `st[(state shr nb) + deltaFs]`
 *
 * After all symbols are encoded, the final state (minus `sz`) is written as
 * `accLog` bits to allow the decoder to initialise.
 */
private fun fseEncodeSym(
    state: LongArray,   // state[0] is the mutable state
    sym: Int,
    ee: Array<FseEe>,
    st: IntArray,
    bw: RevBitWriter,
) {
    val e = ee[sym]
    val nb = ((state[0] + e.deltaNb) ushr 16).toInt()
    bw.addBits(state[0], nb)
    val slotI = (state[0] ushr nb).toInt() + e.deltaFs
    val slot = maxOf(0, slotI)
    state[0] = st[slot].toLong()
}

/**
 * Decode one symbol from the backward bitstream, updating the FSE state.
 *
 * 1. Look up `de[state]` to get `sym`, `nb`, and `base`.
 * 2. New state = `base + read(nb bits)`.
 */
private fun fseDecodeSym(
    state: IntArray,   // state[0] is the mutable state
    de: Array<FseDe>,
    br: RevBitReader,
): Int {
    val e = de[state[0]]
    val sym = e.sym
    state[0] = (e.base + br.readBits(e.nb)).toInt()
    return sym
}

// ─── LL/ML/OF code number computation ────────────────────────────────────────

/**
 * Map a literal length value to its LL code number (0..35).
 *
 * Codes 0..15 are identity; codes 16+ cover ranges via lookup.
 * Linear scan: last code whose baseline ≤ ll is the correct code.
 */
internal fun llToCode(ll: Long): Int {
    var code = 0
    for (i in LL_CODES.indices) {
        if (LL_CODES[i][0] <= ll) code = i else break
    }
    return code
}

/**
 * Map a match length value to its ML code number (0..52).
 */
internal fun mlToCode(ml: Long): Int {
    var code = 0
    for (i in ML_CODES.indices) {
        if (ML_CODES[i][0] <= ml) code = i else break
    }
    return code
}

// ─── Sequence struct ──────────────────────────────────────────────────────────

/**
 * One ZStd sequence: (literal_length, match_length, match_offset).
 *
 * A sequence means: emit `ll` literal bytes from the literals section,
 * then copy `ml` bytes starting `off` positions back in the output buffer.
 * After all sequences, any remaining literals are appended.
 */
internal data class Seq(
    val ll: Long,  // literal length (bytes to copy from literal section before this match)
    val ml: Long,  // match length (bytes to copy from output history)
    val off: Long, // match offset (1-indexed: 1 = last byte written)
)

/**
 * Convert LZSS tokens into ZStd sequences + a flat literals buffer.
 *
 * LZSS produces a stream of `Literal(byte)` and `Match{offset, length}`.
 * ZStd groups consecutive literals before each match into a single sequence.
 * Any trailing literals (after the last match) go into the literals buffer
 * without a corresponding sequence entry.
 */
internal fun tokensToSeqs(tokens: List<LzssToken>): Pair<ByteArray, List<Seq>> {
    val lits = ArrayList<Byte>()
    val seqs = ArrayList<Seq>()
    var litRun = 0L

    for (tok in tokens) {
        when (tok) {
            is LzssToken.Literal -> {
                lits.add(tok.value)
                litRun++
            }
            is LzssToken.Match -> {
                seqs.add(Seq(litRun, tok.length.toLong(), tok.offset.toLong()))
                litRun = 0L
            }
        }
    }
    // Trailing literals stay in `lits`; no sequence for them.
    return Pair(lits.toByteArray(), seqs)
}

// ─── Literals section encoding ────────────────────────────────────────────────
//
// ZStd literals can be Huffman-coded or raw. We use **Raw_Literals** (type=0),
// which is the simplest: no Huffman table, bytes are stored verbatim.
//
// Header format depends on literal count:
//   ≤ 31 bytes:   1-byte header  = (lit_len shl 3) or 0b000
//   ≤ 4095 bytes: 2-byte header  = (lit_len shl 4) or 0b0100
//   else:         3-byte header  = (lit_len shl 4) or 0b1100
//
// The bottom 2 bits = Literals_Block_Type (0 = Raw).
// The next 2 bits = Size_Format.

internal fun encodeLiteralsSection(lits: ByteArray): ByteArray {
    val n = lits.size
    val out = ArrayList<Byte>(n + 3)

    // Raw_Literals header format (RFC 8878 §3.1.1.2.1):
    // bits [1:0] = Literals_Block_Type = 00 (Raw)
    // bits [3:2] = Size_Format: 00 or 10 = 1-byte, 01 = 2-byte, 11 = 3-byte
    //
    // 1-byte:  size in bits [7:3] (5 bits) — header = (size shl 3) or 0b000
    // 2-byte:  size in bits [11:4] (12 bits) — header = (size shl 4) or 0b0100
    // 3-byte:  size in bits [19:4] (16 bits) — header = (size shl 4) or 0b1100
    if (n <= 31) {
        // 1-byte header: size_format=00, type=00
        out.add(((n shl 3) and 0xFF).toByte())
    } else if (n <= 4095) {
        // 2-byte header: size_format=01, type=00 → 0b0100
        val hdr = (n shl 4) or 0b0100
        out.add((hdr and 0xFF).toByte())
        out.add(((hdr shr 8) and 0xFF).toByte())
    } else {
        // 3-byte header: size_format=11, type=00 → 0b1100
        val hdr = (n shl 4) or 0b1100
        out.add((hdr and 0xFF).toByte())
        out.add(((hdr shr 8) and 0xFF).toByte())
        out.add(((hdr shr 16) and 0xFF).toByte())
    }

    for (b in lits) out.add(b)
    return out.toByteArray()
}

/**
 * Decode literals section, returning `(literals, bytes_consumed)`.
 */
internal fun decodeLiteralsSection(data: ByteArray): Pair<ByteArray, Int> {
    if (data.isEmpty()) throw IOException("empty literals section")

    val b0 = data[0].toInt() and 0xFF
    val ltype = b0 and 0b11 // bottom 2 bits = Literals_Block_Type

    if (ltype != 0) {
        throw IOException("unsupported literals type $ltype (only Raw=0 supported)")
    }

    // Decode size_format from bits [3:2] of b0
    val sizeFormat = (b0 shr 2) and 0b11

    // Decode the literal length and header byte count from size_format.
    //
    // Raw_Literals size_format encoding (RFC 8878 §3.1.1.2.1):
    //   0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0..31)
    //   0b01          → 2-byte LE header: size in bits [11:4] (12 bits, values 0..4095)
    //   0b11          → 3-byte LE header: size in bits [19:4] (20 bits, values 0..1MB)
    val (n, headerBytes) = when (sizeFormat) {
        0, 2 -> {
            // 1-byte header: size in bits [7:3] (5 bits = values 0..31)
            Pair(b0 shr 3, 1)
        }
        1 -> {
            // 2-byte header: 12-bit size
            if (data.size < 2) throw IOException("truncated literals header (2-byte)")
            val nb = ((b0 shr 4) and 0xF) or ((data[1].toInt() and 0xFF) shl 4)
            Pair(nb, 2)
        }
        3 -> {
            // 3-byte header: 20-bit size (enough for blocks up to 1 MB)
            if (data.size < 3) throw IOException("truncated literals header (3-byte)")
            val nb = ((b0 shr 4) and 0xF) or
                     ((data[1].toInt() and 0xFF) shl 4) or
                     ((data[2].toInt() and 0xFF) shl 12)
            Pair(nb, 3)
        }
        else -> throw IOException("impossible size_format")
    }

    val start = headerBytes
    val end = start + n
    if (end > data.size) {
        throw IOException("literals data truncated: need $end, have ${data.size}")
    }

    return Pair(data.copyOfRange(start, end), end)
}

// ─── Sequences section encoding ───────────────────────────────────────────────
//
// Layout:
//   [sequence_count: 1-3 bytes]
//   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
//   [FSE bitstream: variable]
//
// Symbol compression modes byte:
//   bits [7:6] = LL mode
//   bits [5:4] = OF mode
//   bits [3:2] = ML mode
//   bits [1:0] = reserved (0)
// Mode 0 = Predefined, Mode 1 = RLE, Mode 2 = FSE_Compressed, Mode 3 = Repeat.
// We always write 0x00 (all Predefined).
//
// The FSE bitstream is a backward bit-stream (reverse bit writer):
//   - Sequences are encoded in REVERSE ORDER (last first).
//   - For each sequence:
//       OF extra bits, ML extra bits, LL extra bits  (in this order)
//       then FSE symbol for ML, OF, LL              (reversed decode order)
//   - After all sequences, flush the final FSE states:
//       (state_of - sz_of) as OF_ACC_LOG bits
//       (state_ml - sz_ml) as ML_ACC_LOG bits
//       (state_ll - sz_ll) as LL_ACC_LOG bits
//   - Add sentinel and flush.
//
// The decoder does the mirror:
//   1. Read LL_ACC_LOG bits → initial state_ll
//   2. Read ML_ACC_LOG bits → initial state_ml
//   3. Read OF_ACC_LOG bits → initial state_of
//   4. For each sequence:
//       decode LL symbol (state transition)
//       decode OF symbol
//       decode ML symbol
//       read LL extra bits
//       read ML extra bits
//       read OF extra bits
//   5. Apply sequence to output buffer.

internal fun encodeSeqCount(count: Int): ByteArray {
    return if (count == 0) {
        byteArrayOf(0)
    } else if (count < 128) {
        byteArrayOf(count.toByte())
    } else if (count < 0x7FFF) {
        val v = (count or 0x8000).toShort()
        // Write as little-endian u16
        byteArrayOf((v.toInt() and 0xFF).toByte(), ((v.toInt() shr 8) and 0xFF).toByte())
    } else {
        // 3-byte encoding: first byte = 0xFF, next 2 bytes = count - 0x7F00
        val r = count - 0x7F00
        byteArrayOf(0xFF.toByte(), (r and 0xFF).toByte(), ((r shr 8) and 0xFF).toByte())
    }
}

internal fun decodeSeqCount(data: ByteArray): Pair<Int, Int> {
    if (data.isEmpty()) throw IOException("empty sequence count")
    val b0 = data[0].toInt() and 0xFF
    return if (b0 < 128) {
        Pair(b0, 1)
    } else if (b0 < 0xFF) {
        // 2-byte encoding: the pair is a LE u16 with the high bit set.
        if (data.size < 2) throw IOException("truncated sequence count")
        val v = (b0 and 0xFF) or ((data[1].toInt() and 0xFF) shl 8)
        Pair(v and 0x7FFF, 2)
    } else {
        // 3-byte encoding: byte0=0xFF, then (count - 0x7F00) as LE u16
        if (data.size < 3) throw IOException("truncated sequence count (3-byte)")
        val count = 0x7F00 + (data[1].toInt() and 0xFF) + ((data[2].toInt() and 0xFF) shl 8)
        Pair(count, 3)
    }
}

/**
 * Encode the sequences section using predefined FSE tables.
 */
internal fun encodeSequencesSection(seqs: List<Seq>): ByteArray {
    // Build encode tables (precomputed from the predefined distributions).
    val (eeLl, stLl) = buildEncodeTable(LL_NORM, LL_ACC_LOG)
    val (eeMl, stMl) = buildEncodeTable(ML_NORM, ML_ACC_LOG)
    val (eeOf, stOf) = buildEncodeTable(OF_NORM, OF_ACC_LOG)

    val szLl = (1 shl LL_ACC_LOG).toLong()
    val szMl = (1 shl ML_ACC_LOG).toLong()
    val szOf = (1 shl OF_ACC_LOG).toLong()

    // FSE encoder states start at table_size (= sz).
    // The state range [sz, 2*sz) maps to slot range [0, sz).
    val stateLl = longArrayOf(szLl)
    val stateMl = longArrayOf(szMl)
    val stateOf = longArrayOf(szOf)

    val bw = RevBitWriter()

    // Encode sequences in reverse order.
    for (i in seqs.indices.reversed()) {
        val seq = seqs[i]
        val llCode = llToCode(seq.ll)
        val mlCode = mlToCode(seq.ml)

        // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
        // code = floor(log2(raw)); extra = raw - (1 shl code)
        val rawOff = seq.off + 3L
        val ofCode = if (rawOff <= 1L) 0 else (63 - java.lang.Long.numberOfLeadingZeros(rawOff)).toInt()
        val ofExtra = rawOff - (1L shl ofCode)

        // Write extra bits (OF, ML, LL in this order for backward stream).
        bw.addBits(ofExtra, ofCode)
        val mlExtra = seq.ml - ML_CODES[mlCode][0]
        bw.addBits(mlExtra, ML_CODES[mlCode][1].toInt())
        val llExtra = seq.ll - LL_CODES[llCode][0]
        bw.addBits(llExtra, LL_CODES[llCode][1].toInt())

        // FSE encode symbols in the order that the backward bitstream reverses
        // to match the decoder's read order (LL first, OF second, ML third).
        //
        // Since the backward stream reverses write order, we write the REVERSE
        // of the decode order: ML → OF → LL (LL is written last = at the top
        // of the bitstream = read first by the decoder).
        //
        // Decode order: LL, OF, ML
        // Encode order (reversed): ML, OF, LL
        fseEncodeSym(stateMl, mlCode, eeMl, stMl, bw)
        fseEncodeSym(stateOf, ofCode, eeOf, stOf, bw)
        fseEncodeSym(stateLl, llCode, eeLl, stLl, bw)
    }

    // Flush final states (low accLog bits of state - sz).
    bw.addBits(stateOf[0] - szOf, OF_ACC_LOG)
    bw.addBits(stateMl[0] - szMl, ML_ACC_LOG)
    bw.addBits(stateLl[0] - szLl, LL_ACC_LOG)
    bw.flush()

    return bw.finish()
}

// ─── Block-level compress ─────────────────────────────────────────────────────

/**
 * Compress one block into ZStd compressed block format.
 *
 * Returns `null` if the compressed form is larger than the input (in which
 * case the caller should use a Raw block instead).
 */
private fun compressBlock(block: ByteArray): ByteArray? {
    // Use LZSS to generate LZ77 tokens.
    // Window = 32 KB, max match = 255, min match = 3 (same as ZStd spec defaults
    // with a bigger window to improve compression ratio).
    val tokens = Lzss.encode(block, 32768, 255, 3)

    // Convert tokens to ZStd sequences.
    val (lits, seqs) = tokensToSeqs(tokens)

    // If no sequences were found, LZ77 had nothing to compress.
    // A compressed block with 0 sequences still has overhead, so fall back.
    if (seqs.isEmpty()) return null

    val out = ArrayList<Byte>()

    // Encode literals section (Raw_Literals).
    for (b in encodeLiteralsSection(lits)) out.add(b)

    // Encode sequences section.
    for (b in encodeSeqCount(seqs.size)) out.add(b)
    out.add(0x00) // Symbol_Compression_Modes = all Predefined

    val bitstream = encodeSequencesSection(seqs)
    for (b in bitstream) out.add(b)

    return if (out.size >= block.size) null else out.toByteArray()
}

/**
 * Decompress one ZStd compressed block.
 *
 * Reads the literals section, sequences section, and applies the sequences
 * to the output buffer to reconstruct the original data.
 */
private fun decompressBlock(data: ByteArray, output: ArrayList<Byte>) {
    // ── Literals section ─────────────────────────────────────────────────
    val (lits, litConsumed) = decodeLiteralsSection(data)
    var pos = litConsumed

    // ── Sequences count ──────────────────────────────────────────────────
    if (pos >= data.size) {
        // Block has only literals, no sequences.
        for (b in lits) output.add(b)
        return
    }

    val (nSeqs, scBytes) = decodeSeqCount(data.copyOfRange(pos, data.size))
    pos += scBytes

    if (nSeqs == 0) {
        // No sequences — all content is in literals.
        for (b in lits) output.add(b)
        return
    }

    // ── Symbol compression modes ─────────────────────────────────────────
    if (pos >= data.size) throw IOException("missing symbol compression modes byte")
    val modesByte = data[pos].toInt() and 0xFF
    pos++

    // Check that all modes are Predefined (0).
    val llMode = (modesByte shr 6) and 3
    val ofMode = (modesByte shr 4) and 3
    val mlMode = (modesByte shr 2) and 3
    if (llMode != 0 || ofMode != 0 || mlMode != 0) {
        throw IOException(
            "unsupported FSE modes: LL=$llMode OF=$ofMode ML=$mlMode (only Predefined=0 supported)"
        )
    }

    // ── FSE bitstream ────────────────────────────────────────────────────
    val bitstream = data.copyOfRange(pos, data.size)
    val br = RevBitReader(bitstream)

    // Build decode tables from predefined distributions.
    val dtLl = buildDecodeTable(LL_NORM, LL_ACC_LOG)
    val dtMl = buildDecodeTable(ML_NORM, ML_ACC_LOG)
    val dtOf = buildDecodeTable(OF_NORM, OF_ACC_LOG)

    // Initialise FSE states from the bitstream.
    // The encoder wrote: state_ll, state_ml, state_of (each as accLog bits).
    // The decoder reads them in the same order.
    val stateLl = intArrayOf(br.readBits(LL_ACC_LOG).toInt())
    val stateMl = intArrayOf(br.readBits(ML_ACC_LOG).toInt())
    val stateOf = intArrayOf(br.readBits(OF_ACC_LOG).toInt())

    // Track position in the literals buffer.
    var litPos = 0

    // Apply each sequence.
    for (seqIdx in 0 until nSeqs) {
        // Decode symbols (state transitions) — order: LL, OF, ML.
        val llCode = fseDecodeSym(stateLl, dtLl, br)
        val ofCode = fseDecodeSym(stateOf, dtOf, br)
        val mlCode = fseDecodeSym(stateMl, dtMl, br)

        // Validate codes.
        if (llCode >= LL_CODES.size) throw IOException("invalid LL code $llCode")
        if (mlCode >= ML_CODES.size) throw IOException("invalid ML code $mlCode")

        val llInfo = LL_CODES[llCode]
        val mlInfo = ML_CODES[mlCode]

        val ll = llInfo[0] + br.readBits(llInfo[1].toInt())
        val ml = mlInfo[0] + br.readBits(mlInfo[1].toInt())

        // Offset: raw = (1 shl of_code) or extra_bits; offset = raw - 3
        val ofRaw = (1L shl ofCode) or br.readBits(ofCode)
        if (ofRaw < 3L) throw IOException("decoded offset underflow: of_raw=$ofRaw")
        val offset = ofRaw - 3L

        // Emit `ll` literal bytes from the literals buffer.
        val litEnd = litPos + ll.toInt()
        if (litEnd > lits.size) {
            throw IOException(
                "literal run $ll overflows literals buffer (pos=$litPos len=${lits.size})"
            )
        }
        for (k in litPos until litEnd) output.add(lits[k])
        litPos = litEnd

        // Copy `ml` bytes from `offset` back in the output buffer.
        // offset = 0 would reference past the end; minimum valid offset is 1.
        if (offset == 0L || offset > output.size.toLong()) {
            throw IOException("bad match offset $offset (output len ${output.size})")
        }
        val copyStart = output.size - offset.toInt()
        for (k in 0 until ml.toInt()) {
            output.add(output[copyStart + k])
        }
    }

    // Any remaining literals after the last sequence.
    for (k in litPos until lits.size) output.add(lits[k])
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Pure-Kotlin ZStd compression and decompression (RFC 8878 / CMP07).
 *
 * Supports the full predefined-FSE subset of the ZStd format:
 *   - Raw, RLE, and Compressed blocks
 *   - Predefined FSE tables for LL / ML / OF
 *   - Raw literals (no Huffman coding)
 *
 * The output of [compress] is a conforming ZStd frame readable by the
 * `zstd` CLI or any other conforming decoder.
 *
 * ## Example
 * ```kotlin
 * val original = "hello, ZStd!".encodeToByteArray()
 * val compressed = Zstd.compress(original)
 * val restored = Zstd.decompress(compressed)
 * assert(original.contentEquals(restored))
 * ```
 */
object Zstd {

    /**
     * Compress [data] to ZStd format (RFC 8878).
     *
     * The output is a valid ZStd frame that can be decompressed by the `zstd`
     * CLI tool or any conforming implementation.
     *
     * @param data the uncompressed input bytes
     * @return ZStd-compressed bytes
     */
    fun compress(data: ByteArray): ByteArray {
        val out = ArrayList<Byte>()

        // ── ZStd frame header ────────────────────────────────────────────────
        // Magic number (4 bytes LE).
        // MAGIC = 0xFD2FB528L; written as little-endian u32.
        val magicBuf = ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN)
        magicBuf.putInt(MAGIC.toInt())
        for (b in magicBuf.array()) out.add(b)

        // Frame Header Descriptor (FHD):
        //   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
        //   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
        //   bit 4:   Content_Checksum_Flag = 0
        //   bit 3-2: reserved = 0
        //   bit 1-0: Dict_ID_Flag = 0
        // = 0b1110_0000 = 0xE0
        out.add(0xE0.toByte())

        // Frame_Content_Size (8 bytes LE) — the uncompressed size.
        val fcsBuf = ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN)
        fcsBuf.putLong(data.size.toLong())
        for (b in fcsBuf.array()) out.add(b)

        // ── Blocks ───────────────────────────────────────────────────────────
        // Handle the special case of completely empty input: emit one empty raw block.
        if (data.isEmpty()) {
            // Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
            out.add(0x01)
            out.add(0x00)
            out.add(0x00)
            return out.toByteArray()
        }

        var offset = 0
        while (offset < data.size) {
            val end = minOf(offset + MAX_BLOCK_SIZE, data.size)
            val block = data.copyOfRange(offset, end)
            val last = end == data.size

            // ── Try RLE block ─────────────────────────────────────────────
            // If all bytes in the block are identical, a single-byte RLE block
            // encodes it in just 1 byte (plus 3-byte header = 4 bytes total).
            val firstByte = block[0]
            val allSame = block.all { it == firstByte }

            if (allSame) {
                // RLE block header: type=01, size=blockLen, last flag
                val hdr = (block.size.toLong() shl 3) or (0b01L shl 1) or (if (last) 1L else 0L)
                out.add((hdr and 0xFF).toByte())
                out.add(((hdr shr 8) and 0xFF).toByte())
                out.add(((hdr shr 16) and 0xFF).toByte())
                out.add(firstByte)
            } else {
                // ── Try compressed block ──────────────────────────────────
                val compressed = compressBlock(block)
                if (compressed != null) {
                    val hdr = (compressed.size.toLong() shl 3) or (0b10L shl 1) or (if (last) 1L else 0L)
                    out.add((hdr and 0xFF).toByte())
                    out.add(((hdr shr 8) and 0xFF).toByte())
                    out.add(((hdr shr 16) and 0xFF).toByte())
                    for (b in compressed) out.add(b)
                } else {
                    // ── Raw block (fallback) ──────────────────────────────
                    val hdr = (block.size.toLong() shl 3) or (0b00L shl 1) or (if (last) 1L else 0L)
                    out.add((hdr and 0xFF).toByte())
                    out.add(((hdr shr 8) and 0xFF).toByte())
                    out.add(((hdr shr 16) and 0xFF).toByte())
                    for (b in block) out.add(b)
                }
            }

            offset = end
        }

        return out.toByteArray()
    }

    /**
     * Decompress a ZStd frame, returning the original data.
     *
     * Accepts any valid ZStd frame with:
     *   - Single-segment or multi-segment layout
     *   - Raw, RLE, or Compressed blocks
     *   - Predefined FSE modes (no per-frame table description)
     *
     * @param data ZStd-compressed bytes
     * @return the original uncompressed bytes
     * @throws IOException if the input is truncated, has a bad magic number,
     *   or contains unsupported features (non-predefined FSE tables, Huffman
     *   literals, reserved block types)
     */
    fun decompress(data: ByteArray): ByteArray {
        if (data.size < 5) throw IOException("frame too short")

        // ── Validate magic ───────────────────────────────────────────────────
        val buf = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN)
        val magic = buf.int.toLong() and 0xFFFFFFFFL
        if (magic != MAGIC) {
            throw IOException("bad magic: 0x${magic.toString(16)} (expected 0x${MAGIC.toString(16)})")
        }

        var pos = 4

        // ── Parse Frame Header Descriptor ───────────────────────────────────
        // FHD encodes several flags that control the header layout.
        val fhd = data[pos].toInt() and 0xFF
        pos++

        // FCS_Field_Size: bits [7:6] of FHD.
        //   00 → 0 bytes if Single_Segment=0, else 1 byte
        //   01 → 2 bytes
        //   10 → 4 bytes
        //   11 → 8 bytes
        val fcsFlag = (fhd shr 6) and 3

        // Single_Segment_Flag: bit 5. When set, the window descriptor is omitted.
        val singleSeg = (fhd shr 5) and 1

        // Dict_ID_Flag: bits [1:0]. Indicates how many bytes the dict ID occupies.
        val dictFlag = fhd and 3

        // ── Window Descriptor ────────────────────────────────────────────────
        // Present only if Single_Segment_Flag = 0. We skip it.
        if (singleSeg == 0) pos++

        // ── Dict ID ──────────────────────────────────────────────────────────
        val dictIdBytes = when (dictFlag) {
            0 -> 0; 1 -> 1; 2 -> 2; else -> 4
        }
        pos += dictIdBytes

        // ── Frame Content Size ───────────────────────────────────────────────
        // We read but don't validate FCS (we trust the blocks to be correct).
        val fcsBytes = when (fcsFlag) {
            0 -> if (singleSeg == 1) 1 else 0
            1 -> 2
            2 -> 4
            3 -> 8
            else -> 0
        }
        pos += fcsBytes

        // ── Blocks ───────────────────────────────────────────────────────────
        // Guard against decompression bombs: cap total output at 256 MB.
        val maxOutput = 256 * 1024 * 1024
        val output = ArrayList<Byte>()

        while (true) {
            if (pos + 3 > data.size) throw IOException("truncated block header")

            // 3-byte little-endian block header.
            val hdr = ((data[pos].toLong() and 0xFFL) or
                      ((data[pos + 1].toLong() and 0xFFL) shl 8) or
                      ((data[pos + 2].toLong() and 0xFFL) shl 16))
            pos += 3

            val last = (hdr and 1L) != 0L
            val btype = ((hdr shr 1) and 3L).toInt()
            val bsize = (hdr shr 3).toInt()

            when (btype) {
                0 -> {
                    // Raw block: `bsize` bytes of verbatim content.
                    if (pos + bsize > data.size) {
                        throw IOException("raw block truncated: need $bsize bytes at pos $pos")
                    }
                    if (output.size + bsize > maxOutput) {
                        throw IOException("decompressed size exceeds limit of $maxOutput bytes")
                    }
                    for (k in pos until pos + bsize) output.add(data[k])
                    pos += bsize
                }
                1 -> {
                    // RLE block: 1 byte repeated `bsize` times.
                    if (pos >= data.size) throw IOException("RLE block missing byte")
                    if (output.size + bsize > maxOutput) {
                        throw IOException("decompressed size exceeds limit of $maxOutput bytes")
                    }
                    val rleByte = data[pos]
                    pos++
                    repeat(bsize) { output.add(rleByte) }
                }
                2 -> {
                    // Compressed block.
                    if (pos + bsize > data.size) {
                        throw IOException("compressed block truncated: need $bsize bytes")
                    }
                    val blockData = data.copyOfRange(pos, pos + bsize)
                    pos += bsize
                    decompressBlock(blockData, output)
                }
                3 -> throw IOException("reserved block type 3")
                else -> throw IOException("unknown block type $btype")
            }

            if (last) break
        }

        return output.toByteArray()
    }
}
