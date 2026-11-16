package com.codingadventures.itf

/**
 * Representation of an interleaved digit pair in an ITF barcode in Kotlin.
 *
 * @property pair the two-character digit slice
 * @property barPattern the 5-element wide/narrow pattern for the bars
 * @property spacePattern the 5-element wide/narrow pattern for the spaces
 * @property binaryPattern the 18-module interleaved binary pattern ('1' for bar, '0' for space)
 * @property sourceIndex 0-based pair index in the input
 */
data class EncodedPair(
    val pair: String,
    val barPattern: String,
    val spacePattern: String,
    val binaryPattern: String,
    val sourceIndex: Int
)
