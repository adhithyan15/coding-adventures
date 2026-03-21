import { describe, it, expect } from "vitest";
import { SparseMemory } from "../src/sparse-memory.js";

describe("SparseMemory", () => {
  const makeSimple = () =>
    new SparseMemory([
      { base: 0x00000000, size: 0x1000, name: "RAM" },
      { base: 0xfff00000, size: 0x1000, name: "ROM", readOnly: true },
    ]);

  describe("construction", () => {
    it("creates regions from config", () => {
      const mem = makeSimple();
      expect(mem.regionCount()).toBe(2);
    });

    it("accepts pre-loaded data", () => {
      const data = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
      const mem = new SparseMemory([
        { base: 0x1000, size: 4, name: "preloaded", data },
      ]);
      expect(mem.readByte(0x1000)).toBe(0xde);
      expect(mem.readByte(0x1003)).toBe(0xef);
    });
  });

  describe("readByte / writeByte", () => {
    it("reads and writes single bytes", () => {
      const mem = makeSimple();
      mem.writeByte(0x100, 42);
      expect(mem.readByte(0x100)).toBe(42);
    });

    it("zero-fills unwritten memory", () => {
      const mem = makeSimple();
      expect(mem.readByte(0x500)).toBe(0);
    });

    it("throws on unmapped address", () => {
      const mem = makeSimple();
      expect(() => mem.readByte(0x80000000)).toThrow("unmapped address");
    });

    it("silently ignores writes to read-only regions", () => {
      const mem = makeSimple();
      mem.loadBytes(0xfff00000, [0xab]);
      mem.writeByte(0xfff00000, 0xff);
      expect(mem.readByte(0xfff00000)).toBe(0xab); // unchanged
    });
  });

  describe("readWord / writeWord", () => {
    it("reads and writes 32-bit words in little-endian", () => {
      const mem = makeSimple();
      mem.writeWord(0x00, 0xdeadbeef);
      expect(mem.readWord(0x00)).toBe(0xdeadbeef >>> 0);
      expect(mem.readByte(0x00)).toBe(0xef); // LSB first
      expect(mem.readByte(0x03)).toBe(0xde); // MSB last
    });

    it("silently ignores word writes to read-only regions", () => {
      const mem = makeSimple();
      mem.loadBytes(0xfff00000, [0x01, 0x02, 0x03, 0x04]);
      mem.writeWord(0xfff00000, 0xffffffff);
      expect(mem.readWord(0xfff00000)).toBe(0x04030201);
    });
  });

  describe("loadBytes", () => {
    it("copies bytes into a region", () => {
      const mem = makeSimple();
      mem.loadBytes(0x00, [0x48, 0x65, 0x6c, 0x6c, 0x6f]);
      expect(mem.readByte(0x00)).toBe(0x48);
      expect(mem.readByte(0x04)).toBe(0x6f);
    });

    it("bypasses readOnly check for initial loading", () => {
      const mem = makeSimple();
      mem.loadBytes(0xfff00000, [0xaa, 0xbb]);
      expect(mem.readByte(0xfff00000)).toBe(0xaa);
      expect(mem.readByte(0xfff00001)).toBe(0xbb);
    });
  });

  describe("dump", () => {
    it("returns a copy of memory contents", () => {
      const mem = makeSimple();
      mem.writeByte(0x00, 0x11);
      mem.writeByte(0x01, 0x22);
      const dumped = mem.dump(0x00, 2);
      expect(dumped).toEqual([0x11, 0x22]);
    });
  });
});
