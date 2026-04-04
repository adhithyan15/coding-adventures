# @coding-adventures/polynomial-native

Native Node.js addon wrapping the Rust [`polynomial`](../../rust/polynomial/) crate via `node-bridge` N-API FFI.

## What it does

This package exposes polynomial arithmetic over `f64` coefficients to JavaScript. Polynomials are represented as plain `number[]` arrays in **little-endian** order — index 0 is the constant term (coefficient of x⁰), index 1 is the coefficient of x¹, etc.

```
[3.0, 0.0, 2.0]  →  3 + 0·x + 2·x²  =  3 + 2x²
[1.0, 2.0, 3.0]  →  1 + 2x + 3x²
[]               →  the zero polynomial
```

## How it fits in the stack

```
polynomial (Rust crate, MA00)
    ↓  wrapped by
polynomial-native (this package)
    ↓  used by
gf256-native, reed-solomon-native (future)
```

The Rust crate does all the computation. The native addon is a thin N-API shim that:
1. Converts JS `number[]` arrays to Rust `Vec<f64>` slices.
2. Calls the Rust function.
3. Converts the result back to a JS `number[]`.

No JavaScript logic lives here — it is pure glue code.

## Installation

This package is not published to npm. Install from source within this monorepo:

```bash
cargo build --release --manifest-path Cargo.toml
cp target/release/libpolynomial_native_node.dylib polynomial_native_node.node  # macOS
npm ci
```

Or use the repo's build tool which handles compilation and copying automatically.

## Usage

```typescript
import {
  normalize, degree, zero, one,
  add, subtract, multiply,
  divmodPoly, divide, modulo,
  evaluate, gcd,
} from "@coding-adventures/polynomial-native";

// Basic arithmetic
const a = [1.0, 2.0, 3.0]; // 1 + 2x + 3x²
const b = [4.0, 5.0];       // 4 + 5x

console.log(add(a, b));       // [5, 7, 3]  →  5 + 7x + 3x²
console.log(multiply(a, b));  // convolution result

// Evaluation using Horner's method
console.log(evaluate([3.0, 0.0, 1.0], 2.0)); // 7  →  3 + 0·2 + 1·4

// Division (throws if divisor is zero)
const [q, r] = divmodPoly([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]);
console.log(q); // quotient
console.log(r); // remainder

// GCD
const g = gcd([2.0, -3.0, 1.0], [-1.0, 1.0]);
console.log(g); // common factor
```

## API

| Function | Signature | Description |
|---|---|---|
| `normalize` | `(poly: number[]) => number[]` | Strip trailing near-zero coefficients |
| `degree` | `(poly: number[]) => number` | Index of highest non-zero coefficient |
| `zero` | `() => number[]` | Additive identity `[0.0]` |
| `one` | `() => number[]` | Multiplicative identity `[1.0]` |
| `add` | `(a, b: number[]) => number[]` | Term-by-term addition |
| `subtract` | `(a, b: number[]) => number[]` | Term-by-term subtraction |
| `multiply` | `(a, b: number[]) => number[]` | Convolution (degree m+n result) |
| `divmodPoly` | `(dividend, divisor: number[]) => [number[], number[]]` | Long division → `[quotient, remainder]` |
| `divide` | `(a, b: number[]) => number[]` | Quotient only |
| `modulo` | `(a, b: number[]) => number[]` | Remainder only |
| `evaluate` | `(poly: number[], x: number) => number` | Horner's method at point x |
| `gcd` | `(a, b: number[]) => number[]` | Euclidean GCD |

`divmodPoly`, `divide`, and `modulo` throw a JS `Error` if the divisor is the zero polynomial.

## Testing

```bash
npx vitest run
```

All 35+ tests cover every function including edge cases (zero polynomial, division errors, GCD).

## Build

```bash
cargo build --release --manifest-path Cargo.toml
```

The compiled `.node` file is a platform-specific dynamic library loaded by Node.js via N-API.
