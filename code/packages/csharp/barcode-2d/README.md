# CodingAdventures.Barcode2D

C# port of the `@coding-adventures/barcode-2d` abstraction layer.

## What it does

This package provides the two building blocks every 2D barcode renderer needs:

1. **`ModuleGrid`** — the universal intermediate representation produced by
   every 2D barcode encoder (QR Code, Data Matrix, Aztec Code, PDF417,
   MaxiCode). It is an immutable 2D boolean grid: `true` = dark module,
   `false` = light module.

2. **`Barcode2D.Layout()`** — the single function that converts abstract module
   coordinates into pixel-level `PaintScene` instructions ready for the PaintVM
   to render.

## Where it fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → Barcode2D.Layout()  ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by the PaintVM
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `Layout()` are measured in "module units" — abstract
grid steps. Only `Layout()` multiplies by `ModuleSizePx` to produce real pixel
coordinates. Encoders never need to know anything about screen resolution or
output format.

## Supported module shapes

| Shape  | Used by                                         | Render instruction |
|--------|-------------------------------------------------|--------------------|
| Square | QR Code, Data Matrix, Aztec Code, PDF417        | `PaintRect`        |
| Hex    | MaxiCode (ISO/IEC 16023, flat-top hexagons)     | `PaintPath`        |

## Usage

```csharp
using CodingAdventures.Barcode2D;

// 1. Create an all-light grid (21×21 = QR Code v1).
var grid = ModuleGrid.Create(21, 21);

// 2. An encoder sets modules dark one by one (immutable updates).
grid = grid.SetModule(0, 0, true);
grid = grid.SetModule(0, 1, false); // already light, no-op in value terms

// 3. Convert to a PaintScene using default config (10 px modules, 4-module quiet zone).
var scene = Barcode2D.Layout(grid);

// 4. Customise rendering.
var cfg = Barcode2DLayoutConfig.Default with
{
    ModuleSizePx     = 5,
    QuietZoneModules = 4,
    Foreground       = "#1a1a1a",
    Background       = "#fafafa",
};
var customScene = Barcode2D.Layout(grid, cfg);

// 5. MaxiCode hex grid (33 rows × 30 cols, flat-top hexagons).
var hexGrid = ModuleGrid.Create(33, 30, ModuleShape.Hex);
hexGrid = hexGrid.SetModule(16, 15, true); // centre module
var hexCfg = new Barcode2DLayoutConfig(10, 1, "#000000", "#ffffff", ModuleShape.Hex);
var hexScene = Barcode2D.LayoutHex(hexGrid, hexCfg);
```

## API reference

### `ModuleGrid`

| Member | Description |
|--------|-------------|
| `Create(rows, cols, shape?)` | Factory. All modules start light. Defaults to `Square`. |
| `SetModule(row, col, dark)` | Immutable update. Returns new grid. Throws `ArgumentOutOfRangeException` on bad coords. |
| `Rows`, `Cols` | Grid dimensions. |
| `Modules[row][col]` | `true` = dark, `false` = light. |
| `ModuleShape` | `Square` or `Hex`. |

### `Barcode2DLayoutConfig`

Record with a `Default` singleton. Use `with` expressions to create variants.

| Field | Default | Description |
|-------|---------|-------------|
| `ModuleSizePx` | `10` | Pixels per module (> 0). |
| `QuietZoneModules` | `4` | Quiet-zone modules on each side (≥ 0). |
| `Foreground` | `"#000000"` | Colour for dark modules. |
| `Background` | `"#ffffff"` | Colour for light modules and quiet zone. |
| `ModuleShape` | `Square` | Must match the grid's shape. |

### `Barcode2D` (static)

| Method | Description |
|--------|-------------|
| `Layout(grid, config?)` | Validate + dispatch. The primary API. |
| `LayoutSquare(grid, config?)` | Square renderer. Validates independently. |
| `LayoutHex(grid, config?)` | Hex renderer. Validates independently. |

## Dependencies

- `CodingAdventures.PaintInstructions` — provides `PaintScene`, `PaintRect`,
  `PaintPath`, and associated types.

## Tests

71 xUnit tests; all pass with 100% line, branch, and method coverage.

```
cd code/packages/csharp/barcode-2d
mise exec -- dotnet test tests/CodingAdventures.Barcode2D.Tests/CodingAdventures.Barcode2D.Tests.csproj
```
