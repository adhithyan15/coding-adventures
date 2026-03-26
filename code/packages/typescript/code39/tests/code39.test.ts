import { describe, expect, it } from "vitest";
import { renderSvg } from "@coding-adventures/draw-instructions-svg";
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
  drawOneDimensionalBarcode,
  drawCode39,
  renderCode39,
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
    expect(runs[0]).toMatchObject({ color: "bar", sourceChar: "*", sourceIndex: 0 });
    expect(runs[9]).toMatchObject({ color: "space", isInterCharacterGap: true });
  });
});

describe("drawOneDimensionalBarcode()", () => {
  it("builds a reusable draw scene", () => {
    const scene = drawOneDimensionalBarcode(expandCode39Runs("AB"), "AB");
    expect(scene.width).toBeGreaterThan(0);
    expect(scene.instructions.length).toBeGreaterThan(0);
    expect(scene.metadata?.symbology).toBe("code39");
  });

  it("rejects invalid configuration", () => {
    expect(() =>
      drawOneDimensionalBarcode(expandCode39Runs("A"), "A", {
        ...DEFAULT_RENDER_CONFIG,
        wideUnit: 4,
      }),
    ).toThrow(InvalidConfigurationError);
  });
});

describe("drawCode39()", () => {
  it("returns a scene with a label", () => {
    const scene = drawCode39("A");
    expect(scene.metadata?.label).toBe("Code 39 barcode for A");
  });
});

describe("renderCode39()", () => {
  it("renders through any draw renderer", () => {
    const output = renderCode39("OK", {
      render(scene) {
        return `${scene.width}:${scene.instructions.length}`;
      },
    });
    expect(output).toMatch(/^\d+:\d+$/);
  });

  it("works with the svg renderer package", () => {
    const svg = renderCode39("A", { render: renderSvg });
    expect(svg).toContain("<svg");
    expect(svg).toContain("Code 39 barcode for A");
  });
});
