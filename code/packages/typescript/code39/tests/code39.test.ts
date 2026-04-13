import { describe, expect, it } from "vitest";
import {
  VERSION,
  BarcodeError,
  InvalidCharacterError,
  InvalidConfigurationError,
  DEFAULT_RENDER_CONFIG,
  normalizeCode39,
  encodeCode39Char,
  encodeCode39,
  expandCode39Runs,
  layoutCode39,
  drawCode39,
} from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("errors", () => {
  it("BarcodeError extends Error", () => {
    expect(new BarcodeError("bad")).toBeInstanceOf(Error);
  });

  it("InvalidCharacterError extends BarcodeError", () => {
    expect(new InvalidCharacterError("bad")).toBeInstanceOf(BarcodeError);
  });

  it("InvalidConfigurationError extends BarcodeError", () => {
    expect(new InvalidConfigurationError("bad")).toBeInstanceOf(BarcodeError);
  });
});

describe("normalizeCode39()", () => {
  it("uppercases lowercase input", () => {
    expect(normalizeCode39("abc-123")).toBe("ABC-123");
  });

  it("preserves spaces", () => {
    expect(normalizeCode39("ab c")).toBe("AB C");
  });

  it("rejects unsupported characters", () => {
    expect(() => normalizeCode39("ABC@123")).toThrow(InvalidCharacterError);
  });
});

describe("encodeCode39Char()", () => {
  it("encodes A using the expected width pattern", () => {
    expect(encodeCode39Char("A")).toEqual({
      char: "A",
      isStartStop: false,
      pattern: "WNNNNWNNW",
    });
  });
});

describe("encodeCode39()", () => {
  it("wraps user data in start/stop markers", () => {
    expect(encodeCode39("A").map((entry) => entry.char)).toEqual(["*", "A", "*"]);
  });
});

describe("expandCode39Runs()", () => {
  it("expands one data character into three encoded symbols with gaps", () => {
    const runs = expandCode39Runs("A");
    expect(runs).toHaveLength(29);
    expect(runs[0]).toMatchObject({ color: "bar", sourceLabel: "*", sourceIndex: 0, role: "start" });
    expect(runs[9]).toMatchObject({ color: "space", role: "inter-character-gap" });
  });
});

describe("layoutCode39()", () => {
  it("builds a reusable paint scene", () => {
    const scene = layoutCode39("AB");
    expect(scene.width).toBeGreaterThan(0);
    expect(scene.instructions.length).toBeGreaterThan(0);
    expect(scene.metadata?.symbology).toBe("code39");
  });

  it("rejects invalid configuration", () => {
    expect(() =>
      layoutCode39("A", {
        renderConfig: { moduleWidth: 0 },
      }),
    ).toThrow(InvalidConfigurationError);
  });

  it("inherits the shared 1D defaults", () => {
    expect(DEFAULT_RENDER_CONFIG).toMatchObject({
      moduleWidth: 4,
      barHeight: 120,
      includeHumanReadableText: false,
    });
  });
});

describe("drawCode39()", () => {
  it("returns a scene with a label", () => {
    const scene = drawCode39("A");
    expect(scene.metadata?.label).toBe("Code 39 barcode for A");
  });
});
