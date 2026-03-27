import { describe, expect, it } from "vitest";
import {
  VERSION,
  InvalidCodabarInputError,
  normalizeCodabar,
  encodeCodabar,
  expandCodabarRuns,
  drawCodabar,
} from "../src/index.js";

describe("VERSION", () => {
  it("stays at 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("normalizeCodabar()", () => {
  it("wraps a body string with A guards by default", () => {
    expect(normalizeCodabar("40156")).toBe("A40156A");
  });

  it("preserves explicit guards", () => {
    expect(normalizeCodabar("B40156D")).toBe("B40156D");
  });

  it("rejects invalid body characters", () => {
    expect(() => normalizeCodabar("40*56")).toThrow(InvalidCodabarInputError);
  });
});

describe("encodeCodabar()", () => {
  it("marks the outer symbols as start and stop", () => {
    const encoded = encodeCodabar("40156");
    expect(encoded[0]).toMatchObject({ char: "A", role: "start" });
    expect(encoded[encoded.length - 1]).toMatchObject({ char: "A", role: "stop" });
  });
});

describe("expandCodabarRuns()", () => {
  it("adds inter-character gaps between symbols", () => {
    const runs = expandCodabarRuns("40156");
    expect(runs.some((run) => run.role === "inter-character-gap")).toBe(true);
  });
});

describe("drawCodabar()", () => {
  it("returns a barcode scene", () => {
    const scene = drawCodabar("40156");
    expect(scene.metadata?.symbology).toBe("codabar");
  });
});
