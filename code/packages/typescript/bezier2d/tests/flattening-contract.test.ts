import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { Point } from "@coding-adventures/point2d";
import { CubicBezier, QuadraticBezier } from "../src/index.js";

type FixtureCase = {
  id: string;
  degree: 2 | 3;
  control_points: [number, number][];
  tolerance: number;
  expected_disposition: "chord" | "subdivide" | "invalid-tolerance";
  expected_termination?: "budget-error";
};

const fixture = JSON.parse(readFileSync(new URL(
  "../../../../specs/fixtures/bezier2d-flattening-v1/cases.json",
  import.meta.url,
), "utf8")) as { cases: FixtureCase[] };

function curveFor(testCase: FixtureCase): QuadraticBezier | CubicBezier {
  const points = testCase.control_points.map(([x, y]) => new Point(x, y));
  return testCase.degree === 2
    ? new QuadraticBezier(points[0], points[1], points[2])
    : new CubicBezier(points[0], points[1], points[2], points[3]);
}

describe("G2D02 safe flattening", () => {
  for (const testCase of fixture.cases) {
    it(testCase.id, () => {
      const curve = curveFor(testCase);
      if (testCase.expected_disposition === "invalid-tolerance") {
        expect(() => curve.toPolyline(testCase.tolerance)).toThrow(/tolerance/);
        return;
      }
      if (testCase.expected_termination === "budget-error") {
        expect(() => curve.toPolyline(testCase.tolerance)).toThrow(/budget/);
        return;
      }

      const points = curve.toPolyline(testCase.tolerance);
      expect(points[0]).toBe(curve.p0);
      expect(points.at(-1)).toBe(testCase.degree === 2 ? curve.p2 : curve.p3);
      expect(points.every((point) => Number.isFinite(point.x) && Number.isFinite(point.y))).toBe(true);
      expect(testCase.expected_disposition === "chord" ? points.length === 2 : points.length > 2).toBe(true);
    });
  }

  it.each([0, -1, Infinity, -Infinity, NaN])("rejects tolerance %s", (tolerance) => {
    const curve = new QuadraticBezier(new Point(0, 0), new Point(0.5, 0), new Point(1, 0));
    expect(() => curve.toPolyline(tolerance)).toThrow(/tolerance/);
  });

  it.each([Infinity, -Infinity, NaN])("rejects control %s", (coordinate) => {
    const curve = new CubicBezier(
      new Point(0, 0), new Point(coordinate, 1), new Point(0, -1), new Point(0, 0),
    );
    expect(() => curve.toPolyline(0.1)).toThrow(/control/);
  });

  it("fails closed on derived overflow", () => {
    const curve = new QuadraticBezier(new Point(1e308, 0), new Point(0, 1), new Point(-1e308, 0));
    expect(() => curve.toPolyline(0.1)).toThrow(/finite/);
  });

  it("does not underflow subnormal geometry into a chord", () => {
    const curve = new QuadraticBezier(new Point(0, 0), new Point(0, 1e-200), new Point(1e-200, 0));
    expect(() => curve.toPolyline(Number.MIN_VALUE)).toThrow(/budget/);
  });
});
