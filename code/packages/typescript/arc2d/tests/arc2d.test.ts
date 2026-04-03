import { describe, it, expect } from "vitest";
import { CenterArc, SvgArc } from "../src/index.js";
import { Point } from "@coding-adventures/point2d";
import { PI } from "trig";

const EPS = 1e-5;
function approxEq(a: number, b: number) { return Math.abs(a - b) < EPS; }
function ptEq(a: Point, b: Point) { return approxEq(a.x, b.x) && approxEq(a.y, b.y); }

describe("CenterArc", () => {
  it("unit circle quarter arc endpoints", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, PI / 2, 0);
    expect(ptEq(arc.evaluate(0), new Point(1, 0))).toBe(true);
    expect(ptEq(arc.evaluate(1), new Point(0, 1))).toBe(true);
  });

  it("full circle midpoint", () => {
    const arc = new CenterArc(Point.origin(), 2, 2, 0, 2 * PI, 0);
    const mid = arc.evaluate(0.5);
    expect(approxEq(mid.x, -2)).toBe(true);
    expect(approxEq(mid.y, 0)).toBe(true);
  });

  it("tangent at start of quarter circle points upward", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, PI / 2, 0);
    const t0 = arc.tangent(0);
    expect(approxEq(t0.x, 0)).toBe(true);
    expect(t0.y).toBeGreaterThan(0);
  });

  it("bounding box of unit circle is approximately [-1,-1,2,2]", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, 2 * PI, 0);
    const bb = arc.boundingBox();
    expect(Math.abs(bb.x + 1)).toBeLessThan(0.05);
    expect(Math.abs(bb.width - 2)).toBeLessThan(0.05);
  });

  it("quarter circle produces 1 bezier", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, PI / 2, 0);
    expect(arc.toCubicBeziers().length).toBe(1);
  });

  it("full circle produces 4 beziers", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, 2 * PI, 0);
    expect(arc.toCubicBeziers().length).toBe(4);
  });

  it("bezier approximation is accurate for quarter circle", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, PI / 2, 0);
    const b = arc.toCubicBeziers()[0];
    const arcMid = arc.evaluate(0.5);
    const bezMid = b.evaluate(0.5);
    expect(arcMid.distance(bezMid)).toBeLessThan(0.001);
  });

  it("adjacent beziers share endpoints", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, 2 * PI, 0);
    const bz = arc.toCubicBeziers();
    for (let i = 0; i < bz.length - 1; i++) {
      expect(bz[i].p3.distance(bz[i + 1].p0)).toBeLessThan(1e-6);
    }
  });
});

describe("SvgArc", () => {
  it("degenerate: same endpoints returns null", () => {
    const arc = new SvgArc(Point.origin(), Point.origin(), 1, 1, 0, false, true);
    expect(arc.toCenterArc()).toBeNull();
  });

  it("degenerate: zero radius returns null", () => {
    const arc = new SvgArc(new Point(0, 0), new Point(1, 0), 0, 1, 0, false, true);
    expect(arc.toCenterArc()).toBeNull();
  });

  it("quarter circle from (1,0) to (0,1) — center at origin", () => {
    const arc = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true);
    const ca = arc.toCenterArc()!;
    expect(approxEq(ca.center.x, 0)).toBe(true);
    expect(approxEq(ca.center.y, 0)).toBe(true);
  });

  it("CCW sweep gives positive sweep angle", () => {
    const arc = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true);
    const ca = arc.toCenterArc()!;
    expect(ca.sweepAngle).toBeGreaterThan(0);
  });

  it("CW sweep gives negative sweep angle", () => {
    const arc = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, false);
    const ca = arc.toCenterArc()!;
    expect(ca.sweepAngle).toBeLessThan(0);
  });

  it("large arc has |sweepAngle| > PI", () => {
    const arc = new SvgArc(new Point(1, 0), new Point(-1, 0), 1, 1, 0, true, true);
    const ca = arc.toCenterArc()!;
    expect(Math.abs(ca.sweepAngle)).toBeGreaterThan(PI - 1e-6);
  });

  it("evaluate delegates to center arc", () => {
    const arc = new SvgArc(new Point(1, 0), new Point(0, 1), 1, 1, 0, false, true);
    const p = arc.evaluate(0);
    expect(p).not.toBeNull();
    expect(approxEq(p!.x, 1)).toBe(true);
  });

  it("toCubicBeziers for degenerate returns empty", () => {
    const arc = new SvgArc(Point.origin(), Point.origin(), 1, 1, 0, false, true);
    expect(arc.toCubicBeziers()).toEqual([]);
  });

  it("boundingBox for valid arc returns non-null", () => {
    const arc = new SvgArc(new Point(1, 0), new Point(-1, 0), 1, 1, 0, true, true);
    expect(arc.boundingBox()).not.toBeNull();
  });
});
