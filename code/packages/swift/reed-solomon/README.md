# reed-solomon

Reed-Solomon error-correcting codes over GF(2^8) for Swift.

Part of the [coding-adventures](https://github.com/adhithyan15/coding-adventures)
educational computing stack — layer MA02.

## What It Does

This library implements Reed-Solomon (RS) encoding and decoding over GF(256),
the finite field with 256 elements. RS codes are a family of block
error-correcting codes invented by Irving Reed and Gustave Solomon in 1960.

RS codes are used everywhere:
- **QR codes**: up to 30% of the symbol can be scratched and still decoded.
- **CDs and DVDs**: CIRC two-level RS corrects scratches and burst errors.
- **Hard drives**: firmware sector-level error correction.
- **Voyager probes**: images sent across 20+ billion kilometres.
- **RAID-6**: the two parity drives ARE an (n, n-2) RS code over GF(256).

## Error-Correction Capacity

An RS code with `nCheck` check bytes can correct up to `t = nCheck / 2` byte
errors in unknown positions. For example:

| `nCheck` | `t` (errors correctable) |
|----------|--------------------------|
| 2        | 1                        |
| 4        | 2                        |
| 6        | 3                        |
| 8        | 4                        |

## API

All functions live in the `ReedSolomon` enum namespace:

```swift
import ReedSolomon

// Encode a message with 4 check bytes (corrects up to 2 errors)
let message: [UInt8] = [72, 101, 108, 108, 111]   // "Hello"
let codeword = try ReedSolomon.encode(message, nCheck: 4)
// codeword = [72, 101, 108, 108, 111, c0, c1, c2, c3]
//             ← message bytes →        ← check bytes →

// Simulate a transmission error
var corrupted = codeword
corrupted[0] ^= 0xFF   // flip all bits in first byte

// Decode — corrects the error automatically
let recovered = try ReedSolomon.decode(corrupted, nCheck: 4)
// recovered == [72, 101, 108, 108, 111]  ✓

// Inspect syndromes (0 = no errors)
let synds = ReedSolomon.syndromes(codeword, nCheck: 4)
// synds == [0, 0, 0, 0]

// Build the generator polynomial
let gen = ReedSolomon.buildGenerator(4)
// gen = LE coefficient array of degree-4 polynomial

// Compute error locator polynomial (Berlekamp-Massey)
let locator = ReedSolomon.errorLocator(synds)
// locator = [1] (no errors)
```

## Error Types

```swift
do {
    let decoded = try ReedSolomon.decode(data, nCheck: 4)
} catch let e as ReedSolomon.TooManyErrors {
    // More than t errors — codeword is unrecoverable
} catch let e as ReedSolomon.InvalidInput {
    print("Bad parameters: \(e.reason)")
}
```

## Polynomial Conventions

Two conventions coexist in this implementation:

- **Big-endian (BE)**: `codeword[0]` is the highest-degree coefficient.
  Used for codewords and syndrome evaluation.
- **Little-endian (LE)**: `poly[i]` is the coefficient of x^i.
  Used internally for generator, locator, and error evaluator polynomials.

## How It Works

### Encoding (3 steps)

1. Build the generator polynomial `g(x) = (x+α)(x+α²)…(x+α^{nCheck})`.
2. Form the shifted message `M(x)·x^{nCheck}` (append `nCheck` zero bytes).
3. Compute check bytes = `M(x)·x^{nCheck}` mod `g(x)`.

The codeword `C(x) = M(x)·x^{nCheck} XOR R(x)` is divisible by `g(x)`, so
`C(α^i) = 0` for `i = 1…nCheck`. This is the property the decoder exploits.

### Decoding (5 steps)

1. **Syndromes**: `S_j = received(α^j)` — all zero means no errors.
2. **Berlekamp-Massey**: find the error locator polynomial `Λ(x)`.
3. **Chien search**: find positions `p` where `Λ(α^{-(n-1-p)}) = 0`.
4. **Forney algorithm**: compute error magnitudes from `Ω(x) = S(x)·Λ(x)`.
5. **Apply corrections**: `received[p] ^= magnitude[p]`.

## Where It Fits

```
MA00 polynomial    (polynomial arithmetic over Double)
      ↓
MA01 gf256         (GF(2^8) field arithmetic: add=XOR, mul=table lookup)
      ↓
MA02 reed-solomon  ← you are here
      ↓
QR codes, CD/DVD, hard drives, RAID-6, Voyager probes
```

## Running Tests

```bash
swift test
```

## Cross-Language Test Vector

`buildGenerator(2)` must return `[8, 6, 1]` in all language implementations.
This is verified by the test `testBuildGenerator2CrossLanguageVector`.

## License

MIT
