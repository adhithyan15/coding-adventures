# @coding-adventures/aztec-code

Aztec Code encoder — ISO/IEC 24778:2008 compliant.

Aztec Code was invented by Andrew Longacre Jr. at Welch Allyn in 1995. Unlike
QR Code (which requires three corner finder patterns), Aztec uses a single
**bullseye** at the center of the symbol. The scanner finds the center first and
reads outward in a clockwise spiral — no large quiet zone is needed.

## Where Aztec Code is used

- **IATA boarding passes** — the 2D barcode on every airline boarding pass
- **Eurostar and Amtrak rail tickets** — printed and mobile tickets
- **PostNL, Deutsche Post, La Poste** — European postal routing labels
- **US military ID cards**

## Installation

```bash
npm install @coding-adventures/aztec-code
```

## Quick start

```typescript
import { encode, renderSvg } from "@coding-adventures/aztec-code";

// Encode a string to a module grid
const grid = encode("Hello, World!");
console.log(`${grid.rows}x${grid.cols} symbol`); // e.g. "23x23 symbol"

// Render to SVG
const svg = renderSvg("Hello, World!");
// <svg ...><rect .../></svg>
```

## API

### `encode(data, options?)`

Encode a string or `Uint8Array` to a `ModuleGrid`.

```typescript
encode("Hello, World!");
encode(new Uint8Array([0x41, 0x42, 0x43]));
encode("test", { minEccPercent: 33 });
```

Returns a `ModuleGrid` where `modules[row][col] === true` means a dark module.

### `encodeAndLayout(data, options?, config?)`

Encode and convert to a `PaintScene` in one call.

### `renderSvg(data, options?, config?)`

Encode, layout, and render to SVG in one call.

### `explain(data, options?)`

Returns an `AnnotatedModuleGrid` (v0.1.0: role annotations not yet populated).

### `AztecOptions`

```typescript
interface AztecOptions {
  minEccPercent?: number; // default: 23, range: 10-90
}
```

## Symbol variants

| Variant | Layers | Size range |
|---------|--------|------------|
| Compact | 1-4 | 15x15 to 27x27 |
| Full | 1-32 | 19x19 to 143x143 |

## Dependencies

- `@coding-adventures/barcode-2d` — `ModuleGrid` type and `layout()` function
- `@coding-adventures/reed-solomon` — present in package.json
- `@coding-adventures/gf256` — GF(256) field arithmetic reference
- `@coding-adventures/paint-vm-svg` — SVG rendering backend

## Testing

```bash
npm test
npm run test:coverage
```

68 tests, 100% line coverage, 93% branch coverage.

## License

MIT
