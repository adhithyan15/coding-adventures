// gf256_native.test.ts -- Comprehensive tests for the native GF(2^8) addon
// ==========================================================================
//
// These tests verify that the Rust GF(256) implementation is correctly
// exposed to JavaScript via the N-API node-bridge.
//
// ## GF(2^8) properties that drive test cases
//
// 1. Addition is XOR: add(a, b) = a XOR b
// 2. Subtraction equals addition: subtract(a, b) = add(a, b)
// 3. Every element is its own additive inverse: add(a, a) = 0
// 4. Multiplication by 0 gives 0
// 5. Multiplication by 1 is identity
// 6. Division by self gives 1 (for non-zero)
// 7. inverse(a) * a = 1
// 8. power(2, 255) = 1 (group order = 255)
// 9. PRIMITIVE_POLYNOMIAL = 285 = 0x11D
//
// We use the generator g=2 to derive expected values. The ALOG table
// starts: ALOG[0]=1, ALOG[1]=2, ALOG[7]=128, ALOG[8]=29, ALOG[9]=58.

import { describe, it, expect } from "vitest";
import {
  ZERO,
  ONE,
  PRIMITIVE_POLYNOMIAL,
  add,
  subtract,
  multiply,
  divide,
  power,
  inverse,
} from "../index.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

describe("constants", () => {
  it("ZERO equals 0", () => {
    expect(ZERO).toBe(0);
  });

  it("ONE equals 1", () => {
    expect(ONE).toBe(1);
  });

  it("PRIMITIVE_POLYNOMIAL equals 285 (0x11D)", () => {
    expect(PRIMITIVE_POLYNOMIAL).toBe(285);
  });
});

// ---------------------------------------------------------------------------
// add
// ---------------------------------------------------------------------------

describe("add", () => {
  it("is XOR: add(0x53, 0xCA) = 0x99 = 153", () => {
    expect(add(0x53, 0xCA)).toBe(0x99);
  });

  it("every element is its own inverse: add(x, x) = 0", () => {
    expect(add(5, 5)).toBe(0);
    expect(add(255, 255)).toBe(0);
    expect(add(1, 1)).toBe(0);
    expect(add(0, 0)).toBe(0);
  });

  it("add(x, 0) = x (ZERO is additive identity)", () => {
    expect(add(42, ZERO)).toBe(42);
    expect(add(ZERO, 42)).toBe(42);
    expect(add(0, 0)).toBe(0);
  });

  it("is commutative: add(a, b) = add(b, a)", () => {
    expect(add(17, 83)).toBe(add(83, 17));
    expect(add(0, 200)).toBe(add(200, 0));
  });

  it("is associative: add(a, add(b, c)) = add(add(a, b), c)", () => {
    const a = 10, b = 20, c = 30;
    expect(add(a, add(b, c))).toBe(add(add(a, b), c));
  });
});

// ---------------------------------------------------------------------------
// subtract
// ---------------------------------------------------------------------------

describe("subtract", () => {
  it("is identical to add (characteristic-2 field)", () => {
    expect(subtract(0x53, 0xCA)).toBe(add(0x53, 0xCA));
    expect(subtract(42, 17)).toBe(add(42, 17));
    expect(subtract(0, 255)).toBe(add(0, 255));
  });

  it("subtract(x, x) = 0 for all x", () => {
    expect(subtract(5, 5)).toBe(0);
    expect(subtract(0, 0)).toBe(0);
    expect(subtract(255, 255)).toBe(0);
  });

  it("subtract(x, 0) = x", () => {
    expect(subtract(42, 0)).toBe(42);
  });
});

// ---------------------------------------------------------------------------
// multiply
// ---------------------------------------------------------------------------

describe("multiply", () => {
  it("multiply(0, x) = 0 for any x", () => {
    expect(multiply(0, 0)).toBe(0);
    expect(multiply(0, 255)).toBe(0);
    expect(multiply(0, 1)).toBe(0);
  });

  it("multiply(x, 0) = 0 for any x", () => {
    expect(multiply(255, 0)).toBe(0);
    expect(multiply(1, 0)).toBe(0);
  });

  it("multiply(1, x) = x (ONE is multiplicative identity)", () => {
    expect(multiply(ONE, 42)).toBe(42);
    expect(multiply(ONE, 0)).toBe(0);
    expect(multiply(42, ONE)).toBe(42);
  });

  it("multiply(2, 128) = 29 (first reduction step: 256 XOR 285 = 29)", () => {
    // 2 * 128 = 256 in integers; in GF(256), 256 >= 256, so XOR with 0x11D=285
    // 256 XOR 285 = 0x100 XOR 0x11D = 0x01D = 29
    expect(multiply(2, 128)).toBe(29);
  });

  it("is commutative: multiply(a, b) = multiply(b, a)", () => {
    expect(multiply(3, 7)).toBe(multiply(7, 3));
    expect(multiply(100, 200)).toBe(multiply(200, 100));
  });

  it("multiply(a, inverse(a)) = 1 for non-zero a", () => {
    // a * a^(-1) = 1 in any field
    expect(multiply(5, inverse(5))).toBe(1);
    expect(multiply(255, inverse(255))).toBe(1);
    expect(multiply(2, inverse(2))).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// divide
// ---------------------------------------------------------------------------

describe("divide", () => {
  it("divide(0, b) = 0 for any non-zero b", () => {
    expect(divide(0, 1)).toBe(0);
    expect(divide(0, 255)).toBe(0);
  });

  it("divide(a, 1) = a (dividing by multiplicative identity)", () => {
    expect(divide(42, 1)).toBe(42);
    expect(divide(255, 1)).toBe(255);
  });

  it("divide(a, a) = 1 for any non-zero a", () => {
    expect(divide(5, 5)).toBe(1);
    expect(divide(255, 255)).toBe(1);
    expect(divide(1, 1)).toBe(1);
  });

  it("divide is the inverse of multiply: divide(multiply(a, b), b) = a", () => {
    const a = 37, b = 89;
    expect(divide(multiply(a, b), b)).toBe(a);
  });

  it("throws when dividing by zero", () => {
    expect(() => divide(1, 0)).toThrow();
    expect(() => divide(0, 0)).toThrow();
    expect(() => divide(255, 0)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// power
// ---------------------------------------------------------------------------

describe("power", () => {
  it("power(b, 0) = 1 for any non-zero b (empty product)", () => {
    expect(power(2, 0)).toBe(1);
    expect(power(255, 0)).toBe(1);
    expect(power(42, 0)).toBe(1);
  });

  it("power(0, 0) = 1 by convention", () => {
    expect(power(0, 0)).toBe(1);
  });

  it("power(0, n) = 0 for n > 0", () => {
    expect(power(0, 1)).toBe(0);
    expect(power(0, 10)).toBe(0);
    expect(power(0, 255)).toBe(0);
  });

  it("power(b, 1) = b", () => {
    expect(power(2, 1)).toBe(2);
    expect(power(42, 1)).toBe(42);
  });

  it("power(2, 8) = 29 (first overflow reduction)", () => {
    // ALOG[8] = 29 (from the table construction algorithm)
    expect(power(2, 8)).toBe(29);
  });

  it("power(2, 255) = 1 (multiplicative group order is 255)", () => {
    // Every non-zero element g satisfies g^255 = 1 (Fermat's little theorem)
    expect(power(2, 255)).toBe(1);
  });

  it("power(2, 256) = 2 (wraps around the cyclic group)", () => {
    // g^256 = g^255 * g = 1 * g = g = 2
    expect(power(2, 256)).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// inverse
// ---------------------------------------------------------------------------

describe("inverse", () => {
  it("inverse(1) = 1", () => {
    expect(inverse(1)).toBe(1);
  });

  it("a * inverse(a) = 1 for all non-zero a", () => {
    // Verify the fundamental identity for a sample of values
    for (const a of [1, 2, 3, 5, 7, 42, 128, 255]) {
      expect(multiply(a, inverse(a))).toBe(1);
    }
  });

  it("inverse(inverse(a)) = a (double inverse is identity)", () => {
    expect(inverse(inverse(5))).toBe(5);
    expect(inverse(inverse(255))).toBe(255);
  });

  it("inverse is consistent with divide: inverse(a) = divide(1, a)", () => {
    expect(inverse(5)).toBe(divide(1, 5));
    expect(inverse(255)).toBe(divide(1, 255));
    expect(inverse(128)).toBe(divide(1, 128));
  });

  it("throws when a = 0 (zero has no multiplicative inverse)", () => {
    expect(() => inverse(0)).toThrow();
  });
});
