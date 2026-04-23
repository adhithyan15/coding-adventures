import { describe, expect, it } from "vitest";

import { decodeHex, encodeHex } from "../src/index.js";

describe("intel-4004-packager", () => {
  it("encodes and decodes a small binary image", () => {
    const binary = new Uint8Array([0xd5, 0xb2, 0x01]);
    const hexText = encodeHex(binary);
    expect(hexText).toContain(":03000000D5B20175");
    const decoded = decodeHex(hexText);
    expect(decoded.origin).toBe(0);
    expect(Array.from(decoded.binary)).toEqual(Array.from(binary));
  });

  it("rejects oversized decoded images", () => {
    const huge = ":0100000000FF\n:0110000000EF\n:00000001FF\n";
    expect(() => decodeHex(huge)).toThrow(/decoded image too large/);
  });
});
