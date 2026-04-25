using static CodingAdventures.PaintInstructions.PaintInstructions;

namespace CodingAdventures.Barcode2D;

// =============================================================================
// Barcode2D.cs — 2D Barcode Abstraction Layer
// =============================================================================
//
// This file is the C# port of the TypeScript @coding-adventures/barcode-2d
// package. It sits in the rendering pipeline at a single, well-defined point:
//
//   Input data
//     → format encoder (qr-code, data-matrix, aztec…)
//     → ModuleGrid           ← the encoder fills in dark/light modules
//     → Barcode2D.Layout()   ← THIS FILE converts modules to pixel instructions
//     → PaintScene           ← consumed by PaintVM
//     → backend (SVG, Metal, Canvas, terminal…)
//
// All coordinates before Layout() are in "module units" — abstract grid steps.
// Only Layout() multiplies by ModuleSizePx to produce real pixel coordinates.
//
// ## Why two shapes?
//
// Every 2D barcode standard either uses:
//   - Square modules: QR Code (ISO 18004), Data Matrix (ISO 16022),
//     Aztec Code (ISO 24778), PDF417 (ISO 15438).
//   - Flat-top hexagons: MaxiCode (ISO 16023) — used on UPS parcels.
//
// The moduleShape field on ModuleGrid picks between LayoutSquare and LayoutHex.
//
// ## Immutability
//
// ModuleGrid is intentionally immutable: SetModule() returns a new grid.
// This makes encoders easy to test (no accidental mutation side-effects) and
// enables backtracking patterns (e.g. QR mask evaluation) with zero cost.

// =============================================================================
// ModuleShape — the two kinds of modules found in 2D barcode standards
// =============================================================================

/// <summary>
/// The geometric shape of each module in the grid.
///
/// <list type="bullet">
///   <item>
///     <term>Square</term>
///     <description>
///       Used by QR Code, Data Matrix, Aztec Code, and PDF417.
///       The overwhelmingly common shape. Each module renders as a filled
///       rectangle via a <c>PaintRect</c> instruction.
///     </description>
///   </item>
///   <item>
///     <term>Hex</term>
///     <description>
///       Used by MaxiCode (ISO/IEC 16023), the fixed-size postal barcode
///       on UPS and FedEx parcels. MaxiCode uses flat-top hexagons arranged
///       in an offset-row grid. Each module renders as a <c>PaintPath</c>
///       tracing six vertices.
///     </description>
///   </item>
/// </list>
/// </summary>
public enum ModuleShape
{
    /// <summary>Square modules — QR Code, Data Matrix, Aztec, PDF417.</summary>
    Square,

    /// <summary>Flat-top hexagonal modules — MaxiCode (ISO/IEC 16023).</summary>
    Hex,
}

// =============================================================================
// ModuleGrid — the universal output of every 2D barcode encoder
// =============================================================================

/// <summary>
/// The universal intermediate representation produced by every 2D barcode
/// encoder. It is an immutable 2D boolean grid:
///
/// <code>
/// Modules[row][col] == true   →  dark module (ink / filled)
/// Modules[row][col] == false  →  light module (background / empty)
/// </code>
///
/// Row 0 is the top row; column 0 is the leftmost column. This matches the
/// natural reading order used in every 2D barcode standard.
///
/// <para>
/// <strong>Immutability:</strong> Use <see cref="SetModule"/> to produce a new
/// grid with a single module changed. The original grid is never modified.
/// Only the affected row is re-allocated; all other rows are shared between
/// old and new instances, keeping per-module updates O(cols).
/// </para>
///
/// <para>
/// <strong>MaxiCode fixed size:</strong> MaxiCode grids are always 33 rows ×
/// 30 columns with <see cref="ModuleShape.Hex"/>. Physical symbols are always
/// approximately 1 inch × 1 inch.
/// </para>
/// </summary>
public class ModuleGrid
{
    /// <summary>Number of rows (height) in the grid.</summary>
    public int Rows { get; }

    /// <summary>Number of columns (width) in the grid.</summary>
    public int Cols { get; }

    /// <summary>
    /// Two-dimensional boolean grid. Access with <c>Modules[row][col]</c>.
    /// <c>true</c> = dark module, <c>false</c> = light module.
    /// </summary>
    public IReadOnlyList<IReadOnlyList<bool>> Modules { get; }

    /// <summary>Shape of each module — square (common) or flat-top hex (MaxiCode).</summary>
    public ModuleShape ModuleShape { get; }

    internal ModuleGrid(int rows, int cols, IReadOnlyList<IReadOnlyList<bool>> modules, ModuleShape moduleShape)
    {
        Rows = rows;
        Cols = cols;
        Modules = modules;
        ModuleShape = moduleShape;
    }

    /// <summary>
    /// Create a new <see cref="ModuleGrid"/> with every module set to
    /// <c>false</c> (light). This is the starting point for every 2D barcode
    /// encoder.
    ///
    /// <example>
    /// <code>
    /// // Start a 21×21 QR Code v1 grid (all light)
    /// var grid = ModuleGrid.Create(21, 21);
    /// // grid.Modules[0][0] == false
    /// // grid.Rows == 21, grid.Cols == 21
    /// </code>
    /// </example>
    /// </summary>
    /// <param name="rows">Number of rows (height).</param>
    /// <param name="cols">Number of columns (width).</param>
    /// <param name="moduleShape">Shape of each module. Defaults to <see cref="ModuleShape.Square"/>.</param>
    /// <returns>A new all-light <see cref="ModuleGrid"/>.</returns>
    public static ModuleGrid Create(int rows, int cols, ModuleShape moduleShape = ModuleShape.Square)
    {
        // Build a 2D array of false values. Each row is an independent bool[]
        // so that SetModule can replace one row without copying the entire grid.
        var modules = new IReadOnlyList<bool>[rows];
        for (int r = 0; r < rows; r++)
        {
            modules[r] = new bool[cols]; // default bool is false = light
        }
        return new ModuleGrid(rows, cols, modules, moduleShape);
    }

    /// <summary>
    /// Return a new <see cref="ModuleGrid"/> identical to this one except that
    /// the module at <c>(row, col)</c> is set to <paramref name="dark"/>.
    ///
    /// <para>
    /// <strong>Pure and immutable:</strong> this method never modifies the
    /// original grid. Only the affected row is re-allocated; all other rows are
    /// shared between the old and new grids (structural sharing).
    /// </para>
    ///
    /// <example>
    /// <code>
    /// var g  = ModuleGrid.Create(3, 3);
    /// var g2 = g.SetModule(1, 1, true);
    /// // g.Modules[1][1]  == false  (original unchanged)
    /// // g2.Modules[1][1] == true
    /// // !ReferenceEquals(g, g2)   (new object)
    /// </code>
    /// </example>
    /// </summary>
    /// <param name="row">Zero-based row index.</param>
    /// <param name="col">Zero-based column index.</param>
    /// <param name="dark"><c>true</c> for a dark (ink) module; <c>false</c> for light.</param>
    /// <returns>A new grid with the single module changed.</returns>
    /// <exception cref="ArgumentOutOfRangeException">
    /// Thrown if <paramref name="row"/> or <paramref name="col"/> is outside
    /// the grid bounds. This is always a programming error in the encoder.
    /// </exception>
    public ModuleGrid SetModule(int row, int col, bool dark)
    {
        if (row < 0 || row >= Rows)
            throw new ArgumentOutOfRangeException(nameof(row),
                $"SetModule: row {row} out of range [0, {Rows - 1}]");
        if (col < 0 || col >= Cols)
            throw new ArgumentOutOfRangeException(nameof(col),
                $"SetModule: col {col} out of range [0, {Cols - 1}]");

        // Copy only the affected row; all other rows are shared (structural sharing).
        var newRow = ((bool[])Modules[row]).ToArray();
        newRow[col] = dark;

        // Build a new rows array. Rows other than the changed one are re-used.
        var newModules = new IReadOnlyList<bool>[Rows];
        for (int r = 0; r < Rows; r++)
        {
            newModules[r] = r == row ? newRow : Modules[r];
        }

        return new ModuleGrid(Rows, Cols, newModules, ModuleShape);
    }
}

// =============================================================================
// ModuleRole — what a module structurally represents (for annotated grids)
// =============================================================================

/// <summary>
/// The structural role of a module within its barcode symbol.
///
/// These roles are generic — they apply across all 2D barcode formats.
///
/// <list type="bullet">
///   <item><term>Finder</term><description>Locator pattern. QR Code uses three 7×7 corner finders. Aztec uses the central bullseye.</description></item>
///   <item><term>Separator</term><description>Quiet-zone strip between finder and data area. Always light.</description></item>
///   <item><term>Timing</term><description>Alternating calibration strip. Enables the scanner to measure module size.</description></item>
///   <item><term>Alignment</term><description>Secondary locator in high-version QR Code symbols.</description></item>
///   <item><term>Format</term><description>Encodes ECC level + mask indicator (QR) or other symbol metadata.</description></item>
///   <item><term>Data</term><description>One bit of an encoded codeword.</description></item>
///   <item><term>Ecc</term><description>One bit of an error correction codeword (Reed-Solomon).</description></item>
///   <item><term>Padding</term><description>Filler bits when the message is shorter than symbol capacity.</description></item>
/// </list>
/// </summary>
public enum ModuleRole
{
    /// <summary>Finder/locator pattern module.</summary>
    Finder,
    /// <summary>Separator (quiet zone) between finder and data.</summary>
    Separator,
    /// <summary>Timing/calibration strip module.</summary>
    Timing,
    /// <summary>Alignment pattern module (high-version QR only).</summary>
    Alignment,
    /// <summary>Format information module.</summary>
    Format,
    /// <summary>Data codeword bit.</summary>
    Data,
    /// <summary>Error correction codeword bit.</summary>
    Ecc,
    /// <summary>Padding/remainder bit.</summary>
    Padding,
}

// =============================================================================
// ModuleAnnotation — per-module role metadata for visualizers
// =============================================================================

/// <summary>
/// Per-module role annotation, used by visualizers to colour-code the symbol.
///
/// <para>
/// Annotations are entirely optional. <see cref="Barcode2D.Layout"/> only
/// reads <see cref="ModuleGrid.Modules"/>; it never inspects annotations.
/// </para>
///
/// <para>
/// For <see cref="ModuleRole.Data"/> and <see cref="ModuleRole.Ecc"/> modules,
/// <see cref="CodewordIndex"/> and <see cref="BitIndex"/> identify exactly
/// which bit in which codeword this module encodes. Useful for visualizers
/// that highlight one codeword at a time.
/// </para>
///
/// <para>
/// <see cref="Metadata"/> is an escape hatch for format-specific annotations,
/// e.g. <c>{ "format_role": "qr:dark-module" }</c> or
/// <c>{ "format_role": "pdf417:row-indicator", "row": "4" }</c>.
/// </para>
/// </summary>
/// <param name="Role">The structural role of the module.</param>
/// <param name="Dark">Whether the module is dark (ink).</param>
/// <param name="CodewordIndex">Zero-based index into the interleaved codeword stream (data/ecc only).</param>
/// <param name="BitIndex">Zero-based bit index within the codeword, 0 = MSB (data/ecc only).</param>
/// <param name="Metadata">Optional format-specific key/value pairs.</param>
public sealed record ModuleAnnotation(
    ModuleRole Role,
    bool Dark,
    int? CodewordIndex = null,
    int? BitIndex = null,
    IReadOnlyDictionary<string, string>? Metadata = null);

// =============================================================================
// AnnotatedModuleGrid — ModuleGrid with per-module role annotations
// =============================================================================

/// <summary>
/// A <see cref="ModuleGrid"/> extended with per-module role annotations.
///
/// <para>
/// Used by visualizers to render colour-coded diagrams that teach how the
/// barcode is structured. <see cref="Annotations"/>[row][col] corresponds to
/// <c>Modules[row][col]</c>. A <c>null</c> annotation means no annotation is
/// available for that module.
/// </para>
///
/// <para>
/// This type is <em>not</em> required for rendering. <see cref="Barcode2D.Layout"/>
/// accepts a plain <see cref="ModuleGrid"/> and ignores annotations.
/// </para>
/// </summary>
public sealed class AnnotatedModuleGrid : ModuleGrid
{
    /// <summary>
    /// Per-module annotations mirroring <c>Modules</c>. <c>null</c> = no annotation.
    /// </summary>
    public IReadOnlyList<IReadOnlyList<ModuleAnnotation?>> Annotations { get; }

    /// <summary>
    /// Create an <see cref="AnnotatedModuleGrid"/> wrapping an existing grid and
    /// its annotation layer.
    /// </summary>
    public AnnotatedModuleGrid(
        ModuleGrid grid,
        IReadOnlyList<IReadOnlyList<ModuleAnnotation?>> annotations)
        : base(grid.Rows, grid.Cols, grid.Modules, grid.ModuleShape)
    {
        Annotations = annotations;
    }
}

// =============================================================================
// Barcode2DLayoutConfig — pixel-level rendering options
// =============================================================================

/// <summary>
/// Configuration record for <see cref="Barcode2D.Layout"/>.
///
/// <para>All fields have sensible defaults (see <see cref="Barcode2DLayoutConfig.Default"/>)
/// so you only need to specify what differs from the norm.</para>
///
/// <list type="table">
///   <listheader><term>Field</term><description>Default / Why</description></listheader>
///   <item><term>ModuleSizePx</term><description>10 — produces a readable QR at 210×210 px</description></item>
///   <item><term>QuietZoneModules</term><description>4 — QR Code minimum per ISO/IEC 18004</description></item>
///   <item><term>Foreground</term><description>#000000 — black ink on white paper</description></item>
///   <item><term>Background</term><description>#ffffff — white paper</description></item>
///   <item><term>ModuleShape</term><description>Square — the overwhelmingly common case</description></item>
/// </list>
/// </summary>
/// <param name="ModuleSizePx">
/// Size of one module in pixels. For square modules this is both width and height.
/// For hex modules it is the hexagon's width (flat-to-flat = side length).
/// Must be &gt; 0.
/// </param>
/// <param name="QuietZoneModules">
/// Number of module-width quiet-zone units added on each side of the grid.
/// QR Code requires ≥ 4. Data Matrix requires ≥ 1. Must be ≥ 0.
/// </param>
/// <param name="Foreground">CSS-style colour string for dark modules, e.g. <c>"#000000"</c>.</param>
/// <param name="Background">CSS-style colour string for light modules and quiet zone.</param>
/// <param name="ModuleShape">
/// Must match <see cref="ModuleGrid.ModuleShape"/>. If they disagree,
/// <see cref="Barcode2D.Layout"/> throws <see cref="InvalidBarcode2DConfigException"/>.
/// </param>
public sealed record Barcode2DLayoutConfig(
    double ModuleSizePx,
    int QuietZoneModules,
    string Foreground,
    string Background,
    ModuleShape ModuleShape)
{
    /// <summary>
    /// Sensible defaults:
    /// ModuleSizePx=10, QuietZoneModules=4, Foreground="#000000",
    /// Background="#ffffff", ModuleShape=Square.
    /// </summary>
    public static readonly Barcode2DLayoutConfig Default = new(
        ModuleSizePx: 10,
        QuietZoneModules: 4,
        Foreground: "#000000",
        Background: "#ffffff",
        ModuleShape: ModuleShape.Square);
}

// =============================================================================
// Exceptions
// =============================================================================

/// <summary>Base class for all barcode-2d exceptions.</summary>
public class Barcode2DException : Exception
{
    /// <inheritdoc />
    public Barcode2DException(string message) : base(message) { }
}

/// <summary>
/// Thrown by <see cref="Barcode2D.Layout"/> when the configuration is invalid:
/// <list type="bullet">
///   <item><description><c>ModuleSizePx &lt;= 0</c></description></item>
///   <item><description><c>QuietZoneModules &lt; 0</c></description></item>
///   <item><description><c>config.ModuleShape</c> does not match <c>grid.ModuleShape</c></description></item>
/// </list>
/// </summary>
public sealed class InvalidBarcode2DConfigException : Barcode2DException
{
    /// <inheritdoc />
    public InvalidBarcode2DConfigException(string message) : base(message) { }
}

// =============================================================================
// Barcode2D — the main static API
// =============================================================================

/// <summary>
/// Main entry point for the barcode-2d abstraction layer.
///
/// <para>
/// Three static methods are exposed:
/// </para>
///
/// <list type="bullet">
///   <item>
///     <term><see cref="Layout"/></term>
///     <description>
///       Validates config and dispatches to the right shape renderer.
///       Prefer this over calling <see cref="LayoutSquare"/> or
///       <see cref="LayoutHex"/> directly.
///     </description>
///   </item>
///   <item>
///     <term><see cref="LayoutSquare"/></term>
///     <description>
///       Renders square-module grids (QR Code, Data Matrix, Aztec, PDF417).
///       Validates the config independently.
///     </description>
///   </item>
///   <item>
///     <term><see cref="LayoutHex"/></term>
///     <description>
///       Renders flat-top hex-module grids (MaxiCode). Validates independently.
///     </description>
///   </item>
/// </list>
/// </summary>
public static class Barcode2D
{
    /// <summary>Package version.</summary>
    public const string VERSION = "0.1.0";

    // =========================================================================
    // Layout — the primary public API
    // =========================================================================

    /// <summary>
    /// Convert a <see cref="ModuleGrid"/> into a
    /// <see cref="CodingAdventures.PaintInstructions.PaintScene"/> ready for
    /// the PaintVM.
    ///
    /// <para>
    /// This is the <em>only</em> function in the entire 2D barcode stack that
    /// knows about pixels. Everything above this step works in abstract module
    /// units. Everything below is handled by a paint backend.
    /// </para>
    ///
    /// <para>
    /// <paramref name="config"/> defaults to <see cref="Barcode2DLayoutConfig.Default"/>
    /// when <c>null</c> is passed.
    /// </para>
    /// </summary>
    /// <param name="grid">The module grid to render.</param>
    /// <param name="config">
    /// Pixel-level rendering options. Pass <c>null</c> to use
    /// <see cref="Barcode2DLayoutConfig.Default"/>.
    /// </param>
    /// <returns>A <see cref="CodingAdventures.PaintInstructions.PaintScene"/> ready for the PaintVM.</returns>
    /// <exception cref="InvalidBarcode2DConfigException">
    /// Thrown when <c>ModuleSizePx &lt;= 0</c>, <c>QuietZoneModules &lt; 0</c>,
    /// or <c>config.ModuleShape != grid.ModuleShape</c>.
    /// </exception>
    public static CodingAdventures.PaintInstructions.PaintScene Layout(
        ModuleGrid grid,
        Barcode2DLayoutConfig? config = null)
    {
        var cfg = config ?? Barcode2DLayoutConfig.Default;

        ValidateConfig(cfg, grid);

        return cfg.ModuleShape == ModuleShape.Square
            ? RenderSquare(grid, cfg)
            : RenderHex(grid, cfg);
    }

    // =========================================================================
    // LayoutSquare — square-module grids (QR, Data Matrix, Aztec, PDF417)
    // =========================================================================

    /// <summary>
    /// Render a <see cref="ModuleGrid"/> with <see cref="ModuleShape.Square"/>
    /// modules into a <see cref="CodingAdventures.PaintInstructions.PaintScene"/>.
    ///
    /// <para>
    /// Each dark module at <c>(row, col)</c> becomes one
    /// <c>PaintRect</c> instruction:
    /// <code>
    /// quietZonePx = QuietZoneModules × ModuleSizePx
    /// x = quietZonePx + col × ModuleSizePx
    /// y = quietZonePx + row × ModuleSizePx
    /// </code>
    /// </para>
    ///
    /// <para>
    /// Total canvas including quiet zone on all four sides:
    /// <code>
    /// totalWidth  = (cols + 2 × QuietZoneModules) × ModuleSizePx
    /// totalHeight = (rows + 2 × QuietZoneModules) × ModuleSizePx
    /// </code>
    /// </para>
    ///
    /// <para>
    /// The scene always starts with one background <c>PaintRect</c> covering
    /// the full symbol, ensuring the quiet zone and light modules are filled
    /// even if the backend default is transparent.
    /// </para>
    /// </summary>
    /// <param name="grid">The module grid to render. Must have <see cref="ModuleShape.Square"/>.</param>
    /// <param name="config">Rendering options. Defaults to <see cref="Barcode2DLayoutConfig.Default"/>.</param>
    /// <returns>A <see cref="CodingAdventures.PaintInstructions.PaintScene"/>.</returns>
    /// <exception cref="InvalidBarcode2DConfigException">Thrown if config is invalid.</exception>
    public static CodingAdventures.PaintInstructions.PaintScene LayoutSquare(
        ModuleGrid grid,
        Barcode2DLayoutConfig? config = null)
    {
        var cfg = config ?? Barcode2DLayoutConfig.Default;
        ValidateConfig(cfg, grid);
        return RenderSquare(grid, cfg);
    }

    // =========================================================================
    // LayoutHex — flat-top hex-module grids (MaxiCode)
    // =========================================================================

    /// <summary>
    /// Render a <see cref="ModuleGrid"/> with <see cref="ModuleShape.Hex"/>
    /// modules into a <see cref="CodingAdventures.PaintInstructions.PaintScene"/>.
    ///
    /// <para>
    /// Used for MaxiCode (ISO/IEC 16023), the fixed-size 2D barcode used by
    /// UPS and FedEx. MaxiCode uses flat-top hexagons in an offset-row grid:
    /// </para>
    ///
    /// <code>
    /// Row 0:  ⬡ ⬡ ⬡ ⬡ ⬡     (no horizontal offset)
    /// Row 1:   ⬡ ⬡ ⬡ ⬡ ⬡    (offset right by hexWidth/2)
    /// Row 2:  ⬡ ⬡ ⬡ ⬡ ⬡
    /// </code>
    ///
    /// <para>
    /// Geometry for a flat-top hexagon centered at (cx, cy) with circumradius R:
    /// </para>
    /// <code>
    /// hexWidth  = ModuleSizePx
    /// hexHeight = ModuleSizePx × (√3 / 2)
    /// circumR   = ModuleSizePx / √3
    ///
    /// Vertex i: ( cx + R × cos(i × 60°),
    ///             cy + R × sin(i × 60°) )   for i in [0..5]
    ///
    /// cx = quietZonePx + col × hexWidth + (row % 2) × (hexWidth / 2)
    /// cy = quietZonePx + row × hexHeight
    /// </code>
    /// </summary>
    /// <param name="grid">The module grid to render. Must have <see cref="ModuleShape.Hex"/>.</param>
    /// <param name="config">Rendering options. Must set <c>ModuleShape = Hex</c>.</param>
    /// <returns>A <see cref="CodingAdventures.PaintInstructions.PaintScene"/>.</returns>
    /// <exception cref="InvalidBarcode2DConfigException">Thrown if config is invalid.</exception>
    public static CodingAdventures.PaintInstructions.PaintScene LayoutHex(
        ModuleGrid grid,
        Barcode2DLayoutConfig? config = null)
    {
        var cfg = config ?? new Barcode2DLayoutConfig(
            ModuleSizePx: 10,
            QuietZoneModules: 4,
            Foreground: "#000000",
            Background: "#ffffff",
            ModuleShape: ModuleShape.Hex);
        ValidateConfig(cfg, grid);
        return RenderHex(grid, cfg);
    }

    // =========================================================================
    // Private: ValidateConfig
    // =========================================================================

    /// <summary>
    /// Validate a <see cref="Barcode2DLayoutConfig"/> against a grid.
    /// Throws <see cref="InvalidBarcode2DConfigException"/> on any violation.
    /// </summary>
    private static void ValidateConfig(Barcode2DLayoutConfig cfg, ModuleGrid grid)
    {
        if (cfg.ModuleSizePx <= 0)
            throw new InvalidBarcode2DConfigException(
                $"ModuleSizePx must be > 0, got {cfg.ModuleSizePx}");

        if (cfg.QuietZoneModules < 0)
            throw new InvalidBarcode2DConfigException(
                $"QuietZoneModules must be >= 0, got {cfg.QuietZoneModules}");

        if (cfg.ModuleShape != grid.ModuleShape)
            throw new InvalidBarcode2DConfigException(
                $"config.ModuleShape \"{cfg.ModuleShape}\" does not match " +
                $"grid.ModuleShape \"{grid.ModuleShape}\"");
    }

    // =========================================================================
    // Private: RenderSquare
    // =========================================================================

    /// <summary>
    /// Internal square-module renderer.
    ///
    /// Algorithm:
    ///   1. Compute total pixel dimensions including quiet zone.
    ///   2. Emit one background PaintRect covering the entire symbol.
    ///   3. For each dark module, emit one filled PaintRect.
    ///
    /// Light modules are covered implicitly by the background rect — no
    /// explicit light rects are emitted, keeping instruction count proportional
    /// to the number of dark modules rather than the total grid area.
    /// </summary>
    private static CodingAdventures.PaintInstructions.PaintScene RenderSquare(
        ModuleGrid grid, Barcode2DLayoutConfig cfg)
    {
        var sz = cfg.ModuleSizePx;
        var qz = cfg.QuietZoneModules * sz;   // quiet zone in pixels

        var totalWidth  = (grid.Cols + 2 * cfg.QuietZoneModules) * sz;
        var totalHeight = (grid.Rows + 2 * cfg.QuietZoneModules) * sz;

        var instructions = new List<CodingAdventures.PaintInstructions.PaintInstructionBase>();

        // ── 1. Background rect ───────────────────────────────────────────────
        // Covers the entire canvas so the quiet zone and all light modules are
        // filled with the background colour even on transparent-default backends.
        instructions.Add(PaintRect(0, 0, totalWidth, totalHeight,
            new CodingAdventures.PaintInstructions.PaintRectOptions { Fill = cfg.Background }));

        // ── 2. One rect per dark module ──────────────────────────────────────
        for (int row = 0; row < grid.Rows; row++)
        {
            for (int col = 0; col < grid.Cols; col++)
            {
                if (!grid.Modules[row][col]) continue;

                // Top-left pixel corner of this module.
                double x = qz + col * sz;
                double y = qz + row * sz;

                instructions.Add(PaintRect(x, y, sz, sz,
                    new CodingAdventures.PaintInstructions.PaintRectOptions { Fill = cfg.Foreground }));
            }
        }

        return PaintScene(totalWidth, totalHeight, cfg.Background, instructions);
    }

    // =========================================================================
    // Private: RenderHex
    // =========================================================================

    /// <summary>
    /// Internal flat-top hex-module renderer (MaxiCode geometry).
    ///
    /// A "flat-top" hexagon has two flat edges at top and bottom:
    ///
    /// <code>
    ///    ___
    ///   /   \   ← two vertices at the top
    ///  |     |
    ///   \___/   ← two vertices at the bottom
    /// </code>
    ///
    /// Vertex angles start at 0° (right midpoint) and step 60° clockwise:
    ///
    /// <code>
    ///  i   angle  cos(θ)   sin(θ)   role
    ///  0     0°    1        0        right midpoint
    ///  1    60°    0.5      √3/2     bottom-right
    ///  2   120°   -0.5      √3/2     bottom-left
    ///  3   180°   -1        0        left midpoint
    ///  4   240°   -0.5     -√3/2     top-left
    ///  5   300°    0.5     -√3/2     top-right
    /// </code>
    ///
    /// Tiling constants (where s = ModuleSizePx = flat-to-flat width = side length):
    ///
    /// <code>
    /// hexWidth  = s
    /// hexHeight = s × (√3 / 2)   ← vertical step between row centres
    /// circumR   = s / √3          ← centre-to-vertex distance
    /// </code>
    /// </summary>
    private static CodingAdventures.PaintInstructions.PaintScene RenderHex(
        ModuleGrid grid, Barcode2DLayoutConfig cfg)
    {
        var sz = cfg.ModuleSizePx;

        // Hex tiling geometry.
        double hexWidth  = sz;
        double hexHeight = sz * (Math.Sqrt(3.0) / 2.0);
        double circumR   = sz / Math.Sqrt(3.0);

        double qz = cfg.QuietZoneModules * sz; // quiet zone in pixels

        // +hexWidth/2 so the odd-row offset columns don't clip outside canvas.
        double totalWidth  = (grid.Cols + 2 * cfg.QuietZoneModules) * hexWidth + hexWidth / 2.0;
        double totalHeight = (grid.Rows + 2 * cfg.QuietZoneModules) * hexHeight;

        var instructions = new List<CodingAdventures.PaintInstructions.PaintInstructionBase>();

        // Background rect.
        instructions.Add(PaintRect(0, 0, totalWidth, totalHeight,
            new CodingAdventures.PaintInstructions.PaintRectOptions { Fill = cfg.Background }));

        // One PaintPath per dark module.
        for (int row = 0; row < grid.Rows; row++)
        {
            for (int col = 0; col < grid.Cols; col++)
            {
                if (!grid.Modules[row][col]) continue;

                // Centre of this hexagon in pixel space.
                // Odd rows are offset right by half a hexagon width.
                double cx = qz + col * hexWidth + (row % 2) * (hexWidth / 2.0);
                double cy = qz + row * hexHeight;

                var cmds = BuildFlatTopHexPath(cx, cy, circumR);
                instructions.Add(PaintPath(cmds,
                    new CodingAdventures.PaintInstructions.PaintPathOptions { Fill = cfg.Foreground }));
            }
        }

        return PaintScene(totalWidth, totalHeight, cfg.Background, instructions);
    }

    // =========================================================================
    // Private: BuildFlatTopHexPath
    // =========================================================================

    /// <summary>
    /// Build the six <c>PathCommand</c>s for a flat-top regular hexagon centered
    /// at (<paramref name="cx"/>, <paramref name="cy"/>) with circumradius
    /// <paramref name="circumR"/>.
    ///
    /// <para>
    /// The six vertices are at angles 0°, 60°, 120°, 180°, 240°, 300° from the
    /// centre:
    /// </para>
    ///
    /// <code>
    /// vertex_i = ( cx + R × cos(i × 60°),
    ///              cy + R × sin(i × 60°) )
    /// </code>
    ///
    /// <para>
    /// Path: MoveTo(vertex_0), LineTo(vertex_1..5), ClosePath.
    /// </para>
    /// </summary>
    /// <param name="cx">Centre x in pixels.</param>
    /// <param name="cy">Centre y in pixels.</param>
    /// <param name="circumR">Circumscribed circle radius (centre to vertex) in pixels.</param>
    /// <returns>Seven path commands: one MoveTo, five LineTo, one ClosePath.</returns>
    private static IReadOnlyList<CodingAdventures.PaintInstructions.PathCommand> BuildFlatTopHexPath(
        double cx, double cy, double circumR)
    {
        const double DegToRad = Math.PI / 180.0;

        var commands = new List<CodingAdventures.PaintInstructions.PathCommand>(7);

        // Vertex 0: MoveTo
        double angle0 = 0 * 60 * DegToRad; // = 0
        commands.Add(new CodingAdventures.PaintInstructions.MoveToCommand(
            cx + circumR * Math.Cos(angle0),
            cy + circumR * Math.Sin(angle0)));

        // Vertices 1–5: LineTo
        for (int i = 1; i <= 5; i++)
        {
            double angle = i * 60.0 * DegToRad;
            commands.Add(new CodingAdventures.PaintInstructions.LineToCommand(
                cx + circumR * Math.Cos(angle),
                cy + circumR * Math.Sin(angle)));
        }

        // Close back to vertex 0.
        commands.Add(new CodingAdventures.PaintInstructions.ClosePathCommand());

        return commands;
    }
}
