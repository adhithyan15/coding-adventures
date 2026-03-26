import { describe, expect, it } from "vitest";
import {
  VERSION,
  InvalidEan13CheckDigitError,
  InvalidEan13InputError,
  computeEan13CheckDigit,
  normalizeEan13,
  leftParityPattern,
  encodeEan13,
  expandEan13Runs,
  drawEan13,
} from "../src/index.js";

describe("VERSION", () => {
  it("stays at 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("computeEan13CheckDigit()", () => {
  it("computes the expected check digit", () => {
    expect(computeEan13CheckDigit("400638133393")).toBe("1");
  });
});

describe("normalizeEan13()", () => {
  it("computes the check digit for 12-digit input", () => {
    expect(normalizeEan13("400638133393")).toBe("4006381333931");
  });

  it("rejects non-digit input", () => {
    expect(() => normalizeEan13("40063813339A")).toThrow(InvalidEan13InputError);
  });

  it("rejects a bad supplied check digit", () => {
    expect(() => normalizeEan13("4006381333932")).toThrow(InvalidEan13CheckDigitError);
  });
});

describe("leftParityPattern()", () => {
  it("encodes the first digit indirectly through parity", () => {
    expect(leftParityPattern("400638133393")).toBe("LGLLGG");
  });
});

describe("encodeEan13()", () => {
  it("encodes the left side with L/G parity and the right side with R", () => {
    const encoded = encodeEan13("400638133393");
    expect(encoded[0]).toMatchObject({ digit: "0", encoding: "L" });
    expect(encoded[1]).toMatchObject({ digit: "0", encoding: "G" });
    expect(encoded[11]).toMatchObject({ digit: "1", encoding: "R", role: "check" });
  });
});

describe("expandEan13Runs()", () => {
  it("creates a 95-module barcode stream", () => {
    const runs = expandEan13Runs("400638133393");
    expect(runs.reduce((sum, run) => sum + run.modules, 0)).toBe(95);
  });
});

describe("drawEan13()", () => {
  it("returns scene metadata for the chosen parity pattern", () => {
    const scene = drawEan13("400638133393");
    expect(scene.metadata?.symbology).toBe("ean-13");
    expect(scene.metadata?.leftParity).toBe("LGLLGG");
  });
});
