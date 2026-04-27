# barcode-2d

Shared 2D barcode abstraction layer for Perl.

## What it does

This package provides the two building blocks every 2D barcode encoder needs:

1. **ModuleGrid** — the universal intermediate representation produced by every
   2D barcode encoder (QR, Data Matrix, Aztec, PDF417, MaxiCode). It is a 2D
   boolean grid: `1` = dark module, `0` = light module.

2. **`layout()`** — the single function that converts abstract module coordinates
   into pixel-level `PaintScene` instructions ready for the PaintVM (P2D01) to
   render.

## Where this fits in the pipeline

```
Input data
  → format encoder (qr-code, data-matrix, aztec…)
  → ModuleGrid          ← produced by the encoder
  → layout()            ← THIS PACKAGE converts to pixels
  → PaintScene          ← consumed by paint-vm (P2D01)
  → backend (SVG, Metal, Canvas, terminal…)
```

All coordinates before `layout()` are measured in "module units" — abstract
grid steps. Only `layout()` multiplies by `module_size_px` to produce real
pixel coordinates.

## Usage

```perl
use lib 'path/to/paint-instructions/lib';
use CodingAdventures::Barcode2D;

# 1. Start with an all-light 21×21 grid (QR Code v1 size).
my $grid = CodingAdventures::Barcode2D->make_module_grid(21, 21);

# 2. Paint modules dark (encoder logic goes here).
$grid = CodingAdventures::Barcode2D->set_module($grid, 0, 0, 1);
$grid = CodingAdventures::Barcode2D->set_module($grid, 1, 1, 1);

# 3. Convert to a PaintScene for rendering.
my $scene = CodingAdventures::Barcode2D->layout($grid, {
    module_size_px     => 10,
    quiet_zone_modules => 4,
    foreground         => '#000000',
    background         => '#ffffff',
});

# $scene is a PaintScene hashref ready for the PaintVM.
print "Canvas: $scene->{width}×$scene->{height}\n";
```

## Supported module shapes

- **`square`** (default): used by QR Code, Data Matrix, Aztec Code, PDF417.
  Each module becomes a `PaintRect`.

- **`hex`** (flat-top hexagons): used by MaxiCode (ISO/IEC 16023).
  Each module becomes a `PaintPath` tracing six vertices.

```perl
# MaxiCode: 33×30 hex grid
my $grid = CodingAdventures::Barcode2D->make_module_grid(33, 30, 'hex');
my $scene = CodingAdventures::Barcode2D->layout($grid, {
    module_shape => 'hex',
    module_size_px => 10,
});
```

## API

### `make_module_grid($rows, $cols, $module_shape?)`

Create a new all-light ModuleGrid. `$module_shape` defaults to `'square'`.

### `set_module($grid, $row, $col, $dark)`

Return a new ModuleGrid with module at `($row, $col)` set to `$dark` (1 or 0).
The original `$grid` is never modified.

### `layout($grid, $config?)`

Convert a ModuleGrid into a PaintScene hashref. `$config` is an optional
hashref that overrides any defaults:

| Key                  | Default     | Description                           |
|----------------------|-------------|---------------------------------------|
| `module_size_px`     | `10`        | Pixels per module side                |
| `quiet_zone_modules` | `4`         | Quiet zone modules on each side       |
| `foreground`         | `#000000`   | Dark module fill color                |
| `background`         | `#ffffff`   | Background / light module fill color  |
| `module_shape`       | `square`    | Must match the grid's `module_shape`  |

Croaks with `InvalidBarcode2DConfigError:` prefix on invalid config.

## Development

```bash
bash BUILD
```
