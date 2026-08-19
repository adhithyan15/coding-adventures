package com.codingadventures.paintvmascii;

/**
 * Errors this backend can report without throwing or returning a partial
 * rendering.
 *
 * <p>Every error carries enough of the offending instruction's fields for a
 * caller to diagnose the problem without re-inspecting the scene.
 */
public sealed interface PaintVmAsciiError {
    /** {@link AsciiOptions#scaleX} was not a positive integer. */
    record InvalidScaleX(int scaleX) implements PaintVmAsciiError {}

    /** {@link AsciiOptions#scaleY} was not a positive integer. */
    record InvalidScaleY(int scaleY) implements PaintVmAsciiError {}

    /** The scene's width or height was negative. */
    record InvalidSceneDimensions(int width, int height) implements PaintVmAsciiError {}

    /**
     * The scene's cell-grid size (width/scaleX by height/scaleY) exceeds the
     * bound this backend is willing to materialize. Checked both per-axis
     * and by total cell count — a product-only check can be bypassed by a
     * zero-width, huge-height (or vice versa) scene.
     */
    record SceneTooLarge(int width, int height) implements PaintVmAsciiError {}

    /** A {@code PaintRect}'s width or height was negative. */
    record InvalidRectangleGeometry(int x, int y, int width, int height) implements PaintVmAsciiError {}

    /** A {@code PaintLine}'s coordinates included a NaN or infinite value. */
    record InvalidLineGeometry(double x1, double y1, double x2, double y2) implements PaintVmAsciiError {}

    /**
     * A {@code PaintClip}'s coordinates were non-finite, either directly or
     * via the {@code x+width}/{@code y+height} extent (two individually
     * finite values can sum to infinity).
     */
    record InvalidClipGeometry(double x, double y, double width, double height) implements PaintVmAsciiError {}

    /**
     * A {@code PaintGroup} or {@code PaintLayer} used a feature this
     * text-mode backend cannot represent (non-identity transform,
     * non-default opacity, filters, non-normal blend mode), or the
     * instruction is a {@code PaintPath} (this backend renders no vector
     * geometry).
     */
    record UnsupportedInstruction(String reason) implements PaintVmAsciiError {}
}
