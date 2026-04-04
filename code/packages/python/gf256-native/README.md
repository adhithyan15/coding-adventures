# gf256-native

A Rust-backed Python native extension for GF(2^8) — Galois Field with 256
elements — arithmetic. Wraps the `gf256` Rust crate via `python-bridge`:
zero third-party dependencies, no PyO3.

## What is GF(256)?

GF(2^8) is a finite field where elements are bytes (integers 0–255). The
arithmetic is different from ordinary integer arithmetic:

| Operation | In GF(256) | Example |
|---|---|---|
| Add | XOR | `0x53 ^ 0xCA = 0x99` |
| Subtract | XOR (same as add!) | `0x53 ^ 0xCA = 0x99` |
| Multiply | Log/antilog table lookup | `2 * 128 = 29` (after reduction) |
| Divide | Multiply by inverse | `a / b = a * inverse(b)` |

The multiplication uses reduction modulo the primitive polynomial
`p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285`.

## Why does this matter?

GF(256) is the foundation of:
- **Reed-Solomon error correction** — QR codes, CDs, DVDs, deep-space comms
- **AES encryption** — SubBytes (S-box) and MixColumns steps

## Where does it fit?

```
code/packages/rust/gf256         ← core Rust implementation
code/packages/python/gf256-native  ← this package (Python bindings)
```

## Usage

```python
import gf256_native as gf

# Constants
gf.ZERO                 # 0
gf.ONE                  # 1
gf.PRIMITIVE_POLYNOMIAL # 285 (0x11D)

# Addition is XOR
gf.add(0x53, 0xCA)      # 0x99

# Multiplication via log tables
gf.multiply(2, 128)     # 29

# Division
gf.divide(1, 1)         # 1
gf.divide(4, 2)         # 2

# Exponentiation: 2^255 = 1 (Fermat's little theorem)
gf.power(2, 255)        # 1

# Inverse: a * inverse(a) = 1
gf.inverse(1)           # 1
gf.multiply(3, gf.inverse(3))  # 1

# Division by zero raises ValueError
gf.divide(1, 0)         # ValueError
gf.inverse(0)           # ValueError
```

## Building

```bash
cargo build --release
cp target/release/libgf256_native.dylib src/gf256_native/gf256_native.so
PYTHONPATH=src python -m pytest tests/ -v
```

Or use the `BUILD` file with the repo's build tool.
