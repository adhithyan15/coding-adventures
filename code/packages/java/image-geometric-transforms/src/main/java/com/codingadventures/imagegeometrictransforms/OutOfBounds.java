package com.codingadventures.imagegeometrictransforms;

/**
 * Policy for samples that fall outside the source image.
 *
 * <ul>
 *   <li>{@link #ZERO} — return transparent black.</li>
 *   <li>{@link #REPLICATE} — clamp to the nearest edge pixel.</li>
 *   <li>{@link #REFLECT} — mirror as if the edges were hinges.</li>
 *   <li>{@link #WRAP} — tile the image; wraps around at each boundary.</li>
 * </ul>
 */
public enum OutOfBounds {
    ZERO, REPLICATE, REFLECT, WRAP
}
