/**
 * Tests for the Canvas backend.
 *
 * These tests run in a jsdom environment (configured in vitest.config.ts).
 * jsdom does NOT implement the Canvas 2D context, so we build a manual mock
 * object with vi.fn() for every method and plain writable properties for every
 * state field (fillStyle, lineWidth, filter, etc.).
 *
 * Because we are testing an imperative API we verify that the correct Canvas
 * methods are called with the correct arguments — not that pixels match.
 * This is the standard approach for testing Canvas code: verify the calls,
 * not the pixels.
 *
 * For pixel-accurate tests, use the export() function with a real OffscreenCanvas
 * (not available in jsdom) or use node-canvas in a separate integration test.
 */
import { describe, expect, it, vi, beforeEach } from "vitest";
import {
  VERSION,
  createCanvasVM,
  resolveFill,
} from "../src/index.js";
import {
  paintScene,
  paintRect,
  paintEllipse,
  paintPath,
  paintLine,
  paintGroup,
  paintLayer,
  paintClip,
  paintGradient,
  paintImage,
  paintText,
  type PixelContainer,
} from "@coding-adventures/paint-instructions";
import { ExportNotSupportedError } from "@coding-adventures/paint-vm";
import { UnsupportedFontBindingError } from "../src/index.js";

// ============================================================================
// Global Path2D mock
//
// jsdom does not implement the Path2D API. We inject a minimal mock so that
// the source code's `new Path2D()` calls succeed and the mock's methods
// (moveTo, lineTo, etc.) can be called without throwing.
// ============================================================================

class MockPath2D {
  moveTo = vi.fn();
  lineTo = vi.fn();
  quadraticCurveTo = vi.fn();
  bezierCurveTo = vi.fn();
  closePath = vi.fn();
  // Stub for arc_to fallback path (lineTo alias)
  // Path2D has no arc — handled via lineTo in applyCommandsToPath2D
}

if (typeof globalThis.Path2D === "undefined") {
  (globalThis as Record<string, unknown>).Path2D = MockPath2D;
}

// ============================================================================
// Global ImageData mock
//
// jsdom in some configurations does not expose ImageData globally. We inject a
// minimal mock so that the source code's `new ImageData(data, w, h)` calls
// succeed. The mock just stores the arguments so putImageData can be called.
// ============================================================================

class MockImageData {
  data: Uint8ClampedArray;
  width: number;
  height: number;
  constructor(data: Uint8ClampedArray, width: number, height: number) {
    this.data = data;
    this.width = width;
    this.height = height;
  }
}

if (typeof globalThis.ImageData === "undefined") {
  (globalThis as Record<string, unknown>).ImageData = MockImageData;
}

// ============================================================================
// Mock Canvas context factory
// ============================================================================

/**
 * Build a minimal CanvasRenderingContext2D-shaped mock object.
 *
 * Every method is a vi.fn(). Every state property (fillStyle, filter, etc.)
 * is a plain writable field. The source code mutates these properties directly
 * (ctx.fillStyle = "#3b82f6") so the tests can simply check their values after
 * execute() returns.
 *
 * createLinearGradient / createRadialGradient return a mock gradient object with
 * an addColorStop spy. This is sufficient for the gradient registry tests.
 */
function makeCtx(): CanvasRenderingContext2D {
  const mockGradient = { addColorStop: vi.fn() };

  const ctx = {
    // --- state properties (writable by source code) ---
    fillStyle: "" as string | CanvasGradient | CanvasPattern,
    strokeStyle: "" as string | CanvasGradient | CanvasPattern,
    lineWidth: 1,
    lineCap: "butt" as CanvasLineCap,
    lineJoin: "miter" as CanvasLineJoin,
    globalAlpha: 1,
    globalCompositeOperation: "source-over" as GlobalCompositeOperation,
    filter: "none",
    font: "",
    textBaseline: "alphabetic" as CanvasTextBaseline,
    textAlign: "start" as CanvasTextAlign,

    // --- methods (tracked by spies) ---
    clearRect: vi.fn(),
    fillRect: vi.fn(),
    strokeRect: vi.fn(),
    beginPath: vi.fn(),
    moveTo: vi.fn(),
    lineTo: vi.fn(),
    ellipse: vi.fn(),
    fill: vi.fn(),
    stroke: vi.fn(),
    save: vi.fn(),
    restore: vi.fn(),
    transform: vi.fn(),
    clip: vi.fn(),
    fillText: vi.fn(),
    roundRect: vi.fn(),
    putImageData: vi.fn(),
    rect: vi.fn(),
    arcTo: vi.fn(),
    closePath: vi.fn(),
    scale: vi.fn(),
    getImageData: vi.fn(() => ({
      data: new Uint8ClampedArray(100 * 100 * 4).buffer,
    })),
    createLinearGradient: vi.fn(() => mockGradient),
    createRadialGradient: vi.fn(() => mockGradient),
  } as unknown as CanvasRenderingContext2D;

  return ctx;
}

// Reset all vi.fn() spies between tests to keep assertions isolated
beforeEach(() => {
  vi.clearAllMocks();
});

// ============================================================================
// VERSION
// ============================================================================

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ============================================================================
// createCanvasVM — factory
// ============================================================================

describe("createCanvasVM()", () => {
  it("returns a VM with all 11 instruction kinds registered", () => {
    const vm = createCanvasVM();
    const kinds = vm.registeredKinds().sort();
    expect(kinds).toEqual([
      "clip", "ellipse", "glyph_run", "gradient", "group",
      "image", "layer", "line", "path", "rect", "text",
    ]);
  });
});

// ============================================================================
// execute() — clear and dispatch
// ============================================================================

describe("execute() — clear", () => {
  it("calls clearRect with scene dimensions", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(paintScene(400, 300, "#ffffff", []), ctx);
    expect(ctx.clearRect).toHaveBeenCalledWith(0, 0, 400, 300);
  });

  it("fills background when not transparent", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(paintScene(400, 300, "#f8fafc", []), ctx);
    expect(ctx.fillStyle).toBe("#f8fafc");
    expect(ctx.fillRect).toHaveBeenCalledWith(0, 0, 400, 300);
  });

  it("does not fillRect for transparent background", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(paintScene(400, 300, "transparent", []), ctx);
    expect(ctx.fillRect).not.toHaveBeenCalled();
  });

  it("does not fillRect for none background", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(paintScene(400, 300, "none", []), ctx);
    expect(ctx.fillRect).not.toHaveBeenCalled();
  });
});

// ============================================================================
// PaintRect handler
// ============================================================================

describe("PaintRect handler", () => {
  it("calls fillRect for a rect with fill", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintRect(10, 20, 100, 50, { fill: "#3b82f6" }),
      ]),
      ctx,
    );
    expect(ctx.fillRect).toHaveBeenCalledWith(10, 20, 100, 50);
    expect(ctx.fillStyle).toBe("#3b82f6");
  });

  it("calls strokeRect for a rect with stroke", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { stroke: "#000", stroke_width: 2 }),
      ]),
      ctx,
    );
    expect(ctx.strokeRect).toHaveBeenCalledWith(0, 0, 100, 50);
    expect(ctx.strokeStyle).toBe("#000");
    expect(ctx.lineWidth).toBe(2);
  });

  it("uses beginPath for a rounded rect (roundRect available)", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { fill: "#3b82f6", corner_radius: 8 }),
      ]),
      ctx,
    );
    // roundRect() is defined on the mock so the roundRect path is taken
    expect(ctx.beginPath).toHaveBeenCalled();
    expect(ctx.roundRect).toHaveBeenCalledWith(0, 0, 100, 50, 8);
    expect(ctx.fill).toHaveBeenCalled();
  });

  it("falls back to arcTo polyfill when roundRect is missing", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    // Remove roundRect to trigger the arcTo polyfill path
    (ctx as Record<string, unknown>).roundRect = undefined;
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { fill: "#3b82f6", corner_radius: 8 }),
      ]),
      ctx,
    );
    expect(ctx.beginPath).toHaveBeenCalled();
    expect(ctx.arcTo).toHaveBeenCalled();
  });

  it("rect with neither fill nor stroke renders nothing visible (no crash)", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [paintRect(0, 0, 50, 50)]),
        ctx,
      ),
    ).not.toThrow();
  });

  it("rounded rect with stroke calls stroke()", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { stroke: "#000", stroke_width: 2, corner_radius: 4 }),
      ]),
      ctx,
    );
    expect(ctx.stroke).toHaveBeenCalled();
    expect(ctx.lineWidth).toBe(2);
  });
});

// ============================================================================
// PaintEllipse handler
// ============================================================================

describe("PaintEllipse handler", () => {
  it("calls ctx.ellipse and fill for an ellipse with fill", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintEllipse(100, 100, 50, 30, { fill: "#ef4444" }),
      ]),
      ctx,
    );
    expect(ctx.ellipse).toHaveBeenCalledWith(100, 100, 50, 30, 0, 0, Math.PI * 2);
    expect(ctx.fill).toHaveBeenCalled();
    expect(ctx.fillStyle).toBe("#ef4444");
  });

  it("calls ctx.stroke for an ellipse with stroke", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintEllipse(100, 100, 50, 30, { stroke: "#333", stroke_width: 1.5 }),
      ]),
      ctx,
    );
    expect(ctx.stroke).toHaveBeenCalled();
    expect(ctx.strokeStyle).toBe("#333");
    expect(ctx.lineWidth).toBe(1.5);
  });
});

// ============================================================================
// PaintPath handler
// ============================================================================

describe("PaintPath handler", () => {
  it("calls fill for a path with fill", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintPath([
          { kind: "move_to", x: 0, y: 0 },
          { kind: "line_to", x: 100, y: 0 },
          { kind: "close" },
        ], { fill: "#ef4444" }),
      ]),
      ctx,
    );
    // Path2D is used internally — verify fill was called
    expect(ctx.fill).toHaveBeenCalled();
    expect(ctx.fillStyle).toBe("#ef4444");
  });

  it("sets lineCap and lineJoin", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintPath(
          [{ kind: "move_to", x: 0, y: 0 }],
          { stroke: "#000", stroke_cap: "round", stroke_join: "bevel" },
        ),
      ]),
      ctx,
    );
    expect(ctx.lineCap).toBe("round");
    expect(ctx.lineJoin).toBe("bevel");
  });

  it("handles quad_to and cubic_to commands without crashing", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [
          paintPath([
            { kind: "move_to", x: 0, y: 0 },
            { kind: "quad_to", cx: 50, cy: 0, x: 100, y: 50 },
            { kind: "cubic_to", cx1: 0, cy1: 25, cx2: 50, cy2: 75, x: 100, y: 100 },
          ], { stroke: "#000" }),
        ]),
        ctx,
      ),
    ).not.toThrow();
  });

  it("handles arc_to command (falls back to lineTo)", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [
          paintPath([
            { kind: "move_to", x: 0, y: 0 },
            { kind: "arc_to", rx: 50, ry: 50, x_rotation: 0, large_arc: false, sweep: true, x: 100, y: 100 },
          ], { stroke: "#000" }),
        ]),
        ctx,
      ),
    ).not.toThrow();
  });
});

// ============================================================================
// PaintLine handler
// ============================================================================

describe("PaintLine handler", () => {
  it("calls moveTo, lineTo, and stroke", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLine(0, 50, 200, 50, "#9ca3af", { stroke_width: 2 }),
      ]),
      ctx,
    );
    expect(ctx.moveTo).toHaveBeenCalledWith(0, 50);
    expect(ctx.lineTo).toHaveBeenCalledWith(200, 50);
    expect(ctx.stroke).toHaveBeenCalled();
    expect(ctx.strokeStyle).toBe("#9ca3af");
    expect(ctx.lineWidth).toBe(2);
  });

  it("sets lineCap for a line with stroke_cap", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLine(0, 0, 100, 0, "#000", { stroke_cap: "round" }),
      ]),
      ctx,
    );
    expect(ctx.lineCap).toBe("round");
  });

  it("uses default lineWidth of 1 when stroke_width is not set", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLine(0, 0, 100, 0, "#000"),
      ]),
      ctx,
    );
    expect(ctx.lineWidth).toBe(1);
  });
});

// ============================================================================
// PaintGroup handler
// ============================================================================

describe("PaintGroup handler", () => {
  it("calls save() and restore() around children", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintGroup([paintRect(0, 0, 50, 50, { fill: "#3b82f6" })]),
      ]),
      ctx,
    );
    expect(ctx.save).toHaveBeenCalled();
    expect(ctx.restore).toHaveBeenCalled();
  });

  it("applies transform via ctx.transform()", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintGroup([], { transform: [1, 0, 0, 1, 50, 100] }),
      ]),
      ctx,
    );
    expect(ctx.transform).toHaveBeenCalledWith(1, 0, 0, 1, 50, 100);
  });

  it("sets globalAlpha for opacity", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintGroup([paintRect(0, 0, 50, 50, { fill: "#fff" })], { opacity: 0.5 }),
      ]),
      ctx,
    );
    expect(ctx.globalAlpha).toBe(0.5);
  });

  it("dispatches children inside the group", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintGroup([
          paintRect(0, 0, 50, 50, { fill: "#f00" }),
          paintEllipse(100, 100, 20, 20, { fill: "#00f" }),
        ]),
      ]),
      ctx,
    );
    expect(ctx.fillRect).toHaveBeenCalled();
    expect(ctx.ellipse).toHaveBeenCalled();
  });
});

// ============================================================================
// PaintLayer handler
// ============================================================================

describe("PaintLayer handler", () => {
  it("calls save() and restore() around layer render", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([paintRect(0, 0, 50, 50, { fill: "#3b82f6" })]),
      ]),
      ctx,
    );
    expect(ctx.save).toHaveBeenCalled();
    expect(ctx.restore).toHaveBeenCalled();
  });

  it("sets ctx.filter for blur", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { filters: [{ kind: "blur", radius: 10 }] }),
      ]),
      ctx,
    );
    expect(ctx.filter).toBe("blur(10px)");
  });

  it("sets ctx.filter for multiple filters chained", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          filters: [
            { kind: "blur", radius: 5 },
            { kind: "brightness", amount: 1.2 },
          ],
        }),
      ]),
      ctx,
    );
    expect(ctx.filter).toBe("blur(5px) brightness(1.2)");
  });

  it("sets globalCompositeOperation for non-normal blend modes", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { blend_mode: "multiply" }),
      ]),
      ctx,
    );
    expect(ctx.globalCompositeOperation).toBe("multiply");
  });

  it("maps color_dodge blend mode to 'color-dodge'", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { blend_mode: "color_dodge" }),
      ]),
      ctx,
    );
    expect(ctx.globalCompositeOperation).toBe("color-dodge");
  });

  it("does NOT set globalCompositeOperation for normal blend mode", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { blend_mode: "normal" }),
      ]),
      ctx,
    );
    // "normal" is the default; we don't write it to avoid an unnecessary state change
    expect(ctx.globalCompositeOperation).toBe("source-over"); // mock default
  });

  it("applies transform inside a layer", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { transform: [1, 0, 0, 1, 10, 20] }),
      ]),
      ctx,
    );
    expect(ctx.transform).toHaveBeenCalledWith(1, 0, 0, 1, 10, 20);
  });

  it("sets globalAlpha for layer opacity", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { opacity: 0.7 }),
      ]),
      ctx,
    );
    expect(ctx.globalAlpha).toBe(0.7);
  });
});

// ============================================================================
// PaintClip handler
// ============================================================================

describe("PaintClip handler", () => {
  it("calls save, clip, restore around children", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintClip(0, 0, 100, 100, [
          paintRect(0, 0, 200, 200, { fill: "#3b82f6" }),
        ]),
      ]),
      ctx,
    );
    expect(ctx.save).toHaveBeenCalled();
    expect(ctx.clip).toHaveBeenCalled();
    expect(ctx.restore).toHaveBeenCalled();
  });

  it("renders children inside the clip region", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintClip(0, 0, 100, 100, [
          paintRect(0, 0, 200, 200, { fill: "#3b82f6" }),
          paintEllipse(50, 50, 20, 20, { fill: "#fff" }),
        ]),
      ]),
      ctx,
    );
    expect(ctx.fillRect).toHaveBeenCalled();
    expect(ctx.ellipse).toHaveBeenCalled();
  });
});

// ============================================================================
// PaintGradient handler
// ============================================================================

describe("PaintGradient handler", () => {
  it("stores a linear gradient in the registry for later use", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 300, "transparent", [
        paintGradient("linear",
          [{ offset: 0, color: "#3b82f6" }, { offset: 1, color: "#8b5cf6" }],
          { id: "grad1", x1: 0, y1: 0, x2: 400, y2: 0 },
        ),
      ]),
      ctx,
    );
    // Verify the gradient can be resolved from the registry
    const resolved = resolveFill("url(#grad1)", ctx);
    expect(resolved).not.toBe("url(#grad1)"); // resolved to CanvasGradient, not the fallback string
  });

  it("returns the fill string as-is for non-url fills", () => {
    const ctx = makeCtx();
    expect(resolveFill("#3b82f6", ctx)).toBe("#3b82f6");
    expect(resolveFill("rgba(0,0,0,0.5)", ctx)).toBe("rgba(0,0,0,0.5)");
  });

  it("returns the fill string unchanged for unknown url references", () => {
    const ctx = makeCtx();
    expect(resolveFill("url(#nonexistent)", ctx)).toBe("url(#nonexistent)");
  });

  it("silently ignores a gradient without an id", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(100, 100, "transparent", [
          paintGradient("linear", [{ offset: 0, color: "#000" }]),
        ]),
        ctx,
      ),
    ).not.toThrow();
  });

  it("stores a radial gradient in the registry", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintGradient("radial",
          [{ offset: 0, color: "#fff" }, { offset: 1, color: "#000" }],
          { id: "radial1", cx: 100, cy: 100, r: 50 },
        ),
      ]),
      ctx,
    );
    const resolved = resolveFill("url(#radial1)", ctx);
    expect(resolved).not.toBe("url(#radial1)");
  });

  it("clears the gradient registry on each execute()", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    // First execute registers a gradient
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintGradient("linear", [{ offset: 0, color: "#000" }], { id: "g1", x1: 0, y1: 0, x2: 100, y2: 0 }),
      ]),
      ctx,
    );
    expect(resolveFill("url(#g1)", ctx)).not.toBe("url(#g1)");
    // Second execute clears the registry (no gradient registered this time)
    vm.execute(paintScene(200, 200, "transparent", []), ctx);
    // Now the old gradient should be gone
    expect(resolveFill("url(#g1)", ctx)).toBe("url(#g1)");
  });
});

// ============================================================================
// PaintImage handler
// ============================================================================

describe("PaintImage handler", () => {
  it("calls putImageData for a PixelContainer src (fixed RGBA8)", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();

    // PixelContainer is now fixed RGBA8: { width, height, data: Uint8Array }.
    // The old interface (channels, bit_depth, pixels) was removed when
    // pixel-container was simplified from a configurable type to a fixed RGBA8 type.
    const pixels: PixelContainer = {
      width: 10,
      height: 10,
      data: new Uint8Array(10 * 10 * 4),
    };
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintImage(5, 5, 10, 10, pixels),
      ]),
      ctx,
    );
    expect(ctx.putImageData).toHaveBeenCalled();
  });

  it("skips putImageData when data.length does not match width*height*4 (DoS guard)", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    // Declared 10×10 but only provides 16 bytes — dimension/length mismatch.
    // Without the length check ImageData would throw a DOMException; with it
    // we skip silently and render nothing.
    const pixels: PixelContainer = {
      width: 10,
      height: 10,
      data: new Uint8Array(16), // should be 10*10*4=400
    };
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintImage(0, 0, 10, 10, pixels),
      ]),
      ctx,
    );
    expect(ctx.putImageData).not.toHaveBeenCalled();
  });

  it("draws placeholder rect for URI string src", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintImage(10, 10, 100, 80, "https://example.com/photo.png"),
      ]),
      ctx,
    );
    // Placeholder rect is drawn via save/fillRect/restore
    expect(ctx.save).toHaveBeenCalled();
    expect(ctx.fillRect).toHaveBeenCalled();
  });

  it("sets globalAlpha when opacity is specified on a PixelContainer image", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();

    const pixels: PixelContainer = {
      width: 4,
      height: 4,
      data: new Uint8Array(4 * 4 * 4),
    };
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintImage(0, 0, 4, 4, pixels, { opacity: 0.5 }),
      ]),
      ctx,
    );
    expect(ctx.globalAlpha).toBe(0.5);
  });
});

// ============================================================================
// PaintGlyphRun handler
// ============================================================================

describe("PaintGlyphRun handler", () => {
  it("calls fillText for each glyph", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [
            { glyph_id: 65, x: 10, y: 50 },
            { glyph_id: 66, x: 20, y: 50 },
          ],
          font_ref: "Inter",
          font_size: 16,
        },
      ]),
      ctx,
    );
    expect(ctx.fillText).toHaveBeenCalledTimes(2);
  });

  it("uses default black fill", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [{ glyph_id: 65, x: 10, y: 50 }],
          font_ref: "Inter",
          font_size: 12,
        },
      ]),
      ctx,
    );
    expect(ctx.fillStyle).toBe("#000000");
  });

  it("uses custom fill when specified", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [{ glyph_id: 65, x: 10, y: 50 }],
          font_ref: "Inter",
          font_size: 12,
          fill: "#ff0000",
        },
      ]),
      ctx,
    );
    expect(ctx.fillStyle).toBe("#ff0000");
  });

  it("sets font string from font_size and font_ref", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [{ glyph_id: 65, x: 10, y: 50 }],
          font_ref: "Helvetica",
          font_size: 24,
        },
      ]),
      ctx,
    );
    expect(ctx.font).toBe("24px Helvetica");
  });

  it("sanitizes font_ref to strip CSS-injection characters", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [{ glyph_id: 65, x: 10, y: 50 }],
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          font_ref: "Inter; content: attr(x)" as any,
          font_size: 14,
        },
      ]),
      ctx,
    );
    // The semicolon and colon are stripped; parentheses are stripped
    expect(ctx.font).not.toContain(";");
    expect(ctx.font).not.toContain("(");
    expect(ctx.font).toContain("Inter");
  });
});

// ============================================================================
// export()
// ============================================================================

// ============================================================================
// Security hardening tests
// ============================================================================

describe("Security — filter numeric validation (canvas)", () => {
  it("throws RangeError when blur radius is NaN", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [
          paintLayer([], { filters: [{ kind: "blur", radius: NaN }] }),
        ]),
        ctx,
      ),
    ).toThrow(RangeError);
  });

  it("throws RangeError when drop_shadow dx is Infinity", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [
          paintLayer([], {
            filters: [{ kind: "drop_shadow", dx: Infinity, dy: 4, blur: 8, color: "#000" }],
          }),
        ]),
        ctx,
      ),
    ).toThrow(RangeError);
  });

  it("replaces unsafe drop_shadow color with 'black'", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        paintLayer([], { filters: [{ kind: "drop_shadow", dx: 2, dy: 2, blur: 4, color: ") brightness(100" as any }] }),
      ]),
      ctx,
    );
    // The injected color is replaced with "black"; the injected content must not appear
    expect(ctx.filter).not.toContain(") brightness(100");
    expect(ctx.filter).toContain("drop-shadow(2px 2px 4px black)");
  });
});

describe("Security — font_size validation (canvas)", () => {
  it("throws RangeError when font_size is NaN", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [
          {
            kind: "glyph_run",
            glyphs: [{ glyph_id: 65, x: 10, y: 50 }],
            font_ref: "Inter",
            font_size: NaN,
          },
        ]),
        ctx,
      ),
    ).toThrow(RangeError);
  });
});

describe("Security — glyph_id guard (canvas)", () => {
  it("substitutes U+FFFD for out-of-range glyph_id instead of throwing", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    // String.fromCodePoint would throw for 0x200000 without the guard
    expect(() =>
      vm.execute(
        paintScene(200, 200, "transparent", [
          {
            kind: "glyph_run",
            glyphs: [{ glyph_id: 0x200000, x: 10, y: 50 }],
            font_ref: "Inter",
            font_size: 16,
          },
        ]),
        ctx,
      ),
    ).not.toThrow();
    // fillText should still be called (with the replacement character)
    expect(ctx.fillText).toHaveBeenCalledWith("\uFFFD", 10, 50);
  });
});

describe("Security — blend mode allowlist (canvas)", () => {
  it("falls back to source-over for unknown blend mode instead of passing through arbitrary string", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        paintLayer([], { blend_mode: "hack;something" as any }),
      ]),
      ctx,
    );
    // The malicious blend mode must not be set; falls back to source-over (default)
    expect(ctx.globalCompositeOperation).toBe("source-over");
  });
});

describe("export()", () => {
  it("throws ExportNotSupportedError when OffscreenCanvas is not available", () => {
    const vm = createCanvasVM();
    const scene = paintScene(100, 100, "#fff", []);
    const original = (globalThis as Record<string, unknown>).OffscreenCanvas;
    delete (globalThis as Record<string, unknown>).OffscreenCanvas;
    try {
      expect(() => vm.export(scene)).toThrowError(ExportNotSupportedError);
    } finally {
      if (original !== undefined) {
        (globalThis as Record<string, unknown>).OffscreenCanvas = original;
      }
    }
  });
});

// ============================================================================
// Filter CSS string generation — all filter kinds
// ============================================================================

describe("Canvas filter string", () => {
  it("sets drop_shadow filter", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          filters: [{ kind: "drop_shadow", dx: 4, dy: 4, blur: 8, color: "#000" }],
        }),
      ]),
      ctx,
    );
    expect(ctx.filter).toContain("drop-shadow(4px 4px 8px #000)");
  });

  it("chains contrast, saturate, hue_rotate, invert, opacity filters", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          filters: [
            { kind: "contrast", amount: 1.5 },
            { kind: "saturate", amount: 2.0 },
            { kind: "hue_rotate", angle: 45 },
            { kind: "invert", amount: 0.5 },
            { kind: "opacity", amount: 0.8 },
          ],
        }),
      ]),
      ctx,
    );
    expect(ctx.filter).toContain("contrast(1.5)");
    expect(ctx.filter).toContain("saturate(2)");
    expect(ctx.filter).toContain("hue-rotate(45deg)");
    expect(ctx.filter).toContain("invert(0.5)");
    expect(ctx.filter).toContain("opacity(0.8)");
  });

  it("skips color_matrix filter (no CSS equivalent) — filter stays 'none'", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          filters: [{ kind: "color_matrix", matrix: new Array(20).fill(0) as number[] }],
        }),
      ]),
      ctx,
    );
    // color_matrix has no CSS equivalent — buildCanvasFilter returns "none"
    // handleLayer only sets ctx.filter when filterStr !== "none"
    expect(ctx.filter).toBe("none");
  });

  it("does not set ctx.filter when filters array is empty", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([], { filters: [] }),
      ]),
      ctx,
    );
    expect(ctx.filter).toBe("none"); // mock initial value — not changed
  });

  it("does not set ctx.filter when no filters property", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(200, 200, "transparent", [
        paintLayer([]),
      ]),
      ctx,
    );
    expect(ctx.filter).toBe("none");
  });
});

// ============================================================================
// PaintText — TXT03d canvas-native text path
// ============================================================================

describe("PaintText dispatch", () => {
  it("sets ctx.font from a minimal canvas: font_ref and calls fillText", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(20, 40, "Hello", "canvas:Helvetica@16", 16, "#111"),
      ]),
      ctx,
    );
    // font shorthand: weight 400 is the default when no weight suffix is present
    expect(ctx.font).toBe("400 16px 'Helvetica'");
    expect(ctx.fillStyle).toBe("#111");
    expect(ctx.textBaseline).toBe("alphabetic");
    expect(ctx.fillText).toHaveBeenCalledWith("Hello", 20, 40);
  });

  it("parses weight from canvas:<family>@<size>:<weight>", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(0, 20, "Bold", "canvas:Helvetica@16:700", 16, "#000"),
      ]),
      ctx,
    );
    expect(ctx.font).toBe("700 16px 'Helvetica'");
  });

  it("parses italic style from canvas:<family>@<size>:<weight>:italic", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(0, 20, "Oblique", "canvas:Georgia@14:400:italic", 14, "#000"),
      ]),
      ctx,
    );
    expect(ctx.font).toBe("italic 400 14px 'Georgia'");
  });

  it("ignores the size encoded in font_ref — font_size argument wins", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    // font_ref says 16; but font_size is the authoritative size from the layout engine
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(0, 40, "Sized", "canvas:Helvetica@16:700", 32, "#000"),
      ]),
      ctx,
    );
    expect(ctx.font).toBe("700 32px 'Helvetica'");
  });

  it("sanitizes a family name that contains CSS-injection characters", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    // Adversarial family with characters that could break out of the string
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(0, 20, "X", "canvas:Helvetica'; color: red;@16:400", 16, "#000"),
      ]),
      ctx,
    );
    // Semicolons, apostrophes, and braces stripped; only alnum/space/hyphen/comma
    // remain inside the quoted family name.
    expect(ctx.font).toMatch(/^400 16px '[A-Za-z ]+'$/);
    expect(ctx.font).not.toContain(";");
    expect(ctx.font).not.toContain("'; ");
  });

  it("clamps a nonsense weight to the default 400", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(0, 20, "X", "canvas:Helvetica@16:99999", 16, "#000"),
      ]),
      ctx,
    );
    expect(ctx.font).toBe("400 16px 'Helvetica'");
  });

  it("throws UnsupportedFontBindingError for non-canvas scheme", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(400, 100, "transparent", [
          paintText(0, 20, "X", "coretext:Helvetica-Bold@16", 16, "#000"),
        ]),
        ctx,
      ),
    ).toThrow(UnsupportedFontBindingError);
  });

  it("throws RangeError when font_size is not a finite number", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    expect(() =>
      vm.execute(
        paintScene(400, 100, "transparent", [
          paintText(0, 20, "X", "canvas:Helvetica@16", NaN, "#000"),
        ]),
        ctx,
      ),
    ).toThrow(RangeError);
  });

  it("falls back to sans-serif when family portion is empty", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(0, 20, "X", "canvas:@16:400", 16, "#000"),
      ]),
      ctx,
    );
    expect(ctx.font).toBe("400 16px 'sans-serif'");
  });

  it("honors text_align by setting ctx.textAlign before fillText", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(50, 20, "Centered", "canvas:Helvetica@16", 16, "#000", { text_align: "center" }),
      ]),
      ctx,
    );
    expect(ctx.textAlign).toBe("center");
    expect(ctx.fillText).toHaveBeenCalledWith("Centered", 50, 20);
  });

  it("supports text_align = \"end\" for right-aligned cells", () => {
    const vm = createCanvasVM();
    const ctx = makeCtx();
    vm.execute(
      paintScene(400, 100, "transparent", [
        paintText(100, 20, "Right", "canvas:Helvetica@16", 16, "#000", { text_align: "end" }),
      ]),
      ctx,
    );
    expect(ctx.textAlign).toBe("end");
  });
});
