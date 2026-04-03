// ============================================================================
// @coding-adventures/affine2d — 2D Affine Transformation Matrix
// ============================================================================
//
// This module provides `Affine2D`: the standard 6-float representation of any
// 2D affine transform, used by SVG, HTML Canvas, PDF, Cairo, Core Graphics.
//
// The 3×3 matrix (with implicit third row [0,0,1]):
//   ┌ a  c  e ┐   ┌ x ┐   ┌ ax + cy + e ┐
//   │ b  d  f │ × │ y │ = │ bx + dy + f │
//   └ 0  0  1 ┘   └ 1 ┘   └      1      ┘

import { sin, cos, tan } from "trig";
import { Point } from "@coding-adventures/point2d";

/** A 2D affine transformation matrix stored as [a, b, c, d, e, f]. */
export class Affine2D {
  constructor(
    /** Horizontal scale / cos(rotation). */
    public readonly a: number,
    /** Vertical shear / sin(rotation). */
    public readonly b: number,
    /** Horizontal shear / -sin(rotation). */
    public readonly c: number,
    /** Vertical scale / cos(rotation). */
    public readonly d: number,
    /** Horizontal translation. */
    public readonly e: number,
    /** Vertical translation. */
    public readonly f: number
  ) {}

  // -----------------------------------------------------------------------
  // Factory Functions
  // -----------------------------------------------------------------------

  /** Identity transform: leaves every point unchanged. [1,0,0,1,0,0] */
  static identity(): Affine2D {
    return new Affine2D(1, 0, 0, 1, 0, 0);
  }

  /** Pure translation by (tx, ty). */
  static translate(tx: number, ty: number): Affine2D {
    return new Affine2D(1, 0, 0, 1, tx, ty);
  }

  /**
   * CCW rotation by angle radians.
   * a=cos(θ), b=sin(θ), c=-sin(θ), d=cos(θ), e=0, f=0.
   */
  static rotate(angle: number): Affine2D {
    const c = cos(angle);
    const s = sin(angle);
    return new Affine2D(c, s, -s, c, 0, 0);
  }

  /** Rotation about an arbitrary center point. */
  static rotateAround(center: Point, angle: number): Affine2D {
    return Affine2D.translate(-center.x, -center.y)
      .then(Affine2D.rotate(angle))
      .then(Affine2D.translate(center.x, center.y));
  }

  /** Non-uniform scale: (sx, sy) in each axis. */
  static scale(sx: number, sy: number): Affine2D {
    return new Affine2D(sx, 0, 0, sy, 0, 0);
  }

  /** Uniform scale: same factor in both axes. */
  static scaleUniform(s: number): Affine2D {
    return Affine2D.scale(s, s);
  }

  /** Horizontal skew (shear along X) by angle radians. */
  static skewX(angle: number): Affine2D {
    return new Affine2D(1, 0, tan(angle), 1, 0, 0);
  }

  /** Vertical skew (shear along Y) by angle radians. */
  static skewY(angle: number): Affine2D {
    return new Affine2D(1, tan(angle), 0, 1, 0, 0);
  }

  // -----------------------------------------------------------------------
  // Composition
  // -----------------------------------------------------------------------

  /**
   * Apply `next` after `this`. Returns the composed transform.
   * `this.then(next)` first applies `this`, then `next`.
   */
  then(next: Affine2D): Affine2D {
    return next.multiply(this);
  }

  /**
   * Compose: `this` applied after `other`.
   *
   * Given A (this) and B (other), result = A·B so:
   *   result.a = a1*a2 + c1*b2  etc.
   */
  multiply(other: Affine2D): Affine2D {
    return new Affine2D(
      this.a * other.a + this.c * other.b,
      this.b * other.a + this.d * other.b,
      this.a * other.c + this.c * other.d,
      this.b * other.c + this.d * other.d,
      this.a * other.e + this.c * other.f + this.e,
      this.b * other.e + this.d * other.f + this.f
    );
  }

  // -----------------------------------------------------------------------
  // Application
  // -----------------------------------------------------------------------

  /** Apply to a point (including translation): x'=ax+cy+e, y'=bx+dy+f. */
  applyToPoint(p: Point): Point {
    return new Point(
      this.a * p.x + this.c * p.y + this.e,
      this.b * p.x + this.d * p.y + this.f
    );
  }

  /** Apply to a vector (ignoring translation): x'=ax+cy, y'=bx+dy. */
  applyToVector(v: Point): Point {
    return new Point(
      this.a * v.x + this.c * v.y,
      this.b * v.x + this.d * v.y
    );
  }

  // -----------------------------------------------------------------------
  // Properties
  // -----------------------------------------------------------------------

  /** Determinant of the 2×2 linear part: a*d - b*c. */
  determinant(): number {
    return this.a * this.d - this.b * this.c;
  }

  /**
   * Inverse of this transform, or null if singular (det ≈ 0).
   *
   * inv_a = d/det, inv_b = -b/det, inv_c = -c/det, inv_d = a/det,
   * inv_e = (c*f - d*e)/det, inv_f = (b*e - a*f)/det.
   */
  invert(): Affine2D | null {
    const det = this.determinant();
    if (Math.abs(det) < 1e-12) return null;
    return new Affine2D(
      this.d / det,
      -this.b / det,
      -this.c / det,
      this.a / det,
      (this.c * this.f - this.d * this.e) / det,
      (this.b * this.e - this.a * this.f) / det
    );
  }

  /** True if this is approximately the identity (within 1e-10). */
  isIdentity(): boolean {
    const eps = 1e-10;
    return (
      Math.abs(this.a - 1) < eps &&
      Math.abs(this.b) < eps &&
      Math.abs(this.c) < eps &&
      Math.abs(this.d - 1) < eps &&
      Math.abs(this.e) < eps &&
      Math.abs(this.f) < eps
    );
  }

  /** True if this is a pure translation (a≈1, b≈0, c≈0, d≈1). */
  isTranslationOnly(): boolean {
    const eps = 1e-10;
    return (
      Math.abs(this.a - 1) < eps &&
      Math.abs(this.b) < eps &&
      Math.abs(this.c) < eps &&
      Math.abs(this.d - 1) < eps
    );
  }

  /** Return the six components as [a, b, c, d, e, f]. */
  toArray(): [number, number, number, number, number, number] {
    return [this.a, this.b, this.c, this.d, this.e, this.f];
  }
}
