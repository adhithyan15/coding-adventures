package com.codingadventures.itf

import com.codingadventures.barcodelayout1d.BarcodeLayout1DV1
import com.codingadventures.paintinstructions.PaintScene

/**
 * Pure Kotlin implementation of the Interleaved 2 of 5 (ITF) 1D barcode symbology.
 *
 * Literate Design & Symbology Mechanics (AIM-BC2 / ISO/IEC 16390):
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
object Itf {
    val version: String
        get() = ItfConstants.VERSION

    /**
     * Validates and normalizes ITF payload data.
     *
     * Validation precedence:
     * 1. Null or unpaired UTF-16 surrogates -> IllegalArgumentException
     * 2. Unicode scalar count > 4096 -> input-too-long
     * 3. Unicode scalar count is 0 or odd -> invalid-length
     * 4. Strict ASCII digits ('0'-'9') only -> invalid-character
     */
    @JvmStatic
    fun normalizeItf(data: String?): String {
        if (data == null) {
            throw IllegalArgumentException(ItfMessages.MESSAGE_NULL_INPUT)
        }

        var scalarCount = 0
        var i = 0
        while (i < data.length) {
            val ch = data[i]
            if (ch.isHighSurrogate()) {
                i++
                if (i >= data.length || !data[i].isLowSurrogate()) {
                    throw IllegalArgumentException(ItfMessages.MESSAGE_INVALID_SCALARS)
                }
            } else if (ch.isLowSurrogate()) {
                throw IllegalArgumentException(ItfMessages.MESSAGE_INVALID_SCALARS)
            }
            scalarCount++
            i++
        }

        if (scalarCount > ItfConstants.MAX_SCALAR_LIMIT) {
            throw InvalidItfInputException(ItfErrorCode.INPUT_TOO_LONG)
        }

        if (scalarCount == 0 || scalarCount % ItfConstants.PAIR_LENGTH != 0) {
            throw InvalidItfInputException(ItfErrorCode.INVALID_LENGTH)
        }

        for (idx in 0 until data.length) {
            val ch = data[idx]
            if (ch < ItfConstants.ASCII_ZERO || ch > ItfConstants.ASCII_NINE) {
                throw InvalidItfInputException(ItfErrorCode.INVALID_CHARACTER)
            }
        }

        return data
    }

    private fun encodePair(pair: String, sourceIndex: Int): EncodedPair {
        val firstDigit = pair[0] - ItfConstants.ASCII_ZERO
        val secondDigit = pair[1] - ItfConstants.ASCII_ZERO
        val barPattern = ItfConstants.DIGIT_PATTERNS[firstDigit]
        val spacePattern = ItfConstants.DIGIT_PATTERNS[secondDigit]

        val sb = StringBuilder()
        for (i in 0 until ItfConstants.DIGIT_ELEMENT_COUNT) {
            sb.append(if (barPattern[i] == ItfConstants.WIDE_MARKER) ItfConstants.WIDE_BAR else ItfConstants.NARROW_BAR)
            sb.append(if (spacePattern[i] == ItfConstants.WIDE_MARKER) ItfConstants.WIDE_SPACE else ItfConstants.NARROW_SPACE)
        }

        return EncodedPair(pair, barPattern, spacePattern, sb.toString(), sourceIndex)
    }

    /**
     * Encodes normalized ITF data into pairs of interleaved bar/space elements.
     */
    @JvmStatic
    fun encodeItf(data: String?): List<EncodedPair> {
        val normalized = normalizeItf(data)
        val pairs = ArrayList<EncodedPair>(normalized.length / ItfConstants.PAIR_LENGTH)
        var i = 0
        while (i < normalized.length) {
            val pair = normalized.substring(i, i + ItfConstants.PAIR_LENGTH)
            pairs.add(encodePair(pair, i / ItfConstants.PAIR_LENGTH))
            i += ItfConstants.PAIR_LENGTH
        }
        return java.util.List.copyOf(pairs)
    }

    private fun expandEncodedPairs(encodedPairs: List<EncodedPair>): List<BarcodeLayout1DV1.Run> {
        val runs = ArrayList<BarcodeLayout1DV1.Run>()

        runs.addAll(
            BarcodeLayout1DV1.expandBinary(
                ItfConstants.START_PATTERN,
                ItfConstants.LABEL_START,
                ItfConstants.SOURCE_INDEX_START,
                ItfConstants.ROLE_START
            )
        )

        for (pair in encodedPairs) {
            runs.addAll(
                BarcodeLayout1DV1.expandBinary(
                    pair.binaryPattern,
                    pair.pair,
                    pair.sourceIndex.toLong(),
                    ItfConstants.ROLE_DATA
                )
            )
        }

        runs.addAll(
            BarcodeLayout1DV1.expandBinary(
                ItfConstants.STOP_PATTERN,
                ItfConstants.LABEL_STOP,
                ItfConstants.SOURCE_INDEX_STOP,
                ItfConstants.ROLE_STOP
            )
        )

        return java.util.List.copyOf(runs)
    }

    /**
     * Expands ITF barcode runs including start pattern, interleaved pairs, and stop pattern.
     */
    @JvmStatic
    fun expandItfRuns(data: String?): List<BarcodeLayout1DV1.Run> {
        val encodedPairs = encodeItf(data)
        return expandEncodedPairs(encodedPairs)
    }

    /**
     * Projects the ITF barcode into a 2D vector paint scene with quiet zones and metadata.
     */
    @JvmStatic
    @JvmOverloads
    fun layoutItf(data: String?, options: BarcodeLayout1DV1.SceneOptions? = null): PaintScene {
        val encodedPairs = encodeItf(data)
        val runs = expandEncodedPairs(encodedPairs)

        val symbols = ArrayList<BarcodeLayout1DV1.Symbol>(encodedPairs.size + ItfConstants.FRAMING_SYMBOL_COUNT)
        symbols.add(
            BarcodeLayout1DV1.Symbol(
                ItfConstants.LABEL_START,
                ItfConstants.START_PATTERN.length,
                ItfConstants.SOURCE_INDEX_START,
                ItfConstants.ROLE_START
            )
        )
        for (pair in encodedPairs) {
            symbols.add(
                BarcodeLayout1DV1.Symbol(
                    pair.pair,
                    pair.binaryPattern.length,
                    pair.sourceIndex.toLong(),
                    ItfConstants.ROLE_DATA
                )
            )
        }
        symbols.add(
            BarcodeLayout1DV1.Symbol(
                ItfConstants.LABEL_STOP,
                ItfConstants.STOP_PATTERN.length,
                ItfConstants.SOURCE_INDEX_STOP,
                ItfConstants.ROLE_STOP
            )
        )

        val callerMetadata = options?.metadata ?: emptyMap()
        val metadata = HashMap<String, String>(callerMetadata)
        metadata[ItfConstants.METADATA_KEY_SYMBOLOGY] = ItfConstants.METADATA_VALUE_SYMBOLOGY
        metadata[ItfConstants.METADATA_KEY_PAIR_COUNT] = encodedPairs.size.toString()

        val label = options?.label ?: ItfConstants.DEFAULT_SCENE_LABEL
        val moduleWidth = options?.moduleWidth ?: ItfConstants.DEFAULT_MODULE_WIDTH
        val barHeight = options?.barHeight ?: ItfConstants.DEFAULT_BAR_HEIGHT
        val foreground = options?.foreground ?: ItfConstants.DEFAULT_FOREGROUND
        val background = options?.background ?: ItfConstants.DEFAULT_BACKGROUND
        val humanReadableText = options?.humanReadableText
        val includeHrt = options?.includeHumanReadableText ?: false

        val mergedOptions = BarcodeLayout1DV1.SceneOptions(
            moduleWidth,
            barHeight,
            foreground,
            background,
            label,
            java.util.Map.copyOf(metadata),
            humanReadableText,
            includeHrt,
            java.util.List.copyOf(symbols)
        )

        return BarcodeLayout1DV1.projectScene(runs, ItfConstants.QUIET_ZONE_MODULES, mergedOptions)
    }

    /**
     * Alias for [layoutItf].
     */
    @JvmStatic
    @JvmOverloads
    fun drawItf(data: String?, options: BarcodeLayout1DV1.SceneOptions? = null): PaintScene {
        return layoutItf(data, options)
    }
}
