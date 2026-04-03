import { describe, it, expect } from "vitest";
import { Affine2D } from "../src/index.js";
import { Point } from "@coding-adventures/point2d";

const EPS = 1e-9;

function approxEq(a: number, b: number): boolean {
  return Math.abs(a - b) < EPS;
}

function pointApproxEq(a: Point, b: Point): boolean {
  return approxEq(a.x, b.x) && approxEq(a.y, b.y);
}

function affineApproxEq(a: Affine2D, b: Affine2D): boolean {
  return (
    approxEq(a.a, b.a) && approxEq(a.b, b.b) &&
    approxEq(a.c, b.c) && approxEq(a.d, b.d) &&
    approxEq(a.e, b.e) && approxEq(a.f, b.f)
  );
}

describe("Affine2D", () => {
  describe("factory functions", () => {
    it("identity has correct components", () => {
      const id = Affine2D.identity();
      expect(id.a).toBe(1); expect(id.b).toBe(0);
      expect(id.c).toBe(0); expect(id.d).toBe(1);
      expect(id.e).toBe(0); expect(id.f).toBe(0);
    });

    it("identity leaves point unchanged", () => {
      const id = Affine2D.identity();
      const p = new Point(3, 4);
      expect(pointApproxEq(id.applyToPoint(p), p)).toBe(true);
    });

    it("translate moves a point", () => {
      const t = Affine2D.translate(5, -3);
      const q = t.applyToPoint(new Point(1, 2));
      expect(approxEq(q.x, 6)).toBe(true);
      expect(approxEq(q.y, -1)).toBe(true);
    });

    it("translate does not affect vectors", () => {
      const t = Affine2D.translate(100, 200);
      const v = new Point(1, 1);
      expect(pointApproxEq(t.applyToVector(v), v)).toBe(true);
    });

    it("rotate 90 degrees CCW", () => {
      const r = Affine2D.rotate(Math.PI / 2);
      const q = r.applyToPoint(new Point(1, 0));
      expect(approxEq(q.x, 0)).toBe(true);
      expect(approxEq(q.y, 1)).toBe(true);
    });

    it("rotate 360 degrees is identity", () => {
      expect(Affine2D.rotate(2 * Math.PI).isIdentity()).toBe(true);
    });

    it("rotateAround keeps center fixed", () => {
      const center = new Point(1, 0);
      const r = Affine2D.rotateAround(center, Math.PI / 2);
      const q = r.applyToPoint(center);
      expect(approxEq(q.x, 1)).toBe(true);
      expect(approxEq(q.y, 0)).toBe(true);
    });

    it("scale stretches a point", () => {
      const s = Affine2D.scale(2, 3);
      const q = s.applyToPoint(new Point(1, 1));
      expect(approxEq(q.x, 2)).toBe(true);
      expect(approxEq(q.y, 3)).toBe(true);
    });

    it("scaleUniform", () => {
      const s = Affine2D.scaleUniform(5);
      const q = s.applyToPoint(new Point(2, 3));
      expect(approxEq(q.x, 10)).toBe(true);
      expect(approxEq(q.y, 15)).toBe(true);
    });

    it("skewX by 45 degrees", () => {
      const sk = Affine2D.skewX(Math.PI / 4);
      const q = sk.applyToPoint(new Point(0, 1));
      expect(approxEq(q.x, 1)).toBe(true); // tan(45)*1
      expect(approxEq(q.y, 1)).toBe(true);
    });

    it("skewY by 45 degrees", () => {
      const sk = Affine2D.skewY(Math.PI / 4);
      const q = sk.applyToPoint(new Point(1, 0));
      expect(approxEq(q.x, 1)).toBe(true);
      expect(approxEq(q.y, 1)).toBe(true); // tan(45)*1
    });
  });

  describe("composition", () => {
    it("multiply by identity is identity", () => {
      const m = Affine2D.translate(3, 4);
      const id = Affine2D.identity();
      expect(affineApproxEq(m.multiply(id), m)).toBe(true);
      expect(affineApproxEq(id.multiply(m), m)).toBe(true);
    });

    it("two 90-degree rotations equal 180 degrees", () => {
      const r90 = Affine2D.rotate(Math.PI / 2);
      const r180 = Affine2D.rotate(Math.PI);
      expect(affineApproxEq(r90.multiply(r90), r180)).toBe(true);
    });

    it("scale then translate", () => {
      const composed = Affine2D.translate(10, 0).multiply(Affine2D.scaleUniform(2));
      const q = composed.applyToPoint(new Point(1, 1));
      expect(approxEq(q.x, 12)).toBe(true);
      expect(approxEq(q.y, 2)).toBe(true);
    });
  });

  describe("determinant and invert", () => {
    it("determinant of identity is 1", () => {
      expect(approxEq(Affine2D.identity().determinant(), 1)).toBe(true);
    });

    it("determinant of scale(2,3) is 6", () => {
      expect(approxEq(Affine2D.scale(2, 3).determinant(), 6)).toBe(true);
    });

    it("rotation has determinant 1", () => {
      expect(approxEq(Affine2D.rotate(Math.PI / 3).determinant(), 1)).toBe(true);
    });

    it("invert of identity is identity", () => {
      const inv = Affine2D.identity().invert()!;
      expect(affineApproxEq(inv, Affine2D.identity())).toBe(true);
    });

    it("multiply with inverse gives identity", () => {
      const t = Affine2D.translate(3, -7);
      const composed = t.multiply(t.invert()!);
      expect(composed.isIdentity()).toBe(true);
    });

    it("singular matrix returns null inverse", () => {
      const singular = new Affine2D(0, 0, 0, 0, 0, 0);
      expect(singular.invert()).toBeNull();
    });
  });

  describe("predicates", () => {
    it("isIdentity for identity", () => {
      expect(Affine2D.identity().isIdentity()).toBe(true);
    });

    it("not isIdentity for translate", () => {
      expect(Affine2D.translate(1, 0).isIdentity()).toBe(false);
    });

    it("isTranslationOnly for identity", () => {
      expect(Affine2D.identity().isTranslationOnly()).toBe(true);
    });

    it("isTranslationOnly for pure translate", () => {
      expect(Affine2D.translate(5, 3).isTranslationOnly()).toBe(true);
    });

    it("not isTranslationOnly for rotate", () => {
      expect(Affine2D.rotate(0.1).isTranslationOnly()).toBe(false);
    });

    it("toArray returns 6 elements", () => {
      const m = new Affine2D(1, 2, 3, 4, 5, 6);
      expect(m.toArray()).toEqual([1, 2, 3, 4, 5, 6]);
    });
  });
});
