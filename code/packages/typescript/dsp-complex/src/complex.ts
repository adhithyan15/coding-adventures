import { InvalidInputError } from "./errors.js";

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
    if (!Number.isFinite(re) || !Number.isFinite(im)) {
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
    if (!Number.isFinite(r) || !Number.isFinite(theta)) {
      throw new InvalidInputError("Polar coordinates must be finite numbers");
    }
    if (r < 0) {
      throw new InvalidInputError("Magnitude r must be non-negative");
    }
    return new Complex(r * Math.cos(theta), r * Math.sin(theta));
  }

  add(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      return new Complex(this.re + other, this.im);
    }
    return new Complex(this.re + other.re, this.im + other.im);
  }

  sub(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      return new Complex(this.re - other, this.im);
    }
    return new Complex(this.re - other.re, this.im - other.im);
  }

  mul(other: Complex | number): Complex {
    if (typeof other === "number") {
      if (!Number.isFinite(other)) {
        throw new InvalidInputError("Operand must be a finite number");
      }
      return new Complex(this.re * other, this.im * other);
    }
    const r = this.re * other.re - this.im * other.im;
    const i = this.re * other.im + this.im * other.re;
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
    const denom = other.re * other.re + other.im * other.im;
    if (denom === 0) {
      throw new InvalidInputError("Division by zero complex number");
    }
    const r = (this.re * other.re + this.im * other.im) / denom;
    const i = (this.im * other.re - this.re * other.im) / denom;
    return new Complex(r, i);
  }

  scale(factor: number): Complex {
    if (!Number.isFinite(factor)) {
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
    if (!Number.isFinite(eps) || eps < 0) {
      throw new InvalidInputError(`eps must be a non-negative finite number; got ${eps}`);
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
