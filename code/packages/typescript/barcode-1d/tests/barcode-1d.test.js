import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  SUPPORTED_BARCODE_1D_SYMBOLOGIES,
  getPaintBackend,
  layoutBarcode1D,
  renderBarcode1DToPng,
  renderPaintSceneToPng,
} from "../index.js";

const UNSUPPORTED_BACKEND_ERROR =
  /barcode-1d native rendering currently supports only macOS \(paint-metal\) and Windows \(Direct2D\/GDI\)/;

describe("SUPPORTED_BARCODE_1D_SYMBOLOGIES", () => {
  it("lists the supported symbologies", () => {
    assert.deepStrictEqual(SUPPORTED_BARCODE_1D_SYMBOLOGIES, [
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

    assert.equal(scene.metadata?.symbology, "code39");
    assert.ok(scene.instructions.length > 0);
  });

  it("passes Codabar guard options through", () => {
    const scene = layoutBarcode1D({
      symbology: "codabar",
      data: "40156",
      start: "B",
      stop: "D",
    });

    assert.equal(scene.metadata?.start, "B");
    assert.equal(scene.metadata?.stop, "D");
  });
});

describe("native rendering", () => {
  it("reports the active paint backend for this platform", () => {
    const backend = getPaintBackend();

    if (process.platform === "darwin") {
      assert.equal(backend, "paint-metal");
    } else if (process.platform === "win32") {
      assert.equal(backend, "paint-vm-direct2d");
    } else {
      assert.equal(backend, "unsupported");
    }
  });

  it("handles direct PaintScene rendering on supported and unsupported platforms", () => {
    const scene = layoutBarcode1D({
      symbology: "code39",
      data: "ADHITHYA",
    });
    const backend = getPaintBackend();

    if (backend === "unsupported") {
      assert.throws(() => renderPaintSceneToPng(scene), UNSUPPORTED_BACKEND_ERROR);
      return;
    }

    const png = renderPaintSceneToPng(scene);
    assert.equal(Buffer.isBuffer(png), true);
    assert.deepStrictEqual(Array.from(png.subarray(0, 8)), [137, 80, 78, 71, 13, 10, 26, 10]);
  });

  it("handles high-level barcode rendering on supported and unsupported platforms", () => {
    const backend = getPaintBackend();

    if (backend === "unsupported") {
      assert.throws(
        () =>
          renderBarcode1DToPng({
            symbology: "upc-a",
            data: "03600029145",
          }),
        UNSUPPORTED_BACKEND_ERROR,
      );
      return;
    }

    const png = renderBarcode1DToPng({
      symbology: "upc-a",
      data: "03600029145",
    });

    assert.equal(Buffer.isBuffer(png), true);
    assert.ok(png.length > 8);
  });
});
