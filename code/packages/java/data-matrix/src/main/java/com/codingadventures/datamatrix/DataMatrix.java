package com.codingadventures.datamatrix;

import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Data Matrix ECC200 encoder — ISO/IEC 16022:2006 compliant.
 *
 * <p>Data Matrix was invented by RVSI Acuity CiMatrix in 1989 and standardised
 * as ISO/IEC 16022:2006. The ECC200 variant uses Reed-Solomon error correction
 * over GF(256)/0x12D and is the dominant form worldwide.
 *
 * <p>It is used wherever a small, high-density, damage-tolerant mark is needed:
 * <ul>
 *   <li>PCB traceability (every board carries a Data Matrix)</li>
 *   <li>Pharmaceutical unit-dose packaging (US FDA DSCSA mandate)</li>
 *   <li>Aerospace parts marking (rivets, shims, brackets — etched in metal)</li>
 *   <li>US Postal Service registered mail and customs forms</li>
 *   <li>Medical device identification (GS1 DataMatrix on surgical instruments)</li>
 * </ul>
 *
 * <h2>Encoding pipeline</h2>
 *
 * <pre>
 * input string
 *   → ASCII encoding     (chars+1; digit pairs packed into one codeword)
 *   → symbol selection   (smallest symbol whose capacity ≥ codeword count)
 *   → pad to capacity    (scrambled-pad codewords fill unused slots)
 *   → RS blocks + ECC    (GF(256)/0x12D, b=1 convention, pre-built gen polys)
 *   → interleave blocks  (data round-robin then ECC round-robin)
 *   → grid init          (L-finder + timing border + alignment borders)
 *   → Utah placement     (diagonal codeword placement, no masking!)
 *   → ModuleGrid         (abstract boolean grid, true = dark)
 * </pre>
 *
 * <h2>Key differences from QR Code</h2>
 *
 * <ul>
 *   <li>Uses GF(256)/0x12D (not QR's 0x11D)</li>
 *   <li>Uses b=1 RS convention (roots α^1..α^n) — matches MA02 reed-solomon</li>
 *   <li>L-shaped finder + clock border instead of three finder squares</li>
 *   <li>Utah diagonal placement instead of two-column zigzag</li>
 *   <li>NO masking step — diagonal placement distributes bits well enough</li>
 * </ul>
 *
 * <h2>Usage</h2>
 *
 * <pre>{@code
 * // Encode a string to the smallest fitting symbol:
 * ModuleGrid grid = DataMatrix.encode("Hello World", null);
 * assert grid.rows == 16;  // 11 chars → 11 codewords → 16×16 symbol
 *
 * // Encode with explicit shape preference:
 * DataMatrixOptions opts = new DataMatrixOptions(DataMatrixOptions.SymbolShape.SQUARE);
 * ModuleGrid grid2 = DataMatrix.encode("A", opts);
 * assert grid2.rows == 10;  // "A" → 1 codeword → 10×10 symbol
 * }</pre>
 */
public final class DataMatrix {

    /** Utility class — no instances. */
    private DataMatrix() {}

    // =========================================================================
    // Public types
    // =========================================================================

    /**
     * Thrown when the input data is too long to fit in any Data Matrix symbol.
     *
     * <p>The maximum capacity is 1558 codewords (144×144 symbol).
     */
    public static final class InputTooLongException extends RuntimeException {
        public InputTooLongException(String message) {
            super(message);
        }
    }

    /**
     * Symbol shape preference.
     *
     * <p>Data Matrix supports both square symbols (10×10 through 144×144) and
     * rectangular symbols (8×18 through 16×48). By default the encoder prefers
     * square symbols, which are the most common in practice.
     */
    public enum SymbolShape {
        /** Select from square symbols only (default). */
        SQUARE,
        /** Select from rectangular symbols only. */
        RECTANGULAR,
        /** Try both and pick the smallest overall. */
        ANY
    }

    /**
     * Options controlling symbol selection and encoding.
     *
     * <p>All fields have defaults — passing {@code null} to {@link #encode} is
     * equivalent to passing {@code new DataMatrixOptions()}.
     */
    public static final class DataMatrixOptions {
        /** Shape preference (default: SQUARE). */
        public final SymbolShape shape;

        /** Create options with default shape (SQUARE). */
        public DataMatrixOptions() {
            this.shape = SymbolShape.SQUARE;
        }

        /** Create options with the given shape preference. */
        public DataMatrixOptions(SymbolShape shape) {
            this.shape = shape;
        }
    }

    // =========================================================================
    // GF(256) over 0x12D — Data Matrix field
    // =========================================================================

    /**
     * Data Matrix uses GF(256) with primitive polynomial 0x12D:
     *
     * <pre>
     * p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  = 0x12D = 301
     * </pre>
     *
     * <p>This is DIFFERENT from QR Code's 0x11D polynomial. Both are degree-8
     * irreducible polynomials over GF(2), but the fields are non-isomorphic.
     * The generator element α = 2 generates all 255 non-zero elements.
     *
     * <p>Key values:
     * <pre>
     * α^0  = 1    (0x01)
     * α^1  = 2    (0x02)
     * α^7  = 128  (0x80)
     * α^8  = 0x2D = 45  (0x80<<1 = 0x100 XOR 0x12D = 0x2D)
     * α^9  = 0x5A = 90
     * α^10 = 0xB4 = 180
     * α^255 = 1   (multiplicative order = 255)
     * </pre>
     *
     * <p>We precompute exp and log tables in a static initializer.
     */
    private static final int[] GF_EXP = new int[256];
    private static final int[] GF_LOG = new int[256];

    static {
        int val = 1;
        for (int i = 0; i < 255; i++) {
            GF_EXP[i] = val;
            GF_LOG[val] = i;
            val <<= 1;             // multiply by α (= x in GF(2)[x])
            if ((val & 0x100) != 0) {  // degree-8 term appeared → reduce by 0x12D
                val ^= 0x12D;
            }
        }
        // gfExp[255] = gfExp[0] = 1 (multiplicative order = 255)
        GF_EXP[255] = GF_EXP[0];
        // GF_LOG[0] remains 0 — undefined sentinel, guarded before use
    }

    /**
     * Multiply two GF(256)/0x12D elements using log/antilog tables.
     *
     * <p>For a, b ≠ 0:  a × b = α^{(log[a] + log[b]) mod 255}
     * If either operand is 0, the product is 0 (zero absorbs multiplication).
     *
     * <p>The log/antilog trick turns a polynomial-reduction multiplication into
     * two table lookups and a modular addition — constant time regardless of
     * the degree of the polynomial.
     */
    static int gfMul(int a, int b) {
        if (a == 0 || b == 0) return 0;
        return GF_EXP[(GF_LOG[a] + GF_LOG[b]) % 255];
    }

    // =========================================================================
    // Symbol size table
    // =========================================================================

    /**
     * Descriptor for a single Data Matrix ECC200 symbol size.
     *
     * <p>A "data region" is one rectangular sub-area of the symbol interior.
     * Small symbols (≤ 26×26) have a single 1×1 region. Larger symbols
     * subdivide into a grid of regions separated by alignment borders.
     *
     * <p>The Utah placement algorithm works on the <b>logical data matrix</b> —
     * all region interiors concatenated — then maps back to physical coordinates.
     *
     * <p>Field names follow the ISO/IEC 16022:2006 Table 7 terminology:
     * <ul>
     *   <li>{@code symbolRows} / {@code symbolCols} — total symbol dimensions</li>
     *   <li>{@code regionRows} / {@code regionCols} — rr × rc data regions</li>
     *   <li>{@code dataRegionHeight} / {@code dataRegionWidth} — interior per region</li>
     *   <li>{@code dataCW} — total data codeword capacity</li>
     *   <li>{@code eccCW} — total ECC codeword count</li>
     *   <li>{@code numBlocks} — interleaved RS blocks</li>
     *   <li>{@code eccPerBlock} — ECC codewords per block</li>
     * </ul>
     */
    record SymbolSizeEntry(
            int symbolRows,
            int symbolCols,
            int regionRows,
            int regionCols,
            int dataRegionHeight,
            int dataRegionWidth,
            int dataCW,
            int eccCW,
            int numBlocks,
            int eccPerBlock
    ) {}

    /**
     * All square symbol sizes for Data Matrix ECC200.
     *
     * <p>Source: ISO/IEC 16022:2006, Table 7 (square symbols).
     * Every entry has been verified against the standard tables.
     *
     * <p>For single-region symbols (regionRows=regionCols=1), the logical data
     * matrix is exactly the interior of the symbol (symbolRows-2) × (symbolCols-2).
     * For multi-region symbols, the interior is subdivided by alignment borders.
     */
    private static final SymbolSizeEntry[] SQUARE_SIZES = {
        //  sR   sC  rR rC drH drW  dCW eCW blk ePB
        new SymbolSizeEntry(10,  10,  1, 1,  8,  8,    3,   5, 1,  5),
        new SymbolSizeEntry(12,  12,  1, 1, 10, 10,    5,   7, 1,  7),
        new SymbolSizeEntry(14,  14,  1, 1, 12, 12,    8,  10, 1, 10),
        new SymbolSizeEntry(16,  16,  1, 1, 14, 14,   12,  12, 1, 12),
        new SymbolSizeEntry(18,  18,  1, 1, 16, 16,   18,  14, 1, 14),
        new SymbolSizeEntry(20,  20,  1, 1, 18, 18,   22,  18, 1, 18),
        new SymbolSizeEntry(22,  22,  1, 1, 20, 20,   30,  20, 1, 20),
        new SymbolSizeEntry(24,  24,  1, 1, 22, 22,   36,  24, 1, 24),
        new SymbolSizeEntry(26,  26,  1, 1, 24, 24,   44,  28, 1, 28),
        new SymbolSizeEntry(32,  32,  2, 2, 14, 14,   62,  36, 2, 18),
        new SymbolSizeEntry(36,  36,  2, 2, 16, 16,   86,  42, 2, 21),
        new SymbolSizeEntry(40,  40,  2, 2, 18, 18,  114,  48, 2, 24),
        new SymbolSizeEntry(44,  44,  2, 2, 20, 20,  144,  56, 4, 14),
        new SymbolSizeEntry(48,  48,  2, 2, 22, 22,  174,  68, 4, 17),
        new SymbolSizeEntry(52,  52,  2, 2, 24, 24,  204,  84, 4, 21),
        new SymbolSizeEntry(64,  64,  4, 4, 14, 14,  280, 112, 4, 28),
        new SymbolSizeEntry(72,  72,  4, 4, 16, 16,  368, 144, 4, 36),
        new SymbolSizeEntry(80,  80,  4, 4, 18, 18,  456, 192, 4, 48),
        new SymbolSizeEntry(88,  88,  4, 4, 20, 20,  576, 224, 4, 56),
        new SymbolSizeEntry(96,  96,  4, 4, 22, 22,  696, 272, 4, 68),
        new SymbolSizeEntry(104, 104, 4, 4, 24, 24,  816, 336, 6, 56),
        new SymbolSizeEntry(120, 120, 6, 6, 18, 18, 1050, 408, 6, 68),
        new SymbolSizeEntry(132, 132, 6, 6, 20, 20, 1304, 496, 8, 62),
        new SymbolSizeEntry(144, 144, 6, 6, 22, 22, 1558, 620,10, 62),
    };

    /**
     * All rectangular symbol sizes for Data Matrix ECC200.
     *
     * <p>Source: ISO/IEC 16022:2006, Table 7 (rectangular symbols).
     * Rectangular symbols follow all the same encoding rules as square symbols.
     * The Utah algorithm handles non-square grids correctly via its boundary
     * wrap conditions.
     */
    private static final SymbolSizeEntry[] RECT_SIZES = {
        //  sR  sC  rR rC drH drW  dCW eCW blk ePB
        new SymbolSizeEntry( 8, 18,  1, 1,  6, 16,   5,  7, 1,  7),
        new SymbolSizeEntry( 8, 32,  1, 2,  6, 14,  10, 11, 1, 11),
        new SymbolSizeEntry(12, 26,  1, 1, 10, 24,  16, 14, 1, 14),
        new SymbolSizeEntry(12, 36,  1, 2, 10, 16,  22, 18, 1, 18),
        new SymbolSizeEntry(16, 36,  1, 2, 14, 16,  32, 24, 1, 24),
        new SymbolSizeEntry(16, 48,  1, 2, 14, 22,  49, 28, 1, 28),
    };

    // =========================================================================
    // Reed-Solomon generator polynomials
    // =========================================================================

    /**
     * Build a generator polynomial g(x) = ∏(x + α^k) for k=1..nEcc
     * over GF(256)/0x12D using the b=1 convention.
     *
     * <p>The b=1 convention means the roots are α^1, α^2, ..., α^{nEcc}.
     * This is exactly what ISO/IEC 16022 requires for Data Matrix ECC200.
     *
     * <p>The returned array has {@code nEcc+1} elements, with the implicit
     * leading coefficient 1 at index 0 and the constant term at index nEcc.
     *
     * <p>Results are cached — generator polynomials of the same length are
     * only computed once per JVM lifetime.
     *
     * <p>Algorithm (multiply-in one root at a time):
     * <pre>
     * g = [1]  (degree-0 polynomial = 1)
     * for i = 1 to nEcc:
     *   root = α^i = GF_EXP[i]
     *   g = g × (x + root)    // polynomial multiplication in GF(256)
     * </pre>
     */
    private static final int[][] GEN_POLY_CACHE = new int[70][];

    static int[] buildGenerator(int nEcc) {
        if (GEN_POLY_CACHE[nEcc] != null) return GEN_POLY_CACHE[nEcc];

        int[] g = {1};  // g(x) = 1
        for (int i = 1; i <= nEcc; i++) {
            int ai = GF_EXP[i];  // α^i — the next root to multiply in
            int[] next = new int[g.length + 1];
            // Multiply g by (x + α^i):
            //   next[j]   ^= g[j]          (coefficient of x^j from x term)
            //   next[j+1] ^= g[j] × α^i   (coefficient of x^j from α^i constant)
            for (int j = 0; j < g.length; j++) {
                next[j]     ^= g[j];
                next[j + 1] ^= gfMul(g[j], ai);
            }
            g = next;
        }
        GEN_POLY_CACHE[nEcc] = g;
        return g;
    }

    // Pre-build all generators needed for the symbol size table
    static {
        for (SymbolSizeEntry e : SQUARE_SIZES) buildGenerator(e.eccPerBlock());
        for (SymbolSizeEntry e : RECT_SIZES)   buildGenerator(e.eccPerBlock());
    }

    // =========================================================================
    // Reed-Solomon encoding (b=1, GF(256)/0x12D)
    // =========================================================================

    /**
     * Compute {@code nEcc} ECC bytes for a data block using LFSR polynomial division.
     *
     * <p>Algorithm: R(x) = D(x) × x^{nEcc} mod G(x)
     *
     * <p>LFSR (shift register) implementation — the standard approach for systematic
     * RS encoding used in QR Code, Data Matrix, Aztec Code, etc.:
     * <pre>
     * rem = [0, 0, ..., 0]   (length nEcc)
     * for each data byte d:
     *   feedback = d XOR rem[0]
     *   shift rem left: rem[i] ← rem[i+1]
     *   rem[i] ^= gen[i+1] × feedback   for i = 0..nEcc-1
     * </pre>
     *
     * @param data      data codewords for this block
     * @param generator generator polynomial (nEcc+1 elements, leading 1)
     * @return          nEcc ECC codewords
     */
    static int[] rsEncodeBlock(int[] data, int[] generator) {
        int nEcc = generator.length - 1;
        int[] rem = new int[nEcc];  // remainder polynomial, all zeros initially
        for (int d : data) {
            int fb = d ^ rem[0];  // feedback = data byte XOR front of remainder
            // Shift register left
            System.arraycopy(rem, 1, rem, 0, nEcc - 1);
            rem[nEcc - 1] = 0;
            // XOR feedback × generator coefficients into each position
            if (fb != 0) {
                for (int i = 0; i < nEcc; i++) {
                    rem[i] ^= gfMul(generator[i + 1], fb);
                }
            }
        }
        return rem;
    }

    // =========================================================================
    // ASCII data encoding
    // =========================================================================

    /**
     * Encode input bytes in Data Matrix ASCII mode.
     *
     * <p>ASCII mode rules (ISO/IEC 16022:2006 §5.2.1):
     * <ul>
     *   <li>Two consecutive ASCII digits → codeword = 130 + (d1×10 + d2).
     *       Saves one codeword versus encoding each digit separately.</li>
     *   <li>Single ASCII char (0–127) → codeword = ASCII_value + 1</li>
     *   <li>Extended ASCII (128–255) → two codewords: 235 (UPPER_SHIFT), then ASCII-127</li>
     * </ul>
     *
     * <p>The digit-pair optimization is critical for manufacturing lot codes and
     * serial numbers that are mostly digit strings. "12345678" encodes as just
     * 4 codewords instead of 8.
     *
     * <p>Examples:
     * <pre>
     * "A"    → [66]           (65+1)
     * " "    → [33]           (32+1)
     * "12"   → [142]          (130+12, digit pair)
     * "1234" → [142, 174]     (two digit pairs)
     * "1A"   → [50, 66]       (digit then letter — no pair, 'A' is not a digit)
     * "00"   → [130]          (130+0)
     * "99"   → [229]          (130+99)
     * </pre>
     */
    static int[] encodeAscii(byte[] input) {
        List<Integer> codewords = new ArrayList<>();
        int i = 0;
        while (i < input.length) {
            int c = input[i] & 0xFF;  // unsigned byte value
            // Digit-pair check: both current and next byte are ASCII digits 0x30..0x39
            if (c >= 0x30 && c <= 0x39
                    && i + 1 < input.length
                    && (input[i + 1] & 0xFF) >= 0x30
                    && (input[i + 1] & 0xFF) <= 0x39) {
                int d1 = c - 0x30;                      // first digit  (0–9)
                int d2 = (input[i + 1] & 0xFF) - 0x30;  // second digit (0–9)
                codewords.add(130 + d1 * 10 + d2);
                i += 2;
            } else if (c <= 127) {
                // Standard ASCII single character
                codewords.add(c + 1);
                i++;
            } else {
                // Extended ASCII (128–255): UPPER_SHIFT then shifted value
                codewords.add(235);       // UPPER_SHIFT codeword
                codewords.add(c - 127);   // shifted codeword (1–128)
                i++;
            }
        }
        return codewords.stream().mapToInt(Integer::intValue).toArray();
    }

    // =========================================================================
    // Pad codewords (ISO/IEC 16022:2006 §5.2.3)
    // =========================================================================

    /**
     * Pad encoded codewords to exactly {@code dataCW} length.
     *
     * <p>Padding rules from ISO/IEC 16022:2006 §5.2.3:
     * <ol>
     *   <li>First pad codeword is always 129.</li>
     *   <li>Subsequent pads use a scrambled value:
     *       {@code scrambled = 129 + (149 × k mod 253) + 1}
     *       {@code if scrambled > 254: scrambled -= 254}
     *       where k is the 1-indexed position within the full codeword stream.</li>
     * </ol>
     *
     * <p>The scrambling prevents a run of "129 129 129..." from creating a
     * degenerate placement pattern in the Utah algorithm. It uses a different
     * modulus (253/254) than the Base256 randomizer (255/256).
     *
     * <p>Example for "A" (codeword [66]) in a 10×10 symbol (dataCW=3):
     * <pre>
     * k=2: pad = 129 (first pad, always literal 129)
     * k=3: scrambled = 129 + (149×3 mod 253) + 1
     *                = 129 + (447 mod 253) + 1
     *                = 129 + 194 + 1 = 324; 324 > 254 → 324 - 254 = 70
     * Final: [66, 129, 70]
     * </pre>
     */
    static int[] padCodewords(int[] codewords, int dataCW) {
        int[] padded = new int[dataCW];
        System.arraycopy(codewords, 0, padded, 0, codewords.length);

        // k is 1-indexed position within the full codeword stream
        int k = codewords.length + 1;  // position of first pad byte
        for (int i = codewords.length; i < dataCW; i++) {
            if (i == codewords.length) {
                // First pad is always literal 129
                padded[i] = 129;
            } else {
                // Subsequent pads are scrambled using 149×k formula
                int scrambled = 129 + ((149 * k) % 253) + 1;
                if (scrambled > 254) scrambled -= 254;
                padded[i] = scrambled;
            }
            k++;
        }
        return padded;
    }

    // =========================================================================
    // Symbol selection
    // =========================================================================

    /**
     * Select the smallest symbol whose dataCW capacity fits the encoded codeword count.
     *
     * <p>Iterates sizes in ascending order (smallest first).
     * Square symbols are preferred by default; rectangular symbols are included
     * when shape = RECTANGULAR or ANY.
     *
     * @param codewordCount number of encoded codewords (before padding)
     * @param shape         shape preference (SQUARE, RECTANGULAR, or ANY)
     * @return              the smallest fitting symbol size entry
     * @throws InputTooLongException if no symbol can accommodate the input
     */
    static SymbolSizeEntry selectSymbol(int codewordCount, SymbolShape shape) {
        List<SymbolSizeEntry> candidates = new ArrayList<>();
        if (shape == SymbolShape.SQUARE || shape == SymbolShape.ANY) {
            Collections.addAll(candidates, SQUARE_SIZES);
        }
        if (shape == SymbolShape.RECTANGULAR || shape == SymbolShape.ANY) {
            Collections.addAll(candidates, RECT_SIZES);
        }

        // Sort by dataCW ascending (smallest capacity first), break ties by area
        candidates.sort((a, b) -> {
            if (a.dataCW() != b.dataCW()) return a.dataCW() - b.dataCW();
            return (a.symbolRows() * a.symbolCols()) - (b.symbolRows() * b.symbolCols());
        });

        for (SymbolSizeEntry e : candidates) {
            if (e.dataCW() >= codewordCount) return e;
        }

        throw new InputTooLongException(
                "Encoded data requires " + codewordCount +
                " codewords, exceeds maximum 1558 (144×144 symbol).");
    }

    // =========================================================================
    // RS block splitting and interleaving
    // =========================================================================

    /**
     * Split padded data into RS blocks, compute ECC for each, and interleave.
     *
     * <p>For multi-block symbols, data is split as evenly as possible:
     * <ul>
     *   <li>If dataCW is divisible by numBlocks: each block gets dataCW/numBlocks.</li>
     *   <li>Otherwise: the first (dataCW mod numBlocks) blocks get one extra codeword.
     *       This is the ISO/IEC 16022 interleaving convention.</li>
     * </ul>
     *
     * <p>Interleaving distributes burst errors: a physical scratch destroying N
     * contiguous modules affects at most ceil(N / numBlocks) codewords per block,
     * well within each block's correction capacity.
     *
     * <p>Interleaved output layout:
     * <pre>
     * [data[0][0], data[1][0], ..., data[B-1][0],
     *  data[0][1], data[1][1], ..., data[B-1][1],
     *  ...
     *  ecc[0][0],  ecc[1][0],  ..., ecc[B-1][0],
     *  ecc[0][1],  ecc[1][1],  ..., ecc[B-1][1],
     *  ...]
     * </pre>
     */
    private static int[] computeInterleaved(int[] data, SymbolSizeEntry e) {
        int numBlocks = e.numBlocks();
        int eccPerBlock = e.eccPerBlock();
        int[] gen = buildGenerator(eccPerBlock);

        // Split data into blocks
        int baseLen = e.dataCW() / numBlocks;
        int extraBlocks = e.dataCW() % numBlocks;  // these blocks get baseLen+1

        int[][] dataBlocks = new int[numBlocks][];
        int offset = 0;
        for (int b = 0; b < numBlocks; b++) {
            int len = (b < extraBlocks) ? baseLen + 1 : baseLen;
            dataBlocks[b] = new int[len];
            System.arraycopy(data, offset, dataBlocks[b], 0, len);
            offset += len;
        }

        // Compute ECC for each block
        int[][] eccBlocks = new int[numBlocks][];
        for (int b = 0; b < numBlocks; b++) {
            eccBlocks[b] = rsEncodeBlock(dataBlocks[b], gen);
        }

        // Interleave: data round-robin
        int maxDataLen = 0;
        for (int[] db : dataBlocks) maxDataLen = Math.max(maxDataLen, db.length);

        List<Integer> interleaved = new ArrayList<>();
        for (int pos = 0; pos < maxDataLen; pos++) {
            for (int b = 0; b < numBlocks; b++) {
                if (pos < dataBlocks[b].length) {
                    interleaved.add(dataBlocks[b][pos]);
                }
            }
        }

        // Interleave: ECC round-robin
        for (int pos = 0; pos < eccPerBlock; pos++) {
            for (int b = 0; b < numBlocks; b++) {
                interleaved.add(eccBlocks[b][pos]);
            }
        }

        return interleaved.stream().mapToInt(Integer::intValue).toArray();
    }

    // =========================================================================
    // Grid initialization (border + alignment borders)
    // =========================================================================

    /**
     * Initialize the physical module grid with fixed structural elements.
     *
     * <p>The "finder + clock" border (outermost ring of every Data Matrix symbol):
     *
     * <pre>
     * Top row (row 0):        alternating dark/light starting dark at col 0
     *                         These are the timing clock dots for the top edge.
     * Right col (col C-1):    alternating dark/light starting dark at row 0
     *                         Timing clock dots for the right edge.
     * Bottom row (row R-1):   all dark — horizontal leg of the L-finder.
     * Left col  (col 0):      all dark — vertical leg of the L-finder.
     * </pre>
     *
     * <p>The L-shaped solid-dark bar (left+bottom) tells a scanner where the symbol
     * starts and which way it is oriented (the asymmetry distinguishes rotation).
     * The alternating pattern on top and right is a timing clock — the scanner uses
     * it to measure module pitch and correct for slight distortion.
     *
     * <p>For multi-region symbols (e.g. 32×32 with 2×2 regions), alignment borders
     * are placed between data regions. Each alignment border is 2 modules wide:
     * <ul>
     *   <li>Row/Col AB+0: all dark</li>
     *   <li>Row/Col AB+1: alternating dark/light starting dark</li>
     * </ul>
     */
    private static boolean[][] initGrid(SymbolSizeEntry e) {
        int R = e.symbolRows();
        int C = e.symbolCols();
        boolean[][] grid = new boolean[R][C];  // all false = light initially

        // ── Alignment borders (written FIRST so outer border can override)
        // Between each pair of adjacent region rows/cols there are 2 border rows/cols.
        for (int rr = 0; rr < e.regionRows() - 1; rr++) {
            // Physical row of first AB row:
            //   outer border (1) + (rr+1)×drH + rr×2 (previous alignment borders)
            int abRow0 = 1 + (rr + 1) * e.dataRegionHeight() + rr * 2;
            int abRow1 = abRow0 + 1;
            for (int c = 0; c < C; c++) {
                grid[abRow0][c] = true;            // all dark
                grid[abRow1][c] = (c % 2 == 0);   // alternating dark/light
            }
        }

        for (int rc = 0; rc < e.regionCols() - 1; rc++) {
            // Physical col of first AB col:
            int abCol0 = 1 + (rc + 1) * e.dataRegionWidth() + rc * 2;
            int abCol1 = abCol0 + 1;
            for (int r = 0; r < R; r++) {
                grid[r][abCol0] = true;            // all dark
                grid[r][abCol1] = (r % 2 == 0);   // alternating dark/light
            }
        }

        // ── Top row (row 0): alternating dark/light starting dark at col 0
        // Written after alignment borders so outer timing overrides AB at intersections.
        for (int c = 0; c < C; c++) grid[0][c] = (c % 2 == 0);

        // ── Right column (col C-1): alternating dark/light starting dark at row 0
        for (int r = 0; r < R; r++) grid[r][C - 1] = (r % 2 == 0);

        // ── Left column (col 0): all dark (L-finder left leg)
        // Written AFTER timing rows/cols to override any timing value at col 0.
        for (int r = 0; r < R; r++) grid[r][0] = true;

        // ── Bottom row (row R-1): all dark (L-finder bottom leg)
        // Written LAST so the L-finder overrides alignment border alternating values
        // and the right-column timing at (R-1, C-1).
        for (int c = 0; c < C; c++) grid[R - 1][c] = true;

        return grid;
    }

    // =========================================================================
    // Utah placement algorithm
    // =========================================================================

    /**
     * Apply boundary wrap rules to a (row, col) in the logical grid.
     *
     * <p>When the standard Utah shape extends beyond the logical grid edge,
     * these rules fold the coordinates back into the valid range.
     *
     * <p>The exact rules from ISO/IEC 16022:2006 Annex F:
     * <pre>
     * If row &lt; 0 and col == 0:    row = 1; col = 3   (special corner singularity)
     * If row &lt; 0 and col == nCols: row = 0; col -= 2  (wrapped past right at top)
     * If row &lt; 0 (and col &gt; 0):   row += nRows; col -= 4
     * If col &lt; 0 (and row &gt;= 0):  col += nCols; row -= 4
     * </pre>
     *
     * <p>These handle the diagonal scanning when near the top or left edges.
     * The special-case ordering matters: check col==0 and col==nCols before
     * the general row&lt;0 rule.
     */
    private static int[] applyWrap(int row, int col, int nRows, int nCols) {
        // Special case: top-left corner singularity
        if (row < 0 && col == 0) return new int[]{1, 3};
        // Special case: wrapped past the right edge at the top
        if (row < 0 && col == nCols) return new int[]{0, col - 2};
        // Wrap row off top → wrap to bottom and shift left
        if (row < 0) return new int[]{row + nRows, col - 4};
        // Wrap col off left → wrap to right and shift up
        if (col < 0) return new int[]{row - 4, col + nCols};
        return new int[]{row, col};
    }

    /**
     * Place one codeword using the standard "Utah" 8-module pattern.
     *
     * <p>The Utah shape (named because it resembles the US state of Utah):
     *
     * <pre>
     *   col: c-2  c-1   c
     * row-2:  .   [1]  [2]
     * row-1: [3]  [4]  [5]
     * row  : [6]  [7]  [8]
     * </pre>
     *
     * <p>Numbers [1]–[8] correspond to bits 1–8 (1 = LSB, 8 = MSB).
     * Bits are placed MSB-first:
     * <pre>
     * bit 8 (MSB): (row,   col)
     * bit 7:       (row,   col-1)
     * bit 6:       (row,   col-2)
     * bit 5:       (row-1, col)
     * bit 4:       (row-1, col-1)
     * bit 3:       (row-1, col-2)
     * bit 2:       (row-2, col)
     * bit 1:       (row-2, col-1)
     * </pre>
     *
     * <p>Each position is wrapped via {@link #applyWrap} before being written.
     * Positions that are already occupied (used[][]) are skipped.
     */
    private static void placeUtah(int codeword, int row, int col,
                                   int nRows, int nCols,
                                   boolean[][] grid, boolean[][] used) {
        // [rawRow, rawCol, bitIndex (7=MSB, 0=LSB)]
        int[][] placements = {
            {row,     col,     7},  // bit 8
            {row,     col - 1, 6},  // bit 7
            {row,     col - 2, 5},  // bit 6
            {row - 1, col,     4},  // bit 5
            {row - 1, col - 1, 3},  // bit 4
            {row - 1, col - 2, 2},  // bit 3
            {row - 2, col,     1},  // bit 2
            {row - 2, col - 1, 0},  // bit 1
        };

        for (int[] p : placements) {
            int[] w = applyWrap(p[0], p[1], nRows, nCols);
            int r = w[0], c = w[1];
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
                grid[r][c] = ((codeword >> p[2]) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /**
     * Corner pattern 1 — triggered at top-left boundary.
     *
     * <p>Places an 8-bit codeword using absolute positions within the logical grid:
     * <pre>
     * bit 8: (0,       nCols-2)
     * bit 7: (0,       nCols-1)
     * bit 6: (1,       0)
     * bit 5: (2,       0)
     * bit 4: (nRows-2, 0)
     * bit 3: (nRows-1, 0)
     * bit 2: (nRows-1, 1)
     * bit 1: (nRows-1, 2)
     * </pre>
     */
    private static void placeCorner1(int codeword, int nRows, int nCols,
                                      boolean[][] grid, boolean[][] used) {
        int[][] positions = {
            {0,        nCols - 2, 7},
            {0,        nCols - 1, 6},
            {1,        0,         5},
            {2,        0,         4},
            {nRows - 2, 0,        3},
            {nRows - 1, 0,        2},
            {nRows - 1, 1,        1},
            {nRows - 1, 2,        0},
        };
        for (int[] p : positions) {
            int r = p[0], c = p[1];
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
                grid[r][c] = ((codeword >> p[2]) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /**
     * Corner pattern 2 — triggered at top-right boundary.
     *
     * <p>Absolute positions:
     * <pre>
     * bit 8: (0,       nCols-2)
     * bit 7: (0,       nCols-1)
     * bit 6: (1,       nCols-1)
     * bit 5: (2,       nCols-1)
     * bit 4: (nRows-1, 0)
     * bit 3: (nRows-1, 1)
     * bit 2: (nRows-1, 2)
     * bit 1: (nRows-1, 3)
     * </pre>
     */
    private static void placeCorner2(int codeword, int nRows, int nCols,
                                      boolean[][] grid, boolean[][] used) {
        int[][] positions = {
            {0,        nCols - 2, 7},
            {0,        nCols - 1, 6},
            {1,        nCols - 1, 5},
            {2,        nCols - 1, 4},
            {nRows - 1, 0,        3},
            {nRows - 1, 1,        2},
            {nRows - 1, 2,        1},
            {nRows - 1, 3,        0},
        };
        for (int[] p : positions) {
            int r = p[0], c = p[1];
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
                grid[r][c] = ((codeword >> p[2]) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /**
     * Corner pattern 3 — triggered at bottom-left boundary.
     *
     * <p>Absolute positions:
     * <pre>
     * bit 8: (0,       nCols-1)
     * bit 7: (1,       0)
     * bit 6: (2,       0)
     * bit 5: (nRows-2, 0)
     * bit 4: (nRows-1, 0)
     * bit 3: (nRows-1, 1)
     * bit 2: (nRows-1, 2)
     * bit 1: (nRows-1, 3)
     * </pre>
     */
    private static void placeCorner3(int codeword, int nRows, int nCols,
                                      boolean[][] grid, boolean[][] used) {
        int[][] positions = {
            {0,        nCols - 1, 7},
            {1,        0,         6},
            {2,        0,         5},
            {nRows - 2, 0,        4},
            {nRows - 1, 0,        3},
            {nRows - 1, 1,        2},
            {nRows - 1, 2,        1},
            {nRows - 1, 3,        0},
        };
        for (int[] p : positions) {
            int r = p[0], c = p[1];
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
                grid[r][c] = ((codeword >> p[2]) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /**
     * Corner pattern 4 — right-edge wrap for odd-dimension matrices.
     *
     * <p>Used when nRows and nCols are both odd (rectangular symbols and some
     * extended square sizes). Absolute positions:
     * <pre>
     * bit 8: (nRows-3, nCols-1)
     * bit 7: (nRows-2, nCols-1)
     * bit 6: (nRows-1, nCols-3)
     * bit 5: (nRows-1, nCols-2)
     * bit 4: (nRows-1, nCols-1)
     * bit 3: (0,       0)
     * bit 2: (1,       0)
     * bit 1: (2,       0)
     * </pre>
     */
    private static void placeCorner4(int codeword, int nRows, int nCols,
                                      boolean[][] grid, boolean[][] used) {
        int[][] positions = {
            {nRows - 3, nCols - 1, 7},
            {nRows - 2, nCols - 1, 6},
            {nRows - 1, nCols - 3, 5},
            {nRows - 1, nCols - 2, 4},
            {nRows - 1, nCols - 1, 3},
            {0,         0,         2},
            {1,         0,         1},
            {2,         0,         0},
        };
        for (int[] p : positions) {
            int r = p[0], c = p[1];
            if (r >= 0 && r < nRows && c >= 0 && c < nCols && !used[r][c]) {
                grid[r][c] = ((codeword >> p[2]) & 1) == 1;
                used[r][c] = true;
            }
        }
    }

    /**
     * Run the Utah diagonal placement algorithm on the logical data matrix.
     *
     * <p>This algorithm is the most distinctive part of Data Matrix encoding.
     * It was named "Utah" because the 8-module shape used to place each codeword
     * vaguely resembles the outline of the US state of Utah — a rectangle with
     * the top-left corner missing.
     *
     * <h3>How the diagonal walk works</h3>
     *
     * <p>The reference position (row, col) starts at (4, 0). For each position
     * the algorithm scans <em>upward-right</em> (row -= 2, col += 2) until it
     * hits a boundary, then <em>steps</em> (row += 1, col += 3) to the next
     * diagonal and scans <em>downward-left</em> (row += 2, col -= 2).
     *
     * <p>For each valid reference position, the 8 bits of the current codeword
     * are placed at the 8 offsets of the "Utah" shape relative to (row, col).
     *
     * <p>Four special corner patterns handle edge cases where the normal shape
     * would extend outside the grid boundary. These are triggered by specific
     * (row, col) reference positions.
     *
     * <h3>No masking</h3>
     *
     * <p>Unlike QR Code, Data Matrix does NOT apply any masking after placement.
     * The diagonal placement distributes bits naturally across the symbol without
     * needing the 8-pattern evaluation that QR requires.
     *
     * @param codewords  full interleaved codeword stream (data + ECC)
     * @param nRows      logical data matrix height (sum of all region heights)
     * @param nCols      logical data matrix width  (sum of all region widths)
     * @return           nRows × nCols boolean grid (true = dark module)
     */
    static boolean[][] utahPlacement(int[] codewords, int nRows, int nCols) {
        boolean[][] grid = new boolean[nRows][nCols];  // all false = light
        boolean[][] used = new boolean[nRows][nCols];  // tracks placed modules

        int cwIdx = 0;
        int row = 4;
        int col = 0;

        while (true) {
            // ── Corner special cases (triggered by specific reference positions)
            // Corner 1: fires when row == nRows and col == 0, for symbols where
            //           nRows mod 4 == 0 or nCols mod 4 == 0.
            if (row == nRows && col == 0
                    && (nRows % 4 == 0 || nCols % 4 == 0)
                    && cwIdx < codewords.length) {
                placeCorner1(codewords[cwIdx++], nRows, nCols, grid, used);
            }

            // Corner 2: fires when row == nRows-2 and col == 0 and nCols mod 4 != 0
            if (row == nRows - 2 && col == 0
                    && nCols % 4 != 0
                    && cwIdx < codewords.length) {
                placeCorner2(codewords[cwIdx++], nRows, nCols, grid, used);
            }

            // Corner 3: fires when row == nRows-2 and col == 0 and nCols mod 8 == 4
            if (row == nRows - 2 && col == 0
                    && nCols % 8 == 4
                    && cwIdx < codewords.length) {
                placeCorner3(codewords[cwIdx++], nRows, nCols, grid, used);
            }

            // Corner 4: fires when row == nRows+4 and col == 2 and nCols mod 8 == 0
            if (row == nRows + 4 && col == 2
                    && nCols % 8 == 0
                    && cwIdx < codewords.length) {
                placeCorner4(codewords[cwIdx++], nRows, nCols, grid, used);
            }

            // ── Standard diagonal traversal: scan upward-right (row-=2, col+=2)
            do {
                if (row >= 0 && row < nRows && col >= 0 && col < nCols
                        && !used[row][col] && cwIdx < codewords.length) {
                    placeUtah(codewords[cwIdx++], row, col, nRows, nCols, grid, used);
                }
                row -= 2;
                col += 2;
            } while (row >= 0 && col < nCols);

            // ── Step to next diagonal start position
            row += 1;
            col += 3;

            // ── Standard diagonal traversal: scan downward-left (row+=2, col-=2)
            do {
                if (row >= 0 && row < nRows && col >= 0 && col < nCols
                        && !used[row][col] && cwIdx < codewords.length) {
                    placeUtah(codewords[cwIdx++], row, col, nRows, nCols, grid, used);
                }
                row += 2;
                col -= 2;
            } while (row < nRows && col >= 0);

            // ── Step to next diagonal start position
            row += 3;
            col += 1;

            // ── Termination: all codewords placed, or reference fully past the grid
            if (row >= nRows && col >= nCols) break;
            if (cwIdx >= codewords.length) break;
        }

        // ── Fill any remaining unset modules with the "right and bottom fill" pattern.
        // Some symbol sizes have residual modules that the diagonal walk does not reach.
        // ISO/IEC 16022 §10 specifies: fill with (r + c) mod 2 == 1 (dark).
        for (int r = 0; r < nRows; r++) {
            for (int c = 0; c < nCols; c++) {
                if (!used[r][c]) {
                    grid[r][c] = (r + c) % 2 == 1;
                }
            }
        }

        return grid;
    }

    // =========================================================================
    // Logical → Physical coordinate mapping
    // =========================================================================

    /**
     * Map a logical data matrix coordinate to a physical symbol coordinate.
     *
     * <p>The logical data matrix is the concatenation of all data region interiors,
     * treated as a single flat grid. The Utah algorithm works entirely in this
     * logical space. After placement, we map back to the physical grid which
     * includes the outer border and alignment borders.
     *
     * <p>For a symbol with rr × rc data regions, each of size (rh × rw):
     * <pre>
     * physRow = (r / rh) × (rh + 2) + (r mod rh) + 1
     * physCol = (c / rw) × (rw + 2) + (c mod rw) + 1
     * </pre>
     *
     * <p>The {@code +2} accounts for the 2-module alignment border between regions.
     * The {@code +1} accounts for the 1-module outer border (finder + timing).
     *
     * <p>For single-region symbols (rr=rc=1) this simplifies to:
     * {@code physRow = r + 1}, {@code physCol = c + 1}.
     */
    private static int[] logicalToPhysical(int r, int c, SymbolSizeEntry e) {
        int rh = e.dataRegionHeight();
        int rw = e.dataRegionWidth();
        int physRow = (r / rh) * (rh + 2) + (r % rh) + 1;
        int physCol = (c / rw) * (rw + 2) + (c % rw) + 1;
        return new int[]{physRow, physCol};
    }

    // =========================================================================
    // Full encoding pipeline
    // =========================================================================

    /**
     * Encode a UTF-8 string into a Data Matrix ECC200 ModuleGrid.
     *
     * <p>The smallest symbol that fits the input is selected automatically.
     * For very long input, the 144×144 symbol accommodates up to 1556 ASCII chars
     * (or more with digit-pair packing for digit-heavy strings).
     *
     * <p>The result is a {@link ModuleGrid} where {@code true} = dark module
     * and {@code false} = light module. Pass it to a barcode-2d layout engine
     * to convert to pixel coordinates.
     *
     * <p>No masking is applied — Data Matrix ECC200 never masks. The Utah
     * diagonal placement distributes bits well enough without it.
     *
     * @param input   text to encode (UTF-8 bytes used for encoding)
     * @param options encoding options (shape preference etc.); may be null
     * @return        complete module grid ready for rendering
     * @throws InputTooLongException if input exceeds 144×144 capacity (1558 codewords)
     *
     * @example
     * <pre>{@code
     * ModuleGrid grid = DataMatrix.encode("Hello World", null);
     * // grid.rows == grid.cols == 16 for "Hello World"
     * }</pre>
     */
    public static ModuleGrid encode(String input, DataMatrixOptions options) {
        return encode(input.getBytes(StandardCharsets.UTF_8), options);
    }

    /**
     * Encode raw bytes into a Data Matrix ECC200 ModuleGrid.
     *
     * <p>Bytes with values 0–127 are encoded directly in ASCII mode.
     * Bytes 128–255 use UPPER_SHIFT (two codewords each).
     *
     * @param input   raw bytes to encode
     * @param options encoding options; may be null (defaults to SQUARE shape)
     * @return        complete module grid
     * @throws InputTooLongException if input exceeds 144×144 capacity
     */
    public static ModuleGrid encode(byte[] input, DataMatrixOptions options) {
        SymbolShape shape = (options != null) ? options.shape : SymbolShape.SQUARE;

        // Step 1: ASCII encode
        int[] codewords = encodeAscii(input);

        // Step 2: Symbol selection
        SymbolSizeEntry entry = selectSymbol(codewords.length, shape);

        // Step 3: Pad to capacity
        int[] padded = padCodewords(codewords, entry.dataCW());

        // Steps 4–6: RS ECC computation + interleaving
        int[] interleaved = computeInterleaved(padded, entry);

        // Step 7: Initialize physical grid (border + alignment borders)
        boolean[][] physGrid = initGrid(entry);

        // Step 8: Run Utah placement on the logical data matrix
        int nRows = entry.regionRows() * entry.dataRegionHeight();
        int nCols = entry.regionCols() * entry.dataRegionWidth();
        boolean[][] logicalGrid = utahPlacement(interleaved, nRows, nCols);

        // Step 9: Map logical → physical coordinates
        for (int r = 0; r < nRows; r++) {
            for (int c = 0; c < nCols; c++) {
                int[] phys = logicalToPhysical(r, c, entry);
                physGrid[phys[0]][phys[1]] = logicalGrid[r][c];
            }
        }

        // Step 10: Return ModuleGrid (no masking — Data Matrix never masks)
        // Convert boolean[][] to List<List<Boolean>> for ModuleGrid
        List<List<Boolean>> modules = new ArrayList<>();
        for (boolean[] row : physGrid) {
            List<Boolean> rowList = new ArrayList<>();
            for (boolean v : row) rowList.add(v);
            modules.add(Collections.unmodifiableList(rowList));
        }

        return new ModuleGrid(
                entry.symbolRows(),
                entry.symbolCols(),
                Collections.unmodifiableList(modules),
                ModuleShape.SQUARE
        );
    }

    // =========================================================================
    // Package-private accessors for testing
    // =========================================================================

    /** @return the GF(256)/0x12D exp table (package-private for tests). */
    static int[] gfExp() { return GF_EXP.clone(); }

    /** @return the GF(256)/0x12D log table (package-private for tests). */
    static int[] gfLog() { return GF_LOG.clone(); }
}
