import { PI, sin, cos, tan, atan, atan2, sqrt, radians, degrees } from "../src/trig";

describe("Trigonometric Functions", () => {
  // --------------------------------------------------------------------------
  // Fundamental values
  // --------------------------------------------------------------------------

  describe("sin(x) at key angles", () => {
    it("sin(0) === 0", () => {
      expect(sin(0)).toBeCloseTo(0.0, 10);
    });

    it("sin(PI/2) === 1", () => {
      expect(sin(PI / 2)).toBeCloseTo(1.0, 10);
    });

    it("sin(PI) === 0", () => {
      expect(sin(PI)).toBeCloseTo(0.0, 10);
    });

    it("sin(3*PI/2) === -1", () => {
      expect(sin((3 * PI) / 2)).toBeCloseTo(-1.0, 10);
    });

    it("sin(2*PI) === 0", () => {
      expect(sin(2 * PI)).toBeCloseTo(0.0, 10);
    });
  });

  describe("cos(x) at key angles", () => {
    it("cos(0) === 1", () => {
      expect(cos(0)).toBeCloseTo(1.0, 10);
    });

    it("cos(PI/2) === 0", () => {
      expect(cos(PI / 2)).toBeCloseTo(0.0, 10);
    });

    it("cos(PI) === -1", () => {
      expect(cos(PI)).toBeCloseTo(-1.0, 10);
    });

    it("cos(3*PI/2) === 0", () => {
      expect(cos((3 * PI) / 2)).toBeCloseTo(0.0, 10);
    });

    it("cos(2*PI) === 1", () => {
      expect(cos(2 * PI)).toBeCloseTo(1.0, 10);
    });
  });

  // --------------------------------------------------------------------------
  // Symmetry properties
  // --------------------------------------------------------------------------

  describe("symmetry", () => {
    const testAngles = [0.1, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0];

    it("sin(-x) === -sin(x)  (odd function)", () => {
      for (const x of testAngles) {
        expect(sin(-x)).toBeCloseTo(-sin(x), 10);
      }
    });

    it("cos(-x) === cos(x)  (even function)", () => {
      for (const x of testAngles) {
        expect(cos(-x)).toBeCloseTo(cos(x), 10);
      }
    });
  });

  // --------------------------------------------------------------------------
  // Pythagorean identity: sin²(x) + cos²(x) === 1
  // --------------------------------------------------------------------------

  describe("Pythagorean identity", () => {
    const testAngles = [0.0, 0.3, 0.7, 1.0, PI / 4, PI / 3, PI / 2, PI, 2.5, 5.0, -1.0, -2.7];

    it("sin²(x) + cos²(x) === 1 for various x", () => {
      for (const x of testAngles) {
        const s = sin(x);
        const c = cos(x);
        expect(s * s + c * c).toBeCloseTo(1.0, 10);
      }
    });
  });

  // --------------------------------------------------------------------------
  // Large inputs (range reduction stress test)
  // --------------------------------------------------------------------------

  describe("large inputs", () => {
    it("sin(1000*PI) is close to 0", () => {
      expect(sin(1000 * PI)).toBeCloseTo(0.0, 10);
    });

    it("cos(1000*PI) is close to 1", () => {
      // cos(1000π) = cos(0) = 1 because 1000 is even
      expect(cos(1000 * PI)).toBeCloseTo(1.0, 10);
    });

    it("sin(999*PI) is close to 0", () => {
      expect(sin(999 * PI)).toBeCloseTo(0.0, 10);
    });

    it("cos(999*PI) is close to -1", () => {
      // cos(999π) = cos(π) = -1 because 999 is odd
      expect(cos(999 * PI)).toBeCloseTo(-1.0, 10);
    });

    it("sin(100.5*PI) is close to 1", () => {
      // sin(100.5π) = sin(0.5π) = 1 because 100 is even
      expect(sin(100.5 * PI)).toBeCloseTo(1.0, 10);
    });
  });

  // --------------------------------------------------------------------------
  // Negative inputs
  // --------------------------------------------------------------------------

  describe("negative inputs", () => {
    it("sin(-PI/2) === -1", () => {
      expect(sin(-PI / 2)).toBeCloseTo(-1.0, 10);
    });

    it("cos(-PI) === -1", () => {
      expect(cos(-PI)).toBeCloseTo(-1.0, 10);
    });
  });

  // --------------------------------------------------------------------------
  // Unit conversions
  // --------------------------------------------------------------------------

  describe("radians()", () => {
    it("radians(0) === 0", () => {
      expect(radians(0)).toBeCloseTo(0.0, 10);
    });

    it("radians(180) === PI", () => {
      expect(radians(180)).toBeCloseTo(PI, 10);
    });

    it("radians(90) === PI/2", () => {
      expect(radians(90)).toBeCloseTo(PI / 2, 10);
    });

    it("radians(360) === 2*PI", () => {
      expect(radians(360)).toBeCloseTo(2 * PI, 10);
    });

    it("radians(45) === PI/4", () => {
      expect(radians(45)).toBeCloseTo(PI / 4, 10);
    });
  });

  describe("degrees()", () => {
    it("degrees(0) === 0", () => {
      expect(degrees(0)).toBeCloseTo(0.0, 10);
    });

    it("degrees(PI) === 180", () => {
      expect(degrees(PI)).toBeCloseTo(180.0, 10);
    });

    it("degrees(PI/2) === 90", () => {
      expect(degrees(PI / 2)).toBeCloseTo(90.0, 10);
    });

    it("degrees(2*PI) === 360", () => {
      expect(degrees(2 * PI)).toBeCloseTo(360.0, 10);
    });
  });

  // --------------------------------------------------------------------------
  // Roundtrip: degrees -> radians -> sin/cos
  // --------------------------------------------------------------------------

  describe("roundtrip with degree conversion", () => {
    it("sin(radians(30)) === 0.5", () => {
      expect(sin(radians(30))).toBeCloseTo(0.5, 10);
    });

    it("cos(radians(60)) === 0.5", () => {
      expect(cos(radians(60))).toBeCloseTo(0.5, 10);
    });

    it("sin(radians(45)) === cos(radians(45))", () => {
      expect(sin(radians(45))).toBeCloseTo(cos(radians(45)), 10);
    });
  });

  // --------------------------------------------------------------------------
  // sqrt
  // --------------------------------------------------------------------------

  describe("sqrt(x)", () => {
    it("sqrt(0) === 0", () => {
      expect(sqrt(0)).toBeCloseTo(0.0, 10);
    });

    it("sqrt(1) === 1", () => {
      expect(sqrt(1)).toBeCloseTo(1.0, 10);
    });

    it("sqrt(4) === 2", () => {
      expect(sqrt(4)).toBeCloseTo(2.0, 10);
    });

    it("sqrt(9) === 3", () => {
      expect(sqrt(9)).toBeCloseTo(3.0, 10);
    });

    it("sqrt(2) ≈ 1.41421356237", () => {
      expect(sqrt(2)).toBeCloseTo(1.41421356237, 10);
    });

    it("sqrt(0.25) === 0.5", () => {
      expect(sqrt(0.25)).toBeCloseTo(0.5, 10);
    });

    it("sqrt(1e10) ≈ 1e5", () => {
      expect(sqrt(1e10)).toBeCloseTo(1e5, 5);
    });

    it("sqrt(x)^2 ≈ x (roundtrip)", () => {
      const s = sqrt(2);
      expect(s * s).toBeCloseTo(2.0, 10);
    });

    it("throws for negative input", () => {
      expect(() => sqrt(-1)).toThrow();
    });
  });

  // --------------------------------------------------------------------------
  // tan
  // --------------------------------------------------------------------------

  describe("tan(x)", () => {
    it("tan(0) === 0", () => {
      expect(tan(0)).toBeCloseTo(0.0, 10);
    });

    it("tan(PI/4) ≈ 1.0", () => {
      expect(tan(PI / 4)).toBeCloseTo(1.0, 10);
    });

    it("tan(PI/6) ≈ 0.57735...", () => {
      expect(tan(PI / 6)).toBeCloseTo(1.0 / sqrt(3), 10);
    });

    it("tan(-PI/4) ≈ -1.0", () => {
      expect(tan(-PI / 4)).toBeCloseTo(-1.0, 10);
    });
  });

  // --------------------------------------------------------------------------
  // atan
  // --------------------------------------------------------------------------

  describe("atan(x)", () => {
    it("atan(0) === 0", () => {
      expect(atan(0)).toBeCloseTo(0.0, 10);
    });

    it("atan(1) ≈ PI/4", () => {
      expect(atan(1)).toBeCloseTo(PI / 4, 10);
    });

    it("atan(-1) ≈ -PI/4", () => {
      expect(atan(-1)).toBeCloseTo(-PI / 4, 10);
    });

    it("atan(sqrt(3)) ≈ PI/3", () => {
      expect(atan(sqrt(3))).toBeCloseTo(PI / 3, 10);
    });

    it("atan(1/sqrt(3)) ≈ PI/6", () => {
      expect(atan(1 / sqrt(3))).toBeCloseTo(PI / 6, 10);
    });

    it("atan(large x) approaches PI/2", () => {
      expect(atan(1e10)).toBeCloseTo(PI / 2, 5);
    });

    it("atan(-large x) approaches -PI/2", () => {
      expect(atan(-1e10)).toBeCloseTo(-PI / 2, 5);
    });

    it("atan(tan(PI/4)) ≈ PI/4 (roundtrip)", () => {
      expect(atan(tan(PI / 4))).toBeCloseTo(PI / 4, 10);
    });
  });

  // --------------------------------------------------------------------------
  // atan2
  // --------------------------------------------------------------------------

  describe("atan2(y, x)", () => {
    it("atan2(0, 1) === 0  (positive x-axis)", () => {
      expect(atan2(0, 1)).toBeCloseTo(0.0, 10);
    });

    it("atan2(1, 0) === PI/2  (positive y-axis)", () => {
      expect(atan2(1, 0)).toBeCloseTo(PI / 2, 10);
    });

    it("atan2(0, -1) === PI  (negative x-axis)", () => {
      expect(atan2(0, -1)).toBeCloseTo(PI, 10);
    });

    it("atan2(-1, 0) === -PI/2  (negative y-axis)", () => {
      expect(atan2(-1, 0)).toBeCloseTo(-PI / 2, 10);
    });

    it("atan2(1, 1) ≈ PI/4  (Q1)", () => {
      expect(atan2(1, 1)).toBeCloseTo(PI / 4, 10);
    });

    it("atan2(1, -1) ≈ 3*PI/4  (Q2)", () => {
      expect(atan2(1, -1)).toBeCloseTo((3 * PI) / 4, 10);
    });

    it("atan2(-1, -1) ≈ -3*PI/4  (Q3)", () => {
      expect(atan2(-1, -1)).toBeCloseTo((-3 * PI) / 4, 10);
    });

    it("atan2(-1, 1) ≈ -PI/4  (Q4)", () => {
      expect(atan2(-1, 1)).toBeCloseTo(-PI / 4, 10);
    });
  });
});
