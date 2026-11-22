import { InvalidInputError, safeFormatValue } from "./errors.js";

/**
 * Compute complex division (ar + i*ai) / (br + i*bi) using Smith's scaling
 * formula so large finite operands (e.g. 1e200) do not overflow denominator squaring.
 */
export function smithComplexDiv(
  ar: number,
  ai: number,
  br: number,
  bi: number
): [number, number] {
  if (br === 0 && bi === 0) {
    throw new InvalidInputError("Division by zero complex number");
  }
  let re: number;
  let im: number;
  if (Math.abs(br) >= Math.abs(bi)) {
    const r = bi / br;
    const denom = br + bi * r;
    re = (ar + ai * r) / denom;
    im = (ai - ar * r) / denom;
  } else {
    const r = br / bi;
    const denom = bi + br * r;
    re = (ar * r + ai) / denom;
    im = (ai * r - ar) / denom;
  }
  if (!Number.isFinite(re) || !Number.isFinite(im)) {
    throw new InvalidInputError("Complex division overflowed to non-finite result");
  }
  return [re, im];
}

/**
 * Immutable Complex number representation with double precision (Float64).
 *
 * Conforms to VER15 security requirements:
 * - CWE-20 fail-closed validation of finite floats.
 * - Rejects NaN, +Infinity, and -Infinity on all constructor and method inputs.
 */
export class Complex {
  readonly re: number;
  readonly im: number;

  constructor(re: number = 0, im: number = 0) {
    if (
      typeof re !== "number" ||
      typeof im !== "number" ||
      !Number.isFinite(re) ||
      !Number.isFinite(im)
    ) {
      throw new InvalidInputError("Complex components must be finite numbers");
    }
    this.re = re;
    this.im = im;
  }

  static zero(): Complex {
    return new Complex(0, 0);
  }

  static one(): Complex {
    return new Complex(1, 0);
  }

  static i(): Complex {
    return new Complex(0, 1);
  }

  /**
   * Construct a Complex number from polar coordinates r * e^(i * theta).
   */
  static fromPolar(r: number, theta: number): Complex {
    if (
      typeof r !== "number" ||
      typeof theta !== "number" ||
      !Number.isFinite(r) ||
      !Number.isFinite(theta)
    ) {
      throw new InvalidInputError("Polar coordinates must be finite numbers");
    }
    if (r < 0) {
      throw new InvalidInputError("Magnitude r must be non-negative");
    }
    return new Complex(r * Math.cos(theta), r * Math.sin(theta));
  }

  private static unwrapComplex(other: unknown): { re: number; im: number } {
    if (
      other === null ||
      other === undefined ||
      typeof other !== "object" ||
      typeof (other as { re?: unknown }).re !== "number" ||
      typeof (other as { im?: unknown }).im !== "number"
    ) {
      throw new InvalidInputError("Operand must be a Complex or finite number");
    }
    return other as { re: number; im: number };
  }

  add(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      return new Complex(this.re + other, this.im);
    }
    const c = Complex.unwrapComplex(other);
    return new Complex(this.re + c.re, this.im + c.im);
  }

  sub(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      return new Complex(this.re - other, this.im);
    }
    const c = Complex.unwrapComplex(other);
    return new Complex(this.re - c.re, this.im - c.im);
  }

  mul(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      return new Complex(this.re * other, this.im * other);
    }
    const c = Complex.unwrapComplex(other);
    const r = this.re * c.re - this.im * c.im;
    const i = this.re * c.im + this.im * c.re;
    return new Complex(r, i);
  }

  div(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      if (other === 0) {
        throw new InvalidInputError("Division by zero");
      }
      return new Complex(this.re / other, this.im / other);
    }
    const c = Complex.unwrapComplex(other);
    const [r, i] = smithComplexDiv(this.re, this.im, c.re, c.im);
    return new Complex(r, i);
  }

  scale(factor: number): Complex {
    if (typeof factor !== "number" || !Number.isFinite(factor)) {
      throw new InvalidInputError("Scale factor must be a finite number");
    }
    return new Complex(this.re * factor, this.im * factor);
  }

  conj(): Complex {
    return new Complex(this.re, -this.im);
  }

  abs(): number {
    return Math.hypot(this.re, this.im);
  }

  absSq(): number {
    return this.re * this.re + this.im * this.im;
  }

  arg(): number {
    return Math.atan2(this.im, this.re);
  }

  exp(): Complex {
    const expRe = Math.exp(this.re);
    return new Complex(expRe * Math.cos(this.im), expRe * Math.sin(this.im));
  }

  neg(): Complex {
    return new Complex(-this.re, -this.im);
  }

  equals(other: Complex, eps: number = 1e-12): boolean {
    if (typeof eps !== "number" || !Number.isFinite(eps) || eps < 0) {
      throw new InvalidInputError(
        `eps must be a non-negative finite number; got ${safeFormatValue(eps)}`
      );
    }
    if (
      other === null ||
      other === undefined ||
      typeof other !== "object" ||
      typeof other.re !== "number" ||
      typeof other.im !== "number"
    ) {
      return false;
    }
    return Math.abs(this.re - other.re) <= eps && Math.abs(this.im - other.im) <= eps;
  }

  clone(): Complex {
    return new Complex(this.re, this.im);
  }

  toString(): string {
    if (this.im >= 0) {
      return `${this.re} + ${this.im}i`;
    }
    return `${this.re} - ${-this.im}i`;
  }
}
