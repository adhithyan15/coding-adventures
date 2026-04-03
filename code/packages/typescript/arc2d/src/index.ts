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

  /** Evaluate at t ∈ [0,1]: start point at t=0, end at t=1. */
  evaluate(t: number): Point {
    const angle = this.startAngle + t * this.sweepAngle;
    const xp = this.rx * cos(angle);
    const yp = this.ry * sin(angle);
    const cosR = cos(this.xRotation);
    const sinR = sin(this.xRotation);
    return new Point(
      cosR * xp - sinR * yp + this.center.x,
      sinR * xp + cosR * yp + this.center.y
    );
  }

  /** Tangent vector at t (not normalized). */
  tangent(t: number): Point {
    const angle = this.startAngle + t * this.sweepAngle;
    const dxp = -this.rx * sin(angle) * this.sweepAngle;
    const dyp = this.ry * cos(angle) * this.sweepAngle;
    const cosR = cos(this.xRotation);
    const sinR = sin(this.xRotation);
    return new Point(
      cosR * dxp - sinR * dyp,
      sinR * dxp + cosR * dyp
    );
  }

  /** Bounding box by sampling 100 points. */
  boundingBox(): Rect {
    const n = 100;
    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;
    for (let i = 0; i <= n; i++) {
      const p = this.evaluate(i / n);
      minX = Math.min(minX, p.x); maxX = Math.max(maxX, p.x);
      minY = Math.min(minY, p.y); maxY = Math.max(maxY, p.y);
    }
    return new Rect(minX, minY, maxX - minX, maxY - minY);
  }

  /** Approximate with cubic Bezier segments (≤90° each). */
  toCubicBeziers(): CubicBezier[] {
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
      const cosA = cos(alpha), sinA = sin(alpha);
      const cosB = cos(beta), sinB = sin(beta);

      const p0l = [this.rx * cosA, this.ry * sinA];
      const p3l = [this.rx * cosB, this.ry * sinB];
      const p1l = [p0l[0] + k * (-this.rx * sinA), p0l[1] + k * (this.ry * cosA)];
      const p2l = [p3l[0] - k * (-this.rx * sinB), p3l[1] - k * (this.ry * cosB)];

      const rt = (lx: number, ly: number) =>
        new Point(cosR * lx - sinR * ly + this.center.x, sinR * lx + cosR * ly + this.center.y);

      beziers.push(new CubicBezier(
        rt(p0l[0], p0l[1]), rt(p1l[0], p1l[1]),
        rt(p2l[0], p2l[1]), rt(p3l[0], p3l[1])
      ));
    }

    return beziers;
  }
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
    if (
      Math.abs(this.from.x - this.to.x) < 1e-12 &&
      Math.abs(this.from.y - this.to.y) < 1e-12
    ) return null;
    if (Math.abs(this.rx) < 1e-12 || Math.abs(this.ry) < 1e-12) return null;

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
    return this.toCenterArc()?.evaluate(t) ?? null;
  }

  boundingBox(): Rect | null {
    return this.toCenterArc()?.boundingBox() ?? null;
  }
}

function angleBetween(ux: number, uy: number, vx: number, vy: number): number {
  return atan2(ux * vy - uy * vx, ux * vx + uy * vy);
}
