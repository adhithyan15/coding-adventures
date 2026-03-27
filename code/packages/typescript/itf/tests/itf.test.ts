import { describe, expect, it } from "vitest";
import {
  VERSION,
  InvalidItfInputError,
  normalizeItf,
  encodeItf,
  expandItfRuns,
  drawItf,
} from "../src/index.js";

describe("VERSION", () => {
  it("stays at 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("normalizeItf()", () => {
  it("accepts even-length digit strings", () => {
    expect(normalizeItf("123456")).toBe("123456");
  });

  it("rejects odd-length input", () => {
    expect(() => normalizeItf("12345")).toThrow(InvalidItfInputError);
  });
});

describe("encodeItf()", () => {
  it("encodes digit pairs", () => {
    const encoded = encodeItf("123456");
    expect(encoded).toHaveLength(3);
    expect(encoded[0]).toMatchObject({ pair: "12" });
  });
});

describe("expandItfRuns()", () => {
  it("includes start and stop patterns", () => {
    const runs = expandItfRuns("123456");
    expect(runs[0]).toMatchObject({ sourceLabel: "start", role: "start" });
    expect(runs[runs.length - 1]).toMatchObject({ sourceLabel: "stop", role: "stop" });
  });
});

describe("drawItf()", () => {
  it("returns a barcode scene", () => {
    const scene = drawItf("123456");
    expect(scene.metadata?.symbology).toBe("itf");
    expect(scene.metadata?.pairCount).toBe(3);
  });
});
