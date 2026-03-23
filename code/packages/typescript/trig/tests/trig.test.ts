import { PI, sin, cos, radians, degrees } from "../src/trig";

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
});
