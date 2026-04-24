package com.codingadventures.paintinstructions;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;

/**
 * The top-level container passed to a paint backend for rendering.
 *
 * <p>A {@code PaintScene} defines the viewport dimensions, the background fill,
 * and the ordered list of {@link PaintInstruction} objects that describe what to draw.
 *
 * <p>Instructions are rendered back-to-front (painter's algorithm): the first instruction
 * in the list is painted first (furthest back), the last instruction is painted on top.
 *
 * <p>The {@link #background} colour is always painted before all instructions, so the
 * quiet zone and any light modules are always correctly filled — even when the backend
 * has a transparent default surface.
 *
 * <h2>Architecture</h2>
 *
 * <pre>
 * Producer (QR encoder, Data Matrix encoder, MaxiCode encoder)
 *   → ModuleGrid                          ← produced by the encoder
 *   → barcode-2d layout()                 ← converts to pixel coordinates
 *   → PaintScene                          ← THIS CLASS
 *   → paint backend (SVG, Canvas, Metal)  ← renders to screen or file
 * </pre>
 *
 * <p>This class is immutable.  All fields are set in the constructor and cannot be changed.
 *
 * <p>Spec: P2D00 paint-instructions.
 */
public final class PaintScene {

    /** Viewport width in pixels. */
    public final int width;

    /** Viewport height in pixels. */
    public final int height;

    /**
     * Background colour, painted before all instructions.
     *
     * <p>Use CSS colour syntax: {@code "#ffffff"} (white), {@code "#000000"} (black),
     * {@code "transparent"} (no background fill).
     */
    public final String background;

    /**
     * Ordered list of paint instructions.
     *
     * <p>Rendered back-to-front: index 0 is painted first (furthest back),
     * the last element is painted on top.
     */
    public final List<PaintInstruction> instructions;

    /**
     * Optional scene-level key/value annotations.
     *
     * <p>Backends may expose these for dev-tools or accessibility annotations.
     * The paint VM ignores metadata — it is carried through unchanged.
     *
     * <p>Example: {@code {"qr:version": "3", "source": "qr-encoder"}}
     */
    public final Map<String, String> metadata;

    /**
     * Construct a PaintScene.
     *
     * @param width        Viewport width in pixels.
     * @param height       Viewport height in pixels.
     * @param background   CSS background colour string.
     * @param instructions Ordered list of paint instructions.
     * @param metadata     Optional scene-level annotations.
     */
    public PaintScene(int width, int height, String background,
                      List<PaintInstruction> instructions,
                      Map<String, String> metadata) {
        this.width = width;
        this.height = height;
        this.background = Objects.requireNonNull(background, "background must not be null");
        this.instructions = Collections.unmodifiableList(
                Objects.requireNonNull(instructions, "instructions must not be null"));
        this.metadata = Collections.unmodifiableMap(
                Objects.requireNonNull(metadata, "metadata must not be null"));
    }

    /**
     * Convenience constructor with empty metadata.
     *
     * @param width        Viewport width in pixels.
     * @param height       Viewport height in pixels.
     * @param background   CSS background colour string.
     * @param instructions Ordered list of paint instructions.
     */
    public PaintScene(int width, int height, String background,
                      List<PaintInstruction> instructions) {
        this(width, height, background, instructions, Map.of());
    }

    @Override
    public String toString() {
        return "PaintScene{width=" + width + ", height=" + height +
                ", background='" + background + "'" +
                ", instructions=" + instructions.size() + " items}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof PaintScene other)) return false;
        return width == other.width &&
                height == other.height &&
                background.equals(other.background) &&
                instructions.equals(other.instructions) &&
                metadata.equals(other.metadata);
    }

    @Override
    public int hashCode() {
        return Objects.hash(width, height, background, instructions, metadata);
    }
}
