# @coding-adventures/fp-arithmetic

IEEE 754 floating-point arithmetic built entirely from logic gates. This is the
shared foundation for both the CPU stack (FPU) and all three accelerator stacks
(GPU/TPU/NPU) in the coding-adventures project.

## Layer position

```
Layer 11: Logic Gates (AND, OR, XOR, NAND)
    |
Layer 10: FP Arithmetic  <-- THIS PACKAGE
    |
    +---> CPU: FPU in cpu-simulator
    +---> GPU: CUDA core (FP32 ALU)
    +---> TPU: Processing element (MAC unit)
    +---> NPU: MAC unit
```

## Supported formats

| Format | Bits | Exponent | Mantissa | Bias | Use case |
|--------|------|----------|----------|------|----------|
| FP32   | 32   | 8        | 23       | 127  | CPU, GPU default |
| FP16   | 16   | 5        | 10       | 15   | GPU mixed precision |
| BF16   | 16   | 8        | 7        | 127  | TPU native, ML training |

## Usage

```typescript
import {
  floatToBits, bitsToFloat, fpAdd, fpMul, fpFma, FP32
} from "@coding-adventures/fp-arithmetic";

// Encode a JavaScript number to IEEE 754 bits
const a = floatToBits(3.14, FP32);
const b = floatToBits(2.71, FP32);

// Add using logic gates
const result = fpAdd(a, b);
console.log(bitsToFloat(result));  // ~5.85

// Multiply
const product = fpMul(a, b);
console.log(bitsToFloat(product));  // ~8.5094

// Fused multiply-add: a * b + c with single rounding
const c = floatToBits(1.0, FP32);
const fmaResult = fpFma(a, b, c);
console.log(bitsToFloat(fmaResult));  // ~9.5094
```

## BigInt for precision

This TypeScript port uses `BigInt` for all mantissa-level bit manipulation.
JavaScript numbers (64-bit doubles) can only represent integers exactly up to
2^53, but FP32 mantissa products can reach 48 bits. BigInt provides the same
arbitrary-precision integer arithmetic that Python has natively.

## Dependencies

None -- this package is fully self-contained. It includes internal
reimplementations of the logic gate primitives (AND, OR, XOR, NOT)
using inline bitwise operations. This makes fp-arithmetic usable
without pulling in transitive dependencies from either the CPU or
accelerator paths.

## Development

```bash
npm install
npx vitest run --coverage
```
