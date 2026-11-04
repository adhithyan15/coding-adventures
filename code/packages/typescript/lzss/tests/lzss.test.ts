import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  literal,
  match,
  encode,
  decode,
  compress,
  decompress,
  serialiseTokens,
  deserialiseTokens,
} from "../src/lzss.js";

// ─── Helpers ─────────────────────────────────────────────────────────────────

function enc(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function rt(data: Uint8Array): Uint8Array {
  return decompress(compress(data));
}

function eq(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

// ─── Spec vectors ─────────────────────────────────────────────────────────────

describe("encode — spec vectors", () => {
  it("empty input", () => {
    assert.deepEqual(encode(new Uint8Array()), []);
  });

  it("single byte", () => {
    assert.deepEqual(encode(enc("A")), [literal(65)]);
  });

  it("no repetition", () => {
    const tokens = encode(enc("ABCDE"));
    assert.equal(tokens.length, 5);
    assert.equal(tokens.every((t) => t.kind === "literal"), true);
  });

  it("AABCBBABC", () => {
    const tokens = encode(enc("AABCBBABC"));
    assert.equal(tokens.length, 7);
    assert.deepEqual(tokens[6], match(5, 3));
  });

  it("ABABAB", () => {
    assert.deepEqual(encode(enc("ABABAB")), [literal(65), literal(66), match(2, 4)]);
  });

  it("AAAAAAA (self-referential match)", () => {
    assert.deepEqual(encode(enc("AAAAAAA")), [literal(65), match(1, 6)]);
  });
});

// ─── Encode properties ────────────────────────────────────────────────────────

describe("encode — properties", () => {
  it("match offset >= 1", () => {
    for (const tok of encode(enc("ABABABAB"))) {
      if (tok.kind === "match") assert.ok(tok.offset >= 1);
    }
  });

  it("match length >= min_match", () => {
    for (const tok of encode(enc("ABABABABABAB"))) {
      if (tok.kind === "match") assert.ok(tok.length >= 3);
    }
  });

  it("match offset within window_size", () => {
    const data = enc("ABCABCABCABC");
    for (const tok of encode(data, 4)) {
      if (tok.kind === "match") assert.ok(tok.offset <= 4);
    }
  });

  it("match length within max_match", () => {
    const data = new Uint8Array(100).fill(65);
    for (const tok of encode(data, 4096, 5)) {
      if (tok.kind === "match") assert.ok(tok.length <= 5);
    }
  });

  it("min_match large forces all literals", () => {
    const tokens = encode(enc("ABABAB"), 4096, 255, 100);
    assert.equal(tokens.every((t) => t.kind === "literal"), true);
  });
});

// ─── Decode ───────────────────────────────────────────────────────────────────

describe("decode", () => {
  it("empty", () => {
    assert.deepEqual(decode([], 0), new Uint8Array());
  });

  it("single literal", () => {
    assert.deepEqual(decode([literal(65)], 1), new Uint8Array([65]));
  });

  it("overlapping match — AAAAAAA", () => {
    const result = decode([literal(65), match(1, 6)], 7);
    assert.deepEqual(result, enc("AAAAAAA"));
  });

  it("ABABAB", () => {
    const result = decode([literal(65), literal(66), match(2, 4)], 6);
    assert.deepEqual(result, enc("ABABAB"));
  });

  it("truncates to original_length", () => {
    const result = decode([literal(65), literal(66), literal(67)], 2);
    assert.deepEqual(result, new Uint8Array([65, 66]));
  });

  it("returns all when -1", () => {
    const result = decode([literal(65), literal(66)]);
    assert.deepEqual(result, new Uint8Array([65, 66]));
  });
});

// ─── Round-trip ───────────────────────────────────────────────────────────────

describe("round-trip", () => {
  const cases: [string, Uint8Array][] = [
    ["empty", new Uint8Array()],
    ["single", enc("A")],
    ["no repetition", enc("ABCDE")],
    ["all identical", enc("AAAAAAA")],
    ["ABABAB", enc("ABABAB")],
    ["AABCBBABC", enc("AABCBBABC")],
    ["hello world", enc("hello world")],
    ["ABC×100", enc("ABC".repeat(100))],
    ["full byte range", Uint8Array.from({ length: 256 }, (_, i) => i)],
    ["binary nulls", new Uint8Array([0, 0, 0, 255, 255])],
    ["repeated pattern", new Uint8Array(Array.from({ length: 300 }, (_, i) => i % 3))],
    ["long ABCDEF", enc("ABCDEF".repeat(500))],
    ["all zeros", new Uint8Array(1000)],
  ];

  for (const [name, data] of cases) {
    it(name, () => {
      assert.equal(eq(rt(data), data), true);
    });
  }
});

// ─── Wire format ─────────────────────────────────────────────────────────────

describe("wire format", () => {
  it("stores original_length in header", () => {
    const data = enc("hello");
    const compressed = compress(data);
    const view = new DataView(compressed.buffer);
    assert.equal(view.getUint32(0, false), 5);
  });

  it("compress is deterministic", () => {
    const data = enc("hello world test");
    assert.deepEqual(compress(data), compress(data));
  });

  it("empty compresses to 8-byte header", () => {
    const c = compress(new Uint8Array());
    assert.equal(c.length, 8);
  });

  it("crafted large block_count is safe", () => {
    const bad = new Uint8Array(16);
    const view = new DataView(bad.buffer);
    view.setUint32(4, 0x40000000, false); // 2^30 blocks
    const result = decompress(bad);
    assert.ok(result instanceof Uint8Array);
  });
});

// ─── Compression effectiveness ───────────────────────────────────────────────

describe("compression effectiveness", () => {
  it("repetitive data compresses", () => {
    const data = enc("ABC".repeat(1000));
    assert.ok(compress(data).length < data.length);
  });

  it("all-same byte compresses", () => {
    const data = new Uint8Array(10000).fill(0x42);
    const compressed = compress(data);
    assert.ok(compressed.length < data.length);
    assert.equal(eq(decompress(compressed), data), true);
  });

  it("LZSS much smaller than raw on repetitive data", () => {
    const data = enc("ABCDEF".repeat(500)); // 3000 bytes
    assert.ok(compress(data).length < data.length / 2);
  });
});

// ─── Serialise/deserialise symmetry ──────────────────────────────────────────

describe("serialise/deserialise", () => {
  it("symmetry for mixed tokens", () => {
    const tokens = [literal(65), literal(66), match(2, 4)];
    const bytes = serialiseTokens(tokens, 6);
    const [recovered, origLen] = deserialiseTokens(bytes);
    assert.equal(origLen, 6);
    assert.deepEqual(recovered, tokens);
  });
});
