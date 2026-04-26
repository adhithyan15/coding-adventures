# PDF417 (Swift)

PDF417 stacked linear barcode encoder for Swift — ISO/IEC 15438:2015
compliant.

PDF417 (Portable Data File 417) was invented by Ynjiun P. Wang at Symbol
Technologies in 1991. The "417" in the name encodes its geometry: every
codeword has exactly **4** bars and **4** spaces (8 elements) and occupies
exactly **17** modules of horizontal space.

## Where PDF417 is used

| Application      | Detail                                                |
|------------------|-------------------------------------------------------|
| AAMVA            | North American driver's licences and government IDs   |
| IATA BCBP        | Airline boarding passes                               |
| USPS             | Domestic shipping labels                              |
| US immigration   | Form I-94, customs declarations                       |
| Healthcare       | Patient wristbands, medication labels                 |

## How it fits in the stack

```text
input string / bytes
  → PDF417 (this package)
  → ModuleGrid    — abstract boolean grid     (Barcode2D)
  → layout()      — pixel coordinates         (Barcode2D)
  → PaintScene    — render instructions       (PaintInstructions)
  → paint backend — PNG, Metal, SVG, terminal …
```

Every layer above the PaintScene step is pixel-agnostic; the same encoder
produces the same `ModuleGrid` whether you eventually render to a PNG or to
a Metal texture.

## Usage

```swift
import PDF417

// 1. Encode a string with auto-selected ECC and dimensions.
let grid = try encode("Hello, PDF417!")
print("\(grid.rows) × \(grid.cols) modules")

// 2. Encode with explicit options.
let g2 = try encode(
    "Hello, PDF417!",
    options: PDF417Options(
        eccLevel: 4,    // higher redundancy
        columns: 6,     // 6 data columns
        rowHeight: 4    // 4 module-rows per logical row
    )
)

// 3. Encode raw bytes (use this for non-UTF-8 binary data).
let bytes: [UInt8] = [0xDE, 0xAD, 0xBE, 0xEF]
let g3 = try encode(bytes: bytes)

// 4. Produce a PaintScene ready for any paint backend.
let scene = try encodeAndLayout("Hello, PDF417!")
print("\(scene.width) × \(scene.height) px")
```

## Encoding pipeline

1. **Byte compaction** — codeword 924 latch. 6 bytes are packed into 5
   base-900 codewords (treating them as a 48-bit integer); remaining
   1–5 bytes are encoded directly (1 codeword each).
2. **Length descriptor** — first codeword counts the symbol's total
   non-padding codewords (length descriptor + data + ECC).
3. **Reed-Solomon ECC** — GF(929) with the b=3 convention; roots
   `α^3, α^4, …, α^{k+2}` for `k = 2^(eccLevel+1)` ECC codewords.
   Auto-level selection: level 2 for ≤ 40 data codewords, up to level 6.
4. **Dimension selection** — auto: `c = ⌈√(total / 3)⌉`,
   `r = ⌈total / c⌉`, clamped to 3–90 rows × 1–30 columns.
5. **Padding** — codeword 900 fills any remaining slots after the data
   stream but BEFORE the ECC codewords.
6. **Row indicators** — each row carries Left and Right Row Indicator
   codewords encoding R, C, and ECC level.
7. **Cluster table lookup** — each codeword (data or indicator) maps to a
   17-module bar/space pattern via one of three cluster tables, picked by
   `row % 3`.
8. **Start/stop patterns** — fixed 17-module start and 18-module stop
   patterns bracket every row.

The final symbol width is `69 + 17 × cols` modules; height is
`rows × rowHeight` modules.

## API

| Symbol               | Purpose                                          |
|----------------------|--------------------------------------------------|
| `encode(_:options:)` | Encode a `String` (UTF-8 bytes) → `ModuleGrid`.  |
| `encode(bytes:options:)` | Encode raw `[UInt8]` → `ModuleGrid`.         |
| `encodeAndLayout(_:options:config:)` | One-shot encode + layout → `PaintScene`. |
| `PDF417Options`      | ECC level, columns, rowHeight overrides.         |
| `PDF417Error`        | Validation errors thrown by `encode()`.          |

## Errors

`encode()` throws `PDF417Error` cases for:

- `.invalidECCLevel(String)` — when `eccLevel` is outside 0–8.
- `.invalidDimensions(String)` — when `columns` is outside 1–30.
- `.inputTooLong(String)` — when the data does not fit in any valid symbol.
- `.layoutError(String)` — when `Barcode2D.layout()` rejects the config
  (only thrown by `encodeAndLayout`).

## Compaction modes

PDF417 supports three compaction modes — text, byte, and numeric. This
v0.1.0 release implements **byte compaction only**, matching the
TypeScript reference. Byte compaction handles ALL input (including ASCII
text) at the cost of being less compact than the specialized modes
(roughly 1.2 codewords per byte). Text and numeric modes are planned for
v0.2.0.

## Testing

```bash
cd code/packages/swift/pdf417
mise exec -- swift test
```

The test suite covers:

- Cluster tables (3 × 929 entries, all decode to 17 modules).
- GF(929) arithmetic (gfAdd, gfMul, multiplicative inverses, identity).
- Byte compaction (empty, single byte, 6-byte group, 12-byte two groups).
- Row indicator computation (LRI / RRI for all 3 clusters).
- Auto-ECC selection at every threshold (40, 160, 320, 863).
- Module-width formula `cols = 69 + 17·c`.
- Start/stop patterns appear in every row.
- Determinism (same input → identical grid).
- Higher ECC level produces equal-or-larger symbol.
- Error cases for invalid ECC level, columns, oversized input.
- `encodeAndLayout` returns a valid `PaintScene`.

## License

MIT — same as the rest of the coding-adventures monorepo.
