package com.codingadventures.itf;

import com.codingadventures.barcodelayout1d.BarcodeLayout1DV1;
import com.codingadventures.paintinstructions.PaintScene;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/**
 * Pure Java implementation of the Interleaved 2 of 5 (ITF) 1D barcode symbology.
 *
 * <p>Literate Design &amp; Symbology Mechanics (AIM-BC2 / ISO/IEC 16390):
 * 1. Dual-digit interleaving: ITF achieves compact numeric representation by encoding
 *    digit pairs together. The first digit defines the 5 bar widths (2 wide, 3 narrow),
 *    while the second digit defines the 5 interleaved space widths (2 wide, 3 narrow),
 *    yielding exactly 10 runs per pair.
 * 2. Scalar counting vs. code units: ITF string length constraints apply strictly to
 *    Unicode scalar values (code points), not raw UTF-16 code units. Lone surrogate halves
 *    are rejected immediately prior to capacity counting to prevent truncation and smuggling.
 * 3. Strict ASCII digits: Symbology standard restricts values to ASCII '0'-'9' (0x30..0x39).
 *    Fullwidth and localized Unicode digits must never be coerced.
 */
public final class Itf {
    private Itf() {}

    public static String getVersion() {
        return ItfConstants.VERSION;
    }

    /**
     * Validates and normalizes ITF payload data.
     *
     * <p>Validation precedence:
     * 1. Null or unpaired UTF-16 surrogates -> IllegalArgumentException
     * 2. Unicode scalar count > 4096 -> input-too-long
     * 3. Unicode scalar count is 0 or odd -> invalid-length
     * 4. Strict ASCII digits ('0'-'9') only -> invalid-character
     *
     * @param data input string
     * @return the verified data
     */
    public static String normalizeItf(String data) {
        if (data == null) {
            throw new IllegalArgumentException(ItfMessages.MESSAGE_NULL_INPUT);
        }

        int scalarCount = 0;
        for (int i = 0; i < data.length(); i++) {
            char ch = data.charAt(i);
            if (Character.isHighSurrogate(ch)) {
                if (++i >= data.length() || !Character.isLowSurrogate(data.charAt(i))) {
                    throw new IllegalArgumentException(ItfMessages.MESSAGE_INVALID_SCALARS);
                }
            } else if (Character.isLowSurrogate(ch)) {
                throw new IllegalArgumentException(ItfMessages.MESSAGE_INVALID_SCALARS);
            }
            scalarCount++;
        }

        if (scalarCount > ItfConstants.MAX_SCALAR_LIMIT) {
            throw new InvalidItfInputException(ItfErrorCode.INPUT_TOO_LONG);
        }

        if (scalarCount == 0 || scalarCount % ItfConstants.PAIR_LENGTH != 0) {
            throw new InvalidItfInputException(ItfErrorCode.INVALID_LENGTH);
        }

        for (int i = 0; i < data.length(); i++) {
            char ch = data.charAt(i);
            if (ch < ItfConstants.ASCII_ZERO || ch > ItfConstants.ASCII_NINE) {
                throw new InvalidItfInputException(ItfErrorCode.INVALID_CHARACTER);
            }
        }

        return data;
    }

    private static EncodedPair encodePair(String pair, int sourceIndex) {
        int firstDigit = pair.charAt(0) - ItfConstants.ASCII_ZERO;
        int secondDigit = pair.charAt(1) - ItfConstants.ASCII_ZERO;
        String barPattern = ItfConstants.DIGIT_PATTERNS.get(firstDigit);
        String spacePattern = ItfConstants.DIGIT_PATTERNS.get(secondDigit);

        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < ItfConstants.DIGIT_ELEMENT_COUNT; i++) {
            sb.append(barPattern.charAt(i) == ItfConstants.WIDE_MARKER ? ItfConstants.WIDE_BAR : ItfConstants.NARROW_BAR);
            sb.append(spacePattern.charAt(i) == ItfConstants.WIDE_MARKER ? ItfConstants.WIDE_SPACE : ItfConstants.NARROW_SPACE);
        }

        return new EncodedPair(pair, barPattern, spacePattern, sb.toString(), sourceIndex);
    }

    /**
     * Encodes normalized ITF data into pairs of interleaved bar/space elements.
     *
     * @param data valid numeric ITF payload
     * @return immutable list of encoded digit pairs
     */
    public static List<EncodedPair> encodeItf(String data) {
        String normalized = normalizeItf(data);
        List<EncodedPair> pairs = new ArrayList<>(normalized.length() / ItfConstants.PAIR_LENGTH);
        for (int i = 0; i < normalized.length(); i += ItfConstants.PAIR_LENGTH) {
            String pair = normalized.substring(i, i + ItfConstants.PAIR_LENGTH);
            pairs.add(encodePair(pair, i / ItfConstants.PAIR_LENGTH));
        }
        return List.copyOf(pairs);
    }

    private static List<BarcodeLayout1DV1.Run> expandEncodedPairs(List<EncodedPair> encodedPairs) {
        List<BarcodeLayout1DV1.Run> runs = new ArrayList<>();

        runs.addAll(BarcodeLayout1DV1.expandBinary(
                ItfConstants.START_PATTERN,
                ItfConstants.LABEL_START,
                ItfConstants.SOURCE_INDEX_START,
                ItfConstants.ROLE_START
        ));

        for (EncodedPair pair : encodedPairs) {
            runs.addAll(BarcodeLayout1DV1.expandBinary(
                    pair.binaryPattern(),
                    pair.pair(),
                    pair.sourceIndex(),
                    ItfConstants.ROLE_DATA
            ));
        }

        runs.addAll(BarcodeLayout1DV1.expandBinary(
                ItfConstants.STOP_PATTERN,
                ItfConstants.LABEL_STOP,
                ItfConstants.SOURCE_INDEX_STOP,
                ItfConstants.ROLE_STOP
        ));

        return List.copyOf(runs);
    }

    /**
     * Expands ITF barcode runs including start pattern, interleaved pairs, and stop pattern.
     *
     * @param data valid numeric ITF payload
     * @return immutable list of 1D barcode runs
     */
    public static List<BarcodeLayout1DV1.Run> expandItfRuns(String data) {
        List<EncodedPair> encodedPairs = encodeItf(data);
        return expandEncodedPairs(encodedPairs);
    }

    /**
     * Projects the ITF barcode into a 2D vector paint scene with quiet zones and metadata.
     *
     * @param data valid numeric ITF payload
     * @param options layout options or null for defaults
     * @return paint scene
     */
    public static PaintScene layoutItf(String data, BarcodeLayout1DV1.SceneOptions options) {
        List<EncodedPair> encodedPairs = encodeItf(data);
        List<BarcodeLayout1DV1.Run> runs = expandEncodedPairs(encodedPairs);

        List<BarcodeLayout1DV1.Symbol> symbols = new ArrayList<>(encodedPairs.size() + ItfConstants.FRAMING_SYMBOL_COUNT);
        symbols.add(new BarcodeLayout1DV1.Symbol(
                ItfConstants.LABEL_START,
                ItfConstants.START_PATTERN.length(),
                ItfConstants.SOURCE_INDEX_START,
                ItfConstants.ROLE_START
        ));
        for (EncodedPair pair : encodedPairs) {
            symbols.add(new BarcodeLayout1DV1.Symbol(
                    pair.pair(),
                    pair.binaryPattern().length(),
                    pair.sourceIndex(),
                    ItfConstants.ROLE_DATA
            ));
        }
        symbols.add(new BarcodeLayout1DV1.Symbol(
                ItfConstants.LABEL_STOP,
                ItfConstants.STOP_PATTERN.length(),
                ItfConstants.SOURCE_INDEX_STOP,
                ItfConstants.ROLE_STOP
        ));

        Map<String, String> callerMetadata = (options != null && options.metadata() != null)
                ? options.metadata()
                : Map.of();
        Map<String, String> metadata = new HashMap<>(callerMetadata);
        metadata.put(ItfConstants.METADATA_KEY_SYMBOLOGY, ItfConstants.METADATA_VALUE_SYMBOLOGY);
        metadata.put(ItfConstants.METADATA_KEY_PAIR_COUNT, Integer.toString(encodedPairs.size()));

        String label = (options != null && options.label() != null)
                ? options.label()
                : ItfConstants.DEFAULT_SCENE_LABEL;

        int moduleWidth = (options != null) ? options.moduleWidth() : ItfConstants.DEFAULT_MODULE_WIDTH;
        int barHeight = (options != null) ? options.barHeight() : ItfConstants.DEFAULT_BAR_HEIGHT;
        String foreground = (options != null) ? options.foreground() : ItfConstants.DEFAULT_FOREGROUND;
        String background = (options != null) ? options.background() : ItfConstants.DEFAULT_BACKGROUND;
        String humanReadableText = (options != null) ? options.humanReadableText() : null;
        boolean includeHrt = (options != null) && options.includeHumanReadableText();

        BarcodeLayout1DV1.SceneOptions mergedOptions = new BarcodeLayout1DV1.SceneOptions(
                moduleWidth,
                barHeight,
                foreground,
                background,
                label,
                Map.copyOf(metadata),
                humanReadableText,
                includeHrt,
                List.copyOf(symbols)
        );

        return BarcodeLayout1DV1.projectScene(runs, ItfConstants.QUIET_ZONE_MODULES, mergedOptions);
    }

    /**
     * Alias for {@link #layoutItf(String, BarcodeLayout1DV1.SceneOptions)}.
     */
    public static PaintScene drawItf(String data, BarcodeLayout1DV1.SceneOptions options) {
        return layoutItf(data, options);
    }

    /**
     * Draws an ITF barcode scene using default rendering options.
     */
    public static PaintScene drawItf(String data) {
        return layoutItf(data, null);
    }
}
