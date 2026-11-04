import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
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
  it("CLEAR_CODE is 256", () => assert.equal(CLEAR_CODE, 256));
  it("STOP_CODE is 257", () => assert.equal(STOP_CODE, 257));
  it("INITIAL_NEXT_CODE is 258", () => assert.equal(INITIAL_NEXT_CODE, 258));
  it("INITIAL_CODE_SIZE is 9", () => assert.equal(INITIAL_CODE_SIZE, 9));
  it("MAX_CODE_SIZE is 16", () => assert.equal(MAX_CODE_SIZE, 16));
});

// ---------------------------------------------------------------------------
// BitWriter / BitReader
// ---------------------------------------------------------------------------

describe("BitWriter / BitReader", () => {
  it("flushes empty writer to empty bytes", () => {
    const w = new BitWriter();
    w.flush();
    assert.equal(w.bytes().length, 0);
  });

  it("roundtrips a single 9-bit code", () => {
    const w = new BitWriter();
    w.write(256, 9);
    w.flush();
    const r = new BitReader(w.bytes());
    assert.equal(r.read(9), 256);
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
      assert.equal(r.read(sizes[i]!), codes[i]!);
    }
  });

  it("throws on exhausted reader", () => {
    const r = new BitReader(new Uint8Array(0));
    assert.throws(() => r.read(9));
  });
});

// ---------------------------------------------------------------------------
// encodeCodes
// ---------------------------------------------------------------------------

describe("encodeCodes", () => {
  it("empty input → [CLEAR, STOP]", () => {
    const [codes, orig] = encodeCodes(new Uint8Array(0));
    assert.equal(orig, 0);
    assert.deepEqual(codes, [CLEAR_CODE, STOP_CODE]);
  });

  it("single byte → [CLEAR, 65, STOP]", () => {
    const [codes, orig] = encodeCodes(toBytes("A"));
    assert.equal(orig, 1);
    assert.equal(codes[0], CLEAR_CODE);
    assert.equal(codes[codes.length - 1], STOP_CODE);
    assert.ok(codes.includes(65));
  });

  it("two distinct → [CLEAR, 65, 66, STOP]", () => {
    const [codes] = encodeCodes(toBytes("AB"));
    assert.deepEqual(codes, [CLEAR_CODE, 65, 66, STOP_CODE]);
  });

  it("ABABAB → CLEAR, 65, 66, 258, 258, STOP", () => {
    const [codes] = encodeCodes(toBytes("ABABAB"));
    assert.deepEqual(codes, [CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]);
  });

  it("AAAAAAA → CLEAR, 65, 258, 259, 65, STOP", () => {
    const [codes] = encodeCodes(toBytes("AAAAAAA"));
    assert.deepEqual(codes, [CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]);
  });
});

// ---------------------------------------------------------------------------
// decodeCodes
// ---------------------------------------------------------------------------

describe("decodeCodes", () => {
  it("CLEAR + STOP → empty", () => {
    assert.deepEqual(decodeCodes([CLEAR_CODE, STOP_CODE]), []);
  });

  it("CLEAR, 65, STOP → [65]", () => {
    assert.deepEqual(decodeCodes([CLEAR_CODE, 65, STOP_CODE]), [65]);
  });

  it("CLEAR, 65, 66, STOP → [65, 66]", () => {
    assert.deepEqual(decodeCodes([CLEAR_CODE, 65, 66, STOP_CODE]), [65, 66]);
  });

  it("CLEAR, 65, 66, 258, 258, STOP → ABABAB bytes", () => {
    const result = decodeCodes([CLEAR_CODE, 65, 66, 258, 258, STOP_CODE]);
    assert.deepEqual(new Uint8Array(result), toBytes("ABABAB"));
  });

  it("CLEAR, 65, 258, 259, 65, STOP → AAAAAAA (tricky token)", () => {
    const result = decodeCodes([CLEAR_CODE, 65, 258, 259, 65, STOP_CODE]);
    assert.deepEqual(new Uint8Array(result), toBytes("AAAAAAA"));
  });

  it("CLEAR mid-stream resets dict", () => {
    const result = decodeCodes([CLEAR_CODE, 65, CLEAR_CODE, 66, STOP_CODE]);
    assert.deepEqual(result, [65, 66]);
  });

  it("invalid code is skipped", () => {
    const result = decodeCodes([CLEAR_CODE, 9999, 65, STOP_CODE]);
    assert.deepEqual(result, [65]);
  });
});

// ---------------------------------------------------------------------------
// packCodes / unpackCodes
// ---------------------------------------------------------------------------

describe("packCodes / unpackCodes", () => {
  it("header stores original_length big-endian", () => {
    const packed = packCodes([CLEAR_CODE, STOP_CODE], 42);
    const view = new DataView(packed.buffer, packed.byteOffset);
    assert.equal(view.getUint32(0, false), 42);
  });

  it("roundtrips empty codes", () => {
    const codes = [CLEAR_CODE, STOP_CODE];
    const packed = packCodes(codes, 0);
    const [unpacked, orig] = unpackCodes(packed);
    assert.equal(orig, 0);
    assert.ok(unpacked.includes(CLEAR_CODE));
    assert.ok(unpacked.includes(STOP_CODE));
  });

  it("roundtrips ABABAB codes", () => {
    const codes = [CLEAR_CODE, 65, 66, 258, 258, STOP_CODE];
    const packed = packCodes(codes, 6);
    const [unpacked, orig] = unpackCodes(packed);
    assert.equal(orig, 6);
    assert.deepEqual(unpacked, codes);
  });

  it("roundtrips AAAAAAA codes", () => {
    const codes = [CLEAR_CODE, 65, 258, 259, 65, STOP_CODE];
    const packed = packCodes(codes, 7);
    const [unpacked, orig] = unpackCodes(packed);
    assert.equal(orig, 7);
    assert.deepEqual(unpacked, codes);
  });

  it("handles truncated input gracefully", () => {
    const [codes, orig] = unpackCodes(new Uint8Array([0, 0]));
    assert.ok(Array.isArray(codes));
    assert.equal(typeof orig, "number");
  });
});

// ---------------------------------------------------------------------------
// compress / decompress
// ---------------------------------------------------------------------------

describe("compress / decompress", () => {
  it("empty", () => {
    assert.deepEqual(roundTrip(new Uint8Array(0)), new Uint8Array(0));
  });

  it("single byte", () => {
    assert.equal(fromBytes(roundTrip(toBytes("A"))), "A");
  });

  it("two distinct bytes", () => {
    assert.equal(fromBytes(roundTrip(toBytes("AB"))), "AB");
  });

  it("ABABAB", () => {
    assert.equal(fromBytes(roundTrip(toBytes("ABABAB"))), "ABABAB");
  });

  it("AAAAAAA (tricky token)", () => {
    assert.equal(fromBytes(roundTrip(toBytes("AAAAAAA"))), "AAAAAAA");
  });

  it("AABABC", () => {
    assert.equal(fromBytes(roundTrip(toBytes("AABABC"))), "AABABC");
  });

  it("long repetitive string", () => {
    const text = "the quick brown fox jumps over the lazy dog ".repeat(20);
    assert.equal(fromBytes(roundTrip(toBytes(text))), text);
  });

  it("binary data", () => {
    const data = new Uint8Array(512);
    for (let i = 0; i < data.length; i++) data[i] = i % 256;
    const result = roundTrip(data);
    assert.deepEqual(result, data);
  });

  it("all zeros", () => {
    const data = new Uint8Array(100);
    assert.deepEqual(roundTrip(data), data);
  });

  it("all 0xFF", () => {
    const data = new Uint8Array(100).fill(0xff);
    assert.deepEqual(roundTrip(data), data);
  });

  it("compresses repetitive data", () => {
    const data = toBytes("ABCABC".repeat(100));
    const compressed = compress(data);
    assert.ok(compressed.length < data.length);
  });

  it("header contains original_length", () => {
    const data = toBytes("hello world");
    const compressed = compress(data);
    const view = new DataView(compressed.buffer, compressed.byteOffset);
    assert.equal(view.getUint32(0, false), data.length);
  });
});
