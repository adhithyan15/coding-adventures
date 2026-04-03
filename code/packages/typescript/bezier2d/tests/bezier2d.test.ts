import { describe, it, expect } from "vitest";
import { QuadraticBezier, CubicBezier } from "../src/index.js";
import { Point } from "@coding-adventures/point2d";

const EPS = 1e-9;
function approxEq(a: number, b: number) { return Math.abs(a - b) < EPS; }
function ptEq(a: Point, b: Point) { return approxEq(a.x, b.x) && approxEq(a.y, b.y); }

describe("QuadraticBezier", () => {
  const q = new QuadraticBezier(
    new Point(0, 0), new Point(1, 2), new Point(2, 0)
  );

  it("evaluates at endpoints", () => {
    expect(ptEq(q.evaluate(0), q.p0)).toBe(true);
    expect(ptEq(q.evaluate(1), q.p2)).toBe(true);
  });

  it("evaluates at midpoint correctly", () => {
    const mid = q.evaluate(0.5);
    expect(approxEq(mid.x, 1)).toBe(true);
    expect(approxEq(mid.y, 1)).toBe(true);
  });

  it("derivative at t=0 is 2*(p1-p0)", () => {
    const d = q.derivative(0);
    expect(approxEq(d.x, 2)).toBe(true);
    expect(approxEq(d.y, 4)).toBe(true);
  });

  it("split gives correct endpoints", () => {
    const [left, right] = q.split(0.5);
    const splitPt = q.evaluate(0.5);
    expect(ptEq(left.p2, splitPt)).toBe(true);
    expect(ptEq(right.p0, splitPt)).toBe(true);
    expect(ptEq(left.p0, q.p0)).toBe(true);
    expect(ptEq(right.p2, q.p2)).toBe(true);
  });

  it("toPolyline for straight line is 2 points", () => {
    const straight = new QuadraticBezier(
      new Point(0, 0), new Point(1, 0), new Point(2, 0)
    );
    const pts = straight.toPolyline(0.1);
    expect(pts.length).toBe(2);
  });

  it("toPolyline has correct endpoints", () => {
    const pts = q.toPolyline(0.1);
    expect(ptEq(pts[0], q.p0)).toBe(true);
    expect(ptEq(pts[pts.length - 1], q.p2)).toBe(true);
  });

  it("boundingBox contains endpoints", () => {
    const bb = q.boundingBox();
    expect(bb.x).toBeLessThanOrEqual(0);
    expect(bb.x + bb.width).toBeGreaterThanOrEqual(2);
  });

  it("elevate produces equivalent cubic", () => {
    const c = q.elevate();
    for (const t of [0, 0.25, 0.5, 0.75, 1]) {
      expect(Math.abs(q.evaluate(t).x - c.evaluate(t).x)).toBeLessThan(1e-9);
      expect(Math.abs(q.evaluate(t).y - c.evaluate(t).y)).toBeLessThan(1e-9);
    }
  });
});

describe("CubicBezier", () => {
  const c = new CubicBezier(
    new Point(0, 0), new Point(1, 2), new Point(3, 2), new Point(4, 0)
  );

  it("evaluates at endpoints", () => {
    expect(ptEq(c.evaluate(0), c.p0)).toBe(true);
    expect(ptEq(c.evaluate(1), c.p3)).toBe(true);
  });

  it("symmetric midpoint has x=2", () => {
    expect(approxEq(c.evaluate(0.5).x, 2)).toBe(true);
  });

  it("derivative of straight line is (3,0)", () => {
    const straight = new CubicBezier(
      new Point(0,0), new Point(1,0), new Point(2,0), new Point(3,0)
    );
    const d = straight.derivative(0);
    expect(approxEq(d.x, 3)).toBe(true);
    expect(approxEq(d.y, 0)).toBe(true);
  });

  it("split gives correct endpoints", () => {
    const [left, right] = c.split(0.5);
    const splitPt = c.evaluate(0.5);
    expect(ptEq(left.p3, splitPt)).toBe(true);
    expect(ptEq(right.p0, splitPt)).toBe(true);
    expect(ptEq(left.p0, c.p0)).toBe(true);
    expect(ptEq(right.p3, c.p3)).toBe(true);
  });

  it("toPolyline for straight line is 2 points", () => {
    const straight = new CubicBezier(
      new Point(0,0), new Point(1,0), new Point(2,0), new Point(3,0)
    );
    expect(straight.toPolyline(0.1).length).toBe(2);
  });

  it("toPolyline for curve has more than 2 points", () => {
    const pts = c.toPolyline(0.1);
    expect(pts.length).toBeGreaterThan(2);
    expect(ptEq(pts[0], c.p0)).toBe(true);
    expect(ptEq(pts[pts.length - 1], c.p3)).toBe(true);
  });

  it("boundingBox contains all sampled points", () => {
    const bb = c.boundingBox();
    for (let i = 0; i <= 20; i++) {
      const t = i / 20;
      const p = c.evaluate(t);
      expect(p.x).toBeGreaterThanOrEqual(bb.x - 1e-6);
      expect(p.x).toBeLessThanOrEqual(bb.x + bb.width + 1e-6);
      expect(p.y).toBeGreaterThanOrEqual(bb.y - 1e-6);
      expect(p.y).toBeLessThanOrEqual(bb.y + bb.height + 1e-6);
    }
  });
});
