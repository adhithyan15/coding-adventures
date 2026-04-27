package com.codingadventures.imagegeometrictransforms;

/**
 * How a free rotation sizes its output canvas.
 *
 * <ul>
 *   <li>{@link #FIT} — grow the canvas to enclose the whole rotated
 *       rectangle; nothing is clipped, but corners are transparent.</li>
 *   <li>{@link #CROP} — keep the original dimensions; corners of the
 *       rotated image that fall outside are clipped.</li>
 * </ul>
 */
public enum RotateBounds {
    FIT, CROP
}
