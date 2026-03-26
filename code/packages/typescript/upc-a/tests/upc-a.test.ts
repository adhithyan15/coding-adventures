import { describe, expect, it } from "vitest";
import {
  VERSION,
  InvalidUpcACheckDigitError,
  InvalidUpcAInputError,
  computeUpcACheckDigit,
  normalizeUpcA,
  encodeUpcA,
  expandUpcARuns,
  drawUpcA,
} from "../src/index.js";

describe("VERSION", () => {
  it("stays at 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("computeUpcACheckDigit()", () => {
  it("computes the expected check digit", () => {
    expect(computeUpcACheckDigit("03600029145")).toBe("2");
  });
});

describe("normalizeUpcA()", () => {
  it("computes the check digit for 11-digit input", () => {
    expect(normalizeUpcA("03600029145")).toBe("036000291452");
  });

  it("rejects non-digit input", () => {
    expect(() => normalizeUpcA("03600A29145")).toThrow(InvalidUpcAInputError);
  });

  it("rejects a bad supplied check digit", () => {
    expect(() => normalizeUpcA("036000291453")).toThrow(InvalidUpcACheckDigitError);
  });
});

describe("encodeUpcA()", () => {
  it("encodes the final digit as the check digit on the right side", () => {
    const encoded = encodeUpcA("03600029145");
    expect(encoded).toHaveLength(12);
    expect(encoded[0]).toMatchObject({ digit: "0", encoding: "L", role: "data" });
    expect(encoded[11]).toMatchObject({ digit: "2", encoding: "R", role: "check" });
  });
});

describe("expandUpcARuns()", () => {
  it("creates a 95-module barcode stream", () => {
    const runs = expandUpcARuns("03600029145");
    const totalModules = runs.reduce((sum, run) => sum + run.modules, 0);
    expect(totalModules).toBe(95);
    expect(runs[0]).toMatchObject({ color: "bar", role: "guard", sourceLabel: "start" });
  });
});

describe("drawUpcA()", () => {
  it("returns a barcode scene", () => {
    const scene = drawUpcA("03600029145");
    expect(scene.metadata?.symbology).toBe("upc-a");
    expect(scene.metadata?.contentModules).toBe(95);
  });
});
