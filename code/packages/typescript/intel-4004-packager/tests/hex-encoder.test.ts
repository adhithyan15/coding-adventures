import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { decodeHex, encodeHex } from "../src/index.js";

describe("intel-4004-packager", () => {
  it("encodes and decodes a small binary image", () => {
    const binary = new Uint8Array([0xd5, 0xb2, 0x01]);
    const hexText = encodeHex(binary);
    assert.match(hexText, /:03000000D5B20175/);
    const decoded = decodeHex(hexText);
    assert.equal(decoded.origin, 0);
    assert.deepEqual(Array.from(decoded.binary), Array.from(binary));
  });

  it("rejects oversized decoded images", () => {
    const huge = ":0100000000FF\n:0110000000EF\n:00000001FF\n";
    assert.throws(() => decodeHex(huge), /decoded image too large/);
  });
});
