import { describe, expect, it } from "vitest";
import {
  VERSION,
  Barcode1DError,
  InvalidBarcode1DConfigurationError,
  DEFAULT_BARCODE_1D_RENDER_CONFIG,
  computeBarcode1DLayout,
  layoutBarcode1D,
  runsFromBinaryPattern,
  runsFromWidthPattern,
  totalModules,
  type Barcode1DRun,
} from "../src/index.js";

describe("VERSION", () => {
  it("stays at 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("runsFromBinaryPattern()", () => {
  it("coalesces adjacent modules into alternating runs", () => {
    expect(
      runsFromBinaryPattern("110100", {
        sourceLabel: "start",
        sourceIndex: -1,
        role: "guard",
      }),
    ).toEqual<Barcode1DRun[]>([
      { color: "bar", modules: 2, sourceLabel: "start", sourceIndex: -1, role: "guard" },
      { color: "space", modules: 1, sourceLabel: "start", sourceIndex: -1, role: "guard" },
      { color: "bar", modules: 1, sourceLabel: "start", sourceIndex: -1, role: "guard" },
      { color: "space", modules: 2, sourceLabel: "start", sourceIndex: -1, role: "guard" },
    ]);
  });

  it("rejects non-binary input", () => {
    expect(() =>
      runsFromBinaryPattern("10A1", {
        sourceLabel: "bad",
        sourceIndex: 0,
        role: "data",
      }),
    ).toThrow(Barcode1DError);
  });
});

describe("runsFromWidthPattern()", () => {
  it("converts symbolic narrow and wide markers into numeric runs", () => {
    expect(
      runsFromWidthPattern("NWNNW", {
        sourceLabel: "0",
        sourceIndex: 0,
        role: "data",
      }),
    ).toEqual<Barcode1DRun[]>([
      { color: "bar", modules: 1, sourceLabel: "0", sourceIndex: 0, role: "data" },
      { color: "space", modules: 3, sourceLabel: "0", sourceIndex: 0, role: "data" },
      { color: "bar", modules: 1, sourceLabel: "0", sourceIndex: 0, role: "data" },
      { color: "space", modules: 1, sourceLabel: "0", sourceIndex: 0, role: "data" },
      { color: "bar", modules: 3, sourceLabel: "0", sourceIndex: 0, role: "data" },
    ]);
  });
});

describe("computeBarcode1DLayout()", () => {
  it("computes total modules and symbol spans from explicit descriptors", () => {
    const runs: Barcode1DRun[] = [
      { color: "bar", modules: 1, sourceLabel: "start", sourceIndex: -1, role: "guard" },
      { color: "space", modules: 1, sourceLabel: "start", sourceIndex: -1, role: "guard" },
      { color: "bar", modules: 1, sourceLabel: "start", sourceIndex: -1, role: "guard" },
      { color: "space", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data" },
      { color: "bar", modules: 2, sourceLabel: "A", sourceIndex: 0, role: "data" },
    ];

    const layout = computeBarcode1DLayout(runs, {
      quietZoneModules: 10,
      symbols: [
        { label: "start", modules: 3, sourceIndex: -1, role: "guard" },
        { label: "A", modules: 3, sourceIndex: 0, role: "data" },
      ],
    });

    expect(layout.contentModules).toBe(6);
    expect(layout.totalModules).toBe(26);
    expect(layout.symbolLayouts).toEqual([
      { label: "start", startModule: 0, endModule: 3, sourceIndex: -1, role: "guard" },
      { label: "A", startModule: 3, endModule: 6, sourceIndex: 0, role: "data" },
    ]);
  });

  it("rejects consecutive runs of the same color", () => {
    expect(() =>
      computeBarcode1DLayout([
        { color: "bar", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data" },
        { color: "bar", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data" },
      ]),
    ).toThrow(Barcode1DError);
  });
});

describe("layoutBarcode1D()", () => {
  it("creates a paint scene with bar rects and scene metadata", () => {
    const runs = runsFromBinaryPattern("101", {
      sourceLabel: "start",
      sourceIndex: -1,
      role: "guard",
    });

    const scene = layoutBarcode1D(runs, {
      label: "Test barcode",
      metadata: { symbology: "test" },
    });

    expect(scene.width).toBe((DEFAULT_BARCODE_1D_RENDER_CONFIG.quietZoneModules * 2 + 3) * DEFAULT_BARCODE_1D_RENDER_CONFIG.moduleWidth);
    expect(scene.instructions).toHaveLength(2);
    expect(scene.metadata?.label).toBe("Test barcode");
    expect(scene.metadata?.symbology).toBe("test");
  });

  it("rejects invalid render config", () => {
    const runs = runsFromBinaryPattern("101", {
      sourceLabel: "start",
      sourceIndex: -1,
      role: "guard",
    });

    expect(() =>
      layoutBarcode1D(runs, {
        renderConfig: { moduleWidth: 0 },
      }),
    ).toThrow(InvalidBarcode1DConfigurationError);
  });

  it("rejects human-readable text until text metrics exist", () => {
    const runs = runsFromBinaryPattern("101", {
      sourceLabel: "start",
      sourceIndex: -1,
      role: "guard",
    });

    expect(() =>
      layoutBarcode1D(runs, {
        humanReadableText: "012345",
      }),
    ).toThrow(InvalidBarcode1DConfigurationError);
  });
});

describe("totalModules()", () => {
  it("adds module widths", () => {
    expect(
      totalModules([
        { color: "bar", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data" },
        { color: "space", modules: 2, sourceLabel: "A", sourceIndex: 0, role: "data" },
        { color: "bar", modules: 3, sourceLabel: "A", sourceIndex: 0, role: "data" },
      ]),
    ).toBe(6);
  });
});
