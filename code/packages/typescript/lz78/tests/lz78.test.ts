import { describe, expect, it } from "vitest";
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
    expect(encode(new Uint8Array())).toEqual([]);
    expect(decode([], 0)).toEqual(new Uint8Array());
  });

  it("single byte", () => {
    const tokens = encode(enc("A"));
    expect(tokens).toEqual([{ dictIndex: 0, nextChar: 65 }]);
    expect(decode(tokens, 1)).toEqual(enc("A"));
  });

  it("no repetition — all literals", () => {
    const tokens = encode(enc("ABCDE"));
    expect(tokens.length).toBe(5);
    for (const t of tokens) expect(t.dictIndex).toBe(0);
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
    expect(encode(enc("AABCBBABC"))).toEqual(want);
    expect(rt("AABCBBABC")).toBe("AABCBBABC");
  });

  it("ABABAB — flush token", () => {
    const want: Token[] = [
      { dictIndex: 0, nextChar: 65 },
      { dictIndex: 0, nextChar: 66 },
      { dictIndex: 1, nextChar: 66 },
      { dictIndex: 3, nextChar: 0 },
    ];
    expect(encode(enc("ABABAB"))).toEqual(want);
    expect(rt("ABABAB")).toBe("ABABAB");
  });

  it("all identical bytes — AAAAAAA", () => {
    const tokens = encode(enc("AAAAAAA"));
    expect(tokens.length).toBe(4);
    expect(tokens[0]).toEqual({ dictIndex: 0, nextChar: 65 });
    expect(tokens[1]).toEqual({ dictIndex: 1, nextChar: 65 });
    expect(tokens[2]).toEqual({ dictIndex: 2, nextChar: 65 });
    expect(tokens[3]).toEqual({ dictIndex: 1, nextChar: 0 });
  });

  it("repeated pair ABABABAB compresses", () => {
    expect(rt("ABABABAB")).toBe("ABABABAB");
    expect(encode(enc("ABABABAB")).length).toBeLessThan(8);
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
      expect(rt(s)).toBe(s);
    });
  }

  it("binary zeros", () => {
    const d = new Uint8Array(3);
    expect(decompress(compress(d))).toEqual(d);
  });

  it("binary 255s", () => {
    const d = new Uint8Array([255, 255, 255]);
    expect(decompress(compress(d))).toEqual(d);
  });

  it("full byte range", () => {
    const d = new Uint8Array(256).map((_, i) => i);
    expect(decompress(compress(d))).toEqual(d);
  });

  it("binary repeat", () => {
    const d = new Uint8Array([0, 1, 2, 0, 1, 2]);
    expect(decompress(compress(d))).toEqual(d);
  });

  it("binary nulls mix", () => {
    const d = new Uint8Array([0, 0, 0, 255, 255]);
    expect(decompress(compress(d))).toEqual(d);
  });
});

// ─── Parameter tests ──────────────────────────────────────────────────────────

describe("Parameters", () => {
  it("maxDictSize respected", () => {
    const tokens = encode(enc("ABCABCABCABCABC"), 10);
    for (const t of tokens) expect(t.dictIndex).toBeLessThan(10);
  });

  it("maxDictSize=1 → all literals", () => {
    const tokens = encode(enc("AAAA"), 1);
    for (const t of tokens) expect(t.dictIndex).toBe(0);
  });
});

// ─── Edge cases ───────────────────────────────────────────────────────────────

describe("Edge cases", () => {
  it("single byte literal", () => {
    expect(encode(enc("X"))).toEqual([{ dictIndex: 0, nextChar: 88 }]);
  });

  it("two bytes", () => {
    expect(decode([{ dictIndex: 0, nextChar: 65 }, { dictIndex: 0, nextChar: 66 }]))
      .toEqual(enc("AB"));
  });

  it("flush token round-trip", () => {
    expect(rt("ABABAB")).toBe("ABABAB");
  });

  it("all null bytes", () => {
    const d = new Uint8Array(100);
    expect(decompress(compress(d))).toEqual(d);
  });

  it("all max bytes", () => {
    const d = new Uint8Array(100).fill(255);
    expect(decompress(compress(d))).toEqual(d);
  });

  it("very long input", () => {
    const chunk = enc("Hello, World! ");
    const parts: Uint8Array[] = [];
    for (let i = 0; i < 100; i++) parts.push(chunk);
    parts.push(new Uint8Array(256).map((_, i) => i));
    const data = new Uint8Array(parts.reduce((a, p) => a + p.length, 0));
    let off = 0;
    for (const p of parts) { data.set(p, off); off += p.length; }
    expect(decompress(compress(data))).toEqual(data);
  });
});

// ─── Serialisation tests ──────────────────────────────────────────────────────

describe("Serialisation", () => {
  it("format size: 8 + 4×tokens", () => {
    const compressed = compress(enc("AB"));
    const tokens = encode(enc("AB"));
    expect(compressed.length).toBe(8 + tokens.length * 4);
  });

  it("deserialise round-trip", () => {
    const tokens: Token[] = [
      { dictIndex: 0, nextChar: 65 },
      { dictIndex: 1, nextChar: 66 },
    ];
    const [got] = deserialiseTokens(serialiseTokens(tokens, 3));
    expect(got).toEqual(tokens);
  });

  it("all spec vectors", () => {
    for (const v of ["", "A", "ABCDE", "AAAAAAA", "ABABABAB", "AABCBBABC"]) {
      expect(rt(v)).toBe(v);
    }
  });

  it("deterministic", () => {
    const d = enc("hello world test data repeated");
    expect(compress(d)).toEqual(compress(d));
  });
});

// ─── Behaviour tests ──────────────────────────────────────────────────────────

describe("Behaviour", () => {
  it("repetitive data compresses", () => {
    const d = enc("ABC".repeat(1000));
    expect(compress(d).length).toBeLessThan(d.length);
  });

  it("incompressible data does not expand excessively", () => {
    const d = new Uint8Array(256).map((_, i) => i);
    expect(compress(d).length).toBeLessThanOrEqual(4 * d.length + 10);
  });

  it("all-same-byte compresses", () => {
    const d = new Uint8Array(10000).fill(65);
    expect(compress(d).length).toBeLessThan(d.length);
    expect(decompress(compress(d))).toEqual(d);
  });
});
