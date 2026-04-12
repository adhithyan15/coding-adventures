/**
 * Tests for the LZ77 compression implementation.
 *
 * Test vectors come from the CMP00 specification. Covers: literals,
 * backreferences, overlapping matches, edge cases, and round-trip invariants.
 * Coverage target: 95%+.
 */

import { describe, expect, it } from "vitest";
import {
  compress,
  decode,
  decompress,
  deserialiseTokens,
  encode,
  serialiseTokens,
  token,
} from "../src/lz77.js";

// Helper: encode a string to Uint8Array.
const enc = (s: string) => new TextEncoder().encode(s);

// Helper: decode a Uint8Array to string.
const dec = (b: Uint8Array) => new TextDecoder().decode(b);

// Helper: construct bytes from an array of byte values.
const bytes = (...values: number[]) => new Uint8Array(values);

// ---- Specification Test Vectors ----

describe("SpecVectors", () => {
  it("empty input produces no tokens", () => {
    expect(encode(new Uint8Array())).toEqual([]);
    expect(decode([])).toEqual(new Uint8Array());
  });

  it("no repetition → all literal tokens", () => {
    const tokens = encode(enc("ABCDE"));
    expect(tokens).toHaveLength(5);
    for (const tok of tokens) {
      expect(tok.offset).toBe(0);
      expect(tok.length).toBe(0);
    }
  });

  it("all identical bytes exploit overlap mechanism", () => {
    // "AAAAAAA" → literal A + backreference (offset=1, length=5, nextChar=A).
    const tokens = encode(enc("AAAAAAA"));
    expect(tokens).toHaveLength(2);
    expect(tokens[0]).toEqual(token(0, 0, 65));
    expect(tokens[1]!.offset).toBe(1);
    expect(tokens[1]!.length).toBe(5);
    expect(tokens[1]!.nextChar).toBe(65);

    expect(dec(decode(tokens))).toBe("AAAAAAA");
  });

  it("repeated pair uses backreference", () => {
    // "ABABABAB" → [A literal, B literal, (offset=2, length=5, nextChar=B)].
    const tokens = encode(enc("ABABABAB"));
    expect(tokens).toHaveLength(3);
    expect(tokens[0]).toEqual(token(0, 0, 65));
    expect(tokens[1]).toEqual(token(0, 0, 66));
    expect(tokens[2]!.offset).toBe(2);
    expect(tokens[2]!.length).toBe(5);
    expect(tokens[2]!.nextChar).toBe(66);

    expect(dec(decode(tokens))).toBe("ABABABAB");
  });

  it("AABCBBABC with min_match=3 → all literals", () => {
    const tokens = encode(enc("AABCBBABC"));
    expect(tokens).toHaveLength(9);
    for (const tok of tokens) {
      expect(tok.offset).toBe(0);
      expect(tok.length).toBe(0);
    }
    expect(dec(decode(tokens))).toBe("AABCBBABC");
  });

  it("AABCBBABC with min_match=2 → round-trip holds", () => {
    const tokens = encode(enc("AABCBBABC"), 4096, 255, 2);
    expect(dec(decode(tokens))).toBe("AABCBBABC");
  });
});

// ---- Round-Trip Invariant Tests ----

describe("RoundTrip", () => {
  it("empty", () => {
    expect(decode(encode(new Uint8Array()))).toEqual(new Uint8Array());
  });

  it("single bytes", () => {
    for (const b of [65, 0, 255]) {
      expect(decode(encode(bytes(b)))).toEqual(bytes(b));
    }
  });

  const strings = [
    "hello world",
    "the quick brown fox",
    "ababababab",
    "aaaaaaaaaa",
  ];
  for (const s of strings) {
    it(`round-trip: "${s}"`, () => {
      expect(dec(decode(encode(enc(s))))).toBe(s);
    });
  }

  it("null bytes round-trip", () => {
    const data = bytes(0, 0, 0);
    expect(decode(encode(data))).toEqual(data);
  });

  it("0xFF bytes round-trip", () => {
    const data = bytes(255, 255, 255);
    expect(decode(encode(data))).toEqual(data);
  });

  it("all 256 byte values round-trip", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    expect(decode(encode(data))).toEqual(data);
  });

  it("compress/decompress round-trip", () => {
    const cases = ["", "A", "ABCDE", "AAAAAAA", "ABABABAB", "hello world"];
    for (const s of cases) {
      const data = enc(s);
      expect(decompress(compress(data))).toEqual(data);
    }
  });
});

// ---- Parameter Tests ----

describe("Parameters", () => {
  it("offsets never exceed windowSize", () => {
    const data = new Uint8Array(5002);
    data[0] = 88; // X
    data.fill(89, 1, 5001); // 5000 Ys
    data[5001] = 88; // X again
    const tokens = encode(data, 100);
    for (const tok of tokens) {
      expect(tok.offset).toBeLessThanOrEqual(100);
    }
  });

  it("lengths never exceed maxMatch", () => {
    const data = new Uint8Array(1000).fill(65);
    const tokens = encode(data, 4096, 50);
    for (const tok of tokens) {
      expect(tok.length).toBeLessThanOrEqual(50);
    }
  });

  it("lengths below minMatch are not emitted as backreferences", () => {
    const tokens = encode(enc("AABAA"), 4096, 255, 2);
    for (const tok of tokens) {
      expect(tok.length === 0 || tok.length >= 2).toBe(true);
    }
  });
});

// ---- Edge Cases ----

describe("EdgeCases", () => {
  it("single byte encodes as literal", () => {
    const tokens = encode(enc("X"));
    expect(tokens).toHaveLength(1);
    expect(tokens[0]).toEqual(token(0, 0, 88));
  });

  it("exact window boundary match", () => {
    const window = 10;
    const data = new Uint8Array(window + 1).fill(88); // 11 Xs
    const tokens = encode(data, window);
    expect(tokens.some((t) => t.offset > 0)).toBe(true);
    expect(decode(tokens)).toEqual(data);
  });

  it("overlapping match decoded byte-by-byte", () => {
    // [A, B] + (offset=2, length=5, nextChar='Z') → ABABABAZ
    const tokens = [
      token(0, 0, 65), // A
      token(0, 0, 66), // B
      token(2, 5, 90), // overlap → ABABAB, then Z
    ];
    expect(dec(decode(tokens))).toBe("ABABABAZ");
  });

  it("binary data with nulls", () => {
    const data = bytes(0, 0, 0, 255, 255);
    expect(decode(encode(data))).toEqual(data);
  });

  it("very long input round-trips correctly", () => {
    const repeated = enc("Hello, World! ".repeat(100));
    const extra = new Uint8Array(500).fill(88);
    const data = new Uint8Array([...repeated, ...extra]);
    expect(decode(encode(data))).toEqual(data);
  });

  it("long run of identical bytes compresses well", () => {
    const data = new Uint8Array(10000).fill(65);
    const tokens = encode(data);
    // ~41 tokens for 10000 As: 1 literal + ~39 matches of 255 + 1 partial.
    expect(tokens.length).toBeLessThan(50);
    expect(decode(tokens)).toEqual(data);
  });

  it("initial buffer seed is used in decode", () => {
    // Seed [A, B] and apply backreference → ABABAZ.
    const tokens = [token(2, 3, 90)];
    const result = decode(tokens, bytes(65, 66));
    expect(dec(result)).toBe("ABABAZ");
  });
});

// ---- Serialisation Tests ----

describe("Serialisation", () => {
  it("serialised format is 4 + N*4 bytes", () => {
    const tokens = [token(0, 0, 65), token(2, 5, 66)];
    const serialised = serialiseTokens(tokens);
    expect(serialised.length).toBe(4 + 2 * 4);
  });

  it("serialise/deserialise is a no-op", () => {
    const tokens = [token(0, 0, 65), token(1, 3, 66), token(2, 5, 67)];
    const serialised = serialiseTokens(tokens);
    const got = deserialiseTokens(serialised);
    expect(got).toHaveLength(tokens.length);
    for (let i = 0; i < tokens.length; i++) {
      expect(got[i]).toEqual(tokens[i]);
    }
  });

  it("empty data deserialises to empty array", () => {
    expect(deserialiseTokens(new Uint8Array())).toEqual([]);
  });

  it("compress/decompress all spec vectors", () => {
    const vectors = ["", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"];
    for (const s of vectors) {
      const data = enc(s);
      expect(decompress(compress(data))).toEqual(data);
    }
  });
});

// ---- Behaviour Tests ----

describe("Behaviour", () => {
  it("incompressible data does not expand beyond 4N+10 bytes", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    const compressed = compress(data);
    expect(compressed.length).toBeLessThanOrEqual(4 * data.length + 10);
  });

  it("repetitive data compresses significantly", () => {
    const data = enc("ABC".repeat(100));
    const compressed = compress(data);
    expect(compressed.length).toBeLessThan(data.length);
  });

  it("compression is deterministic", () => {
    const data = enc("hello world test");
    expect(compress(data)).toEqual(compress(data));
  });
});
