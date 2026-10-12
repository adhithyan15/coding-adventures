import { describe, expect, it } from "vitest";
import { Point } from "@coding-adventures/point2d";
import { PI } from "trig";
import { CenterArc } from "../src/index.js";
import fixtures from "../../../../specs/fixtures/geometry2d-v1/cases.json";

// The neutral oracle lives outside this package. Select by operation at runtime
// so a new record cannot silently become an untested, copied expectation.
type CenterCase = {
  id: string; operation: string; center: number[]; rx: number; ry: number;
  start_angle: number; sweep_angle: number; x_rotation: number;
  expected_bounds?: number[]; expected_count?: number; expected_error?: string;
};
const centerCases = fixtures.cases.filter((entry) =>
  entry.operation === "center-arc-bounds" || entry.operation === "center-arc-cubics"
) as CenterCase[];
const fromCase = (entry: CenterCase) => new CenterArc(
  new Point(entry.center[0], entry.center[1]), entry.rx, entry.ry,
  entry.start_angle, entry.sweep_angle, entry.x_rotation
);
const coordinates = (p: Point) => [p.x, p.y];

describe("G2D03 center-form conformance", () => {
  it("reads all seven neutral center records", () => {
    expect(centerCases).toHaveLength(7);
    expect(centerCases.filter((entry) => entry.operation === "center-arc-bounds")).toHaveLength(4);
    expect(centerCases.filter((entry) => entry.operation === "center-arc-cubics")).toHaveLength(3);
  });

  for (const entry of centerCases) {
    it(entry.id, () => {
      const arc = fromCase(entry);
      if (entry.operation === "center-arc-bounds") {
        const bounds = arc.boundingBox();
        [bounds.x, bounds.y, bounds.width, bounds.height].forEach((value, i) => {
          expect(Number.isFinite(value)).toBe(true);
          expect(Math.abs(value - entry.expected_bounds![i])).toBeLessThanOrEqual(fixtures.absolute_tolerance);
        });
      } else if (entry.expected_error) {
        expect(() => arc.toCubicBeziers()).toThrow("arc2d: sweep exceeds one turn");
      } else {
        const cubics = arc.toCubicBeziers();
        expect(cubics).toHaveLength(entry.expected_count!);
        expect(cubics[0].p0.distance(arc.evaluate(0))).toBeLessThan(1e-9);
        expect(cubics.at(-1)!.p3.distance(arc.evaluate(1))).toBeLessThan(1e-9);
        for (let i = 0; i < cubics.length; i++) {
          for (const p of [cubics[i].p0, cubics[i].p1, cubics[i].p2, cubics[i].p3]) {
            expect(coordinates(p).every(Number.isFinite)).toBe(true);
            if (entry.sweep_angle === 0) expect(p.distance(arc.evaluate(0))).toBeLessThan(1e-9);
          }
          if (i > 0) expect(cubics[i - 1].p3.distance(cubics[i].p0)).toBeLessThan(1e-9);
        }
      }
    });
  }

  // Construction remains cheap and compatible. Every public operation checks
  // the value at use time, including values crossing a JS boundary as NaN/Inf.
  const operations = (arc: CenterArc) => [
    () => arc.evaluate(0.5), () => arc.tangent(0.5),
    () => arc.boundingBox(), () => arc.toCubicBeziers(),
  ];
  it("rejects every non-finite field and non-positive radius at use time", () => {
    const valid = [0, 0, 2, 1, 0.3, PI, 0.2];
    for (let field = 0; field < valid.length; field++) {
      for (const value of [NaN, Infinity, -Infinity]) {
        const values = [...valid]; values[field] = value;
        const arc = new CenterArc(new Point(values[0], values[1]),
          values[2], values[3], values[4], values[5], values[6]);
        for (const op of operations(arc)) expect(op).toThrow("arc2d: invalid center arc");
      }
    }
    for (const [rx, ry] of [[0, 1], [-1, 1], [1, 0], [1, -1]]) {
      for (const op of operations(new CenterArc(Point.origin(), rx, ry, 0, PI, 0))) {
        expect(op).toThrow(RangeError);
      }
    }
  });

  it("rejects finite over-turn sweeps before any segment loop", () => {
    for (const sweep of [7, -7, Number.MAX_VALUE]) {
      for (const op of operations(new CenterArc(Point.origin(), 1, 1, 0, sweep, 0))) {
        expect(op).toThrow("arc2d: sweep exceeds one turn");
      }
    }
  });

  it("rejects non-finite parameters and derived output", () => {
    const arc = new CenterArc(Point.origin(), 1, 1, 0, PI, 0);
    for (const t of [NaN, Infinity, -Infinity, Number.MAX_VALUE]) {
      expect(() => arc.evaluate(t)).toThrow("arc2d: invalid center arc");
      expect(() => arc.tangent(t)).toThrow("arc2d: invalid center arc");
    }
    const huge = Number.MAX_VALUE;
    for (const op of operations(new CenterArc(new Point(huge, huge), huge, huge, 0, PI, 0))) {
      expect(op).toThrow("arc2d: invalid center arc");
    }
    // Every extreme can be finite while the rectangle's span overflows.
    const wide = new CenterArc(Point.origin(), 0.75 * huge, 1, 0, 2 * PI, 0);
    expect(() => wide.boundingBox()).toThrow("arc2d: invalid center arc");
  });

  it("includes endpoints, both signed full turns and axis-aligned extrema", () => {
    for (const sweep of [2 * PI, -2 * PI]) {
      const arc = new CenterArc(new Point(1, -2), 3, 2, 5 * PI, sweep, 0);
      const bounds = arc.boundingBox();
      [bounds.x, bounds.y, bounds.width, bounds.height].forEach((value, i) =>
        expect(value).toBeCloseTo([-2, -4, 6, 4][i], 9)
      );
      expect(arc.toCubicBeziers()).toHaveLength(4);
    }
    for (const sweep of [PI / 2, -PI / 2]) {
      const arc = new CenterArc(Point.origin(), 2, 1, 0, sweep, 0);
      const bounds = arc.boundingBox();
      expect(bounds.width).toBeCloseTo(2, 9);
      expect(bounds.height).toBeCloseTo(1, 9);
      expect(bounds.y).toBeCloseTo(sweep > 0 ? 0 : -1, 9);
      const tangent = arc.tangent(0);
      expect(tangent.x).toBeCloseTo(0, 9);
      expect(tangent.y).toBeCloseTo(sweep, 9);
    }
  });

  it("encloses rotated clockwise and counterclockwise arcs", () => {
    // Independent host trig is a containment check, not the expected-extrema
    // oracle: the neutral records above pin the tight numerical bounds.
    for (const start of [-7, 0.4, 8]) {
      for (const sweep of [-2 * PI, -4.1, -0.2, 0.2, 4.1, 2 * PI]) {
        const rotation = 0.6;
        const arc = new CenterArc(new Point(2, -3), 3, 1, start, sweep, rotation);
        const bounds = arc.boundingBox();
        for (let i = 0; i <= 200; i++) {
          const angle = start + sweep * i / 200;
          const x = 2 + 3 * Math.cos(angle) * Math.cos(rotation) - Math.sin(angle) * Math.sin(rotation);
          const y = -3 + 3 * Math.cos(angle) * Math.sin(rotation) + Math.sin(angle) * Math.cos(rotation);
          expect(x).toBeGreaterThanOrEqual(bounds.x - fixtures.absolute_tolerance);
          expect(x).toBeLessThanOrEqual(bounds.x + bounds.width + fixtures.absolute_tolerance);
          expect(y).toBeGreaterThanOrEqual(bounds.y - fixtures.absolute_tolerance);
          expect(y).toBeLessThanOrEqual(bounds.y + bounds.height + fixtures.absolute_tolerance);
        }
      }
    }
  });
});
