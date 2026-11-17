package com.codingadventures.itf;

import java.util.Objects;

/**
 * Representation of an interleaved digit pair in an ITF barcode.
 *
 * @param pair the two-character digit slice
 * @param barPattern the 5-element wide/narrow pattern for the bars
 * @param spacePattern the 5-element wide/narrow pattern for the spaces
 * @param binaryPattern the 18-module interleaved binary pattern ('1' for bar, '0' for space)
 * @param sourceIndex 0-based pair index in the input
 */
public record EncodedPair(
        String pair,
        String barPattern,
        String spacePattern,
        String binaryPattern,
        int sourceIndex
) {
    public EncodedPair {
        Objects.requireNonNull(pair);
        Objects.requireNonNull(barPattern);
        Objects.requireNonNull(spacePattern);
        Objects.requireNonNull(binaryPattern);
    }
}
