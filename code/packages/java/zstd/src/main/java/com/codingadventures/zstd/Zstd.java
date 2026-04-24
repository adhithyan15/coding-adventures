package com.codingadventures.zstd;

// Zstandard (ZStd) lossless compression algorithm — CMP07.
//
// Zstandard (RFC 8878) is a high-ratio, fast compression format created by
// Yann Collet at Facebook (2015). It combines:
//
//   - LZ77 back-references (via LZSS token generation) to exploit repetition
//     in the data — the same "copy from earlier in the output" trick as
//     DEFLATE, but with a 32 KB window.
//
//   - FSE (Finite State Entropy) coding instead of Huffman for the sequence
//     descriptor symbols. FSE is an asymmetric numeral system that approaches
//     the Shannon entropy limit in a single pass.
//
//   - Predefined decode tables (RFC 8878 Appendix B) so short frames need no
//     table description overhead.
//
// Frame layout (RFC 8878 §3):
//
//   ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
//   │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
//   │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
//   └────────┴─────┴──────────────────────┴────────┴──────────────────┘
//
// Each block has a 3-byte header:
//   bit 0        = Last_Block flag
//   bits [2:1]   = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
//   bits [23:3]  = Block_Size
//
// Compression strategy (this implementation):
//   1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
//   2. For each block, try:
//      a. RLE — all bytes identical → 4 bytes total.
//      b. Compressed (LZ77 + FSE) — if output < input length.
//      c. Raw — verbatim copy as fallback.
//
// Series:
//   CMP00 (LZ77)    — Sliding-window back-references
//   CMP01 (LZ78)    — Explicit dictionary (trie)
//   CMP02 (LZSS)    — LZ77 + flag bits
//   CMP03 (LZW)     — LZ78 + pre-initialised alphabet; GIF
//   CMP04 (Huffman) — Entropy coding
//   CMP05 (DEFLATE) — LZ77 + Huffman; ZIP/gzip/PNG/zlib
//   CMP06 (Brotli)  — DEFLATE + context modelling + static dict
//   CMP07 (ZStd)    — LZ77 + FSE; high ratio + speed  ← this package

import com.codingadventures.lzss.Lzss;
import com.codingadventures.lzss.LzssToken;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * Pure-Java ZStd compression and decompression (RFC 8878 / CMP07).
 *
 * <h2>Quick example</h2>
 *
 * <pre>{@code
 * byte[] data = "the quick brown fox jumps over the lazy dog".getBytes();
 * byte[] compressed = Zstd.compress(data);
 * byte[] restored   = Zstd.decompress(compressed);
 * assert Arrays.equals(data, restored);
 * }</pre>
 */
public final class Zstd {

    // ─── Constants ────────────────────────────────────────────────────────────

    /**
     * ZStd magic number: {@code 0xFD2FB528} (little-endian bytes: 28 B5 2F FD).
     *
     * <p>Every valid ZStd frame starts with these 4 bytes. The value was chosen
     * to be unlikely to appear at the start of plaintext files.</p>
     */
    private static final int MAGIC = 0xFD2FB528;

    /**
     * Maximum block size: 128 KB.
     *
     * <p>ZStd allows blocks up to 128 KB. Larger inputs are split across
     * multiple blocks. The spec maximum is actually {@code min(WindowSize, 128 KB)}.</p>
     */
    private static final int MAX_BLOCK_SIZE = 128 * 1024;

    // ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────
    //
    // These tables map a *code number* to a (baseline, extra_bits) pair.
    //
    // For example, LL code 17 means literal_length = 18 + read(1 extra bit),
    // so it covers literal lengths 18 and 19.
    //
    // The FSE state machine tracks one code number per field; extra bits are
    // read directly from the bitstream after state transitions.

    /**
     * Literal Length code table: {@code [baseline, extra_bits]} for codes 0–35.
     *
     * <p>Literal length 0–15 each have their own code (0 extra bits).
     * Larger lengths are grouped with increasing ranges.</p>
     *
     * <p>Layout: {@code LL_CODES[i][0]} = baseline, {@code LL_CODES[i][1]} = extra bits.</p>
     */
    static final int[][] LL_CODES = {
        // code: value = baseline + read(extra_bits)
        {0, 0},  {1, 0},  {2, 0},  {3, 0},  {4, 0},  {5, 0},
        {6, 0},  {7, 0},  {8, 0},  {9, 0},  {10, 0}, {11, 0},
        {12, 0}, {13, 0}, {14, 0}, {15, 0},
        // Grouped ranges start at code 16
        {16, 1}, {18, 1}, {20, 1}, {22, 1},
        {24, 2}, {28, 2},
        {32, 3}, {40, 3},
        {48, 4}, {64, 6},
        {128, 7}, {256, 8}, {512, 9}, {1024, 10}, {2048, 11}, {4096, 12},
        {8192, 13}, {16384, 14}, {32768, 15}, {65536, 16},
    };

    /**
     * Match Length code table: {@code [baseline, extra_bits]} for codes 0–52.
     *
     * <p>Minimum match length in ZStd is 3 (not 0). Code 0 = match length 3.</p>
     */
    static final int[][] ML_CODES = {
        // codes 0..31: individual values 3..34
        {3, 0},  {4, 0},  {5, 0},  {6, 0},  {7, 0},  {8, 0},
        {9, 0},  {10, 0}, {11, 0}, {12, 0}, {13, 0}, {14, 0},
        {15, 0}, {16, 0}, {17, 0}, {18, 0}, {19, 0}, {20, 0},
        {21, 0}, {22, 0}, {23, 0}, {24, 0}, {25, 0}, {26, 0},
        {27, 0}, {28, 0}, {29, 0}, {30, 0}, {31, 0}, {32, 0},
        {33, 0}, {34, 0},
        // codes 32+: grouped ranges
        {35, 1}, {37, 1},  {39, 1},  {41, 1},
        {43, 2}, {47, 2},
        {51, 3}, {59, 3},
        {67, 4}, {83, 4},
        {99, 5}, {131, 7},
        {259, 8}, {515, 9}, {1027, 10}, {2051, 11},
        {4099, 12}, {8195, 13}, {16387, 14}, {32771, 15}, {65539, 16},
    };

    // ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────
    //
    // "Predefined_Mode" means no per-frame table description is transmitted.
    // The decoder builds the same table from these fixed distributions.
    //
    // Entries of -1 mean "probability 1/table_size" — these symbols get one
    // slot in the decode table and their encoder state never needs extra bits.

    /**
     * Predefined normalised distribution for Literal Length FSE.
     *
     * <p>Table accuracy log = 6 → 64 slots.</p>
     */
    static final short[] LL_NORM = {
         4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
         2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
        -1, -1, -1, -1,
    };
    static final int LL_ACC_LOG = 6; // table_size = 64

    /**
     * Predefined normalised distribution for Match Length FSE.
     *
     * <p>Table accuracy log = 6 → 64 slots.</p>
     */
    static final short[] ML_NORM = {
         1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
         1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
         1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
        -1, -1, -1, -1, -1,
    };
    static final int ML_ACC_LOG = 6;

    /**
     * Predefined normalised distribution for Offset FSE.
     *
     * <p>Table accuracy log = 5 → 32 slots.</p>
     */
    static final short[] OF_NORM = {
         1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
         1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
    };
    static final int OF_ACC_LOG = 5; // table_size = 32

    // Private constructor — static utility class.
    private Zstd() {}

    // ─── FSE decode table entry ───────────────────────────────────────────────

    /**
     * One cell in the FSE decode table.
     *
     * <p>To decode a symbol from state S:</p>
     * <ol>
     *   <li>{@code sym} is the output symbol.</li>
     *   <li>Read {@code nb} bits from the bitstream as {@code bits}.</li>
     *   <li>New state = {@code base + bits}.</li>
     * </ol>
     */
    static final class FseDe {
        byte sym;    // decoded symbol
        byte nb;     // number of extra bits to read for next state
        int base;    // base value for next state computation (unsigned short range)

        FseDe() {}
    }

    /**
     * Encode transform for one symbol.
     *
     * <p>Given encoder state S for symbol {@code s}:</p>
     * <pre>
     *   nb_out = (S + deltaNb) >>> 16   (number of bits to emit)
     *   emit low nb_out bits of S
     *   new_S  = stateTbl[(S >>> nb_out) + deltaFs]
     * </pre>
     *
     * <p>{@code deltaNb} and {@code deltaFs} are precomputed from the distribution
     * so the hot-path encode loop needs only arithmetic and a table lookup.</p>
     */
    static final class FseEe {
        /**
         * {@code (max_bits_out << 16) - (count << max_bits_out)}.
         *
         * <p>Used to derive nb_out: {@code nb_out = (state + deltaNb) >>> 16}</p>
         */
        long deltaNb; // treated as unsigned 32-bit but stored as long

        /**
         * {@code cumulative_count_before_sym - count} (may be negative).
         *
         * <p>Used to index stateTbl: {@code new_S = stateTbl[(S >>> nb_out) + deltaFs]}</p>
         */
        int deltaFs;

        FseEe() {}
    }

    // ─── FSE table construction ───────────────────────────────────────────────

    /**
     * Build an FSE decode table from a normalised probability distribution.
     *
     * <p>The algorithm:</p>
     * <ol>
     *   <li>Place symbols with probability -1 (very rare) at the top of the table.</li>
     *   <li>Spread remaining symbols using a deterministic step function derived
     *       from the table size. This ensures each symbol occupies the correct
     *       fraction of slots.</li>
     *   <li>Assign {@code nb} (number of state bits to read) and {@code base}
     *       to each slot so that the decoder can reconstruct the next state.</li>
     * </ol>
     *
     * <p>The step function {@code step = (sz >> 1) + (sz >> 3) + 3} is co-prime
     * to {@code sz} when {@code sz} is a power of two (which it always is in
     * ZStd), ensuring that the walk visits every slot exactly once.</p>
     *
     * @param norm   normalised distribution; -1 entries are "probability 1/sz"
     * @param accLog accuracy log; table size = {@code 1 << accLog}
     * @return the FSE decode table
     */
    static FseDe[] buildDecodeTable(short[] norm, int accLog) {
        int sz = 1 << accLog;
        int step = (sz >> 1) + (sz >> 3) + 3;
        FseDe[] tbl = new FseDe[sz];
        for (int i = 0; i < sz; i++) tbl[i] = new FseDe();
        int[] symNext = new int[norm.length];

        // Phase 1: symbols with probability -1 go at the top (high indices).
        // These symbols each get exactly 1 slot, and their state transition uses
        // the full accLog bits (they can go to any state).
        int high = sz - 1;
        for (int s = 0; s < norm.length; s++) {
            if (norm[s] == -1) {
                tbl[high].sym = (byte) s;
                if (high > 0) high--;
                symNext[s] = 1;
            }
        }

        // Phase 2: spread remaining symbols into the lower portion of the table.
        // Two-pass approach: first symbols with count > 1, then count == 1.
        // This matches the reference implementation's deterministic ordering.
        int pos = 0;
        for (int pass = 0; pass < 2; pass++) {
            for (int s = 0; s < norm.length; s++) {
                if (norm[s] <= 0) continue;
                int cnt = norm[s];
                if ((pass == 0) != (cnt > 1)) continue;
                symNext[s] = cnt;
                for (int k = 0; k < cnt; k++) {
                    tbl[pos].sym = (byte) s;
                    pos = (pos + step) & (sz - 1);
                    while (pos > high) {
                        pos = (pos + step) & (sz - 1);
                    }
                }
            }
        }

        // Phase 3: assign nb (number of state bits to read) and base.
        //
        // For a symbol with count cnt occupying slots i₀, i₁, ...:
        //   The next_state counter starts at cnt and increments.
        //   nb = accLog - floor(log2(next_state))
        //   base = next_state * (1 << nb) - sz
        //
        // This ensures that when we reconstruct state = base + read(nb bits),
        // we land in the range [sz, 2*sz), which is the valid encoder state range.
        int[] sn = symNext.clone();
        for (int i = 0; i < sz; i++) {
            int s = tbl[i].sym & 0xFF;
            int ns = sn[s];
            sn[s]++;
            // floor(log2(ns)) = 31 - Integer.numberOfLeadingZeros(ns)
            int nb = accLog - (31 - Integer.numberOfLeadingZeros(ns));
            // base = ns * (1 << nb) - sz
            int base = (ns << nb) - sz;
            tbl[i].nb = (byte) nb;
            tbl[i].base = base;
        }

        return tbl;
    }

    /**
     * Build FSE encode tables from a normalised distribution.
     *
     * <p>Returns a pair: {@code [0]} = {@code FseEe[]} per symbol,
     * {@code [1]} = {@code int[]} state table (slot → output state in
     * {@code [sz, 2*sz)}).</p>
     *
     * <h3>The encode/decode symmetry</h3>
     *
     * <p>The FSE decoder assigns {@code (sym, nb, base)} to each table cell in
     * INDEX ORDER. For symbol {@code s}, the j-th cell (in ascending index order)
     * has:</p>
     * <pre>
     *   ns = count[s] + j
     *   nb = accLog - floor(log2(ns))
     *   base = ns * (1 &lt;&lt; nb) - sz
     * </pre>
     *
     * <p>The FSE encoder must use the SAME indexing: slot {@code cumul[s]+j}
     * maps to the j-th table cell for symbol {@code s} (in ascending index
     * order).</p>
     *
     * @param norm   normalised distribution
     * @param accLog accuracy log
     * @return Object array: index 0 = FseEe[], index 1 = int[] state table
     */
    static Object[] buildEncodeTable(short[] norm, int accLog) {
        int sz = 1 << accLog;

        // Step 1: compute cumulative sums.
        int[] cumul = new int[norm.length];
        int total = 0;
        for (int s = 0; s < norm.length; s++) {
            cumul[s] = total;
            int cnt = norm[s] == -1 ? 1 : Math.max(0, norm[s]);
            total += cnt;
        }

        // Step 2: build the spread table (which symbol occupies each table slot).
        //
        // This uses the same spreading algorithm as buildDecodeTable, producing
        // a mapping from table index to symbol.
        int step = (sz >> 1) + (sz >> 3) + 3;
        int[] spread = new int[sz]; // spread[index] = symbol
        int idxHigh = sz - 1;

        // Phase 1: probability -1 symbols at the high end
        for (int s = 0; s < norm.length; s++) {
            if (norm[s] == -1) {
                spread[idxHigh] = s;
                if (idxHigh > 0) idxHigh--;
            }
        }
        int idxLimit = idxHigh;

        // Phase 2: spread remaining symbols using the step function
        int pos2 = 0;
        for (int pass = 0; pass < 2; pass++) {
            for (int s = 0; s < norm.length; s++) {
                if (norm[s] <= 0) continue;
                int cnt = norm[s];
                if ((pass == 0) != (cnt > 1)) continue;
                for (int k = 0; k < cnt; k++) {
                    spread[pos2] = s;
                    pos2 = (pos2 + step) & (sz - 1);
                    while (pos2 > idxLimit) {
                        pos2 = (pos2 + step) & (sz - 1);
                    }
                }
            }
        }

        // Step 3: build the state table by iterating spread in INDEX ORDER.
        //
        // For each table index i (in ascending order), determine which
        // occurrence of symbol s = spread[i] this is (j = 0, 1, 2, ...).
        // The encode slot is cumul[s] + j, and the encoder output state is
        // i + sz (so the decoder, in state i, will decode symbol s).
        //
        // We use symOcc[s] to count how many times symbol s has appeared so
        // far (in index order), so j = symOcc[s] when we see it at index i.
        int[] symOcc = new int[norm.length];
        int[] st = new int[sz]; // encoded as unsigned shorts but stored as int

        for (int i = 0; i < sz; i++) {
            int s = spread[i];
            int j = symOcc[s];
            symOcc[s]++;
            int slot = cumul[s] + j;
            st[slot] = i + sz;
        }

        // Step 4: build FseEe entries.
        //
        // For symbol s with count c and max_bits_out mbo:
        //   deltaNb = (mbo << 16) - (c << mbo)
        //   deltaFs = cumul[s] - c
        //
        // Encode step: given current encoder state E ∈ [sz, 2*sz):
        //   nb = (E + deltaNb) >>> 16     (number of state bits to emit)
        //   emit low nb bits of E
        //   new_E = st[(E >>> nb) + deltaFs]
        FseEe[] ee = new FseEe[norm.length];
        for (int s = 0; s < norm.length; s++) {
            ee[s] = new FseEe();
            int cnt = norm[s] == -1 ? 1 : Math.max(0, norm[s]);
            if (cnt == 0) continue;
            int mbo;
            if (cnt == 1) {
                mbo = accLog;
            } else {
                // max_bits_out = accLog - floor(log2(cnt))
                mbo = accLog - (31 - Integer.numberOfLeadingZeros(cnt));
            }
            // Use long arithmetic to avoid overflow: (mbo << 16) can be up to 6*65536 = 393216
            long deltaNb = ((long) mbo << 16) - ((long) cnt << mbo);
            ee[s].deltaNb = deltaNb;
            ee[s].deltaFs = cumul[s] - cnt;
        }

        return new Object[]{ee, st};
    }

    // ─── Reverse bit-writer ───────────────────────────────────────────────────
    //
    // ZStd's sequence bitstream is written *backwards* relative to the data
    // flow: the encoder writes bits that the decoder will read last, first.
    // This allows the decoder to read a forward-only stream while decoding
    // sequences in order.
    //
    // Byte layout: [byte0, byte1, ..., byteN] where byteN is the last byte
    // written, and it contains a sentinel bit (the highest set bit) that marks
    // the end of meaningful data. The decoder initialises by finding this
    // sentinel.
    //
    // Bit layout within each byte: LSB = first bit written.
    //
    // Example: write bits 1, 0, 1, 1 (4 bits) then flush:
    //   reg = 0b1011, bits = 4
    //   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
    //   buf = [0x1B]
    //
    // The decoder reads this as: find MSB (bit 4 = sentinel), then read
    // bits 3..0 = 0b1011 = the original 4 bits.

    /**
     * Backward bit-stream writer.
     *
     * <p>Accumulates bits LSB-first into a 64-bit register, flushing whole
     * bytes as they fill. The final {@link #flush()} call appends a sentinel
     * bit above the remaining data bits so the decoder can locate the stream
     * end.</p>
     */
    static final class RevBitWriter {
        private final List<Byte> buf = new ArrayList<>();
        private long reg;  // accumulation register (bits fill from LSB)
        private int bits;  // number of valid bits in reg

        /**
         * Add the low-order {@code nb} bits of {@code val} to the stream.
         *
         * @param val source value; only the lowest {@code nb} bits are used
         * @param nb  number of bits to write (0–64)
         */
        void addBits(long val, int nb) {
            if (nb == 0) return;
            long mask = (nb == 64) ? -1L : ((1L << nb) - 1L);
            reg |= (val & mask) << bits;
            bits += nb;
            while (bits >= 8) {
                buf.add((byte) (reg & 0xFF));
                reg >>>= 8;
                bits -= 8;
            }
        }

        /**
         * Flush remaining bits with a sentinel and mark the stream end.
         *
         * <p>The sentinel is a {@code 1} bit placed at position {@code bits} in
         * the last byte. The decoder locates it with leading-zeros arithmetic.</p>
         */
        void flush() {
            int sentinel = 1 << bits; // bit above all remaining data bits
            byte lastByte = (byte) ((reg & 0xFF) | sentinel);
            buf.add(lastByte);
            reg = 0;
            bits = 0;
        }

        /**
         * Return the accumulated bytes as a primitive array.
         *
         * @return the raw byte array containing the encoded bit stream
         */
        byte[] finish() {
            byte[] out = new byte[buf.size()];
            for (int i = 0; i < out.length; i++) out[i] = buf.get(i);
            return out;
        }
    }

    // ─── Reverse bit-reader ───────────────────────────────────────────────────
    //
    // Mirrors RevBitWriter: reads bits from the END of the buffer going
    // backwards. The stream is laid out so that the LAST bits written by the
    // encoder are at the END of the byte buffer (in the sentinel-containing
    // last byte). The reader initialises at the last byte and reads backward
    // toward byte 0.
    //
    // Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
    // readBits(n) extracts the top n bits and shifts the register left by n.
    //
    // Why left-aligned? The writer accumulates bits LSB-first. Within each
    // flushed byte, bit 0 = earliest written, bit N = latest written. To read
    // the LATEST bits first (which were in the highest byte positions and in
    // the high bits of each byte), we need a left-aligned register so that
    // reading from the top gives the highest-position bits first.

    /**
     * Backward bit-stream reader.
     *
     * <p>Reads from the end of the byte array going backward toward byte 0.
     * Valid bits are left-aligned in a 64-bit register; {@link #readBits(int)}
     * extracts from the top and shifts left.</p>
     */
    static final class RevBitReader {
        private final byte[] data;
        private long reg;   // shift register, valid bits packed at the TOP (MSB side)
        private int bits;   // how many valid bits are loaded (count from MSB)
        private int pos;    // index of the next byte to load (decrements toward 0)

        /**
         * Initialise the reader from a complete backward bit stream.
         *
         * <p>The last byte contains a sentinel: the highest set bit marks the
         * end of meaningful data; bits below it are the most recently written
         * data bits.</p>
         *
         * @param data the encoded bit stream (produced by {@link RevBitWriter})
         * @throws IOException if the stream is empty or the sentinel byte is zero
         */
        RevBitReader(byte[] data) throws IOException {
            if (data.length == 0)
                throw new IOException("empty bitstream");

            // Find the sentinel bit in the last byte.
            // The sentinel is the highest set bit; valid data bits are below it.
            int last = data[data.length - 1] & 0xFF;
            if (last == 0)
                throw new IOException("bitstream last byte is zero (no sentinel)");

            // sentinelPos = bit index (0 = LSB) of the sentinel in the last byte.
            // validBits = number of data bits below the sentinel.
            int sentinelPos = 0;
            for (int b = 7; b >= 0; b--) {
                if ((last & (1 << b)) != 0) {
                    sentinelPos = b;
                    break;
                }
            }
            int validBits = sentinelPos;

            // Place the valid bits of the sentinel byte at the TOP of the register.
            // Example: last=0b00011110, sentinel at bit4, validBits=4,
            //   data bits = last & 0b1111 = 0b1110.
            //   After shifting to top: reg bit63=1, bit62=1, bit61=1, bit60=0.
            long mask = (validBits == 0) ? 0L : ((1L << validBits) - 1L);
            long r = (validBits == 0) ? 0L : ((long)(last & mask)) << (64 - validBits);

            this.data = data;
            this.reg = r;
            this.bits = validBits;
            this.pos = data.length - 1; // sentinel byte already consumed; load from here-1

            // Fill the register from earlier bytes.
            reload();
        }

        /**
         * Load more bytes into the register from the stream going backward.
         *
         * <p>Each new byte is placed just BELOW the currently loaded bits (in
         * the left-aligned register, that means at position
         * {@code 64 - bits - 8}).</p>
         */
        private void reload() {
            while (bits <= 56 && pos > 0) {
                pos--;
                int shift = 64 - bits - 8;
                reg |= ((long)(data[pos] & 0xFF)) << shift;
                bits += 8;
            }
        }

        /**
         * Read {@code nb} bits from the top of the register (returns 0 if nb == 0).
         *
         * <p>This returns the most recently written bits first (highest stream
         * positions first), mirroring the encoder's backward order.</p>
         *
         * @param nb number of bits to read (0–64)
         * @return the extracted value
         */
        long readBits(int nb) {
            if (nb == 0) return 0L;
            long val = reg >>> (64 - nb);
            reg = (nb == 64) ? 0L : (reg << nb);
            bits = Math.max(0, bits - nb);
            if (bits < 24) reload();
            return val;
        }
    }

    // ─── FSE encode/decode helpers ────────────────────────────────────────────

    /**
     * Encode one symbol into the backward bitstream, updating the FSE state.
     *
     * <p>The encoder maintains state in {@code [sz, 2*sz)}. To emit symbol
     * {@code sym}:</p>
     * <ol>
     *   <li>Compute how many bits to flush:
     *       {@code nb = (state + deltaNb) >>> 16}</li>
     *   <li>Write the low {@code nb} bits of {@code state} to the bitstream.</li>
     *   <li>New state = {@code st[(state >>> nb) + deltaFs]}</li>
     * </ol>
     *
     * <p>After all symbols are encoded, the final state (minus {@code sz}) is
     * written as {@code accLog} bits to allow the decoder to initialise.</p>
     *
     * @param state a single-element array holding the current encoder state
     *              (used as an in-out parameter since Java lacks ref params)
     * @param sym   the symbol to encode
     * @param ee    the encode transform array, indexed by symbol
     * @param st    the encoder state table
     * @param bw    the backward bit-stream writer
     */
    static void fseEncodeSym(long[] state, int sym, FseEe[] ee, int[] st, RevBitWriter bw) {
        FseEe e = ee[sym];
        // state is in [sz, 2*sz), so it fits in a long safely.
        long s = state[0];
        // Compute number of state bits to emit.
        // deltaNb may be negative (stored as long), so add carefully.
        long nb = (s + e.deltaNb) >>> 16;
        bw.addBits(s, (int) nb);
        int slotI = (int)(s >>> (int) nb) + e.deltaFs;
        int slot = Math.max(0, slotI);
        state[0] = st[slot] & 0xFFFFL; // unsigned short
    }

    /**
     * Decode one symbol from the backward bitstream, updating the FSE state.
     *
     * <ol>
     *   <li>Look up {@code de[state]} to get {@code sym}, {@code nb}, and
     *       {@code base}.</li>
     *   <li>New state = {@code base + read(nb bits)}.</li>
     * </ol>
     *
     * @param state a single-element array holding the current decoder state
     * @param de    the decode table
     * @param br    the backward bit-stream reader
     * @return the decoded symbol
     */
    static int fseDecodeSym(int[] state, FseDe[] de, RevBitReader br) {
        FseDe e = de[state[0]];
        int sym = e.sym & 0xFF;
        state[0] = e.base + (int) br.readBits(e.nb & 0xFF);
        return sym;
    }

    // ─── LL/ML/OF code number computation ────────────────────────────────────

    /**
     * Map a literal length value to its LL code number (0–35).
     *
     * <p>Codes 0–15 are identity; codes 16+ cover ranges via lookup.
     * We scan linearly because the table is only 36 entries long.</p>
     *
     * @param ll the literal length
     * @return the LL code number
     */
    static int llToCode(int ll) {
        // Linear scan over LL_CODES. The last code whose baseline ≤ ll is correct.
        int code = 0;
        for (int i = 0; i < LL_CODES.length; i++) {
            if (LL_CODES[i][0] <= ll) code = i;
            else break;
        }
        return code;
    }

    /**
     * Map a match length value to its ML code number (0–52).
     *
     * @param ml the match length
     * @return the ML code number
     */
    static int mlToCode(int ml) {
        int code = 0;
        for (int i = 0; i < ML_CODES.length; i++) {
            if (ML_CODES[i][0] <= ml) code = i;
            else break;
        }
        return code;
    }

    // ─── Sequence record ──────────────────────────────────────────────────────

    /**
     * One ZStd sequence: (literal_length, match_length, match_offset).
     *
     * <p>A sequence means: emit {@code ll} literal bytes from the literals
     * section, then copy {@code ml} bytes starting {@code off} positions back
     * in the output buffer. After all sequences, any remaining literals are
     * appended.</p>
     */
    private record Seq(int ll, int ml, int off) {}

    // ─── Token → sequence conversion ─────────────────────────────────────────

    /**
     * Convert LZSS tokens into ZStd sequences + a flat literals buffer.
     *
     * <p>LZSS produces a stream of {@code Literal(byte)} and
     * {@code Match{offset, length}}. ZStd groups consecutive literals before
     * each match into a single sequence. Any trailing literals (after the last
     * match) go into the literals buffer without a corresponding sequence
     * entry.</p>
     *
     * @param tokens the LZSS token list
     * @return an Object array: index 0 = {@code byte[]} lits,
     *         index 1 = {@code List<Seq>} seqs
     */
    private static Object[] tokensToSeqs(List<LzssToken> tokens) {
        List<Byte> litList = new ArrayList<>();
        List<Seq> seqs = new ArrayList<>();
        int litRun = 0;

        for (LzssToken tok : tokens) {
            switch (tok) {
                case LzssToken.Literal lit -> {
                    litList.add(lit.value());
                    litRun++;
                }
                case LzssToken.Match match -> {
                    seqs.add(new Seq(litRun, match.length(), match.offset()));
                    litRun = 0;
                }
            }
        }

        // Trailing literals stay in lits; no sequence for them.
        byte[] lits = new byte[litList.size()];
        for (int i = 0; i < lits.length; i++) lits[i] = litList.get(i);

        return new Object[]{lits, seqs};
    }

    // ─── Literals section encoding ────────────────────────────────────────────
    //
    // ZStd literals can be Huffman-coded or raw. We use Raw_Literals (type=0),
    // which is the simplest: no Huffman table, bytes are stored verbatim.
    //
    // Header format depends on literal count:
    //   ≤ 31 bytes:   1-byte header  = (lit_len << 3) | 0b000
    //   ≤ 4095 bytes: 2-byte header  = (lit_len << 4) | 0b0100
    //   else:         3-byte header  = (lit_len << 4) | 0b1100
    //
    // The bottom 2 bits = Literals_Block_Type (0 = Raw).
    // The next 2 bits = Size_Format.

    /**
     * Encode literals as a Raw_Literals section (RFC 8878 §3.1.1.2.1).
     *
     * <p>Header encoding:</p>
     * <ul>
     *   <li>bits [1:0] = Literals_Block_Type = 00 (Raw)</li>
     *   <li>bits [3:2] = Size_Format: 00 or 10 = 1-byte, 01 = 2-byte, 11 = 3-byte</li>
     * </ul>
     *
     * @param lits the raw literal bytes
     * @return the encoded literals section (header + data)
     */
    private static byte[] encodeLiteralsSection(byte[] lits) {
        int n = lits.length;
        List<Byte> out = new ArrayList<>(n + 3);

        if (n <= 31) {
            // 1-byte header: size_format=00, type=00
            out.add((byte) (n << 3));
        } else if (n <= 4095) {
            // 2-byte header: size_format=01, type=00 → 0b0100
            int hdr = (n << 4) | 0b0100;
            out.add((byte) (hdr & 0xFF));
            out.add((byte) ((hdr >> 8) & 0xFF));
        } else {
            // 3-byte header: size_format=11, type=00 → 0b1100
            int hdr = (n << 4) | 0b1100;
            out.add((byte) (hdr & 0xFF));
            out.add((byte) ((hdr >> 8) & 0xFF));
            out.add((byte) ((hdr >> 16) & 0xFF));
        }

        for (byte b : lits) out.add(b);

        byte[] result = new byte[out.size()];
        for (int i = 0; i < result.length; i++) result[i] = out.get(i);
        return result;
    }

    /**
     * Decode a Raw_Literals section.
     *
     * <p>Size format decoding (RFC 8878 §3.1.1.2.1):</p>
     * <ul>
     *   <li>0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0–31)</li>
     *   <li>0b01 → 2-byte LE header: size in bits [11:4] (12 bits, values 0–4095)</li>
     *   <li>0b11 → 3-byte LE header: size in bits [19:4] (20 bits, values 0–1MB)</li>
     * </ul>
     *
     * @param data  the compressed block data
     * @param start offset into {@code data} where the literals section starts
     * @return int[2] where [0] = litsStart index, [1] = totalConsumed bytes;
     *         the caller slices {@code data[litsStart .. litsStart + n]}
     * @throws IOException if the section is truncated or has an unsupported type
     */
    private static int[] decodeLiteralsSectionHeader(byte[] data, int start) throws IOException {
        if (start >= data.length)
            throw new IOException("empty literals section");

        int b0 = data[start] & 0xFF;
        int ltype = b0 & 0b11; // bottom 2 bits = Literals_Block_Type

        if (ltype != 0)
            throw new IOException("unsupported literals type " + ltype + " (only Raw=0 supported)");

        int sizeFormat = (b0 >> 2) & 0b11;
        int n;
        int headerBytes;
        switch (sizeFormat) {
            case 0, 2 -> {
                // 1-byte header: size in bits [7:3] (5 bits = values 0..31)
                n = b0 >> 3;
                headerBytes = 1;
            }
            case 1 -> {
                // 2-byte header: 12-bit size
                if (start + 2 > data.length)
                    throw new IOException("truncated literals header (2-byte)");
                n = ((b0 >> 4) & 0xF) | ((data[start + 1] & 0xFF) << 4);
                headerBytes = 2;
            }
            case 3 -> {
                // 3-byte header: 20-bit size (enough for blocks up to 1 MB)
                if (start + 3 > data.length)
                    throw new IOException("truncated literals header (3-byte)");
                n = ((b0 >> 4) & 0xF)
                        | ((data[start + 1] & 0xFF) << 4)
                        | ((data[start + 2] & 0xFF) << 12);
                headerBytes = 3;
            }
            default -> throw new IOException("impossible size_format");
        }

        int dataStart = start + headerBytes;
        int dataEnd = dataStart + n;
        if (dataEnd > data.length)
            throw new IOException("literals data truncated: need " + dataEnd +
                    ", have " + data.length);

        // Return [dataStart, dataEnd] so caller can slice
        return new int[]{dataStart, dataEnd};
    }

    // ─── Sequences section encoding ───────────────────────────────────────────
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

    /**
     * Encode the Number_of_Sequences field per RFC 8878 §3.1.1.1.2.
     *
     * <p>Encoding (mirrors the Rust reference):</p>
     * <ul>
     *   <li>0       : 1 byte = 0x00</li>
     *   <li>1–127   : 1 byte = count</li>
     *   <li>128–32766 : 2 bytes LE; the LE u16 = count | 0x8000</li>
     *   <li>32767+  : 3 bytes; byte0 = 0xFF, then (count - 0x7F00) as LE u16</li>
     * </ul>
     *
     * <p>The 2-byte form uses {@code byte0 = (count | 0x8000) & 0xFF} and
     * {@code byte1 = (count | 0x8000) >> 8}.  Because count is in [128, 32766],
     * byte0 is in [0x80, 0xFE], safely distinguishable from the 3-byte marker
     * 0xFF and from the 1-byte range [0x00, 0x7F].</p>
     *
     * @param count the sequence count
     * @return the encoded bytes
     */
    static byte[] encodeSeqCount(int count) {
        if (count < 128) {
            return new byte[]{(byte) count};
        }
        if (count < 0x7FFF) { // 128..32766 inclusive
            // Pack as a LE u16 with the high bit set.
            int v = count | 0x8000;
            return new byte[]{(byte) (v & 0xFF), (byte) ((v >> 8) & 0xFF)};
        }
        // 32767+ : 3-byte encoding; byte0=0xFF, next two = (count - 0x7F00) LE u16
        int r = count - 0x7F00;
        return new byte[]{(byte) 0xFF, (byte) (r & 0xFF), (byte) ((r >> 8) & 0xFF)};
    }

    /**
     * Decode the Number_of_Sequences field.
     *
     * @param data the block data
     * @param pos  start offset
     * @return int[2]: [0] = count, [1] = bytes consumed
     * @throws IOException if the field is truncated
     */
    static int[] decodeSeqCount(byte[] data, int pos) throws IOException {
        if (pos >= data.length)
            throw new IOException("empty sequence count");
        int b0 = data[pos] & 0xFF;
        if (b0 < 128) {
            // 1-byte: count = b0
            return new int[]{b0, 1};
        }
        if (b0 < 0xFF) {
            // 2-byte LE u16: count = (u16 value) & 0x7FFF
            if (pos + 2 > data.length)
                throw new IOException("truncated sequence count");
            int v = (b0 & 0xFF) | ((data[pos + 1] & 0xFF) << 8);
            return new int[]{v & 0x7FFF, 2};
        }
        // 3-byte encoding: byte0=0xFF, then (count - 0x7F00) as LE u16
        if (pos + 3 > data.length)
            throw new IOException("truncated sequence count (3-byte)");
        int count = 0x7F00 + (data[pos + 1] & 0xFF) + ((data[pos + 2] & 0xFF) << 8);
        return new int[]{count, 3};
    }

    /**
     * Encode the sequences section using predefined FSE tables.
     *
     * <p>This builds the three predefined FSE encode tables (LL, ML, OF), then
     * encodes all sequences in reverse order into a backward bit stream. The
     * final FSE states are written last (so the decoder, reading backward, sees
     * them first).</p>
     *
     * @param seqs the sequence list
     * @return the encoded FSE bitstream bytes
     */
    @SuppressWarnings("unchecked")
    static byte[] encodeSequencesSection(List<Seq> seqs) {
        // Build encode tables (precomputed from the predefined distributions).
        Object[] resLl = buildEncodeTable(LL_NORM, LL_ACC_LOG);
        Object[] resMl = buildEncodeTable(ML_NORM, ML_ACC_LOG);
        Object[] resOf = buildEncodeTable(OF_NORM, OF_ACC_LOG);

        FseEe[] eeLl = (FseEe[]) resLl[0]; int[] stLl = (int[]) resLl[1];
        FseEe[] eeMl = (FseEe[]) resMl[0]; int[] stMl = (int[]) resMl[1];
        FseEe[] eeOf = (FseEe[]) resOf[0]; int[] stOf = (int[]) resOf[1];

        long szLl = 1L << LL_ACC_LOG;
        long szMl = 1L << ML_ACC_LOG;
        long szOf = 1L << OF_ACC_LOG;

        // FSE encoder states start at table_size (= sz).
        // The state range [sz, 2*sz) maps to slot range [0, sz).
        long[] stateLl = {szLl};
        long[] stateMl = {szMl};
        long[] stateOf = {szOf};

        RevBitWriter bw = new RevBitWriter();

        // Encode sequences in reverse order.
        for (int si = seqs.size() - 1; si >= 0; si--) {
            Seq seq = seqs.get(si);
            int llCode = llToCode(seq.ll());
            int mlCode = mlToCode(seq.ml());

            // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
            // code = floor(log2(raw)); extra = raw - (1 << code)
            int rawOff = seq.off() + 3;
            int ofCode = (rawOff <= 1) ? 0 : (31 - Integer.numberOfLeadingZeros(rawOff));
            int ofExtra = rawOff - (1 << ofCode);

            // Write extra bits (OF, ML, LL in this order for backward stream).
            bw.addBits(ofExtra, ofCode);
            int mlExtra = seq.ml() - ML_CODES[mlCode][0];
            bw.addBits(mlExtra, ML_CODES[mlCode][1]);
            int llExtra = seq.ll() - LL_CODES[llCode][0];
            bw.addBits(llExtra, LL_CODES[llCode][1]);

            // FSE encode symbols in the order that the backward bitstream reverses
            // to match the decoder's read order (LL first, OF second, ML third).
            //
            // Since the backward stream reverses write order, we write the REVERSE
            // of the decode order: ML → OF → LL (LL is written last = at the top
            // of the bitstream = read first by the decoder).
            //
            // Decode order: LL, OF, ML
            // Encode order (reversed): ML, OF, LL
            fseEncodeSym(stateMl, mlCode, eeMl, stMl, bw);
            fseEncodeSym(stateOf, ofCode, eeOf, stOf, bw);
            fseEncodeSym(stateLl, llCode, eeLl, stLl, bw);
        }

        // Flush final states (low accLog bits of state - sz).
        bw.addBits(stateOf[0] - szOf, OF_ACC_LOG);
        bw.addBits(stateMl[0] - szMl, ML_ACC_LOG);
        bw.addBits(stateLl[0] - szLl, LL_ACC_LOG);
        bw.flush();

        return bw.finish();
    }

    // ─── Block-level compress ─────────────────────────────────────────────────

    /**
     * Compress one block into ZStd compressed block format.
     *
     * <p>Returns {@code null} if the compressed form is larger than the input
     * (in which case the caller should use a Raw block instead).</p>
     *
     * @param data   the full input array
     * @param offset start of the block within {@code data}
     * @param length number of bytes in the block
     * @return compressed block bytes, or {@code null} if not beneficial
     */
    private static byte[] compressBlock(byte[] data, int offset, int length) {
        // Extract the block slice.
        byte[] block = Arrays.copyOfRange(data, offset, offset + length);

        // Use LZSS to generate LZ77 tokens.
        // Window = 32 KB, max match = 255, min match = 3.
        List<LzssToken> tokens = Lzss.encode(block, 32768, 255, 3);

        // Convert tokens to ZStd sequences.
        Object[] sr = tokensToSeqs(tokens);
        byte[] lits = (byte[]) sr[0];
        @SuppressWarnings("unchecked")
        List<Seq> seqs = (List<Seq>) sr[1];

        // If no sequences were found, LZ77 had nothing to compress.
        // A compressed block with 0 sequences still has overhead, so fall back.
        if (seqs.isEmpty()) return null;

        List<Byte> result = new ArrayList<>();

        // Encode literals section (Raw_Literals).
        for (byte b : encodeLiteralsSection(lits)) result.add(b);

        // Encode sequences section.
        for (byte b : encodeSeqCount(seqs.size())) result.add(b);
        result.add((byte) 0x00); // Symbol_Compression_Modes = all Predefined

        byte[] bitstream = encodeSequencesSection(seqs);
        for (byte b : bitstream) result.add(b);

        if (result.size() >= length) return null; // Not beneficial

        byte[] out = new byte[result.size()];
        for (int i = 0; i < out.length; i++) out[i] = result.get(i);
        return out;
    }

    /**
     * Decompress one ZStd compressed block.
     *
     * <p>Reads the literals section, sequences section, and applies the
     * sequences to the output buffer to reconstruct the original data.</p>
     *
     * @param data       the full compressed data array
     * @param blockStart offset where the block payload begins
     * @param blockLen   length of the block payload
     * @param output     the growing output buffer
     * @throws IOException if the block is malformed
     */
    private static void decompressBlock(
            byte[] data, int blockStart, int blockLen, List<Byte> output)
            throws IOException {

        // ── Literals section ─────────────────────────────────────────────────
        int[] litRange = decodeLiteralsSectionHeader(data, blockStart);
        // litRange[0] = first byte of literals data
        // litRange[1] = one past the last byte
        int litDataStart = litRange[0];
        int litDataEnd = litRange[1];
        // Consume header + data = litDataEnd - blockStart bytes
        int pos = litDataEnd;
        int blockEnd = blockStart + blockLen;

        // ── Sequences count ──────────────────────────────────────────────────
        if (pos >= blockEnd) {
            // Block has only literals, no sequences.
            for (int i = litDataStart; i < litDataEnd; i++) output.add(data[i]);
            return;
        }

        int[] sc = decodeSeqCount(data, pos);
        int nSeqs = sc[0];
        pos += sc[1];

        if (nSeqs == 0) {
            // No sequences — all content is in literals.
            for (int i = litDataStart; i < litDataEnd; i++) output.add(data[i]);
            return;
        }

        // ── Symbol compression modes ─────────────────────────────────────────
        if (pos >= blockEnd)
            throw new IOException("missing symbol compression modes byte");
        int modesByte = data[pos] & 0xFF;
        pos++;

        // Check that all modes are Predefined (0).
        int llMode = (modesByte >> 6) & 3;
        int ofMode = (modesByte >> 4) & 3;
        int mlMode = (modesByte >> 2) & 3;
        if (llMode != 0 || ofMode != 0 || mlMode != 0)
            throw new IOException("unsupported FSE modes: LL=" + llMode +
                    " OF=" + ofMode + " ML=" + mlMode +
                    " (only Predefined=0 supported)");

        // ── FSE bitstream ────────────────────────────────────────────────────
        if (pos >= blockEnd)
            throw new IOException("missing FSE bitstream");
        byte[] bitstreamSlice = Arrays.copyOfRange(data, pos, blockEnd);
        RevBitReader br = new RevBitReader(bitstreamSlice);

        // Build decode tables from predefined distributions.
        FseDe[] dtLl = buildDecodeTable(LL_NORM, LL_ACC_LOG);
        FseDe[] dtMl = buildDecodeTable(ML_NORM, ML_ACC_LOG);
        FseDe[] dtOf = buildDecodeTable(OF_NORM, OF_ACC_LOG);

        // Initialise FSE states from the bitstream.
        // The encoder wrote: state_ll, state_ml, state_of (each as accLog bits).
        // The decoder reads them in the same order.
        int[] stateLl = {(int) br.readBits(LL_ACC_LOG)};
        int[] stateMl = {(int) br.readBits(ML_ACC_LOG)};
        int[] stateOf = {(int) br.readBits(OF_ACC_LOG)};

        // Track position in the literals buffer.
        int litPos = litDataStart;

        // Apply each sequence.
        for (int i = 0; i < nSeqs; i++) {
            // Decode symbols (state transitions) — order: LL, OF, ML.
            int llCode = fseDecodeSym(stateLl, dtLl, br);
            int ofCode = fseDecodeSym(stateOf, dtOf, br);
            int mlCode = fseDecodeSym(stateMl, dtMl, br);

            // Validate codes.
            if (llCode >= LL_CODES.length)
                throw new IOException("invalid LL code " + llCode);
            if (mlCode >= ML_CODES.length)
                throw new IOException("invalid ML code " + mlCode);

            int[] llInfo = LL_CODES[llCode];
            int[] mlInfo = ML_CODES[mlCode];

            int ll = llInfo[0] + (int) br.readBits(llInfo[1]);
            int ml = mlInfo[0] + (int) br.readBits(mlInfo[1]);

            // Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3
            int ofRaw = (1 << ofCode) | (int) br.readBits(ofCode);
            if (ofRaw < 3)
                throw new IOException("decoded offset underflow: of_raw=" + ofRaw);
            int matchOffset = ofRaw - 3;

            // Emit ll literal bytes from the literals buffer.
            int litEnd = litPos + ll;
            if (litEnd > litDataEnd)
                throw new IOException("literal run " + ll +
                        " overflows literals buffer (pos=" + (litPos - litDataStart) +
                        " len=" + (litDataEnd - litDataStart) + ")");
            for (int j = litPos; j < litEnd; j++) output.add(data[j]);
            litPos = litEnd;

            // Copy ml bytes from matchOffset back in the output buffer.
            // offset = 0 would be a back-reference to (output.size() - 0),
            // which is past the end. The minimum valid offset is 1.
            if (matchOffset == 0 || matchOffset > output.size())
                throw new IOException("bad match offset " + matchOffset +
                        " (output len " + output.size() + ")");
            int copyStart = output.size() - matchOffset;
            for (int j = 0; j < ml; j++) {
                output.add(output.get(copyStart + j));
            }
        }

        // Any remaining literals after the last sequence.
        for (int i = litPos; i < litDataEnd; i++) output.add(data[i]);
    }

    // ─── Public API ───────────────────────────────────────────────────────────

    /**
     * Compress {@code data} to ZStd format (RFC 8878).
     *
     * <p>The output is a valid ZStd frame that can be decompressed by the
     * {@code zstd} CLI tool or any conforming implementation.</p>
     *
     * <p>Compression strategy:</p>
     * <ol>
     *   <li>Split data into 128 KB blocks.</li>
     *   <li>For each block, try RLE (all same byte), then LZ77+FSE compressed,
     *       then raw verbatim as a fallback.</li>
     * </ol>
     *
     * <h3>Frame header layout</h3>
     * <pre>{@code
     * [0..3] Magic  = 0xFD2FB528 LE
     * [4]    FHD    = 0xE0 (FCS_flag=11→8B, Single_Segment=1)
     * [5..12] FCS   = uncompressed size as u64 LE
     * [13..] Blocks
     * }</pre>
     *
     * @param data the uncompressed input bytes (must not be null)
     * @return ZStd-compressed bytes
     */
    public static byte[] compress(byte[] data) {
        if (data == null) throw new NullPointerException("data must not be null");

        List<Byte> out = new ArrayList<>();

        // ── ZStd frame header ────────────────────────────────────────────────
        // Magic number (4 bytes LE).
        int magic = MAGIC;
        out.add((byte) (magic & 0xFF));
        out.add((byte) ((magic >> 8) & 0xFF));
        out.add((byte) ((magic >> 16) & 0xFF));
        out.add((byte) ((magic >> 24) & 0xFF));

        // Frame Header Descriptor (FHD):
        //   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
        //   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor follows)
        //   bit 4:   Content_Checksum_Flag = 0
        //   bit 3-2: reserved = 0
        //   bit 1-0: Dict_ID_Flag = 0
        // = 0b1110_0000 = 0xE0
        out.add((byte) 0xE0);

        // Frame_Content_Size (8 bytes LE) — the uncompressed size.
        long fcs = data.length;
        for (int i = 0; i < 8; i++) {
            out.add((byte) (fcs & 0xFF));
            fcs >>>= 8;
        }

        // ── Blocks ───────────────────────────────────────────────────────────
        // Handle the special case of completely empty input: emit one empty raw block.
        if (data.length == 0) {
            // Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
            out.add((byte) 0x01);
            out.add((byte) 0x00);
            out.add((byte) 0x00);
            return toByteArray(out);
        }

        int offset = 0;
        while (offset < data.length) {
            int end = Math.min(offset + MAX_BLOCK_SIZE, data.length);
            int blockLen = end - offset;
            boolean last = (end == data.length);

            // ── Try RLE block ─────────────────────────────────────────────
            // If all bytes in the block are identical, encode as RLE
            // (1 byte payload + 3-byte header = 4 bytes total).
            boolean allSame = true;
            byte firstByte = data[offset];
            for (int i = offset + 1; i < end && allSame; i++) {
                if (data[i] != firstByte) allSame = false;
            }

            if (allSame) {
                // RLE block header: type=01, size=blockLen, last=1/0
                int hdr = (blockLen << 3) | (0b01 << 1) | (last ? 1 : 0);
                out.add((byte) (hdr & 0xFF));
                out.add((byte) ((hdr >> 8) & 0xFF));
                out.add((byte) ((hdr >> 16) & 0xFF));
                out.add(firstByte);
            } else {
                // ── Try compressed block ──────────────────────────────────
                byte[] compressed = compressBlock(data, offset, blockLen);
                if (compressed != null) {
                    int hdr = (compressed.length << 3) | (0b10 << 1) | (last ? 1 : 0);
                    out.add((byte) (hdr & 0xFF));
                    out.add((byte) ((hdr >> 8) & 0xFF));
                    out.add((byte) ((hdr >> 16) & 0xFF));
                    for (byte b : compressed) out.add(b);
                } else {
                    // ── Raw block (fallback) ──────────────────────────────
                    int hdr = (blockLen << 3) | (0b00 << 1) | (last ? 1 : 0);
                    out.add((byte) (hdr & 0xFF));
                    out.add((byte) ((hdr >> 8) & 0xFF));
                    out.add((byte) ((hdr >> 16) & 0xFF));
                    for (int i = offset; i < end; i++) out.add(data[i]);
                }
            }

            offset = end;
        }

        return toByteArray(out);
    }

    /**
     * Decompress a ZStd frame, returning the original data.
     *
     * <p>Accepts any valid ZStd frame with:</p>
     * <ul>
     *   <li>Single-segment or multi-segment layout</li>
     *   <li>Raw, RLE, or Compressed blocks</li>
     *   <li>Predefined FSE modes (no per-frame table description)</li>
     * </ul>
     *
     * @param data ZStd-compressed bytes (must not be null)
     * @return the original uncompressed bytes
     * @throws IOException if the input is truncated, has a bad magic number,
     *                     or contains unsupported features (non-predefined FSE
     *                     tables, Huffman literals, reserved block types)
     */
    public static byte[] decompress(byte[] data) throws IOException {
        if (data == null) throw new NullPointerException("data must not be null");
        if (data.length < 5)
            throw new IOException("frame too short");

        // ── Validate magic ───────────────────────────────────────────────────
        int magic = (data[0] & 0xFF)
                | ((data[1] & 0xFF) << 8)
                | ((data[2] & 0xFF) << 16)
                | ((data[3] & 0xFF) << 24);
        if (magic != MAGIC)
            throw new IOException("bad magic: 0x" + Integer.toHexString(magic) +
                    " (expected 0x" + Integer.toHexString(MAGIC) + ")");

        int pos = 4;

        // ── Parse Frame Header Descriptor ────────────────────────────────────
        // FHD encodes several flags that control the header layout.
        int fhd = data[pos++] & 0xFF;

        // FCS_Field_Size: bits [7:6] of FHD.
        //   00 → 0 bytes if Single_Segment=0, else 1 byte
        //   01 → 2 bytes (value + 256)
        //   10 → 4 bytes
        //   11 → 8 bytes
        int fcsFlag = (fhd >> 6) & 3;

        // Single_Segment_Flag: bit 5. When set, the window descriptor is omitted.
        int singleSeg = (fhd >> 5) & 1;

        // Dict_ID_Flag: bits [1:0]. Indicates how many bytes the dict ID occupies.
        int dictFlag = fhd & 3;

        // ── Window Descriptor ────────────────────────────────────────────────
        // Present only if Single_Segment_Flag = 0. We skip it.
        if (singleSeg == 0) pos++;

        // ── Dict ID ──────────────────────────────────────────────────────────
        int[] dictIdTable = {0, 1, 2, 4};
        pos += dictIdTable[dictFlag];

        // ── Frame Content Size ───────────────────────────────────────────────
        // We read but don't validate FCS (we trust the blocks to be correct).
        int fcsBytes;
        if (fcsFlag == 0) {
            fcsBytes = (singleSeg == 1) ? 1 : 0;
        } else if (fcsFlag == 1) {
            fcsBytes = 2;
        } else if (fcsFlag == 2) {
            fcsBytes = 4;
        } else {
            fcsBytes = 8;
        }
        pos += fcsBytes;

        // ── Blocks ───────────────────────────────────────────────────────────
        // Guard against decompression bombs: cap total output at 256 MB.
        final int MAX_OUTPUT = 256 * 1024 * 1024;
        List<Byte> output = new ArrayList<>();

        while (true) {
            if (pos + 3 > data.length)
                throw new IOException("truncated block header");

            // 3-byte little-endian block header.
            int hdr = (data[pos] & 0xFF)
                    | ((data[pos + 1] & 0xFF) << 8)
                    | ((data[pos + 2] & 0xFF) << 16);
            pos += 3;

            boolean last = (hdr & 1) != 0;
            int btype = (hdr >> 1) & 3;
            int bsize = hdr >> 3;

            switch (btype) {
                case 0 -> {
                    // Raw block: bsize bytes of verbatim content.
                    if (pos + bsize > data.length)
                        throw new IOException("raw block truncated: need " + bsize +
                                " bytes at pos " + pos);
                    if (output.size() + bsize > MAX_OUTPUT)
                        throw new IOException("decompressed size exceeds limit of " +
                                MAX_OUTPUT + " bytes");
                    for (int i = pos; i < pos + bsize; i++) output.add(data[i]);
                    pos += bsize;
                }
                case 1 -> {
                    // RLE block: 1 byte repeated bsize times.
                    if (pos >= data.length)
                        throw new IOException("RLE block missing byte");
                    if (output.size() + bsize > MAX_OUTPUT)
                        throw new IOException("decompressed size exceeds limit of " +
                                MAX_OUTPUT + " bytes");
                    byte rleByte = data[pos++];
                    for (int i = 0; i < bsize; i++) output.add(rleByte);
                }
                case 2 -> {
                    // Compressed block.
                    if (pos + bsize > data.length)
                        throw new IOException("compressed block truncated: need " + bsize +
                                " bytes");
                    decompressBlock(data, pos, bsize, output);
                    pos += bsize;
                }
                case 3 -> throw new IOException("reserved block type 3");
                default -> throw new IOException("unknown block type " + btype);
            }

            if (last) break;
        }

        return toByteArray(output);
    }

    // ─── Utility ─────────────────────────────────────────────────────────────

    /**
     * Convert a {@code List<Byte>} to a primitive {@code byte[]}.
     *
     * @param list the list to convert
     * @return a new byte array containing the list's elements
     */
    private static byte[] toByteArray(List<Byte> list) {
        byte[] arr = new byte[list.size()];
        for (int i = 0; i < arr.length; i++) arr[i] = list.get(i);
        return arr;
    }
}
