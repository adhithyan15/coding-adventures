# reed-solomon (Haskell)

Reed-Solomon error-correcting codes over GF(256).

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
math foundation, implementing **MA02** from the spec series.

## What Is Reed-Solomon?

Reed-Solomon is a block error-correcting code: given a message of `k` bytes,
the encoder appends `nCheck` redundancy bytes. A decoder can then recover
the original message even if up to `t = nCheck / 2` bytes are corrupted.

| System | How RS Helps |
|--------|-------------|
| QR codes | Up to 30% of a QR symbol can be scratched and still decode |
| CDs / DVDs | CIRC two-level RS corrects scratches |
| Hard drives | Sector-level error correction |
| Voyager probes | Images across billions of kilometres |
| RAID-6 | Two parity drives = an `(n, n-2)` RS code |

## Building Blocks

```
MA00  polynomial   — GF(256) polynomial arithmetic
MA01  gf256        — GF(2^8) field arithmetic
MA02  reed-solomon — THIS PACKAGE
```

## Usage

```haskell
import ReedSolomon

let msg    = [72, 101, 108, 108, 111]  -- "Hello"
    nCheck = 8                          -- t = 4 errors correctable

-- Encode: produces systematic codeword
let Right codeword = encode msg nCheck
-- codeword[0..4] == msg (systematic)

-- Corrupt 4 bytes — still recoverable
let corrupted = flip corruptAt codeword <$> zip [0,2,4,6] [0xFF,0xAA,0xBB,0xCC]

-- Decode
let Right recovered = decode corrupted nCheck
-- recovered == msg
```

## API

| Function | Description |
|----------|-------------|
| `encode msg nCheck` | Produce systematic codeword `msg ++ checkBytes` |
| `decode received nCheck` | Recover message, correcting up to `t` errors |
| `syndromes received nCheck` | Compute syndrome bytes |
| `errorLocator synds` | Berlekamp-Massey error locator polynomial |
| `buildGenerator nCheck` | Build the generator polynomial |

### Errors

```haskell
data RSError
    = TooManyErrors         -- > t = nCheck/2 errors; unrecoverable
    | InvalidInput String   -- bad parameters
```

## Decoding Pipeline

```
received
  │
  ▼ Step 1: Syndromes — all zero? return message.
  ▼ Step 2: Berlekamp-Massey → Λ(x), error count L
  ▼ Step 3: Chien search → error positions
  ▼ Step 4: Forney algorithm → error magnitudes
  ▼ Step 5: XOR-correct + strip check bytes
```

## Package Structure

```
reed-solomon/
├── src/
│   └── ReedSolomon.hs        — implementation
├── test/
│   ├── Spec.hs                — test entry point
│   └── ReedSolomonSpec.hs     — Hspec tests
├── reed-solomon.cabal
├── BUILD
└── README.md
```

## Building and Testing

```bash
cabal test
```

## Dependencies

- **`gf256`**: All GF(256) coefficient arithmetic.
- **No `polynomial` dependency**: RS internally uses its own GF(256)
  polynomial helpers for performance and clarity.

## Spec

See [`code/specs/MA02-reed-solomon.md`](../../../specs/MA02-reed-solomon.md)
for the full specification including algorithm details, test vectors, and
connection to QR codes.
