import { describe, it, expect } from "vitest";
import {
  PpmCodec,
  encodePpm,
  decodePpm,
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
// Header structure
// ============================================================================

describe("PPM header", () => {
  it("starts with P6\\n", () => {
    const ppm = encodePpm(solid(2, 2, 0, 0, 0, 255));
    expect(ppm[0]).toBe(0x50); // P
    expect(ppm[1]).toBe(0x36); // 6
    expect(ppm[2]).toBe(0x0a); // \n
  });

  it("contains width and height in header", () => {
    const ppm = encodePpm(solid(640, 480, 0, 0, 0, 255));
    const header = new TextDecoder().decode(ppm.slice(0, 20));
    expect(header).toContain("640");
    expect(header).toContain("480");
  });

  it("encoded size = header length + width*height*3", () => {
    const ppm = encodePpm(solid(3, 2, 0, 0, 0, 255));
    const header = new TextEncoder().encode("P6\n3 2\n255\n");
    expect(ppm.length).toBe(header.length + 3 * 2 * 3);
  });
});

// ============================================================================
// Alpha handling
// ============================================================================

describe("PPM alpha", () => {
  it("drops alpha on encode (3 bytes per pixel)", () => {
    const c = createPixelContainer(1, 1);
    setPixel(c, 0, 0, 10, 20, 30, 128);
    const ppm = encodePpm(c);
    const header = new TextEncoder().encode("P6\n1 1\n255\n");
    expect(ppm.length).toBe(header.length + 3);
    expect(ppm[header.length]).toBe(10); // R
    expect(ppm[header.length + 1]).toBe(20); // G
    expect(ppm[header.length + 2]).toBe(30); // B
  });

  it("decoded pixels always have A=255", () => {
    const c = solid(2, 2, 100, 150, 200, 128);
    const decoded = decodePpm(encodePpm(c));
    for (let y = 0; y < 2; y++) {
      for (let x = 0; x < 2; x++) {
        expect(pixelAt(decoded, x, y)[3]).toBe(255);
      }
    }
  });
});

// ============================================================================
// Round-trip
// ============================================================================

describe("PPM round-trip", () => {
  it("preserves RGB of a solid colour", () => {
    const c = solid(5, 3, 100, 150, 200, 255);
    const decoded = decodePpm(encodePpm(c));
    expect(decoded.width).toBe(5);
    expect(decoded.height).toBe(3);
    for (let y = 0; y < 3; y++) {
      for (let x = 0; x < 5; x++) {
        const [r, g, b] = pixelAt(decoded, x, y);
        expect([r, g, b]).toEqual([100, 150, 200]);
      }
    }
  });

  it("preserves multi-colour pixels", () => {
    const c = createPixelContainer(2, 2);
    setPixel(c, 0, 0, 255, 0, 0, 255);
    setPixel(c, 1, 0, 0, 255, 0, 255);
    setPixel(c, 0, 1, 0, 0, 255, 255);
    setPixel(c, 1, 1, 128, 64, 32, 255);
    const decoded = decodePpm(encodePpm(c));
    for (let y = 0; y < 2; y++) {
      for (let x = 0; x < 2; x++) {
        const [r1, g1, b1] = pixelAt(c, x, y);
        const [r2, g2, b2] = pixelAt(decoded, x, y);
        expect([r2, g2, b2]).toEqual([r1, g1, b1]);
      }
    }
  });
});

// ============================================================================
// Comment handling in decode
// ============================================================================

describe("PPM comment handling", () => {
  it("decodes a file with a comment in the header", () => {
    const text = "P6\n# this is a comment\n4 4\n255\n";
    const header = new TextEncoder().encode(text);
    const pixels = new Uint8Array(4 * 4 * 3);
    const data = new Uint8Array(header.length + pixels.length);
    data.set(header, 0);
    data.set(pixels, header.length);
    const result = decodePpm(data);
    expect(result.width).toBe(4);
    expect(result.height).toBe(4);
  });
});

// ============================================================================
// Decode errors
// ============================================================================

describe("PPM decode errors", () => {
  it("throws on wrong magic", () => {
    const bad = new TextEncoder().encode("P3\n1 1\n255\n\x00\x00\x00");
    expect(() => decodePpm(bad)).toThrow("invalid magic");
  });

  it("throws on unsupported max value", () => {
    const bad = new TextEncoder().encode("P6\n1 1\n65535\n\x00\x00\x00\x00\x00\x00");
    expect(() => decodePpm(bad)).toThrow("unsupported max value");
  });

  it("throws on truncated pixel data", () => {
    const header = new TextEncoder().encode("P6\n4 4\n255\n");
    const data = new Uint8Array(header.length + 3); // only 3 bytes instead of 48
    data.set(header, 0);
    expect(() => decodePpm(data)).toThrow("truncated");
  });
});

// ============================================================================
// PpmCodec trait
// ============================================================================

describe("PpmCodec", () => {
  const codec = new PpmCodec();

  it("mimeType is image/x-portable-pixmap", () => {
    expect(codec.mimeType).toBe("image/x-portable-pixmap");
  });

  it("encode/decode round-trips via codec", () => {
    const c = solid(3, 3, 60, 120, 180, 255);
    const decoded = codec.decode(codec.encode(c));
    for (let y = 0; y < 3; y++) {
      for (let x = 0; x < 3; x++) {
        const [r, g, b] = pixelAt(decoded, x, y);
        expect([r, g, b]).toEqual([60, 120, 180]);
      }
    }
  });
});
