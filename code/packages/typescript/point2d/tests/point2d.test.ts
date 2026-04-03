import { describe, it, expect } from "vitest";
import { Point, Rect } from "../src/index.js";

const EPS = 1e-9;

function approxEq(a: number, b: number): boolean {
  return Math.abs(a - b) < EPS;
}

function pointApproxEq(a: Point, b: Point): boolean {
  return approxEq(a.x, b.x) && approxEq(a.y, b.y);
}

describe("Point", () => {
  describe("construction", () => {
    it("origin returns (0,0)", () => {
      const o = Point.origin();
      expect(o.x).toBe(0);
      expect(o.y).toBe(0);
    });

    it("new creates point with given coords", () => {
      const p = new Point(3, -5);
      expect(p.x).toBe(3);
      expect(p.y).toBe(-5);
    });
  });

  describe("arithmetic", () => {
    it("add", () => {
      const a = new Point(1, 2);
      const b = new Point(3, 4);
      const c = a.add(b);
      expect(c.x).toBe(4);
      expect(c.y).toBe(6);
    });

    it("subtract", () => {
      const a = new Point(5, 7);
      const b = new Point(2, 3);
      const r = a.subtract(b);
      expect(r.x).toBe(3);
      expect(r.y).toBe(4);
    });

    it("scale", () => {
      const p = new Point(3, 4);
      expect(pointApproxEq(p.scale(2), new Point(6, 8))).toBe(true);
      expect(pointApproxEq(p.scale(0), Point.origin())).toBe(true);
      expect(pointApproxEq(p.scale(-1), new Point(-3, -4))).toBe(true);
    });

    it("negate", () => {
      const p = new Point(3, -4);
      expect(pointApproxEq(p.negate(), new Point(-3, 4))).toBe(true);
    });
  });

  describe("vector operations", () => {
    it("dot product of perpendicular vectors is zero", () => {
      const x = new Point(1, 0);
      const y = new Point(0, 1);
      expect(x.dot(y)).toBe(0);
    });

    it("dot product of parallel vectors is product of magnitudes", () => {
      const p = new Point(3, 0);
      const q = new Point(5, 0);
      expect(p.dot(q)).toBe(15);
    });

    it("cross product CCW is positive", () => {
      const x = new Point(1, 0);
      const y = new Point(0, 1);
      expect(x.cross(y)).toBe(1);
    });

    it("cross product CW is negative", () => {
      const x = new Point(1, 0);
      const y = new Point(0, 1);
      expect(y.cross(x)).toBe(-1);
    });

    it("magnitude of 3-4-5 triangle", () => {
      const p = new Point(3, 4);
      expect(approxEq(p.magnitude(), 5)).toBe(true);
    });

    it("magnitude of origin is zero", () => {
      expect(Point.origin().magnitude()).toBe(0);
    });

    it("magnitudeSquared is exact without sqrt", () => {
      const p = new Point(3, 4);
      expect(p.magnitudeSquared()).toBe(25);
    });

    it("normalize gives unit vector", () => {
      const p = new Point(3, 4);
      const n = p.normalize();
      expect(approxEq(n.x, 0.6)).toBe(true);
      expect(approxEq(n.y, 0.8)).toBe(true);
      expect(approxEq(n.magnitude(), 1)).toBe(true);
    });

    it("normalize of zero vector returns origin", () => {
      const n = Point.origin().normalize();
      expect(n.x).toBe(0);
      expect(n.y).toBe(0);
    });

    it("distance between two points", () => {
      const a = Point.origin();
      const b = new Point(3, 4);
      expect(approxEq(a.distance(b), 5)).toBe(true);
    });

    it("distanceSquared avoids sqrt", () => {
      const a = Point.origin();
      const b = new Point(3, 4);
      expect(a.distanceSquared(b)).toBe(25);
    });
  });

  describe("interpolation and direction", () => {
    it("lerp at t=0 returns self", () => {
      const a = new Point(1, 2);
      const b = new Point(5, 6);
      expect(pointApproxEq(a.lerp(b, 0), a)).toBe(true);
    });

    it("lerp at t=1 returns other", () => {
      const a = new Point(1, 2);
      const b = new Point(5, 6);
      expect(pointApproxEq(a.lerp(b, 1), b)).toBe(true);
    });

    it("lerp at t=0.5 returns midpoint", () => {
      const a = new Point(0, 0);
      const b = new Point(10, 10);
      expect(pointApproxEq(a.lerp(b, 0.5), new Point(5, 5))).toBe(true);
    });

    it("perpendicular of (1,0) is (0,1)", () => {
      const p = new Point(1, 0);
      expect(pointApproxEq(p.perpendicular(), new Point(0, 1))).toBe(true);
    });

    it("perpendicular of (0,1) is (-1,0)", () => {
      const p = new Point(0, 1);
      expect(pointApproxEq(p.perpendicular(), new Point(-1, 0))).toBe(true);
    });

    it("perpendicular twice is negate", () => {
      const p = new Point(3, 4);
      expect(pointApproxEq(p.perpendicular().perpendicular(), p.negate())).toBe(true);
    });

    it("angle of (1,0) is 0", () => {
      expect(approxEq(new Point(1, 0).angle(), 0)).toBe(true);
    });

    it("angle of (0,1) is π/2", () => {
      expect(approxEq(new Point(0, 1).angle(), Math.PI / 2)).toBe(true);
    });

    it("angle of (-1,0) is ±π", () => {
      expect(approxEq(Math.abs(new Point(-1, 0).angle()), Math.PI)).toBe(true);
    });

    it("angle of (0,-1) is -π/2", () => {
      expect(approxEq(new Point(0, -1).angle(), -Math.PI / 2)).toBe(true);
    });
  });
});

describe("Rect", () => {
  describe("construction", () => {
    it("new creates rect with given values", () => {
      const r = new Rect(1, 2, 10, 5);
      expect(r.x).toBe(1);
      expect(r.y).toBe(2);
      expect(r.width).toBe(10);
      expect(r.height).toBe(5);
    });

    it("fromPoints computes width and height", () => {
      const r = Rect.fromPoints(new Point(1, 2), new Point(11, 7));
      expect(r.x).toBe(1);
      expect(r.y).toBe(2);
      expect(r.width).toBe(10);
      expect(r.height).toBe(5);
    });

    it("zero returns all-zero rect", () => {
      const r = Rect.zero();
      expect(r.x).toBe(0);
      expect(r.width).toBe(0);
    });
  });

  describe("accessors", () => {
    it("minPoint, maxPoint, center", () => {
      const r = new Rect(2, 3, 8, 4);
      expect(pointApproxEq(r.minPoint(), new Point(2, 3))).toBe(true);
      expect(pointApproxEq(r.maxPoint(), new Point(10, 7))).toBe(true);
      expect(pointApproxEq(r.center(), new Point(6, 5))).toBe(true);
    });
  });

  describe("predicates", () => {
    it("isEmpty for zero rect", () => {
      expect(Rect.zero().isEmpty()).toBe(true);
    });

    it("isEmpty for negative dimension", () => {
      expect(new Rect(0, 0, -1, 5).isEmpty()).toBe(true);
    });

    it("not empty for positive dimensions", () => {
      expect(new Rect(0, 0, 5, 5).isEmpty()).toBe(false);
    });

    it("containsPoint inside", () => {
      const r = new Rect(0, 0, 10, 10);
      expect(r.containsPoint(new Point(5, 5))).toBe(true);
    });

    it("containsPoint top-left corner inclusive", () => {
      const r = new Rect(0, 0, 10, 10);
      expect(r.containsPoint(new Point(0, 0))).toBe(true);
    });

    it("containsPoint right edge exclusive", () => {
      const r = new Rect(0, 0, 10, 10);
      expect(r.containsPoint(new Point(10, 5))).toBe(false);
    });

    it("containsPoint bottom edge exclusive", () => {
      const r = new Rect(0, 0, 10, 10);
      expect(r.containsPoint(new Point(5, 10))).toBe(false);
    });

    it("containsPoint outside", () => {
      const r = new Rect(0, 0, 10, 10);
      expect(r.containsPoint(new Point(-1, 5))).toBe(false);
      expect(r.containsPoint(new Point(5, -1))).toBe(false);
    });
  });

  describe("set operations", () => {
    it("union non-overlapping rects", () => {
      const a = new Rect(0, 0, 5, 5);
      const b = new Rect(10, 10, 5, 5);
      const u = a.union(b);
      expect(approxEq(u.x, 0)).toBe(true);
      expect(approxEq(u.width, 15)).toBe(true);
      expect(approxEq(u.height, 15)).toBe(true);
    });

    it("union with empty returns the other", () => {
      const a = new Rect(1, 2, 5, 5);
      const empty = Rect.zero();
      const u = a.union(empty);
      expect(u.x).toBe(a.x);
      expect(u.width).toBe(a.width);
    });

    it("intersection overlapping rects", () => {
      const a = new Rect(0, 0, 10, 10);
      const b = new Rect(5, 5, 10, 10);
      const i = a.intersection(b)!;
      expect(approxEq(i.x, 5)).toBe(true);
      expect(approxEq(i.y, 5)).toBe(true);
      expect(approxEq(i.width, 5)).toBe(true);
      expect(approxEq(i.height, 5)).toBe(true);
    });

    it("intersection non-overlapping returns null", () => {
      const a = new Rect(0, 0, 5, 5);
      const b = new Rect(10, 10, 5, 5);
      expect(a.intersection(b)).toBeNull();
    });

    it("intersection touching edge returns null", () => {
      const a = new Rect(0, 0, 5, 5);
      const b = new Rect(5, 0, 5, 5);
      expect(a.intersection(b)).toBeNull();
    });

    it("expandBy positive amount", () => {
      const r = new Rect(1, 1, 8, 8);
      const e = r.expandBy(1);
      expect(approxEq(e.x, 0)).toBe(true);
      expect(approxEq(e.y, 0)).toBe(true);
      expect(approxEq(e.width, 10)).toBe(true);
      expect(approxEq(e.height, 10)).toBe(true);
    });

    it("expandBy negative amount shrinks", () => {
      const r = new Rect(0, 0, 10, 10);
      const s = r.expandBy(-1);
      expect(approxEq(s.x, 1)).toBe(true);
      expect(approxEq(s.width, 8)).toBe(true);
    });
  });
});
