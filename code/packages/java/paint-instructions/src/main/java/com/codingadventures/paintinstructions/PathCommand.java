package com.codingadventures.paintinstructions;

import java.util.Objects;

/**
 * A single drawing command inside a vector path.
 *
 * <p>Paths are sequences of commands that together describe a closed polygon.
 * The typical pattern for a hexagon is:
 *
 * <pre>
 *   MoveTo(x0, y0) → LineTo(x1, y1) → LineTo(x2, y2) → … → ClosePath
 * </pre>
 *
 * <p>Each concrete subclass carries the geometry needed for that command:
 *
 * <ul>
 *   <li>{@link MoveTo} — lift the pen and move to {@code (x, y)} without drawing.</li>
 *   <li>{@link LineTo} — draw a straight line from the current position to {@code (x, y)}.</li>
 *   <li>{@link ClosePath} — close the sub-path with a straight line back to the most
 *       recent {@link MoveTo} point.</li>
 * </ul>
 *
 * <p>Java 17+ sealed classes guarantee that the compiler knows every possible subtype.
 * Pattern matching with {@code instanceof} over a {@code PathCommand} is therefore
 * exhaustive — add a new subtype and the compiler flags every dispatch site that
 * needs updating.
 *
 * <p>Example — equilateral triangle:
 * <pre>
 *   List&lt;PathCommand&gt; triangle = List.of(
 *       new PathCommand.MoveTo(0.0, 0.0),
 *       new PathCommand.LineTo(10.0, 0.0),
 *       new PathCommand.LineTo(5.0, 8.66),
 *       PathCommand.ClosePath.INSTANCE
 *   );
 * </pre>
 *
 * <p>Example — flat-top hexagon (all six vertices):
 * <pre>
 *   List&lt;PathCommand&gt; hex = buildFlatTopHexPath(cx, cy, circumR);
 *   // Starts with MoveTo(vertex0), then 5 × LineTo, then ClosePath.
 * </pre>
 *
 * <p>Spec: P2D00 paint-instructions.
 */
public abstract sealed class PathCommand
        permits PathCommand.MoveTo, PathCommand.LineTo, PathCommand.ClosePath {

    // Private constructor — only the permitted subtypes can extend this class.
    private PathCommand() {}

    // =========================================================================
    // MoveTo — lift pen and move to (x, y)
    // =========================================================================

    /**
     * Lift the pen and move to {@code (x, y)} without drawing.
     *
     * <p>This is always the first command in a path (or the first command after
     * a {@link ClosePath} if a path has multiple sub-paths).
     *
     * @param x Horizontal position in pixels.
     * @param y Vertical position in pixels.
     */
    public static final class MoveTo extends PathCommand {
        /** Horizontal position in pixels. */
        public final double x;
        /** Vertical position in pixels. */
        public final double y;

        /**
         * Construct a MoveTo command.
         *
         * @param x Horizontal destination in pixels.
         * @param y Vertical destination in pixels.
         */
        public MoveTo(double x, double y) {
            this.x = x;
            this.y = y;
        }

        @Override
        public String toString() {
            return "MoveTo{x=" + x + ", y=" + y + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof MoveTo other)) return false;
            return Double.compare(x, other.x) == 0 && Double.compare(y, other.y) == 0;
        }

        @Override
        public int hashCode() {
            return Objects.hash(x, y);
        }
    }

    // =========================================================================
    // LineTo — draw a straight line to (x, y)
    // =========================================================================

    /**
     * Draw a straight line from the current pen position to {@code (x, y)}.
     *
     * <p>The pen position becomes {@code (x, y)} after this command, ready for
     * the next command in the sequence.
     *
     * @param x Horizontal destination in pixels.
     * @param y Vertical destination in pixels.
     */
    public static final class LineTo extends PathCommand {
        /** Horizontal destination in pixels. */
        public final double x;
        /** Vertical destination in pixels. */
        public final double y;

        /**
         * Construct a LineTo command.
         *
         * @param x Horizontal destination in pixels.
         * @param y Vertical destination in pixels.
         */
        public LineTo(double x, double y) {
            this.x = x;
            this.y = y;
        }

        @Override
        public String toString() {
            return "LineTo{x=" + x + ", y=" + y + "}";
        }

        @Override
        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (!(obj instanceof LineTo other)) return false;
            return Double.compare(x, other.x) == 0 && Double.compare(y, other.y) == 0;
        }

        @Override
        public int hashCode() {
            return Objects.hash(x, y);
        }
    }

    // =========================================================================
    // ClosePath — close the current subpath
    // =========================================================================

    /**
     * Close the current sub-path.
     *
     * <p>Draws a straight line from the current position back to the point
     * specified in the most recent {@link MoveTo}.  For a convex polygon the
     * result is a closed filled shape.
     *
     * <p>Because this command carries no data, it is modelled as a singleton.
     * Use {@link #INSTANCE} for convenience instead of {@code new ClosePath()}.
     *
     * <p>Why a private constructor?  A no-data command has no reason to have
     * multiple instances.  The singleton saves allocation in tight loops that
     * build many hex paths.
     */
    public static final class ClosePath extends PathCommand {

        /** Reusable singleton instance — avoids repeated {@code new ClosePath()} allocations. */
        public static final ClosePath INSTANCE = new ClosePath();

        /** Private — use {@link #INSTANCE}. */
        private ClosePath() {}

        @Override
        public String toString() {
            return "ClosePath";
        }

        @Override
        public boolean equals(Object obj) {
            return obj instanceof ClosePath;
        }

        @Override
        public int hashCode() {
            return ClosePath.class.hashCode();
        }
    }
}
