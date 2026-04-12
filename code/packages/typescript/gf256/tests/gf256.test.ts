import { describe, it, expect } from "vitest";
import {
  VERSION,
  ZERO,
  ONE,
  PRIMITIVE_POLYNOMIAL,
  ALOG,
  LOG,
  add,
  subtract,
  multiply,
  divide,
  power,
  inverse,
  zero,
  one,
  createField,
} from "../src/index.js";

// =============================================================================
// VERSION
// =============================================================================

describe("VERSION", () => {
  it("is a semver string", () => {
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

// =============================================================================
// Constants
// =============================================================================

describe("constants", () => {
  it("ZERO is 0", () => {
    expect(ZERO).toBe(0);
  });

  it("ONE is 1", () => {
    expect(ONE).toBe(1);
  });

  it("PRIMITIVE_POLYNOMIAL is 0x11D", () => {
    expect(PRIMITIVE_POLYNOMIAL).toBe(0x11d);
    expect(PRIMITIVE_POLYNOMIAL).toBe(285);
  });
});

// =============================================================================
// Log/Antilog Table Consistency
// =============================================================================

describe("LOG and ALOG tables", () => {
  it("ALOG has 256 entries (0..255, where ALOG[255]=1)", () => {
    expect(ALOG.length).toBe(256);
  });

  it("LOG has 256 entries", () => {
    expect(LOG.length).toBe(256);
  });

  it("ALOG[0] = 1 (g^0 = 1)", () => {
    expect(ALOG[0]).toBe(1);
  });

  it("ALOG[1] = 2 (g^1 = 2)", () => {
    expect(ALOG[1]).toBe(2);
  });

  it("ALOG[8] = 29 (first reduction step)", () => {
    // 2^8 = 256; 256 XOR 0x11D = 0x100 XOR 0x11D = 0x1D = 29
    expect(ALOG[8]).toBe(29);
  });

  it("ALOG values are all in range [1, 255]", () => {
    for (let i = 0; i < 255; i++) {
      expect(ALOG[i]).toBeGreaterThanOrEqual(1);
      expect(ALOG[i]).toBeLessThanOrEqual(255);
    }
  });

  it("ALOG[0..254] is a bijection: all 255 non-zero values appear exactly once", () => {
    const seen = new Set(ALOG.slice(0, 255));
    expect(seen.size).toBe(255);
    expect(seen.has(0)).toBe(false); // 0 never appears in ALOG[0..254]
  });

  it("ALOG[LOG[x]] = x for all x in 1..255", () => {
    for (let x = 1; x <= 255; x++) {
      // LOG[x] is in [0, 254], so ALOG[LOG[x]] is in ALOG[0..254]
      expect(ALOG[LOG[x]]).toBe(x);
    }
  });

  it("LOG[ALOG[i]] = i for all i in 0..254", () => {
    for (let i = 0; i < 255; i++) {
      expect(LOG[ALOG[i]]).toBe(i);
    }
  });

  it("LOG[1] = 0 (log of identity is 0)", () => {
    expect(LOG[1]).toBe(0);
  });

  it("LOG[2] = 1 (the generator has log 1)", () => {
    expect(LOG[2]).toBe(1);
  });
});

// =============================================================================
// add
// =============================================================================

describe("add", () => {
  it("add(0, x) = x for all x", () => {
    for (let x = 0; x <= 255; x++) {
      expect(add(0, x)).toBe(x);
      expect(add(x, 0)).toBe(x);
    }
  });

  it("add(x, x) = 0 for all x (characteristic 2)", () => {
    for (let x = 0; x <= 255; x++) {
      expect(add(x, x)).toBe(0);
    }
  });

  it("is commutative", () => {
    for (let x = 0; x < 32; x++) {
      for (let y = 0; y < 32; y++) {
        expect(add(x, y)).toBe(add(y, x));
      }
    }
  });

  it("is associative", () => {
    const a = 0x53;
    const b = 0xca;
    const c = 0x7f;
    expect(add(add(a, b), c)).toBe(add(a, add(b, c)));
  });

  it("specific XOR check: 0x53 XOR 0xCA = 0x99", () => {
    expect(add(0x53, 0xca)).toBe(0x53 ^ 0xca);
  });

  it("is the same operation as XOR", () => {
    for (let x = 0; x < 256; x++) {
      expect(add(x, 0x42)).toBe(x ^ 0x42);
    }
  });
});

// =============================================================================
// subtract
// =============================================================================

describe("subtract", () => {
  it("subtract(x, x) = 0 for all x", () => {
    for (let x = 0; x <= 255; x++) {
      expect(subtract(x, x)).toBe(0);
    }
  });

  it("is the same as add in characteristic 2", () => {
    for (let x = 0; x < 32; x++) {
      for (let y = 0; y < 32; y++) {
        expect(subtract(x, y)).toBe(add(x, y));
      }
    }
  });

  it("subtract(0, x) = x (negation is identity in char 2)", () => {
    for (let x = 0; x <= 255; x++) {
      expect(subtract(0, x)).toBe(x);
    }
  });
});

// =============================================================================
// multiply
// =============================================================================

describe("multiply", () => {
  it("multiply(x, 0) = 0 for all x", () => {
    for (let x = 0; x <= 255; x++) {
      expect(multiply(x, 0)).toBe(0);
      expect(multiply(0, x)).toBe(0);
    }
  });

  it("multiply(x, 1) = x for all x (identity)", () => {
    for (let x = 0; x <= 255; x++) {
      expect(multiply(x, 1)).toBe(x);
      expect(multiply(1, x)).toBe(x);
    }
  });

  it("is commutative", () => {
    for (let x = 0; x < 32; x++) {
      for (let y = 0; y < 32; y++) {
        expect(multiply(x, y)).toBe(multiply(y, x));
      }
    }
  });

  it("is associative", () => {
    const a = 0x53;
    const b = 0xca;
    const c = 0x3d;
    expect(multiply(multiply(a, b), c)).toBe(multiply(a, multiply(b, c)));
  });

  it("known spot check: 0x53 × inverse(0x53) = 0x01", () => {
    // With the 0x11D polynomial, inverse(0x53) = 0x8C.
    expect(multiply(0x53, 0x8c)).toBe(0x01);
  });

  it("multiply(x, 2) = ALOG[(LOG[x]+1) mod 255] for x != 0", () => {
    for (let x = 1; x <= 255; x++) {
      const expected = ALOG[(LOG[x] + 1) % 255];
      expect(multiply(x, 2)).toBe(expected);
    }
  });

  it("is distributive over add", () => {
    const a = 0x34;
    const b = 0x56;
    const c = 0x78;
    expect(multiply(a, add(b, c))).toBe(add(multiply(a, b), multiply(a, c)));
  });
});

// =============================================================================
// divide
// =============================================================================

describe("divide", () => {
  it("divide(x, 1) = x for all x", () => {
    for (let x = 0; x <= 255; x++) {
      expect(divide(x, 1)).toBe(x);
    }
  });

  it("divide(0, x) = 0 for all x != 0", () => {
    for (let x = 1; x <= 255; x++) {
      expect(divide(0, x)).toBe(0);
    }
  });

  it("divide(x, x) = 1 for all x != 0", () => {
    for (let x = 1; x <= 255; x++) {
      expect(divide(x, x)).toBe(1);
    }
  });

  it("throws for division by zero", () => {
    expect(() => divide(1, 0)).toThrow();
    expect(() => divide(0, 0)).toThrow();
  });

  it("divide is inverse of multiply: divide(multiply(a, b), b) = a", () => {
    for (let a = 0; a < 32; a++) {
      for (let b = 1; b < 32; b++) {
        expect(divide(multiply(a, b), b)).toBe(a);
      }
    }
  });
});

// =============================================================================
// power
// =============================================================================

describe("power", () => {
  it("x^0 = 1 for all x != 0", () => {
    for (let x = 1; x <= 255; x++) {
      expect(power(x, 0)).toBe(1);
    }
  });

  it("0^0 = 1 by convention", () => {
    expect(power(0, 0)).toBe(1);
  });

  it("0^n = 0 for n > 0", () => {
    expect(power(0, 1)).toBe(0);
    expect(power(0, 5)).toBe(0);
  });

  it("x^1 = x for all x", () => {
    for (let x = 0; x <= 255; x++) {
      expect(power(x, 1)).toBe(x);
    }
  });

  it("g^255 = 1 (generator has order 255)", () => {
    // The multiplicative group has order 255, so g^255 = 1.
    expect(power(2, 255)).toBe(1);
  });

  it("g^i = ALOG[i] for i in 0..254", () => {
    for (let i = 0; i < 255; i++) {
      expect(power(2, i)).toBe(ALOG[i]);
    }
  });

  it("x^254 = inverse(x) (Fermat's little theorem)", () => {
    // x^255 = 1, so x^254 = x^(-1)
    for (let x = 1; x <= 20; x++) {
      expect(power(x, 254)).toBe(inverse(x));
    }
  });
});

// =============================================================================
// inverse
// =============================================================================

describe("inverse", () => {
  it("throws for inverse of 0", () => {
    expect(() => inverse(0)).toThrow();
  });

  it("inverse(1) = 1", () => {
    expect(inverse(1)).toBe(1);
  });

  it("x * inverse(x) = 1 for x = 1..10", () => {
    for (let x = 1; x <= 10; x++) {
      expect(multiply(x, inverse(x))).toBe(1);
    }
  });

  it("x * inverse(x) = 1 for x = 250..255", () => {
    for (let x = 250; x <= 255; x++) {
      expect(multiply(x, inverse(x))).toBe(1);
    }
  });

  it("x * inverse(x) = 1 for all x in 1..255", () => {
    for (let x = 1; x <= 255; x++) {
      expect(multiply(x, inverse(x))).toBe(1);
    }
  });

  it("inverse is its own inverse: inverse(inverse(x)) = x", () => {
    for (let x = 1; x <= 255; x++) {
      expect(inverse(inverse(x))).toBe(x);
    }
  });

  it("known pair: inverse(0x53) = 0x8C with 0x11D polynomial", () => {
    // With primitive polynomial 0x11D, inverse(0x53) = 0x8C.
    expect(inverse(0x53)).toBe(0x8c);
    expect(multiply(0x53, inverse(0x53))).toBe(1);
    expect(multiply(0x8c, inverse(0x8c))).toBe(1);
  });
});

// =============================================================================
// zero and one
// =============================================================================

describe("zero()", () => {
  it("returns 0", () => {
    expect(zero()).toBe(0);
  });

  it("is additive identity", () => {
    expect(add(zero(), 0x42)).toBe(0x42);
    expect(add(0x42, zero())).toBe(0x42);
  });
});

describe("one()", () => {
  it("returns 1", () => {
    expect(one()).toBe(1);
  });

  it("is multiplicative identity", () => {
    expect(multiply(one(), 0x42)).toBe(0x42);
    expect(multiply(0x42, one())).toBe(0x42);
  });
});

// =============================================================================
// createField — parameterizable field factory
// =============================================================================

describe("createField", () => {
  describe("AES field (0x11B)", () => {
    const aes = createField(0x11B);

    it("multiply(0x53, 0x8C) = 1 — AES GF(2^8) inverses", () => {
      expect(aes.multiply(0x53, 0x8C)).toBe(0x01);
    });

    it("multiply(0x57, 0x83) = 0xC1 — FIPS 197 Appendix B", () => {
      expect(aes.multiply(0x57, 0x83)).toBe(0xC1);
    });

    it("inverse(0x53) = 0x8C", () => {
      expect(aes.inverse(0x53)).toBe(0x8C);
    });

    it("multiply(a, inverse(a)) = 1 for a in 1..20", () => {
      for (let a = 1; a <= 20; a++) {
        expect(aes.multiply(a, aes.inverse(a))).toBe(1);
      }
    });

    it("commutativity", () => {
      const vals = [0, 1, 0x53, 0x8C, 0xFF];
      for (const a of vals) {
        for (const b of vals) {
          expect(aes.multiply(a, b)).toBe(aes.multiply(b, a));
        }
      }
    });

    it("add is XOR (polynomial-independent)", () => {
      expect(aes.add(0x53, 0xCA)).toBe(0x53 ^ 0xCA);
    });

    it("divide by zero throws", () => {
      expect(() => aes.divide(5, 0)).toThrow("GF256Field: division by zero");
    });

    it("inverse of zero throws", () => {
      expect(() => aes.inverse(0)).toThrow("GF256Field: zero has no multiplicative inverse");
    });

    it("polynomial property is stored", () => {
      expect(aes.polynomial).toBe(0x11B);
    });
  });

  describe("RS field (0x11D) matches module-level functions", () => {
    const rs = createField(0x11D);

    it("multiply matches module multiply for sample values", () => {
      for (let a = 0; a < 16; a++) {
        for (let b = 0; b < 16; b++) {
          expect(rs.multiply(a, b)).toBe(multiply(a, b));
        }
      }
    });

    it("inverse matches module inverse for sample values", () => {
      for (let a = 1; a <= 16; a++) {
        expect(rs.inverse(a)).toBe(inverse(a));
      }
    });
  });
});
