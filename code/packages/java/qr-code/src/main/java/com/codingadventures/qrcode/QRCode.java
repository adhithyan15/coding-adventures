package com.codingadventures.qrcode;

import com.codingadventures.barcode2d.Barcode2D;
import com.codingadventures.barcode2d.Barcode2DLayoutConfig;
import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;
import com.codingadventures.gf256.GF256;
import com.codingadventures.paintinstructions.PaintScene;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/**
 * QR Code Encoder — ISO/IEC 18004:2015 compliant.
 *
 * <p>Encodes any UTF-8 string into a scannable QR Code and returns a
 * {@link ModuleGrid} (abstract boolean grid) that can be passed to
 * {@code barcode-2d}'s {@link Barcode2D#layout} for pixel rendering.
 *
 * <h2>Encoding Pipeline</h2>
 *
 * <pre>
 * input string
 *   → mode selection    (numeric / alphanumeric / byte)
 *   → version selection (smallest v1–40 that fits at the ECC level)
 *   → bit stream        (mode indicator + char count + data + padding)
 *   → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
 *   → interleave        (data CWs round-robin, then ECC CWs)
 *   → grid init         (finder × 3, separators, timing, alignment, format, dark)
 *   → zigzag placement  (two-column snake from bottom-right)
 *   → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
 *   → finalize          (format info + version info v7+)
 *   → ModuleGrid
 * </pre>
 *
 * <h2>Standards References</h2>
 * <ul>
 *   <li>ISO/IEC 18004:2015 — the primary QR Code specification</li>
 *   <li>Thonky QR Code Tutorial (thonky.com/qr-code-tutorial) — excellent worked examples</li>
 *   <li>Nayuki QR Code generator (nayuki.io) — canonical open-source reference (MIT)</li>
 * </ul>
 *
 * <h2>Quick Start</h2>
 * <pre>{@code
 * // Encode a string, get back a boolean grid:
 * ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
 *
 * // Convert to a PaintScene for SVG/pixel rendering:
 * Barcode2DLayoutConfig config = Barcode2DLayoutConfig.defaults();
 * PaintScene scene = QRCode.encodeAndLayout("HELLO WORLD", QRCode.EccLevel.M, config);
 * }</pre>
 */
public final class QRCode {

    /** Version string for this package. */
    public static final String VERSION = "0.1.0";

    // Private constructor: this class is purely a namespace for static utilities.
    private QRCode() {}

    // =========================================================================
    // Public types
    // =========================================================================

    /**
     * Error-correction level for the QR Code.
     *
     * <p>Higher levels recover more corrupted modules but require larger symbols
     * (more modules) to fit the same data.
     *
     * <table border="1">
     * <tr><th>Level</th><th>Approx. recovery</th><th>Use case</th></tr>
     * <tr><td>L</td><td>~7%</td><td>Clean environments, maximize data density</td></tr>
     * <tr><td>M</td><td>~15%</td><td>General purpose (default recommendation)</td></tr>
     * <tr><td>Q</td><td>~25%</td><td>Industrial printing with some damage expected</td></tr>
     * <tr><td>H</td><td>~30%</td><td>Maximum redundancy, logos/overlays</td></tr>
     * </table>
     */
    public enum EccLevel {
        /** ~7% of codewords recoverable. */
        L,
        /** ~15% of codewords recoverable (common default). */
        M,
        /** ~25% of codewords recoverable. */
        Q,
        /** ~30% of codewords recoverable. */
        H
    }

    /**
     * Checked exception thrown when the input cannot be encoded.
     *
     * <p>Currently thrown when the input string is too long to fit in any
     * QR Code version at the requested ECC level.
     */
    public static final class QRCodeException extends Exception {
        /** Constructs a {@code QRCodeException} with the given message. */
        public QRCodeException(String message) {
            super(message);
        }
    }

    // =========================================================================
    // ECC level constants
    // =========================================================================

    /**
     * Returns the 2-bit ISO format-information ECC indicator for a given level.
     *
     * <p>The QR Code format-information word encodes the ECC level in bits 13-14.
     * ISO 18004 Table 12 assigns these indicator values — note M=00 is not a typo.
     *
     * <pre>
     *   L = 01  (binary)
     *   M = 00
     *   Q = 11
     *   H = 10
     * </pre>
     */
    private static int eccIndicator(EccLevel ecc) {
        return switch (ecc) {
            case L -> 0b01;
            case M -> 0b00;
            case Q -> 0b11;
            case H -> 0b10;
        };
    }

    /**
     * Returns the array index for ECC tables: L=0, M=1, Q=2, H=3.
     */
    private static int eccIdx(EccLevel ecc) {
        return switch (ecc) {
            case L -> 0;
            case M -> 1;
            case Q -> 2;
            case H -> 3;
        };
    }

    // =========================================================================
    // ISO 18004:2015 — Capacity tables (Table 9)
    // =========================================================================

    /**
     * ECC codewords per block, indexed [eccIdx][version].
     *
     * <p>Each block is independently Reed-Solomon encoded. The number of ECC
     * codewords per block determines the correction capability: t = ecc_per_block / 2
     * errors can be corrected (or twice as many erasures).
     *
     * <p>Index 0 is a sentinel (-1) because QR versions start at 1.
     */
    private static final int[][] ECC_CODEWORDS_PER_BLOCK = {
        // L:
        {-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22,
              24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30,
              30, 30, 30, 30, 30, 30, 30, 30, 30, 30},
        // M:
        {-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24,
              28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28,
              28, 28, 28, 28, 28, 28, 28, 28, 28, 28},
        // Q:
        {-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30,
              24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30,
              30, 30, 30, 30, 30, 30, 30, 30, 30, 30},
        // H:
        {-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24,
              30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30,
              30, 30, 30, 30, 30, 30, 30, 30, 30, 30},
    };

    /**
     * Number of error-correction blocks, indexed [eccIdx][version].
     *
     * <p>The total codeword stream is divided into this many independent blocks.
     * Larger versions and higher ECC levels require more blocks to keep each
     * individual block from being too long (long polynomials are harder to decode).
     */
    private static final int[][] NUM_BLOCKS = {
        // L:
        {-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,
               6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15,
              16, 17, 18, 19, 19, 20, 21, 22, 24, 25},
        // M:
        {-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10,
              10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29,
              31, 33, 35, 37, 38, 40, 43, 45, 47, 49},
        // Q:
        {-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12,
              17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40,
              43, 45, 48, 51, 53, 56, 59, 62, 65, 68},
        // H:
        {-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18,
              16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48,
              51, 54, 57, 60, 63, 66, 70, 74, 77, 80},
    };

    /**
     * Alignment pattern center coordinates, indexed by version - 1.
     *
     * <p>Alignment patterns are 5×5 square patterns placed at the intersections
     * of these row/column coordinates (all combinations, except those occupied
     * by finder patterns). They help scanners compensate for image distortion.
     *
     * <p>Version 1 has no alignment patterns. Version 2 has one at (18,18).
     * Version 40 has a 7×7 grid of them.
     */
    private static final int[][] ALIGNMENT_POSITIONS = {
        {},                               // v1
        {6, 18},                          // v2
        {6, 22},                          // v3
        {6, 26},                          // v4
        {6, 30},                          // v5
        {6, 34},                          // v6
        {6, 22, 38},                      // v7
        {6, 24, 42},                      // v8
        {6, 26, 46},                      // v9
        {6, 28, 50},                      // v10
        {6, 30, 54},                      // v11
        {6, 32, 58},                      // v12
        {6, 34, 62},                      // v13
        {6, 26, 46, 66},                  // v14
        {6, 26, 48, 70},                  // v15
        {6, 26, 50, 74},                  // v16
        {6, 30, 54, 78},                  // v17
        {6, 30, 56, 82},                  // v18
        {6, 30, 58, 86},                  // v19
        {6, 34, 62, 90},                  // v20
        {6, 28, 50, 72, 94},              // v21
        {6, 26, 50, 74, 98},              // v22
        {6, 30, 54, 78, 102},             // v23
        {6, 28, 54, 80, 106},             // v24
        {6, 32, 58, 84, 110},             // v25
        {6, 30, 58, 86, 114},             // v26
        {6, 34, 62, 90, 118},             // v27
        {6, 26, 50, 74,  98, 122},        // v28
        {6, 30, 54, 78, 102, 126},        // v29
        {6, 26, 52, 78, 104, 130},        // v30
        {6, 30, 56, 82, 108, 134},        // v31
        {6, 34, 60, 86, 112, 138},        // v32
        {6, 30, 58, 86, 114, 142},        // v33
        {6, 34, 62, 90, 118, 146},        // v34
        {6, 30, 54, 78, 102, 126, 150},   // v35
        {6, 24, 50, 76, 102, 128, 154},   // v36
        {6, 28, 54, 80, 106, 132, 158},   // v37
        {6, 32, 58, 84, 110, 136, 162},   // v38
        {6, 26, 54, 82, 110, 138, 166},   // v39
        {6, 30, 58, 86, 114, 142, 170},   // v40
    };

    // =========================================================================
    // Grid geometry
    // =========================================================================

    /**
     * Returns the side length in modules of a QR Code symbol.
     *
     * <p>Formula: {@code 4V + 17} where V is the version number (1–40).
     * Version 1 → 21×21, version 2 → 25×25, version 40 → 177×177.
     *
     * <p>The "+17" accounts for the three 7-module finders, their separators,
     * and two timing strips.
     */
    private static int symbolSize(int version) {
        return 4 * version + 17;
    }

    /**
     * Total number of raw data modules (data + ECC bits combined) in the symbol.
     *
     * <p>Formula from Nayuki's reference implementation (public domain).
     * Subtracts the non-data functional modules: finders (3×49 + separators),
     * timing strips, alignment patterns, format info, dark module,
     * and (for v7+) version info areas.
     *
     * @param version QR Code version, 1–40
     * @return total number of modules available for data + ECC bits
     */
    private static int numRawDataModules(int version) {
        long v = version;
        long result = (16L * v + 128L) * v + 64L;
        if (version >= 2) {
            long numAlign = (v / 7) + 2;
            result -= (25L * numAlign - 10L) * numAlign - 55L;
            if (version >= 7) {
                result -= 36L;
            }
        }
        return (int) result;
    }

    /**
     * Returns the number of data codewords (bytes) available for user data.
     *
     * <p>Computed as: total raw codewords − ECC codewords.
     *
     * @param version 1–40
     * @param ecc ECC level
     * @return data codeword count for this version and ECC level
     */
    private static int numDataCodewords(int version, EccLevel ecc) {
        int e = eccIdx(ecc);
        int rawCw = numRawDataModules(version) / 8;
        int eccCw = NUM_BLOCKS[e][version] * ECC_CODEWORDS_PER_BLOCK[e][version];
        return rawCw - eccCw;
    }

    /**
     * Returns the number of remainder bits to append after all codewords.
     *
     * <p>The raw module count may not be an exact multiple of 8; the leftover
     * modules after placing all codewords are filled with 0-bits.
     * Most versions have 0 remainder bits; v14–20 have 3; v21–27 have 4; etc.
     */
    private static int numRemainderBits(int version) {
        return numRawDataModules(version) % 8;
    }

    // =========================================================================
    // Reed-Solomon (b=0 convention)
    // =========================================================================

    /**
     * Build the monic RS generator polynomial of degree {@code n}.
     *
     * <p>The generator is: {@code g(x) = ∏(x + α^i) for i in 0..n-1}
     * where α = 2 is the primitive element of GF(256).
     *
     * <p>The "b=0" convention means the roots are α^0, α^1, …, α^{n-1}
     * (starting at exponent 0, not 1). QR Code uses b=0 per ISO 18004 §7.5.
     *
     * <p>The output has {@code n+1} entries; index 0 is the leading coefficient (1).
     *
     * <p>Example: for n=2, g(x) = (x + α^0)(x + α^1) = (x+1)(x+2) = x² + 3x + 2
     * in GF(256) (addition is XOR): g = [1, 3, 2].
     *
     * @param n degree of the generator (= number of ECC codewords)
     * @return coefficient array, length n+1, leading coefficient first
     */
    private static int[] buildGenerator(int n) {
        int[] g = new int[]{1};
        for (int i = 0; i < n; i++) {
            // α^i in GF(256), primitive element α = 2.  GF256.pow(2, i).
            int ai = GF256.pow(2, i);
            // g_new(x) = g(x) * (x + α^i):
            //   multiply each coefficient of g by x (shift right by 1 slot)
            //   and XOR (add in GF(256)) with the shift multiplied by α^i.
            int[] next = new int[g.length + 1];
            for (int j = 0; j < g.length; j++) {
                next[j] ^= g[j];               // coefficient of x^{...} from g(x)·x
                next[j + 1] ^= GF256.mul(g[j], ai); // coefficient from g(x)·α^i
            }
            g = next;
        }
        return g;
    }

    /**
     * Compute {@code n} Reed-Solomon ECC bytes for the given data.
     *
     * <p>Mathematically: returns the remainder of {@code D(x) · x^n mod G(x)},
     * computed via LFSR (linear feedback shift register) polynomial division.
     *
     * <p>The LFSR processes one data byte at a time.  For each byte {@code b}:
     * <ol>
     *   <li>Feedback byte fb = b XOR rem[0]  (top of shift register)</li>
     *   <li>Shift rem left by one (rem[i] ← rem[i+1])</li>
     *   <li>XOR each position i of rem with {@code generator[i+1] * fb}</li>
     * </ol>
     * After all data bytes, rem holds the ECC bytes.
     *
     * @param data      data bytes to protect
     * @param generator generator polynomial from {@link #buildGenerator}
     * @return ECC bytes, length = generator.length - 1
     */
    private static int[] rsEncode(int[] data, int[] generator) {
        int n = generator.length - 1;
        int[] rem = new int[n];
        for (int b : data) {
            int fb = b ^ rem[0];
            // Shift the register left.
            System.arraycopy(rem, 1, rem, 0, n - 1);
            rem[n - 1] = 0;
            if (fb != 0) {
                for (int i = 0; i < n; i++) {
                    rem[i] ^= GF256.mul(generator[i + 1], fb);
                }
            }
        }
        return rem;
    }

    // =========================================================================
    // Data encoding modes
    // =========================================================================

    /**
     * The 45-character alphanumeric character set, in ISO 18004 order.
     *
     * <p>Each character's index in this string is its encoded value.
     * Characters 0-9 have values 0-9, A-Z have 10-35, and the special
     * characters ($, %, *, +, -, ., /, :, space) have values 36-44.
     *
     * <p>Pairs of characters are encoded as {@code (45 * first) + second}
     * in 11 bits; lone trailing characters use 6 bits.
     */
    private static final String ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

    /**
     * Internal encoding modes.
     *
     * <p>The encoder automatically selects the most compact mode:
     * <ul>
     *   <li>Numeric: only digits 0-9 (densest: ~3.3 bits/char)</li>
     *   <li>Alphanumeric: digits, uppercase A-Z, and 9 special chars (~5.5 bits/char)</li>
     *   <li>Byte: any UTF-8 bytes (least dense: 8 bits/char)</li>
     * </ul>
     */
    private enum EncodingMode {
        NUMERIC,
        ALPHANUMERIC,
        BYTE
    }

    /**
     * Selects the most compact encoding mode for the given input.
     *
     * <p>Mode selection is a simple priority check:
     * <ol>
     *   <li>If all chars are ASCII digits → Numeric</li>
     *   <li>Else if all chars are in the 45-char alphanumeric set → Alphanumeric</li>
     *   <li>Otherwise → Byte (UTF-8)</li>
     * </ol>
     *
     * @param input the string to encode
     * @return the most compact applicable mode
     */
    private static EncodingMode selectMode(String input) {
        boolean allDigits = true;
        boolean allAlphanum = true;
        for (int i = 0; i < input.length(); i++) {
            char c = input.charAt(i);
            if (c < '0' || c > '9') allDigits = false;
            if (ALPHANUM_CHARS.indexOf(c) < 0) allAlphanum = false;
        }
        if (allDigits) return EncodingMode.NUMERIC;
        if (allAlphanum) return EncodingMode.ALPHANUMERIC;
        return EncodingMode.BYTE;
    }

    /**
     * Returns the 4-bit mode indicator value for a given encoding mode.
     *
     * <p>ISO 18004 Table 2:
     * <ul>
     *   <li>Numeric:      0001</li>
     *   <li>Alphanumeric: 0010</li>
     *   <li>Byte:         0100</li>
     * </ul>
     */
    private static int modeIndicator(EncodingMode mode) {
        return switch (mode) {
            case NUMERIC -> 0b0001;
            case ALPHANUMERIC -> 0b0010;
            case BYTE -> 0b0100;
        };
    }

    /**
     * Returns the number of bits used to encode the character count field.
     *
     * <p>The character count field follows the mode indicator. Its width depends
     * on both the mode and the symbol version (ISO 18004 Table 3):
     *
     * <pre>
     *                 v1–9   v10–26  v27–40
     *   Numeric:        10     12      14
     *   Alphanumeric:    9     11      13
     *   Byte:            8     16      16
     * </pre>
     *
     * @param mode    the encoding mode
     * @param version QR version (1–40)
     * @return number of bits for the character count
     */
    private static int charCountBits(EncodingMode mode, int version) {
        return switch (mode) {
            case NUMERIC -> version <= 9 ? 10 : (version <= 26 ? 12 : 14);
            case ALPHANUMERIC -> version <= 9 ? 9 : (version <= 26 ? 11 : 13);
            case BYTE -> version <= 9 ? 8 : 16;
        };
    }

    // =========================================================================
    // Bit writer utility
    // =========================================================================

    /**
     * Accumulates individual bits and converts to a byte array.
     *
     * <p>Bits are written MSB-first: {@code write(0b1011, 4)} appends bits
     * 1, 0, 1, 1 in that order. The internal store is a list of 0/1 ints
     * for simplicity; {@link #toBytes()} packs them into bytes.
     */
    private static final class BitWriter {
        private final List<Integer> bits = new ArrayList<>();

        /**
         * Appends the {@code count} least-significant bits of {@code value},
         * MSB first.
         *
         * <p>Example: {@code write(0b101, 3)} appends 1, 0, 1.
         *
         * @param value the bit pattern to write
         * @param count how many bits to write (1..32)
         */
        void write(int value, int count) {
            for (int i = count - 1; i >= 0; i--) {
                bits.add((value >>> i) & 1);
            }
        }

        /** Returns the total number of bits written so far. */
        int bitLen() {
            return bits.size();
        }

        /**
         * Packs the accumulated bits into bytes, MSB-first, zero-padding the
         * last byte if needed.
         *
         * @return byte array; length = ceil(bitLen() / 8)
         */
        int[] toBytes() {
            int byteCount = (bits.size() + 7) / 8;
            int[] result = new int[byteCount];
            for (int i = 0; i < bits.size(); i++) {
                result[i / 8] |= bits.get(i) << (7 - (i % 8));
            }
            return result;
        }
    }

    // =========================================================================
    // Data segment encoding
    // =========================================================================

    /**
     * Encodes a numeric string into the bit writer.
     *
     * <p>Groups of three digits are packed into 10 bits (max value 999 &lt; 2^10).
     * A remaining pair uses 7 bits (max 99 &lt; 2^7), and a single digit uses 4 bits.
     *
     * <p>This achieves ~3.33 bits per digit compared to 8 bits in byte mode.
     *
     * <p>Example: "01234567" → groups "012", "345", "67"
     * → 012 = 10 bits, 345 = 10 bits, 67 = 7 bits.
     *
     * @param input must contain only ASCII digit characters
     * @param w     the bit writer to append to
     */
    private static void encodeNumeric(String input, BitWriter w) {
        int i = 0;
        while (i + 2 < input.length()) {
            int val = (input.charAt(i) - '0') * 100
                    + (input.charAt(i + 1) - '0') * 10
                    + (input.charAt(i + 2) - '0');
            w.write(val, 10);
            i += 3;
        }
        if (i + 1 < input.length()) {
            int val = (input.charAt(i) - '0') * 10
                    + (input.charAt(i + 1) - '0');
            w.write(val, 7);
            i += 2;
        }
        if (i < input.length()) {
            w.write(input.charAt(i) - '0', 4);
        }
    }

    /**
     * Encodes an alphanumeric string into the bit writer.
     *
     * <p>Pairs of characters are packed into 11 bits using the formula
     * {@code 45 * value(c1) + value(c2)}, where value() is the position
     * in {@link #ALPHANUM_CHARS}. A lone trailing character uses 6 bits.
     *
     * <p>Example: "AC" → 10 * 45 + 12 = 462 → 11 bits.
     *
     * @param input must contain only characters from {@link #ALPHANUM_CHARS}
     * @param w     the bit writer to append to
     */
    private static void encodeAlphanumeric(String input, BitWriter w) {
        int i = 0;
        while (i + 1 < input.length()) {
            int v1 = ALPHANUM_CHARS.indexOf(input.charAt(i));
            int v2 = ALPHANUM_CHARS.indexOf(input.charAt(i + 1));
            w.write(45 * v1 + v2, 11);
            i += 2;
        }
        if (i < input.length()) {
            w.write(ALPHANUM_CHARS.indexOf(input.charAt(i)), 6);
        }
    }

    /**
     * Encodes a string in byte mode — one UTF-8 byte per 8-bit codeword.
     *
     * <p>Byte mode handles arbitrary UTF-8 content. Multi-byte UTF-8 sequences
     * (e.g. 3-byte sequences for U+0100 and above) are written byte-by-byte
     * in sequence. The character count field must hold the byte count, not the
     * Unicode character count.
     *
     * @param input any UTF-8 string
     * @param w     the bit writer to append to
     */
    private static void encodeByteMode(String input, BitWriter w) {
        for (byte b : input.getBytes(java.nio.charset.StandardCharsets.UTF_8)) {
            w.write(b & 0xFF, 8);
        }
    }

    /**
     * Builds the complete data codeword sequence for the given input.
     *
     * <p>The structure of the bit stream is:
     * <ol>
     *   <li>4-bit mode indicator</li>
     *   <li>Character count field (width depends on mode and version)</li>
     *   <li>Encoded data bits</li>
     *   <li>Terminator: up to 4 zero-bits (may be fewer if already at capacity)</li>
     *   <li>Bit-boundary padding: zero-pad to next byte boundary</li>
     *   <li>Pad bytes: alternating 0xEC / 0x11 to fill remaining capacity</li>
     * </ol>
     *
     * <p>The pad bytes were chosen by the QR Code committee for their property
     * that they produce minimal self-interference patterns in the grid.
     *
     * @param input   the string to encode
     * @param version QR version to encode at (determines capacity and count-field width)
     * @param ecc     ECC level (determines capacity)
     * @return byte array of exactly {@link #numDataCodewords(int, EccLevel)} entries
     */
    private static int[] buildDataCodewords(String input, int version, EccLevel ecc) {
        EncodingMode mode = selectMode(input);
        int capacity = numDataCodewords(version, ecc);
        BitWriter w = new BitWriter();

        w.write(modeIndicator(mode), 4);

        // Character count: byte mode counts bytes, others count characters.
        int charCount = (mode == EncodingMode.BYTE)
                ? input.getBytes(java.nio.charset.StandardCharsets.UTF_8).length
                : input.length();
        w.write(charCount, charCountBits(mode, version));

        switch (mode) {
            case NUMERIC -> encodeNumeric(input, w);
            case ALPHANUMERIC -> encodeAlphanumeric(input, w);
            case BYTE -> encodeByteMode(input, w);
        }

        // Terminator: up to 4 zero bits (stop if we'd exceed capacity).
        int available = capacity * 8;
        int termLen = Math.min(available - w.bitLen(), 4);
        if (termLen > 0) w.write(0, termLen);

        // Byte-boundary padding.
        int rem = w.bitLen() % 8;
        if (rem != 0) w.write(0, 8 - rem);

        // Pad bytes 0xEC and 0x11 fill unused capacity.
        int[] bytes = Arrays.copyOf(w.toBytes(), capacity);
        int pad = 0xEC;
        for (int i = w.bitLen() / 8; i < capacity; i++) {
            bytes[i] = pad;
            pad = (pad == 0xEC) ? 0x11 : 0xEC;
        }
        return bytes;
    }

    // =========================================================================
    // Block processing
    // =========================================================================

    /**
     * A single RS block: one data segment and its computed ECC bytes.
     *
     * <p>The total data codeword stream is split into several of these blocks.
     * When blocks have different sizes, the shorter blocks come first
     * (group 1), then the longer blocks (group 2). Group 2 blocks have
     * exactly one more data codeword than group 1 blocks.
     */
    private record Block(int[] data, int[] ecc) {}

    /**
     * Splits the data codewords into RS blocks and computes ECC for each.
     *
     * <p>ISO 18004 §7.5 describes the two-group block structure.
     * For most versions, all blocks are the same size. For versions where
     * total data codewords don't divide evenly, some blocks get one extra byte.
     *
     * <p>Example (version 5-H): 22 total data CW, 4 blocks.
     * 22 / 4 = 5 remainder 2.
     * → 2 short blocks of 5 bytes (group 1), 2 long blocks of 6 bytes (group 2).
     *
     * @param data    full data codeword array
     * @param version QR version
     * @param ecc     ECC level
     * @return list of blocks with data and ECC
     */
    private static List<Block> computeBlocks(int[] data, int version, EccLevel ecc) {
        int e = eccIdx(ecc);
        int totalBlocks = NUM_BLOCKS[e][version];
        int eccLen = ECC_CODEWORDS_PER_BLOCK[e][version];
        int totalData = numDataCodewords(version, ecc);
        int shortLen = totalData / totalBlocks;
        int numLong = totalData % totalBlocks;  // number of "long" blocks (one extra byte)
        int[] gen = buildGenerator(eccLen);

        List<Block> blocks = new ArrayList<>();
        int offset = 0;

        // Group 1: (totalBlocks - numLong) short blocks
        int g1count = totalBlocks - numLong;
        for (int i = 0; i < g1count; i++) {
            int[] d = Arrays.copyOfRange(data, offset, offset + shortLen);
            int[] eccBytes = rsEncode(d, gen);
            blocks.add(new Block(d, eccBytes));
            offset += shortLen;
        }
        // Group 2: numLong long blocks (shortLen + 1 bytes each)
        for (int i = 0; i < numLong; i++) {
            int[] d = Arrays.copyOfRange(data, offset, offset + shortLen + 1);
            int[] eccBytes = rsEncode(d, gen);
            blocks.add(new Block(d, eccBytes));
            offset += shortLen + 1;
        }
        return blocks;
    }

    /**
     * Interleaves block codewords into the final bit-stream order.
     *
     * <p>The QR Code standard requires round-robin interleaving: first take
     * codeword index 0 from every block, then index 1 from every block, etc.
     * This spreads burst errors across multiple blocks, making them independently
     * correctable.
     *
     * <pre>
     * Block A: A0 A1 A2 A3 | EA0 EA1
     * Block B: B0 B1 B2 B3 | EB0 EB1
     * Block C: C0 C1 C2 C3 C4 | EC0 EC1
     *
     * Interleaved: A0 B0 C0 | A1 B1 C1 | A2 B2 C2 | A3 B3 C3 | C4 | EA0 EB0 EC0 | EA1 EB1 EC1
     * </pre>
     *
     * @param blocks list of blocks from {@link #computeBlocks}
     * @return interleaved codeword array ready for placement
     */
    private static int[] interleaveBlocks(List<Block> blocks) {
        int maxData = blocks.stream().mapToInt(b -> b.data().length).max().orElse(0);
        int maxEcc  = blocks.stream().mapToInt(b -> b.ecc().length).max().orElse(0);
        List<Integer> result = new ArrayList<>();

        for (int i = 0; i < maxData; i++) {
            for (Block b : blocks) {
                if (i < b.data().length) result.add(b.data()[i]);
            }
        }
        for (int i = 0; i < maxEcc; i++) {
            for (Block b : blocks) {
                if (i < b.ecc().length) result.add(b.ecc()[i]);
            }
        }
        return result.stream().mapToInt(Integer::intValue).toArray();
    }

    // =========================================================================
    // Grid construction
    // =========================================================================

    /**
     * Mutable working grid: modules (boolean state) plus a reserved-flag grid.
     *
     * <p>The {@code reserved} flag marks every module position that belongs to a
     * functional pattern (finders, separators, timing, alignment, format info,
     * version info, dark module). Reserved modules are never overwritten by data
     * placement and are never toggled by masking.
     */
    private static final class WorkGrid {
        final int size;
        final boolean[][] modules;
        final boolean[][] reserved;

        WorkGrid(int size) {
            this.size = size;
            this.modules = new boolean[size][size];
            this.reserved = new boolean[size][size];
        }

        /** Sets a module and optionally marks it reserved. */
        void set(int row, int col, boolean dark, boolean reserve) {
            modules[row][col] = dark;
            if (reserve) reserved[row][col] = true;
        }
    }

    /**
     * Places a 7×7 finder pattern at the given top-left corner.
     *
     * <p>A finder pattern looks like:
     * <pre>
     *   ███████
     *   █     █
     *   █ ███ █
     *   █ ███ █
     *   █ ███ █
     *   █     █
     *   ███████
     * </pre>
     * The outer ring (border) and inner 3×3 core are dark; the ring in between is light.
     * All finder-pattern modules are reserved.
     *
     * @param g    the working grid
     * @param top  row of the top-left corner
     * @param left column of the top-left corner
     */
    private static void placeFinder(WorkGrid g, int top, int left) {
        for (int dr = 0; dr < 7; dr++) {
            for (int dc = 0; dc < 7; dc++) {
                boolean onBorder = (dr == 0 || dr == 6 || dc == 0 || dc == 6);
                boolean inCore   = (dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4);
                g.set(top + dr, left + dc, onBorder || inCore, true);
            }
        }
    }

    /**
     * Places a 5×5 alignment pattern centered at (row, col).
     *
     * <p>An alignment pattern is a smaller finder-like pattern:
     * <pre>
     *   █████
     *   █   █
     *   █ █ █
     *   █   █
     *   █████
     * </pre>
     * The border and center are dark; the inner ring is light.
     *
     * @param g   working grid
     * @param row center row
     * @param col center column
     */
    private static void placeAlignment(WorkGrid g, int row, int col) {
        for (int dr = -2; dr <= 2; dr++) {
            for (int dc = -2; dc <= 2; dc++) {
                boolean onBorder = (Math.abs(dr) == 2 || Math.abs(dc) == 2);
                boolean isCenter = (dr == 0 && dc == 0);
                g.set(row + dr, col + dc, onBorder || isCenter, true);
            }
        }
    }

    /**
     * Places all alignment patterns for the given version.
     *
     * <p>Every (row, col) combination from {@link #ALIGNMENT_POSITIONS} gets a pattern,
     * except positions already occupied by a finder pattern (identified by the
     * reserved flag being already set at the center).
     *
     * @param g       working grid
     * @param version 1–40
     */
    private static void placeAllAlignments(WorkGrid g, int version) {
        int[] positions = ALIGNMENT_POSITIONS[version - 1];
        for (int row : positions) {
            for (int col : positions) {
                if (!g.reserved[row][col]) {
                    placeAlignment(g, row, col);
                }
            }
        }
    }

    /**
     * Places the horizontal and vertical timing strips.
     *
     * <p>The timing strips are alternating dark/light rows/columns along row 6 and
     * column 6, running between the finder separators. They define the module grid
     * coordinate system for scanners.
     *
     * <p>Row 6, columns 8 to size-9: dark if column is even.
     * Column 6, rows 8 to size-9: dark if row is even.
     *
     * @param g working grid
     */
    private static void placeTiming(WorkGrid g) {
        int sz = g.size;
        for (int c = 8; c <= sz - 9; c++) g.set(6, c, c % 2 == 0, true);
        for (int r = 8; r <= sz - 9; r++) g.set(r, 6, r % 2 == 0, true);
    }

    /**
     * Reserves the format information areas without writing values yet.
     *
     * <p>Two copies of the 15-bit format word are written:
     * <ul>
     *   <li>Copy 1: around the top-left finder (row 8, cols 0-8; col 8, rows 0-8)</li>
     *   <li>Copy 2: near the top-right finder (row 8, cols n-8 to n-1) and
     *               bottom-left finder (col 8, rows n-7 to n-1)</li>
     * </ul>
     *
     * @param g working grid
     */
    private static void reserveFormatInfo(WorkGrid g) {
        int sz = g.size;
        // Copy 1: row 8 (except timing col 6) and col 8 (except timing row 6).
        for (int c = 0; c <= 8; c++) if (c != 6) g.reserved[8][c] = true;
        for (int r = 0; r <= 8; r++) if (r != 6) g.reserved[r][8] = true;
        // Copy 2: bottom-left and top-right corners.
        for (int r = sz - 7; r < sz; r++) g.reserved[r][8] = true;
        for (int c = sz - 8; c < sz; c++) g.reserved[8][c] = true;
    }

    /**
     * Computes the 15-bit format information word.
     *
     * <p>The format word encodes the ECC level (2 bits) and mask number (3 bits),
     * protected by a (15,5) BCH code for robustness.
     *
     * <p>Construction:
     * <ol>
     *   <li>5-bit data = (eccIndicator &lt;&lt; 3) | maskNum</li>
     *   <li>Polynomial division: remainder of (data &lt;&lt; 10) mod 0x537 (BCH generator)</li>
     *   <li>15-bit word = (data &lt;&lt; 10) | remainder</li>
     *   <li>XOR with 0x5412 (ISO masking sequence) to prevent all-zero format areas</li>
     * </ol>
     *
     * @param ecc  ECC level
     * @param mask mask number 0–7
     * @return 15-bit format word
     */
    private static int computeFormatBits(EccLevel ecc, int mask) {
        int data = (eccIndicator(ecc) << 3) | mask;
        int rem = data << 10;
        for (int i = 14; i >= 10; i--) {
            if (((rem >>> i) & 1) == 1) rem ^= (0x537 << (i - 10));
        }
        return ((data << 10) | (rem & 0x3FF)) ^ 0x5412;
    }

    /**
     * Writes the 15-bit format information word into both copy locations.
     *
     * <p>The format word is labeled f14 (MSB) down to f0 (LSB).
     *
     * <p>Copy 1 placement (ISO 18004 §7.9):
     * <ul>
     *   <li>Row 8, cols 0-5: f14 down to f9 (MSB first, left to right)</li>
     *   <li>Row 8, col 7: f8  (col 6 is the timing column, skipped)</li>
     *   <li>Row 8, col 8: f7</li>
     *   <li>Col 8, row 7: f6  (row 6 is the timing row, skipped)</li>
     *   <li>Col 8, rows 5-0: f5 down to f0 (row 5 = f5, row 0 = f0)</li>
     * </ul>
     *
     * <p>Copy 2 placement:
     * <ul>
     *   <li>Row 8, cols n-1 to n-8: f0 at n-1, f7 at n-8 (LSB first, right to left)</li>
     *   <li>Col 8, rows n-7 to n-1: f8 at n-7, f14 at n-1</li>
     * </ul>
     *
     * @param g   working grid
     * @param fmt 15-bit format word from {@link #computeFormatBits}
     */
    private static void writeFormatInfo(WorkGrid g, int fmt) {
        int sz = g.size;

        // Copy 1 — top-left finder area, MSB (f14) first.
        for (int i = 0; i <= 5; i++) g.modules[8][i] = ((fmt >>> (14 - i)) & 1) == 1;
        g.modules[8][7] = ((fmt >>> 8) & 1) == 1;   // f8 (skip timing col 6)
        g.modules[8][8] = ((fmt >>> 7) & 1) == 1;   // f7
        g.modules[7][8] = ((fmt >>> 6) & 1) == 1;   // f6 (skip timing row 6)
        for (int i = 0; i <= 5; i++) g.modules[i][8] = ((fmt >>> i) & 1) == 1; // f0..f5

        // Copy 2 — top-right and bottom-left areas.
        for (int i = 0; i <= 7; i++) g.modules[8][sz - 1 - i] = ((fmt >>> i) & 1) == 1;
        for (int i = 8; i <= 14; i++) g.modules[sz - 15 + i][8] = ((fmt >>> i) & 1) == 1;
    }

    /**
     * Reserves the version information areas for versions 7 and above.
     *
     * <p>Two 6×3 blocks are reserved: one at the top-right of the grid
     * (rows 0-5, cols size-11 to size-9) and one at the bottom-left
     * (rows size-11 to size-9, cols 0-5).
     *
     * @param g       working grid
     * @param version 1–40
     */
    private static void reserveVersionInfo(WorkGrid g, int version) {
        if (version < 7) return;
        int sz = g.size;
        for (int r = 0; r < 6; r++) {
            for (int dc = 0; dc < 3; dc++) {
                g.reserved[r][sz - 11 + dc] = true;
            }
        }
        for (int dr = 0; dr < 3; dr++) {
            for (int c = 0; c < 6; c++) {
                g.reserved[sz - 11 + dr][c] = true;
            }
        }
    }

    /**
     * Computes the 18-bit version information word for versions 7+.
     *
     * <p>The version number (6 bits) is protected by a (18,6) Golay code
     * (generator polynomial 0x1F25). The 18-bit word is placed in two
     * 6×3 areas of the symbol.
     *
     * @param version 7–40
     * @return 18-bit version word
     */
    private static int computeVersionBits(int version) {
        int v = version;
        int rem = v << 12;
        for (int i = 17; i >= 12; i--) {
            if (((rem >>> i) & 1) == 1) rem ^= (0x1F25 << (i - 12));
        }
        return (v << 12) | (rem & 0xFFF);
    }

    /**
     * Writes the 18-bit version information into both 6×3 areas.
     *
     * <p>The 18 bits are read from bit 0 upward (LSB first), placed into
     * the two 6-row × 3-column areas. Bit i is placed at:
     * <ul>
     *   <li>Row a = 5 - (i / 3), col b = size - 9 - (i % 3): top-right area</li>
     *   <li>Row b = size - 9 - (i % 3), col a = 5 - (i / 3): bottom-left area (transposed)</li>
     * </ul>
     *
     * @param g       working grid
     * @param version 7–40
     */
    private static void writeVersionInfo(WorkGrid g, int version) {
        if (version < 7) return;
        int sz = g.size;
        int bits = computeVersionBits(version);
        for (int i = 0; i < 18; i++) {
            boolean dark = ((bits >>> i) & 1) == 1;
            int a = 5 - (i / 3);
            int b = sz - 9 - (i % 3);
            g.modules[a][b] = dark;
            g.modules[b][a] = dark;
        }
    }

    /**
     * Places the dark module at the fixed position (4V+9, 8).
     *
     * <p>The dark module is always dark, regardless of masking or ECC level.
     * It is located at row 4V+9, column 8. For version 1, this is (13, 8).
     *
     * @param g       working grid
     * @param version QR version 1–40
     */
    private static void placeDarkModule(WorkGrid g, int version) {
        g.set(4 * version + 9, 8, true, true);
    }

    /**
     * Places all codeword bits into the grid using the two-column zigzag scan.
     *
     * <p>The bit placement algorithm:
     * <ol>
     *   <li>Start at the bottom-right corner, scanning upward (up = true).</li>
     *   <li>Visit each column-pair from right to left, stepping by 2 columns.</li>
     *   <li>Skip column 6 (timing column) by jumping from col 7 to col 5.</li>
     *   <li>Within each column-pair, fill the right column then the left column.</li>
     *   <li>Skip reserved modules; place the next bit in non-reserved modules.</li>
     *   <li>After filling one column-pair, flip direction (up↔down) and move left 2.</li>
     * </ol>
     *
     * <p>Bits are placed MSB-first per codeword, left-to-right within each byte.
     * Remainder bits (zero-valued) are appended after all codewords.
     *
     * @param g          working grid
     * @param codewords  interleaved data+ECC codewords
     * @param version    QR version (for remainder bit count)
     */
    private static void placeBits(WorkGrid g, int[] codewords, int version) {
        int sz = g.size;

        // Expand codewords to individual bits, MSB first.
        List<Boolean> bits = new ArrayList<>();
        for (int cw : codewords) {
            for (int b = 7; b >= 0; b--) {
                bits.add(((cw >>> b) & 1) == 1);
            }
        }
        for (int i = 0; i < numRemainderBits(version); i++) bits.add(false);

        int bitIdx = 0;
        boolean up = true;
        int col = sz - 1;

        outer:
        while (true) {
            for (int vi = 0; vi < sz; vi++) {
                int row = up ? (sz - 1 - vi) : vi;
                for (int dc = 0; dc <= 1; dc++) {
                    int c = col - dc;
                    if (c < 0) continue;
                    if (c == 6) continue;   // skip timing column
                    if (g.reserved[row][c]) continue;
                    g.modules[row][c] = (bitIdx < bits.size()) && bits.get(bitIdx);
                    bitIdx++;
                }
            }
            up = !up;
            if (col < 2) break;
            col -= 2;
            if (col == 6) col = 5;  // skip timing column
        }
    }

    /**
     * Initializes the working grid with all functional patterns.
     *
     * <p>Order matters: timing strips must be placed before alignment patterns,
     * because the timing-strip modules at row/col 6 are reserved and alignment
     * patterns at (6, x) must not overwrite them.
     *
     * @param version QR version 1–40
     * @return a WorkGrid with all functional patterns placed and reserved
     */
    private static WorkGrid buildGrid(int version) {
        int sz = symbolSize(version);
        WorkGrid g = new WorkGrid(sz);

        // Three finder patterns: top-left, top-right, bottom-left.
        placeFinder(g, 0, 0);
        placeFinder(g, 0, sz - 7);
        placeFinder(g, sz - 7, 0);

        // Separator rows/columns (light border around each finder).
        for (int i = 0; i <= 7; i++) {
            g.set(7, i, false, true);        // TL bottom
            g.set(i, 7, false, true);        // TL right
            g.set(7, sz - 1 - i, false, true); // TR bottom
            g.set(i, sz - 8, false, true);   // TR left
            g.set(sz - 8, i, false, true);   // BL top
            g.set(sz - 1 - i, 7, false, true); // BL right
        }

        placeTiming(g);
        placeAllAlignments(g, version);
        reserveFormatInfo(g);
        reserveVersionInfo(g, version);
        placeDarkModule(g, version);

        return g;
    }

    // =========================================================================
    // Masking and penalty
    // =========================================================================

    /**
     * Returns true if a module at (row, col) should be toggled by the given mask.
     *
     * <p>ISO 18004 §8.8.1 defines 8 mask patterns. Each pattern is a Boolean
     * function of the module coordinates. The mask is applied by XOR: a module
     * is toggled (dark↔light) iff the mask condition is true at its position.
     *
     * <p>Mask patterns (indices 0-7):
     * <pre>
     *   0: (r+c) % 2 == 0
     *   1: r % 2 == 0
     *   2: c % 3 == 0
     *   3: (r+c) % 3 == 0
     *   4: (r/2 + c/3) % 2 == 0
     *   5: (r*c) % 2 + (r*c) % 3 == 0
     *   6: ((r*c) % 2 + (r*c) % 3) % 2 == 0
     *   7: ((r+c) % 2 + (r*c) % 3) % 2 == 0
     * </pre>
     *
     * @param mask mask number 0–7
     * @param r    module row
     * @param c    module column
     * @return true if the module should be toggled
     */
    private static boolean maskCondition(int mask, int r, int c) {
        return switch (mask) {
            case 0 -> (r + c) % 2 == 0;
            case 1 -> r % 2 == 0;
            case 2 -> c % 3 == 0;
            case 3 -> (r + c) % 3 == 0;
            case 4 -> (r / 2 + c / 3) % 2 == 0;
            case 5 -> (r * c) % 2 + (r * c) % 3 == 0;
            case 6 -> ((r * c) % 2 + (r * c) % 3) % 2 == 0;
            case 7 -> ((r + c) % 2 + (r * c) % 3) % 2 == 0;
            default -> false;
        };
    }

    /**
     * Applies a mask pattern to the data modules and returns the result.
     *
     * <p>Only non-reserved modules are affected. Reserved functional-pattern
     * modules keep their original values regardless of the mask.
     *
     * @param modules  source module grid
     * @param reserved reserved flag grid
     * @param sz       grid side length
     * @param mask     mask number 0–7
     * @return a new grid with the mask applied
     */
    private static boolean[][] applyMask(boolean[][] modules, boolean[][] reserved, int sz, int mask) {
        boolean[][] result = new boolean[sz][sz];
        for (int r = 0; r < sz; r++) {
            for (int c = 0; c < sz; c++) {
                result[r][c] = modules[r][c];
                if (!reserved[r][c]) {
                    result[r][c] ^= maskCondition(mask, r, c);
                }
            }
        }
        return result;
    }

    /**
     * Computes the total penalty score for a masked grid.
     *
     * <p>ISO 18004 §8.8.2 defines four penalty rules. Lower scores are better:
     * the encoder picks the mask with the lowest penalty.
     *
     * <h3>Rule 1 — Runs of same-color modules</h3>
     * <p>For each row and column, count consecutive same-color runs of length ≥ 5.
     * Penalty = (run_length - 2) for each such run.
     *
     * <h3>Rule 2 — 2×2 same-color blocks</h3>
     * <p>Any 2×2 area of modules all the same color adds 3 penalty points.
     *
     * <h3>Rule 3 — Finder-pattern-like sequences</h3>
     * <p>The patterns {@code 1011101 0000} and {@code 0000 1011101} in any row
     * or column add 40 penalty points each.
     *
     * <h3>Rule 4 — Dark module ratio</h3>
     * <p>The percentage of dark modules should be close to 50%.
     * Penalty = 10 × (floor(|ratio_in_5pct_units - 10|)).
     *
     * @param modules the masked grid
     * @param sz      grid side length
     * @return total penalty score
     */
    private static int computePenalty(boolean[][] modules, int sz) {
        int penalty = 0;

        // Rule 1: runs of ≥ 5 same-color modules.
        for (int a = 0; a < sz; a++) {
            for (int horiz = 0; horiz <= 1; horiz++) {
                int run = 1;
                boolean prev = (horiz == 1) ? modules[a][0] : modules[0][a];
                for (int i = 1; i < sz; i++) {
                    boolean cur = (horiz == 1) ? modules[a][i] : modules[i][a];
                    if (cur == prev) {
                        run++;
                    } else {
                        if (run >= 5) penalty += run - 2;
                        run = 1;
                        prev = cur;
                    }
                }
                if (run >= 5) penalty += run - 2;
            }
        }

        // Rule 2: 2×2 blocks of same color.
        for (int r = 0; r < sz - 1; r++) {
            for (int c = 0; c < sz - 1; c++) {
                boolean d = modules[r][c];
                if (d == modules[r][c + 1] && d == modules[r + 1][c] && d == modules[r + 1][c + 1]) {
                    penalty += 3;
                }
            }
        }

        // Rule 3: finder-pattern-like sequences.
        // ISO 18004 Annex C: the two patterns to look for are
        //   p1 = 1 0 1 1 1 0 1 0 0 0 0   (dark-light-dark-dark-dark-light-dark + 4 lights)
        //   p2 = 0 0 0 0 1 0 1 1 1 0 1   (4 lights + dark-light-dark-dark-dark-light-dark)
        int[] p1 = {1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0};
        int[] p2 = {0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1};
        for (int a = 0; a < sz; a++) {
            for (int b = 0; b + 10 < sz; b++) {
                boolean mh1 = true, mh2 = true, mv1 = true, mv2 = true;
                for (int k = 0; k < 11; k++) {
                    int bh = modules[a][b + k] ? 1 : 0;
                    int bv = modules[b + k][a] ? 1 : 0;
                    if (bh != p1[k]) mh1 = false;
                    if (bh != p2[k]) mh2 = false;
                    if (bv != p1[k]) mv1 = false;
                    if (bv != p2[k]) mv2 = false;
                }
                if (mh1) penalty += 40;
                if (mh2) penalty += 40;
                if (mv1) penalty += 40;
                if (mv2) penalty += 40;
            }
        }

        // Rule 4: dark module ratio deviation from 50%.
        int dark = 0;
        for (int r = 0; r < sz; r++) for (int c = 0; c < sz; c++) if (modules[r][c]) dark++;
        double total = (double)(sz * sz);
        double ratio = (dark / total) * 100.0;
        int prev5 = (int)(Math.floor(ratio / 5.0)) * 5;
        int a = Math.abs(prev5 - 50);
        int bv = Math.abs(prev5 + 5 - 50);
        penalty += (Math.min(a, bv) / 5) * 10;

        return penalty;
    }

    // =========================================================================
    // Version selection
    // =========================================================================

    /**
     * Selects the minimum QR Code version that can hold the given input.
     *
     * <p>Iterates versions 1–40, computing the required bits for each mode
     * and comparing against the capacity at the requested ECC level.
     *
     * <p>Required bits formula:
     * <ul>
     *   <li>4 (mode indicator) + char_count_bits + encoded data bits</li>
     *   <li>Numeric: ceil(n * 10 / 3) data bits</li>
     *   <li>Alphanumeric: ceil(n * 11 / 2) data bits</li>
     *   <li>Byte: n * 8 data bits (n = UTF-8 byte count)</li>
     * </ul>
     *
     * @param input the string to encode
     * @param ecc   the desired ECC level
     * @return smallest version 1–40 that fits the input
     * @throws QRCodeException if no version can hold the input
     */
    private static int selectVersion(String input, EccLevel ecc) throws QRCodeException {
        EncodingMode mode = selectMode(input);
        int byteLen = input.getBytes(java.nio.charset.StandardCharsets.UTF_8).length;
        int charLen = input.length();

        for (int v = 1; v <= 40; v++) {
            int capacity = numDataCodewords(v, ecc);
            int dataBits = switch (mode) {
                case BYTE -> byteLen * 8;
                case NUMERIC -> (charLen * 10 + 2) / 3;    // ceil(n*10/3)
                case ALPHANUMERIC -> (charLen * 11 + 1) / 2; // ceil(n*11/2)
            };
            int bitsNeeded = 4 + charCountBits(mode, v) + dataBits;
            int cwNeeded = (bitsNeeded + 7) / 8;
            if (cwNeeded <= capacity) return v;
        }
        throw new QRCodeException(String.format(
            "Input (%d chars, ECC=%s) exceeds version-40 capacity.", input.length(), ecc));
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encodes a UTF-8 string into a QR Code {@link ModuleGrid}.
     *
     * <p>Returns a {@code (4V+17) × (4V+17)} boolean grid where {@code true}
     * means dark module. Automatically selects the minimum version that fits
     * the input at the given ECC level.
     *
     * <p>The encoding pipeline:
     * <ol>
     *   <li>Select mode (numeric/alphanumeric/byte) and version</li>
     *   <li>Build data codewords with mode header and padding</li>
     *   <li>Split into RS blocks and compute ECC</li>
     *   <li>Interleave blocks</li>
     *   <li>Initialize grid with functional patterns</li>
     *   <li>Place codeword bits via zigzag scan</li>
     *   <li>Evaluate 8 masks, pick lowest-penalty mask</li>
     *   <li>Write final format info and version info</li>
     * </ol>
     *
     * @param input any UTF-8 string (max ~7089 characters in numeric mode)
     * @param ecc   desired error-correction level
     * @return a {@link ModuleGrid} suitable for rendering with {@link Barcode2D#layout}
     * @throws QRCodeException if the input is too long for version-40 at this ECC level
     *
     * @see #encodeAndLayout(String, EccLevel, Barcode2DLayoutConfig)
     */
    public static ModuleGrid encode(String input, EccLevel ecc) throws QRCodeException {
        // Early-exit guard: version 40 holds at most 7089 numeric characters
        // (~2953 bytes in byte mode). Check the byte length to prevent large
        // allocations before selectVersion rejects the input.
        if (input.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 7089) {
            throw new QRCodeException(String.format(
                "Input byte length %d exceeds 7089 (the QR Code v40 numeric-mode maximum).",
                input.getBytes(java.nio.charset.StandardCharsets.UTF_8).length));
        }

        int version = selectVersion(input, ecc);
        int sz = symbolSize(version);

        int[] dataCw = buildDataCodewords(input, version, ecc);
        List<Block> blocks = computeBlocks(dataCw, version, ecc);
        int[] interleaved = interleaveBlocks(blocks);

        WorkGrid grid = buildGrid(version);
        placeBits(grid, interleaved, version);

        // Evaluate 8 masks; pick the one with the lowest penalty score.
        int bestMask = 0;
        int bestPenalty = Integer.MAX_VALUE;
        for (int m = 0; m < 8; m++) {
            boolean[][] masked = applyMask(grid.modules, grid.reserved, sz, m);
            // Write format info into a temporary copy to include it in penalty scoring.
            int fmt = computeFormatBits(ecc, m);
            WorkGrid test = new WorkGrid(sz);
            for (int r = 0; r < sz; r++) {
                test.modules[r] = masked[r].clone();
                test.reserved[r] = grid.reserved[r].clone();
            }
            writeFormatInfo(test, fmt);
            int p = computePenalty(test.modules, sz);
            if (p < bestPenalty) {
                bestPenalty = p;
                bestMask = m;
            }
        }

        // Finalize: apply best mask and write permanent format + version info.
        boolean[][] finalMods = applyMask(grid.modules, grid.reserved, sz, bestMask);
        WorkGrid finalGrid = new WorkGrid(sz);
        for (int r = 0; r < sz; r++) {
            finalGrid.modules[r] = finalMods[r].clone();
            finalGrid.reserved[r] = grid.reserved[r].clone();
        }
        writeFormatInfo(finalGrid, computeFormatBits(ecc, bestMask));
        writeVersionInfo(finalGrid, version);

        // Convert to List<List<Boolean>> for the ModuleGrid constructor.
        List<List<Boolean>> modulesList = new ArrayList<>(sz);
        for (int r = 0; r < sz; r++) {
            List<Boolean> row = new ArrayList<>(sz);
            for (int c = 0; c < sz; c++) row.add(finalGrid.modules[r][c]);
            modulesList.add(row);
        }

        return new ModuleGrid(sz, sz, modulesList, ModuleShape.SQUARE);
    }

    /**
     * Encodes a string and converts the result to a pixel-resolved {@link PaintScene}.
     *
     * <p>Convenience method that chains {@link #encode} and {@link Barcode2D#layout}.
     *
     * @param input  any UTF-8 string
     * @param ecc    ECC level
     * @param config layout configuration (module size, quiet zone, colors)
     * @return a {@link PaintScene} ready for SVG/canvas rendering
     * @throws QRCodeException if the input is too long or the layout config is invalid
     */
    public static PaintScene encodeAndLayout(String input, EccLevel ecc, Barcode2DLayoutConfig config)
            throws QRCodeException {
        ModuleGrid grid = encode(input, ecc);
        return Barcode2D.layout(grid, config);
    }
}
