/**
 * Tests for the LZ77 compression implementation.
 *
 * Test vectors come from the CMP00 specification. Covers: literals,
 * backreferences, overlapping matches, edge cases, and round-trip invariants.
 * Coverage target: 95%+.
 */

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
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
    assert.deepEqual(encode(new Uint8Array()), []);
    assert.deepEqual(decode([]), new Uint8Array());
  });

  it("no repetition → all literal tokens", () => {
    const tokens = encode(enc("ABCDE"));
    assert.equal(tokens.length, 5);
    for (const tok of tokens) {
      assert.equal(tok.offset, 0);
      assert.equal(tok.length, 0);
    }
  });

  it("all identical bytes exploit overlap mechanism", () => {
    // "AAAAAAA" → literal A + backreference (offset=1, length=5, nextChar=A).
    const tokens = encode(enc("AAAAAAA"));
    assert.equal(tokens.length, 2);
    assert.deepEqual(tokens[0], token(0, 0, 65));
    assert.equal(tokens[1]!.offset, 1);
    assert.equal(tokens[1]!.length, 5);
    assert.equal(tokens[1]!.nextChar, 65);

    assert.equal(dec(decode(tokens)), "AAAAAAA");
  });

  it("repeated pair uses backreference", () => {
    // "ABABABAB" → [A literal, B literal, (offset=2, length=5, nextChar=B)].
    const tokens = encode(enc("ABABABAB"));
    assert.equal(tokens.length, 3);
    assert.deepEqual(tokens[0], token(0, 0, 65));
    assert.deepEqual(tokens[1], token(0, 0, 66));
    assert.equal(tokens[2]!.offset, 2);
    assert.equal(tokens[2]!.length, 5);
    assert.equal(tokens[2]!.nextChar, 66);

    assert.equal(dec(decode(tokens)), "ABABABAB");
  });

  it("AABCBBABC with min_match=3 → all literals", () => {
    const tokens = encode(enc("AABCBBABC"));
    assert.equal(tokens.length, 9);
    for (const tok of tokens) {
      assert.equal(tok.offset, 0);
      assert.equal(tok.length, 0);
    }
    assert.equal(dec(decode(tokens)), "AABCBBABC");
  });

  it("AABCBBABC with min_match=2 → round-trip holds", () => {
    const tokens = encode(enc("AABCBBABC"), 4096, 255, 2);
    assert.equal(dec(decode(tokens)), "AABCBBABC");
  });
});

// ---- Round-Trip Invariant Tests ----

describe("RoundTrip", () => {
  it("empty", () => {
    assert.deepEqual(decode(encode(new Uint8Array())), new Uint8Array());
  });

  it("single bytes", () => {
    for (const b of [65, 0, 255]) {
      assert.deepEqual(decode(encode(bytes(b))), bytes(b));
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
      assert.equal(dec(decode(encode(enc(s)))), s);
    });
  }

  it("null bytes round-trip", () => {
    const data = bytes(0, 0, 0);
    assert.deepEqual(decode(encode(data)), data);
  });

  it("0xFF bytes round-trip", () => {
    const data = bytes(255, 255, 255);
    assert.deepEqual(decode(encode(data)), data);
  });

  it("all 256 byte values round-trip", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    assert.deepEqual(decode(encode(data)), data);
  });

  it("compress/decompress round-trip", () => {
    const cases = ["", "A", "ABCDE", "AAAAAAA", "ABABABAB", "hello world"];
    for (const s of cases) {
      const data = enc(s);
      assert.deepEqual(decompress(compress(data)), data);
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
      assert.ok(tok.offset <= 100);
    }
  });

  it("lengths never exceed maxMatch", () => {
    const data = new Uint8Array(1000).fill(65);
    const tokens = encode(data, 4096, 50);
    for (const tok of tokens) {
      assert.ok(tok.length <= 50);
    }
  });

  it("lengths below minMatch are not emitted as backreferences", () => {
    const tokens = encode(enc("AABAA"), 4096, 255, 2);
    for (const tok of tokens) {
      assert.ok(tok.length === 0 || tok.length >= 2);
    }
  });
});

// ---- Edge Cases ----

describe("EdgeCases", () => {
  it("single byte encodes as literal", () => {
    const tokens = encode(enc("X"));
    assert.equal(tokens.length, 1);
    assert.deepEqual(tokens[0], token(0, 0, 88));
  });

  it("exact window boundary match", () => {
    const window = 10;
    const data = new Uint8Array(window + 1).fill(88); // 11 Xs
    const tokens = encode(data, window);
    assert.ok(tokens.some((t) => t.offset > 0));
    assert.deepEqual(decode(tokens), data);
  });

  it("overlapping match decoded byte-by-byte", () => {
    // [A, B] + (offset=2, length=5, nextChar='Z') → ABABABAZ
    const tokens = [
      token(0, 0, 65), // A
      token(0, 0, 66), // B
      token(2, 5, 90), // overlap → ABABAB, then Z
    ];
    assert.equal(dec(decode(tokens)), "ABABABAZ");
  });

  it("binary data with nulls", () => {
    const data = bytes(0, 0, 0, 255, 255);
    assert.deepEqual(decode(encode(data)), data);
  });

  it("very long input round-trips correctly", () => {
    const repeated = enc("Hello, World! ".repeat(100));
    const extra = new Uint8Array(500).fill(88);
    const data = new Uint8Array([...repeated, ...extra]);
    assert.deepEqual(decode(encode(data)), data);
  });

  it("long run of identical bytes compresses well", () => {
    const data = new Uint8Array(10000).fill(65);
    const tokens = encode(data);
    // ~41 tokens for 10000 As: 1 literal + ~39 matches of 255 + 1 partial.
    assert.ok(tokens.length < 50);
    assert.deepEqual(decode(tokens), data);
  });

  it("initial buffer seed is used in decode", () => {
    // Seed [A, B] and apply backreference → ABABAZ.
    const tokens = [token(2, 3, 90)];
    const result = decode(tokens, bytes(65, 66));
    assert.equal(dec(result), "ABABAZ");
  });
});

// ---- Serialisation Tests ----

describe("Serialisation", () => {
  it("serialised format is 4 + N*4 bytes", () => {
    const tokens = [token(0, 0, 65), token(2, 5, 66)];
    const serialised = serialiseTokens(tokens);
    assert.equal(serialised.length, 4 + 2 * 4);
  });

  it("serialise/deserialise is a no-op", () => {
    const tokens = [token(0, 0, 65), token(1, 3, 66), token(2, 5, 67)];
    const serialised = serialiseTokens(tokens);
    const got = deserialiseTokens(serialised);
    assert.equal(got.length, tokens.length);
    for (let i = 0; i < tokens.length; i++) {
      assert.deepEqual(got[i], tokens[i]);
    }
  });

  it("empty data deserialises to empty array", () => {
    assert.deepEqual(deserialiseTokens(new Uint8Array()), []);
  });

  it("compress/decompress all spec vectors", () => {
    const vectors = ["", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"];
    for (const s of vectors) {
      const data = enc(s);
      assert.deepEqual(decompress(compress(data)), data);
    }
  });
});

// ---- Behaviour Tests ----

describe("Behaviour", () => {
  it("incompressible data does not expand beyond 4N+10 bytes", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    const compressed = compress(data);
    assert.ok(compressed.length <= 4 * data.length + 10);
  });

  it("repetitive data compresses significantly", () => {
    const data = enc("ABC".repeat(100));
    const compressed = compress(data);
    assert.ok(compressed.length < data.length);
  });

  it("compression is deterministic", () => {
    const data = enc("hello world test");
    assert.deepEqual(compress(data), compress(data));
  });
});
