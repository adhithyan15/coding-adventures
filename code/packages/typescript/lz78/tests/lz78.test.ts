import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  type Token,
  compress,
  decode,
  decompress,
  deserialiseTokens,
  encode,
  serialiseTokens,
} from "../src/lz78.js";

// Helper: ASCII string → Uint8Array
const enc = (s: string): Uint8Array => new TextEncoder().encode(s);
// Helper: Uint8Array → ASCII string
const str = (b: Uint8Array): string => new TextDecoder().decode(b);
// Helper: round-trip
const rt = (s: string): string => str(decompress(compress(enc(s))));

// ─── Spec vectors ─────────────────────────────────────────────────────────────

describe("Spec vectors", () => {
  it("empty input", () => {
    assert.deepEqual(encode(new Uint8Array()), []);
    assert.deepEqual(decode([], 0), new Uint8Array());
  });

  it("single byte", () => {
    const tokens = encode(enc("A"));
    assert.deepEqual(tokens, [{ dictIndex: 0, nextChar: 65 }]);
    assert.deepEqual(decode(tokens, 1), enc("A"));
  });

  it("no repetition — all literals", () => {
    const tokens = encode(enc("ABCDE"));
    assert.equal(tokens.length, 5);
    for (const t of tokens) assert.equal(t.dictIndex, 0);
  });

  it("AABCBBABC", () => {
    const want: Token[] = [
      { dictIndex: 0, nextChar: 65 },
      { dictIndex: 1, nextChar: 66 },
      { dictIndex: 0, nextChar: 67 },
      { dictIndex: 0, nextChar: 66 },
      { dictIndex: 4, nextChar: 65 },
      { dictIndex: 4, nextChar: 67 },
    ];
    assert.deepEqual(encode(enc("AABCBBABC")), want);
    assert.equal(rt("AABCBBABC"), "AABCBBABC");
  });

  it("ABABAB — flush token", () => {
    const want: Token[] = [
      { dictIndex: 0, nextChar: 65 },
      { dictIndex: 0, nextChar: 66 },
      { dictIndex: 1, nextChar: 66 },
      { dictIndex: 3, nextChar: 0 },
    ];
    assert.deepEqual(encode(enc("ABABAB")), want);
    assert.equal(rt("ABABAB"), "ABABAB");
  });

  it("all identical bytes — AAAAAAA", () => {
    const tokens = encode(enc("AAAAAAA"));
    assert.equal(tokens.length, 4);
    assert.deepEqual(tokens[0], { dictIndex: 0, nextChar: 65 });
    assert.deepEqual(tokens[1], { dictIndex: 1, nextChar: 65 });
    assert.deepEqual(tokens[2], { dictIndex: 2, nextChar: 65 });
    assert.deepEqual(tokens[3], { dictIndex: 1, nextChar: 0 });
  });

  it("repeated pair ABABABAB compresses", () => {
    assert.equal(rt("ABABABAB"), "ABABABAB");
    assert.ok(encode(enc("ABABABAB")).length < 8);
  });
});

// ─── Round-trip tests ─────────────────────────────────────────────────────────

describe("Round-trip", () => {
  const cases = [
    "",
    "A",
    "ABCDE",
    "AAAAAAA",
    "ABABABAB",
    "AABCBBABC",
    "hello world",
    "the quick brown fox",
    "ababababab",
    "aaaaaaaaaa",
  ];

  for (const s of cases) {
    it(`ascii: ${JSON.stringify(s)}`, () => {
      assert.equal(rt(s), s);
    });
  }

  it("binary zeros", () => {
    const d = new Uint8Array(3);
    assert.deepEqual(decompress(compress(d)), d);
  });

  it("binary 255s", () => {
    const d = new Uint8Array([255, 255, 255]);
    assert.deepEqual(decompress(compress(d)), d);
  });

  it("full byte range", () => {
    const d = new Uint8Array(256).map((_, i) => i);
    assert.deepEqual(decompress(compress(d)), d);
  });

  it("binary repeat", () => {
    const d = new Uint8Array([0, 1, 2, 0, 1, 2]);
    assert.deepEqual(decompress(compress(d)), d);
  });

  it("binary nulls mix", () => {
    const d = new Uint8Array([0, 0, 0, 255, 255]);
    assert.deepEqual(decompress(compress(d)), d);
  });
});

// ─── Parameter tests ──────────────────────────────────────────────────────────

describe("Parameters", () => {
  it("maxDictSize respected", () => {
    const tokens = encode(enc("ABCABCABCABCABC"), 10);
    for (const t of tokens) assert.ok(t.dictIndex < 10);
  });

  it("maxDictSize=1 → all literals", () => {
    const tokens = encode(enc("AAAA"), 1);
    for (const t of tokens) assert.equal(t.dictIndex, 0);
  });
});

// ─── Edge cases ───────────────────────────────────────────────────────────────

describe("Edge cases", () => {
  it("single byte literal", () => {
    assert.deepEqual(encode(enc("X")), [{ dictIndex: 0, nextChar: 88 }]);
  });

  it("two bytes", () => {
    assert.deepEqual(
      decode([{ dictIndex: 0, nextChar: 65 }, { dictIndex: 0, nextChar: 66 }]),
      enc("AB"),
    );
  });

  it("flush token round-trip", () => {
    assert.equal(rt("ABABAB"), "ABABAB");
  });

  it("all null bytes", () => {
    const d = new Uint8Array(100);
    assert.deepEqual(decompress(compress(d)), d);
  });

  it("all max bytes", () => {
    const d = new Uint8Array(100).fill(255);
    assert.deepEqual(decompress(compress(d)), d);
  });

  it("very long input", () => {
    const chunk = enc("Hello, World! ");
    const parts: Uint8Array[] = [];
    for (let i = 0; i < 100; i++) parts.push(chunk);
    parts.push(new Uint8Array(256).map((_, i) => i));
    const data = new Uint8Array(parts.reduce((a, p) => a + p.length, 0));
    let off = 0;
    for (const p of parts) { data.set(p, off); off += p.length; }
    assert.deepEqual(decompress(compress(data)), data);
  });
});

// ─── Serialisation tests ──────────────────────────────────────────────────────

describe("Serialisation", () => {
  it("format size: 8 + 4×tokens", () => {
    const compressed = compress(enc("AB"));
    const tokens = encode(enc("AB"));
    assert.equal(compressed.length, 8 + tokens.length * 4);
  });

  it("deserialise round-trip", () => {
    const tokens: Token[] = [
      { dictIndex: 0, nextChar: 65 },
      { dictIndex: 1, nextChar: 66 },
    ];
    const [got] = deserialiseTokens(serialiseTokens(tokens, 3));
    assert.deepEqual(got, tokens);
  });

  it("all spec vectors", () => {
    for (const v of ["", "A", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"]) {
      assert.equal(rt(v), v);
    }
  });

  it("deterministic", () => {
    const d = enc("hello world test data repeated");
    assert.deepEqual(compress(d), compress(d));
  });
});

// ─── Behaviour tests ──────────────────────────────────────────────────────────

describe("Behaviour", () => {
  it("repetitive data compresses", () => {
    const d = enc("ABC".repeat(1000));
    assert.ok(compress(d).length < d.length);
  });

  it("incompressible data does not expand excessively", () => {
    const d = new Uint8Array(256).map((_, i) => i);
    assert.ok(compress(d).length <= 4 * d.length + 10);
  });

  it("all-same-byte compresses", () => {
    const d = new Uint8Array(10000).fill(65);
    assert.ok(compress(d).length < d.length);
    assert.deepEqual(decompress(compress(d)), d);
  });
});
