# @coding-adventures/test-matchers

Zero-dependency TypeScript Level 1 test matchers and pure deep-equality assertions.

## Features

- **Zero Runtime Dependencies**: Standard library only (`"dependencies": {}`).
- **Pure Deep Equality**: Bit-exact IEEE-754 float comparison (preserving `-0 !== +0` and canonicalizing NaN payloads), arbitrary-precision integers via native `BigInt`, byte sequences (`Uint8Array`), strings, arrays, objects, sets, and maps.
- **Ergonomic Matchers**: Familiar `expect()` syntax (`toBe`, `toEqual`, `toStrictEqual`, `toBeTruthy`, `toBeFalsy`, `toBeNull`, `toBeUndefined`, `toBeDefined`, `toBeInstanceOf`, `toBeNaN`, `toBeGreaterThan`, `toBeGreaterThanOrEqual`, `toBeLessThan`, `toBeLessThanOrEqual`, `toBeCloseTo`, `toContain`, `toHaveLength`, `toMatch`, `toHaveProperty`, `toThrow`).
- **Inverted Assertions**: Complete `.not` support across all matchers.
- **Structured Errors**: Clear `AssertionError` with `actual` and `expected` fields.

## Installation

```bash
npm install @coding-adventures/test-matchers
```

## Quick Start

```typescript
import { expect } from "@coding-adventures/test-matchers";

// Primitive & reference identity
expect(42).toBe(42);
expect("hello").not.toBe("world");

// Deep equality
expect({ a: 1, b: [2, 3] }).toEqual({ a: 1, b: [2, 3] });
expect(new Uint8Array([1, 2, 3])).toEqual(new Uint8Array([1, 2, 3]));

// Collections & lengths
expect([1, 2, 3]).toContain(2);
expect("foobar").toHaveLength(6);

// Exceptions
expect(() => {
  throw new Error("unauthorized");
}).toThrow("unauthorized");
```
