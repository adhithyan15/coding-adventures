package com.codingadventures.imagepointops;

/**
 * Strategy for collapsing RGB into a single luminance value.
 *
 * <p>All three methods compute a weighted average of the linear-light RGB
 * components. The weights differ because luminance perception is not
 * uniform across spectra — the human eye is ~twice as sensitive to green
 * as to red, and ~10× as sensitive to green as to blue.</p>
 *
 * <ul>
 *   <li>{@link #REC709} — HDTV / sRGB standard: 0.2126 R + 0.7152 G + 0.0722 B.
 *       Most accurate for modern displays.</li>
 *   <li>{@link #BT601}  — older NTSC/PAL standard: 0.299 R + 0.587 G + 0.114 B.
 *       Still common in JPEG and many image libraries.</li>
 *   <li>{@link #AVERAGE} — 1/3 R + 1/3 G + 1/3 B. Cheap and physically
 *       wrong; provided for parity with naive implementations and tests.</li>
 * </ul>
 */
public enum GreyscaleMethod {
    REC709,
    BT601,
    AVERAGE
}
