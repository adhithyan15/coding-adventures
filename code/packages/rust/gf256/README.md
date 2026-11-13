# gf256

Galois Field GF(2^8) arithmetic — the math behind Reed-Solomon error correction.

## Overview

GF(2^8) is a finite field with 256 elements (the integers 0–255). It is used by:

- **Reed-Solomon error correction** — QR codes, CDs, DVDs, hard drives
- **AES encryption** — SubBytes and MixColumns steps (with a *different*
  polynomial, see `Field`)
- **CRCs and checksums** — polynomial division over GF(2)

The key insight: in GF(2^8), **addition IS XOR** and **subtraction IS addition**.
No carry. No overflow. Just XOR.

Multiplication uses precomputed log/antilog tables (initialized once via
`std::sync::OnceLock`) to achieve O(1) performance.

## Primitive Polynomial

```text
p(x) = x^8 + x^4 + x^3 + x^2 + 1   =   0x11D   =   285
```

This is the standard primitive polynomial for most Reed-Solomon implementations.
Note: AES uses 0x11B — a different polynomial. Under 0x11D the inverse pair is
`0x53 * 0x8C = 1`; the famous AES pair `0x53 * 0xCA = 1` holds only under 0x11B
(use `Field::new(0x11B)` for that).

## Where This Fits

```
MA00  polynomial      — polynomial arithmetic over f64
MA01  gf256           ← this crate
MA02  reed-solomon    — Reed-Solomon error correction
```

## Usage

```rust
use gf256::{add, subtract, multiply, divide, power, inverse, ZERO, ONE};

// Addition is XOR
assert_eq!(add(0x53, 0xca), 0x99);

// 0x53 and 0x8C are multiplicative inverses under 0x11D
assert_eq!(multiply(0x53, 0x8c), 1);
assert_eq!(inverse(0x53), 0x8c);

// Generator 2 has order 255
assert_eq!(power(2, 255), 1);

// 2^8 = 0x1D (first reduction step)
assert_eq!(power(2, 8), 0x1d);

// divide
assert_eq!(divide(multiply(3, 5), 5), 3);
```

## API

| Function | Signature | Description |
|----------|-----------|-------------|
| `add(a, b)` | `(u8, u8) → u8` | XOR |
| `subtract(a, b)` | `(u8, u8) → u8` | XOR (same as add) |
| `multiply(a, b)` | `(u8, u8) → u8` | log-table multiplication |
| `divide(a, b)` | `(u8, u8) → u8` | division; **panics** if b = 0 |
| `power(base, exp)` | `(u8, u32) → u8` | exponentiation; `0^0 = 1`; total |
| `inverse(a)` | `(u8) → u8` | multiplicative inverse; **panics** if a = 0 |

Constants: `ZERO = 0`, `ONE = 1`, `PRIMITIVE_POLYNOMIAL = 0x11D`.

`Field::new(poly)` builds the same operations for another polynomial (for
example AES `0x11B`) with table-free shift-and-XOR arithmetic. It does **not**
validate `poly`: a polynomial that is not a degree-8 irreducible silently gives
arithmetic that is not a field.

There are no `Result`s: dividing by zero and inverting zero panic.

## Constant time

**Not constant time.** `add`/`subtract` are one XOR; `multiply`, `divide`,
`power` and `inverse` branch on zero and index lookup tables with
operand-derived values, and `Field` multiplication branches on operand bits.
Do not use them on secret data where cache-timing is in the threat model (the
CWE-208 row of the assurance report is *applicable and unaddressed*).

## Verification

The public operations are specified by `code/specs/gf256.warrant` (a Warrant
document: known vectors, field laws, and independent bit-level models of XOR and
shift-and-add multiplication). `tests/generated/warrant_evidence.rs` is
*generated* from it (never edit it; `BUILD` fails if it drifts) and run against
the real exports: examples, bounded-exhaustive suites over all 65,536 byte pairs
and all 2^24 byte triples, property suites, fuzz suites, and sabotage controls
that inject plausible faults (wrong reduction polynomial, off-by-one antilog
index, multiplication returning the sum, OR for XOR, exponent mod 256, ...) and
must fail. `evidence/test-manifest.json` is written by that run;
`assurance_report.json` is derived from it. Constant-time and step-bound proofs
are **not live**: the WIR front end cannot lower this source
(`evidence/proof-manifest.json` records the blocker).

## Runtime capability conformance

The library is pure (`required_capabilities.json` declares no capabilities).
`cargo test -p gf256 --test runtime_capability` proves it at run time on Linux:
every public operation, including `Field` for both the Reed-Solomon (0x11D) and
AES (0x11B) polynomials and the first construction of the lookup tables, runs
inside a seccomp/Landlock sandbox derived from that manifest while an independent
`strace` observer confirms nothing but the baseline syscalls happen (VER08b).
`BUILD_linux` sets `RCG_STRICT=1` so an unavailable sandbox fails the build
instead of reporting NOT-LIVE.

## Spec

See `code/specs/MA01-gf256.md` in the coding-adventures monorepo, and
`code/specs/gf256.warrant`.
