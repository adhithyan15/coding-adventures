import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  boxBlurKernel,
  box_blur_kernel,
  gaussianBlurKernel,
  gaussian_blur_kernel,
  laplacianKernel,
  laplacian_kernel,
  sharpenKernel,
  sharpen_kernel,
  sobelXKernel,
  sobel_x_kernel,
  sobelYKernel,
  sobel_y_kernel,
} from "../src/index.js";

describe("Image Filter Kernels", () => {
  it("gaussianBlurKernel generates normalized symmetric Gaussian weights", () => {
    for (const size of [3, 5, 7, 9]) {
      const k = gaussianBlurKernel(1.0, size);
      assert.equal(k.length, size);

      let sum = 0.0;
      for (let i = 0; i < size; i++) sum += k[i];
      assert.ok(Math.abs(sum - 1.0) < 1e-12, `sum ${sum} != 1.0`);

      // Symmetry
      for (let i = 0; i < size; i++) {
        assert.ok(Math.abs(k[i] - k[size - 1 - i]) < 1e-12);
      }
    }
    const alias = gaussian_blur_kernel(1.0, 5);
    assert.equal(alias.length, 5);
  });

  it("boxBlurKernel generates uniform weights summing to 1.0", () => {
    for (const size of [1, 3, 5, 10]) {
      const k = boxBlurKernel(size);
      assert.equal(k.length, size);
      for (let i = 0; i < size; i++) {
        assert.equal(k[i], 1.0 / size);
      }
      let sum = 0.0;
      for (let i = 0; i < size; i++) sum += k[i];
      assert.ok(Math.abs(sum - 1.0) < 1e-12);
    }
    const alias = box_blur_kernel(4);
    assert.equal(alias.length, 4);
  });

  it("sobel kernels match mathematical specification", () => {
    const sx = sobelXKernel();
    const sxAlias = sobel_x_kernel();
    assert.deepEqual(Array.from(sx), [-1, 0, 1, -2, 0, 2, -1, 0, 1]);
    assert.deepEqual(Array.from(sxAlias), Array.from(sx));

    const sy = sobelYKernel();
    const syAlias = sobel_y_kernel();
    assert.deepEqual(Array.from(sy), [-1, -2, -1, 0, 0, 0, 1, 2, 1]);
    assert.deepEqual(Array.from(syAlias), Array.from(sy));
  });

  it("laplacianKernel matches second-derivative operator summing to 0", () => {
    const lap = laplacianKernel();
    const lapAlias = laplacian_kernel();
    assert.deepEqual(Array.from(lap), [0, 1, 0, 1, -4, 1, 0, 1, 0]);
    assert.deepEqual(Array.from(lapAlias), Array.from(lap));

    let sum = 0.0;
    for (let i = 0; i < lap.length; i++) sum += lap[i];
    assert.equal(sum, 0.0);
  });

  it("sharpenKernel scales with amount and preserves mean (sums to 1.0)", () => {
    for (const amount of [0.0, 0.5, 1.0, 2.0]) {
      const sh = sharpenKernel(amount);
      assert.equal(sh.length, 9);
      let sum = 0.0;
      for (let i = 0; i < sh.length; i++) sum += sh[i];
      assert.ok(Math.abs(sum - 1.0) < 1e-12);
    }
    const alias = sharpen_kernel(1.5);
    assert.equal(alias[4], 1.0 + 4.0 * 1.5);
  });
});
