import { Complex } from "./complex.js";
import { MAX_SAMPLES } from "./constants.js";
import {
  InvalidInputError,
  LengthMismatchError,
  OddInterleavedLengthError,
} from "./errors.js";

/**
 * Contiguous complex-valued array stored in interleaved [re_0, im_0, re_1, im_1, ...] Float64 format.
 *
 * Conforms to DSP00, DSP01, and VER15 security requirements:
 * - CWE-400 pre-flight bounds checking against MAX_SAMPLES before memory allocation.
 * - CWE-20 fail-closed validation rejecting NaN/±Infinity.
 * - CWE-674 strictly iterative loops with zero recursion.
 */
export class ComplexArray {
  private readonly _data: Float64Array;

  constructor(interleaved?: Float64Array | ArrayLike<number>) {
    if (!interleaved || interleaved.length === 0) {
      this._data = new Float64Array(0);
      return;
    }

    const rawLen = interleaved.length;
    if (rawLen % 2 !== 0) {
      throw new OddInterleavedLengthError(rawLen);
    }
    const n = rawLen / 2;
    if (n > MAX_SAMPLES) {
      throw new InvalidInputError(
        `complex sample count ${n} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
      );
    }

    const buf = new Float64Array(rawLen);
    for (let i = 0; i < rawLen; i++) {
      const val = Number(interleaved[i]);
      if (!Number.isFinite(val)) {
        throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
      }
      buf[i] = val;
    }

    this._data = buf;
  }

  static fromReal(real: Float64Array | ArrayLike<number>): ComplexArray {
    const n = real.length;
    if (n > MAX_SAMPLES) {
      throw new InvalidInputError(
        `real length ${n} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
      );
    }

    const buf = new Float64Array(2 * n);
    for (let i = 0; i < n; i++) {
      const val = Number(real[i]);
      if (!Number.isFinite(val)) {
        throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
      }
      buf[2 * i] = val;
      buf[2 * i + 1] = 0.0;
    }

    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = buf;
    return arr;
  }

  static fromRealImag(
    real: Float64Array | ArrayLike<number>,
    imag: Float64Array | ArrayLike<number>
  ): ComplexArray {
    if (real.length !== imag.length) {
      throw new LengthMismatchError(real.length, imag.length);
    }
    const n = real.length;
    if (n > MAX_SAMPLES) {
      throw new InvalidInputError(
        `array length ${n} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
      );
    }

    const buf = new Float64Array(2 * n);
    for (let i = 0; i < n; i++) {
      const r = Number(real[i]);
      const im = Number(imag[i]);
      if (!Number.isFinite(r) || !Number.isFinite(im)) {
        throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
      }
      buf[2 * i] = r;
      buf[2 * i + 1] = im;
    }

    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = buf;
    return arr;
  }

  static fromInterleaved(data: Float64Array | ArrayLike<number>): ComplexArray {
    return new ComplexArray(data);
  }

  static zeros(length: number): ComplexArray {
    if (!Number.isFinite(length) || !Number.isInteger(length) || length < 0) {
      throw new InvalidInputError("length must be a non-negative integer");
    }
    if (length > MAX_SAMPLES) {
      throw new InvalidInputError(
        `length ${length} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
      );
    }
    const buf = new Float64Array(2 * length);
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = buf;
    return arr;
  }

  get length(): number {
    return this._data.length / 2;
  }

  isEmpty(): boolean {
    return this._data.length === 0;
  }

  get shape(): [number, 2] {
    return [this.length, 2];
  }

  get(index: number): Complex {
    const n = this.length;
    let idx = index;
    if (idx < 0) {
      idx += n;
    }
    if (idx < 0 || idx >= n || !Number.isInteger(idx)) {
      throw new RangeError("ComplexArray index out of range");
    }
    return new Complex(this._data[2 * idx], this._data[2 * idx + 1]);
  }

  set(index: number, val: Complex | { re: number; im: number }): void {
    const n = this.length;
    let idx = index;
    if (idx < 0) {
      idx += n;
    }
    if (idx < 0 || idx >= n || !Number.isInteger(idx)) {
      throw new RangeError("ComplexArray index out of range");
    }
    if (!Number.isFinite(val.re) || !Number.isFinite(val.im)) {
      throw new InvalidInputError("Complex value components must be finite numbers");
    }
    this._data[2 * idx] = val.re;
    this._data[2 * idx + 1] = val.im;
  }

  real(): Float64Array {
    const n = this.length;
    const out = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      out[i] = this._data[2 * i];
    }
    return out;
  }

  imag(): Float64Array {
    const n = this.length;
    const out = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      out[i] = this._data[2 * i + 1];
    }
    return out;
  }

  magnitude(): Float64Array {
    const n = this.length;
    const out = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      out[i] = Math.hypot(this._data[2 * i], this._data[2 * i + 1]);
    }
    return out;
  }

  phase(): Float64Array {
    const n = this.length;
    const out = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      out[i] = Math.atan2(this._data[2 * i + 1], this._data[2 * i]);
    }
    return out;
  }

  conjugate(): ComplexArray {
    const n = this.length;
    const out = new Float64Array(2 * n);
    for (let i = 0; i < n; i++) {
      out[2 * i] = this._data[2 * i];
      out[2 * i + 1] = -this._data[2 * i + 1];
    }
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = out;
    return arr;
  }

  scale(factor: number): ComplexArray {
    if (!Number.isFinite(factor)) {
      throw new InvalidInputError("Scale factor must be a finite number");
    }
    const len = this._data.length;
    const out = new Float64Array(len);
    for (let i = 0; i < len; i++) {
      out[i] = this._data[i] * factor;
    }
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = out;
    return arr;
  }

  add(other: ComplexArray): ComplexArray {
    if (this.length !== other.length) {
      throw new LengthMismatchError(this.length, other.length);
    }
    const len = this._data.length;
    const out = new Float64Array(len);
    const otherData = other._data;
    for (let i = 0; i < len; i++) {
      out[i] = this._data[i] + otherData[i];
    }
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = out;
    return arr;
  }

  sub(other: ComplexArray): ComplexArray {
    if (this.length !== other.length) {
      throw new LengthMismatchError(this.length, other.length);
    }
    const len = this._data.length;
    const out = new Float64Array(len);
    const otherData = other._data;
    for (let i = 0; i < len; i++) {
      out[i] = this._data[i] - otherData[i];
    }
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = out;
    return arr;
  }

  mul(other: ComplexArray): ComplexArray {
    if (this.length !== other.length) {
      throw new LengthMismatchError(this.length, other.length);
    }
    const n = this.length;
    const out = new Float64Array(2 * n);
    const a = this._data;
    const b = other._data;
    for (let i = 0; i < n; i++) {
      const ar = a[2 * i];
      const ai = a[2 * i + 1];
      const br = b[2 * i];
      const bi = b[2 * i + 1];
      out[2 * i] = ar * br - ai * bi;
      out[2 * i + 1] = ar * bi + ai * br;
    }
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = out;
    return arr;
  }

  div(other: ComplexArray): ComplexArray {
    if (this.length !== other.length) {
      throw new LengthMismatchError(this.length, other.length);
    }
    const n = this.length;
    const out = new Float64Array(2 * n);
    const a = this._data;
    const b = other._data;
    for (let i = 0; i < n; i++) {
      const ar = a[2 * i];
      const ai = a[2 * i + 1];
      const br = b[2 * i];
      const bi = b[2 * i + 1];
      const denom = br * br + bi * bi;
      if (denom === 0) {
        throw new InvalidInputError("Division by zero in complex array element");
      }
      out[2 * i] = (ar * br + ai * bi) / denom;
      out[2 * i + 1] = (ai * br - ar * bi) / denom;
    }
    const arr = Object.create(ComplexArray.prototype) as ComplexArray;
    (arr as unknown as { _data: Float64Array })._data = out;
    return arr;
  }

  asInterleaved(): Float64Array {
    return new Float64Array(this._data);
  }

  toComplexList(): Complex[] {
    const n = this.length;
    const list: Complex[] = new Array(n);
    for (let i = 0; i < n; i++) {
      list[i] = new Complex(this._data[2 * i], this._data[2 * i + 1]);
    }
    return list;
  }

  *[Symbol.iterator](): IterableIterator<Complex> {
    const n = this.length;
    for (let i = 0; i < n; i++) {
      yield new Complex(this._data[2 * i], this._data[2 * i + 1]);
    }
  }

  equals(other: ComplexArray, eps: number = 1e-12): boolean {
    if (!Number.isFinite(eps) || eps < 0) {
      throw new InvalidInputError(`eps must be a non-negative finite number; got ${eps}`);
    }
    if (this.length !== other.length) {
      return false;
    }
    const len = this._data.length;
    const b = other._data;
    for (let i = 0; i < len; i++) {
      if (Math.abs(this._data[i] - b[i]) > eps) {
        return false;
      }
    }
    return true;
  }
}

/**
 * Alias for ComplexArray matching DSP00/DSP01 naming conventions across languages.
 */
export const ComplexTensor = ComplexArray;
export type ComplexTensor = ComplexArray;
