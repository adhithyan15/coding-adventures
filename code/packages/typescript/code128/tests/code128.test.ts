import { describe, expect, it } from "vitest";
import {
  VERSION,
  InvalidCode128InputError,
  normalizeCode128B,
  computeCode128Checksum,
  encodeCode128B,
  expandCode128Runs,
  drawCode128,
} from "../src/index.js";

describe("VERSION", () => {
  it("stays at 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("normalizeCode128B()", () => {
  it("accepts printable ascii", () => {
    expect(normalizeCode128B("Code 128")).toBe("Code 128");
  });

  it("rejects control characters", () => {
    expect(() => normalizeCode128B("bad\ninput")).toThrow(InvalidCode128InputError);
  });
});

describe("computeCode128Checksum()", () => {
  it("matches the classic Code 128 example", () => {
    expect(computeCode128Checksum([35, 79, 68, 69, 0, 17, 18, 24])).toBe(64);
  });
});

describe("encodeCode128B()", () => {
  it("adds Start B, checksum, and stop", () => {
    const encoded = encodeCode128B("Code 128");
    expect(encoded[0]).toMatchObject({ label: "Start B", role: "start", value: 104 });
    expect(encoded[encoded.length - 2]).toMatchObject({ label: "Checksum 64", role: "check" });
    expect(encoded[encoded.length - 1]).toMatchObject({ label: "Stop", role: "stop", value: 106 });
  });
});

describe("expandCode128Runs()", () => {
  it("creates a run stream that ends with the stop pattern", () => {
    const runs = expandCode128Runs("Hi");
    expect(runs[runs.length - 1]).toMatchObject({ sourceLabel: "Stop", role: "stop" });
  });
});

describe("drawCode128()", () => {
  it("returns a barcode scene", () => {
    const scene = drawCode128("Code 128");
    expect(scene.metadata?.symbology).toBe("code128");
    expect(scene.metadata?.codeSet).toBe("B");
    expect(scene.metadata?.checksum).toBe(64);
  });
});
