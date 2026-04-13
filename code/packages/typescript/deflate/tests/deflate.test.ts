import { describe, expect, it } from "vitest";
import { compress, decompress } from "../src/deflate.js";

function roundtrip(data: Uint8Array): void {
  const compressed = compress(data);
  const result = decompress(compressed);
  expect(Array.from(result)).toEqual(Array.from(data));
}

function fromString(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

describe("deflate", () => {
  describe("edge cases", () => {
    it("empty input", () => {
      const compressed = compress(new Uint8Array(0));
      const result = decompress(compressed);
      expect(result.length).toBe(0);
    });

    it("single byte 0x00", () => {
      roundtrip(new Uint8Array([0x00]));
    });

    it("single byte 0xFF", () => {
      roundtrip(new Uint8Array([0xff]));
    });

    it("single byte repeated", () => {
      roundtrip(new Uint8Array(20).fill(65)); // "AAAAAAAAAAAAAAAAAAAA"
    });
  });

  describe("spec examples", () => {
    it("AAABBC — all literals", () => {
      const data = fromString("AAABBC");
      roundtrip(data);
      const compressed = compress(data);
      const view = new DataView(
        compressed.buffer,
        compressed.byteOffset,
        compressed.byteLength
      );
      const distCount = view.getUint16(6, false);
      expect(distCount).toBe(0); // no matches
    });

    it("AABCBBABC — one match", () => {
      const data = fromString("AABCBBABC");
      roundtrip(data);
      const compressed = compress(data);
      const view = new DataView(
        compressed.buffer,
        compressed.byteOffset,
        compressed.byteLength
      );
      const origLen = view.getUint32(0, false);
      expect(origLen).toBe(9);
      const distCount = view.getUint16(6, false);
      expect(distCount).toBeGreaterThan(0); // has match
    });
  });

  describe("match tests", () => {
    it("overlapping match (run encoding)", () => {
      roundtrip(fromString("AAAAAAA"));
      roundtrip(fromString("ABABABABABAB"));
    });

    it("multiple matches", () => {
      roundtrip(fromString("ABCABCABCABC"));
      roundtrip(fromString("hello hello hello world"));
    });

    it("max match length ~255", () => {
      roundtrip(new Uint8Array(300).fill(65));
    });
  });

  describe("data variety", () => {
    it("all 256 byte values", () => {
      const data = new Uint8Array(256);
      for (let i = 0; i < 256; i++) data[i] = i;
      roundtrip(data);
    });

    it("binary data 1000 bytes", () => {
      const data = new Uint8Array(1000);
      for (let i = 0; i < 1000; i++) data[i] = i % 256;
      roundtrip(data);
    });

    it("longer text with repetition", () => {
      const base = fromString("the quick brown fox jumps over the lazy dog ");
      const data = new Uint8Array(base.length * 10);
      for (let i = 0; i < 10; i++) data.set(base, i * base.length);
      roundtrip(data);
    });
  });

  describe("compression ratio", () => {
    it("highly repetitive data compresses to < 50%", () => {
      const base = fromString("ABCABC");
      const data = new Uint8Array(base.length * 100);
      for (let i = 0; i < 100; i++) data.set(base, i * base.length);
      const compressed = compress(data);
      expect(compressed.length).toBeLessThan(data.length * 0.5);
    });
  });

  describe("various match lengths", () => {
    it.each([3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255])(
      "match length %d",
      (length) => {
        const prefix = new Uint8Array(length).fill(65);
        const separator = new Uint8Array([66, 66, 66]);
        const data = new Uint8Array(prefix.length + separator.length + prefix.length);
        data.set(prefix, 0);
        data.set(separator, prefix.length);
        data.set(prefix, prefix.length + separator.length);
        roundtrip(data);
      }
    );
  });

  describe("diverse round-trips", () => {
    it("zeros 100", () => roundtrip(new Uint8Array(100)));
    it("0xFF × 100", () => roundtrip(new Uint8Array(100).fill(0xff)));
    it("alphabet repeated", () => {
      const base = fromString("abcdefghijklmnopqrstuvwxyz");
      const data = new Uint8Array(base.length * 10);
      for (let i = 0; i < 10; i++) data.set(base, i * base.length);
      roundtrip(data);
    });
  });
});
