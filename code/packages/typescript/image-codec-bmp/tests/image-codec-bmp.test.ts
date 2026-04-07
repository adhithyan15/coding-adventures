import { describe, it, expect } from "vitest";
import {
  BmpCodec,
  encodeBmp,
  decodeBmp,
} from "../src/index.js";
import {
  createPixelContainer,
  setPixel,
  pixelAt,
  fillPixels,
} from "@coding-adventures/pixel-container";

// Helper: solid-colour container.
function solid(w: number, h: number, r: number, g: number, b: number, a: number) {
  const c = createPixelContainer(w, h);
  fillPixels(c, r, g, b, a);
  return c;
}

// ============================================================================
// Header structure
// ============================================================================

describe("BMP header", () => {
  it("starts with magic BM", () => {
    const bmp = encodeBmp(solid(4, 4, 0, 0, 0, 255));
    expect(bmp[0]).toBe(0x42);
    expect(bmp[1]).toBe(0x4d);
  });

  it("file size matches buffer length", () => {
    const bmp = encodeBmp(solid(4, 4, 0, 0, 0, 255));
    const view = new DataView(bmp.buffer);
    expect(view.getUint32(2, true)).toBe(bmp.length);
  });

  it("pixel offset is 54", () => {
    const bmp = encodeBmp(solid(2, 2, 0, 0, 0, 255));
    const view = new DataView(bmp.buffer);
    expect(view.getUint32(10, true)).toBe(54);
  });

  it("biHeight is negative (top-down)", () => {
    const bmp = encodeBmp(solid(3, 5, 0, 0, 0, 255));
    const view = new DataView(bmp.buffer);
    expect(view.getInt32(22, true)).toBe(-5);
  });

  it("biBitCount is 32", () => {
    const bmp = encodeBmp(solid(1, 1, 0, 0, 0, 255));
    const view = new DataView(bmp.buffer);
    expect(view.getUint16(28, true)).toBe(32);
  });

  it("total size = 54 + width*height*4", () => {
    const bmp = encodeBmp(solid(4, 4, 0, 0, 0, 255));
    expect(bmp.length).toBe(54 + 4 * 4 * 4);
  });
});

// ============================================================================
// Round-trip
// ============================================================================

describe("BMP round-trip", () => {
  it("preserves solid colour", () => {
    const original = solid(4, 4, 200, 100, 50, 255);
    const decoded  = decodeBmp(encodeBmp(original));
    expect(decoded.width).toBe(4);
    expect(decoded.height).toBe(4);
    expect(decoded.data).toEqual(original.data);
  });

  it("preserves multi-colour image", () => {
    const c = createPixelContainer(2, 2);
    setPixel(c, 0, 0, 255, 0,   0,   255);
    setPixel(c, 1, 0, 0,   255, 0,   255);
    setPixel(c, 0, 1, 0,   0,   255, 255);
    setPixel(c, 1, 1, 128, 128, 128, 255);
    const decoded = decodeBmp(encodeBmp(c));
    for (let y = 0; y < 2; y++) {
      for (let x = 0; x < 2; x++) {
        expect(pixelAt(decoded, x, y)).toEqual(pixelAt(c, x, y));
      }
    }
  });

  it("preserves alpha", () => {
    const c = createPixelContainer(2, 1);
    setPixel(c, 0, 0, 10, 20, 30, 128);
    setPixel(c, 1, 0, 50, 60, 70, 0);
    const decoded = decodeBmp(encodeBmp(c));
    expect(pixelAt(decoded, 0, 0)).toEqual([10, 20, 30, 128]);
    expect(pixelAt(decoded, 1, 0)).toEqual([50, 60, 70, 0]);
  });
});

// ============================================================================
// BGRA swap
// ============================================================================

describe("BMP BGRA swap", () => {
  it("stores R in byte[56], G in byte[57], B in byte[54] for first pixel", () => {
    const c = createPixelContainer(1, 1);
    setPixel(c, 0, 0, 11, 22, 33, 44); // R=11 G=22 B=33 A=44
    const bmp = encodeBmp(c);
    // BMP stores BGRA: B=33 G=22 R=11 A=44
    expect(bmp[54]).toBe(33); // B
    expect(bmp[55]).toBe(22); // G
    expect(bmp[56]).toBe(11); // R
    expect(bmp[57]).toBe(44); // A
  });
});

// ============================================================================
// Decode errors
// ============================================================================

describe("BMP decode errors", () => {
  it("throws on file too short", () => {
    expect(() => decodeBmp(new Uint8Array(10))).toThrow("too short");
  });

  it("throws on invalid magic", () => {
    const bad = new Uint8Array(54);
    bad[0] = 0x50; bad[1] = 0x4e; // "PN" not "BM"
    expect(() => decodeBmp(bad)).toThrow("invalid magic");
  });

  it("throws on unsupported bit depth", () => {
    const bmp = encodeBmp(solid(2, 2, 0, 0, 0, 255));
    const view = new DataView(bmp.buffer);
    view.setUint16(28, 24, true); // change to 24-bit
    expect(() => decodeBmp(bmp)).toThrow("unsupported bit depth");
  });

  it("throws on truncated pixel data", () => {
    const bmp = encodeBmp(solid(4, 4, 0, 0, 0, 255));
    expect(() => decodeBmp(bmp.slice(0, 60))).toThrow("truncated");
  });
});

// ============================================================================
// BmpCodec trait
// ============================================================================

describe("BmpCodec", () => {
  const codec = new BmpCodec();

  it("mimeType is image/bmp", () => {
    expect(codec.mimeType).toBe("image/bmp");
  });

  it("encode/decode round-trips via codec", () => {
    const c = solid(3, 3, 60, 120, 180, 255);
    const decoded = codec.decode(codec.encode(c));
    expect(decoded.data).toEqual(c.data);
  });
});
