package com.codingadventures.microqr;

import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.barcode2d.ModuleShape;
import com.codingadventures.gf256.GF256;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * Micro QR Code encoder — ISO/IEC 18004:2015 Annex E compliant.
 *
 * <p>Micro QR Code is the compact variant of QR Code, designed for applications
 * where even the smallest standard QR (21×21 at version 1) is too large.
 * Common use cases include surface-mount component labels, circuit board
 * markings, and miniature industrial tags.
 *
 * <h2>Symbol sizes</h2>
 *
 * <pre>
 * M1: 11×11   M2: 13×13   M3: 15×15   M4: 17×17
 * formula: size = 2 × version_number + 9
 * </pre>
 *
 * <h2>Key differences from regular QR Code</h2>
 *
 * <ul>
 *   <li><b>Single finder pattern</b> at top-left only (one 7×7 square, not three).</li>
 *   <li><b>Timing at row 0 / col 0</b> (not row 6 / col 6).</li>
 *   <li><b>Only 4 mask patterns</b> (not 8).</li>
 *   <li><b>Format XOR mask 0x4445</b> (not 0x5412).</li>
 *   <li><b>Single copy of format info</b> (not two).</li>
 *   <li><b>2-module quiet zone</b> (not 4).</li>
 *   <li><b>Narrower mode indicators</b> (0–3 bits instead of 4).</li>
 *   <li><b>Single block</b> (no interleaving).</li>
 * </ul>
 *
 * <h2>Encoding pipeline</h2>
 *
 * <pre>
 * input string
 *   → auto-select smallest symbol (M1..M4) and mode
 *   → build bit stream (mode indicator + char count + data + terminator + padding)
 *   → Reed-Solomon ECC (GF(256)/0x11D, b=0, single block)
 *   → initialize grid (finder, L-shaped separator, timing at row0/col0, format reserved)
 *   → zigzag data placement (two-column snake from bottom-right)
 *   → evaluate 4 mask patterns, pick lowest penalty
 *   → write format information (15 bits, single copy, XOR 0x4445)
 *   → ModuleGrid
 * </pre>
 *
 * <h2>Usage</h2>
 *
 * <pre>{@code
 * // Auto-select smallest symbol and default ECC (M = medium):
 * ModuleGrid grid = MicroQR.encode("HELLO", null, null);
 * assert grid.rows == 13; // M2 symbol
 *
 * // Encode to a specific version and ECC level:
 * ModuleGrid m4 = MicroQR.encode("https://a.b", MicroQRVersion.M4, EccLevel.L);
 * assert m4.rows == 17;
 * }</pre>
 */
public final class MicroQR {

    /** This is a utility class — no instances. */
    private MicroQR() {}

    // =========================================================================
    // Public enumerations
    // =========================================================================

    /**
     * Micro QR symbol designator.
     *
     * <p>Each step up adds two rows/columns.  The formula is:
     * {@code size = 2 × version_number + 9}
     *
     * <pre>
     * M1 = 11×11   M2 = 13×13   M3 = 15×15   M4 = 17×17
     * </pre>
     */
    public enum MicroQRVersion {
        M1, M2, M3, M4
    }

    /**
     * Error correction level for Micro QR.
     *
     * <table>
     *   <caption>ECC level availability</caption>
     *   <tr><th>Level</th><th>Available in</th><th>Recovery</th></tr>
     *   <tr><td>DETECTION</td><td>M1 only</td><td>detects errors only</td></tr>
     *   <tr><td>L</td><td>M2, M3, M4</td><td>~7% of codewords</td></tr>
     *   <tr><td>M</td><td>M2, M3, M4</td><td>~15% of codewords</td></tr>
     *   <tr><td>Q</td><td>M4 only</td><td>~25% of codewords</td></tr>
     * </table>
     *
     * <p>Level H is <em>not</em> available in any Micro QR symbol.
     */
    public enum EccLevel {
        DETECTION, L, M, Q
    }

    // =========================================================================
    // Errors
    // =========================================================================

    /**
     * Base exception for Micro QR encoding errors.
     *
     * <p>All encoding failures throw a subclass of this exception, making it
     * easy to catch any Micro QR error in a single catch block while still
     * being able to distinguish the specific failure with {@code instanceof}.
     */
    public static class MicroQRException extends RuntimeException {
        public MicroQRException(String message) { super(message); }
    }

    /** Input string is too long to fit in any M1–M4 symbol at any ECC level. */
    public static final class InputTooLongException extends MicroQRException {
        public InputTooLongException(String msg) { super(msg); }
    }

    /** The requested ECC level is not available for the chosen symbol. */
    public static final class ECCNotAvailableException extends MicroQRException {
        public ECCNotAvailableException(String msg) { super(msg); }
    }

    /** The requested encoding mode is not available for the chosen symbol. */
    public static final class UnsupportedModeException extends MicroQRException {
        public UnsupportedModeException(String msg) { super(msg); }
    }

    /** A character cannot be encoded in the selected mode. */
    public static final class InvalidCharacterException extends MicroQRException {
        public InvalidCharacterException(String msg) { super(msg); }
    }

    // =========================================================================
    // Encoding mode
    // =========================================================================

    /**
     * Encoding mode determines how input characters are packed into bits.
     *
     * <p>Selection priority (most compact first): numeric > alphanumeric > byte.
     */
    private enum EncodingMode {
        NUMERIC, ALPHANUMERIC, BYTE
    }

    /**
     * The 45-character alphanumeric set shared with regular QR Code.
     *
     * <p>Characters are assigned indices 0–44 in the order shown.  This is
     * the same table used by standard QR Code; pairs of characters are packed
     * into 11 bits using {@code first_index × 45 + second_index}.
     */
    private static final String ALPHANUM_CHARS = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";

    // =========================================================================
    // Symbol configuration table
    // =========================================================================

    /**
     * Compile-time constants for one (version, ECC) combination.
     *
     * <p>There are exactly 8 valid combinations:
     * M1/Detection, M2/L, M2/M, M3/L, M3/M, M4/L, M4/M, M4/Q.
     *
     * <p>All data in this record comes directly from ISO 18004:2015 Annex E.
     * Embedding these as constants avoids error-prone runtime calculations
     * and makes the encoder's behaviour identical across all language ports.
     */
    private record SymbolConfig(
        MicroQRVersion version,
        EccLevel ecc,
        /** 3-bit symbol indicator placed in format information (0..7). */
        int symbolIndicator,
        /** Symbol side length in modules (11, 13, 15, or 17). */
        int size,
        /** Number of data codewords (full 8-bit bytes except M1 uses 2.5 bytes). */
        int dataCw,
        /** Number of ECC codewords. */
        int eccCw,
        /** Maximum numeric characters. 0 = not supported. */
        int numericCap,
        /** Maximum alphanumeric characters. 0 = not supported. */
        int alphaCap,
        /** Maximum byte characters. 0 = not supported. */
        int byteCap,
        /** Terminator bit count (3/5/7/9). */
        int terminatorBits,
        /** Mode indicator bit width (0=M1, 1=M2, 2=M3, 3=M4). */
        int modeIndicatorBits,
        /** Character count field width for numeric mode. */
        int ccBitsNumeric,
        /** Character count field width for alphanumeric mode. */
        int ccBitsAlpha,
        /** Character count field width for byte mode. */
        int ccBitsByte,
        /** True for M1 only: last data "codeword" is 4 bits, total = 20 bits. */
        boolean m1HalfCw
    ) {}

    /**
     * All 8 valid Micro QR symbol configurations from ISO 18004:2015 Annex E.
     *
     * <p>Data capacities, codeword counts, and field widths are compile-time
     * constants.  The table is ordered from smallest (M1/Detection) to largest
     * (M4/Q) so that the auto-selection algorithm can iterate in order and stop
     * at the first configuration that fits the input.
     *
     * <p>Capacity table reference:
     * <pre>
     * Symbol | ECC | Numeric | Alpha | Byte | Data CWs | ECC CWs
     * -------|-----|---------|-------|------|----------|---------
     * M1     | Det |       5 |     — |    — |        3 |       2
     * M2     | L   |      10 |     6 |    4 |        5 |       5
     * M2     | M   |       8 |     5 |    3 |        4 |       6
     * M3     | L   |      23 |    14 |    9 |       11 |       6
     * M3     | M   |      18 |    11 |    7 |        9 |       8
     * M4     | L   |      35 |    21 |   15 |       16 |       8
     * M4     | M   |      30 |    18 |   13 |       14 |      10
     * M4     | Q   |      21 |    13 |    9 |       10 |      14
     * </pre>
     */
    private static final SymbolConfig[] SYMBOL_CONFIGS = {
        // M1 / Detection
        new SymbolConfig(MicroQRVersion.M1, EccLevel.DETECTION,
            0, 11, 3, 2, 5, 0, 0, 3, 0, 3, 0, 0, true),
        // M2 / L
        new SymbolConfig(MicroQRVersion.M2, EccLevel.L,
            1, 13, 5, 5, 10, 6, 4, 5, 1, 4, 3, 4, false),
        // M2 / M
        new SymbolConfig(MicroQRVersion.M2, EccLevel.M,
            2, 13, 4, 6, 8, 5, 3, 5, 1, 4, 3, 4, false),
        // M3 / L
        new SymbolConfig(MicroQRVersion.M3, EccLevel.L,
            3, 15, 11, 6, 23, 14, 9, 7, 2, 5, 4, 4, false),
        // M3 / M
        new SymbolConfig(MicroQRVersion.M3, EccLevel.M,
            4, 15, 9, 8, 18, 11, 7, 7, 2, 5, 4, 4, false),
        // M4 / L
        new SymbolConfig(MicroQRVersion.M4, EccLevel.L,
            5, 17, 16, 8, 35, 21, 15, 9, 3, 6, 5, 5, false),
        // M4 / M
        new SymbolConfig(MicroQRVersion.M4, EccLevel.M,
            6, 17, 14, 10, 30, 18, 13, 9, 3, 6, 5, 5, false),
        // M4 / Q
        new SymbolConfig(MicroQRVersion.M4, EccLevel.Q,
            7, 17, 10, 14, 21, 13, 9, 9, 3, 6, 5, 5, false),
    };

    // =========================================================================
    // RS generator polynomials (compile-time constants)
    // =========================================================================

    /**
     * Monic RS generator polynomials for GF(256)/0x11D with b=0 convention.
     *
     * <p>The generator polynomial of degree n is:
     * <pre>
     *   g(x) = (x + α⁰)(x + α¹)···(x + α^{n-1})
     * </pre>
     *
     * <p>Array length is n+1 (leading monic coefficient 0x01 included).
     * Only the counts {2, 5, 6, 8, 10, 14} are needed for Micro QR.
     *
     * <p>These are the same polynomials used in regular QR Code for blocks with
     * matching ECC codeword counts.  They are embedded as compile-time constants
     * to avoid any computation errors at runtime.
     */
    private static int[] getGenerator(int eccCount) {
        return switch (eccCount) {
            case 2  -> new int[]{0x01, 0x03, 0x02};
            case 5  -> new int[]{0x01, 0x1f, 0xf6, 0x44, 0xd9, 0x68};
            case 6  -> new int[]{0x01, 0x3f, 0x4e, 0x17, 0x9b, 0x05, 0x37};
            case 8  -> new int[]{0x01, 0x63, 0x0d, 0x60, 0x6d, 0x5b, 0x10, 0xa2, 0xa3};
            case 10 -> new int[]{0x01, 0xf6, 0x75, 0xa8, 0xd0, 0xc3, 0xe3, 0x36, 0xe1, 0x3c, 0x45};
            case 14 -> new int[]{0x01, 0xf6, 0x9a, 0x60, 0x97, 0x8a, 0xf1, 0xa4, 0xa1, 0x8e, 0xfc, 0x7a, 0x52, 0xad, 0xac};
            default -> throw new IllegalArgumentException("No generator for ecc_count=" + eccCount);
        };
    }

    // =========================================================================
    // Pre-computed format information table
    // =========================================================================

    /**
     * All 32 pre-computed format words (after XOR with 0x4445).
     *
     * <p>Indexed as {@code FORMAT_TABLE[symbolIndicator][maskPattern]}.
     *
     * <p>The 15-bit format word structure:
     * <pre>
     *   [symbol_indicator (3b)] [mask_pattern (2b)] [BCH-10 remainder]
     * </pre>
     * XOR-masked with 0x4445 (Micro QR specific, not 0x5412 like regular QR).
     * This prevents a Micro QR symbol from being misread as a regular QR symbol.
     *
     * <p>Pre-computed table (mask 0–3 per column):
     * <pre>
     * Symbol+ECC  | Mask 0 | Mask 1 | Mask 2 | Mask 3
     * ------------|--------|--------|--------|--------
     * M1  (000)   | 0x4445 | 0x4172 | 0x4E2B | 0x4B1C
     * M2-L (001)  | 0x5528 | 0x501F | 0x5F46 | 0x5A71
     * M2-M (010)  | 0x6649 | 0x637E | 0x6C27 | 0x6910
     * M3-L (011)  | 0x7764 | 0x7253 | 0x7D0A | 0x783D
     * M3-M (100)  | 0x06DE | 0x03E9 | 0x0CB0 | 0x0987
     * M4-L (101)  | 0x17F3 | 0x12C4 | 0x1D9D | 0x18AA
     * M4-M (110)  | 0x24B2 | 0x2185 | 0x2EDC | 0x2BEB
     * M4-Q (111)  | 0x359F | 0x30A8 | 0x3FF1 | 0x3AC6
     * </pre>
     */
    private static final int[][] FORMAT_TABLE = {
        {0x4445, 0x4172, 0x4E2B, 0x4B1C},  // M1
        {0x5528, 0x501F, 0x5F46, 0x5A71},  // M2-L
        {0x6649, 0x637E, 0x6C27, 0x6910},  // M2-M
        {0x7764, 0x7253, 0x7D0A, 0x783D},  // M3-L
        {0x06DE, 0x03E9, 0x0CB0, 0x0987},  // M3-M
        {0x17F3, 0x12C4, 0x1D9D, 0x18AA},  // M4-L
        {0x24B2, 0x2185, 0x2EDC, 0x2BEB},  // M4-M
        {0x359F, 0x30A8, 0x3FF1, 0x3AC6},  // M4-Q
    };

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encode a string to a Micro QR Code {@link ModuleGrid}.
     *
     * <p>Automatically selects the smallest symbol (M1..M4) and most compact
     * encoding mode that can hold the input.  Pass {@code version} and/or
     * {@code ecc} to override the auto-selection.
     *
     * <p>Default ECC level when {@code ecc} is {@code null}: the smallest
     * symbol is tried with all applicable ECC levels (L first), so the
     * resulting level may vary.  To pin the level, pass an explicit value.
     *
     * @param input   The string to encode (must not be null).
     * @param version Override the symbol version (null = auto-select).
     * @param ecc     Override the ECC level (null = auto-select).
     * @return A complete {@link ModuleGrid} ready for rendering.
     * @throws InputTooLongException      if the input exceeds M4 capacity.
     * @throws ECCNotAvailableException   if the version+ECC combination does
     *                                    not exist in the Micro QR standard.
     * @throws UnsupportedModeException   if no encoding mode is available for
     *                                    the input in the selected symbol.
     */
    public static ModuleGrid encode(String input, MicroQRVersion version, EccLevel ecc) {
        SymbolConfig cfg = selectConfig(input, version, ecc);
        EncodingMode mode = selectMode(input, cfg);
        return encodeWithConfig(input, cfg, mode);
    }

    /**
     * Convenience overload with auto-selected version and ECC.
     *
     * <p>Equivalent to {@code encode(input, null, null)}.
     *
     * @param input The string to encode.
     * @return A complete {@link ModuleGrid}.
     */
    public static ModuleGrid encode(String input) {
        return encode(input, null, null);
    }

    // =========================================================================
    // Symbol selection
    // =========================================================================

    /**
     * Find the smallest symbol configuration that can hold the given input.
     *
     * <p>Iterates through SYMBOL_CONFIGS in order (smallest first).  For each
     * candidate that satisfies the version/ECC filter:
     * <ol>
     *   <li>Determine the best encoding mode for this symbol.</li>
     *   <li>Check that the input length fits within the mode capacity.</li>
     *   <li>Return the first match.</li>
     * </ol>
     *
     * @param input   The string to encode.
     * @param version Version filter (null = any).
     * @param ecc     ECC filter (null = any).
     * @return The matching {@link SymbolConfig}.
     * @throws ECCNotAvailableException   if no config matches version+ECC.
     * @throws InputTooLongException      if no config can hold the input.
     */
    private static SymbolConfig selectConfig(
            String input, MicroQRVersion version, EccLevel ecc) {

        boolean foundMatchingFilter = false;

        for (SymbolConfig cfg : SYMBOL_CONFIGS) {
            if (version != null && cfg.version() != version) continue;
            if (ecc != null && cfg.ecc() != ecc) continue;

            foundMatchingFilter = true;

            EncodingMode mode;
            try {
                mode = selectMode(input, cfg);
            } catch (UnsupportedModeException e) {
                continue;
            }

            int len = (mode == EncodingMode.BYTE)
                ? input.getBytes().length
                : input.length();
            int cap = switch (mode) {
                case NUMERIC -> cfg.numericCap();
                case ALPHANUMERIC -> cfg.alphaCap();
                case BYTE -> cfg.byteCap();
            };

            if (cap > 0 && len <= cap) {
                return cfg;
            }
        }

        if (!foundMatchingFilter) {
            throw new ECCNotAvailableException(
                "No symbol configuration matches version=" + version + " ecc=" + ecc
            );
        }

        throw new InputTooLongException(
            "Input (length " + input.length() + ") does not fit in any Micro QR symbol " +
            "(version=" + version + ", ecc=" + ecc + "). " +
            "Maximum is 35 numeric chars in M4-L."
        );
    }

    /**
     * Select the most compact encoding mode supported by the given config.
     *
     * <p>Selection priority: numeric > alphanumeric > byte.
     * If no supported mode can encode the input, throws {@link UnsupportedModeException}.
     *
     * @param input The string to encode.
     * @param cfg   The symbol configuration.
     * @return The selected encoding mode.
     */
    private static EncodingMode selectMode(String input, SymbolConfig cfg) {
        // Numeric: all characters are ASCII digits 0–9
        if (cfg.ccBitsNumeric() > 0 && isNumeric(input)) {
            return EncodingMode.NUMERIC;
        }
        // Alphanumeric: all characters in the 45-char QR alphanumeric set
        if (cfg.alphaCap() > 0 && isAlphanumeric(input)) {
            return EncodingMode.ALPHANUMERIC;
        }
        // Byte: raw bytes (any string is valid — UTF-8 bytes are used directly)
        if (cfg.byteCap() > 0) {
            return EncodingMode.BYTE;
        }
        throw new UnsupportedModeException(
            "Input cannot be encoded in any mode supported by " +
            cfg.version() + "-" + cfg.ecc()
        );
    }

    /** Returns true if every character in {@code s} is an ASCII digit 0–9. */
    private static boolean isNumeric(String s) {
        if (s.isEmpty()) return true;
        for (char c : s.toCharArray()) {
            if (c < '0' || c > '9') return false;
        }
        return true;
    }

    /** Returns true if every character in {@code s} is in the 45-char alphanumeric set. */
    private static boolean isAlphanumeric(String s) {
        for (char c : s.toCharArray()) {
            if (ALPHANUM_CHARS.indexOf(c) < 0) return false;
        }
        return true;
    }

    // =========================================================================
    // Data encoding
    // =========================================================================

    /**
     * Build the complete data codeword byte sequence.
     *
     * <p>For all symbols except M1:
     * <pre>
     *   [mode indicator] [char count] [data bits] [terminator] [byte-align] [0xEC/0x11 fill]
     *   → exactly cfg.dataCw bytes
     * </pre>
     *
     * <p>For M1 ({@code m1HalfCw = true}):
     * <pre>
     *   Total capacity = 20 bits = 2 full bytes + 4-bit nibble.
     *   The RS encoder receives 3 bytes where byte[2] has data in the
     *   upper 4 bits and zero in the lower 4 bits.
     * </pre>
     *
     * @param input The string to encode.
     * @param cfg   The symbol configuration.
     * @param mode  The encoding mode.
     * @return The data codeword bytes.
     */
    private static byte[] buildDataCodewords(String input, SymbolConfig cfg, EncodingMode mode) {
        // Total usable data bit capacity:
        // M1 uses 3 codewords but the last is only 4 bits → 20 bits total.
        // All others: dataCw * 8 bits.
        int totalBits = cfg.m1HalfCw() ? (cfg.dataCw() * 8 - 4) : (cfg.dataCw() * 8);

        BitWriter w = new BitWriter();

        // Mode indicator (0/1/2/3 bits depending on symbol)
        if (cfg.modeIndicatorBits() > 0) {
            w.write(modeIndicatorValue(mode, cfg), cfg.modeIndicatorBits());
        }

        // Character count indicator
        int charCount = (mode == EncodingMode.BYTE)
            ? input.getBytes().length
            : input.length();
        int ccBits = charCountBits(mode, cfg);
        w.write(charCount, ccBits);

        // Encoded data bits
        switch (mode) {
            case NUMERIC      -> encodeNumeric(input, w);
            case ALPHANUMERIC -> encodeAlphanumeric(input, w);
            case BYTE         -> encodeByteMode(input, w);
        }

        // Terminator: up to terminatorBits zero bits, truncated if capacity full
        int remaining = totalBits - w.bitLen();
        if (remaining > 0) {
            w.write(0, Math.min(cfg.terminatorBits(), remaining));
        }

        if (cfg.m1HalfCw()) {
            // M1: pack into exactly 20 bits → 3 bytes (last byte: data in upper nibble)
            int[] bits = w.toBitArray();
            // Resize to 20 bits
            int[] padded = new int[20];
            for (int i = 0; i < Math.min(bits.length, 20); i++) {
                padded[i] = bits[i];
            }
            byte b0 = (byte)(
                (padded[0]  << 7) | (padded[1]  << 6) | (padded[2]  << 5) | (padded[3]  << 4) |
                (padded[4]  << 3) | (padded[5]  << 2) | (padded[6]  << 1) |  padded[7]
            );
            byte b1 = (byte)(
                (padded[8]  << 7) | (padded[9]  << 6) | (padded[10] << 5) | (padded[11] << 4) |
                (padded[12] << 3) | (padded[13] << 2) | (padded[14] << 1) |  padded[15]
            );
            byte b2 = (byte)(
                (padded[16] << 7) | (padded[17] << 6) | (padded[18] << 5) | (padded[19] << 4)
            );
            return new byte[]{b0, b1, b2};
        }

        // Pad to byte boundary with zero bits
        int rem = w.bitLen() % 8;
        if (rem != 0) {
            w.write(0, 8 - rem);
        }

        // Fill remaining codewords with alternating 0xEC / 0x11
        byte[] bytes = w.toBytes();
        byte[] result = new byte[cfg.dataCw()];
        System.arraycopy(bytes, 0, result, 0, Math.min(bytes.length, cfg.dataCw()));
        byte pad = (byte) 0xEC;
        for (int i = bytes.length; i < cfg.dataCw(); i++) {
            result[i] = pad;
            pad = (pad == (byte) 0xEC) ? (byte) 0x11 : (byte) 0xEC;
        }
        return result;
    }

    /**
     * Returns the mode indicator value for the given mode and symbol configuration.
     *
     * <pre>
     * M1 (0 bits): no indicator needed — only numeric mode exists
     * M2 (1 bit):  0=numeric, 1=alphanumeric
     * M3 (2 bits): 00=numeric, 01=alphanumeric, 10=byte
     * M4 (3 bits): 000=numeric, 001=alphanumeric, 010=byte, 011=kanji
     * </pre>
     */
    private static int modeIndicatorValue(EncodingMode mode, SymbolConfig cfg) {
        return switch (cfg.modeIndicatorBits()) {
            case 0 -> 0;
            case 1 -> (mode == EncodingMode.NUMERIC) ? 0 : 1;
            case 2 -> switch (mode) {
                case NUMERIC      -> 0b00;
                case ALPHANUMERIC -> 0b01;
                case BYTE         -> 0b10;
            };
            case 3 -> switch (mode) {
                case NUMERIC      -> 0b000;
                case ALPHANUMERIC -> 0b001;
                case BYTE         -> 0b010;
            };
            default -> 0;
        };
    }

    /**
     * Returns the character count field width for the given mode and symbol.
     *
     * <pre>
     * Mode         | M1 | M2 | M3 | M4
     * -------------|----|----|----|----|
     * Numeric      |  3 |  4 |  5 |  6
     * Alphanumeric |  — |  3 |  4 |  5
     * Byte         |  — |  — |  4 |  5
     * </pre>
     */
    private static int charCountBits(EncodingMode mode, SymbolConfig cfg) {
        return switch (mode) {
            case NUMERIC      -> cfg.ccBitsNumeric();
            case ALPHANUMERIC -> cfg.ccBitsAlpha();
            case BYTE         -> cfg.ccBitsByte();
        };
    }

    /**
     * Encode numeric string: groups of 3 → 10 bits, pair → 7 bits, single → 4 bits.
     *
     * <p>Example: {@code "12345"} → groups {@code "123"}, {@code "45"}
     * → 10-bit value 123, 7-bit value 45.
     *
     * <p>This is identical to standard QR Code numeric encoding.
     */
    private static void encodeNumeric(String input, BitWriter w) {
        int i = 0;
        while (i + 2 < input.length()) {
            int val = (input.charAt(i)   - '0') * 100
                    + (input.charAt(i+1) - '0') * 10
                    + (input.charAt(i+2) - '0');
            w.write(val, 10);
            i += 3;
        }
        if (i + 1 < input.length()) {
            int val = (input.charAt(i) - '0') * 10 + (input.charAt(i+1) - '0');
            w.write(val, 7);
            i += 2;
        }
        if (i < input.length()) {
            w.write(input.charAt(i) - '0', 4);
        }
    }

    /**
     * Encode alphanumeric string: pairs → 11 bits, single → 6 bits.
     *
     * <p>Each pair of characters is packed into 11 bits as:
     * {@code first_index × 45 + second_index}.
     * A trailing single character uses 6 bits.
     *
     * <p>This is identical to standard QR Code alphanumeric encoding.
     */
    private static void encodeAlphanumeric(String input, BitWriter w) {
        int i = 0;
        while (i + 1 < input.length()) {
            int a = ALPHANUM_CHARS.indexOf(input.charAt(i));
            int b = ALPHANUM_CHARS.indexOf(input.charAt(i + 1));
            w.write(a * 45 + b, 11);
            i += 2;
        }
        if (i < input.length()) {
            int a = ALPHANUM_CHARS.indexOf(input.charAt(i));
            w.write(a, 6);
        }
    }

    /**
     * Encode byte mode: each UTF-8 byte → 8 bits.
     *
     * <p>Multi-byte UTF-8 sequences are each treated as individual byte values.
     * The character count field counts bytes, not Unicode code points.
     */
    private static void encodeByteMode(String input, BitWriter w) {
        for (byte b : input.getBytes()) {
            w.write(b & 0xFF, 8);
        }
    }

    // =========================================================================
    // Reed-Solomon encoder
    // =========================================================================

    /**
     * Compute ECC bytes using LFSR polynomial division over GF(256)/0x11D.
     *
     * <p>Returns the remainder of {@code D(x)·x^n mod G(x)}.
     * Uses the b=0 convention (first root is α^0 = 1).
     *
     * <p>This algorithm is identical to the QR Code RS encoder.  It runs in
     * O(k·n) where k = number of data codewords and n = number of ECC codewords.
     *
     * <pre>
     * Algorithm (LFSR / polynomial long division):
     *   ecc = [0] × n
     *   for each data byte b:
     *       feedback = b XOR ecc[0]
     *       shift ecc left by one position
     *       for i in 0..n-1:
     *           ecc[i] ^= GF.mul(generator[i+1], feedback)
     *   result = ecc
     * </pre>
     *
     * @param data      Data codewords.
     * @param generator Monic generator polynomial (length = eccCount + 1).
     * @return ECC codewords (length = eccCount).
     */
    static byte[] rsEncode(byte[] data, int[] generator) {
        int n = generator.length - 1;
        int[] rem = new int[n];
        for (byte b : data) {
            int fb = (b & 0xFF) ^ rem[0];
            // Shift register left (discard rem[0], push 0 at end)
            System.arraycopy(rem, 1, rem, 0, n - 1);
            rem[n - 1] = 0;
            if (fb != 0) {
                for (int i = 0; i < n; i++) {
                    rem[i] ^= GF256.mul(generator[i + 1], fb);
                }
            }
        }
        byte[] result = new byte[n];
        for (int i = 0; i < n; i++) {
            result[i] = (byte) rem[i];
        }
        return result;
    }

    // =========================================================================
    // Grid construction
    // =========================================================================

    /**
     * Working mutable grid holding module values and reservation flags.
     *
     * <p>Unlike the immutable {@link ModuleGrid} returned to callers, this
     * internal class is mutable — it is used only during encoding and discarded
     * after the final grid is built.
     *
     * <p>{@code modules[row][col] = true} means a dark module;
     * {@code reserved[row][col] = true} means the module is structural and
     * must not be touched during data placement or masking.
     */
    private static final class WorkGrid {
        final int size;
        final boolean[][] modules;
        final boolean[][] reserved;

        WorkGrid(int size) {
            this.size = size;
            this.modules  = new boolean[size][size];
            this.reserved = new boolean[size][size];
        }

        void set(int row, int col, boolean dark, boolean reserve) {
            modules[row][col] = dark;
            if (reserve) reserved[row][col] = true;
        }
    }

    /**
     * Place the 7×7 finder pattern at the top-left corner (rows 0–6, cols 0–6).
     *
     * <pre>
     * ■ ■ ■ ■ ■ ■ ■
     * ■ □ □ □ □ □ ■
     * ■ □ ■ ■ ■ □ ■
     * ■ □ ■ ■ ■ □ ■
     * ■ □ ■ ■ ■ □ ■
     * ■ □ □ □ □ □ ■
     * ■ ■ ■ ■ ■ ■ ■
     * </pre>
     *
     * <p>This is exactly the same 7×7 finder as regular QR Code.  The 1:1:3:1:1
     * ratio of dark-light-dark columns/rows is what scanners use to detect the
     * symbol.  Because Micro QR has only one finder (top-left), orientation is
     * unambiguous — the data area is always to the bottom-right.
     */
    private static void placeFinder(WorkGrid g) {
        for (int dr = 0; dr < 7; dr++) {
            for (int dc = 0; dc < 7; dc++) {
                boolean onBorder = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                boolean inCore   = dr >= 2 && dr <= 4 && dc >= 2 && dc <= 4;
                g.set(dr, dc, onBorder || inCore, true);
            }
        }
    }

    /**
     * Place the L-shaped separator (light modules at row 7 cols 0–7 and col 7 rows 0–7).
     *
     * <p>Unlike regular QR which surrounds all three finders with a full-perimeter
     * separator, Micro QR's single finder only needs separation on its bottom and
     * right sides.  The top and left sides of the finder are the symbol boundary.
     *
     * <p>The corner module at (row 7, col 7) is covered by both the horizontal
     * and vertical strips — it is simply light, and both iterations writing it
     * produce the same value.
     */
    private static void placeSeparator(WorkGrid g) {
        for (int i = 0; i <= 7; i++) {
            g.set(7, i, false, true);  // bottom strip: row 7, cols 0–7
            g.set(i, 7, false, true);  // right strip:  col 7, rows 0–7
        }
    }

    /**
     * Place timing pattern extensions along row 0 and col 0.
     *
     * <p>Micro QR places timing patterns on the outer edges of the finder
     * (row 0 and col 0) and extends them to the far edge of the symbol.
     * This differs from regular QR, which places timing on row 6 and col 6.
     *
     * <p>Positions 0–6: already set by the finder pattern.
     * Position 7: separator (always light).
     * Position 8 onward: alternating dark/light, dark at even index.
     */
    private static void placeTiming(WorkGrid g) {
        int sz = g.size;
        // Horizontal timing: row 0, cols 8 to size-1
        for (int c = 8; c < sz; c++) {
            g.set(0, c, c % 2 == 0, true);
        }
        // Vertical timing: col 0, rows 8 to size-1
        for (int r = 8; r < sz; r++) {
            g.set(r, 0, r % 2 == 0, true);
        }
    }

    /**
     * Reserve the 15 format information module positions.
     *
     * <p>These modules are reserved before data placement so the zigzag
     * algorithm skips them.  They are filled with the actual format word
     * after mask selection.
     *
     * <pre>
     * Row 8, cols 1–8 → bits f14..f7 (MSB first, f14 at col 1)
     * Col 8, rows 1–7 → bits f6..f0  (f6 at row 7, f0 at row 1)
     * </pre>
     *
     * <p>The format info occupies exactly 15 modules — one per bit.
     */
    private static void reserveFormatInfo(WorkGrid g) {
        for (int c = 1; c <= 8; c++) g.set(8, c, false, true);
        for (int r = 1; r <= 7; r++) g.set(r, 8, false, true);
    }

    /**
     * Write the 15-bit format word into the reserved positions.
     *
     * <p>Bit f14 (MSB) is placed first.  The L-shaped strip goes rightward
     * along row 8 then upward along col 8:
     *
     * <pre>
     * Row 8, col 1  ← f14  (MSB)
     * Row 8, col 2  ← f13
     * ...
     * Row 8, col 8  ← f7
     * Col 8, row 7  ← f6
     * Col 8, row 6  ← f5
     * ...
     * Col 8, row 1  ← f0   (LSB)
     * </pre>
     *
     * @param g   The work grid to write into.
     * @param fmt The 15-bit format word.
     */
    private static void writeFormatInfo(boolean[][] modules, int fmt) {
        // Row 8, cols 1–8: bits f14 down to f7
        for (int i = 0; i < 8; i++) {
            modules[8][1 + i] = ((fmt >> (14 - i)) & 1) == 1;
        }
        // Col 8, rows 7 down to 1: bits f6 down to f0
        for (int i = 0; i < 7; i++) {
            modules[7 - i][8] = ((fmt >> (6 - i)) & 1) == 1;
        }
    }

    /**
     * Initialize the grid with all structural modules.
     *
     * <p>Calls the four placement functions in order:
     * finder → separator → timing → format info reservation.
     * After this call, all reserved modules are set and the remaining
     * modules are ready for data placement.
     */
    private static WorkGrid buildGrid(SymbolConfig cfg) {
        WorkGrid g = new WorkGrid(cfg.size());
        placeFinder(g);
        placeSeparator(g);
        placeTiming(g);
        reserveFormatInfo(g);
        return g;
    }

    // =========================================================================
    // Data placement (two-column zigzag)
    // =========================================================================

    /**
     * Place bits from the final codeword stream into the grid via two-column zigzag.
     *
     * <p>Scans from the bottom-right corner, moving left two columns at a time,
     * alternating upward and downward directions.  Reserved modules are skipped.
     *
     * <p>Unlike regular QR, there is no timing column at col 6 to hop over —
     * Micro QR's timing is at col 0, which is reserved and auto-skipped.
     *
     * <pre>
     * col = size - 1   (start at rightmost column)
     * dir = UP
     *
     * while col &gt;= 1:
     *   for each row in current direction:
     *     for sub_col in [col, col-1]:
     *       if reserved: skip
     *       place bit
     *   flip direction
     *   col -= 2
     * </pre>
     *
     * @param g    The work grid.
     * @param bits The bit stream to place (boolean array, true = dark).
     */
    private static void placeBits(WorkGrid g, boolean[] bits) {
        int sz = g.size;
        int bitIdx = 0;
        boolean up = true;

        for (int col = sz - 1; col >= 1; col -= 2) {
            for (int vi = 0; vi < sz; vi++) {
                int row = up ? (sz - 1 - vi) : vi;
                for (int dc = 0; dc <= 1; dc++) {
                    int c = col - dc;
                    if (g.reserved[row][c]) continue;
                    g.modules[row][c] = (bitIdx < bits.length) && bits[bitIdx++];
                }
            }
            up = !up;
        }
    }

    // =========================================================================
    // Masking
    // =========================================================================

    /**
     * Returns true if mask pattern {@code maskIdx} applies to module (row, col).
     *
     * <p>The 4 Micro QR mask conditions (subset of regular QR's 8):
     *
     * <pre>
     * Pattern 0: (row + col) mod 2 == 0
     * Pattern 1: row mod 2 == 0
     * Pattern 2: col mod 3 == 0
     * Pattern 3: (row + col) mod 3 == 0
     * </pre>
     *
     * <p>The more complex patterns 4–7 from regular QR are not used in Micro QR.
     * The smaller symbol size means the simpler patterns are sufficient to break
     * up degenerate all-dark or all-light sequences.
     *
     * @param maskIdx The mask pattern index (0–3).
     * @param row     Module row.
     * @param col     Module column.
     * @return True if the mask should flip this module.
     */
    static boolean maskCondition(int maskIdx, int row, int col) {
        return switch (maskIdx) {
            case 0 -> (row + col) % 2 == 0;
            case 1 -> row % 2 == 0;
            case 2 -> col % 3 == 0;
            case 3 -> (row + col) % 3 == 0;
            default -> false;
        };
    }

    /**
     * Apply mask pattern to all non-reserved modules, returning a new module array.
     *
     * <p>A mask XORs (flips) module values: if the mask condition is true for
     * a non-reserved module, the module's dark/light value is inverted.  This
     * breaks up degenerate patterns that could confuse scanner detection.
     *
     * <p>Masking is applied <em>only</em> to data and ECC modules — structural
     * modules (finder, separator, timing, format info) are never masked.
     *
     * @param modules  The unmasked module array.
     * @param reserved The reservation flags.
     * @param sz       Symbol side length.
     * @param maskIdx  Mask pattern to apply (0–3).
     * @return A new module array with the mask applied.
     */
    private static boolean[][] applyMask(
            boolean[][] modules, boolean[][] reserved, int sz, int maskIdx) {
        boolean[][] result = new boolean[sz][sz];
        for (int r = 0; r < sz; r++) {
            for (int c = 0; c < sz; c++) {
                if (!reserved[r][c]) {
                    result[r][c] = modules[r][c] != maskCondition(maskIdx, r, c);
                } else {
                    result[r][c] = modules[r][c];
                }
            }
        }
        return result;
    }

    // =========================================================================
    // Penalty scoring
    // =========================================================================

    /**
     * Compute the 4-rule penalty score (same rules as regular QR Code).
     *
     * <p>The four rules penalize patterns that could interfere with scanner
     * detection.  All four masks are evaluated and the one with the lowest
     * penalty is selected.
     *
     * <h3>Rule 1 — Adjacent run penalty</h3>
     * <p>Scan each row and each column for runs of ≥5 consecutive modules of
     * the same color.  Add {@code run_length − 2} to the penalty for each
     * qualifying run.  (Run of 5 → +3, run of 6 → +4, etc.)
     *
     * <h3>Rule 2 — 2×2 block penalty</h3>
     * <p>For each 2×2 square with all four modules the same color (all dark
     * or all light), add 3 to the penalty.
     *
     * <h3>Rule 3 — Finder-pattern-like sequences</h3>
     * <p>Scan all rows and columns for the 11-module sequences
     * {@code 1 0 1 1 1 0 1 0 0 0 0} or its reverse {@code 0 0 0 0 1 0 1 1 1 0 1}.
     * Each occurrence adds 40 to the penalty.
     *
     * <h3>Rule 4 — Dark-module proportion</h3>
     * <p>Penalizes symbols that are heavily dark or heavily light.
     * See inline comments for the formula.
     *
     * @param modules The (masked) module array.
     * @param sz      Symbol side length.
     * @return The total penalty score.
     */
    static int computePenalty(boolean[][] modules, int sz) {
        int penalty = 0;

        // ── Rule 1: adjacent same-color runs of ≥ 5 ─────────────────────────
        for (int a = 0; a < sz; a++) {
            // Check row a
            int run = 1;
            boolean prev = modules[a][0];
            for (int i = 1; i < sz; i++) {
                boolean cur = modules[a][i];
                if (cur == prev) {
                    run++;
                } else {
                    if (run >= 5) penalty += run - 2;
                    run = 1;
                    prev = cur;
                }
            }
            if (run >= 5) penalty += run - 2;

            // Check column a
            run = 1;
            prev = modules[0][a];
            for (int i = 1; i < sz; i++) {
                boolean cur = modules[i][a];
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

        // ── Rule 2: 2×2 same-color blocks ────────────────────────────────────
        for (int r = 0; r < sz - 1; r++) {
            for (int c = 0; c < sz - 1; c++) {
                boolean d = modules[r][c];
                if (d == modules[r][c+1] && d == modules[r+1][c] && d == modules[r+1][c+1]) {
                    penalty += 3;
                }
            }
        }

        // ── Rule 3: finder-pattern-like sequences ─────────────────────────────
        // The patterns look like a finder to a scanner and must be avoided.
        // Requires at least 11 modules in a line to match, so skip if sz < 11.
        if (sz >= 11) {
        int[] p1 = {1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0};
        int[] p2 = {0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1};
        int limit = sz - 11;
        for (int a = 0; a < sz; a++) {
            for (int b = 0; b <= limit; b++) {
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
        } // end if (sz >= 11)

        // ── Rule 4: dark proportion deviation from 50% ───────────────────────
        // Count dark modules, compute percent, find nearest multiples of 5.
        // Penalty = min(|prev5 - 50|, |next5 - 50|) / 5 × 10.
        int dark = 0;
        for (boolean[] row : modules) {
            for (boolean m : row) {
                if (m) dark++;
            }
        }
        int total = sz * sz;
        int darkPct = (dark * 100) / total;
        int prev5 = (darkPct / 5) * 5;
        int next5 = prev5 + 5;
        int r4 = Math.min(Math.abs(prev5 - 50), Math.abs(next5 - 50));
        penalty += (r4 / 5) * 10;

        return penalty;
    }

    // =========================================================================
    // Main encoding logic (private, used by public encode() method)
    // =========================================================================

    /**
     * Core encoding: given a resolved config and mode, produce the final grid.
     *
     * <p>Steps:
     * <ol>
     *   <li>Build data codewords from the input bit stream.</li>
     *   <li>Compute RS ECC codewords and append to data.</li>
     *   <li>Flatten codeword stream to a bit array (M1: last data codeword = 4 bits).</li>
     *   <li>Initialize grid with structural modules.</li>
     *   <li>Place data bits via two-column zigzag.</li>
     *   <li>Evaluate all 4 masks, pick lowest penalty.</li>
     *   <li>Apply best mask and write final format info.</li>
     *   <li>Wrap in immutable {@link ModuleGrid} and return.</li>
     * </ol>
     */
    private static ModuleGrid encodeWithConfig(
            String input, SymbolConfig cfg, EncodingMode mode) {

        // Step 1: Build data codewords
        byte[] dataCw = buildDataCodewords(input, cfg, mode);

        // Step 2: Compute RS ECC
        int[] gen = getGenerator(cfg.eccCw());
        byte[] eccCw = rsEncode(dataCw, gen);

        // Step 3: Flatten to bit stream
        // For M1: data[dataCw-1] has data in upper 4 bits → contribute only 4 bits.
        int totalCws = dataCw.length + eccCw.length;
        // Count total bits
        int totalBits = 0;
        for (int i = 0; i < dataCw.length; i++) {
            totalBits += (cfg.m1HalfCw() && i == cfg.dataCw() - 1) ? 4 : 8;
        }
        totalBits += eccCw.length * 8;

        boolean[] bits = new boolean[totalBits];
        int bitIdx = 0;

        for (int i = 0; i < dataCw.length; i++) {
            int bitsInCw = (cfg.m1HalfCw() && i == cfg.dataCw() - 1) ? 4 : 8;
            int cw = dataCw[i] & 0xFF;
            for (int b = bitsInCw - 1; b >= 0; b--) {
                bits[bitIdx++] = ((cw >> (b + (8 - bitsInCw))) & 1) == 1;
            }
        }
        for (byte b : eccCw) {
            int cw = b & 0xFF;
            for (int bit = 7; bit >= 0; bit--) {
                bits[bitIdx++] = ((cw >> bit) & 1) == 1;
            }
        }

        // Step 4: Initialize grid
        WorkGrid grid = buildGrid(cfg);

        // Step 5: Place data bits
        placeBits(grid, bits);

        // Step 6: Evaluate all 4 masks, pick lowest penalty
        int bestMask = 0;
        int bestPenalty = Integer.MAX_VALUE;

        for (int m = 0; m < 4; m++) {
            boolean[][] masked = applyMask(grid.modules, grid.reserved, cfg.size(), m);
            int fmt = FORMAT_TABLE[cfg.symbolIndicator()][m];

            // Write format info into a temporary copy for penalty evaluation
            boolean[][] tmp = new boolean[cfg.size()][];
            for (int r = 0; r < cfg.size(); r++) {
                tmp[r] = masked[r].clone();
            }
            writeFormatInfo(tmp, fmt);

            int p = computePenalty(tmp, cfg.size());
            if (p < bestPenalty) {
                bestPenalty = p;
                bestMask = m;
            }
        }

        // Step 7: Apply best mask and write final format info
        boolean[][] finalModules = applyMask(grid.modules, grid.reserved, cfg.size(), bestMask);
        int finalFmt = FORMAT_TABLE[cfg.symbolIndicator()][bestMask];
        writeFormatInfo(finalModules, finalFmt);

        // Step 8: Wrap in immutable ModuleGrid
        int sz = cfg.size();
        List<List<Boolean>> rows = new ArrayList<>(sz);
        for (int r = 0; r < sz; r++) {
            List<Boolean> row = new ArrayList<>(sz);
            for (int c = 0; c < sz; c++) {
                row.add(finalModules[r][c]);
            }
            rows.add(Collections.unmodifiableList(row));
        }

        return new ModuleGrid(sz, sz, Collections.unmodifiableList(rows), ModuleShape.SQUARE);
    }

    // =========================================================================
    // Bit writer helper
    // =========================================================================

    /**
     * Accumulates bits MSB-first, then converts to byte array or bit array.
     *
     * <p>Each call to {@link #write(int, int)} appends {@code count}
     * least-significant bits of {@code value} to the stream, MSB first.
     * This matches QR/Micro-QR's big-endian bit ordering within each codeword.
     *
     * <p>Example: {@code write(0b101, 3)} appends bits 1, 0, 1.
     */
    private static final class BitWriter {
        private final List<Integer> bits = new ArrayList<>();

        /** Append the {@code count} LSBs of {@code value}, MSB first. */
        void write(int value, int count) {
            for (int i = count - 1; i >= 0; i--) {
                bits.add((value >> i) & 1);
            }
        }

        /** Returns the number of bits written so far. */
        int bitLen() { return bits.size(); }

        /**
         * Convert bit stream to byte array.
         *
         * <p>Groups of 8 bits are packed into one byte (MSB first).  If
         * the number of bits is not a multiple of 8, the last byte is
         * zero-padded on the right.
         */
        byte[] toBytes() {
            int nBytes = (bits.size() + 7) / 8;
            byte[] result = new byte[nBytes];
            for (int i = 0; i < bits.size(); i++) {
                if (bits.get(i) == 1) {
                    result[i / 8] |= (byte)(1 << (7 - (i % 8)));
                }
            }
            return result;
        }

        /**
         * Return the bit stream as a plain int array (each element 0 or 1).
         * Used by the M1 half-codeword packing logic.
         */
        int[] toBitArray() {
            int[] arr = new int[bits.size()];
            for (int i = 0; i < bits.size(); i++) arr[i] = bits.get(i);
            return arr;
        }
    }
}
