import { describe, it, expect } from "vitest";
import {
  createPixelContainer,
  setPixel,
  pixelAt,
} from "@coding-adventures/pixel-container";
import {
  invert,
  threshold,
  thresholdLuminance,
  posterize,
  swapRgbBgr,
  extractChannel,
  brightness,
  contrast,
  gamma,
  exposure,
  greyscale,
  sepia,
  colourMatrix,
  saturate,
  hueRotate,
  srgbToLinearImage,
  linearToSrgbImage,
  applyLut1dU8,
  buildLut1dU8,
  buildGammaLut,
} from "../src/index.js";

// Helper: create a 1×1 image with a single pixel.
function solid(r: number, g: number, b: number, a: number) {
  const img = createPixelContainer(1, 1);
  setPixel(img, 0, 0, r, g, b, a);
  return img;
}

describe("dimensions are preserved", () => {
  it("invert preserves 3×5 size", () => {
    const img = createPixelContainer(3, 5);
    const out = invert(img);
    expect(out.width).toBe(3);
    expect(out.height).toBe(5);
  });
});

describe("invert", () => {
  it("flips RGB, preserves alpha", () => {
    const out = invert(solid(10, 100, 200, 128));
    expect(pixelAt(out, 0, 0)).toEqual([245, 155, 55, 128]);
  });

  it("double invert is identity", () => {
    const img = solid(30, 80, 180, 255);
    const out = invert(invert(img));
    expect(pixelAt(out, 0, 0)).toEqual(pixelAt(img, 0, 0));
  });
});

describe("threshold", () => {
  it("above threshold → white", () => {
    const out = threshold(solid(200, 200, 200, 255), 128);
    expect(pixelAt(out, 0, 0)).toEqual([255, 255, 255, 255]);
  });

  it("below threshold → black", () => {
    const out = threshold(solid(50, 50, 50, 255), 128);
    expect(pixelAt(out, 0, 0)).toEqual([0, 0, 0, 255]);
  });
});

describe("thresholdLuminance", () => {
  it("white pixel → white", () => {
    const out = thresholdLuminance(solid(255, 255, 255, 255), 128);
    expect(pixelAt(out, 0, 0)).toEqual([255, 255, 255, 255]);
  });
});

describe("posterize", () => {
  it("2 levels binarises", () => {
    const out = posterize(solid(50, 50, 50, 255), 2);
    const [r] = pixelAt(out, 0, 0);
    expect(r === 0 || r === 255).toBe(true);
  });
});

describe("swapRgbBgr", () => {
  it("swaps R and B", () => {
    const out = swapRgbBgr(solid(255, 0, 0, 255));
    expect(pixelAt(out, 0, 0)).toEqual([0, 0, 255, 255]);
  });
});

describe("extractChannel", () => {
  it("extract R zeroes G and B", () => {
    const out = extractChannel(solid(100, 150, 200, 255), 0);
    expect(pixelAt(out, 0, 0)).toEqual([100, 0, 0, 255]);
  });

  it("extract G zeroes R and B", () => {
    const out = extractChannel(solid(100, 150, 200, 255), 1);
    expect(pixelAt(out, 0, 0)).toEqual([0, 150, 0, 255]);
  });
});

describe("brightness", () => {
  it("adds offset, clamps to 255", () => {
    const out = brightness(solid(250, 10, 10, 255), 20);
    const [r, g] = pixelAt(out, 0, 0);
    expect(r).toBe(255);
    expect(g).toBe(30);
  });

  it("clamps to 0 on negative offset", () => {
    const out = brightness(solid(5, 10, 10, 255), -20);
    const [r] = pixelAt(out, 0, 0);
    expect(r).toBe(0);
  });
});

describe("contrast", () => {
  it("factor=1 is identity (within rounding)", () => {
    const img = solid(100, 150, 200, 255);
    const out = contrast(img, 1.0);
    const orig = pixelAt(img, 0, 0);
    const result = pixelAt(out, 0, 0);
    expect(Math.abs(result[0] - orig[0])).toBeLessThanOrEqual(1);
    expect(Math.abs(result[1] - orig[1])).toBeLessThanOrEqual(1);
    expect(Math.abs(result[2] - orig[2])).toBeLessThanOrEqual(1);
  });
});

describe("gamma", () => {
  it("gamma=1 is identity (within rounding)", () => {
    const img = solid(100, 150, 200, 255);
    const out = gamma(img, 1.0);
    const orig = pixelAt(img, 0, 0);
    const result = pixelAt(out, 0, 0);
    expect(Math.abs(result[0] - orig[0])).toBeLessThanOrEqual(1);
  });

  it("gamma<1 brightens midtones", () => {
    const img = solid(128, 128, 128, 255);
    const out = gamma(img, 0.5);
    const [r] = pixelAt(out, 0, 0);
    expect(r).toBeGreaterThan(128);
  });
});

describe("exposure", () => {
  it("+1 stop doubles linear light", () => {
    const img = solid(100, 100, 100, 255);
    const out = exposure(img, 1);
    const [r] = pixelAt(out, 0, 0);
    const [origR] = pixelAt(img, 0, 0);
    expect(r).toBeGreaterThan(origR);
  });
});

describe("greyscale", () => {
  it("white stays white (all methods)", () => {
    const img = solid(255, 255, 255, 255);
    for (const method of ["rec709", "bt601", "average"] as const) {
      const out = greyscale(img, method);
      expect(pixelAt(out, 0, 0)).toEqual([255, 255, 255, 255]);
    }
  });

  it("black stays black", () => {
    const out = greyscale(solid(0, 0, 0, 255));
    expect(pixelAt(out, 0, 0)).toEqual([0, 0, 0, 255]);
  });

  it("R=G=B → greyscale equal channels", () => {
    const out = greyscale(solid(100, 100, 100, 255));
    const [r, g, b] = pixelAt(out, 0, 0);
    expect(r).toBe(g);
    expect(g).toBe(b);
  });
});

describe("sepia", () => {
  it("preserves alpha", () => {
    const out = sepia(solid(128, 128, 128, 200));
    expect(pixelAt(out, 0, 0)[3]).toBe(200);
  });
});

describe("colourMatrix", () => {
  it("identity matrix is identity (within rounding)", () => {
    const img = solid(80, 120, 200, 255);
    const out = colourMatrix(img, [[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    const orig = pixelAt(img, 0, 0);
    const result = pixelAt(out, 0, 0);
    expect(Math.abs(result[0] - orig[0])).toBeLessThanOrEqual(1);
    expect(Math.abs(result[1] - orig[1])).toBeLessThanOrEqual(1);
    expect(Math.abs(result[2] - orig[2])).toBeLessThanOrEqual(1);
  });
});

describe("saturate", () => {
  it("factor=0 gives greyscale (equal channels)", () => {
    const out = saturate(solid(200, 100, 50, 255), 0);
    const [r, g, b] = pixelAt(out, 0, 0);
    expect(r).toBe(g);
    expect(g).toBe(b);
  });
});

describe("hueRotate", () => {
  it("360° is identity (within rounding)", () => {
    const img = solid(200, 80, 40, 255);
    const out = hueRotate(img, 360);
    const orig = pixelAt(img, 0, 0);
    const result = pixelAt(out, 0, 0);
    expect(Math.abs(result[0] - orig[0])).toBeLessThanOrEqual(2);
    expect(Math.abs(result[1] - orig[1])).toBeLessThanOrEqual(2);
    expect(Math.abs(result[2] - orig[2])).toBeLessThanOrEqual(2);
  });
});

describe("srgbToLinearImage / linearToSrgbImage", () => {
  it("round-trip is approximately identity (within rounding)", () => {
    const img = solid(100, 150, 200, 255);
    const out = linearToSrgbImage(srgbToLinearImage(img));
    const orig = pixelAt(img, 0, 0);
    const result = pixelAt(out, 0, 0);
    expect(Math.abs(result[0] - orig[0])).toBeLessThanOrEqual(2);
    expect(Math.abs(result[1] - orig[1])).toBeLessThanOrEqual(2);
    expect(Math.abs(result[2] - orig[2])).toBeLessThanOrEqual(2);
  });
});

describe("applyLut1dU8", () => {
  it("invert LUT inverts the image", () => {
    const invertLut = new Uint8Array(256).map((_, i) => 255 - i);
    const out = applyLut1dU8(solid(100, 0, 200, 255), invertLut, invertLut, invertLut);
    expect(pixelAt(out, 0, 0)).toEqual([155, 255, 55, 255]);
  });
});

describe("buildLut1dU8", () => {
  it("identity function produces identity LUT (within rounding)", () => {
    const lut = buildLut1dU8((v) => v);
    for (let i = 0; i < 256; i++) {
      expect(Math.abs(lut[i] - i)).toBeLessThanOrEqual(1);
    }
  });
});

describe("buildGammaLut", () => {
  it("gamma=1 produces identity LUT (within rounding)", () => {
    const lut = buildGammaLut(1);
    for (let i = 0; i < 256; i++) {
      expect(Math.abs(lut[i] - i)).toBeLessThanOrEqual(1);
    }
  });
});
