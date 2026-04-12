import { describe, it, expect } from "vitest";
import {
  compress,
  decompress,
  encodeCodes,
  decodeCodes,
  packCodes,
  unpackCodes,
  BitWriter,
  BitReader,
  CLEAR_CODE,
  STOP_CODE,
  INITIAL_NEXT_CODE,
  INITIAL_CODE_SIZE,
  MAX_CODE_SIZE,
} from "../src/lzw.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toBytes(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

function fromBytes(u: Uint8Array): string {
  return new TextDecoder().decode(u);
}

function roundTrip(data: Uint8Array): Uint8Array {
  return decompress(compress(data));
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

describe("constants", () => {
  it("CLEAR_CODE is 256", () => expect(CLEAR_CODE).toBe(256));
  it("STOP_CODE is 257", () => expect(STOP_CODE).toBe(257));
  it("INITIAL_NEXT_CODE is 258", () => expect(INITIAL_NEXT_CODE).toBe(258));
  it("INITIAL_CODE_SIZE is 9", () => expect(INITIAL_CODE_SIZE).toBe(9));
  it("MAX_CODE_SIZE is 16", () => expect(MAX_CODE_SIZE).toBe(16));
});

// ---------------------------------------------------------------------------
// BitWriter / BitReader
// ---------------------------------------------------------------------------

describe("BitWriter / BitReader", () => {
  it("flushes empty writer to empty bytes", () => {
    const w = new BitWriter();
    w.flush();
    expect(w.bytes().length).toBe(0);
  });

  it("roundtrips a single 9-bit code", () => {
    const w = new BitWriter();
    w.write(256, 9);
    w.flush();
    const r = new BitReader(w.bytes());
    expect(r.read(9)).toBe(256);
  });

  it("roundtrips multiple codes at varying widths", () => {
    const codes = [CLEAR_CODE, 65, 66, 258, STOP_CODE];
    const sizes = [9, 9, 9, 9, 9];

    const w = new BitWriter();
    for (let i = 0; i < codes.length; i++) {
      w.write(codes[i]!, sizes[i]!);
    }
    w.flush();

    const r = new BitReader(w.bytes());
    for (let i = 0; i < codes.length; i++) {
      expect(r.read(sizes[i]!)).toBe(codes[i]!);
    }
  });

  it("throws on exhausted reader", () => {
    const r = new BitReader(new Uint8Array(0));
    expect(() => r.read(9)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// encodeCodes
// ---------------------------------------------------------------------------

describe("encodeCodes", () => {
  it("empty input → [CLEAR, STOP]", () => {
    const [codes, orig] = encodeCodes(new Uint8Array(0));
    expect(orig).toBe(0);
    expect(codes).toEqual([CLEAR_CODE, STOP_CODE]);
  });

  it("single byte → [CLEAR, 65, STOP]", () => {
    const [codes, orig] = encodeCodes(toBytes("A"));
    expect(orig).toBe(1);
    expect(codes[0]).toBe(CLEAR_CODE);
    expect(codes[codes.length - 1]).toBe(STOP_CODE);
    expect(codes).toContain(65);
  });

  it("two distinct → [CLEAR, 65, 66, STOP]", () => {
    const [codes] = encodeCodes(toBytes("AB"));
    expect(codes).toEqual([CLEAR_CODE, 65, 66, STOP_CODE]);
  });

  it("ABABAB → CLEAR, 65, 66, 258, 258, STOP", () => {
    const [codes] = encodeCodes(toBytes("ABABAB"));
    expect(codes).toEqual([CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]);
  });

  it("AAAAAAA → CLEAR, 65, 258, 259, 65, STOP", () => {
    const [codes] = encodeCodes(toBytes("AAAAAAA"));
    expect(codes).toEqual([CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]);
  });
});

// ---------------------------------------------------------------------------
// decodeCodes
// ---------------------------------------------------------------------------

describe("decodeCodes", () => {
  it("CLEAR + STOP → empty", () => {
    expect(decodeCodes([CLEAR_CODE, STOP_CODE])).toEqual([]);
  });

  it("CLEAR, 65, STOP → [65]", () => {
    expect(decodeCodes([CLEAR_CODE, 65, STOP_CODE])).toEqual([65]);
  });

  it("CLEAR, 65, 66, STOP → [65, 66]", () => {
    expect(decodeCodes([CLEAR_CODE, 65, 66, STOP_CODE])).toEqual([65, 66]);
  });

  it("CLEAR, 65, 66, 258, 258, STOP → ABABAB bytes", () => {
    const result = decodeCodes([CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]);
    expect(new Uint8Array(result)).toEqual(toBytes("ABABAB"));
  });

  it("CLEAR, 65, 258, 259, 65, STOP → AAAAAAA (tricky token)", () => {
    const result = decodeCodes([CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]);
    expect(new Uint8Array(result)).toEqual(toBytes("AAAAAAA"));
  });

  it("CLEAR mid-stream resets dict", () => {
    const result = decodeCodes([CLEAR_CODE, 65, CLEAR_CODE, 66, STOP_CODE]);
    expect(result).toEqual([65, 66]);
  });

  it("invalid code is skipped", () => {
    const result = decodeCodes([CLEAR_CODE, 9999, 65, STOP_CODE]);
    expect(result).toEqual([65]);
  });
});

// ---------------------------------------------------------------------------
// packCodes / unpackCodes
// ---------------------------------------------------------------------------

describe("packCodes / unpackCodes", () => {
  it("header stores original_length big-endian", () => {
    const packed = packCodes([CLEAR_CODE, STOP_CODE], 42);
    const view = new DataView(packed.buffer, packed.byteOffset);
    expect(view.getUint32(0, false)).toBe(42);
  });

  it("roundtrips empty codes", () => {
    const codes = [CLEAR_CODE, STOP_CODE];
    const packed = packCodes(codes, 0);
    const [unpacked, orig] = unpackCodes(packed);
    expect(orig).toBe(0);
    expect(unpacked).toContain(CLEAR_CODE);
    expect(unpacked).toContain(STOP_CODE);
  });

  it("roundtrips ABABAB codes", () => {
    const codes = [CLEAR_CODE, 65, 66, 258, 258, STOP_CODE];
    const packed = packCodes(codes, 6);
    const [unpacked, orig] = unpackCodes(packed);
    expect(orig).toBe(6);
    expect(unpacked).toEqual(codes);
  });

  it("roundtrips AAAAAAA codes", () => {
    const codes = [CLEAR_CODE, 65, 258, 259, 65, STOP_CODE];
    const packed = packCodes(codes, 7);
    const [unpacked, orig] = unpackCodes(packed);
    expect(orig).toBe(7);
    expect(unpacked).toEqual(codes);
  });

  it("handles truncated input gracefully", () => {
    const [codes, orig] = unpackCodes(new Uint8Array([0, 0]));
    expect(Array.isArray(codes)).toBe(true);
    expect(typeof orig).toBe("number");
  });
});

// ---------------------------------------------------------------------------
// compress / decompress
// ---------------------------------------------------------------------------

describe("compress / decompress", () => {
  it("empty", () => {
    expect(roundTrip(new Uint8Array(0))).toEqual(new Uint8Array(0));
  });

  it("single byte", () => {
    expect(fromBytes(roundTrip(toBytes("A")))).toBe("A");
  });

  it("two distinct bytes", () => {
    expect(fromBytes(roundTrip(toBytes("AB")))).toBe("AB");
  });

  it("ABABAB", () => {
    expect(fromBytes(roundTrip(toBytes("ABABAB")))).toBe("ABABAB");
  });

  it("AAAAAAA (tricky token)", () => {
    expect(fromBytes(roundTrip(toBytes("AAAAAAA")))).toBe("AAAAAAA");
  });

  it("AABABC", () => {
    expect(fromBytes(roundTrip(toBytes("AABABC")))).toBe("AABABC");
  });

  it("long repetitive string", () => {
    const text = "the quick brown fox jumps over the lazy dog ".repeat(20);
    expect(fromBytes(roundTrip(toBytes(text)))).toBe(text);
  });

  it("binary data", () => {
    const data = new Uint8Array(512);
    for (let i = 0; i < data.length; i++) data[i] = i % 256;
    const result = roundTrip(data);
    expect(result).toEqual(data);
  });

  it("all zeros", () => {
    const data = new Uint8Array(100);
    expect(roundTrip(data)).toEqual(data);
  });

  it("all 0xFF", () => {
    const data = new Uint8Array(100).fill(0xff);
    expect(roundTrip(data)).toEqual(data);
  });

  it("compresses repetitive data", () => {
    const data = toBytes("ABCABC".repeat(100));
    const compressed = compress(data);
    expect(compressed.length).toBeLessThan(data.length);
  });

  it("header contains original_length", () => {
    const data = toBytes("hello world");
    const compressed = compress(data);
    const view = new DataView(compressed.buffer, compressed.byteOffset);
    expect(view.getUint32(0, false)).toBe(data.length);
  });
});
