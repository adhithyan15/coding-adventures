package com.codingadventures.paintinstructions;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;


/**
 * A single drawing instruction inside a {@link PaintScene}.
 *
 * <p>Instructions are polymorphic via a sealed abstract class. The concrete
 * subtypes cover both the shapes needed by 2D barcode standards and the full
 * text-mode rendering contract ({@code P2D02-paint-vm-ascii.md}):
 *
 * <ul>
 *   <li>{@link PaintRect} — filled and/or stroked rectangle. For square-module
 *       barcodes (QR Code, Data Matrix, Aztec, PDF417).</li>
 *   <li>{@link PaintPath} — for hex-module barcodes (MaxiCode).</li>
 *   <li>{@link PaintLine} — a stroked line segment between two points.</li>
 *   <li>{@link PaintGlyphRun} — pre-positioned glyphs ({@link PaintGlyphPlacement}).</li>
 *   <li>{@link PaintGroup} — a child list with an optional {@link Transform2D} and opacity.</li>
 *   <li>{@link PaintClip} — a rectangular clip region wrapping a child list.</li>
 *   <li>{@link PaintLayer} — a child list with a filter flag, blend mode, opacity, and transform.</li>
 * </ul>
 *
 * <p>Why a sealed class?  The sealed hierarchy guarantees exhaustive {@code instanceof}
 * coverage.  If a new instruction type is ever added, the compiler immediately flags
 * every dispatch point that needs updating.
 *
 * <p>Spec: P2D00 paint-instructions, P2D02 paint-vm-ascii.
 */
public abstract sealed class PaintInstruction
        permits PaintInstruction.PaintRect, PaintInstruction.PaintPath,
                PaintInstruction.PaintLine, PaintInstruction.PaintGlyphRun,
                PaintInstruction.PaintGroup, PaintInstruction.PaintClip,
                PaintInstruction.PaintLayer {

    // Private constructor — only permitted subclasses can extend.
    private PaintInstruction() {}

    // =========================================================================
    // PaintRect
    // =========================================================================

    /**
     * A filled axis-aligned rectangle.
     *
     * <p>Coordinates use the top-left corner as origin, with x pointing right and y
     * pointing down — the standard 2D raster convention.
     *
     * <pre>
     * (x, y) ─────────────────────┐
     *   │                         │
     *   │   PaintRect             │  height
     *   │                         │
     *   └─────────────────────────┘
     *              width
     * </pre>
     *
     * <p>Used by QR Code, Data Matrix, Aztec Code, and PDF417 — all of which
     * use square modules on a rectangular grid.  Each dark module in the barcode
     * becomes one {@code PaintRect}.
     *
     * <p>Example — a 10×10 dark module at column 4, row 4:
     *
     * <pre>
     *   new PaintInstruction.PaintRect(40, 40, 10, 10, "#000000", Map.of())
     * </pre>
     */
    public static final class PaintRect extends PaintInstruction {
        /** Left edge of the rectangle in pixels. */
        public final int x;
        /** Top edge of the rectangle in pixels. */
        public final int y;
        /** Width in pixels. Must be ≥ 0. */
        public final int width;
        /** Height in pixels. Must be ≥ 0. */
        public final int height;
        /** CSS colour string for the fill, e.g. {@code "#000000"}. Empty means no fill. */
        public final String fill;
        /** CSS colour string for the stroke. Empty means no stroke. */
        public final String stroke;
        /** Stroke width in pixels. Ignored when {@link #stroke} is empty. */
        public final double strokeWidth;
        /**
         * Optional key/value annotations.  Carried through the pipeline unchanged;
         * backends may expose these for dev-tools or accessibility.
         */
        public final Map<String, String> metadata;

        /**
         * Construct a PaintRect with an explicit stroke.
         *
         * @param x           Left edge in pixels.
         * @param y           Top edge in pixels.
         * @param width       Width in pixels (≥ 0).
         * @param height      Height in pixels (≥ 0).
         * @param fill        CSS colour string. Empty means no fill.
         * @param stroke      CSS colour string. Empty means no stroke.
         * @param strokeWidth Stroke width in pixels.
         * @param metadata    Optional annotations.
         */
        public PaintRect(int x, int y, int width, int height,
                         String fill, String stroke, double strokeWidth,
                         Map<String, String> metadata) {
            this.x = x;
            this.y = y;
            this.width = width;
            this.height = height;
            this.fill = Objects.requireNonNull(fill, "fill must not be null");
            this.stroke = Objects.requireNonNull(stroke, "stroke must not be null");
            this.strokeWidth = strokeWidth;
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        /**
         * Construct a PaintRect with no stroke (fill-only), matching the original
         * pre-P2D02 contract. Kept for source compatibility with every existing
         * caller of this constructor.
         *
         * @param x        Left edge in pixels.
         * @param y        Top edge in pixels.
         * @param width    Width in pixels (≥ 0).
         * @param height   Height in pixels (≥ 0).
         * @param fill     CSS colour string.
         * @param metadata Optional annotations.
         */
        public PaintRect(int x, int y, int width, int height,
                         String fill, Map<String, String> metadata) {
            this(x, y, width, height, fill, "", 0.0, metadata);
        }

        /**
         * Convenience constructor with empty metadata and no stroke.
         *
         * @param x      Left edge in pixels.
         * @param y      Top edge in pixels.
         * @param width  Width in pixels.
         * @param height Height in pixels.
         * @param fill   CSS colour string.
         */
        public PaintRect(int x, int y, int width, int height, String fill) {
            this(x, y, width, height, fill, "", 0.0, Map.of());
        }

        @Override
        public String toString() {
            return "PaintRect{x=" + x + ", y=" + y +
                    ", width=" + width + ", height=" + height +
                    ", fill='" + fill + "', stroke='" + stroke +
                    "', strokeWidth=" + strokeWidth + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintRect other)) return false;
            return x == other.x && y == other.y &&
                    width == other.width && height == other.height &&
                    fill.equals(other.fill) &&
                    stroke.equals(other.stroke) &&
                    Double.compare(strokeWidth, other.strokeWidth) == 0 &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(x, y, width, height, fill, stroke, strokeWidth, metadata);
        }
    }

    // =========================================================================
    // PaintPath
    // =========================================================================

    /**
     * A filled closed polygon described by a list of {@link PathCommand}s.
     *
     * <p>The commands must form a closed shape: they should start with a
     * {@link PathCommand.MoveTo} and end with {@link PathCommand.ClosePath}.
     * The resulting polygon is filled with the {@link #fill} colour.
     *
     * <p>Used by MaxiCode (ISO/IEC 16023), which uses flat-top hexagons arranged
     * in an offset-row grid.  Each dark module in a MaxiCode grid becomes one
     * {@code PaintPath} whose six vertices are computed from the module's
     * {@code (row, col)} position.
     *
     * <p>Example — a triangle:
     *
     * <pre>
     *   List&lt;PathCommand&gt; cmds = List.of(
     *       new PathCommand.MoveTo(0, 0),
     *       new PathCommand.LineTo(10, 0),
     *       new PathCommand.LineTo(5, 8.66),
     *       PathCommand.ClosePath.INSTANCE
     *   );
     *   new PaintInstruction.PaintPath(cmds, "#1a1a1a", Map.of())
     * </pre>
     */
    public static final class PaintPath extends PaintInstruction {
        /**
         * Ordered path commands describing the polygon.
         *
         * <p>Must begin with a {@link PathCommand.MoveTo} and end with
         * {@link PathCommand.ClosePath}.
         */
        public final List<PathCommand> commands;
        /** CSS colour string for the fill, e.g. {@code "#000000"}. */
        public final String fill;
        /**
         * Optional key/value annotations.  Carried through the pipeline unchanged.
         */
        public final Map<String, String> metadata;

        /**
         * Construct a PaintPath.
         *
         * @param commands Ordered path commands (must start with MoveTo, end with ClosePath).
         * @param fill     CSS colour string.
         * @param metadata Optional annotations.
         */
        public PaintPath(List<PathCommand> commands, String fill, Map<String, String> metadata) {
            this.commands = Collections.unmodifiableList(
                    Objects.requireNonNull(commands, "commands must not be null"));
            this.fill = Objects.requireNonNull(fill, "fill must not be null");
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        /**
         * Convenience constructor with empty metadata.
         *
         * @param commands Ordered path commands.
         * @param fill     CSS colour string.
         */
        public PaintPath(List<PathCommand> commands, String fill) {
            this(commands, fill, Map.of());
        }

        @Override
        public String toString() {
            return "PaintPath{commands=" + commands.size() +
                    " cmds, fill='" + fill + "'}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintPath other)) return false;
            return commands.equals(other.commands) &&
                    fill.equals(other.fill) &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(commands, fill, metadata);
        }
    }

    // =========================================================================
    // PaintLine
    // =========================================================================

    /**
     * A stroked line segment between two points.
     *
     * <p>A line with no stroke is invisible, so unlike {@link PaintRect}
     * {@link #stroke} is not optional here — it is required to have some
     * value for the line to be drawn at all.
     */
    public static final class PaintLine extends PaintInstruction {
        /** Start point x. */
        public final double x1;
        /** Start point y. */
        public final double y1;
        /** End point x. */
        public final double x2;
        /** End point y. */
        public final double y2;
        /** CSS stroke colour. Required — a line with no stroke is invisible. */
        public final String stroke;
        /** Stroke width in pixels. */
        public final double strokeWidth;
        /** Optional key/value annotations; ignored by the renderer. */
        public final Map<String, String> metadata;

        public PaintLine(double x1, double y1, double x2, double y2,
                         String stroke, double strokeWidth, Map<String, String> metadata) {
            this.x1 = x1;
            this.y1 = y1;
            this.x2 = x2;
            this.y2 = y2;
            this.stroke = Objects.requireNonNull(stroke, "stroke must not be null");
            this.strokeWidth = strokeWidth;
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        public PaintLine(double x1, double y1, double x2, double y2, String stroke, double strokeWidth) {
            this(x1, y1, x2, y2, stroke, strokeWidth, Map.of());
        }

        @Override
        public String toString() {
            return "PaintLine{x1=" + x1 + ", y1=" + y1 + ", x2=" + x2 + ", y2=" + y2 +
                    ", stroke='" + stroke + "', strokeWidth=" + strokeWidth + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintLine other)) return false;
            return Double.compare(x1, other.x1) == 0 && Double.compare(y1, other.y1) == 0 &&
                    Double.compare(x2, other.x2) == 0 && Double.compare(y2, other.y2) == 0 &&
                    stroke.equals(other.stroke) &&
                    Double.compare(strokeWidth, other.strokeWidth) == 0 &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(x1, y1, x2, y2, stroke, strokeWidth, metadata);
        }
    }

    // =========================================================================
    // PaintGlyphRun
    // =========================================================================

    /**
     * Pre-positioned glyphs, each already placed in scene coordinates.
     *
     * <p>{@link #fontRef}, {@link #fontSize}, and {@link #fill} are required
     * fields but are ignored by text-mode (ASCII) backends.
     */
    public static final class PaintGlyphRun extends PaintInstruction {
        /** Pre-positioned glyphs. */
        public final List<PaintGlyphPlacement> glyphs;
        /** Opaque font identifier. Text-mode backends ignore this. */
        public final String fontRef;
        /** Font size in scene units. Text-mode backends ignore this. */
        public final double fontSize;
        /** CSS colour of the glyphs. Text-mode backends ignore this. */
        public final String fill;
        /** Optional key/value annotations; ignored by the renderer. */
        public final Map<String, String> metadata;

        public PaintGlyphRun(List<PaintGlyphPlacement> glyphs, String fontRef, double fontSize,
                             String fill, Map<String, String> metadata) {
            this.glyphs = Collections.unmodifiableList(
                    Objects.requireNonNull(glyphs, "glyphs must not be null"));
            this.fontRef = Objects.requireNonNull(fontRef, "fontRef must not be null");
            this.fontSize = fontSize;
            this.fill = Objects.requireNonNull(fill, "fill must not be null");
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        public PaintGlyphRun(List<PaintGlyphPlacement> glyphs, String fontRef, double fontSize, String fill) {
            this(glyphs, fontRef, fontSize, fill, Map.of());
        }

        @Override
        public String toString() {
            return "PaintGlyphRun{glyphs=" + glyphs.size() + ", fontRef='" + fontRef + "'}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintGlyphRun other)) return false;
            return glyphs.equals(other.glyphs) &&
                    fontRef.equals(other.fontRef) &&
                    Double.compare(fontSize, other.fontSize) == 0 &&
                    fill.equals(other.fill) &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(glyphs, fontRef, fontSize, fill, metadata);
        }
    }

    // =========================================================================
    // PaintGroup
    // =========================================================================

    /**
     * A child list with an optional {@link Transform2D} and opacity.
     */
    public static final class PaintGroup extends PaintInstruction {
        /** Instructions inside this group, rendered back-to-front. */
        public final List<PaintInstruction> children;
        /** Optional affine transform applied to all children. Empty means no transform. */
        public final Optional<Transform2D> transform;
        /** Optional group-level compositing opacity (0.0-1.0). Empty means fully opaque. */
        public final Optional<Double> opacity;
        /** Optional key/value annotations; ignored by the renderer. */
        public final Map<String, String> metadata;

        public PaintGroup(List<PaintInstruction> children, Optional<Transform2D> transform,
                          Optional<Double> opacity, Map<String, String> metadata) {
            this.children = Collections.unmodifiableList(
                    Objects.requireNonNull(children, "children must not be null"));
            this.transform = Objects.requireNonNull(transform, "transform must not be null");
            this.opacity = Objects.requireNonNull(opacity, "opacity must not be null");
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        /** Construct a plain (untransformed, fully opaque) group with no metadata. */
        public PaintGroup(List<PaintInstruction> children) {
            this(children, Optional.empty(), Optional.empty(), Map.of());
        }

        @Override
        public String toString() {
            return "PaintGroup{children=" + children.size() + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintGroup other)) return false;
            return children.equals(other.children) &&
                    transform.equals(other.transform) &&
                    opacity.equals(other.opacity) &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(children, transform, opacity, metadata);
        }
    }

    // =========================================================================
    // PaintClip
    // =========================================================================

    /**
     * A rectangular clip region wrapping a child list.
     */
    public static final class PaintClip extends PaintInstruction {
        /** Clip rectangle top-left x. */
        public final double x;
        /** Clip rectangle top-left y. */
        public final double y;
        /** Clip rectangle width. */
        public final double width;
        /** Clip rectangle height. */
        public final double height;
        /** Instructions rendered inside the clip region. */
        public final List<PaintInstruction> children;
        /** Optional key/value annotations; ignored by the renderer. */
        public final Map<String, String> metadata;

        public PaintClip(double x, double y, double width, double height,
                         List<PaintInstruction> children, Map<String, String> metadata) {
            this.x = x;
            this.y = y;
            this.width = width;
            this.height = height;
            this.children = Collections.unmodifiableList(
                    Objects.requireNonNull(children, "children must not be null"));
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        public PaintClip(double x, double y, double width, double height, List<PaintInstruction> children) {
            this(x, y, width, height, children, Map.of());
        }

        @Override
        public String toString() {
            return "PaintClip{x=" + x + ", y=" + y + ", width=" + width + ", height=" + height +
                    ", children=" + children.size() + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintClip other)) return false;
            return Double.compare(x, other.x) == 0 && Double.compare(y, other.y) == 0 &&
                    Double.compare(width, other.width) == 0 && Double.compare(height, other.height) == 0 &&
                    children.equals(other.children) &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(x, y, width, height, children, metadata);
        }
    }

    // =========================================================================
    // PaintLayer
    // =========================================================================

    /**
     * A child list with a filter flag, blend mode, opacity, and transform.
     *
     * <p>{@link #hasFilters} is a simplified stand-in for the full filter-effect
     * union — no backend in this repository's Java port implements pixel-level
     * filters, so all that matters for dispatch is whether to reject the layer.
     */
    public static final class PaintLayer extends PaintInstruction {
        /** Instructions rendered into the (conceptual) offscreen buffer. */
        public final List<PaintInstruction> children;
        /** Whether any pixel-level filter (blur, drop shadow, etc.) is attached. */
        public final boolean hasFilters;
        /** Optional blend mode name. Empty or "normal" means standard alpha compositing. */
        public final Optional<String> blendMode;
        /** Optional layer-level opacity (0.0-1.0). */
        public final Optional<Double> opacity;
        /** Optional affine transform applied to the layer as a whole. */
        public final Optional<Transform2D> transform;
        /** Optional key/value annotations; ignored by the renderer. */
        public final Map<String, String> metadata;

        public PaintLayer(List<PaintInstruction> children, boolean hasFilters,
                          Optional<String> blendMode, Optional<Double> opacity,
                          Optional<Transform2D> transform, Map<String, String> metadata) {
            this.children = Collections.unmodifiableList(
                    Objects.requireNonNull(children, "children must not be null"));
            this.hasFilters = hasFilters;
            this.blendMode = Objects.requireNonNull(blendMode, "blendMode must not be null");
            this.opacity = Objects.requireNonNull(opacity, "opacity must not be null");
            this.transform = Objects.requireNonNull(transform, "transform must not be null");
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        /** Construct a plain (untransformed, unfiltered, fully opaque, normal-blend) layer. */
        public PaintLayer(List<PaintInstruction> children) {
            this(children, false, Optional.empty(), Optional.empty(), Optional.empty(), Map.of());
        }

        @Override
        public String toString() {
            return "PaintLayer{children=" + children.size() + ", hasFilters=" + hasFilters + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintLayer other)) return false;
            return children.equals(other.children) &&
                    hasFilters == other.hasFilters &&
                    blendMode.equals(other.blendMode) &&
                    opacity.equals(other.opacity) &&
                    transform.equals(other.transform) &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(children, hasFilters, blendMode, opacity, transform, metadata);
        }
    }
}
