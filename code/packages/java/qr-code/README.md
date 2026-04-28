# java/qr-code

Java implementation of the QR Code encoder — ISO/IEC 18004:2015 compliant.

## What it does

Encodes any UTF-8 string into a scannable QR Code and returns a `ModuleGrid`
(a boolean 2-D grid where `true` = dark module). The grid can be passed to
`barcode-2d`'s layout engine for SVG or pixel-canvas rendering.

## Where it fits

```
your string
    │
    ▼
qr-code          ← you are here: mode selection, RS ECC, grid construction, masking
    │
    ▼
barcode-2d       ← converts ModuleGrid to PaintScene (pixel layout)
    │
    ▼
paint-instructions  ← PaintScene = list of PaintInstruction (rects, paths, fills)
```

Supporting packages pulled in as composite builds:

- `gf256` — GF(256) arithmetic (multiply, power, log tables)
- `polynomial` — polynomial operations
- `reed-solomon` — generic RS encoder/decoder
- `paint-instructions` — PaintScene / PaintInstruction types
- `barcode-2d` — 2D barcode layout engine

## Usage

```java
import com.codingadventures.qrcode.QRCode;
import com.codingadventures.barcode2d.Barcode2DLayoutConfig;
import com.codingadventures.barcode2d.ModuleGrid;
import com.codingadventures.paintinstructions.PaintScene;

// Encode to a boolean grid:
ModuleGrid grid = QRCode.encode("HELLO WORLD", QRCode.EccLevel.M);
System.out.println(grid.rows + "×" + grid.cols);  // "21×21" for version 1

// Render to a PaintScene:
Barcode2DLayoutConfig config = Barcode2DLayoutConfig.defaults();
PaintScene scene = QRCode.encodeAndLayout("https://example.com", QRCode.EccLevel.M, config);
System.out.println("Scene: " + scene.width + "×" + scene.height + " px");
```

## Encoding pipeline

```
input string
  → mode selection    (numeric / alphanumeric / byte)
  → version selection (smallest v1–40 that fits at the ECC level)
  → bit stream        (mode indicator + char count + data + padding)
  → blocks + RS ECC   (GF(256) b=0 convention, poly 0x11D)
  → interleave        (data CWs round-robin, then ECC CWs)
  → grid init         (finder × 3, separators, timing, alignment, format, dark)
  → zigzag placement  (two-column snake from bottom-right)
  → mask evaluation   (8 patterns, 4-rule penalty, pick lowest)
  → finalize          (format info MSB-first + version info v7+)
  → ModuleGrid
```

## ECC levels

| Level | Recovery | Use case |
|-------|----------|----------|
| L     | ~7%      | Clean environments, maximize data density |
| M     | ~15%     | General purpose (recommended default) |
| Q     | ~25%     | Industrial printing, some damage expected |
| H     | ~30%     | Maximum redundancy, overlaid logos |

## Error handling

`QRCode.encode()` and `QRCode.encodeAndLayout()` throw `QRCode.QRCodeException`
(a checked exception) when the input is too long to fit in any version at the
requested ECC level. Version 40 holds at most:

- 7089 numeric characters
- 4296 alphanumeric characters
- 2953 bytes (byte/UTF-8 mode)

## Building and testing

```sh
cd code/packages/java/qr-code
gradle test
```

Requires Java 21 and Gradle (via `mise exec --` in this monorepo).

## Standards

- ISO/IEC 18004:2015 — QR Code specification
- Thonky QR Code Tutorial (thonky.com/qr-code-tutorial)
- Nayuki QR Code generator (nayuki.io) — canonical open-source reference
