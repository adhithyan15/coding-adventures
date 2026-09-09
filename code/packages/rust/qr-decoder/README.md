# qr-decoder (Rust)

QR Code **decoder** — the inverse of [`qr-code`](../qr-code)'s encoder.
Takes an already-located, already-sampled [`ModuleGrid`](../barcode-2d)
(exactly what `qr_code::encode` produces — mask already applied,
format/version info already written, no quiet zone) and recovers the
original bytes.

Closes the "decode a QR matrix back to a string" half of the roadmap slot
`MA04` that `MA02-reed-solomon.md` originally reserved, per
[`code/specs/MA04-qr-decoder.md`](../../specs/MA04-qr-decoder.md).

## Scope: `ModuleGrid → Vec<u8>` only

```text
                        (deferred: locate a QR code
                         within an arbitrary raster image —
                         binarization, finder-pattern scanning,
                         perspective correction — separate,
                         larger, open-ended follow-up work)
                                    |
                                    v
       ModuleGrid  ──────>  qr_decoder::decode  ──────>  Vec<u8>
```

This crate does **not** find a QR code inside a photograph or screenshot.
It starts one step later than a real-world scanner would: someone has
already handed it a clean module grid. Verification is by round-tripping
against this workspace's own `qr-code` encoder, not against a real
photograph.

## Pipeline

```text
ModuleGrid
  → validate size (rows == cols == 4V+17 for some V in 1..=40)
  → qr_code::reserved_modules(V)               (function-pattern map)
  → read format info (Copy 1, then Copy 2)      → (ecc_level, mask)
  → read version info if V>=7 (Copy 1, then 2)  → must match V from size
  → unmask every non-reserved module            (XOR, self-inverse)
  → walk qr_code::data_module_order(V)          → raw bit stream
  → pack into codewords, discard remainder bits
  → de-interleave into per-block (data, ecc)     (mirrors compute_blocks)
  → reed_solomon::decode_with_base(_, ecc_len, 0) per block
  → concatenate corrected data codewords in block order
  → parse one data segment (numeric/alphanumeric/byte)
  → Vec<u8>
```

Every step reuses `qr-code`'s own tables, geometry, and traversal (exposed
as new `pub` functions alongside this crate) rather than reimplementing
them — encode and decode agree by construction, not because two
independently-written implementations happen to match.

## Usage

```rust
use qr_code::{encode, EccLevel};
use qr_decoder::decode;

let grid = encode("https://example.com", EccLevel::Q).unwrap();
let recovered = decode(&grid).unwrap();
assert_eq!(recovered, b"https://example.com");
```

## API

### `decode(grid: &ModuleGrid) -> Result<Vec<u8>, QrDecodeError>`

The only public function.

### `QrDecodeError`

Six variants, each structural (no decoded content in any of them — this
crate has no knowledge that a caller might feed it secret-shaped data such
as an `otpauth://` TOTP seed):

| Variant | Meaning |
|---|---|
| `InvalidSize` | Not square, or size isn't `4V+17` for any `V` in `1..=40` |
| `FormatInfoUnreadable` | Both format-info copies failed BCH validation |
| `VersionInfoUnreadable` | V>=7 and both version-info copies failed BCH, or disagreed with the size-derived version |
| `UnrecoverableBlock(usize)` | Reed-Solomon exhausted its correction capacity for the block at this index |
| `UnsupportedMode(u8)` | Kanji (`0b1000`) or ECI (`0b0111`) mode indicator — `qr-code`'s encoder never produces these |
| `Truncated` | The bit stream ran out before a character count, mode payload, or codeword read could complete |

## Data modes supported

Exactly what `qr-code`'s own encoder can produce — numeric, alphanumeric,
and byte mode, never more than one segment per message (no mixed-mode
packing, no ECI header). Byte mode is the one the actual driving use case
exercises: an `otpauth://totp/...` URI contains lowercase letters and
`?`/`=`/`&`, none of which are in the 45-character alphanumeric set, so
`qr-code`'s own mode-selection heuristic always encodes it in byte mode.

## Robustness

`ModuleGrid` is a public, hand-constructible type (a caller could build one
with garbage content, not only via `qr_code::encode`), so every module
index, bit-stream read, and character-count check is bounds-checked —
`#![forbid(unsafe_code)]`, and no `.unwrap()`/`.expect()` is reachable on a
malformed-but-correctly-sized grid.

Format info and version info each get a **two-copy fallback** (try Copy 1,
fall back to Copy 2 on BCH failure) but not full nearest-codeword error
correction — see "Explicitly out of scope" below.

## Where this fits in the stack

```text
code/packages/rust/
  gf256            — GF(256) field arithmetic
  reed-solomon     — RS encode/decode over GF(256), now base-parameterized
  barcode-2d       — ModuleGrid, the shared 2D-barcode intermediate representation
  qr-code          — QR Code encoder (ModuleGrid producer)
  qr-decoder  <───  THIS CRATE — QR Code decoder (ModuleGrid consumer)
```

Depended on by: nothing yet in this workspace. The eventual consumer is
the image-locating follow-up work that will turn `qr_decoder::decode(&grid)`
into a real `vault-pm import otpauth-qr FILE` command.

## Explicitly out of scope

- **Locating a QR code within an arbitrary raster image** — pixel
  binarization beyond a fixed threshold, finder-pattern run-length
  scanning, candidate clustering, module-grid sampling, perspective/
  rotation/lighting robustness for photographed (not just screenshotted)
  codes. Tracked in its own issue.
- **Full format-info/version-info error correction** — nearest-valid-
  codeword search by Hamming distance (could correct up to 3 bit errors
  even when *both* copies are damaged). This crate implements only the
  simpler two-copy fallback ISO 18004 already provides.
- **Kanji mode, ECI, structured append, mixed-mode segments** — none of
  these exist in the paired `qr-code` encoder either; adding decode
  support without encoder support would be asymmetric scope with no
  round-trip test-fixture story.
- **Micro QR, rMQR** — tracks whatever `qr-code` eventually supports.

## Testing

30 tests (28 unit + 2 doctests), including an exhaustive round trip across
all 40 versions x all 4 ECC levels in byte mode (160 encode-then-decode
cases), numeric/alphanumeric sweeps across the version range, corruption-
and-recovery within RS capacity (proving `reed_solomon::decode_with_base`'s
b=0 wiring is genuinely correct, not just "compiles and round-trips on
undamaged input"), beyond-capacity rejection, format/version-info fallback
and failure paths, and dedicated coverage for every `QrDecodeError` variant.

```sh
cargo test -p qr-decoder -- --nocapture
```
