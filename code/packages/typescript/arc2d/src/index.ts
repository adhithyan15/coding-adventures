// ============================================================================
// @coding-adventures/arc2d — Elliptical Arcs
// ============================================================================
//
// Two arc parameterizations and conversion between them.

import { sin, cos, tan, sqrt, atan2, PI } from "trig";
import { Point, Rect } from "@coding-adventures/point2d";
import { CubicBezier } from "@coding-adventures/bezier2d";

// ============================================================================
// CenterArc
// ============================================================================

/** Elliptical arc in center form. */
export class CenterArc {
  constructor(
    public readonly center: Point,
    public readonly rx: number,
    public readonly ry: number,
    public readonly startAngle: number,
    public readonly sweepAngle: number,
    public readonly xRotation: number
  ) {}

  // Direct construction is kept for callers. Validation at each operation
  // stops NaN/Infinity from entering trigonometry and bounds segment work to
  // at most four cubics. Positive radii are a center-form requirement; SVG's
  // absolute-radius correction belongs to its separate endpoint conversion.
  private validate(): void {
    requireFinite(this.center.x, this.center.y, this.rx, this.ry,
      this.startAngle, this.sweepAngle, this.xRotation);
    if (this.rx <= 0 || this.ry <= 0) throw new RangeError("arc2d: invalid center arc");
    if (Math.abs(this.sweepAngle) > 2 * PI) throw new RangeError("arc2d: sweep exceeds one turn");
  }

  private pointAt(angle: number): Point {
    requireFinite(angle);
    return this.rotateTranslate(this.rx * cos(angle), this.ry * sin(angle));
  }

  private rotateTranslate(x: number, y: number): Point {
    const cosR = cos(this.xRotation), sinR = sin(this.xRotation);
    const px = cosR * x - sinR * y + this.center.x;
    const py = sinR * x + cosR * y + this.center.y;
    requireFinite(px, py);
    return new Point(px, py);
  }

  /** Evaluate at t ∈ [0,1]: start point at t=0, end at t=1. */
  evaluate(t: number): Point {
    this.validate();
    requireFinite(t);
    return this.pointAt(this.startAngle + t * this.sweepAngle);
  }

  /** Tangent vector at t (not normalized). */
  tangent(t: number): Point {
    this.validate();
    requireFinite(t);
    const angle = this.startAngle + t * this.sweepAngle;
    requireFinite(angle);
    const dxp = -this.rx * sin(angle) * this.sweepAngle;
    const dyp = this.ry * cos(angle) * this.sweepAngle;
    const cosR = cos(this.xRotation);
    const sinR = sin(this.xRotation);
    const x = cosR * dxp - sinR * dyp, y = sinR * dxp + cosR * dyp;
    requireFinite(x, y);
    return new Point(x, y);
  }

  /** Exact bounds from endpoints and the four possible rotated extrema. */
  boundingBox(): Rect {
    this.validate();
    const start = this.pointAt(this.startAngle);
    let minX = start.x, maxX = start.x;
    let minY = start.y, maxY = start.y;
    const include = (p: Point) => {
      minX = Math.min(minX, p.x); maxX = Math.max(maxX, p.x);
      minY = Math.min(minY, p.y); maxY = Math.max(maxY, p.y);
    };
    include(this.pointAt(this.startAngle + this.sweepAngle));
    if (this.sweepAngle !== 0) {
      // For x=A*cos(theta)+B*sin(theta), dx/dtheta=0 at
      // atan2(B,A) and that angle+PI. The y coordinate has its own pair.
      const cosR = cos(this.xRotation), sinR = sin(this.xRotation);
      const xAngle = atan2(-this.ry * sinR, this.rx * cosR);
      const yAngle = atan2(this.ry * cosR, this.rx * sinR);
      for (const candidate of [xAngle, xAngle + PI, yAngle, yAngle + PI]) {
        // JS remainder keeps the dividend's sign. Add one turn only for a
        // negative remainder, then compare distance in the sweep direction.
        let distance = (this.sweepAngle > 0
          ? candidate - this.startAngle
          : this.startAngle - candidate) % (2 * PI);
        if (distance < 0) distance += 2 * PI;
        if (distance <= Math.abs(this.sweepAngle)) include(this.pointAt(candidate));
      }
    }
    const width = maxX - minX, height = maxY - minY;
    requireFinite(width, height);
    return new Rect(minX, minY, width, height);
  }

  /** Approximate with cubic Bezier segments (≤90° each). */
  toCubicBeziers(): CubicBezier[] {
    this.validate();
    const maxSeg = PI / 2;
    const nSegs = Math.max(1, Math.ceil(Math.abs(this.sweepAngle) / maxSeg));
    const segSweep = this.sweepAngle / nSegs;
    const cosR = cos(this.xRotation);
    const sinR = sin(this.xRotation);
    const k = (4 / 3) * tan(segSweep / 4);

    const beziers: CubicBezier[] = [];

    for (let i = 0; i < nSegs; i++) {
      const alpha = this.startAngle + i * segSweep;
      const beta = alpha + segSweep;
      requireFinite(alpha, beta);
      const cosA = cos(alpha), sinA = sin(alpha);
      const cosB = cos(beta), sinB = sin(beta);

      const p0l = [this.rx * cosA, this.ry * sinA];
      const p3l = [this.rx * cosB, this.ry * sinB];
      const p1l = [p0l[0] + k * (-this.rx * sinA), p0l[1] + k * (this.ry * cosA)];
      const p2l = [p3l[0] - k * (-this.rx * sinB), p3l[1] - k * (this.ry * cosB)];

      const rt = (lx: number, ly: number) => {
        const x = cosR * lx - sinR * ly + this.center.x;
        const y = sinR * lx + cosR * ly + this.center.y;
        requireFinite(x, y);
        return new Point(x, y);
      };

      beziers.push(new CubicBezier(
        rt(p0l[0], p0l[1]), rt(p1l[0], p1l[1]),
        rt(p2l[0], p2l[1]), rt(p3l[0], p3l[1])
      ));
    }

    return beziers;
  }
}

function requireFinite(...values: number[]): void {
  if (!values.every(Number.isFinite)) throw new RangeError("arc2d: invalid center arc");
}

// ============================================================================
// SvgArc
// ============================================================================

/** Elliptical arc in SVG endpoint form (A command parameters). */
export class SvgArc {
  constructor(
    public readonly from: Point,
    public readonly to: Point,
    public readonly rx: number,
    public readonly ry: number,
    public readonly xRotation: number,
    public readonly largeArc: boolean,
    public readonly sweep: boolean
  ) {}

  /** Convert to center form using the W3C algorithm. Returns null if degenerate. */
  toCenterArc(): CenterArc | null {
    if (this.from.distanceSquared(this.to) < 1e-20) return null;
    if (Math.abs(this.rx) < 1e-10 || Math.abs(this.ry) < 1e-10) return null;

    const cosR = cos(this.xRotation);
    const sinR = sin(this.xRotation);

    const dx = (this.from.x - this.to.x) / 2;
    const dy = (this.from.y - this.to.y) / 2;
    const x1p = cosR * dx + sinR * dy;
    const y1p = -sinR * dx + cosR * dy;

    let rx = Math.abs(this.rx);
    let ry = Math.abs(this.ry);
    const lambda = (x1p / rx) ** 2 + (y1p / ry) ** 2;
    if (lambda > 1) {
      const sqL = sqrt(lambda);
      rx *= sqL;
      ry *= sqL;
    }

    const rx2 = rx * rx, ry2 = ry * ry;
    const x1p2 = x1p * x1p, y1p2 = y1p * y1p;
    const num = rx2 * ry2 - rx2 * y1p2 - ry2 * x1p2;
    const den = rx2 * y1p2 + ry2 * x1p2;

    const sq = Math.abs(den) < 1e-12 ? 0 : sqrt(Math.max(0, num / den));
    const sign = this.largeArc === this.sweep ? -1 : 1;

    const cxp = sign * sq * (rx * y1p / ry);
    const cyp = sign * sq * -(ry * x1p / rx);

    const midX = (this.from.x + this.to.x) / 2;
    const midY = (this.from.y + this.to.y) / 2;
    const cx = cosR * cxp - sinR * cyp + midX;
    const cy = sinR * cxp + cosR * cyp + midY;

    const ux = (x1p - cxp) / rx;
    const uy = (y1p - cyp) / ry;
    const vx = (-x1p - cxp) / rx;
    const vy = (-y1p - cyp) / ry;

    const startAngle = angleBetween(1, 0, ux, uy);
    let sweepAngle = angleBetween(ux, uy, vx, vy);

    if (!this.sweep && sweepAngle > 0) sweepAngle -= 2 * PI;
    if (this.sweep && sweepAngle < 0) sweepAngle += 2 * PI;

    return new CenterArc(new Point(cx, cy), rx, ry, startAngle, sweepAngle, this.xRotation);
  }

  toCubicBeziers(): CubicBezier[] {
    return this.toCenterArc()?.toCubicBeziers() ?? [];
  }

  evaluate(t: number): Point | null {
    return this.toCenterArc()?.evaluate(t) ?? this.from.lerp(this.to, t);
  }

  boundingBox(): Rect | null {
    return this.toCenterArc()?.boundingBox() ?? Rect.fromPoints(
      new Point(Math.min(this.from.x, this.to.x), Math.min(this.from.y, this.to.y)),
      new Point(Math.max(this.from.x, this.to.x), Math.max(this.from.y, this.to.y))
    );
  }
}

function angleBetween(ux: number, uy: number, vx: number, vy: number): number {
  return atan2(ux * vy - uy * vx, ux * vx + uy * vy);
}
