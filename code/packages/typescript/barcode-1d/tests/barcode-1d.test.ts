import { describe, expect, it } from "vitest";

import {
  SUPPORTED_BARCODE_1D_SYMBOLOGIES,
  getPaintBackend,
  layoutBarcode1D,
  renderBarcode1DToPng,
  renderPaintSceneToPng,
} from "../index.js";

describe("SUPPORTED_BARCODE_1D_SYMBOLOGIES", () => {
  it("lists the supported symbologies", () => {
    expect(SUPPORTED_BARCODE_1D_SYMBOLOGIES).toEqual([
      "code39",
      "codabar",
      "code128",
      "ean-13",
      "itf",
      "upc-a",
    ]);
  });
});

describe("layoutBarcode1D()", () => {
  it("routes Code 39 through the shared layout pipeline", () => {
    const scene = layoutBarcode1D({
      symbology: "code39",
      data: "ADHITHYA",
    });

    expect(scene.metadata?.symbology).toBe("code39");
    expect(scene.instructions.length).toBeGreaterThan(0);
  });

  it("passes Codabar guard options through", () => {
    const scene = layoutBarcode1D({
      symbology: "codabar",
      data: "40156",
      start: "B",
      stop: "D",
    });

    expect(scene.metadata?.start).toBe("B");
    expect(scene.metadata?.stop).toBe("D");
  });
});

describe("native rendering", () => {
  it("reports the active paint backend for this platform", () => {
    const backend = getPaintBackend();

    if (process.platform === "darwin") {
      expect(backend).toBe("paint-metal");
    } else if (process.platform === "win32") {
      expect(backend).toBe("paint-vm-direct2d");
    } else {
      expect(backend).toBe("unsupported");
    }
  });

  it("renders a PaintScene directly to PNG bytes", () => {
    const scene = layoutBarcode1D({
      symbology: "code39",
      data: "ADHITHYA",
    });
    const png = renderPaintSceneToPng(scene);

    expect(Buffer.isBuffer(png)).toBe(true);
    expect(Array.from(png.subarray(0, 8))).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
  });

  it("renders a high-level barcode request to PNG bytes", () => {
    const png = renderBarcode1DToPng({
      symbology: "upc-a",
      data: "03600029145",
    });

    expect(Buffer.isBuffer(png)).toBe(true);
    expect(png.length).toBeGreaterThan(8);
  });
});
