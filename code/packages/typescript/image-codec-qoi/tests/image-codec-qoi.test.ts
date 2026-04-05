import { describe, it, expect } from "vitest";
import {
  QoiCodec,
  encodeQoi,
  decodeQoi,
} from "../src/index.js";
import {
  createPixelContainer,
  setPixel,
  pixelAt,
  fillPixels,
} from "@coding-adventures/pixel-container";

function solid(w: number, h: number, r: number, g: number, b: number, a: number) {
  const c = createPixelContainer(w, h);
  fillPixels(c, r, g, b, a);
  return c;
}

// ============================================================================
// Header
// ============================================================================

describe("QOI header", () => {
  it("starts with qoif magic", () => {
    const qoi = encodeQoi(solid(4, 4, 0, 0, 0, 255));
    expect(qoi[0]).toBe(0x71); // q
    expect(qoi[1]).toBe(0x6f); // o
    expect(qoi[2]).toBe(0x69); // i
    expect(qoi[3]).toBe(0x66); // f
  });

  it("encodes width and height as big-endian u32", () => {
    const qoi = encodeQoi(solid(300, 200, 0, 0, 0, 255));
    const view = new DataView(qoi.buffer);
    expect(view.getUint32(4, false)).toBe(300);
    expect(view.getUint32(8, false)).toBe(200);
  });

  it("ends with end marker", () => {
    const qoi = encodeQoi(solid(2, 2, 0, 0, 0, 255));
    const end = qoi.slice(qoi.length - 8);
    expect(Array.from(end)).toEqual([0, 0, 0, 0, 0, 0, 0, 1]);
  });
});

// ============================================================================
// Round-trip — solid colours exercise OP_RUN heavily
// ============================================================================

describe("QOI round-trip", () => {
  it("solid colour round-trips correctly", () => {
    const c = solid(8, 8, 100, 150, 200, 255);
    const decoded = decodeQoi(encodeQoi(c));
    expect(decoded.width).toBe(8);
    expect(decoded.height).toBe(8);
    for (let y = 0; y < 8; y++) {
      for (let x = 0; x < 8; x++) {
        expect(pixelAt(decoded, x, y)).toEqual([100, 150, 200, 255]);
      }
    }
  });

  it("gradient image round-trips (exercises DIFF/LUMA/RGB)", () => {
    const c = createPixelContainer(16, 16);
    for (let y = 0; y < 16; y++) {
      for (let x = 0; x < 16; x++) {
        setPixel(c, x, y, x * 16, y * 16, (x + y) * 8, 255);
      }
    }
    const decoded = decodeQoi(encodeQoi(c));
    for (let y = 0; y < 16; y++) {
      for (let x = 0; x < 16; x++) {
        expect(pixelAt(decoded, x, y)).toEqual(pixelAt(c, x, y));
      }
    }
  });

  it("multi-colour 2×2 image round-trips", () => {
    const c = createPixelContainer(2, 2);
    setPixel(c, 0, 0, 255, 0, 0, 255);
    setPixel(c, 1, 0, 0, 255, 0, 255);
    setPixel(c, 0, 1, 0, 0, 255, 255);
    setPixel(c, 1, 1, 128, 128, 128, 255);
    const decoded = decodeQoi(encodeQoi(c));
    for (let y = 0; y < 2; y++) {
      for (let x = 0; x < 2; x++) {
        expect(pixelAt(decoded, x, y)).toEqual(pixelAt(c, x, y));
      }
    }
  });

  it("preserves alpha channel", () => {
    const c = createPixelContainer(2, 1);
    setPixel(c, 0, 0, 255, 0, 0, 128);
    setPixel(c, 1, 0, 0, 0, 255, 64);
    const decoded = decodeQoi(encodeQoi(c));
    expect(pixelAt(decoded, 0, 0)).toEqual([255, 0, 0, 128]);
    expect(pixelAt(decoded, 1, 0)).toEqual([0, 0, 255, 64]);
  });

  it("repeated-pixel image compresses (OP_RUN)", () => {
    // A solid 100×100 image (10000 pixels) should compress dramatically.
    // Uncompressed: 10000 * 5 bytes (OP_RGBA) = 50000 bytes.
    // With OP_RUN: header(14) + first pixel OP_RGB(4) + ceil(9999/62) run ops(162) + end(8) ≈ 188 bytes.
    const c = solid(100, 100, 200, 200, 200, 255);
    const qoi = encodeQoi(c);
    expect(qoi.length).toBeLessThan(250); // well below uncompressed
  });
});

// ============================================================================
// Decode errors
// ============================================================================

describe("QOI decode errors", () => {
  it("throws on file too short", () => {
    expect(() => decodeQoi(new Uint8Array(10))).toThrow("too short");
  });

  it("throws on invalid magic", () => {
    const bad = new Uint8Array(22);
    bad[0] = 0x50; // "P" not "q"
    expect(() => decodeQoi(bad)).toThrow("invalid magic");
  });

  it("throws on zero dimensions", () => {
    const buf = new Uint8Array(22);
    buf.set([0x71, 0x6f, 0x69, 0x66]); // magic
    // width=0, height=1 → invalid
    expect(() => decodeQoi(buf)).toThrow("invalid dimensions");
  });
});

// ============================================================================
// QoiCodec trait
// ============================================================================

describe("QoiCodec", () => {
  const codec = new QoiCodec();

  it("mimeType is image/qoi", () => {
    expect(codec.mimeType).toBe("image/qoi");
  });

  it("encode/decode round-trips via codec", () => {
    const c = solid(4, 4, 60, 120, 180, 255);
    const decoded = codec.decode(codec.encode(c));
    expect(decoded.data).toEqual(c.data);
  });
});
