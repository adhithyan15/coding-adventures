/**
 * Tests for the Canvas renderer.
 *
 * Since these tests run in Node.js (not a browser), there is no real
 * CanvasRenderingContext2D available. We use a lightweight mock object that
 * records every call made to it via `vi.fn()`.
 *
 * This lets us verify:
 * - which canvas methods were called
 * - with which arguments
 * - in which order
 *
 * without spinning up a headless browser or pulling in a heavy canvas
 * polyfill like `canvas` (node-canvas). The mock is a plain object
 * cast to `CanvasRenderingContext2D` via `unknown` — TypeScript lets us
 * do that in test code where the mock satisfies the methods we actually call.
 */
import { describe, expect, it, vi } from "vitest";
import {
  createScene,
  drawClip,
  drawGroup,
  drawLine,
  drawRect,
  drawText,
} from "@coding-adventures/draw-instructions";
import {
  VERSION,
  createCanvasRenderer,
  renderCanvas,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Shared mock factory
// ---------------------------------------------------------------------------

/**
 * Build a fresh mock CanvasRenderingContext2D for each test.
 *
 * We only stub the methods and properties that draw-instructions-canvas
 * actually uses. Every method is a `vi.fn()` so we can inspect call counts
 * and arguments.
 */
function makeMockCtx() {
  return {
    // Settable style properties
    fillStyle: "" as string | CanvasGradient | CanvasPattern,
    strokeStyle: "" as string | CanvasGradient | CanvasPattern,
    lineWidth: 1 as number,
    font: "" as string,
    textAlign: "start" as CanvasTextAlign,

    // Rectangle drawing
    fillRect: vi.fn<[number, number, number, number], void>(),
    strokeRect: vi.fn<[number, number, number, number], void>(),

    // Text drawing
    fillText: vi.fn<[string, number, number], void>(),

    // Path operations
    beginPath: vi.fn<[], void>(),
    moveTo: vi.fn<[number, number], void>(),
    lineTo: vi.fn<[number, number], void>(),
    stroke: vi.fn<[], void>(),
    rect: vi.fn<[number, number, number, number], void>(),
    clip: vi.fn<[], void>(),

    // State stack
    save: vi.fn<[], void>(),
    restore: vi.fn<[], void>(),
  } as unknown as CanvasRenderingContext2D;
}

// ---------------------------------------------------------------------------
// VERSION
// ---------------------------------------------------------------------------

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

describe("background painting", () => {
  it("fills the scene background before any instructions", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, []);
    renderCanvas(scene, ctx);

    // The very first fillRect should be the background.
    expect(ctx.fillRect).toHaveBeenCalledWith(0, 0, 100, 50);
    expect(ctx.fillStyle).toBe("#ffffff");
  });

  it("uses the scene background color", () => {
    const ctx = makeMockCtx();
    const scene = createScene(80, 40, [], { background: "#112233" });
    renderCanvas(scene, ctx);

    // fillStyle is set by assignment — check what it was set to last time
    // before fillRect was called for the background (it's the first call).
    expect(ctx.fillStyle).toBe("#112233");
    expect(ctx.fillRect).toHaveBeenCalledWith(0, 0, 80, 40);
  });
});

// ---------------------------------------------------------------------------
// Rect instruction
// ---------------------------------------------------------------------------

describe("rect instruction", () => {
  it("calls fillRect with correct coordinates", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [drawRect(10, 20, 30, 40, "#aabbcc")]);
    renderCanvas(scene, ctx);

    // Second fillRect call (after background).
    expect(ctx.fillRect).toHaveBeenNthCalledWith(2, 10, 20, 30, 40);
    expect(ctx.fillStyle).toBe("#aabbcc");
  });

  it("does not call strokeRect when no stroke is set", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [drawRect(0, 0, 50, 25)]);
    renderCanvas(scene, ctx);

    expect(ctx.strokeRect).not.toHaveBeenCalled();
  });

  it("calls strokeRect when stroke color is set", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [
      drawRect(5, 5, 40, 20, "#fff", { stroke: "#ff0000", strokeWidth: 2 }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.strokeRect).toHaveBeenCalledWith(5, 5, 40, 20);
    expect(ctx.strokeStyle).toBe("#ff0000");
    expect(ctx.lineWidth).toBe(2);
  });

  it("defaults strokeWidth to 1 when stroke color is set but width is omitted", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [
      drawRect(5, 5, 40, 20, "#fff", { stroke: "#000" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.strokeRect).toHaveBeenCalled();
    expect(ctx.lineWidth).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Text instruction
// ---------------------------------------------------------------------------

describe("text instruction", () => {
  it("calls fillText with the right value and position", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [drawText(100, 50, "Hello")]);
    renderCanvas(scene, ctx);

    expect(ctx.fillText).toHaveBeenCalledWith("Hello", 100, 50);
  });

  it("builds the font string from fontSize and fontFamily", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawText(0, 0, "X", { fontSize: 18, fontFamily: "serif" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.font).toBe("normal 18px serif");
  });

  it("includes font-weight in the font string when bold", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawText(0, 0, "Bold", { fontWeight: "bold", fontSize: 14, fontFamily: "sans-serif" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.font).toBe("bold 14px sans-serif");
  });

  it('maps align "middle" to textAlign "center"', () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawText(100, 50, "Mid", { align: "middle" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.textAlign).toBe("center");
  });

  it('passes align "start" through unchanged', () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawText(0, 50, "Start", { align: "start" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.textAlign).toBe("start");
  });

  it('passes align "end" through unchanged', () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawText(200, 50, "End", { align: "end" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.textAlign).toBe("end");
  });

  it("uses the fill color from the instruction", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [
      drawText(50, 25, "Colored", { fill: "#00ff00" }),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.fillStyle).toBe("#00ff00");
  });
});

// ---------------------------------------------------------------------------
// Line instruction
// ---------------------------------------------------------------------------

describe("line instruction", () => {
  it("traces and strokes the correct path", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [drawLine(10, 20, 180, 20, "#333", 1)]);
    renderCanvas(scene, ctx);

    expect(ctx.beginPath).toHaveBeenCalledTimes(1);
    expect(ctx.moveTo).toHaveBeenCalledWith(10, 20);
    expect(ctx.lineTo).toHaveBeenCalledWith(180, 20);
    expect(ctx.stroke).toHaveBeenCalledTimes(1);
  });

  it("sets strokeStyle and lineWidth before stroking", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [drawLine(0, 0, 100, 100, "#0000ff", 3)]);
    renderCanvas(scene, ctx);

    expect(ctx.strokeStyle).toBe("#0000ff");
    expect(ctx.lineWidth).toBe(3);
  });

  it("does not call fillRect for a line", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [drawLine(0, 25, 100, 25)]);
    renderCanvas(scene, ctx);

    // fillRect is called once (background). Not for the line.
    expect(ctx.fillRect).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// Group instruction
// ---------------------------------------------------------------------------

describe("group instruction", () => {
  it("renders all children without save/restore", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawGroup([drawRect(10, 10, 20, 20), drawRect(40, 10, 20, 20)]),
    ]);
    renderCanvas(scene, ctx);

    // Two child rects + 1 background = 3 fillRect calls total.
    expect(ctx.fillRect).toHaveBeenCalledTimes(3);
    // No save/restore for a plain group.
    expect(ctx.save).not.toHaveBeenCalled();
    expect(ctx.restore).not.toHaveBeenCalled();
  });

  it("renders nested groups recursively", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawGroup([
        drawGroup([drawRect(5, 5, 10, 10)]),
      ]),
    ]);
    renderCanvas(scene, ctx);

    // 1 background + 1 child rect.
    expect(ctx.fillRect).toHaveBeenCalledTimes(2);
  });
});

// ---------------------------------------------------------------------------
// Clip instruction
// ---------------------------------------------------------------------------

describe("clip instruction", () => {
  it("saves and restores state around clip children", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawClip(10, 10, 80, 30, [drawText(50, 25, "Clipped")]),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.save).toHaveBeenCalledTimes(1);
    expect(ctx.restore).toHaveBeenCalledTimes(1);
  });

  it("defines the clip region with beginPath + rect + clip", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawClip(5, 10, 90, 40, [drawRect(0, 0, 200, 100)]),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.beginPath).toHaveBeenCalledTimes(1);
    expect(ctx.rect).toHaveBeenCalledWith(5, 10, 90, 40);
    expect(ctx.clip).toHaveBeenCalledTimes(1);
  });

  it("renders children inside the clip region", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawClip(0, 0, 100, 50, [drawText(50, 25, "Visible")]),
    ]);
    renderCanvas(scene, ctx);

    expect(ctx.fillText).toHaveBeenCalledWith("Visible", 50, 25);
  });

  it("handles empty clip children without error", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [drawClip(0, 0, 50, 25, [])]);
    renderCanvas(scene, ctx);

    expect(ctx.save).toHaveBeenCalledTimes(1);
    expect(ctx.restore).toHaveBeenCalledTimes(1);
    expect(ctx.fillText).not.toHaveBeenCalled();
  });

  it("supports nested clips with correct save/restore pairing", () => {
    const ctx = makeMockCtx();
    const scene = createScene(200, 100, [
      drawClip(0, 0, 100, 50, [
        drawClip(10, 10, 80, 30, [drawRect(20, 20, 40, 10)]),
      ]),
    ]);
    renderCanvas(scene, ctx);

    // Each clip: one save + one restore.
    expect(ctx.save).toHaveBeenCalledTimes(2);
    expect(ctx.restore).toHaveBeenCalledTimes(2);
  });
});

// ---------------------------------------------------------------------------
// createCanvasRenderer factory
// ---------------------------------------------------------------------------

describe("createCanvasRenderer()", () => {
  it("returns a DrawRenderer<void> that paints to the given context", () => {
    const ctx = makeMockCtx();
    const renderer = createCanvasRenderer(ctx);
    const scene = createScene(50, 50, [drawRect(5, 5, 10, 10)]);

    renderer.render(scene);

    // Background + one rect.
    expect(ctx.fillRect).toHaveBeenCalledTimes(2);
  });

  it("paints the same result as renderCanvas()", () => {
    const ctx1 = makeMockCtx();
    const ctx2 = makeMockCtx();
    const scene = createScene(100, 50, [
      drawRect(10, 10, 30, 20, "#ff0000"),
      drawText(50, 25, "Hi"),
    ]);

    const renderer = createCanvasRenderer(ctx1);
    renderer.render(scene);
    renderCanvas(scene, ctx2);

    // Same number of fill calls.
    expect(ctx1.fillRect).toHaveBeenCalledTimes(
      (ctx2.fillRect as ReturnType<typeof vi.fn>).mock.calls.length,
    );
    expect(ctx1.fillText).toHaveBeenCalledWith("Hi", 50, 25);
    expect(ctx2.fillText).toHaveBeenCalledWith("Hi", 50, 25);
  });
});

// ---------------------------------------------------------------------------
// Empty and edge cases
// ---------------------------------------------------------------------------

describe("edge cases", () => {
  it("renders an empty scene without error", () => {
    const ctx = makeMockCtx();
    const scene = createScene(0, 0, []);
    expect(() => renderCanvas(scene, ctx)).not.toThrow();
  });

  it("renders a single-pixel scene", () => {
    const ctx = makeMockCtx();
    const scene = createScene(1, 1, [drawRect(0, 0, 1, 1, "#000000")]);
    renderCanvas(scene, ctx);

    expect(ctx.fillRect).toHaveBeenCalledTimes(2); // background + rect
  });

  it("handles metadata on instructions without error", () => {
    const ctx = makeMockCtx();
    const scene = createScene(100, 50, [
      drawRect(0, 0, 10, 10, "#ccc", { metadata: { charIndex: 3 } }),
    ]);
    expect(() => renderCanvas(scene, ctx)).not.toThrow();
  });
});
