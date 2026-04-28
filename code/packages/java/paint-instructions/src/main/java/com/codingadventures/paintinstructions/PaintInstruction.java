package com.codingadventures.paintinstructions;

import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;


/**
 * A single drawing instruction inside a {@link PaintScene}.
 *
 * <p>Instructions are polymorphic via a sealed abstract class.  The two concrete
 * subtypes cover the two shapes needed by all current 2D barcode standards:
 *
 * <ul>
 *   <li>{@link PaintRect} — for square-module barcodes (QR Code, Data Matrix, Aztec, PDF417).</li>
 *   <li>{@link PaintPath} — for hex-module barcodes (MaxiCode).</li>
 * </ul>
 *
 * <p>Why a sealed class?  The sealed hierarchy guarantees exhaustive {@code instanceof}
 * coverage.  If a new instruction type is ever added (e.g., {@code PaintCircle} for a
 * bullseye ring), the compiler immediately flags every dispatch point that needs updating.
 *
 * <p>Spec: P2D00 paint-instructions.
 */
public abstract sealed class PaintInstruction
        permits PaintInstruction.PaintRect, PaintInstruction.PaintPath {

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
        /** CSS colour string for the fill, e.g. {@code "#000000"}. */
        public final String fill;
        /**
         * Optional key/value annotations.  Carried through the pipeline unchanged;
         * backends may expose these for dev-tools or accessibility.
         */
        public final Map<String, String> metadata;

        /**
         * Construct a PaintRect.
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
            this.x = x;
            this.y = y;
            this.width = width;
            this.height = height;
            this.fill = Objects.requireNonNull(fill, "fill must not be null");
            this.metadata = Collections.unmodifiableMap(
                    Objects.requireNonNull(metadata, "metadata must not be null"));
        }

        /**
         * Convenience constructor with empty metadata.
         *
         * @param x      Left edge in pixels.
         * @param y      Top edge in pixels.
         * @param width  Width in pixels.
         * @param height Height in pixels.
         * @param fill   CSS colour string.
         */
        public PaintRect(int x, int y, int width, int height, String fill) {
            this(x, y, width, height, fill, Map.of());
        }

        @Override
        public String toString() {
            return "PaintRect{x=" + x + ", y=" + y +
                    ", width=" + width + ", height=" + height +
                    ", fill='" + fill + "'}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof PaintRect other)) return false;
            return x == other.x && y == other.y &&
                    width == other.width && height == other.height &&
                    fill.equals(other.fill) &&
                    metadata.equals(other.metadata);
        }

        @Override
        public int hashCode() {
            return Objects.hash(x, y, width, height, fill, metadata);
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
}
