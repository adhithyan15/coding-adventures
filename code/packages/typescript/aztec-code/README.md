# @coding-adventures/aztec-code

Aztec Code encoder conforming to **ISO/IEC 24778:2008**.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995. You
encounter it every time you board an aeroplane (IATA boarding passes), ride a
train (Eurostar, Amtrak), or show a US driver's licence (AAMVA). It has become
the de facto standard for ticketing and identity documents because:

- **No quiet zone required** — the scanner finds the central bullseye first,
  then reads outward. The symbol can be printed right to the label edge.
- **Compact at small sizes** — a single uppercase letter fits in a 15×15
  symbol (225 modules total vs. QR Code's 441 for version 1).
- **High density at large sizes** — 32 layers in full mode gives 143×143,
  storing ~3,500 bytes with 23% ECC.

## Stack position

```
paint-vm-svg / paint-vm-canvas / paint-metal
        │
    barcode-2d (layout)
        │
   aztec-code  ← this package
```

## Usage

```typescript
import { encode, encodeAndLayout, renderSvg } from "@coding-adventures/aztec-code";

// Encode to a ModuleGrid (abstract boolean grid).
const grid = encode("https://example.com");
console.log(`${grid.rows}×${grid.cols} Aztec Code`);

// Render to SVG.
const svg = renderSvg("Hello World");
// Inject safely via DOMParser, not innerHTML.
const parser = new DOMParser();
document.body.appendChild(
  parser.parseFromString(svg, "image/svg+xml").documentElement
);

// Options
const gridHighEcc = encode("Hello", { minEccPercent: 50 });
const gridCompact = encode("Hi", { compact: true });
```

## API

### `encode(input, options?) → ModuleGrid`

Encodes a UTF-8 string or `Uint8Array` into an Aztec Code `ModuleGrid`.

**Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `minEccPercent` | `number` | `23` | Minimum ECC percentage (10–90). Higher = larger but more resilient symbol. |
| `compact` | `boolean` | `false` | Force compact form (15×15–27×27). Throws if data does not fit in 4 compact layers. |

**Throws** `InputTooLongError` if the input exceeds the largest supported symbol.

### `encodeAndLayout(input, options?, config?) → PaintScene`

Encode and convert to a `PaintScene` suitable for rendering by `paint-vm`.

### `renderSvg(input, options?, config?) → string`

One-shot convenience: encode and render to an SVG string.

### `explain(input, options?) → AnnotatedModuleGrid`

Encode with per-module role annotations (for interactive visualizers).
v0.1.0 returns null annotations; full annotations are v0.2.0.

## Symbol structure

```
┌─────────────────────────┐
│       quiet zone        │
│  ┌───────────────────┐  │
│  │   data layers     │  │
│  │  ┌─────────────┐  │  │
│  │  │ mode message│  │  │
│  │  │ ┌─────────┐ │  │  │
│  │  │ │bullseye │ │  │  │
│  │  │ └─────────┘ │  │  │
│  │  └─────────────┘  │  │
│  └───────────────────┘  │
└─────────────────────────┘
```

- **Bullseye**: concentric dark/light rings at the center. Compact = 11×11
  (radius 5); Full = 15×15 (radius 7). Even Chebyshev distance = dark.
- **Mode message ring**: immediately outside the bullseye. Four dark corner
  modules are orientation marks; the remaining perimeter bits carry the mode
  message (GF(16) RS protected).
- **Reference grid** (full symbols only): alternating dark/light lines at
  every 16 modules from the center row and column, helping decoders correct
  for severe perspective distortion.
- **Data layers**: clockwise spiral bands of 2 modules wide, radiating
  outward. Each layer adds 4 modules to each dimension.

## v0.1.0 simplifications

- **Byte mode only**: all input is encoded via Binary-Shift from Upper mode.
  Multi-mode optimization (Digit/Upper/Lower/Mixed/Punct) is v0.2.0.
- **8-bit codewords**: RS is computed over GF(256)/0x12D (the Data Matrix /
  Aztec polynomial), implemented inline without importing `gf256` or
  `reed-solomon` (those packages use the QR Code polynomial 0x11D, which is
  incompatible).

## Dependencies

- `@coding-adventures/barcode-2d` — `ModuleGrid` type and `layout()` function.
- `@coding-adventures/paint-vm-svg` — SVG rendering backend.

GF(256)/0x12D and GF(16)/0x13 arithmetic are implemented inline because the
shared `gf256` and `reed-solomon` packages use 0x11D (the QR Code polynomial).

## Testing

```bash
npm test               # vitest
npm run test:coverage  # coverage report
```

Coverage target: ≥ 80% (lines + branches).
