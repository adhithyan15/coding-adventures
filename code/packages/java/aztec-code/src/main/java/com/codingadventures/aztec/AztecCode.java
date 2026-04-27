package com.codingadventures.aztec;

import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;

import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * Aztec Code encoder — ISO/IEC 24778:2008 compliant.
 *
 * <p>Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995 and
 * published as a patent-free format. Unlike QR Code (which uses three square
 * finder patterns at three corners), Aztec Code places a single
 * <strong>bullseye finder pattern at the center</strong> of the symbol. The
 * scanner finds the center first, then reads outward in a spiral — no large
 * quiet zone is needed.
 *
 * <h2>Where Aztec Code is used today</h2>
 * <ul>
 *   <li><strong>IATA boarding passes</strong> — the barcode on every airline
 *       boarding pass</li>
 *   <li><strong>Eurostar and Amtrak rail tickets</strong> — printed and
 *       on-screen tickets</li>
 *   <li><strong>PostNL, Deutsche Post, La Poste</strong> — European postal
 *       routing</li>
 *   <li><strong>US military ID cards</strong></li>
 * </ul>
 *
 * <h2>Symbol variants</h2>
 *
 * <pre>
 * Compact: 1-4 layers,  size = 11 + 4*layers  (15x15 to 27x27)
 * Full:    1-32 layers, size = 15 + 4*layers  (19x19 to 143x143)
 * </pre>
 *
 * <h2>Encoding pipeline (v0.1.0 — byte-mode only)</h2>
 *
 * <pre>
 * input string / bytes
 *   -&gt; Binary-Shift codewords from Upper mode
 *   -&gt; symbol size selection (smallest compact then full that fits at 23% ECC)
 *   -&gt; pad to exact codeword count
 *   -&gt; GF(256)/0x12D Reed-Solomon ECC (poly 0x12D, b=1 roots alpha^1..alpha^n)
 *   -&gt; bit stuffing (insert complement after 4 consecutive identical bits)
 *   -&gt; GF(16) mode message (layers + codeword count + 5 or 6 RS nibbles)
 *   -&gt; ModuleGrid  (bullseye -&gt; orientation marks -&gt; mode msg -&gt; data spiral)
 * </pre>
 *
 * <h2>v0.1.0 simplifications</h2>
 *
 * <ol>
 *   <li>Byte-mode only — all input encoded via Binary-Shift from Upper mode.
 *       Multi-mode (Digit/Upper/Lower/Mixed/Punct) optimization is v0.2.0.</li>
 *   <li>8-bit codewords -&gt; GF(256) RS (same polynomial as Data Matrix:
 *       0x12D). GF(16) and GF(32) RS for 4-bit/5-bit codewords are v0.2.0.</li>
 *   <li>Default ECC = 23%.</li>
 *   <li>Auto-select compact vs full (force-compact option is v0.2.0).</li>
 * </ol>
 *
 * <h2>Quick start</h2>
 *
 * <pre>{@code
 * // Encode a string, get back a boolean grid:
 * ModuleGrid grid = AztecCode.encode("HELLO WORLD");
 *
 * // Encode with explicit options:
 * AztecOptions opts = new AztecOptions(50);   // 50% ECC instead of 23%
 * ModuleGrid grid2  = AztecCode.encode("HELLO WORLD", opts);
 * }</pre>
 */
public final class AztecCode {

    /** Version string for this package. */
    public static final String VERSION = "0.1.0";

    /** Utility class — no instances. */
    private AztecCode() {}

    // =========================================================================
    // Public types
    // =========================================================================

    /**
     * Options for Aztec Code encoding.
     *
     * <p>{@code minEccPercent} is clamped to the range [10, 90] by
     * {@link #encode(String, AztecOptions)}. Higher values produce larger
     * symbols with more redundancy.
     */
    public static final class AztecOptions {

        /** Minimum error-correction percentage (default: 23, range 10-90). */
        public final int minEccPercent;

        /** Construct with the default 23% ECC level. */
        public AztecOptions() {
            this(23);
        }

        /**
         * Construct with an explicit ECC percentage.
         *
         * @param minEccPercent minimum error-correction percentage (10-90).
         */
        public AztecOptions(int minEccPercent) {
            this.minEccPercent = minEccPercent;
        }
    }

    /**
     * Base error class for Aztec Code failures.
     *
     * <p>This is a {@link RuntimeException}: encoder failures are programmer
     * errors (bad options, input too long) rather than recoverable I/O.
     */
    public static class AztecException extends RuntimeException {
        /** Construct an {@code AztecException} with the given message. */
        public AztecException(String message) {
            super(message);
        }
    }

    /** Thrown when the input is too long to fit in any 32-layer Aztec symbol. */
    public static final class InputTooLongException extends AztecException {
        /** Construct an {@code InputTooLongException} with the given message. */
        public InputTooLongException(String message) {
            super(message);
        }
    }

    // =========================================================================
    // GF(16) arithmetic — for mode-message Reed-Solomon
    // =========================================================================
    //
    // GF(16) is the finite field with 16 elements, built from the primitive
    // polynomial:
    //
    //   p(x) = x^4 + x + 1   (binary: 10011 = 0x13)
    //
    // Every non-zero element can be written as a power of the primitive
    // element alpha. alpha is the root of p(x), so alpha^4 = alpha + 1.
    //
    // The log table maps a field element (1..15) to its discrete log (0..14).
    // The antilog (exponentiation) table maps a log value to its element.
    //
    // alpha^0=1, alpha^1=2, alpha^2=4, alpha^3=8,
    // alpha^4=3, alpha^5=6, alpha^6=12, alpha^7=11,
    // alpha^8=5, alpha^9=10, alpha^10=7, alpha^11=14,
    // alpha^12=15, alpha^13=13, alpha^14=9, alpha^15=1 (period=15)

    /** GF(16) discrete logarithm: {@code LOG16[e] = i} means {@code alpha^i = e}. */
    static final int[] LOG16 = {
            -1, // log(0) = undefined
             0, // log(1)
             1, // log(2)
             4, // log(3)
             2, // log(4)
             8, // log(5)
             5, // log(6)
            10, // log(7)
             3, // log(8)
            14, // log(9)
             9, // log(10)
             7, // log(11)
             6, // log(12)
            13, // log(13)
            11, // log(14)
            12, // log(15)
    };

    /** GF(16) antilogarithm: {@code ALOG16[i] = alpha^i}. */
    static final int[] ALOG16 = {
            1, 2, 4, 8, 3, 6, 12, 11, 5, 10, 7, 14, 15, 13, 9, 1,
    };

    /**
     * Multiply two GF(16) elements.
     *
     * <p>Uses log/antilog: {@code a*b = ALOG16[(LOG16[a] + LOG16[b]) mod 15]}.
     * Returns 0 if either operand is 0.
     */
    static int gf16Mul(int a, int b) {
        if (a == 0 || b == 0) return 0;
        return ALOG16[(LOG16[a] + LOG16[b]) % 15];
    }

    /**
     * Build the GF(16) RS generator polynomial with roots
     * {@code alpha^1 .. alpha^n}.
     *
     * <p>Returns coefficients with {@code g[0]} = constant term and
     * {@code g[n]} = 1 (monic).
     */
    static int[] buildGf16Generator(int n) {
        int[] g = {1};
        for (int i = 1; i <= n; i++) {
            int ai = ALOG16[i % 15];
            int[] next = new int[g.length + 1];
            for (int j = 0; j < g.length; j++) {
                next[j + 1] ^= g[j];
                next[j] ^= gf16Mul(ai, g[j]);
            }
            g = next;
        }
        return g;
    }

    /**
     * Compute n GF(16) RS check nibbles for the given data nibbles.
     *
     * <p>Uses the LFSR polynomial-division algorithm: feed each data symbol
     * through a shift register whose taps come from the generator polynomial.
     */
    static int[] gf16RsEncode(int[] data, int n) {
        int[] g = buildGf16Generator(n);
        int[] rem = new int[n];
        for (int b : data) {
            int fb = b ^ rem[0];
            for (int i = 0; i < n - 1; i++) {
                rem[i] = rem[i + 1] ^ gf16Mul(g[i + 1], fb);
            }
            rem[n - 1] = gf16Mul(g[n], fb);
        }
        return rem;
    }

    // =========================================================================
    // GF(256)/0x12D arithmetic — for 8-bit data codewords
    // =========================================================================
    //
    // Aztec Code uses GF(256) with primitive polynomial:
    //   p(x) = x^8 + x^5 + x^4 + x^2 + x + 1  =  0x12D
    //
    // This is the SAME polynomial as Data Matrix ECC200, but DIFFERENT from
    // QR Code (0x11D). We implement it inline since the repo's gf256 package
    // uses 0x11D.
    //
    // Generator convention: b=1, roots alpha^1..alpha^n (MA02 style).

    /** Primitive polynomial used by Aztec Code RS over GF(256). */
    static final int GF256_POLY = 0x12d;

    /** {@code EXP_12D[i] = alpha^i} in GF(256)/0x12D, doubled for fast multiply. */
    static final int[] EXP_12D = new int[512];

    /** {@code LOG_12D[e]} = discrete log of {@code e} in GF(256)/0x12D. */
    static final int[] LOG_12D = new int[256];

    static {
        // Build tables at class-load time. The primitive element is alpha = 2.
        int x = 1;
        for (int i = 0; i < 255; i++) {
            EXP_12D[i] = x;
            EXP_12D[i + 255] = x;
            LOG_12D[x] = i;
            x <<= 1;
            if ((x & 0x100) != 0) x ^= GF256_POLY;
            x &= 0xff;
        }
        EXP_12D[255] = 1;
    }

    /**
     * Multiply two GF(256)/0x12D elements.
     *
     * <p>Uses log/antilog lookup with the doubled EXP table to avoid the
     * modulo-255 reduction.
     */
    static int gf256Mul(int a, int b) {
        if (a == 0 || b == 0) return 0;
        return EXP_12D[LOG_12D[a] + LOG_12D[b]];
    }

    /**
     * Build the GF(256)/0x12D RS generator polynomial with roots
     * {@code alpha^1 .. alpha^n}.
     *
     * <p>Returns big-endian coefficients (highest degree first), matching the
     * TypeScript reference implementation.
     */
    static int[] buildGf256Generator(int n) {
        int[] g = {1};
        for (int i = 1; i <= n; i++) {
            int ai = EXP_12D[i];
            int[] next = new int[g.length + 1];
            for (int j = 0; j < g.length; j++) {
                next[j] ^= g[j];
                next[j + 1] ^= gf256Mul(g[j], ai);
            }
            g = next;
        }
        return g;
    }

    /** Compute {@code nCheck} GF(256)/0x12D RS check bytes for the given data. */
    static int[] gf256RsEncode(int[] data, int nCheck) {
        int[] g = buildGf256Generator(nCheck);
        int n = g.length - 1;
        int[] rem = new int[n];
        for (int b : data) {
            int fb = b ^ rem[0];
            for (int i = 0; i < n - 1; i++) {
                rem[i] = rem[i + 1] ^ gf256Mul(g[i + 1], fb);
            }
            rem[n - 1] = gf256Mul(g[n], fb);
        }
        return rem;
    }

    // =========================================================================
    // Aztec Code capacity tables
    // =========================================================================
    //
    // Derived from ISO/IEC 24778:2008 Table 1.
    //
    // Each entry is (totalBits, maxBytes8):
    //   totalBits — total data+ECC bit positions in the spiral.
    //   maxBytes8 — maximum number of 8-bit codewords (bytes).

    /** Compact (1-4 layer) capacity. Index 0 unused. */
    static final int[][] COMPACT_CAPACITY = {
            {0,   0 }, // index 0 unused
            {72,  9 }, // 1 layer, 15x15
            {200, 25}, // 2 layers, 19x19
            {392, 49}, // 3 layers, 23x23
            {648, 81}, // 4 layers, 27x27
    };

    /** Full (1-32 layer) capacity. Index 0 unused. */
    static final int[][] FULL_CAPACITY = {
            {0,    0   }, // index 0 unused
            {88,   11  }, // 1 layer
            {216,  27  }, // 2 layers
            {360,  45  }, // 3 layers
            {520,  65  }, // 4 layers
            {696,  87  }, // 5 layers
            {888,  111 }, // 6 layers
            {1096, 137 }, // 7 layers
            {1320, 165 }, // 8 layers
            {1560, 195 }, // 9 layers
            {1816, 227 }, // 10 layers
            {2088, 261 }, // 11 layers
            {2376, 297 }, // 12 layers
            {2680, 335 }, // 13 layers
            {3000, 375 }, // 14 layers
            {3336, 417 }, // 15 layers
            {3688, 461 }, // 16 layers
            {4056, 507 }, // 17 layers
            {4440, 555 }, // 18 layers
            {4840, 605 }, // 19 layers
            {5256, 657 }, // 20 layers
            {5688, 711 }, // 21 layers
            {6136, 767 }, // 22 layers
            {6600, 825 }, // 23 layers
            {7080, 885 }, // 24 layers
            {7576, 947 }, // 25 layers
            {8088, 1011}, // 26 layers
            {8616, 1077}, // 27 layers
            {9160, 1145}, // 28 layers
            {9720, 1215}, // 29 layers
            {10296, 1287}, // 30 layers
            {10888, 1361}, // 31 layers
            {11496, 1437}, // 32 layers
    };

    // =========================================================================
    // Data encoding — Binary-Shift from Upper mode (v0.1.0 byte-mode path)
    // =========================================================================
    //
    // All input is wrapped in a single Binary-Shift block from Upper mode:
    //   1. Emit 5 bits = 0b11111 (Binary-Shift escape in Upper mode)
    //   2. If len <= 31: 5 bits for length
    //      If len > 31:  5 bits = 0b00000, then 11 bits for length
    //   3. Each byte as 8 bits, MSB first

    /**
     * Encode input bytes as a flat bit array using the Binary-Shift escape.
     *
     * <p>Returns a list of 0/1 values, MSB first.
     */
    static List<Integer> encodeBytesAsBits(byte[] input) {
        // The long-length escape uses an 11-bit field (max 2047). Guard here
        // so that oversized inputs fail loudly rather than silently truncating
        // the length field and producing a correctly-structured but corrupted symbol.
        if (input.length > 2047) {
            throw new InputTooLongException(
                "Binary-Shift byte count " + input.length + " exceeds 11-bit field max (2047 bytes).");
        }

        List<Integer> bits = new ArrayList<>();
        writeBits(bits, 31, 5); // Binary-Shift escape

        int len = input.length;
        if (len <= 31) {
            writeBits(bits, len, 5);
        } else {
            writeBits(bits, 0, 5);
            writeBits(bits, len, 11);
        }

        for (byte b : input) {
            writeBits(bits, b & 0xff, 8);
        }
        return bits;
    }

    /** Append {@code count} bits of {@code value} (MSB first) to {@code bits}. */
    private static void writeBits(List<Integer> bits, int value, int count) {
        for (int i = count - 1; i >= 0; i--) {
            bits.add((value >> i) & 1);
        }
    }

    // =========================================================================
    // Symbol-size selection
    // =========================================================================

    /** Picked symbol description: compact-or-full + layer count + cw counts. */
    static final class SymbolSpec {
        final boolean compact;
        final int layers;
        final int dataCwCount;
        final int eccCwCount;
        final int totalBits;

        SymbolSpec(boolean compact, int layers, int dataCwCount, int eccCwCount, int totalBits) {
            this.compact = compact;
            this.layers = layers;
            this.dataCwCount = dataCwCount;
            this.eccCwCount = eccCwCount;
            this.totalBits = totalBits;
        }
    }

    /**
     * Select the smallest symbol that can hold {@code dataBitCount} bits at
     * {@code minEccPct}.
     *
     * <p>Tries compact 1-4, then full 1-32. Adds 20% conservative stuffing
     * overhead since we cannot know exactly how many stuff bits we will need
     * until we have the final RS-encoded stream.
     *
     * @throws InputTooLongException if no symbol fits.
     */
    static SymbolSpec selectSymbol(int dataBitCount, int minEccPct) {
        int stuffedBitCount = (int) Math.ceil(dataBitCount * 1.2);

        for (int layers = 1; layers <= 4; layers++) {
            int[] cap = COMPACT_CAPACITY[layers];
            int totalBytes = cap[1];
            int eccCwCount = (int) Math.ceil((minEccPct / 100.0) * totalBytes);
            int dataCwCount = totalBytes - eccCwCount;
            if (dataCwCount <= 0) continue;
            if ((int) Math.ceil(stuffedBitCount / 8.0) <= dataCwCount) {
                return new SymbolSpec(true, layers, dataCwCount, eccCwCount, cap[0]);
            }
        }

        for (int layers = 1; layers <= 32; layers++) {
            int[] cap = FULL_CAPACITY[layers];
            int totalBytes = cap[1];
            int eccCwCount = (int) Math.ceil((minEccPct / 100.0) * totalBytes);
            int dataCwCount = totalBytes - eccCwCount;
            if (dataCwCount <= 0) continue;
            if ((int) Math.ceil(stuffedBitCount / 8.0) <= dataCwCount) {
                return new SymbolSpec(false, layers, dataCwCount, eccCwCount, cap[0]);
            }
        }

        throw new InputTooLongException(
                "Input is too long to fit in any Aztec Code symbol (" +
                        dataBitCount + " bits needed)");
    }

    // =========================================================================
    // Padding
    // =========================================================================

    /**
     * Pad {@code bits} with zeros up to a multiple of 8, then up to
     * {@code targetBytes * 8} bits, and truncate at exactly that length.
     */
    static List<Integer> padToBytes(List<Integer> bits, int targetBytes) {
        List<Integer> out = new ArrayList<>(bits);
        while (out.size() % 8 != 0) out.add(0);
        while (out.size() < targetBytes * 8) out.add(0);
        return out.subList(0, targetBytes * 8);
    }

    // =========================================================================
    // Bit stuffing
    // =========================================================================
    //
    // After every 4 consecutive identical bits (all 0 or all 1), insert one
    // complement bit. Applies only to the data+ECC bit stream.
    //
    // Example:
    //   Input:  1 1 1 1 0 0 0 0
    //   After 4 ones:  insert 0  -> [1,1,1,1,0]
    //   After 4 zeros: insert 1  -> [1,1,1,1,0, 0,0,0,1,0]

    /**
     * Apply Aztec bit stuffing to the data+ECC bit stream.
     *
     * <p>Inserts a complement bit after every run of 4 identical bits.
     */
    static List<Integer> stuffBits(List<Integer> bits) {
        List<Integer> stuffed = new ArrayList<>(bits.size() + bits.size() / 4 + 1);
        int runVal = -1;
        int runLen = 0;

        for (int bit : bits) {
            if (bit == runVal) {
                runLen++;
            } else {
                runVal = bit;
                runLen = 1;
            }
            stuffed.add(bit);

            if (runLen == 4) {
                int stuffBit = 1 - bit;
                stuffed.add(stuffBit);
                runVal = stuffBit;
                runLen = 1;
            }
        }
        return stuffed;
    }

    // =========================================================================
    // Mode message encoding
    // =========================================================================
    //
    // The mode message encodes layer count and data codeword count, protected
    // by GF(16) RS.
    //
    // Compact (28 bits = 7 nibbles):
    //   m = ((layers-1) << 6) | (dataCwCount-1)
    //   2 data nibbles + 5 ECC nibbles
    //
    // Full (40 bits = 10 nibbles):
    //   m = ((layers-1) << 11) | (dataCwCount-1)
    //   4 data nibbles + 6 ECC nibbles

    /**
     * Encode the mode message as a flat bit list (28 bits compact, 40 full).
     */
    static List<Integer> encodeModeMessage(boolean compact, int layers, int dataCwCount) {
        int[] dataNibbles;
        int numEcc;

        if (compact) {
            int m = ((layers - 1) << 6) | (dataCwCount - 1);
            dataNibbles = new int[] {m & 0xf, (m >> 4) & 0xf};
            numEcc = 5;
        } else {
            int m = ((layers - 1) << 11) | (dataCwCount - 1);
            dataNibbles = new int[] {
                    m & 0xf, (m >> 4) & 0xf, (m >> 8) & 0xf, (m >> 12) & 0xf
            };
            numEcc = 6;
        }

        int[] eccNibbles = gf16RsEncode(dataNibbles, numEcc);
        int[] allNibbles = new int[dataNibbles.length + eccNibbles.length];
        System.arraycopy(dataNibbles, 0, allNibbles, 0, dataNibbles.length);
        System.arraycopy(eccNibbles, 0, allNibbles, dataNibbles.length, eccNibbles.length);

        List<Integer> bits = new ArrayList<>(allNibbles.length * 4);
        for (int nibble : allNibbles) {
            for (int i = 3; i >= 0; i--) {
                bits.add((nibble >> i) & 1);
            }
        }
        return bits;
    }

    // =========================================================================
    // Grid construction
    // =========================================================================

    /** Symbol size: compact = 11+4*layers, full = 15+4*layers. */
    static int symbolSize(boolean compact, int layers) {
        return compact ? 11 + 4 * layers : 15 + 4 * layers;
    }

    /** Bullseye radius: compact = 5, full = 7. */
    static int bullseyeRadius(boolean compact) {
        return compact ? 5 : 7;
    }

    /**
     * Draw the bullseye finder pattern.
     *
     * <p>Color at Chebyshev distance {@code d} from center:
     * <ul>
     *   <li>{@code d <= 1}: DARK (solid 3x3 inner core)</li>
     *   <li>{@code d > 1, d even}: LIGHT</li>
     *   <li>{@code d > 1, d odd}:  DARK</li>
     * </ul>
     */
    static void drawBullseye(boolean[][] modules, boolean[][] reserved,
                             int cx, int cy, boolean compact) {
        int br = bullseyeRadius(compact);
        for (int row = cy - br; row <= cy + br; row++) {
            for (int col = cx - br; col <= cx + br; col++) {
                int d = Math.max(Math.abs(col - cx), Math.abs(row - cy));
                boolean dark = (d <= 1) || (d % 2 == 1);
                modules[row][col] = dark;
                reserved[row][col] = true;
            }
        }
    }

    /**
     * Draw reference grid for full Aztec symbols.
     *
     * <p>Grid lines at rows/cols that are multiples of 16 from center. Module
     * value alternates dark/light from center.
     */
    static void drawReferenceGrid(boolean[][] modules, boolean[][] reserved,
                                  int cx, int cy, int size) {
        for (int row = 0; row < size; row++) {
            for (int col = 0; col < size; col++) {
                boolean onH = ((cy - row) % 16) == 0;
                boolean onV = ((cx - col) % 16) == 0;
                if (!onH && !onV) continue;

                boolean dark;
                if (onH && onV) {
                    dark = true;
                } else if (onH) {
                    dark = ((cx - col) % 2) == 0;
                } else {
                    dark = ((cy - row) % 2) == 0;
                }

                modules[row][col] = dark;
                reserved[row][col] = true;
            }
        }
    }

    /**
     * Place orientation marks and mode-message bits.
     *
     * <p>The mode-message ring is the perimeter at Chebyshev radius
     * {@code bullseyeRadius+1}. The 4 corners are orientation marks (DARK).
     * The remaining non-corner positions carry mode-message bits clockwise
     * from TL+1.
     *
     * <p>Returns positions in the ring after the mode-message bits, for
     * subsequent data placement.
     */
    static List<int[]> drawOrientationAndModeMessage(boolean[][] modules,
                                                     boolean[][] reserved,
                                                     int cx, int cy,
                                                     boolean compact,
                                                     List<Integer> modeMessageBits) {
        int r = bullseyeRadius(compact) + 1;

        // Enumerate non-corner perimeter positions clockwise from TL+1.
        // Each entry is {col, row}.
        List<int[]> nonCorner = new ArrayList<>();

        // Top edge (skip both corners)
        for (int col = cx - r + 1; col <= cx + r - 1; col++) {
            nonCorner.add(new int[] {col, cy - r});
        }
        // Right edge (skip both corners)
        for (int row = cy - r + 1; row <= cy + r - 1; row++) {
            nonCorner.add(new int[] {cx + r, row});
        }
        // Bottom edge: right to left (skip both corners)
        for (int col = cx + r - 1; col >= cx - r + 1; col--) {
            nonCorner.add(new int[] {col, cy + r});
        }
        // Left edge: bottom to top (skip both corners)
        for (int row = cy + r - 1; row >= cy - r + 1; row--) {
            nonCorner.add(new int[] {cx - r, row});
        }

        // Place 4 orientation mark corners as DARK
        int[][] corners = {
                {cx - r, cy - r},
                {cx + r, cy - r},
                {cx + r, cy + r},
                {cx - r, cy + r},
        };
        for (int[] corner : corners) {
            modules[corner[1]][corner[0]] = true;
            reserved[corner[1]][corner[0]] = true;
        }

        // Place mode message bits
        int n = Math.min(modeMessageBits.size(), nonCorner.size());
        for (int i = 0; i < n; i++) {
            int[] pos = nonCorner.get(i);
            modules[pos[1]][pos[0]] = modeMessageBits.get(i) == 1;
            reserved[pos[1]][pos[0]] = true;
        }

        // Return remaining positions for data bits
        List<int[]> remaining = new ArrayList<>();
        for (int i = modeMessageBits.size(); i < nonCorner.size(); i++) {
            remaining.add(nonCorner.get(i));
        }
        return remaining;
    }

    // =========================================================================
    // Data layer spiral placement
    // =========================================================================
    //
    // Bits are placed in a clockwise spiral starting from the innermost data
    // layer. Each layer band is 2 modules wide. Pairs: outer row/col first,
    // then inner.
    //
    // For compact: d_inner of first layer = bullseyeRadius + 2 = 7
    // For full:    d_inner of first layer = bullseyeRadius + 2 = 9

    /**
     * Place all data bits using the clockwise layer spiral.
     *
     * <p>Fills the mode-ring remaining positions first, then spirals outward.
     */
    static void placeDataBits(boolean[][] modules, boolean[][] reserved,
                              List<Integer> bits, int cx, int cy,
                              boolean compact, int layers,
                              List<int[]> modeRingRemainingPositions) {
        int size = modules.length;
        int[] bitIndex = {0};

        // Fill remaining mode ring positions first
        for (int[] pos : modeRingRemainingPositions) {
            int col = pos[0], row = pos[1];
            int bit = bitIndex[0] < bits.size() ? bits.get(bitIndex[0]) : 0;
            modules[row][col] = bit == 1;
            bitIndex[0]++;
        }

        // Spiral through data layers
        int br = bullseyeRadius(compact);
        int dStart = br + 2; // mode msg ring at br+1, first data layer at br+2

        for (int L = 0; L < layers; L++) {
            int dI = dStart + 2 * L; // inner radius
            int dO = dI + 1;          // outer radius

            // Top edge: left to right
            for (int col = cx - dI + 1; col <= cx + dI; col++) {
                placeBit(modules, reserved, bits, bitIndex, col, cy - dO, size);
                placeBit(modules, reserved, bits, bitIndex, col, cy - dI, size);
            }
            // Right edge: top to bottom
            for (int row = cy - dI + 1; row <= cy + dI; row++) {
                placeBit(modules, reserved, bits, bitIndex, cx + dO, row, size);
                placeBit(modules, reserved, bits, bitIndex, cx + dI, row, size);
            }
            // Bottom edge: right to left
            for (int col = cx + dI; col >= cx - dI + 1; col--) {
                placeBit(modules, reserved, bits, bitIndex, col, cy + dO, size);
                placeBit(modules, reserved, bits, bitIndex, col, cy + dI, size);
            }
            // Left edge: bottom to top
            for (int row = cy + dI; row >= cy - dI + 1; row--) {
                placeBit(modules, reserved, bits, bitIndex, cx - dO, row, size);
                placeBit(modules, reserved, bits, bitIndex, cx - dI, row, size);
            }
        }
    }

    private static void placeBit(boolean[][] modules, boolean[][] reserved,
                                 List<Integer> bits, int[] bitIndex,
                                 int col, int row, int size) {
        if (row < 0 || row >= size || col < 0 || col >= size) return;
        if (reserved[row][col]) return;
        int bit = bitIndex[0] < bits.size() ? bits.get(bitIndex[0]) : 0;
        modules[row][col] = bit == 1;
        bitIndex[0]++;
    }

    // =========================================================================
    // Public encode entrypoints
    // =========================================================================

    /**
     * Encode a string with the default 23% ECC level.
     *
     * @param data input string (will be UTF-8 encoded).
     * @return abstract module grid; {@code true} = dark module.
     * @throws InputTooLongException if the input cannot fit in any symbol.
     */
    public static ModuleGrid encode(String data) {
        return encode(data, new AztecOptions());
    }

    /**
     * Encode a string with explicit options.
     *
     * @param data input string (will be UTF-8 encoded).
     * @param options encoding options ({@code minEccPercent} clamped to 10-90).
     * @return abstract module grid; {@code true} = dark module.
     * @throws InputTooLongException if the input cannot fit in any symbol.
     */
    public static ModuleGrid encode(String data, AztecOptions options) {
        return encode(data == null ? new byte[0] : data.getBytes(StandardCharsets.UTF_8), options);
    }

    /**
     * Encode raw bytes with the default 23% ECC level.
     */
    public static ModuleGrid encode(byte[] data) {
        return encode(data, new AztecOptions());
    }

    /**
     * Encode raw bytes with explicit options.
     *
     * <p>This is the canonical entry point — the {@code String} overloads
     * delegate here after UTF-8 encoding.
     */
    public static ModuleGrid encode(byte[] data, AztecOptions options) {
        if (data == null) data = new byte[0];
        AztecOptions opts = options == null ? new AztecOptions() : options;

        // Clamp the ECC percentage to the spec-recommended range.
        int minEccPct = Math.max(10, Math.min(90, opts.minEccPercent));

        // Step 1: encode data
        List<Integer> dataBits = encodeBytesAsBits(data);

        // Step 2: select symbol
        SymbolSpec spec = selectSymbol(dataBits.size(), minEccPct);
        boolean compact = spec.compact;
        int layers = spec.layers;
        int dataCwCount = spec.dataCwCount;
        int eccCwCount = spec.eccCwCount;

        // Step 3: pad to dataCwCount bytes
        List<Integer> paddedBits = padToBytes(dataBits, dataCwCount);

        int[] dataBytes = new int[dataCwCount];
        for (int i = 0; i < dataCwCount; i++) {
            int b = 0;
            for (int k = 0; k < 8; k++) {
                b = (b << 1) | paddedBits.get(i * 8 + k);
            }
            // All-zero codeword avoidance: last codeword 0x00 -> 0xFF
            if (b == 0 && i == dataCwCount - 1) b = 0xff;
            dataBytes[i] = b;
        }

        // Step 4: compute RS ECC
        int[] eccBytes = gf256RsEncode(dataBytes, eccCwCount);

        // Step 5: build bit stream + stuff
        int[] allBytes = new int[dataBytes.length + eccBytes.length];
        System.arraycopy(dataBytes, 0, allBytes, 0, dataBytes.length);
        System.arraycopy(eccBytes, 0, allBytes, dataBytes.length, eccBytes.length);

        List<Integer> rawBits = new ArrayList<>(allBytes.length * 8);
        for (int b : allBytes) {
            for (int i = 7; i >= 0; i--) {
                rawBits.add((b >> i) & 1);
            }
        }
        List<Integer> stuffedBits = stuffBits(rawBits);

        // Step 6: mode message
        List<Integer> modeMsg = encodeModeMessage(compact, layers, dataCwCount);

        // Step 7: initialize grid
        int size = symbolSize(compact, layers);
        int cx = size / 2;
        int cy = size / 2;

        boolean[][] modules = new boolean[size][size];
        boolean[][] reserved = new boolean[size][size];

        // Reference grid first (full only), then bullseye overwrites
        if (!compact) {
            drawReferenceGrid(modules, reserved, cx, cy, size);
        }
        drawBullseye(modules, reserved, cx, cy, compact);

        List<int[]> modeRingRemainingPositions = drawOrientationAndModeMessage(
                modules, reserved, cx, cy, compact, modeMsg);

        // Step 8: place data spiral
        placeDataBits(modules, reserved, stuffedBits, cx, cy, compact, layers,
                modeRingRemainingPositions);

        // Convert mutable boolean[][] -> ModuleGrid (immutable)
        List<List<Boolean>> rows = new ArrayList<>(size);
        for (int r = 0; r < size; r++) {
            List<Boolean> row = new ArrayList<>(size);
            for (int c = 0; c < size; c++) {
                row.add(modules[r][c]);
            }
            rows.add(row);
        }
        return new ModuleGrid(size, size, rows, ModuleShape.SQUARE);
    }

    // =========================================================================
    // Test helpers — package-private accessors used by AztecCodeTest
    // =========================================================================
    //
    // These exist so the test class (in the same package) can verify field
    // arithmetic and intermediate stages without making the underlying tables
    // and helpers part of the public API.

    static int[] gf256ExpTable() {
        return Arrays.copyOf(EXP_12D, EXP_12D.length);
    }

    static int[] gf256LogTable() {
        return Arrays.copyOf(LOG_12D, LOG_12D.length);
    }
}
