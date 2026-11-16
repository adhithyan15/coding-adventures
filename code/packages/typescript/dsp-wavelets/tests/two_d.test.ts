import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { dwt2d, idwt2d, Periodic, Symmetric } from "../src/index.js";

describe("2-D Discrete Wavelet Transform (dwt2d, idwt2d)", () => {
  it("dwt2d Haar 4x4", () => {
    const rows = 4;
    const cols = 4;
    const image = new Float64Array(rows * cols);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        image[r * cols + c] = r * cols + c;
      }
    }

    const coeffs = dwt2d(image, rows, cols, "haar", 1, Periodic);
    assert.equal(coeffs.length, rows * cols);

    const rec = idwt2d(coeffs, rows, cols, "haar", 1, Periodic);
    assert.equal(rec.length, image.length);
    for (let i = 0; i < image.length; i++) {
      assert.ok(Math.abs(image[i] - rec[i]) < 1e-5);
    }
  });

  it("dwt2d multilevel 8x8 db2", () => {
    const rows = 8;
    const cols = 8;
    const image = new Float64Array(rows * cols);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        image[r * cols + c] = Math.sin(r * 0.5) * Math.cos(c * 0.5);
      }
    }

    const coeffs = dwt2d(image, rows, cols, "db2", 2, Periodic);
    assert.equal(coeffs.length, rows * cols);

    const rec = idwt2d(coeffs, rows, cols, "db2", 2, Periodic);
    assert.equal(rec.length, image.length);
    for (let i = 0; i < image.length; i++) {
      assert.ok(Math.abs(image[i] - rec[i]) < 1e-4);
    }
  });

  it("dwt2d nonsquare 4x8", () => {
    const rows = 4;
    const cols = 8;
    const image = new Float64Array(rows * cols);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        image[r * cols + c] = r * 10 + c;
      }
    }

    const coeffs = dwt2d(image, rows, cols, "haar", 1, Symmetric);
    const rec = idwt2d(coeffs, rows, cols, "haar", 1, Symmetric);
    assert.equal(rec.length, image.length);
    for (let i = 0; i < image.length; i++) {
      assert.ok(Math.abs(image[i] - rec[i]) < 1e-5);
    }
  });
});
