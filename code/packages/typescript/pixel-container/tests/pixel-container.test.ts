import { describe, it, expect } from "vitest";
import {
  VERSION,
  createPixelContainer,
  pixelAt,
  setPixel,
  fillPixels,
  type PixelContainer,
  type ImageCodec,
} from "../src/index.js";

// ============================================================================
// VERSION
// ============================================================================

describe("VERSION", () => {
  it("exports a semver string", () => {
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

// ============================================================================
// createPixelContainer
// ============================================================================

describe("createPixelContainer", () => {
  it("creates a buffer with width * height * 4 bytes", () => {
    const c = createPixelContainer(4, 3);
    expect(c.data.length).toBe(48);
  });

  it("sets width and height", () => {
    const c = createPixelContainer(100, 200);
    expect(c.width).toBe(100);
    expect(c.height).toBe(200);
  });

  it("initialises all bytes to zero", () => {
    const c = createPixelContainer(5, 5);
    for (let i = 0; i < c.data.length; i++) {
      expect(c.data[i]).toBe(0);
    }
  });

  it("handles 1x1 container", () => {
    const c = createPixelContainer(1, 1);
    expect(c.data.length).toBe(4);
  });

  it("handles 0x0 container", () => {
    const c = createPixelContainer(0, 0);
    expect(c.data.length).toBe(0);
  });
});

// ============================================================================
// pixelAt
// ============================================================================

describe("pixelAt", () => {
  it("returns [0,0,0,0] for a fresh container", () => {
    const c = createPixelContainer(4, 4);
    expect(pixelAt(c, 2, 2)).toEqual([0, 0, 0, 0]);
  });

  it("returns the correct RGBA after setPixel", () => {
    const c = createPixelContainer(4, 4);
    setPixel(c, 1, 2, 200, 100, 50, 255);
    expect(pixelAt(c, 1, 2)).toEqual([200, 100, 50, 255]);
  });

  it("returns [0,0,0,0] for x >= width", () => {
    const c = createPixelContainer(3, 3);
    expect(pixelAt(c, 3, 0)).toEqual([0, 0, 0, 0]);
  });

  it("returns [0,0,0,0] for y >= height", () => {
    const c = createPixelContainer(3, 3);
    expect(pixelAt(c, 0, 3)).toEqual([0, 0, 0, 0]);
  });

  it("returns [0,0,0,0] for negative coordinates", () => {
    const c = createPixelContainer(3, 3);
    expect(pixelAt(c, -1, 0)).toEqual([0, 0, 0, 0]);
    expect(pixelAt(c, 0, -1)).toEqual([0, 0, 0, 0]);
  });

  it("uses row-major layout: offset = (y*width + x) * 4", () => {
    const c = createPixelContainer(3, 2);
    // pixel (x=2, y=1) → offset = (1*3 + 2)*4 = 20
    c.data[20] = 11;
    c.data[21] = 22;
    c.data[22] = 33;
    c.data[23] = 44;
    expect(pixelAt(c, 2, 1)).toEqual([11, 22, 33, 44]);
  });
});

// ============================================================================
// setPixel
// ============================================================================

describe("setPixel", () => {
  it("writes RGBA to the correct offset", () => {
    const c = createPixelContainer(4, 4);
    setPixel(c, 0, 0, 10, 20, 30, 40);
    expect(c.data[0]).toBe(10);
    expect(c.data[1]).toBe(20);
    expect(c.data[2]).toBe(30);
    expect(c.data[3]).toBe(40);
  });

  it("does not affect neighbouring pixels", () => {
    const c = createPixelContainer(4, 4);
    setPixel(c, 2, 1, 255, 0, 0, 255);
    expect(pixelAt(c, 1, 1)).toEqual([0, 0, 0, 0]);
    expect(pixelAt(c, 3, 1)).toEqual([0, 0, 0, 0]);
  });

  it("is a no-op for out-of-bounds x", () => {
    const c = createPixelContainer(2, 2);
    setPixel(c, 99, 0, 1, 2, 3, 4);
    expect(Array.from(c.data)).toEqual(new Array(16).fill(0));
  });

  it("is a no-op for out-of-bounds y", () => {
    const c = createPixelContainer(2, 2);
    setPixel(c, 0, 99, 1, 2, 3, 4);
    expect(Array.from(c.data)).toEqual(new Array(16).fill(0));
  });

  it("round-trips with pixelAt", () => {
    const c = createPixelContainer(10, 10);
    setPixel(c, 5, 7, 128, 64, 32, 200);
    expect(pixelAt(c, 5, 7)).toEqual([128, 64, 32, 200]);
  });
});

// ============================================================================
// fillPixels
// ============================================================================

describe("fillPixels", () => {
  it("sets every pixel to the given colour", () => {
    const c = createPixelContainer(3, 3);
    fillPixels(c, 100, 150, 200, 255);
    for (let y = 0; y < 3; y++) {
      for (let x = 0; x < 3; x++) {
        expect(pixelAt(c, x, y)).toEqual([100, 150, 200, 255]);
      }
    }
  });

  it("overwrites previously set pixels", () => {
    const c = createPixelContainer(2, 2);
    setPixel(c, 0, 0, 255, 0, 0, 255);
    fillPixels(c, 0, 0, 0, 0);
    expect(pixelAt(c, 0, 0)).toEqual([0, 0, 0, 0]);
  });

  it("works on a 0x0 container without error", () => {
    const c = createPixelContainer(0, 0);
    expect(() => fillPixels(c, 1, 2, 3, 4)).not.toThrow();
  });
});

// ============================================================================
// PixelContainer — type-level structural checks
// ============================================================================

describe("PixelContainer interface", () => {
  it("can be constructed as a plain object", () => {
    const c: PixelContainer = {
      width: 2,
      height: 2,
      data: new Uint8Array(16),
    };
    expect(c.width).toBe(2);
    expect(c.height).toBe(2);
    expect(c.data.length).toBe(16);
  });

  it("data is a Uint8Array", () => {
    const c = createPixelContainer(3, 3);
    expect(c.data).toBeInstanceOf(Uint8Array);
  });
});

// ============================================================================
// ImageCodec — structural (no implementation, just verifies the interface shape)
// ============================================================================

describe("ImageCodec interface", () => {
  it("can be satisfied by a plain object implementing the contract", () => {
    const stub: ImageCodec = {
      mimeType: "image/test",
      encode(pixels: PixelContainer): Uint8Array {
        // Encode: just return width and height as a 2-byte array.
        return new Uint8Array([pixels.width, pixels.height]);
      },
      decode(bytes: Uint8Array): PixelContainer {
        const [w, h] = bytes;
        return createPixelContainer(w, h);
      },
    };

    const c = createPixelContainer(3, 2);
    const encoded = stub.encode(c);
    expect(encoded[0]).toBe(3);
    expect(encoded[1]).toBe(2);

    const decoded = stub.decode(encoded);
    expect(decoded.width).toBe(3);
    expect(decoded.height).toBe(2);
  });

  it("mimeType is a string", () => {
    const stub: ImageCodec = {
      mimeType: "image/bmp",
      encode: () => new Uint8Array(0),
      decode: () => createPixelContainer(0, 0),
    };
    expect(stub.mimeType).toBe("image/bmp");
  });
});
