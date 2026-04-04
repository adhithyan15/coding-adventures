# @coding-adventures/gf256-native

Native Node.js addon wrapping the Rust [`gf256`](../../rust/gf256/) crate via `node-bridge` N-API FFI.

## What it does

This package exposes arithmetic in **GF(2^8)** — the Galois Field with 256 elements — to JavaScript.

GF(2^8) is used in:
- **Reed-Solomon error correction** (QR codes, CDs, DVDs, hard drives)
- **AES encryption** (SubBytes and MixColumns operations)

Elements are integers in [0, 255] (one byte). The arithmetic is NOT ordinary integer arithmetic:
- Addition is XOR (so `a + a = 0` for all `a`)
- Multiplication reduces modulo the primitive polynomial `x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285`

## How it fits in the stack

```
gf256 (Rust crate, MA01)
    ↓  wrapped by
gf256-native (this package)
    ↓  used by
reed-solomon-native (future), qr-code (future)
```

## Installation

This package is not published to npm. Install from source within this monorepo:

```bash
cargo build --release --manifest-path Cargo.toml
cp target/release/libgf256_native_node.dylib gf256_native_node.node  # macOS
npm ci
```

## Usage

```typescript
import {
  ZERO, ONE, PRIMITIVE_POLYNOMIAL,
  add, subtract, multiply, divide, power, inverse,
} from "@coding-adventures/gf256-native";

console.log(PRIMITIVE_POLYNOMIAL); // 285  (= 0x11D)

// Addition is XOR
console.log(add(0x53, 0xCA));  // 0x99 = 153

// Every element is its own additive inverse
console.log(add(42, 42));  // 0

// Multiplication uses log/antilog tables
console.log(multiply(2, 128)); // 29  (256 XOR 285 = 29, first reduction)

// Group order is 255: 2^255 = 1
console.log(power(2, 255));  // 1

// Multiplicative inverse
const a = 42;
console.log(multiply(a, inverse(a)));  // 1

// Division by zero throws
try {
  divide(5, 0);
} catch (e) {
  console.error(e.message); // "GF256: division by zero"
}
```

## API

### Constants

| Name | Value | Description |
|---|---|---|
| `ZERO` | `0` | Additive identity |
| `ONE` | `1` | Multiplicative identity |
| `PRIMITIVE_POLYNOMIAL` | `285` | Irreducible polynomial 0x11D |

### Functions

| Function | Signature | Description |
|---|---|---|
| `add` | `(a: number, b: number) => number` | XOR |
| `subtract` | `(a: number, b: number) => number` | XOR (same as add in GF(2)) |
| `multiply` | `(a: number, b: number) => number` | Log/antilog table multiplication |
| `divide` | `(a: number, b: number) => number` | Throws if `b == 0` |
| `power` | `(base: number, exp: number) => number` | Exponentiation, `exp` is `u32` |
| `inverse` | `(a: number) => number` | Multiplicative inverse, throws if `a == 0` |

## Testing

```bash
npx vitest run
```

All 35+ tests cover every function and edge case including:
- XOR properties of add/subtract
- Zero element behavior
- GF(256) multiplication table spot-checks
- Fermat's little theorem: `power(2, 255) = 1`
- `divide(a, a) = 1` and `multiply(a, inverse(a)) = 1` for non-zero `a`
- Error throwing on zero inputs to `divide` and `inverse`
