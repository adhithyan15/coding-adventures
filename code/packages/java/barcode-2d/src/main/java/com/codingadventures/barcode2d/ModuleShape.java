package com.codingadventures.barcode2d;

/**
 * The shape of each module in a {@link ModuleGrid}.
 *
 * <p>Two barcode module geometries are supported:
 *
 * <ul>
 *   <li>{@link #SQUARE} — used by QR Code, Data Matrix, Aztec Code, PDF417.
 *       The overwhelmingly common shape.  Each module renders as a filled
 *       square ({@link com.codingadventures.paintinstructions.PaintInstruction.PaintRect}).</li>
 *   <li>{@link #HEX} — used by MaxiCode (ISO/IEC 16023).  MaxiCode uses
 *       flat-top hexagons arranged in an offset-row grid.  Each module
 *       renders as a filled hexagon ({@link com.codingadventures.paintinstructions.PaintInstruction.PaintPath}).</li>
 * </ul>
 *
 * <p>The shape is stored on {@link ModuleGrid} so that {@link Barcode2D#layout}
 * can pick the right rendering path without the caller having to specify it again.
 *
 * <p>Spec: DT2D01 barcode-2d.
 */
public enum ModuleShape {

    /**
     * Square modules — QR Code, Data Matrix, Aztec, PDF417.
     *
     * <p>Each dark module at {@code (row, col)} becomes one
     * {@link com.codingadventures.paintinstructions.PaintInstruction.PaintRect}.
     *
     * <p>The most common shape — if you are unsure which to use, use this.
     */
    SQUARE,

    /**
     * Flat-top hexagonal modules — MaxiCode (ISO/IEC 16023).
     *
     * <p>Each dark module at {@code (row, col)} becomes one
     * {@link com.codingadventures.paintinstructions.PaintInstruction.PaintPath}
     * with six vertices.
     *
     * <p>MaxiCode grids are always 33 rows × 30 columns.  Odd-numbered rows are
     * shifted right by half a hexagon width to produce the standard hexagonal
     * tiling pattern:
     *
     * <pre>
     * Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no horizontal offset)
     * Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
     * Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡     (no horizontal offset)
     * </pre>
     */
    HEX,
}
