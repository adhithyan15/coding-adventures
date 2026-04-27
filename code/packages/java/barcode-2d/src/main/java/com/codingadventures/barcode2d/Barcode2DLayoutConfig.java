package com.codingadventures.barcode2d;

import java.util.Objects;

/**
 * Configuration for {@link Barcode2D#layout}.
 *
 * <p>All fields have sensible defaults.  Use the {@link Builder} for ergonomic construction:
 *
 * <pre>
 *   Barcode2DLayoutConfig config = new Barcode2DLayoutConfig.Builder()
 *       .moduleSizePx(10)
 *       .quietZoneModules(4)
 *       .foreground("#000000")
 *       .background("#ffffff")
 *       .moduleShape(ModuleShape.SQUARE)
 *       .build();
 * </pre>
 *
 * <h2>moduleSizePx</h2>
 *
 * <p>The size of one module in pixels. For square modules this is both width and
 * height. For hex modules it is the hexagon's width (flat-to-flat, which equals
 * the side length for a regular hexagon).
 *
 * <p>Must be &gt; 0.
 *
 * <h2>quietZoneModules</h2>
 *
 * <p>The number of module-width quiet-zone units added on each side of the grid.
 * QR Code requires a minimum of 4 modules per ISO/IEC 18004.  Data Matrix requires 1.
 * MaxiCode requires 1.  We default to 4 to be safe for QR Code.
 *
 * <p>Must be ≥ 0.
 *
 * <h2>moduleShape</h2>
 *
 * <p>Must match {@link ModuleGrid#moduleShape}. If they disagree,
 * {@link Barcode2D#layout} throws {@link InvalidBarcode2DConfigException}.
 * This double-check prevents accidentally rendering a MaxiCode hex grid with
 * square modules (which would produce a visually wrong but non-crashing output).
 *
 * <p>Spec: DT2D01 barcode-2d.
 */
public final class Barcode2DLayoutConfig {

    // =========================================================================
    // Defaults
    // =========================================================================

    /**
     * Default module size in pixels.
     *
     * <p>10 px produces a readable QR Code v1 at 290×290 pixels
     * (21 modules + 4-module quiet zone on each side = 29 modules × 10 px = 290 px).
     */
    public static final int DEFAULT_MODULE_SIZE_PX = 10;

    /**
     * Default quiet zone in modules.
     *
     * <p>QR Code requires a minimum of 4 module-widths of quiet zone per ISO/IEC 18004.
     * Data Matrix requires 1; MaxiCode requires 1.
     * Defaulting to 4 is safe for all formats.
     */
    public static final int DEFAULT_QUIET_ZONE_MODULES = 4;

    /** Default foreground (dark module) colour — black ink on white paper. */
    public static final String DEFAULT_FOREGROUND = "#000000";

    /** Default background (light module / quiet zone) colour — white paper. */
    public static final String DEFAULT_BACKGROUND = "#ffffff";

    /** Default module shape — the overwhelmingly common square. */
    public static final ModuleShape DEFAULT_MODULE_SHAPE = ModuleShape.SQUARE;

    // =========================================================================
    // Fields
    // =========================================================================

    /** Size of one module in pixels. Must be &gt; 0. */
    public final int moduleSizePx;

    /** Number of module-width quiet-zone units on each side. Must be ≥ 0. */
    public final int quietZoneModules;

    /** CSS colour for dark modules (ink). E.g. {@code "#000000"}. */
    public final String foreground;

    /** CSS colour for light modules and the quiet zone. E.g. {@code "#ffffff"}. */
    public final String background;

    /** Shape of each module — must match {@link ModuleGrid#moduleShape}. */
    public final ModuleShape moduleShape;

    // =========================================================================
    // Constructor (private — use Builder or defaults())
    // =========================================================================

    private Barcode2DLayoutConfig(int moduleSizePx, int quietZoneModules,
                                   String foreground, String background,
                                   ModuleShape moduleShape) {
        this.moduleSizePx = moduleSizePx;
        this.quietZoneModules = quietZoneModules;
        this.foreground = Objects.requireNonNull(foreground, "foreground must not be null");
        this.background = Objects.requireNonNull(background, "background must not be null");
        this.moduleShape = Objects.requireNonNull(moduleShape, "moduleShape must not be null");
    }

    /**
     * Create a config with all default values.
     *
     * <p>Equivalent to {@code new Barcode2DLayoutConfig.Builder().build()}.
     *
     * <p>Defaults:
     * <ul>
     *   <li>moduleSizePx = 10</li>
     *   <li>quietZoneModules = 4</li>
     *   <li>foreground = "#000000"</li>
     *   <li>background = "#ffffff"</li>
     *   <li>moduleShape = SQUARE</li>
     * </ul>
     *
     * @return A default-valued {@link Barcode2DLayoutConfig}.
     */
    public static Barcode2DLayoutConfig defaults() {
        return new Barcode2DLayoutConfig(
                DEFAULT_MODULE_SIZE_PX,
                DEFAULT_QUIET_ZONE_MODULES,
                DEFAULT_FOREGROUND,
                DEFAULT_BACKGROUND,
                DEFAULT_MODULE_SHAPE
        );
    }

    @Override
    public String toString() {
        return "Barcode2DLayoutConfig{moduleSizePx=" + moduleSizePx +
                ", quietZoneModules=" + quietZoneModules +
                ", foreground='" + foreground + "'" +
                ", background='" + background + "'" +
                ", moduleShape=" + moduleShape + "}";
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (!(obj instanceof Barcode2DLayoutConfig other)) return false;
        return moduleSizePx == other.moduleSizePx &&
                quietZoneModules == other.quietZoneModules &&
                foreground.equals(other.foreground) &&
                background.equals(other.background) &&
                moduleShape == other.moduleShape;
    }

    @Override
    public int hashCode() {
        return Objects.hash(moduleSizePx, quietZoneModules, foreground, background, moduleShape);
    }

    // =========================================================================
    // Builder
    // =========================================================================

    /**
     * Fluent builder for {@link Barcode2DLayoutConfig}.
     *
     * <p>All fields start at their defaults. Override only what you need:
     *
     * <pre>
     *   Barcode2DLayoutConfig config = new Barcode2DLayoutConfig.Builder()
     *       .moduleSizePx(5)
     *       .build();
     * </pre>
     */
    public static final class Builder {

        private int moduleSizePx = DEFAULT_MODULE_SIZE_PX;
        private int quietZoneModules = DEFAULT_QUIET_ZONE_MODULES;
        private String foreground = DEFAULT_FOREGROUND;
        private String background = DEFAULT_BACKGROUND;
        private ModuleShape moduleShape = DEFAULT_MODULE_SHAPE;

        /** Set the module size in pixels. Must be &gt; 0 (validated at {@link Barcode2D#layout} time). */
        public Builder moduleSizePx(int moduleSizePx) {
            this.moduleSizePx = moduleSizePx;
            return this;
        }

        /** Set the quiet zone width in modules. Must be ≥ 0 (validated at {@link Barcode2D#layout} time). */
        public Builder quietZoneModules(int quietZoneModules) {
            this.quietZoneModules = quietZoneModules;
            return this;
        }

        /** Set the foreground (dark module) CSS colour. */
        public Builder foreground(String foreground) {
            this.foreground = Objects.requireNonNull(foreground, "foreground must not be null");
            return this;
        }

        /** Set the background (light module / quiet zone) CSS colour. */
        public Builder background(String background) {
            this.background = Objects.requireNonNull(background, "background must not be null");
            return this;
        }

        /** Set the module shape. Must match the {@link ModuleGrid} being laid out. */
        public Builder moduleShape(ModuleShape moduleShape) {
            this.moduleShape = Objects.requireNonNull(moduleShape, "moduleShape must not be null");
            return this;
        }

        /** Build the immutable {@link Barcode2DLayoutConfig}. */
        public Barcode2DLayoutConfig build() {
            return new Barcode2DLayoutConfig(
                    moduleSizePx, quietZoneModules, foreground, background, moduleShape);
        }
    }
}
