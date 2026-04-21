package com.codingadventures.imagegeometrictransforms;

/**
 * Interpolation strategy used when sampling at fractional coordinates.
 *
 * <ul>
 *   <li>{@link #NEAREST} — pick the closest pixel. Fast, blocky.</li>
 *   <li>{@link #BILINEAR} — linear blend of the four enclosing pixels.
 *       Smooth but slightly blurry.</li>
 *   <li>{@link #BICUBIC} — Catmull-Rom cubic over a 4×4 neighbourhood.
 *       Sharper than bilinear, can introduce ringing on hard edges.</li>
 * </ul>
 */
public enum Interpolation {
    NEAREST, BILINEAR, BICUBIC
}
