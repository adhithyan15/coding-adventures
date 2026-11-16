import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  dct2d,
  dct_2d,
  DctNorm,
  DctType,
  idct2d,
  idct_2d,
} from "../src/index.js";

describe("2-D Discrete Cosine Transform (dct2d and idct2d)", () => {
  it("inverts 2D DCT on square matrix 4x4", () => {
    const height = 4;
    const width = 4;
    const image = new Float64Array(height * width);
    for (let i = 0; i < height * width; i++) {
      image[i] = i * 1.5 - 3.0;
    }

    const transformed = dct2d(image, height, width, DctType.II, DctNorm.ORTHO);
    const recovered = idct2d(transformed, height, width, DctType.III, DctNorm.ORTHO);

    assert.equal(recovered.length, image.length);
    for (let i = 0; i < image.length; i++) {
      assert.ok(Math.abs(recovered[i] - image[i]) < 1e-11);
    }
  });

  it("inverts 2D DCT on non-square matrix 3x5", () => {
    const height = 3;
    const width = 5;
    const image = [
      1, 2, 3, 4, 5,
      6, 7, 8, 9, 10,
      11, 12, 13, 14, 15,
    ];

    const transformed = dct_2d(image, height, width, DctType.II, DctNorm.ORTHO);
    const recovered = idct_2d(transformed, height, width, DctType.III, DctNorm.ORTHO);

    assert.equal(recovered.length, image.length);
    for (let i = 0; i < image.length; i++) {
      assert.ok(Math.abs(recovered[i] - image[i]) < 1e-11);
    }
  });

  it("inverts with NONE norm on 2x2 matrix", () => {
    const height = 2;
    const width = 2;
    const image = [10, 20, 30, 40];
    const transformed = dct2d(image, height, width, DctType.II, DctNorm.NONE);
    const recovered = idct2d(transformed, height, width, DctType.III, DctNorm.NONE);
    for (let i = 0; i < image.length; i++) {
      assert.ok(Math.abs(recovered[i] - image[i]) < 1e-11);
    }
  });
});
