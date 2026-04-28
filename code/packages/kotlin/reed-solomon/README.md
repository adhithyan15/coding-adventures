# reed-solomon — Kotlin

Reed-Solomon error-correcting codes over GF(256).

## What is Reed-Solomon?

Reed-Solomon (RS) is a block error-correcting code. You add `nCheck`
redundancy bytes to a message; the decoder can recover the original data
even if up to `t = nCheck / 2` bytes are corrupted anywhere in the codeword.

## Where RS is used

| System           | How RS helps                                        |
|------------------|-----------------------------------------------------|
| QR codes         | Up to 30% of a QR symbol can be destroyed/scratched |
| CDs / DVDs       | CIRC two-level RS corrects scratches and defects    |
| Hard drives      | Firmware sector-level error correction              |
| Voyager probes   | Transmit images across 20+ billion km               |
| RAID-6           | Two parity drives = (n, n-2) RS code                |

## Building blocks

```
gf256        — GF(2^8) field arithmetic
polynomial   — GF(256) polynomial arithmetic
reed-solomon ← this package
```

## Usage

```kotlin
import com.codingadventures.reedsolomon.*

val message = intArrayOf(72, 101, 108, 108, 111)  // "Hello"
val nCheck = 8  // t = 4 errors correctable

val codeword = encode(message, nCheck)
// codeword[0..4] == message (systematic)

// Corrupt up to t=4 bytes — still recoverable
val corrupted = codeword.copyOf()
corrupted[0] = corrupted[0] xor 0xFF
corrupted[2] = corrupted[2] xor 0xAA

val recovered = decode(corrupted, nCheck)
// recovered deep-equals message
```

## API

| Function | Description |
|----------|-------------|
| `buildGenerator(nCheck)` | Build the RS generator polynomial `g(x)` |
| `encode(message, nCheck)` | Systematic encoding — appends `nCheck` parity bytes |
| `decode(received, nCheck)` | Decode and correct up to `t = nCheck/2` errors |
| `syndromes(received, nCheck)` | Compute the `nCheck` syndrome values |
| `errorLocator(synds)` | Run Berlekamp-Massey to find the error locator polynomial |

### Exceptions

| Class | When thrown |
|-------|-------------|
| `InvalidInputException` | Bad parameters (nCheck=0/odd, length>255, etc.) |
| `TooManyErrorsException` | More than `t` errors — codeword is unrecoverable |

## Decoding pipeline

```
received
  ↓ Step 1: syndromes — all zero? return message unchanged
  ↓ Step 2: Berlekamp-Massey → error locator polynomial Λ(x)
  ↓ Step 3: Chien search → error positions {p₁…pᵥ}
  ↓ Step 4: Forney algorithm → error magnitudes {e₁…eᵥ}
  ↓ Step 5: Apply corrections: codeword[pₖ] ^= eₖ
  ↓ Return first k bytes
```

## Dependencies

- `gf256` — GF(2^8) field operations
- `polynomial` — GF(256) polynomial arithmetic

Both are resolved as local Gradle composite builds.

## Build

```
gradle test
```

## Spec

`code/specs/MA02-reed-solomon.md`
