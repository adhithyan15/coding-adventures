/**
 * Tests for @coding-adventures/image-geometric-transforms (IMG04).
 *
 * Coverage areas:
 *   - Lossless transforms: flipHorizontal, flipVertical, rotate90CW,
 *     rotate90CCW, rotate180, crop, pad
 *   - Double-flip / round-trip identities
 *   - Continuous transforms: scale, rotate, affine, perspectiveWarp
 *   - Sampling: nearest, bilinear, bicubic
 *   - Out-of-bounds policies: zero, replicate, reflect, wrap
 */

import { describe, it, expect } from "vitest";
import {
  createPixelContainer,
  pixelAt,
  setPixel,
} from "@coding-adventures/pixel-container";
import {
  flipHorizontal,
  flipVertical,
  rotate90CW,
  rotate90CCW,
  rotate180,
  crop,
  pad,
  scale,
  rotate,
  affine,
  perspectiveWarp,
  sample,
  type Rgba8,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Create a small test image where each pixel has a unique colour. */
function makeGradient(w: number, h: number) {
  const img = createPixelContainer(w, h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      setPixel(img, x, y, x * 10, y * 10, (x + y) * 5, 255);
    }
  }
  return img;
}

/** Check whether two Rgba8 tuples are equal. */
function pixelEq(a: Rgba8, b: Rgba8): boolean {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && a[3] === b[3];
}

/** Check that two Rgba8 tuples are within `tol` of each other per channel. */
function pixelClose(a: Rgba8, b: Rgba8, tol = 2): boolean {
  return (
    Math.abs(a[0] - b[0]) <= tol &&
    Math.abs(a[1] - b[1]) <= tol &&
    Math.abs(a[2] - b[2]) <= tol &&
    Math.abs(a[3] - b[3]) <= tol
  );
}

// ---------------------------------------------------------------------------
// flipHorizontal
// ---------------------------------------------------------------------------

describe("flipHorizontal", () => {
  it("reverses the pixel order in each row", () => {
    const src = makeGradient(4, 2);
    const out = flipHorizontal(src);

    expect(out.width).toBe(4);
    expect(out.height).toBe(2);

    for (let y = 0; y < 2; y++) {
      for (let x = 0; x < 4; x++) {
        const expected = pixelAt(src, 3 - x, y);
        const actual   = pixelAt(out, x, y);
        expect(pixelEq(actual, expected)).toBe(true);
      }
    }
  });

  it("double flip is the identity", () => {
    const src = makeGradient(5, 3);
    const out = flipHorizontal(flipHorizontal(src));

    for (let y = 0; y < 3; y++) {
      for (let x = 0; x < 5; x++) {
        expect(pixelEq(pixelAt(out, x, y), pixelAt(src, x, y))).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// flipVertical
// ---------------------------------------------------------------------------

describe("flipVertical", () => {
  it("reverses row order", () => {
    const src = makeGradient(3, 4);
    const out = flipVertical(src);

    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 3; x++) {
        expect(pixelEq(pixelAt(out, x, y), pixelAt(src, x, 3 - y))).toBe(true);
      }
    }
  });

  it("double flip is the identity", () => {
    const src = makeGradient(4, 4);
    const out = flipVertical(flipVertical(src));

    for (let y = 0; y < 4; y++) {
      for (let x = 0; x < 4; x++) {
        expect(pixelEq(pixelAt(out, x, y), pixelAt(src, x, y))).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// rotate90CW
// ---------------------------------------------------------------------------

describe("rotate90CW", () => {
  it("swaps width and height", () => {
    const src = makeGradient(5, 3);
    const out = rotate90CW(src);
    expect(out.width).toBe(3);
    expect(out.height).toBe(5);
  });

  it("correctly maps output (0,0) to source (0, H-1)", () => {
    const src = makeGradient(4, 3);
    // CW inverse warp: out[x'=0][y'=0] reads source at (x=y'=0, y=H-1-x'=H-1-0=H-1)
    // H = src.height = 3, so source coordinate is (0, 2)
    const out = rotate90CW(src);
    expect(pixelEq(pixelAt(out, 0, 0), pixelAt(src, 0, src.height - 1))).toBe(true);
  });

  it("four CW rotations return the original dimensions", () => {
    const src = makeGradient(4, 3);
    let cur = src;
    for (let i = 0; i < 4; i++) cur = rotate90CW(cur);
    expect(cur.width).toBe(src.width);
    expect(cur.height).toBe(src.height);
  });
});

// ---------------------------------------------------------------------------
// rotate90CCW
// ---------------------------------------------------------------------------

describe("rotate90CCW", () => {
  it("swaps width and height", () => {
    const src = makeGradient(6, 2);
    const out = rotate90CCW(src);
    expect(out.width).toBe(2);
    expect(out.height).toBe(6);
  });

  it("CW followed by CCW is identity (dimensions)", () => {
    const src = makeGradient(5, 3);
    const roundTrip = rotate90CCW(rotate90CW(src));
    expect(roundTrip.width).toBe(src.width);
    expect(roundTrip.height).toBe(src.height);
  });

  it("CW followed by CCW is identity (pixels)", () => {
    const src = makeGradient(4, 3);
    const roundTrip = rotate90CCW(rotate90CW(src));
    for (let y = 0; y < src.height; y++) {
      for (let x = 0; x < src.width; x++) {
        expect(pixelEq(pixelAt(roundTrip, x, y), pixelAt(src, x, y))).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// rotate180
// ---------------------------------------------------------------------------

describe("rotate180", () => {
  it("preserves dimensions", () => {
    const src = makeGradient(5, 7);
    const out = rotate180(src);
    expect(out.width).toBe(5);
    expect(out.height).toBe(7);
  });

  it("double rotate180 is the identity", () => {
    const src = makeGradient(5, 7);
    const out = rotate180(rotate180(src));
    for (let y = 0; y < 7; y++) {
      for (let x = 0; x < 5; x++) {
        expect(pixelEq(pixelAt(out, x, y), pixelAt(src, x, y))).toBe(true);
      }
    }
  });

  it("pixel (0,0) maps to (W-1, H-1)", () => {
    const src = makeGradient(4, 3);
    const out = rotate180(src);
    expect(pixelEq(pixelAt(out, 0, 0), pixelAt(src, 3, 2))).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// crop
// ---------------------------------------------------------------------------

describe("crop", () => {
  it("produces the correct output size", () => {
    const src = makeGradient(10, 10);
    const out = crop(src, 2, 3, 4, 5);
    expect(out.width).toBe(4);
    expect(out.height).toBe(5);
  });

  it("extracts the correct pixels", () => {
    const src = makeGradient(10, 10);
    const out = crop(src, 2, 3, 4, 5);

    for (let y = 0; y < 5; y++) {
      for (let x = 0; x < 4; x++) {
        expect(pixelEq(pixelAt(out, x, y), pixelAt(src, 2 + x, 3 + y))).toBe(true);
      }
    }
  });

  it("OOB pixels are transparent black", () => {
    const src = makeGradient(4, 4);
    // Crop with origin outside source
    const out = crop(src, 10, 10, 2, 2);
    expect(pixelEq(pixelAt(out, 0, 0), [0, 0, 0, 0])).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// pad
// ---------------------------------------------------------------------------

describe("pad", () => {
  it("produces the correct output dimensions", () => {
    const src = makeGradient(4, 3);
    const out = pad(src, 1, 2, 3, 4, [0, 0, 0, 255]);
    expect(out.width).toBe(4 + 2 + 4);   // src + right + left
    expect(out.height).toBe(3 + 1 + 3);  // src + top + bottom
  });

  it("border pixels have the fill colour", () => {
    const src = makeGradient(4, 3);
    const fill: Rgba8 = [255, 0, 0, 255];
    const out = pad(src, 2, 2, 2, 2, fill);
    // Top-left corner is border
    expect(pixelEq(pixelAt(out, 0, 0), fill)).toBe(true);
    // Bottom-right corner is border
    expect(pixelEq(pixelAt(out, out.width - 1, out.height - 1), fill)).toBe(true);
  });

  it("interior matches the source", () => {
    const src = makeGradient(4, 3);
    const out = pad(src, 1, 1, 1, 1, [0, 0, 0, 0]);

    for (let y = 0; y < 3; y++) {
      for (let x = 0; x < 4; x++) {
        expect(pixelEq(pixelAt(out, 1 + x, 1 + y), pixelAt(src, x, y))).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// scale
// ---------------------------------------------------------------------------

describe("scale", () => {
  it("produces the correct output dimensions on upscale", () => {
    const src = makeGradient(4, 3);
    const out = scale(src, 8, 6);
    expect(out.width).toBe(8);
    expect(out.height).toBe(6);
  });

  it("produces the correct output dimensions on downscale", () => {
    const src = makeGradient(8, 6);
    const out = scale(src, 4, 3);
    expect(out.width).toBe(4);
    expect(out.height).toBe(3);
  });

  it("replicate OOB does not throw during scale", () => {
    const src = makeGradient(3, 3);
    expect(() => scale(src, 7, 7, "nearest")).not.toThrow();
  });

  it("scaling 1x1 to 1x1 with nearest returns the same pixel", () => {
    const src = createPixelContainer(1, 1);
    setPixel(src, 0, 0, 100, 150, 200, 255);
    const out = scale(src, 1, 1, "nearest");
    expect(pixelEq(pixelAt(out, 0, 0), [100, 150, 200, 255])).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// rotate (continuous)
// ---------------------------------------------------------------------------

describe("rotate (continuous)", () => {
  it("rotate(0) is approximately identity", () => {
    const src = makeGradient(8, 8);
    const out = rotate(src, 0, "nearest", "crop");

    let maxDiff = 0;
    for (let y = 0; y < 8; y++) {
      for (let x = 0; x < 8; x++) {
        const sp = pixelAt(src, x, y);
        const op = pixelAt(out, x, y);
        for (let c = 0; c < 4; c++) {
          maxDiff = Math.max(maxDiff, Math.abs(sp[c] - op[c]));
        }
      }
    }
    expect(maxDiff).toBeLessThanOrEqual(2);
  });

  it("fit mode enlarges dimensions for 45° rotation", () => {
    const src = makeGradient(10, 10);
    const out = rotate(src, Math.PI / 4, "bilinear", "fit");
    // sqrt(2) * 10 ≈ 14.14, so ceil is 15 in both dimensions
    expect(out.width).toBeGreaterThan(src.width);
    expect(out.height).toBeGreaterThan(src.height);
  });

  it("crop mode preserves original dimensions", () => {
    const src = makeGradient(8, 8);
    const out = rotate(src, Math.PI / 6, "bilinear", "crop");
    expect(out.width).toBe(src.width);
    expect(out.height).toBe(src.height);
  });
});

// ---------------------------------------------------------------------------
// affine
// ---------------------------------------------------------------------------

describe("affine", () => {
  it("identity matrix is identity (nearest)", () => {
    const src = makeGradient(6, 6);
    const identity: [[number, number, number], [number, number, number]] = [
      [1, 0, 0],
      [0, 1, 0],
    ];
    const out = affine(src, identity, 6, 6, "nearest", "replicate");

    for (let y = 0; y < 6; y++) {
      for (let x = 0; x < 6; x++) {
        expect(pixelClose(pixelAt(out, x, y), pixelAt(src, x, y), 1)).toBe(true);
      }
    }
  });

  it("produces correct output dimensions", () => {
    const src = makeGradient(4, 4);
    const identity: [[number, number, number], [number, number, number]] = [
      [1, 0, 0],
      [0, 1, 0],
    ];
    const out = affine(src, identity, 8, 5);
    expect(out.width).toBe(8);
    expect(out.height).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// perspectiveWarp
// ---------------------------------------------------------------------------

describe("perspectiveWarp", () => {
  it("identity homography is identity (nearest, replicate)", () => {
    const src = makeGradient(6, 6);
    const identity: [[number, number, number], [number, number, number], [number, number, number]] = [
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ];
    const out = perspectiveWarp(src, identity, 6, 6, "nearest", "replicate");

    for (let y = 0; y < 6; y++) {
      for (let x = 0; x < 6; x++) {
        expect(pixelClose(pixelAt(out, x, y), pixelAt(src, x, y), 1)).toBe(true);
      }
    }
  });

  it("produces correct output dimensions", () => {
    const src = makeGradient(4, 4);
    const identity: [[number, number, number], [number, number, number], [number, number, number]] = [
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ];
    const out = perspectiveWarp(src, identity, 10, 7);
    expect(out.width).toBe(10);
    expect(out.height).toBe(7);
  });
});

// ---------------------------------------------------------------------------
// Sampling — nearest
// ---------------------------------------------------------------------------

describe("sample (nearest)", () => {
  it("returns exact pixel value for integer coordinate", () => {
    const src = createPixelContainer(4, 4);
    setPixel(src, 2, 1, 111, 222, 33, 200);
    const result = sample(src, 2, 1, "nearest", "zero");
    expect(result).toEqual([111, 222, 33, 200]);
  });

  it("returns [0,0,0,0] for OOB with 'zero'", () => {
    const src = makeGradient(4, 4);
    expect(sample(src, -1, 0, "nearest", "zero")).toEqual([0, 0, 0, 0]);
  });
});

// ---------------------------------------------------------------------------
// Sampling — bilinear
// ---------------------------------------------------------------------------

describe("sample (bilinear)", () => {
  it("midpoint of two pure-channel pixels blends correctly", () => {
    // A 2×1 image: pixel 0 = black (0,0,0,255), pixel 1 = white (255,255,255,255).
    // The midpoint u=0.5 sits exactly between the two pixels, so we expect the
    // average in linear light ≈ encode(0.5) ≈ 188 (sRGB midpoint in linear light).
    const src = createPixelContainer(2, 1);
    setPixel(src, 0, 0, 0,   0,   0,   255);
    setPixel(src, 1, 0, 255, 255, 255, 255);

    const result = sample(src, 0.5, 0, "bilinear", "replicate");
    // Midpoint blend: decode(0)*0.5 + decode(255)*0.5 = 0.5 in linear
    // encode(0.5) ≈ 188 in sRGB
    expect(result[0]).toBeGreaterThan(180);
    expect(result[0]).toBeLessThan(200);
    expect(result[3]).toBe(255); // alpha unchanged
  });

  it("at an integer coordinate returns original pixel (within tolerance)", () => {
    const src = createPixelContainer(4, 4);
    setPixel(src, 1, 1, 100, 150, 200, 255);
    const result = sample(src, 1, 1, "bilinear", "zero");
    expect(pixelClose(result, [100, 150, 200, 255], 2)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Sampling — bicubic
// ---------------------------------------------------------------------------

describe("sample (bicubic)", () => {
  it("at an integer coordinate returns approximately original pixel", () => {
    const src = makeGradient(8, 8);
    const result = sample(src, 3, 3, "bicubic", "replicate");
    const expected = pixelAt(src, 3, 3);
    expect(pixelClose(result, expected, 3)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// OutOfBounds modes
// ---------------------------------------------------------------------------

describe("OutOfBounds modes", () => {
  it("'replicate' clamps to border pixel", () => {
    const src = createPixelContainer(4, 4);
    setPixel(src, 0, 0, 50, 60, 70, 255);
    const result = sample(src, -5, 0, "nearest", "replicate");
    expect(result).toEqual([50, 60, 70, 255]);
  });

  it("'wrap' tiles the image", () => {
    const src = createPixelContainer(4, 4);
    setPixel(src, 0, 0, 77, 88, 99, 255);
    // Coordinate 4 should wrap to 0
    const result = sample(src, 4, 0, "nearest", "wrap");
    expect(result).toEqual([77, 88, 99, 255]);
  });

  it("'reflect' mirrors at the border", () => {
    const src = createPixelContainer(4, 4);
    setPixel(src, 0, 0, 10, 20, 30, 255);
    setPixel(src, 1, 0, 11, 21, 31, 255);
    // Coordinate -1 should reflect to coordinate 0
    const result = sample(src, -1, 0, "nearest", "reflect");
    expect(result).toEqual([10, 20, 30, 255]);
  });

  it("'zero' returns transparent black for OOB", () => {
    const src = makeGradient(4, 4);
    const result = sample(src, 100, 100, "nearest", "zero");
    expect(result).toEqual([0, 0, 0, 0]);
  });
});
