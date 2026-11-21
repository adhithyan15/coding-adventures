import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, it } from "node:test";
import { fileURLToPath } from "node:url";
import {
  BoundaryMode,
  boxBlurKernel,
  ConvError,
  ConvMode,
  conv1d,
  conv2d,
  convolve,
  convolve2d,
  correlate,
  EmptyKernelError,
  EmptySignalError,
  extendIndex,
  fftConvolve,
  gaussianBlurKernel,
  ImageSizeMismatchError,
  InvalidInputError,
  KernelTooLargeError,
  MAX_KERNEL_LEN,
  MAX_SAMPLES,
  nextPow2,
  overlapAdd,
  parseBoundaryMode,
  parseConvMode,
  safeFormatValue,
  sample,
  sepConv2d,
  sharpenKernel,
} from "../src/index.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe("Hostile inputs & VER15 Security Hardening (dsp-conv)", () => {
  it("rejects empty signals and empty kernels fail-closed (CWE-20)", () => {
    assert.throws(() => conv1d([], [1]), EmptySignalError);
    assert.throws(() => conv1d([1], []), EmptyKernelError);

    assert.throws(() => convolve([], [1]), EmptySignalError);
    assert.throws(() => convolve([1], []), EmptyKernelError);

    assert.throws(() => correlate([], [1]), EmptySignalError);
    assert.throws(() => correlate([1], []), EmptyKernelError);

    assert.throws(() => fftConvolve([], [1]), EmptySignalError);
    assert.throws(() => fftConvolve([1], []), EmptyKernelError);

    assert.throws(() => overlapAdd([], [1]), EmptySignalError);
    assert.throws(() => overlapAdd([1], []), EmptyKernelError);

    assert.throws(() => conv2d([], [1], 1, 1, 1, 1), ImageSizeMismatchError);
    assert.throws(() => conv2d([1], [], 1, 1, 1, 1), ImageSizeMismatchError);

    assert.throws(() => sepConv2d([], [1], [1], 1, 1), ImageSizeMismatchError);
    assert.throws(() => sepConv2d([1], [], [1], 1, 1), EmptyKernelError);
    assert.throws(() => sepConv2d([1], [1], [], 1, 1), EmptyKernelError);

    assert.throws(() => convolve2d([], [1]), EmptySignalError);
    assert.throws(() => convolve2d([1], []), EmptyKernelError);
  });

  it("rejects non-finite floats fail-closed (CWE-20)", () => {
    assert.throws(() => conv1d([NaN], [1]), InvalidInputError);
    assert.throws(() => conv1d([1], [Infinity]), InvalidInputError);
    assert.throws(() => conv1d([1], [-Infinity]), InvalidInputError);

    assert.throws(() => convolve([NaN], [1]), InvalidInputError);
    assert.throws(() => convolve([1], [NaN]), InvalidInputError);

    assert.throws(() => fftConvolve([NaN], [1]), InvalidInputError);
    assert.throws(() => fftConvolve([1], [Infinity]), InvalidInputError);

    assert.throws(() => overlapAdd([NaN], [1]), InvalidInputError);
    assert.throws(() => overlapAdd([1], [NaN]), InvalidInputError);

    assert.throws(() => conv2d([NaN], [1], 1, 1, 1, 1), InvalidInputError);
    assert.throws(() => conv2d([1], [NaN], 1, 1, 1, 1), InvalidInputError);

    assert.throws(() => sepConv2d([NaN], [1], [1], 1, 1), InvalidInputError);
    assert.throws(() => sepConv2d([1], [NaN], [1], 1, 1), InvalidInputError);
    assert.throws(() => sepConv2d([1], [1], [NaN], 1, 1), InvalidInputError);

    assert.throws(() => convolve2d([[NaN]], [[1]]), InvalidInputError);
    assert.throws(() => convolve2d([[1]], [[NaN]]), InvalidInputError);
  });

  it("enforces pre-flight defensive bounds without allocating (CWE-400)", () => {
    const fakeHugeSignal = { length: MAX_SAMPLES + 1, [0]: 1.0 };
    const fakeHugeKernel = { length: MAX_KERNEL_LEN + 1, [0]: 1.0 };

    assert.throws(() => conv1d(fakeHugeSignal, [1]), InvalidInputError);
    assert.throws(() => conv1d([1], fakeHugeKernel), InvalidInputError);

    assert.throws(() => convolve(fakeHugeSignal, [1]), InvalidInputError);
    assert.throws(() => convolve([1], fakeHugeKernel), InvalidInputError);

    assert.throws(() => correlate(fakeHugeSignal, [1]), InvalidInputError);
    assert.throws(() => correlate([1], fakeHugeKernel), InvalidInputError);

    // Sum exceeding MAX_SAMPLES in full mode
    const limitSignal = { length: MAX_SAMPLES, [0]: 1.0 };
    assert.throws(() => convolve(limitSignal, [1, 2], ConvMode.FULL), InvalidInputError);
    assert.throws(() => fftConvolve(limitSignal, [1, 2], ConvMode.FULL), InvalidInputError);
    assert.throws(() => overlapAdd(limitSignal, [1, 2], 32, ConvMode.FULL), InvalidInputError);

    assert.throws(() => fftConvolve(fakeHugeSignal, [1]), InvalidInputError);
    assert.throws(() => fftConvolve([1], fakeHugeKernel), InvalidInputError);

    assert.throws(() => overlapAdd(fakeHugeSignal, [1]), InvalidInputError);
    assert.throws(() => overlapAdd([1], fakeHugeKernel), InvalidInputError);

    assert.throws(
      () => conv2d(fakeHugeSignal, [1], 1, 1, 1, 1),
      ImageSizeMismatchError
    );
    assert.throws(
      () => conv2d([1], [1], MAX_SAMPLES + 1, 1, 1, 1),
      InvalidInputError
    );
    assert.throws(
      () => conv2d([1], [1], 1, 1, MAX_KERNEL_LEN + 1, 1),
      InvalidInputError
    );

    assert.throws(
      () => sepConv2d(fakeHugeSignal, [1], [1], 1, 1),
      ImageSizeMismatchError
    );
    assert.throws(
      () => sepConv2d([1], fakeHugeKernel, [1], 1, 1),
      InvalidInputError
    );
    assert.throws(
      () => sepConv2d([1], [1], fakeHugeKernel, 1, 1),
      InvalidInputError
    );
    assert.throws(
      () => sepConv2d([1], [1], [1], MAX_SAMPLES + 1, 1),
      InvalidInputError
    );

    assert.throws(
      () => convolve2d([1], [1], ConvMode.FULL, BoundaryMode.ZERO, MAX_SAMPLES + 1, 1, 1, 1),
      InvalidInputError
    );
    assert.throws(
      () => convolve2d([1], [1], ConvMode.FULL, BoundaryMode.ZERO, 1, 1, MAX_KERNEL_LEN + 1, 1),
      InvalidInputError
    );
  });

  it("rejects kernel larger than signal in valid mode (CWE-20)", () => {
    assert.throws(() => convolve([1, 2], [1, 2, 3], "valid"), KernelTooLargeError);
    assert.throws(() => fftConvolve([1, 2], [1, 2, 3], "valid"), KernelTooLargeError);
    assert.throws(() => overlapAdd([1, 2], [1, 2, 3], null, "valid"), KernelTooLargeError);

    assert.throws(
      () => conv2d([1, 2, 3, 4], [1, 2, 3], 2, 2, 3, 1),
      KernelTooLargeError
    );
    assert.throws(
      () => convolve2d([1, 2, 3, 4], [1, 2, 3], ConvMode.VALID, BoundaryMode.ZERO, 2, 2, 3, 1),
      KernelTooLargeError
    );
  });

  it("rejects invalid dimensions, modes, and parameters fail-closed", () => {
    assert.throws(() => parseBoundaryMode("invalid_boundary"), InvalidInputError);
    assert.throws(() => parseBoundaryMode(123 as any), InvalidInputError);
    assert.throws(() => parseConvMode("invalid_mode"), InvalidInputError);
    assert.throws(() => parseConvMode(null as any), InvalidInputError);

    assert.throws(() => extendIndex(0, 5, "unknown" as any), InvalidInputError);

    // Kernels validations
    assert.throws(() => gaussianBlurKernel(0.0, 5), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(-1.0, 5), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(NaN, 5), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(1e-200, 3), InvalidInputError); // underflow to 0/0
    assert.throws(() => gaussianBlurKernel(1.0, 4), InvalidInputError); // even
    assert.throws(() => gaussianBlurKernel(1.0, 0), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(1.0, -1), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(1.0, 1.5), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(1.0, MAX_KERNEL_LEN + 1), InvalidInputError);

    assert.throws(() => boxBlurKernel(0), InvalidInputError);
    assert.throws(() => boxBlurKernel(-3), InvalidInputError);
    assert.throws(() => boxBlurKernel(1.5), InvalidInputError);
    assert.throws(() => boxBlurKernel(MAX_KERNEL_LEN + 1), InvalidInputError);

    assert.throws(() => sharpenKernel(NaN), InvalidInputError);
    assert.throws(() => sharpenKernel(Infinity), InvalidInputError);

    // Overlap-add block size
    assert.throws(() => overlapAdd([1, 2], [1], 0), InvalidInputError);
    assert.throws(() => overlapAdd([1, 2], [1], -5), InvalidInputError);
    assert.throws(() => overlapAdd([1, 2], [1], 1.5), InvalidInputError);
    assert.throws(() => overlapAdd([1, 2], [1], MAX_SAMPLES + 1), InvalidInputError);

    // nextPow2 non-safe-integer / NaN guards
    assert.throws(() => nextPow2(NaN), InvalidInputError);
    assert.throws(() => nextPow2(Infinity), InvalidInputError);
    assert.throws(() => nextPow2(-Infinity), InvalidInputError);
    assert.throws(() => nextPow2(2.5), InvalidInputError);

    // 2-D dimension mismatch
    assert.throws(() => conv2d([1], [1], 0, 1, 1, 1), ImageSizeMismatchError);
    assert.throws(() => conv2d([1], [1], 1, 0, 1, 1), ImageSizeMismatchError);
    assert.throws(() => conv2d([1], [1], 1, 1, 0, 1), EmptyKernelError);
    assert.throws(() => conv2d([1], [1], 1, 1, 1, 0), EmptyKernelError);

    assert.throws(() => sepConv2d([1], [1], [1], 0, 1), ImageSizeMismatchError);
    assert.throws(() => sepConv2d([1], [1], [1], 1, 0), ImageSizeMismatchError);

    assert.throws(() => convolve2d([1, 2], [1]), ImageSizeMismatchError); // flat image missing dimensions
    assert.throws(() => convolve2d([1, 2], [1], ConvMode.FULL, BoundaryMode.ZERO, 1), ImageSizeMismatchError);
    assert.throws(() => convolve2d([1, 2], [1], ConvMode.FULL, BoundaryMode.ZERO, 1, 2, 1), ImageSizeMismatchError);
    assert.throws(() => convolve2d([[1, 2], [1]], [[1]]), ImageSizeMismatchError); // jagged
    assert.throws(() => convolve2d([[1, 2]], [[1, 2], [1]]), ImageSizeMismatchError); // jagged kernel
    assert.throws(() => convolve2d([1, 2], [1], ConvMode.FULL, BoundaryMode.ZERO, 1, 3, 1, 1), ImageSizeMismatchError); // img length mismatch
    assert.throws(() => convolve2d([1, 2], [1], ConvMode.FULL, BoundaryMode.ZERO, 1, 2, 1, 2), ImageSizeMismatchError); // ker length mismatch
    assert.throws(() => convolve2d([1, 2], [1], ConvMode.FULL, BoundaryMode.ZERO, -1, 2, 1, 1), ImageSizeMismatchError);
    assert.throws(() => convolve2d([1, 2], [1], ConvMode.FULL, BoundaryMode.ZERO, 1, 2, -1, 1), EmptyKernelError);
  });

  it("verifies error inheritance structure", () => {
    const err = new ConvError("base");
    assert.ok(err instanceof Error);
    assert.equal(err.name, "ConvError");

    const imgErr = new ImageSizeMismatchError("mismatch");
    assert.ok(imgErr instanceof ConvError);
    assert.equal(imgErr.name, "ImageSizeMismatchError");

    const kerErr = new KernelTooLargeError("too big");
    assert.ok(kerErr instanceof ConvError);
    assert.equal(kerErr.name, "KernelTooLargeError");
  });

  it("verifies required_capabilities.json is pure empty []", () => {
    const capPath = path.resolve(__dirname, "../../required_capabilities.json");
    const content = JSON.parse(fs.readFileSync(capPath, "utf-8"));
    assert.deepEqual(content.capabilities, []);
  });

  it("verifies src has zero forbidden imports or calls (VER08a / CWE-1333)", () => {
    const srcDir = path.resolve(__dirname, "../../src");
    const files = fs.readdirSync(srcDir).filter((f) => f.endsWith(".ts"));
    for (const f of files) {
      const code = fs.readFileSync(path.join(srcDir, f), "utf-8");
      assert.ok(!code.includes('from "node:fs"'));
      assert.ok(!code.includes('from "node:net"'));
      assert.ok(!code.includes('from "node:http"'));
      assert.ok(!code.includes('from "node:child_process"'));
      assert.ok(!code.includes("eval("));
      assert.ok(!code.includes("Function("));
      assert.ok(!code.includes("RegExp"));
    }
  });

  it("rejects Symbol, Object.create(null), DataView, BigInt64Array, null inputs, and oversized enum strings (VER15)", () => {
    const sym = Symbol("bad");
    const nullProto = Object.create(null);

    assert.equal(safeFormatValue(sym), "Symbol(bad)");
    assert.equal(safeFormatValue(nullProto), "[object Object]");
    assert.equal(safeFormatValue(null), "null");

    assert.throws(() => parseBoundaryMode(sym as any), InvalidInputError);
    assert.throws(() => parseBoundaryMode(nullProto as any), InvalidInputError);
    assert.throws(() => parseBoundaryMode(" ".repeat(65) + "zero"), InvalidInputError);
    assert.throws(() => parseConvMode(sym as any), InvalidInputError);
    assert.throws(() => parseConvMode(nullProto as any), InvalidInputError);
    assert.throws(() => parseConvMode(" ".repeat(65) + "full"), InvalidInputError);

    assert.throws(() => nextPow2(sym as any), InvalidInputError);
    assert.throws(() => nextPow2(nullProto as any), InvalidInputError);
    assert.throws(() => nextPow2(Number.MAX_SAFE_INTEGER + 1), InvalidInputError);

    assert.throws(() => overlapAdd([1, 2], [1], sym as any), InvalidInputError);
    assert.throws(() => overlapAdd([1, 2], [1], nullProto as any), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(sym as any, 3), InvalidInputError);
    assert.throws(() => gaussianBlurKernel(1.0, nullProto as any), InvalidInputError);
    assert.throws(() => boxBlurKernel(sym as any), InvalidInputError);
    assert.throws(() => sharpenKernel(sym as any), InvalidInputError);
    assert.throws(() => sharpenKernel(nullProto as any), InvalidInputError);

    assert.throws(() => conv2d([1], [1], sym as any, 1, 1, 1), ImageSizeMismatchError);
    assert.throws(() => sepConv2d([1], [1], [1], nullProto as any, 1), ImageSizeMismatchError);

    assert.throws(() => extendIndex(NaN, 5, BoundaryMode.ZERO), InvalidInputError);
    assert.throws(() => extendIndex(0, 0, BoundaryMode.ZERO), InvalidInputError);
    assert.throws(() => extendIndex(0, 5, nullProto as any), InvalidInputError);
    assert.throws(() => sample([sym as any], 0, BoundaryMode.ZERO), InvalidInputError);

    const dv = new DataView(new ArrayBuffer(16));
    const bi = new BigInt64Array(4);
    const bui = new BigUint64Array(4);

    assert.throws(() => conv1d(dv as any, [1]), InvalidInputError);
    assert.throws(() => conv1d(bi as any, [1]), InvalidInputError);
    assert.throws(() => conv1d([1], bui as any), InvalidInputError);
    assert.throws(() => conv1d(null as any, [1]), InvalidInputError);
    assert.throws(() => conv1d({} as any, [1]), InvalidInputError);
    assert.throws(() => conv1d([sym as any], [1]), InvalidInputError);
    assert.throws(() => conv1d([nullProto as any], [1]), InvalidInputError);
    assert.throws(() => correlate([1], [sym as any]), InvalidInputError);
    assert.throws(() => fftConvolve(dv as any, [1]), InvalidInputError);
    assert.throws(() => overlapAdd(bi as any, [1]), InvalidInputError);
    assert.throws(() => conv2d(dv as any, [1], 1, 1, 1, 1), InvalidInputError);
    assert.throws(() => sepConv2d(bi as any, [1], [1], 1, 1), InvalidInputError);
    assert.throws(() => convolve2d([[1], bi as any], [[1]]), ImageSizeMismatchError);
    assert.throws(() => convolve2d([[1]], [[1], dv as any]), ImageSizeMismatchError);
    assert.throws(() => convolve2d(dv as any, [[1]]), InvalidInputError);

    // Findings 3, 4, 5, 10 regressions
    assert.throws(() => sharpenKernel(5e307), InvalidInputError);
    assert.throws(() => sharpenKernel(-5e307), InvalidInputError);
    assert.throws(
      () => convolve2d([[1, 2, 3], [4, 5, 6]], [[1]], "same", "zero", 2, 2, 1, 1),
      ImageSizeMismatchError
    );
    assert.throws(
      () => convolve2d([[1, 2]], [[1]], "same", "zero", 3, 2, 1, 1),
      ImageSizeMismatchError
    );
    assert.throws(
      () => convolve2d([[1, 2]], [[1, 2]], "same", "zero", 1, 2, 1, 1),
      ImageSizeMismatchError
    );
    assert.throws(
      () => convolve2d([[1, 2]], [[1, 2]], "same", "zero", 1, 2, 2, 2),
      ImageSizeMismatchError
    );
    assert.throws(() => fftConvolve([1e308, 1e308], [10, 10]), (err) => err instanceof ConvError && err instanceof InvalidInputError);
    const nullCtorDv = Object.assign(new DataView(new ArrayBuffer(8)), { constructor: null });
    assert.throws(() => conv1d(nullCtorDv as any, [1]), InvalidInputError);
  });
});
