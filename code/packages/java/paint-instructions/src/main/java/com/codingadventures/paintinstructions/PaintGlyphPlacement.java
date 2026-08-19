package com.codingadventures.paintinstructions;

import java.util.Objects;

/**
 * One glyph's position within a {@link PaintInstruction.PaintGlyphRun}.
 *
 * <p>{@link #glyphId} is a font-internal glyph index in the general contract,
 * but text-mode ("ascii") backends relax this to a literal Unicode code point
 * — see {@code P2D02-paint-vm-ascii.md} &sect;"Glyph runs" for the rationale.
 */
public final class PaintGlyphPlacement {
    /** Font-internal glyph index in general, or a literal Unicode code point for ASCII backends. */
    public final int glyphId;
    /** Horizontal position in scene units. */
    public final double x;
    /** Vertical position in scene units. */
    public final double y;

    public PaintGlyphPlacement(int glyphId, double x, double y) {
        this.glyphId = glyphId;
        this.x = x;
        this.y = y;
    }

    @Override
    public String toString() {
        return "PaintGlyphPlacement{glyphId=" + glyphId + ", x=" + x + ", y=" + y + "}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof PaintGlyphPlacement other)) return false;
        return glyphId == other.glyphId &&
                Double.compare(x, other.x) == 0 &&
                Double.compare(y, other.y) == 0;
    }

    @Override
    public int hashCode() {
        return Objects.hash(glyphId, x, y);
    }
}
