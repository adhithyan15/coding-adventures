package com.codingadventures.itf

/**
 * Standard constants for Interleaved 2 of 5 (ITF) encoding and layout in Kotlin.
 */
object ItfConstants {
    const val VERSION: String = "0.1.0"

    const val START_PATTERN: String = "1010"
    const val STOP_PATTERN: String = "11101"

    val DIGIT_PATTERNS: List<String> = listOf(
        "00110", // 0: NNWWN
        "10001", // 1: WNNNW
        "01001", // 2: NWNNW
        "11000", // 3: WWNNN
        "00101", // 4: NNWNW
        "10100", // 5: WNWNN
        "01100", // 6: NWWNN
        "00011", // 7: NNNWW
        "10010", // 8: WNNWN
        "01010"  // 9: NWNWN
    )

    const val MAX_SCALAR_LIMIT: Int = 4096
    const val QUIET_ZONE_MODULES: Int = 10
    const val DIGIT_PAIR_RUN_COUNT: Int = 10
    const val DIGIT_ELEMENT_COUNT: Int = 5
    const val PAIR_LENGTH: Int = 2
    const val FRAMING_SYMBOL_COUNT: Int = 2

    const val ASCII_ZERO: Char = '0'
    const val ASCII_NINE: Char = '9'
    const val WIDE_MARKER: Char = '1'

    const val ROLE_START: String = "start"
    const val ROLE_DATA: String = "data"
    const val ROLE_STOP: String = "stop"

    const val LABEL_START: String = "start"
    const val LABEL_STOP: String = "stop"

    const val SOURCE_INDEX_START: Long = -1L
    const val SOURCE_INDEX_STOP: Long = -2L

    const val WIDE_BAR: String = "111"
    const val NARROW_BAR: String = "1"
    const val WIDE_SPACE: String = "000"
    const val NARROW_SPACE: String = "0"

    const val METADATA_KEY_SYMBOLOGY: String = "symbology"
    const val METADATA_VALUE_SYMBOLOGY: String = "itf"
    const val METADATA_KEY_PAIR_COUNT: String = "pairCount"

    const val DEFAULT_SCENE_LABEL: String = "ITF barcode"
    const val DEFAULT_FOREGROUND: String = "#000000"
    const val DEFAULT_BACKGROUND: String = "#ffffff"
    const val DEFAULT_MODULE_WIDTH: Int = 4
    const val DEFAULT_BAR_HEIGHT: Int = 120
}
