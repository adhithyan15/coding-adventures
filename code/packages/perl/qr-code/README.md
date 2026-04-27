# CodingAdventures::QrCode

ISO/IEC 18004:2015 compliant QR Code encoder — pure Perl implementation.

## What is QR Code?

QR Code (Quick Response) was invented by Masahiro Hara at Denso Wave in 1994
to track automotive parts on assembly lines. The goal was to scan a barcode
from any direction — even at oblique angles — without needing to orient the
label first. Today QR Code is the most widely-deployed 2D barcode on Earth:
every smartphone camera can decode one without a separate app.

Three features make QR Code stand out:

1. **Omnidirectionality** — three "finder" patterns (large black squares at
   three corners) let a decoder locate and orient the symbol from any direction.
2. **Error correction** — up to 30% of the symbol can be obscured (logo
   overlaid, torn, smudged) and the message can still be recovered. This uses
   Reed-Solomon codes over GF(256).
3. **High density** — a version-40 symbol holds 7089 numeric characters (or
   2953 bytes) in 177×177 modules.

## Encoding pipeline

```
input string
  → mode selection    (numeric / alphanumeric / byte)
  → version selection (smallest version 1–40 that fits)
  → bit stream        (mode indicator + char count + data + padding)
  → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
  → interleave        (data CWs, then ECC CWs, round-robin)
  → grid init         (finder, separator, timing, alignment, format, dark)
  → zigzag placement  (two-column snake from bottom-right)
  → mask evaluation   (8 patterns, lowest 4-rule penalty wins)
  → finalize          (format info + version info v7+)
  → ModuleGrid        (boolean grid, 1=dark)
```

## Usage

```perl
use CodingAdventures::QrCode;

# Encode a URL at ECC level M (15% recovery)
my $grid = CodingAdventures::QrCode->encode(
    'https://example.com',
    level => 'M',
);

# $grid is a ModuleGrid hashref:
#   { rows => N, cols => N, modules => [[0,1,...], ...], module_shape => 'square' }
printf "Symbol: %d×%d modules\n", $grid->{rows}, $grid->{cols};

# Render to pixel coordinates with barcode-2d layout()
use CodingAdventures::Barcode2D;
my $scene = CodingAdventures::Barcode2D->layout($grid, { module_size_px => 4 });
```

## API

### `encode($data, level => $ecc)`

Encode a UTF-8 string into a QR Code ModuleGrid.

**Parameters:**
- `$data`  — input string (any UTF-8; will be encoded as byte mode if needed)
- `level`  — ECC level: `'L'`, `'M'`, `'Q'`, or `'H'` (default: `'M'`)

**Returns:** a ModuleGrid hashref:
```perl
{
    rows         => $N,        # symbol width/height in modules
    cols         => $N,        # same as rows (QR Code is always square)
    modules      => \@modules, # 2D array, modules[$r][$c] = 1 (dark) or 0 (light)
    module_shape => 'square',
}
```

**Throws:** string exception prefixed with `"InputTooLong:"` if the input
exceeds the version-40 capacity at the chosen ECC level (~2953 bytes in byte
mode, ~7089 numeric digits in numeric mode).

## ECC levels

| Level | Recovery | Use case                          |
|-------|----------|-----------------------------------|
| L     | ~7 %     | Maximum data density              |
| M     | ~15 %    | General-purpose (common default)  |
| Q     | ~25 %    | Moderate noise or damage expected |
| H     | ~30 %    | Logo overlay; high damage risk    |

## Encoding modes

The mode is selected automatically to minimise the output size:

| Mode          | Input             | Encoding                     |
|---------------|-------------------|------------------------------|
| numeric       | digits `0-9` only | 3 digits → 10 bits           |
| alphanumeric  | 45-char QR set    | 2 chars → 11 bits            |
| byte          | any UTF-8         | 1 byte → 8 bits              |

The 45-char alphanumeric set: `0-9 A-Z space $ % * + - . / :`

## Where this fits in the stack

```
encode()            ← this package (QR02)
  ↓ ModuleGrid
layout()            ← barcode-2d (P2D00)
  ↓ PaintScene
paint-vm-svg        ← SVG renderer
paint-vm-ascii      ← terminal renderer
```

## Dependencies

- `CodingAdventures::Barcode2D` — ModuleGrid representation
- `CodingAdventures::GF256`     — GF(256) field arithmetic for RS encoding
- `Encode`                       — UTF-8 encoding (Perl core)

## Version history

See [CHANGELOG.md](CHANGELOG.md).
