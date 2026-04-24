package com.codingadventures.barcode2d;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * The universal intermediate representation produced by every 2D barcode encoder.
 *
 * <p>A {@code ModuleGrid} is a 2D boolean grid:
 *
 * <pre>
 * modules.get(row).get(col) == true   →  dark module (ink / filled)
 * modules.get(row).get(col) == false  →  light module (background / empty)
 * </pre>
 *
 * <p>Row 0 is the top row.  Column 0 is the leftmost column.  This matches the
 * natural reading order used in every 2D barcode standard.
 *
 * <h2>MaxiCode fixed size</h2>
 *
 * <p>MaxiCode grids are always 33 rows × 30 columns with {@link ModuleShape#HEX}.
 * Physical MaxiCode symbols are always approximately 1 inch × 1 inch.
 *
 * <h2>Immutability</h2>
 *
 * <p>{@code ModuleGrid} is intentionally immutable.  Use
 * {@link Barcode2D#setModule(ModuleGrid, int, int, boolean)} to produce a new
 * grid with one module changed, rather than mutating in place.  This makes
 * encoders easy to test and compose without undo stacks or defensive copies.
 *
 * <p>Immutability also makes backtracking trivial: an encoder trying multiple
 * QR mask patterns can save the pre-mask grid reference, try each mask, score
 * the result, and simply discard the worse grids — no undo stack needed.
 *
 * <p>Spec: DT2D01 barcode-2d.
 */
public final class ModuleGrid {

    /** Number of rows (height of the grid). */
    public final int rows;

    /** Number of columns (width of the grid). */
    public final int cols;

    /**
     * Two-dimensional boolean grid.  Access with {@code modules.get(row).get(col)}.
     * {@code true} = dark module (ink), {@code false} = light module (background).
     *
     * <p>Each inner list is a complete row; the outer list contains all rows in
     * top-to-bottom order.  Both the outer and inner lists are unmodifiable.
     */
    public final List<List<Boolean>> modules;

    /** Shape of each module in the grid. */
    public final ModuleShape moduleShape;

    /**
     * Construct a ModuleGrid.
     *
     * <p>The provided modules list is defensively copied and made unmodifiable.
     * Each inner row list is also wrapped to be unmodifiable.
     *
     * @param rows        Number of rows.
     * @param cols        Number of columns.
     * @param modules     2D boolean grid (rows × cols). Will be wrapped as immutable.
     * @param moduleShape Shape of each module.
     */
    public ModuleGrid(int rows, int cols, List<List<Boolean>> modules, ModuleShape moduleShape) {
        this.rows = rows;
        this.cols = cols;
        // Wrap each inner row as an unmodifiable view, then wrap the outer list.
        List<List<Boolean>> defensive = new ArrayList<>(rows);
        for (List<Boolean> row : modules) {
            defensive.add(Collections.unmodifiableList(new ArrayList<>(row)));
        }
        this.modules = Collections.unmodifiableList(defensive);
        this.moduleShape = Objects.requireNonNull(moduleShape, "moduleShape must not be null");
    }

    /**
     * Convenience constructor with default {@link ModuleShape#SQUARE}.
     *
     * @param rows    Number of rows.
     * @param cols    Number of columns.
     * @param modules 2D boolean grid.
     */
    public ModuleGrid(int rows, int cols, List<List<Boolean>> modules) {
        this(rows, cols, modules, ModuleShape.SQUARE);
    }

    @Override
    public String toString() {
        return "ModuleGrid{rows=" + rows + ", cols=" + cols +
                ", moduleShape=" + moduleShape + "}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof ModuleGrid other)) return false;
        return rows == other.rows &&
                cols == other.cols &&
                modules.equals(other.modules) &&
                moduleShape == other.moduleShape;
    }

    @Override
    public int hashCode() {
        return Objects.hash(rows, cols, modules, moduleShape);
    }
}
