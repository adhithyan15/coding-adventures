package com.codingadventures.pdf417;

import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;

/**
 * PDF417 stacked linear barcode encoder — ISO/IEC 15438:2015 compliant.
 *
 * <p>PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
 * Technologies in 1991. The name encodes its geometry: each codeword has
 * exactly <b>4</b> bars and <b>4</b> spaces (8 elements), and every codeword
 * occupies exactly <b>17</b> modules of horizontal space. "417" = 4 × 17.
 *
 * <h2>Where PDF417 is deployed</h2>
 *
 * <ul>
 *   <li><b>AAMVA</b> — North American driver's licences and government IDs</li>
 *   <li><b>IATA BCBP</b> — Airline boarding passes (the barcode on your phone)</li>
 *   <li><b>USPS</b> — Domestic shipping labels</li>
 *   <li><b>US immigration</b> — Form I-94, customs declarations</li>
 *   <li><b>Healthcare</b> — Patient wristbands, medication labels</li>
 * </ul>
 *
 * <h2>Encoding pipeline (v0.1.0 — byte compaction only)</h2>
 *
 * <pre>
 * raw bytes
 *   → byte compaction     (codeword 924 latch + 6-bytes-to-5-codewords base-900)
 *   → length descriptor   (codeword[0] = total codewords in symbol)
 *   → RS ECC              (GF(929) Reed-Solomon, b=3 convention, α=3)
 *   → dimension selection (auto: roughly square symbol)
 *   → padding             (codeword 900 fills unused slots)
 *   → row indicators      (LRI + RRI per row, encode R/C/ECC level)
 *   → cluster table lookup (codeword → 17-module bar/space pattern)
 *   → start/stop patterns (fixed per row)
 *   → ModuleGrid          (abstract boolean grid)
 * </pre>
 *
 * <h2>GF(929) — why a different field?</h2>
 *
 * <p>QR Code uses GF(256) = GF(2^8). PDF417 uses GF(929) where 929 is prime.
 * Because 929 is prime, GF(929) is simply the integers modulo 929 — no
 * polynomial extension field needed. Arithmetic is modular integer arithmetic:
 * add = (a+b) mod 929, mul = (a*b) mod 929. This is simpler in concept
 * but uses a different algorithm than GF(256)'s carry-less multiplication.
 *
 * <h2>Three cluster tables</h2>
 *
 * <p>Every row uses one of three "cluster" tables (0, 3, 6) based on
 * {@code row % 3}. A scanner sweeping one horizontal row can verify it read
 * a genuine PDF417 row — the cluster mismatch is detectable if you're
 * reading from the wrong position in the symbol.
 *
 * <h2>Usage</h2>
 *
 * <pre>{@code
 * // Encode a string to a ModuleGrid (boolean pixel grid):
 * ModuleGrid grid = PDF417.encode("HELLO WORLD".getBytes());
 *
 * // Encode with options (ECC level 4, 5 columns, row height 4):
 * PDF417Options opts = new PDF417Options();
 * opts.eccLevel = 4;
 * opts.columns = 5;
 * opts.rowHeight = 4;
 * ModuleGrid grid = PDF417.encode("HELLO WORLD".getBytes(), opts);
 * }</pre>
 *
 * @see <a href="https://www.iso.org/standard/43816.html">ISO/IEC 15438:2015</a>
 */
public final class PDF417 {

    /** This is a utility class — no instances. */
    private PDF417() {}

    // =========================================================================
    // Public options class
    // =========================================================================

    /**
     * Configuration options for the PDF417 encoder.
     *
     * <p>All fields are optional; defaults produce a valid, auto-sized symbol
     * with an appropriate error correction level.
     */
    public static final class PDF417Options {
        /**
         * Reed-Solomon error correction level (0–8).
         * Higher levels use more ECC codewords and more symbol area, but can
         * recover from more damage.
         *
         * <p>ECC codewords = 2^(eccLevel+1):
         * <ul>
         *   <li>Level 0 → 2 ECC codewords (minimal, detects 1 error)</li>
         *   <li>Level 2 → 8 ECC codewords (default minimum)</li>
         *   <li>Level 4 → 32 ECC codewords (typical for driver's licences)</li>
         *   <li>Level 8 → 512 ECC codewords (maximum, corrects 255 errors)</li>
         * </ul>
         *
         * <p>Default: auto-selected based on data length.
         */
        public Integer eccLevel = null;

        /**
         * Number of data columns in the symbol (1–30).
         * More columns → wider, shorter symbol.
         * Default: auto-selected for a roughly square symbol.
         */
        public Integer columns = null;

        /**
         * Module-rows per logical PDF417 row (1–10).
         * A larger value produces a taller symbol with more scan-line redundancy.
         * Default: 3.
         */
        public int rowHeight = 3;
    }

    // =========================================================================
    // Public error types
    // =========================================================================

    /** Base class for all PDF417 encoding errors. */
    public static class PDF417Exception extends RuntimeException {
        public PDF417Exception(String message) { super(message); }
    }

    /** Input data is too large to fit in a valid PDF417 symbol. */
    public static class InputTooLongException extends PDF417Exception {
        public InputTooLongException(String message) { super(message); }
    }

    /** User-supplied rows or columns are outside valid bounds. */
    public static class InvalidDimensionsException extends PDF417Exception {
        public InvalidDimensionsException(String message) { super(message); }
    }

    /** ECC level is outside the valid range 0–8. */
    public static class InvalidECCLevelException extends PDF417Exception {
        public InvalidECCLevelException(String message) { super(message); }
    }

    // =========================================================================
    // Constants
    // =========================================================================

    /** GF(929) prime modulus. All field elements are in 0..928. */
    static final int GF929_PRIME = 929;

    /** Generator element α = 3. A primitive root modulo 929. */
    static final int GF929_ALPHA = 3;

    /**
     * Multiplicative group order = PRIME − 1 = 928.
     * By Fermat's little theorem, α^928 ≡ 1 (mod 929).
     */
    static final int GF929_ORDER = 928;

    /**
     * Latch-to-byte-compaction codeword (alternate form, any length).
     * Codeword 924 precedes all byte-compacted data in v0.1.0.
     */
    static final int LATCH_BYTE = 924;

    /**
     * Padding codeword (neutral filler).
     * Value 900 is the latch-to-text codeword, which produces no visible
     * output when encountered after the end of real data — making it safe
     * as a symbol filler.
     */
    static final int PADDING_CW = 900;

    /** Minimum number of rows in any PDF417 symbol. */
    static final int MIN_ROWS = 3;

    /** Maximum number of rows in any PDF417 symbol. */
    static final int MAX_ROWS = 90;

    /** Minimum number of data columns. */
    static final int MIN_COLS = 1;

    /** Maximum number of data columns. */
    static final int MAX_COLS = 30;

    // =========================================================================
    // GF(929) log/antilog tables (built once at class load time)
    // =========================================================================
    //
    // GF(929) arithmetic is just integers modulo 929. Since 929 is prime,
    // every non-zero element has a multiplicative inverse. We build log and
    // antilog tables for O(1) multiplication.
    //
    // Tables:
    //   GF_EXP[i] = α^i mod 929   (antilog table, i in 0..928)
    //   GF_LOG[v] = i  such that α^i ≡ v mod 929   (log table, v in 1..928)
    //
    // Size: 929 ints × 2 arrays × 4 bytes = ~7.4 KB. Negligible.

    /** Antilog table: GF_EXP[i] = 3^i mod 929, for i = 0..928. */
    static final int[] GF_EXP = new int[929];

    /** Log table: GF_LOG[v] = i such that 3^i ≡ v mod 929, for v = 1..928. */
    static final int[] GF_LOG = new int[929];

    static {
        // Build the tables by repeated multiplication by α = 3.
        //
        // Proof that this covers all 928 non-zero elements:
        //   3 is a primitive root mod 929, so the sequence
        //   3^0 = 1, 3^1 = 3, 3^2 = 9, ..., 3^927
        //   visits every element of {1, 2, ..., 928} exactly once.
        //
        //   Fermats little theorem guarantees: 3^928 ≡ 1 (mod 929).
        int val = 1;
        for (int i = 0; i < GF929_ORDER; i++) {
            GF_EXP[i] = val;
            GF_LOG[val] = i;
            val = (val * GF929_ALPHA) % GF929_PRIME;
        }
        // GF_EXP[928] = GF_EXP[0] = 1, for wrap-around in gfMul.
        GF_EXP[GF929_ORDER] = GF_EXP[0];
    }

    // =========================================================================
    // GF(929) field arithmetic
    // =========================================================================

    /**
     * GF(929) addition: (a + b) mod 929.
     *
     * <p>Unlike GF(256) where addition is XOR, GF(929) addition is ordinary
     * modular integer addition. The prime characteristic means we never need
     * XOR — the field has characteristic 929, not 2.
     */
    static int gfAdd(int a, int b) {
        return (a + b) % GF929_PRIME;
    }

    /**
     * GF(929) multiplication using log/antilog tables.
     *
     * <p>The trick: log(a*b) = log(a) + log(b), so:
     * <pre>
     *   a * b = α^(log(a) + log(b))
     * </pre>
     * This turns a modular multiplication into two lookups and an addition —
     * much faster than computing {@code (a * b) % 929} directly (which would
     * require a 64-bit intermediate value or careful 32-bit arithmetic).
     *
     * <p>Special case: if either operand is 0, the product is 0 (0 has no
     * defined logarithm in GF(929), so we handle it explicitly).
     */
    static int gfMul(int a, int b) {
        if (a == 0 || b == 0) return 0;
        return GF_EXP[(GF_LOG[a] + GF_LOG[b]) % GF929_ORDER];
    }

    // =========================================================================
    // Reed-Solomon generator polynomial
    // =========================================================================
    //
    // PDF417 uses the b=3 convention: for ECC level L with k = 2^(L+1) ECC
    // codewords, the generator polynomial is:
    //
    //   g(x) = (x − α^3)(x − α^4) ··· (x − α^{k+2})
    //
    // where α = 3 and arithmetic is in GF(929).
    //
    // We build g iteratively, multiplying in each linear factor (x − α^j):
    //
    //   Start:  g = [1]             (degree-0 polynomial)
    //   After factor (x − α^j):
    //     new_g[i]   += g[i]
    //     new_g[i+1] += g[i] * (−α^j)   where −α^j = 929 − α^j in GF(929)
    //
    // Note: in GF(929), −v = 929 − v for any non-zero v.

    /**
     * Build the RS generator polynomial for the given ECC level.
     *
     * <p>Returns k+1 coefficients {@code [g_k, g_{k-1}, ..., g_1, g_0]} where
     * k = 2^(eccLevel+1) and g_k = 1 (leading coefficient always 1).
     *
     * <p>For ECC level 0 (k=2):
     * <pre>
     *   g(x) = (x − 27)(x − 81) = x² − 108x + 2187 ≡ x² + 821x + 329 in GF(929)
     * </pre>
     */
    static int[] buildGenerator(int eccLevel) {
        int k = 1 << (eccLevel + 1); // 2^(eccLevel+1)
        int[] g = {1};

        for (int j = 3; j <= k + 2; j++) {
            int root = GF_EXP[j % GF929_ORDER]; // α^j
            int negRoot = GF929_PRIME - root;    // −α^j in GF(929)

            int[] newG = new int[g.length + 1];
            for (int i = 0; i < g.length; i++) {
                newG[i] = gfAdd(newG[i], g[i]);
                newG[i + 1] = gfAdd(newG[i + 1], gfMul(g[i], negRoot));
            }
            g = newG;
        }

        return g;
    }

    // =========================================================================
    // Reed-Solomon encoder
    // =========================================================================
    //
    // Given data codewords D = [d_0, ..., d_{n-1}] and generator g(x) of
    // degree k, the ECC codewords are the remainder of D(x) × x^k ÷ g(x).
    //
    // Algorithm: standard LFSR (shift-register) polynomial long division.
    //
    //   For each data codeword d:
    //     feedback = d + ecc[0]           (feed into the register)
    //     shift ecc left by one position
    //     ecc[i] += feedback × g[k - i]   (mix feedback through all cells)
    //
    // After processing all data codewords, ecc contains the k ECC values.
    //
    // Key difference from QR Code: PDF417 uses a SINGLE RS block with NO
    // interleaving. All data codewords feed one encoder to produce all ECC
    // codewords. This is simpler than QR's multi-block interleaved scheme.

    /**
     * Compute k RS ECC codewords for {@code data} over GF(929) with b=3.
     *
     * @param data       sequence of data codeword values (each 0..928)
     * @param eccLevel   ECC level 0–8
     * @return           k = 2^(eccLevel+1) ECC codeword values
     */
    static int[] rsEncode(int[] data, int eccLevel) {
        int[] g = buildGenerator(eccLevel);
        int k = g.length - 1; // number of ECC codewords

        int[] ecc = new int[k];

        for (int d : data) {
            int feedback = gfAdd(d, ecc[0]);
            // Shift register left (drop ecc[0], shift 1..k-1 → 0..k-2, clear last).
            System.arraycopy(ecc, 1, ecc, 0, k - 1);
            ecc[k - 1] = 0;
            // Add feedback multiplied by each generator coefficient.
            for (int i = 0; i < k; i++) {
                ecc[i] = gfAdd(ecc[i], gfMul(g[k - i], feedback));
            }
        }

        return ecc;
    }

    // =========================================================================
    // Byte compaction (v0.1.0: the only compaction mode)
    // =========================================================================
    //
    // Byte compaction encodes raw binary data in two sub-modes:
    //
    // 1. Full-group encoding: every 6 bytes → 5 codewords.
    //    Treat 6 bytes as a 48-bit big-endian integer n, then express n in
    //    base 900. This is efficient because:
    //      256^6 = 281,474,976,710,656
    //      900^5 = 590,490,000,000,000
    //    And 256^6 < 900^5, so 6 bytes always fit in 5 base-900 codewords.
    //    The density is 6/5 = 1.2 bytes per codeword.
    //
    // 2. Remainder encoding: remaining 1–5 bytes → 1 codeword each.
    //    Each byte value (0..255) is a valid codeword value directly.
    //    Density: 1 byte per codeword.
    //
    // Latch codeword 924 precedes all byte-compacted data.
    // We use `long` (64-bit) for the 48-bit intermediate value. Safe because
    // 256^6 ≈ 2.81 × 10^14 fits comfortably in a signed long (max ~9.2 × 10^18).

    /**
     * Encode raw bytes using byte compaction mode (codeword 924 latch).
     *
     * @param bytes raw input bytes
     * @return      list starting with [924, c1, c2, ...]
     */
    static List<Integer> byteCompact(byte[] bytes) {
        List<Integer> codewords = new ArrayList<>();
        codewords.add(LATCH_BYTE); // 924 = latch to byte compaction

        int i = 0;
        int len = bytes.length;

        // Process full 6-byte groups → 5 codewords each.
        while (i + 6 <= len) {
            // Assemble 48-bit big-endian value from 6 bytes.
            long n = 0L;
            for (int j = 0; j < 6; j++) {
                n = n * 256L + (bytes[i + j] & 0xFF);
            }
            // Convert n to base 900 → 5 codewords, stored most-significant first.
            int[] group = new int[5];
            for (int j = 4; j >= 0; j--) {
                group[j] = (int)(n % 900L);
                n = n / 900L;
            }
            for (int cw : group) codewords.add(cw);
            i += 6;
        }

        // Remaining 1–5 bytes → 1 codeword each (direct byte value).
        while (i < len) {
            codewords.add(bytes[i] & 0xFF);
            i++;
        }

        return codewords;
    }

    // =========================================================================
    // ECC level auto-selection
    // =========================================================================
    //
    // The recommended minimum ECC level depends on how many data codewords
    // there are. More data → more damage risk → more ECC headroom needed.
    //
    // Rule (from ISO/IEC 15438:2015 §5.6):
    //   ≤ 40 data codewords  → level 2 (8 ECC codewords)
    //   ≤ 160                → level 3 (16 ECC codewords)
    //   ≤ 320                → level 4 (32 ECC codewords)
    //   ≤ 863                → level 5 (64 ECC codewords)
    //   > 863                → level 6 (128 ECC codewords)

    /**
     * Select the minimum recommended ECC level based on data codeword count.
     *
     * @param dataCount number of data codewords (including latch and length descriptor)
     * @return          recommended ECC level 2..6
     */
    static int autoEccLevel(int dataCount) {
        if (dataCount <= 40) return 2;
        if (dataCount <= 160) return 3;
        if (dataCount <= 320) return 4;
        if (dataCount <= 863) return 5;
        return 6;
    }

    // =========================================================================
    // Dimension selection
    // =========================================================================
    //
    // We need to fit `total` codewords into an r × c grid where:
    //   3 ≤ r ≤ 90   (rows)
    //   1 ≤ c ≤ 30   (columns)
    //
    // Heuristic: pick c = ceil(sqrt(total / 3)), clamped to 1..30.
    // Then r = ceil(total / c), clamped to 3..90.
    //
    // The factor of 3 accounts for the typical 3:1 width:height ratio of
    // PDF417 (each row is 3 module-rows tall, while each codeword is 17
    // modules wide). This produces a roughly square barcode.

    /** Container for the chosen number of rows and columns. */
    static final class Dimensions {
        final int rows;
        final int cols;
        Dimensions(int rows, int cols) { this.rows = rows; this.cols = cols; }
    }

    /**
     * Choose rows and columns for a symbol that holds {@code total} codewords.
     *
     * @param total total number of codewords needed
     * @return      (rows, cols) that satisfies rows × cols ≥ total
     */
    static Dimensions chooseDimensions(int total) {
        int c = (int) Math.ceil(Math.sqrt((double) total / 3.0));
        c = Math.max(MIN_COLS, Math.min(MAX_COLS, c));

        int r = (int) Math.ceil((double) total / c);
        r = Math.max(MIN_ROWS, Math.min(MAX_ROWS, r));

        // If clamping rows to MAX_ROWS forces us under capacity, widen columns.
        if ((long) r * c < total) {
            c = (int) Math.ceil((double) total / r);
            c = Math.min(MAX_COLS, c);
            r = (int) Math.ceil((double) total / c);
            r = Math.min(MAX_ROWS, r);
        }

        return new Dimensions(r, c);
    }

    // =========================================================================
    // Row indicator computation
    // =========================================================================
    //
    // Each row in the symbol carries two row indicator codewords (LRI and RRI)
    // that together encode three pieces of metadata:
    //
    //   R_info = (R - 1) / 3         where R = total rows (3..90)
    //   C_info = C - 1               where C = data columns (1..30)
    //   L_info = 3 * L + (R-1) % 3  where L = ECC level (0..8)
    //
    // Each row belongs to one of three clusters (row % 3 = 0, 1, 2).
    // The cluster determines which metadata goes in LRI and which in RRI:
    //
    //   Cluster 0: LRI = 30 * group + R_info,  RRI = 30 * group + C_info
    //   Cluster 1: LRI = 30 * group + L_info,  RRI = 30 * group + R_info
    //   Cluster 2: LRI = 30 * group + C_info,  RRI = 30 * group + L_info
    //
    // where group = r / 3 (integer division).
    //
    // A scanner reading any three consecutive rows (one of each cluster) can
    // recover R, C, and L from the LRI and RRI values. This makes each row
    // independently decodable — even if some rows are damaged or unreadable.
    //
    // Note: the RRI formula here (cluster 0 → C_info, 1 → R_info, 2 → L_info)
    // follows the Python pdf417 library (which produces verified scannable
    // symbols) rather than the original spec text where the spec has a
    // different column assignment for RRI.

    /**
     * Compute the Left Row Indicator codeword value for row {@code r}.
     *
     * @param r        row index (0-indexed)
     * @param rows     total number of rows in the symbol
     * @param cols     number of data columns
     * @param eccLevel Reed-Solomon ECC level
     * @return         LRI codeword value (0..928)
     */
    public static int computeLRI(int r, int rows, int cols, int eccLevel) {
        int rInfo = (rows - 1) / 3;
        int cInfo = cols - 1;
        int lInfo = 3 * eccLevel + (rows - 1) % 3;
        int rowGroup = r / 3;
        int cluster = r % 3;

        if (cluster == 0) return 30 * rowGroup + rInfo;
        if (cluster == 1) return 30 * rowGroup + lInfo;
        return 30 * rowGroup + cInfo;
    }

    /**
     * Compute the Right Row Indicator codeword value for row {@code r}.
     *
     * @param r        row index (0-indexed)
     * @param rows     total number of rows in the symbol
     * @param cols     number of data columns
     * @param eccLevel Reed-Solomon ECC level
     * @return         RRI codeword value (0..928)
     */
    public static int computeRRI(int r, int rows, int cols, int eccLevel) {
        int rInfo = (rows - 1) / 3;
        int cInfo = cols - 1;
        int lInfo = 3 * eccLevel + (rows - 1) % 3;
        int rowGroup = r / 3;
        int cluster = r % 3;

        if (cluster == 0) return 30 * rowGroup + cInfo;
        if (cluster == 1) return 30 * rowGroup + rInfo;
        return 30 * rowGroup + lInfo;
    }

    // =========================================================================
    // Start and stop patterns
    // =========================================================================
    //
    // Every row in a PDF417 symbol starts with the same 17-module start pattern
    // and ends with the same 18-module stop pattern. These are fixed — they do
    // not depend on the row number or cluster.
    //
    // Start pattern: 11111111010101000 (17 modules)
    //   Bars/spaces: [8, 1, 1, 1, 1, 1, 1, 3]
    //   = bar(8) space(1) bar(1) space(1) bar(1) space(1) bar(1) space(3)
    //   Sum: 8+1+1+1+1+1+1+3 = 17 ✓
    //
    // Stop pattern: 111111101000101001 (18 modules)
    //   Bars/spaces: [7, 1, 1, 3, 1, 1, 1, 2, 1]
    //   = bar(7) space(1) bar(1) space(3) bar(1) space(1) bar(1) space(2) bar(1)
    //   Sum: 7+1+1+3+1+1+1+2+1 = 18 ✓
    //   Note: 5 bars, 4 spaces — asymmetric, distinguishable from codewords.
    //
    // Bit patterns:
    //   1 = dark module (bar),  0 = light module (space).

    /** Start pattern bar/space widths: [8, 1, 1, 1, 1, 1, 1, 3] = 17 modules. */
    static final int[] START_PATTERN = {8, 1, 1, 1, 1, 1, 1, 3};

    /** Stop pattern bar/space widths: [7, 1, 1, 3, 1, 1, 1, 2, 1] = 18 modules. */
    static final int[] STOP_PATTERN = {7, 1, 1, 3, 1, 1, 1, 2, 1};

    // =========================================================================
    // Codeword-to-module expansion
    // =========================================================================
    //
    // Each codeword value (0..928) maps to a 17-module bar/space pattern in
    // each of the three cluster tables. The pattern is stored as a packed int:
    //
    //   bits 31..28 = b1 (bar 1 width)
    //   bits 27..24 = s1 (space 1 width)
    //   bits 23..20 = b2
    //   bits 19..16 = s2
    //   bits 15..12 = b3
    //   bits 11..8  = s3
    //   bits  7..4  = b4
    //   bits  3..0  = s4
    //
    // Alternating dark and light modules: bar, space, bar, space, bar, space,
    // bar, space. The first element is always a bar (dark).
    //
    // To expand: unpack 4 bits per element width, then emit that many dark/light
    // modules in sequence.

    /**
     * Expand a packed bar/space pattern into the given boolean array.
     *
     * @param packed     packed pattern from the cluster table
     * @param modules    output array; module values appended at {@code offset}
     * @param offset     index at which to start writing
     * @return           offset after writing (offset + 17)
     */
    static int expandPattern(int packed, boolean[] modules, int offset) {
        // Unpack 8 element widths from the 32-bit packed value.
        int[] widths = {
            (packed >>> 28) & 0xF, // b1
            (packed >>> 24) & 0xF, // s1
            (packed >>> 20) & 0xF, // b2
            (packed >>> 16) & 0xF, // s2
            (packed >>> 12) & 0xF, // b3
            (packed >>>  8) & 0xF, // s3
            (packed >>>  4) & 0xF, // b4
             packed         & 0xF  // s4
        };

        // Alternate: bar=dark, space=light, bar=dark, ...
        boolean dark = true;
        for (int w : widths) {
            for (int k = 0; k < w; k++) {
                modules[offset++] = dark;
            }
            dark = !dark;
        }
        return offset;
    }

    /**
     * Expand a bar/space width array into the given boolean array.
     *
     * <p>Used for start and stop patterns where widths are stored as plain int[].
     *
     * @param widths     bar/space widths; first element is a bar (dark)
     * @param modules    output array
     * @param offset     index at which to start writing
     * @return           offset after writing
     */
    static int expandWidths(int[] widths, boolean[] modules, int offset) {
        boolean dark = true;
        for (int w : widths) {
            for (int k = 0; k < w; k++) {
                modules[offset++] = dark;
            }
            dark = !dark;
        }
        return offset;
    }

    // =========================================================================
    // Main encoder: encode()
    // =========================================================================

    /**
     * Encode raw bytes as a PDF417 symbol and return the {@link ModuleGrid}.
     *
     * <p>Uses default options (auto ECC level, auto dimensions, row height 3).
     *
     * @param data raw bytes to encode
     * @return     PDF417 symbol as a boolean module grid
     * @throws InputTooLongException if the data exceeds symbol capacity
     */
    public static ModuleGrid encode(byte[] data) {
        return encode(data, new PDF417Options());
    }

    /**
     * Encode raw bytes as a PDF417 symbol and return the {@link ModuleGrid}.
     *
     * <p>Full encoding pipeline:
     * <ol>
     *   <li>Byte-compact the input (codeword 924 latch + 6→5 base-900 groups).</li>
     *   <li>Prepend the length descriptor (total codewords including ECC).</li>
     *   <li>Compute Reed-Solomon ECC over GF(929) with b=3.</li>
     *   <li>Choose symbol dimensions (rows × cols ≥ total codewords).</li>
     *   <li>Pad to fill the grid exactly with codeword 900.</li>
     *   <li>Rasterize: for each row, emit start + LRI + data × cols + RRI + stop.</li>
     * </ol>
     *
     * @param data    raw bytes to encode
     * @param options encoding options (ECC level, columns, row height)
     * @return        PDF417 symbol as a boolean module grid
     * @throws InvalidECCLevelException    if eccLevel is out of range 0–8
     * @throws InvalidDimensionsException  if columns is out of range 1–30
     * @throws InputTooLongException       if data exceeds symbol capacity
     */
    public static ModuleGrid encode(byte[] data, PDF417Options options) {
        // ── Validate ECC level ────────────────────────────────────────────────
        if (options.eccLevel != null && (options.eccLevel < 0 || options.eccLevel > 8)) {
            throw new InvalidECCLevelException(
                "ECC level must be 0–8, got " + options.eccLevel
            );
        }

        // ── Step 1: Byte compaction ───────────────────────────────────────────
        // Convert input bytes to a list of codewords using byte compaction.
        // The list starts with [924, c1, c2, ...] where c_i are base-900 values.
        List<Integer> dataCwordsList = byteCompact(data);
        int[] dataCwords = dataCwordsList.stream().mapToInt(Integer::intValue).toArray();

        // ── Step 2: Choose ECC level ──────────────────────────────────────────
        // The auto-level rule uses the count of data codewords (including the
        // latch codeword 924 but NOT the length descriptor yet).
        int eccLevel = options.eccLevel != null
            ? options.eccLevel
            : autoEccLevel(dataCwords.length + 1);  // +1 for length descriptor
        int eccCount = 1 << (eccLevel + 1); // k = 2^(eccLevel+1)

        // ── Step 3: Length descriptor ─────────────────────────────────────────
        // The length descriptor is always the first codeword in the data region.
        // Its value = total codewords in the symbol (length_desc + data + ECC),
        // NOT including row indicators or start/stop patterns.
        //
        //   length_descriptor = 1 (itself) + len(dataCwords) + eccCount
        int lengthDesc = 1 + dataCwords.length + eccCount;

        // Build fullData = [lengthDesc, dataCwords...] — this is what gets RS-encoded.
        int[] fullData = new int[1 + dataCwords.length];
        fullData[0] = lengthDesc;
        System.arraycopy(dataCwords, 0, fullData, 1, dataCwords.length);

        // ── Step 4: RS ECC ────────────────────────────────────────────────────
        int[] eccCwords = rsEncode(fullData, eccLevel);

        // ── Step 5: Choose dimensions ─────────────────────────────────────────
        int totalCwords = fullData.length + eccCwords.length;

        int cols, rows;

        if (options.columns != null) {
            if (options.columns < MIN_COLS || options.columns > MAX_COLS) {
                throw new InvalidDimensionsException(
                    "columns must be 1–30, got " + options.columns
                );
            }
            cols = options.columns;
            rows = Math.max(MIN_ROWS, (int) Math.ceil((double) totalCwords / cols));
            if (rows > MAX_ROWS) {
                throw new InputTooLongException(
                    "Data requires " + rows + " rows (max 90) with " + cols + " columns."
                );
            }
        } else {
            Dimensions dims = chooseDimensions(totalCwords);
            cols = dims.cols;
            rows = dims.rows;
        }

        // Safety check: grid must fit all codewords.
        if ((long) cols * rows < totalCwords) {
            throw new InputTooLongException(
                "Cannot fit " + totalCwords + " codewords in " + rows + "×" + cols + " grid."
            );
        }

        // ── Step 6: Pad ───────────────────────────────────────────────────────
        // Padding goes between data and ECC in the final sequence.
        // fullSequence = [length_desc, data..., pad..., ecc...]
        int paddingCount = rows * cols - totalCwords;

        int[] fullSequence = new int[rows * cols];
        // Copy fullData (length_desc + byte-compacted data).
        System.arraycopy(fullData, 0, fullSequence, 0, fullData.length);
        // Fill padding with codeword 900 (text latch, neutral filler).
        for (int i = fullData.length; i < fullData.length + paddingCount; i++) {
            fullSequence[i] = PADDING_CW;
        }
        // Append ECC codewords at the end.
        System.arraycopy(eccCwords, 0, fullSequence, fullData.length + paddingCount, eccCwords.length);

        // ── Step 7: Rasterize ─────────────────────────────────────────────────
        int rowHeight = Math.max(1, options.rowHeight);
        return rasterize(fullSequence, rows, cols, eccLevel, rowHeight);
    }

    // =========================================================================
    // Rasterization
    // =========================================================================
    //
    // Convert the flat codeword sequence to a ModuleGrid.
    //
    // Each logical PDF417 row becomes rowHeight identical module-rows.
    // Each module-row has:
    //   start(17) + LRI(17) + data×cols(17 each) + RRI(17) + stop(18)
    //   = 69 + 17×cols  modules wide
    //
    // For example, with cols=5:
    //   69 + 85 = 154 modules per row.

    /**
     * Convert the flat codeword sequence to a {@link ModuleGrid}.
     *
     * @param sequence  flat codeword array, length = rows × cols
     * @param rows      number of logical rows
     * @param cols      number of data columns per row
     * @param eccLevel  ECC level (used for row indicators)
     * @param rowHeight module-rows per logical row
     * @return          the completed module grid
     */
    static ModuleGrid rasterize(int[] sequence, int rows, int cols, int eccLevel, int rowHeight) {
        // Total module columns per row: start + LRI + data*cols + RRI + stop
        int moduleWidth = 69 + 17 * cols;
        // Total module rows: each logical row repeated rowHeight times
        int moduleHeight = rows * rowHeight;

        // Use a mutable boolean[][] during construction for O(1) pixel writes.
        // We convert to the immutable ModuleGrid at the end.
        // (Using Barcode2D.setModule() per pixel would be O(n²) due to immutability.)
        boolean[][] pixels = new boolean[moduleHeight][moduleWidth];

        // Precompute the start-pattern module sequence (same for every row).
        boolean[] startModules = new boolean[17];
        expandWidths(START_PATTERN, startModules, 0);

        // Precompute the stop-pattern module sequence (same for every row).
        boolean[] stopModules = new boolean[18];
        expandWidths(STOP_PATTERN, stopModules, 0);

        // Temporary buffer for one row's module sequence.
        boolean[] rowModules = new boolean[moduleWidth];

        for (int r = 0; r < rows; r++) {
            // Determine which cluster table to use for this row.
            // cluster = (r % 3) * 3 → 0, 3, 6 in the spec, but our tables are
            // indexed 0, 1, 2 (CLUSTER_TABLES[r % 3]).
            int clusterIndex = r % 3;
            int[] clusterTable = ClusterTables.CLUSTER_TABLES[clusterIndex];

            int pos = 0; // current position in rowModules[]

            // 1. Start pattern (17 modules) — fixed, same for every row.
            System.arraycopy(startModules, 0, rowModules, pos, 17);
            pos += 17;

            // 2. Left Row Indicator (17 modules).
            //    The LRI codeword encodes row group, R, C, and L metadata.
            int lri = computeLRI(r, rows, cols, eccLevel);
            pos = expandPattern(clusterTable[lri], rowModules, pos);

            // 3. Data codewords (17 modules each).
            for (int j = 0; j < cols; j++) {
                int cw = sequence[r * cols + j];
                pos = expandPattern(clusterTable[cw], rowModules, pos);
            }

            // 4. Right Row Indicator (17 modules).
            int rri = computeRRI(r, rows, cols, eccLevel);
            pos = expandPattern(clusterTable[rri], rowModules, pos);

            // 5. Stop pattern (18 modules).
            System.arraycopy(stopModules, 0, rowModules, pos, 18);
            pos += 18;

            // Sanity check: we should have produced exactly moduleWidth modules.
            if (pos != moduleWidth) {
                throw new IllegalStateException(
                    "Row " + r + ": got " + pos + " modules, expected " + moduleWidth
                );
            }

            // Write this module row `rowHeight` times into the pixel grid.
            int moduleRowBase = r * rowHeight;
            for (int h = 0; h < rowHeight; h++) {
                System.arraycopy(rowModules, 0, pixels[moduleRowBase + h], 0, moduleWidth);
            }
        }

        // Convert mutable boolean[][] to immutable ModuleGrid.
        List<List<Boolean>> modulesList = new ArrayList<>(moduleHeight);
        for (int row = 0; row < moduleHeight; row++) {
            List<Boolean> rowList = new ArrayList<>(moduleWidth);
            for (int col = 0; col < moduleWidth; col++) {
                rowList.add(pixels[row][col]);
            }
            modulesList.add(Collections.unmodifiableList(rowList));
        }
        return new ModuleGrid(moduleHeight, moduleWidth, Collections.unmodifiableList(modulesList), ModuleShape.SQUARE);
    }

    // =========================================================================
    // Convenience overloads
    // =========================================================================

    /**
     * Encode a UTF-8 string as a PDF417 symbol using default options.
     *
     * @param text UTF-8 string to encode
     * @return     PDF417 symbol as a boolean module grid
     */
    public static ModuleGrid encode(String text) {
        return encode(text.getBytes(java.nio.charset.StandardCharsets.UTF_8));
    }

    /**
     * Encode a UTF-8 string as a PDF417 symbol with the given options.
     *
     * @param text    UTF-8 string to encode
     * @param options encoding options
     * @return        PDF417 symbol as a boolean module grid
     */
    public static ModuleGrid encode(String text, PDF417Options options) {
        return encode(text.getBytes(java.nio.charset.StandardCharsets.UTF_8), options);
    }
}
