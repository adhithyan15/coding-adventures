import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  BoundaryMode,
  ConvMode,
  conv2d,
  convolve2d,
  convolve_2d,
  sepConv2d,
  sep_conv2d,
} from "../src/index.js";

function assertClose(a: ArrayLike<number>, b: ArrayLike<number>, tol = 1e-10): void {
  assert.equal(a.length, b.length, `length mismatch: ${a.length} vs ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    const diff = Math.abs(Number(a[i]) - Number(b[i]));
    assert.ok(
      diff <= tol,
      `mismatch at ${i}: ${a[i]} vs ${b[i]} (diff: ${diff})`
    );
  }
}

describe("2-D and Separable Convolution", () => {
  it("conv2d with centered delta preserves image", () => {
    const img = [
      1, 2, 3,
      4, 5, 6,
      7, 8, 9,
    ];
    const identityKernel = [
      0, 0, 0,
      0, 1, 0,
      0, 0, 0,
    ];
    const out = conv2d(img, identityKernel, 3, 3, 3, 3, BoundaryMode.ZERO);
    assertClose(out, img);
  });

  it("sepConv2d matches 2D direct convolution for separable filter", () => {
    const img = [
      1, 2, 3, 4,
      5, 6, 7, 8,
      9, 10, 11, 12,
      13, 14, 15, 16,
    ];
    const hKer = [0.25, 0.5, 0.25];
    const vKer = [0.2, 0.6, 0.2];

    // Outer product 3x3 kernel
    const outer2d = new Float64Array(9);
    for (let r = 0; r < 3; r++) {
      for (let c = 0; c < 3; c++) {
        outer2d[r * 3 + c] = vKer[r] * hKer[c];
      }
    }

    const res2d = conv2d(img, outer2d, 4, 4, 3, 3, BoundaryMode.REPLICATE);
    const resSep = sepConv2d(img, hKer, vKer, 4, 4, BoundaryMode.REPLICATE);
    const resSepAlias = sep_conv2d(img, hKer, vKer, 4, 4, BoundaryMode.REPLICATE);

    assertClose(resSep, res2d);
    assertClose(resSepAlias, res2d);
  });

  it("convolve2d works on 2-D nested arrays and matches flat arrays", () => {
    const nestedImg = [
      [1, 2, 3],
      [4, 5, 6],
      [7, 8, 9],
    ];
    const nestedKer = [
      [0, 1],
      [1, 0],
    ];

    const resSame = convolve2d(nestedImg, nestedKer, ConvMode.SAME) as number[][];
    assert.equal(resSame.length, 3);
    assert.equal(resSame[0].length, 3);

    const resValid = convolve2d(nestedImg, nestedKer, ConvMode.VALID) as number[][];
    assert.equal(resValid.length, 2);
    assert.equal(resValid[0].length, 2);

    const resFull = convolve2d(nestedImg, nestedKer, ConvMode.FULL) as number[][];
    assert.equal(resFull.length, 4);
    assert.equal(resFull[0].length, 4);

    const flatImg = [1, 2, 3, 4, 5, 6, 7, 8, 9];
    const flatKer = [0, 1, 1, 0];
    const flatSame = convolve_2d(
      flatImg,
      flatKer,
      ConvMode.SAME,
      BoundaryMode.ZERO,
      3,
      3,
      2,
      2
    ) as Float64Array;
    assert.equal(flatSame.length, 9);
  });
});
