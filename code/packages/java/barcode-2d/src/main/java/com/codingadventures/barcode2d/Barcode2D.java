package com.codingadventures.barcode2d;

import com.codingadventures.paintinstructions.PaintInstruction;
import com.codingadventures.paintinstructions.PaintInstructions;
import com.codingadventures.paintinstructions.PaintScene;
import com.codingadventures.paintinstructions.PathCommand;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * barcode-2d — shared 2D barcode abstraction layer.
 *
 * <p>This class provides the two building blocks every 2D barcode format needs:
 *
 * <ol>
 *   <li>{@link ModuleGrid} — the universal intermediate representation produced by
 *       every 2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode).
 *       It is just a 2D boolean grid: {@code true} = dark module, {@code false} = light.</li>
 *   <li>{@link #layout} — the single function that converts abstract module
 *       coordinates into pixel-level {@link PaintScene} instructions ready for the
 *       PaintVM (P2D01) to render.</li>
 * </ol>
 *
 * <h2>Where this fits in the pipeline</h2>
 *
 * <pre>
 * Input data
 *   → format encoder (qr-code, data-matrix, aztec…)
 *   → ModuleGrid          ← produced by the encoder
 *   → layout()            ← THIS CLASS converts to pixels
 *   → PaintScene          ← consumed by paint-vm (P2D01)
 *   → backend (SVG, Metal, Canvas, terminal…)
 * </pre>
 *
 * <p>All coordinates before {@link #layout} are measured in "module units" — abstract
 * grid steps. Only {@link #layout} multiplies by {@link Barcode2DLayoutConfig#moduleSizePx}
 * to produce real pixel coordinates. This means encoders never need to know anything
 * about screen resolution or output format.
 *
 * <h2>Supported module shapes</h2>
 *
 * <ul>
 *   <li><b>Square</b> (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
 *       Each module becomes a {@link PaintInstruction.PaintRect}.</li>
 *   <li><b>Hex</b> (flat-top hexagons): used by MaxiCode. Each module becomes a
 *       {@link PaintInstruction.PaintPath} tracing six vertices.</li>
 * </ul>
 *
 * <p>Spec: DT2D01 barcode-2d.
 */
public final class Barcode2D {

    /** Package version following Semantic Versioning 2.0. */
    public static final String VERSION = "0.1.0";

    // Private constructor — this is a static-only utility class.
    private Barcode2D() {}

    // =========================================================================
    // makeModuleGrid — create an all-light grid
    // =========================================================================

    /**
     * Create a new {@link ModuleGrid} of the given dimensions, with every module
     * set to {@code false} (light).
     *
     * <p>This is the starting point for every 2D barcode encoder. The encoder calls
     * {@code makeModuleGrid(rows, cols)} and then uses {@link #setModule} to paint
     * dark modules one by one as it places finder patterns, timing strips, data bits,
     * and error correction bits.
     *
     * <h3>Example — start a 21×21 QR Code v1 grid</h3>
     *
     * <pre>
     *   ModuleGrid grid = Barcode2D.makeModuleGrid(21, 21);
     *   // grid.modules.get(0).get(0) == false  (all light)
     *   // grid.rows == 21
     *   // grid.cols == 21
     * </pre>
     *
     * @param rows        Number of rows (height of the grid).
     * @param cols        Number of columns (width of the grid).
     * @param moduleShape Shape of each module.
     * @return A new all-light {@link ModuleGrid}.
     */
    public static ModuleGrid makeModuleGrid(int rows, int cols, ModuleShape moduleShape) {
        // Build a 2D array of false values. Each row is an independent list so
        // that setModule() can replace individual rows without copying the entire grid.
        List<List<Boolean>> modules = new ArrayList<>(rows);
        for (int r = 0; r < rows; r++) {
            Boolean[] row = new Boolean[cols];
            Arrays.fill(row, Boolean.FALSE);
            modules.add(new ArrayList<>(Arrays.asList(row)));
        }
        return new ModuleGrid(rows, cols, modules, moduleShape);
    }

    /**
     * Create a new all-light {@link ModuleGrid} with default {@link ModuleShape#SQUARE}.
     *
     * @param rows Number of rows.
     * @param cols Number of columns.
     * @return A new all-light {@link ModuleGrid} with square modules.
     */
    public static ModuleGrid makeModuleGrid(int rows, int cols) {
        return makeModuleGrid(rows, cols, ModuleShape.SQUARE);
    }

    // =========================================================================
    // setModule — immutable single-module update
    // =========================================================================

    /**
     * Return a new {@link ModuleGrid} identical to {@code grid} except that the
     * module at {@code (row, col)} is set to {@code dark}.
     *
     * <p>This method is <b>pure and immutable</b> — it never modifies the input grid.
     * The original grid remains valid and unchanged. Only the affected row is
     * re-allocated; all other rows are shared between old and new grids.
     *
     * <h3>Why immutability matters</h3>
     *
     * <p>Barcode encoders often need to backtrack (e.g. trying different QR mask
     * patterns). Immutable grids make this trivial — save the grid before trying a
     * mask, evaluate it, discard if the score is worse, keep the old one if better.
     * No undo stack needed.
     *
     * <h3>Out-of-bounds</h3>
     *
     * <p>Throws {@link IndexOutOfBoundsException} if {@code row} or {@code col} is
     * outside the grid dimensions. This is a programming error in the encoder.
     *
     * <h3>Example</h3>
     *
     * <pre>
     *   ModuleGrid g  = Barcode2D.makeModuleGrid(3, 3);
     *   ModuleGrid g2 = Barcode2D.setModule(g, 1, 1, true);
     *   // g.modules.get(1).get(1)  == false  (original unchanged)
     *   // g2.modules.get(1).get(1) == true
     *   // g != g2                            (new object)
     * </pre>
     *
     * @param grid The original grid (not modified).
     * @param row  Row index (0-based, top = 0).
     * @param col  Column index (0-based, left = 0).
     * @param dark {@code true} for dark module, {@code false} for light module.
     * @return A new {@link ModuleGrid} with the specified module updated.
     * @throws IndexOutOfBoundsException if row or col is out of range.
     */
    public static ModuleGrid setModule(ModuleGrid grid, int row, int col, boolean dark) {
        Objects.requireNonNull(grid, "grid must not be null");
        if (row < 0 || row >= grid.rows) {
            throw new IndexOutOfBoundsException(
                    "setModule: row " + row + " out of range [0, " + (grid.rows - 1) + "]");
        }
        if (col < 0 || col >= grid.cols) {
            throw new IndexOutOfBoundsException(
                    "setModule: col " + col + " out of range [0, " + (grid.cols - 1) + "]");
        }

        // Copy only the affected row; all other rows are shared (shallow copy of outer list).
        List<List<Boolean>> newModules = new ArrayList<>(grid.modules);
        List<Boolean> newRow = new ArrayList<>(grid.modules.get(row));
        newRow.set(col, dark);
        newModules.set(row, newRow);

        return new ModuleGrid(grid.rows, grid.cols, newModules, grid.moduleShape);
    }

    // =========================================================================
    // layout — ModuleGrid → PaintScene
    // =========================================================================

    /**
     * Convert a {@link ModuleGrid} into a {@link PaintScene} ready for the PaintVM.
     *
     * <p>This is the <b>only</b> function in the entire 2D barcode stack that knows
     * about pixels. Everything above this step works in abstract module units.
     * Everything below this step is handled by the paint backend.
     *
     * <h3>Square modules (the common case)</h3>
     *
     * <p>Each dark module at {@code (row, col)} becomes one
     * {@link PaintInstruction.PaintRect}:
     *
     * <pre>
     * quietZonePx = quietZoneModules * moduleSizePx
     * x = quietZonePx + col * moduleSizePx
     * y = quietZonePx + row * moduleSizePx
     * </pre>
     *
     * <p>Total symbol size (including quiet zone on all four sides):
     *
     * <pre>
     * totalWidth  = (cols + 2 * quietZoneModules) * moduleSizePx
     * totalHeight = (rows + 2 * quietZoneModules) * moduleSizePx
     * </pre>
     *
     * <p>The scene always starts with one background {@link PaintInstruction.PaintRect}
     * covering the full symbol. This ensures the quiet zone and light modules are
     * filled with the background colour even if the backend has a transparent default.
     *
     * <h3>Hex modules (MaxiCode)</h3>
     *
     * <p>Each dark module at {@code (row, col)} becomes one
     * {@link PaintInstruction.PaintPath} tracing a flat-top regular hexagon. Odd-numbered
     * rows are offset by half a hexagon width to produce the standard hexagonal tiling:
     *
     * <pre>
     * Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡
     * Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡
     * Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
     * </pre>
     *
     * <h3>Validation</h3>
     *
     * <p>Throws {@link InvalidBarcode2DConfigException} if:
     * <ul>
     *   <li>{@code moduleSizePx} ≤ 0</li>
     *   <li>{@code quietZoneModules} &lt; 0</li>
     *   <li>{@code config.moduleShape} ≠ {@code grid.moduleShape}</li>
     * </ul>
     *
     * @param grid   The module grid to render.
     * @param config Layout configuration.  Use {@link Barcode2DLayoutConfig#defaults()}
     *               for a fully-default config.
     * @return A {@link PaintScene} ready for the PaintVM.
     * @throws InvalidBarcode2DConfigException on invalid configuration.
     */
    public static PaintScene layout(ModuleGrid grid, Barcode2DLayoutConfig config) {
        Objects.requireNonNull(grid, "grid must not be null");
        Objects.requireNonNull(config, "config must not be null");

        // ── Validation ────────────────────────────────────────────────────────
        if (config.moduleSizePx <= 0) {
            throw new InvalidBarcode2DConfigException(
                    "moduleSizePx must be > 0, got " + config.moduleSizePx);
        }
        if (config.quietZoneModules < 0) {
            throw new InvalidBarcode2DConfigException(
                    "quietZoneModules must be >= 0, got " + config.quietZoneModules);
        }
        if (config.moduleShape != grid.moduleShape) {
            throw new InvalidBarcode2DConfigException(
                    "config.moduleShape " + config.moduleShape +
                    " does not match grid.moduleShape " + grid.moduleShape);
        }

        // ── Dispatch to the correct rendering path ─────────────────────────────
        return switch (config.moduleShape) {
            case SQUARE -> layoutSquare(grid, config);
            case HEX -> layoutHex(grid, config);
        };
    }

    /**
     * Convert a {@link ModuleGrid} into a {@link PaintScene} using default config.
     *
     * <p>Equivalent to {@code layout(grid, Barcode2DLayoutConfig.defaults())}.
     *
     * @param grid The module grid to render.
     * @return A {@link PaintScene} with default layout config.
     */
    public static PaintScene layout(ModuleGrid grid) {
        return layout(grid, Barcode2DLayoutConfig.defaults());
    }

    // =========================================================================
    // layoutSquare — internal helper for square-module grids
    // =========================================================================

    /**
     * Render a square-module {@link ModuleGrid} into a {@link PaintScene}.
     *
     * <p>Called only by {@link #layout} after validation. Package-private so
     * tests can exercise it directly.
     *
     * <p>The algorithm:
     * <ol>
     *   <li>Compute total pixel dimensions including quiet zone.</li>
     *   <li>Emit one background {@link PaintInstruction.PaintRect} covering the entire symbol.</li>
     *   <li>For each dark module, emit one filled {@link PaintInstruction.PaintRect}.</li>
     * </ol>
     *
     * <p>Light modules are implicitly covered by the background rect — no explicit
     * light rects are emitted. This keeps the instruction count proportional to
     * the number of dark modules rather than the total grid size.
     */
    static PaintScene layoutSquare(ModuleGrid grid, Barcode2DLayoutConfig config) {
        int moduleSizePx = config.moduleSizePx;
        int quietZoneModules = config.quietZoneModules;

        // Quiet zone in pixels on each side.
        int quietZonePx = quietZoneModules * moduleSizePx;

        // Total canvas dimensions including quiet zone on all four sides.
        int totalWidth = (grid.cols + 2 * quietZoneModules) * moduleSizePx;
        int totalHeight = (grid.rows + 2 * quietZoneModules) * moduleSizePx;

        List<PaintInstruction> instructions = new ArrayList<>();

        // 1. Background: a single rect covering the entire symbol including quiet zone.
        //    This ensures light modules and the quiet zone are always filled, even when
        //    the backend default is transparent.
        instructions.add(PaintInstructions.paintRect(0, 0, totalWidth, totalHeight, config.background));

        // 2. One PaintRect per dark module.
        for (int row = 0; row < grid.rows; row++) {
            for (int col = 0; col < grid.cols; col++) {
                if (grid.modules.get(row).get(col)) {
                    // Pixel origin of this module (top-left corner of its square).
                    int x = quietZonePx + col * moduleSizePx;
                    int y = quietZonePx + row * moduleSizePx;
                    instructions.add(
                            PaintInstructions.paintRect(x, y, moduleSizePx, moduleSizePx, config.foreground));
                }
            }
        }

        return PaintInstructions.createScene(totalWidth, totalHeight, config.background, instructions);
    }

    // =========================================================================
    // layoutHex — internal helper for hex-module grids (MaxiCode)
    // =========================================================================

    /**
     * Render a hex-module {@link ModuleGrid} into a {@link PaintScene}.
     *
     * <p>Used for MaxiCode (ISO/IEC 16023), which uses flat-top hexagons in an
     * offset-row grid. Odd rows are shifted right by half a hexagon width.
     *
     * <h3>Flat-top hexagon geometry reminder</h3>
     *
     * <p>A "flat-top" hexagon has two flat edges at the top and bottom:
     *
     * <pre>
     *    ___
     *   /   \      ← two vertices at top
     *  |     |
     *   \___/      ← two vertices at bottom
     * </pre>
     *
     * <p>Contrast with "pointy-top" which has a vertex at the top. MaxiCode and
     * most industrial standards use flat-top.
     *
     * <p>For a flat-top hexagon centered at {@code (cx, cy)} with circumradius {@code R}:
     *
     * <pre>
     * Vertices at angles 0°, 60°, 120°, 180°, 240°, 300°:
     *
     *   angle  cos    sin    role
     *     0°    1      0     right midpoint
     *    60°   0.5   √3/2   bottom-right
     *   120°  -0.5   √3/2   bottom-left
     *   180°  -1      0     left midpoint
     *   240°  -0.5  -√3/2   top-left
     *   300°   0.5  -√3/2   top-right
     * </pre>
     *
     * <h3>Tiling</h3>
     *
     * <pre>
     * hexWidth  = moduleSizePx
     * hexHeight = moduleSizePx * (√3 / 2)   ← vertical distance between row centres
     * circumR   = moduleSizePx / √3         ← centre to vertex
     *
     * cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2.0)
     * cy = quietZonePx + row * hexHeight
     * </pre>
     */
    static PaintScene layoutHex(ModuleGrid grid, Barcode2DLayoutConfig config) {
        double moduleSizePx = (double) config.moduleSizePx;
        int quietZoneModules = config.quietZoneModules;

        // Hex geometry:
        //   hexWidth  = one module width (flat-to-flat = side length for regular hex)
        //   hexHeight = vertical distance between row centres
        //   circumR   = circumscribed circle radius (centre to vertex)
        //
        // For a regular hexagon where side length = s:
        //   flat-to-flat distance = s  →  hexWidth = moduleSizePx
        //   row step = s * (√3 / 2) = hexWidth * (√3 / 2)
        //   circumR = s / √3 = hexWidth / √3
        double hexWidth = moduleSizePx;
        double hexHeight = moduleSizePx * (Math.sqrt(3.0) / 2.0);
        double circumR = moduleSizePx / Math.sqrt(3.0);

        double quietZonePx = quietZoneModules * moduleSizePx;

        // Total canvas size.  The +hexWidth/2 accounts for the odd-row offset so
        // the rightmost modules on odd rows don't clip outside the canvas.
        int totalWidth = (int) ((grid.cols + 2 * quietZoneModules) * hexWidth + hexWidth / 2.0);
        int totalHeight = (int) ((grid.rows + 2 * quietZoneModules) * hexHeight);

        List<PaintInstruction> instructions = new ArrayList<>();

        // Background rect.
        instructions.add(PaintInstructions.paintRect(0, 0, totalWidth, totalHeight, config.background));

        // One PaintPath per dark module.
        for (int row = 0; row < grid.rows; row++) {
            for (int col = 0; col < grid.cols; col++) {
                if (grid.modules.get(row).get(col)) {
                    // Centre of this hexagon in pixel space.
                    // Odd rows shift right by hexWidth/2.
                    double cx = quietZonePx + col * hexWidth + (row % 2) * (hexWidth / 2.0);
                    double cy = quietZonePx + row * hexHeight;

                    instructions.add(
                            PaintInstructions.paintPath(buildFlatTopHexPath(cx, cy, circumR), config.foreground));
                }
            }
        }

        return PaintInstructions.createScene(totalWidth, totalHeight, config.background, instructions);
    }

    // =========================================================================
    // buildFlatTopHexPath — geometry helper
    // =========================================================================

    /**
     * Build the seven {@link PathCommand}s for a flat-top regular hexagon.
     *
     * <p>The six vertices are placed at angles 0°, 60°, 120°, 180°, 240°, 300°
     * from the centre {@code (cx, cy)} at circumradius {@code circumR}:
     *
     * <pre>
     * vertex_i = ( cx + circumR * cos(i * 60°),
     *              cy + circumR * sin(i * 60°) )
     * </pre>
     *
     * <p>The path starts with a {@link PathCommand.MoveTo} to vertex 0, then five
     * {@link PathCommand.LineTo} commands to vertices 1–5, then
     * {@link PathCommand.ClosePath#INSTANCE} to close the hexagon.
     *
     * @param cx       Centre x in pixels.
     * @param cy       Centre y in pixels.
     * @param circumR  Circumscribed circle radius (centre to vertex) in pixels.
     * @return Unmodifiable list of 7 path commands describing the flat-top hexagon.
     */
    static List<PathCommand> buildFlatTopHexPath(double cx, double cy, double circumR) {
        List<PathCommand> commands = new ArrayList<>(7);
        double degToRad = Math.PI / 180.0;

        // Vertex 0 → MoveTo; vertices 1..5 → LineTo.
        for (int i = 0; i <= 5; i++) {
            double angle = i * 60.0 * degToRad;
            double vx = cx + circumR * Math.cos(angle);
            double vy = cy + circumR * Math.sin(angle);
            if (i == 0) {
                commands.add(new PathCommand.MoveTo(vx, vy));
            } else {
                commands.add(new PathCommand.LineTo(vx, vy));
            }
        }

        // Close back to vertex 0.
        commands.add(PathCommand.ClosePath.INSTANCE);

        return Collections.unmodifiableList(commands);
    }
}
