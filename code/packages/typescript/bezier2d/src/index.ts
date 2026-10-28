// ============================================================================
// @coding-adventures/bezier2d — Quadratic and Cubic Bezier Curves
// ============================================================================
//
// Pure polynomial arithmetic on 2D points. No trig dependency for curve math
// (we use trig.sqrt only for the bounding box discriminant).

import { sqrt } from "trig";
import { Point, Rect } from "@coding-adventures/point2d";

const MAX_FLATTENING_DEPTH = 32;
const MAX_FLATTENING_SUBDIVISIONS = 65_535;

function finite(value: number): number {
  if (!Number.isFinite(value)) throw new Error("Bezier subdivision produced nonfinite geometry");
  return value;
}

function scaledDistance(left: Point, right: Point): number {
  const dx = finite(left.x - right.x);
  const dy = finite(left.y - right.y);
  const scale = Math.max(Math.abs(dx), Math.abs(dy));
  if (scale === 0) return 0;
  const nx = dx / scale;
  const ny = dy / scale;
  return finite(scale * Math.sqrt(finite(nx * nx + ny * ny)));
}

function distanceToSegment(point: Point, start: Point, end: Point): number {
  const dx = finite(end.x - start.x);
  const dy = finite(end.y - start.y);
  const scale = Math.max(Math.abs(dx), Math.abs(dy));
  if (scale === 0) return scaledDistance(point, start);
  const nx = dx / scale;
  const ny = dy / scale;
  const denominator = finite(nx * nx + ny * ny);
  const nox = finite(finite(point.x - start.x) / scale);
  const noy = finite(finite(point.y - start.y) / scale);
  const projection = Math.max(0, Math.min(1, finite(finite(nox * nx + noy * ny) / denominator)));
  const nearest = new Point(
    finite(start.x + projection * dx),
    finite(start.y + projection * dy),
  );
  return scaledDistance(point, nearest);
}

function flatten(
  curve: QuadraticBezier | CubicBezier,
  tolerance: number,
  controls: readonly Point[],
): Point[] {
  if (!Number.isFinite(tolerance) || tolerance <= 0) {
    throw new RangeError("tolerance must be finite and positive");
  }
  if (controls.some((point) => !Number.isFinite(point.x) || !Number.isFinite(point.y))) {
    throw new RangeError("control points must be finite");
  }

  const points = [controls[0]];
  const pending: [QuadraticBezier | CubicBezier, number][] = [[curve, 0]];
  let subdivisions = 0;
  while (pending.length > 0) {
    const [current, depth] = pending.pop()!;
    const endpoint = current instanceof QuadraticBezier ? current.p2 : current.p3;
    const offCurve = current instanceof QuadraticBezier ? [current.p1] : [current.p1, current.p2];
    const error = Math.max(...offCurve.map((point) => distanceToSegment(point, current.p0, endpoint)));
    if (error <= tolerance) {
      points.push(endpoint);
      continue;
    }
    if (depth >= MAX_FLATTENING_DEPTH || subdivisions >= MAX_FLATTENING_SUBDIVISIONS) {
      throw new Error("Bezier flattening exceeded the depth or subdivision budget");
    }
    subdivisions += 1;
    const [left, right] = current.split(0.5);
    pending.push([right, depth + 1], [left, depth + 1]);
  }
  return points;
}

// ============================================================================
// QuadraticBezier
// ============================================================================

/** A quadratic Bezier curve with three control points. */
export class QuadraticBezier {
  constructor(
    public readonly p0: Point,
    public readonly p1: Point,
    public readonly p2: Point
  ) {}

  /**
   * Evaluate at t ∈ [0,1] using de Casteljau:
   * q0 = lerp(p0,p1,t); q1 = lerp(p1,p2,t); return lerp(q0,q1,t).
   */
  evaluate(t: number): Point {
    const q0 = this.p0.lerp(this.p1, t);
    const q1 = this.p1.lerp(this.p2, t);
    return q0.lerp(q1, t);
  }

  /**
   * Derivative at t: 2 * lerp(p1-p0, p2-p1, t).
   */
  derivative(t: number): Point {
    const d0 = this.p1.subtract(this.p0);
    const d1 = this.p2.subtract(this.p1);
    return d0.lerp(d1, t).scale(2);
  }

  /**
   * Split at t into (left, right) sub-curves using de Casteljau.
   */
  split(t: number): [QuadraticBezier, QuadraticBezier] {
    const q0 = this.p0.lerp(this.p1, t);
    const q1 = this.p1.lerp(this.p2, t);
    const m = q0.lerp(q1, t);
    return [
      new QuadraticBezier(this.p0, q0, m),
      new QuadraticBezier(m, q1, this.p2),
    ];
  }

  /**
   * Adaptive polyline approximation within tolerance.
   * Splits at midpoint if the flatness error exceeds tolerance.
   */
  toPolyline(tolerance: number): Point[] {
    return flatten(this, tolerance, [this.p0, this.p1, this.p2]);
  }

  /** Tight axis-aligned bounding box. */
  boundingBox(): Rect {
    let minX = Math.min(this.p0.x, this.p2.x);
    let maxX = Math.max(this.p0.x, this.p2.x);
    let minY = Math.min(this.p0.y, this.p2.y);
    let maxY = Math.max(this.p0.y, this.p2.y);

    const denomX = this.p0.x - 2 * this.p1.x + this.p2.x;
    if (Math.abs(denomX) > 1e-12) {
      const tx = (this.p0.x - this.p1.x) / denomX;
      if (tx > 0 && tx < 1) {
        const px = this.evaluate(tx);
        minX = Math.min(minX, px.x);
        maxX = Math.max(maxX, px.x);
      }
    }

    const denomY = this.p0.y - 2 * this.p1.y + this.p2.y;
    if (Math.abs(denomY) > 1e-12) {
      const ty = (this.p0.y - this.p1.y) / denomY;
      if (ty > 0 && ty < 1) {
        const py = this.evaluate(ty);
        minY = Math.min(minY, py.y);
        maxY = Math.max(maxY, py.y);
      }
    }

    return new Rect(minX, minY, maxX - minX, maxY - minY);
  }

  /**
   * Degree elevation: convert to an equivalent cubic Bezier.
   * q1 = (1/3)*p0 + (2/3)*p1,  q2 = (2/3)*p1 + (1/3)*p2.
   */
  elevate(): CubicBezier {
    const q1 = this.p0.scale(1 / 3).add(this.p1.scale(2 / 3));
    const q2 = this.p1.scale(2 / 3).add(this.p2.scale(1 / 3));
    return new CubicBezier(this.p0, q1, q2, this.p2);
  }
}

// ============================================================================
// CubicBezier
// ============================================================================

/** A cubic Bezier curve with four control points. */
export class CubicBezier {
  constructor(
    public readonly p0: Point,
    public readonly p1: Point,
    public readonly p2: Point,
    public readonly p3: Point
  ) {}

  /**
   * Evaluate at t ∈ [0,1] using de Casteljau (three levels of lerp).
   */
  evaluate(t: number): Point {
    const p01 = this.p0.lerp(this.p1, t);
    const p12 = this.p1.lerp(this.p2, t);
    const p23 = this.p2.lerp(this.p3, t);
    const p012 = p01.lerp(p12, t);
    const p123 = p12.lerp(p23, t);
    return p012.lerp(p123, t);
  }

  /**
   * Derivative at t: 3 * quadratic Bezier of differences.
   */
  derivative(t: number): Point {
    const d0 = this.p1.subtract(this.p0);
    const d1 = this.p2.subtract(this.p1);
    const d2 = this.p3.subtract(this.p2);
    const oneT = 1 - t;
    const r = d0.scale(oneT * oneT)
      .add(d1.scale(2 * oneT * t))
      .add(d2.scale(t * t));
    return r.scale(3);
  }

  /**
   * Split at t into (left, right) using de Casteljau.
   */
  split(t: number): [CubicBezier, CubicBezier] {
    const p01 = this.p0.lerp(this.p1, t);
    const p12 = this.p1.lerp(this.p2, t);
    const p23 = this.p2.lerp(this.p3, t);
    const p012 = p01.lerp(p12, t);
    const p123 = p12.lerp(p23, t);
    const p0123 = p012.lerp(p123, t);
    return [
      new CubicBezier(this.p0, p01, p012, p0123),
      new CubicBezier(p0123, p123, p23, this.p3),
    ];
  }

  /** Adaptive polyline approximation within tolerance. */
  toPolyline(tolerance: number): Point[] {
    return flatten(this, tolerance, [this.p0, this.p1, this.p2, this.p3]);
  }

  /** Tight axis-aligned bounding box. */
  boundingBox(): Rect {
    let minX = Math.min(this.p0.x, this.p3.x);
    let maxX = Math.max(this.p0.x, this.p3.x);
    let minY = Math.min(this.p0.y, this.p3.y);
    let maxY = Math.max(this.p0.y, this.p3.y);

    for (const t of extremaOfCubicDerivative(this.p0.x, this.p1.x, this.p2.x, this.p3.x)) {
      const px = this.evaluate(t);
      minX = Math.min(minX, px.x);
      maxX = Math.max(maxX, px.x);
    }
    for (const t of extremaOfCubicDerivative(this.p0.y, this.p1.y, this.p2.y, this.p3.y)) {
      const py = this.evaluate(t);
      minY = Math.min(minY, py.y);
      maxY = Math.max(maxY, py.y);
    }

    return new Rect(minX, minY, maxX - minX, maxY - minY);
  }
}

// ============================================================================
// Helper
// ============================================================================

function extremaOfCubicDerivative(v0: number, v1: number, v2: number, v3: number): number[] {
  const a = -3 * v0 + 9 * v1 - 9 * v2 + 3 * v3;
  const b = 6 * v0 - 12 * v1 + 6 * v2;
  const c = -3 * v0 + 3 * v1;
  const roots: number[] = [];

  if (Math.abs(a) < 1e-12) {
    if (Math.abs(b) > 1e-12) {
      const t = -c / b;
      if (t > 0 && t < 1) roots.push(t);
    }
  } else {
    const disc = b * b - 4 * a * c;
    if (disc >= 0) {
      const sq = sqrt(disc);
      const t1 = (-b + sq) / (2 * a);
      const t2 = (-b - sq) / (2 * a);
      if (t1 > 0 && t1 < 1) roots.push(t1);
      if (t2 > 0 && t2 < 1) roots.push(t2);
    }
  }
  return roots;
}
