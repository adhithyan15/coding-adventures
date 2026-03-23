/**
 * Test suite for @ca/uuid
 *
 * Covers: parse(), isValid(), UUID class properties, constants, and all
 * generator functions (v1, v3, v4, v5, v7).
 *
 * RFC test vectors from RFC 4122 Appendix B and online UUID calculators.
 */
import { describe, it, expect } from "vitest";
import {
  VERSION,
  UUID,
  UUIDError,
  parse,
  isValid,
  NAMESPACE_DNS,
  NAMESPACE_URL,
  NAMESPACE_OID,
  NAMESPACE_X500,
  NIL,
  MAX,
  v1,
  v3,
  v4,
  v5,
  v7,
} from "../src/index.js";

// ─── VERSION ─────────────────────────────────────────────────────────────────

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ─── UUIDError ───────────────────────────────────────────────────────────────

describe("UUIDError", () => {
  it("is an instance of Error", () => {
    const err = new UUIDError("test");
    expect(err).toBeInstanceOf(Error);
  });

  it("has name UUIDError", () => {
    const err = new UUIDError("test");
    expect(err.name).toBe("UUIDError");
  });

  it("carries the message", () => {
    const err = new UUIDError("bad uuid");
    expect(err.message).toBe("bad uuid");
  });
});

// ─── parse() ─────────────────────────────────────────────────────────────────

describe("parse()", () => {
  const canonical = "550e8400-e29b-41d4-a716-446655440000";

  it("parses standard lowercase form", () => {
    const id = parse(canonical);
    expect(id.toString()).toBe(canonical);
  });

  it("parses uppercase form", () => {
    const id = parse("550E8400-E29B-41D4-A716-446655440000");
    expect(id.toString()).toBe(canonical);
  });

  it("parses compact (no hyphens) form", () => {
    const id = parse("550e8400e29b41d4a716446655440000");
    expect(id.toString()).toBe(canonical);
  });

  it("parses braced form", () => {
    const id = parse("{550e8400-e29b-41d4-a716-446655440000}");
    expect(id.toString()).toBe(canonical);
  });

  it("parses URN form", () => {
    const id = parse("urn:uuid:550e8400-e29b-41d4-a716-446655440000");
    expect(id.toString()).toBe(canonical);
  });

  it("parses URN form case-insensitively", () => {
    const id = parse("URN:UUID:550e8400-e29b-41d4-a716-446655440000");
    expect(id.toString()).toBe(canonical);
  });

  it("throws UUIDError for empty string", () => {
    expect(() => parse("")).toThrow(UUIDError);
  });

  it("throws UUIDError for too short string", () => {
    expect(() => parse("550e8400-e29b-41d4")).toThrow(UUIDError);
  });

  it("throws UUIDError for too long string", () => {
    expect(() => parse("550e8400-e29b-41d4-a716-446655440000-extra")).toThrow(UUIDError);
  });

  it("throws UUIDError for invalid hex characters", () => {
    expect(() => parse("550e8400-e29b-41d4-a716-44665544000g")).toThrow(UUIDError);
  });

  it("throws UUIDError for invalid format with correct length", () => {
    // 32 chars but with non-hex 'x'
    expect(() => parse("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")).toThrow(UUIDError);
  });

  it("parses NIL UUID string", () => {
    const id = parse("00000000-0000-0000-0000-000000000000");
    expect(id.isNil).toBe(true);
  });

  it("parses MAX UUID string", () => {
    const id = parse("ffffffff-ffff-ffff-ffff-ffffffffffff");
    expect(id.isMax).toBe(true);
  });
});

// ─── isValid() ───────────────────────────────────────────────────────────────

describe("isValid()", () => {
  it("returns true for standard form", () => {
    expect(isValid("550e8400-e29b-41d4-a716-446655440000")).toBe(true);
  });

  it("returns true for compact form", () => {
    expect(isValid("550e8400e29b41d4a716446655440000")).toBe(true);
  });

  it("returns true for braced form", () => {
    expect(isValid("{550e8400-e29b-41d4-a716-446655440000}")).toBe(true);
  });

  it("returns true for URN form", () => {
    expect(isValid("urn:uuid:550e8400-e29b-41d4-a716-446655440000")).toBe(true);
  });

  it("returns false for empty string", () => {
    expect(isValid("")).toBe(false);
  });

  it("returns false for invalid string", () => {
    expect(isValid("not-a-uuid")).toBe(false);
  });

  it("returns false for partial UUID", () => {
    expect(isValid("550e8400-e29b")).toBe(false);
  });
});

// ─── UUID class ───────────────────────────────────────────────────────────────

describe("UUID class", () => {
  describe("constructor", () => {
    it("accepts a Uint8Array of 16 bytes", () => {
      const raw = new Uint8Array(16).fill(1);
      const id = new UUID(raw);
      expect(id.bytes).toEqual(raw);
    });

    it("copies the Uint8Array (immutable)", () => {
      const raw = new Uint8Array(16).fill(1);
      const id = new UUID(raw);
      raw.fill(0); // mutate original
      expect(id.bytes[0]).toBe(1); // UUID is unaffected
    });

    it("throws UUIDError for wrong-length Uint8Array", () => {
      expect(() => new UUID(new Uint8Array(10))).toThrow(UUIDError);
    });

    it("accepts a BigInt", () => {
      const id = new UUID(BigInt(0));
      expect(id.isNil).toBe(true);
    });

    it("accepts a string", () => {
      const id = new UUID("550e8400-e29b-41d4-a716-446655440000");
      expect(id.toString()).toBe("550e8400-e29b-41d4-a716-446655440000");
    });
  });

  describe("bytes property", () => {
    it("returns a copy (mutation-safe)", () => {
      const id = v4();
      const b1 = id.bytes;
      b1[0] = 0xFF; // mutate returned copy
      expect(id.bytes[0]).not.toBe(0xFF); // original unchanged
    });

    it("returns 16 bytes", () => {
      expect(v4().bytes.length).toBe(16);
    });
  });

  describe("int property", () => {
    it("NIL UUID has int === 0n", () => {
      expect(NIL.int).toBe(BigInt(0));
    });

    it("MAX UUID has int === 2^128 - 1", () => {
      const expected = (BigInt(1) << BigInt(128)) - BigInt(1);
      expect(MAX.int).toBe(expected);
    });

    it("round-trips through BigInt constructor", () => {
      const id = v4();
      const id2 = new UUID(id.int);
      expect(id.equals(id2)).toBe(true);
    });
  });

  describe("version property", () => {
    it("NIL UUID has version 0", () => {
      expect(NIL.version).toBe(0);
    });

    it("v4 UUID has version 4", () => {
      expect(v4().version).toBe(4);
    });

    it("v5 UUID has version 5", () => {
      expect(v5(NAMESPACE_DNS, "example.com").version).toBe(5);
    });

    it("v3 UUID has version 3", () => {
      expect(v3(NAMESPACE_DNS, "example.com").version).toBe(3);
    });

    it("v1 UUID has version 1", () => {
      expect(v1().version).toBe(1);
    });

    it("v7 UUID has version 7", () => {
      expect(v7().version).toBe(7);
    });
  });

  describe("variant property", () => {
    it("v4 has variant rfc4122", () => {
      expect(v4().variant).toBe("rfc4122");
    });

    it("v5 has variant rfc4122", () => {
      expect(v5(NAMESPACE_DNS, "test").variant).toBe("rfc4122");
    });

    it("v3 has variant rfc4122", () => {
      expect(v3(NAMESPACE_DNS, "test").variant).toBe("rfc4122");
    });

    it("v1 has variant rfc4122", () => {
      expect(v1().variant).toBe("rfc4122");
    });

    it("v7 has variant rfc4122", () => {
      expect(v7().variant).toBe("rfc4122");
    });

    it("NIL UUID has variant ncs (top bit 0)", () => {
      // NIL = all zeros; byte 8 = 0x00, top bit = 0 → "ncs"
      expect(NIL.variant).toBe("ncs");
    });
  });

  describe("isNil property", () => {
    it("NIL constant isNil is true", () => {
      expect(NIL.isNil).toBe(true);
    });

    it("v4 UUID isNil is false", () => {
      expect(v4().isNil).toBe(false);
    });

    it("zero-filled Uint8Array gives isNil true", () => {
      expect(new UUID(new Uint8Array(16)).isNil).toBe(true);
    });
  });

  describe("isMax property", () => {
    it("MAX constant isMax is true", () => {
      expect(MAX.isMax).toBe(true);
    });

    it("v4 UUID isMax is false", () => {
      expect(v4().isMax).toBe(false);
    });
  });

  describe("toString()", () => {
    it("formats as 8-4-4-4-12 lowercase", () => {
      const s = v4().toString();
      // Standard UUID pattern
      expect(s).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
    });

    it("NIL toString is all zeros", () => {
      expect(NIL.toString()).toBe("00000000-0000-0000-0000-000000000000");
    });

    it("MAX toString is all f's", () => {
      expect(MAX.toString()).toBe("ffffffff-ffff-ffff-ffff-ffffffffffff");
    });
  });

  describe("equals()", () => {
    it("same UUID equals itself", () => {
      const id = v4();
      expect(id.equals(id)).toBe(true);
    });

    it("parsed same string are equal", () => {
      const s = "550e8400-e29b-41d4-a716-446655440000";
      expect(parse(s).equals(parse(s))).toBe(true);
    });

    it("different UUIDs are not equal", () => {
      const a = v4();
      const b = v4();
      // Astronomically unlikely to collide; treat as guaranteed different
      expect(a.equals(b)).toBe(false);
    });

    it("NIL and MAX are not equal", () => {
      expect(NIL.equals(MAX)).toBe(false);
    });
  });

  describe("compareTo()", () => {
    it("returns 0 for equal UUIDs", () => {
      const id = parse("550e8400-e29b-41d4-a716-446655440000");
      expect(id.compareTo(id)).toBe(0);
    });

    it("NIL < MAX returns -1", () => {
      expect(NIL.compareTo(MAX)).toBe(-1);
    });

    it("MAX > NIL returns 1", () => {
      expect(MAX.compareTo(NIL)).toBe(1);
    });

    it("is consistent with byte ordering", () => {
      const lo = new UUID(new Uint8Array(16).fill(0x10));
      const hi = new UUID(new Uint8Array(16).fill(0x20));
      expect(lo.compareTo(hi)).toBe(-1);
      expect(hi.compareTo(lo)).toBe(1);
    });
  });
});

// ─── Namespace Constants ──────────────────────────────────────────────────────

describe("Namespace constants", () => {
  it("NAMESPACE_DNS has correct string", () => {
    expect(NAMESPACE_DNS.toString()).toBe("6ba7b810-9dad-11d1-80b4-00c04fd430c8");
  });

  it("NAMESPACE_URL has correct string", () => {
    expect(NAMESPACE_URL.toString()).toBe("6ba7b811-9dad-11d1-80b4-00c04fd430c8");
  });

  it("NAMESPACE_OID has correct string", () => {
    expect(NAMESPACE_OID.toString()).toBe("6ba7b812-9dad-11d1-80b4-00c04fd430c8");
  });

  it("NAMESPACE_X500 has correct string", () => {
    expect(NAMESPACE_X500.toString()).toBe("6ba7b814-9dad-11d1-80b4-00c04fd430c8");
  });
});

// ─── NIL and MAX Constants ────────────────────────────────────────────────────

describe("NIL constant", () => {
  it("is all zeros", () => {
    expect(NIL.toString()).toBe("00000000-0000-0000-0000-000000000000");
  });

  it("isNil is true", () => {
    expect(NIL.isNil).toBe(true);
  });
});

describe("MAX constant", () => {
  it("is all f's", () => {
    expect(MAX.toString()).toBe("ffffffff-ffff-ffff-ffff-ffffffffffff");
  });

  it("isMax is true", () => {
    expect(MAX.isMax).toBe(true);
  });
});

// ─── v4() ────────────────────────────────────────────────────────────────────

describe("v4()", () => {
  it("returns version 4", () => {
    expect(v4().version).toBe(4);
  });

  it("returns rfc4122 variant", () => {
    expect(v4().variant).toBe("rfc4122");
  });

  it("produces unique UUIDs", () => {
    const a = v4();
    const b = v4();
    expect(a.equals(b)).toBe(false);
  });

  it("produces valid 8-4-4-4-12 format", () => {
    const s = v4().toString();
    expect(s).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });
});

// ─── v5() ────────────────────────────────────────────────────────────────────

describe("v5()", () => {
  it("RFC 4122 vector: v5(NAMESPACE_DNS, 'python.org')", () => {
    const result = v5(NAMESPACE_DNS, "python.org");
    expect(result.toString()).toBe("886313e1-3b8a-5372-9b90-0c9aee199e5d");
  });

  it("returns version 5", () => {
    expect(v5(NAMESPACE_DNS, "example.com").version).toBe(5);
  });

  it("returns rfc4122 variant", () => {
    expect(v5(NAMESPACE_DNS, "example.com").variant).toBe("rfc4122");
  });

  it("is deterministic: same inputs → same output", () => {
    const a = v5(NAMESPACE_URL, "https://example.com");
    const b = v5(NAMESPACE_URL, "https://example.com");
    expect(a.equals(b)).toBe(true);
  });

  it("different names → different UUIDs", () => {
    const a = v5(NAMESPACE_DNS, "foo.com");
    const b = v5(NAMESPACE_DNS, "bar.com");
    expect(a.equals(b)).toBe(false);
  });

  it("different namespaces → different UUIDs for same name", () => {
    const a = v5(NAMESPACE_DNS, "example.com");
    const b = v5(NAMESPACE_URL, "example.com");
    expect(a.equals(b)).toBe(false);
  });
});

// ─── v3() ────────────────────────────────────────────────────────────────────

describe("v3()", () => {
  it("RFC 4122 vector: v3(NAMESPACE_DNS, 'python.org')", () => {
    const result = v3(NAMESPACE_DNS, "python.org");
    expect(result.toString()).toBe("6fa459ea-ee8a-3ca4-894e-db77e160355e");
  });

  it("returns version 3", () => {
    expect(v3(NAMESPACE_DNS, "example.com").version).toBe(3);
  });

  it("returns rfc4122 variant", () => {
    expect(v3(NAMESPACE_DNS, "example.com").variant).toBe("rfc4122");
  });

  it("is deterministic: same inputs → same output", () => {
    const a = v3(NAMESPACE_URL, "https://example.com");
    const b = v3(NAMESPACE_URL, "https://example.com");
    expect(a.equals(b)).toBe(true);
  });

  it("different names → different UUIDs", () => {
    const a = v3(NAMESPACE_DNS, "foo.com");
    const b = v3(NAMESPACE_DNS, "bar.com");
    expect(a.equals(b)).toBe(false);
  });
});

// ─── v1() ────────────────────────────────────────────────────────────────────

describe("v1()", () => {
  it("returns version 1", () => {
    expect(v1().version).toBe(1);
  });

  it("returns rfc4122 variant", () => {
    expect(v1().variant).toBe("rfc4122");
  });

  it("produces unique UUIDs on successive calls", () => {
    const a = v1();
    const b = v1();
    // With random clock_seq and node, UUIDs should differ
    expect(a.equals(b)).toBe(false);
  });

  it("produces valid 8-4-4-4-12 format", () => {
    const s = v1().toString();
    expect(s).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/);
  });
});

// ─── v7() ────────────────────────────────────────────────────────────────────

describe("v7()", () => {
  it("returns version 7", () => {
    expect(v7().version).toBe(7);
  });

  it("returns rfc4122 variant", () => {
    expect(v7().variant).toBe("rfc4122");
  });

  it("produces unique UUIDs", () => {
    const a = v7();
    const b = v7();
    expect(a.equals(b)).toBe(false);
  });

  it("produces valid 8-4-4-4-12 format", () => {
    const s = v7().toString();
    expect(s).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });

  it("time ordering: v7 UUIDs from different milliseconds are ordered", async () => {
    const a = v7();
    // Wait 2ms so the next UUID lands in a later millisecond bucket.
    // Without the delay both calls may resolve within the same ms, making
    // the 48-bit timestamp prefix identical and ordering undefined.
    await new Promise((resolve) => setTimeout(resolve, 2));
    const b = v7();
    expect(a.compareTo(b)).toBeLessThanOrEqual(0);
  });

  it("encodes current timestamp in first 6 bytes", () => {
    const before = Date.now();
    const id = v7();
    const after = Date.now();

    // Extract 48-bit timestamp from bytes 0–5 (big-endian)
    const b = id.bytes;
    const ts = ((b[0] * 256 + b[1]) * 65536 + (b[2] * 256 + b[3])) * 65536 + (b[4] * 256 + b[5]);

    expect(ts).toBeGreaterThanOrEqual(before);
    expect(ts).toBeLessThanOrEqual(after + 1); // small tolerance
  });
});
