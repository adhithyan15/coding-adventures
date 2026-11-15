package com.codingadventures.itf;

import java.util.List;

/**
 * Standard constants for Interleaved 2 of 5 (ITF) encoding and layout.
 */
public final class ItfConstants {
    private ItfConstants() {}

    public static final String VERSION = "0.1.0";

    public static final String START_PATTERN = "1010";
    public static final String STOP_PATTERN = "11101";

    public static final List<String> DIGIT_PATTERNS = List.of(
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
    );

    public static final int MAX_SCALAR_LIMIT = 4096;
    public static final int QUIET_ZONE_MODULES = 10;
    public static final int DIGIT_PAIR_RUN_COUNT = 10;
    public static final int DIGIT_ELEMENT_COUNT = 5;
    public static final int PAIR_LENGTH = 2;
    public static final int FRAMING_SYMBOL_COUNT = 2;

    public static final char ASCII_ZERO = '0';
    public static final char ASCII_NINE = '9';
    public static final char WIDE_MARKER = '1';

    public static final String ROLE_START = "start";
    public static final String ROLE_DATA = "data";
    public static final String ROLE_STOP = "stop";

    public static final String LABEL_START = "start";
    public static final String LABEL_STOP = "stop";

    public static final int SOURCE_INDEX_START = -1;
    public static final int SOURCE_INDEX_STOP = -2;

    public static final String WIDE_BAR = "111";
    public static final String NARROW_BAR = "1";
    public static final String WIDE_SPACE = "000";
    public static final String NARROW_SPACE = "0";

    public static final String METADATA_KEY_SYMBOLOGY = "symbology";
    public static final String METADATA_VALUE_SYMBOLOGY = "itf";
    public static final String METADATA_KEY_PAIR_COUNT = "pairCount";

    public static final String DEFAULT_SCENE_LABEL = "ITF barcode";
    public static final String DEFAULT_FOREGROUND = "#000000";
    public static final String DEFAULT_BACKGROUND = "#ffffff";
    public static final int DEFAULT_MODULE_WIDTH = 4;
    public static final int DEFAULT_BAR_HEIGHT = 120;
}
