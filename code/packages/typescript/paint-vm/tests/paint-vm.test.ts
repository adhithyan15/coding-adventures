import { describe, expect, it, vi } from "vitest";
import {
  VERSION,
  PaintVM,
  UnknownInstructionError,
  DuplicateHandlerError,
  ExportNotSupportedError,
  NullContextError,
  deepEqual,
} from "../src/index.js";
import {
  paintScene,
  paintRect,
  paintEllipse,
  paintGroup,
  paintLayer,
  paintClip,
  type PaintScene,
  type PaintInstruction,
  type PixelContainer,
} from "@coding-adventures/paint-instructions";

// ============================================================================
// VERSION
// ============================================================================

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ============================================================================
// Test context type — a simple string log for testing
// ============================================================================

// For testing, TContext is a string[] log.
// Handlers push a description of what they rendered into the log.
type TestCtx = string[];

function makeVM() {
  const clear = vi.fn((ctx: TestCtx, bg: string) => {
    ctx.push(`clear:${bg}`);
  });
  const vm = new PaintVM<TestCtx>(clear);
  vm.register("rect", (instr, ctx) => {
    if (instr.kind === "rect") ctx.push(`rect:${instr.x},${instr.y}`);
  });
  vm.register("ellipse", (instr, ctx) => {
    if (instr.kind === "ellipse") ctx.push(`ellipse:${instr.cx},${instr.cy}`);
  });
  vm.register("group", (instr, ctx, vm) => {
    if (instr.kind === "group") {
      ctx.push("group:start");
      for (const child of instr.children) vm.dispatch(child, ctx);
      ctx.push("group:end");
    }
  });
  vm.register("layer", (instr, ctx, vm) => {
    if (instr.kind === "layer") {
      ctx.push("layer:start");
      for (const child of instr.children) vm.dispatch(child, ctx);
      ctx.push("layer:end");
    }
  });
  vm.register("clip", (instr, ctx, vm) => {
    if (instr.kind === "clip") {
      ctx.push(`clip:${instr.width}x${instr.height}`);
      for (const child of instr.children) vm.dispatch(child, ctx);
    }
  });
  return { vm, clear };
}

// ============================================================================
// register()
// ============================================================================

describe("register()", () => {
  it("registers a handler without error", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    expect(() => vm.register("rect", vi.fn())).not.toThrow();
  });

  it("throws DuplicateHandlerError on double registration", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    vm.register("rect", vi.fn());
    expect(() => vm.register("rect", vi.fn())).toThrowError(
      DuplicateHandlerError,
    );
  });

  it("DuplicateHandlerError carries the kind", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    vm.register("ellipse", vi.fn());
    try {
      vm.register("ellipse", vi.fn());
    } catch (e) {
      expect(e).toBeInstanceOf(DuplicateHandlerError);
      expect((e as DuplicateHandlerError).kind).toBe("ellipse");
    }
  });

  it("returns the registered kinds list", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    vm.register("rect", vi.fn());
    vm.register("ellipse", vi.fn());
    expect(vm.registeredKinds().sort()).toEqual(["ellipse", "rect"]);
  });
});

// ============================================================================
// dispatch()
// ============================================================================

describe("dispatch()", () => {
  it("dispatches a rect to the rect handler", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    vm.dispatch(paintRect(5, 10, 100, 50), log);
    expect(log).toEqual(["rect:5,10"]);
  });

  it("throws UnknownInstructionError for unregistered kind", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    const instr: PaintInstruction = paintRect(0, 0, 10, 10);
    expect(() => vm.dispatch(instr, [])).toThrowError(UnknownInstructionError);
  });

  it("UnknownInstructionError carries the kind", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    try {
      vm.dispatch(paintRect(0, 0, 10, 10), []);
    } catch (e) {
      expect(e).toBeInstanceOf(UnknownInstructionError);
      expect((e as UnknownInstructionError).kind).toBe("rect");
    }
  });

  it("uses wildcard '*' handler when no specific handler is registered", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    const log: TestCtx = [];
    vm.register("*", (instr, ctx) => {
      ctx.push(`wildcard:${instr.kind}`);
    });
    vm.dispatch(paintRect(0, 0, 10, 10), log);
    expect(log).toEqual(["wildcard:rect"]);
  });

  it("specific handler takes precedence over wildcard", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    const log: TestCtx = [];
    vm.register("rect", (_instr, ctx) => ctx.push("specific:rect"));
    vm.register("*", (_instr, ctx) => ctx.push("wildcard"));
    vm.dispatch(paintRect(0, 0, 10, 10), log);
    expect(log).toEqual(["specific:rect"]);
  });
});

// ============================================================================
// execute()
// ============================================================================

describe("execute()", () => {
  it("calls clear with background before dispatching instructions", () => {
    const { vm, clear } = makeVM();
    const log: TestCtx = [];
    const scene = paintScene(100, 50, "#f0f0f0", [paintRect(0, 0, 10, 10)]);
    vm.execute(scene, log);
    expect(clear).toHaveBeenCalledWith(log, "#f0f0f0", 100, 50);
    expect(log[0]).toBe("clear:#f0f0f0");
  });

  it("dispatches instructions in order (painter's algorithm)", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const scene = paintScene(100, 100, "#fff", [
      paintRect(0, 0, 10, 10),
      paintEllipse(50, 50, 20, 20),
      paintRect(90, 90, 10, 10),
    ]);
    vm.execute(scene, log);
    expect(log).toEqual([
      "clear:#fff",
      "rect:0,0",
      "ellipse:50,50",
      "rect:90,90",
    ]);
  });

  it("executes an empty scene (just clears)", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    vm.execute(paintScene(100, 100, "transparent", []), log);
    expect(log).toEqual(["clear:transparent"]);
  });

  it("throws NullContextError for null context", () => {
    const { vm } = makeVM();
    expect(() =>
      vm.execute(paintScene(100, 100, "#fff", []), null as unknown as TestCtx),
    ).toThrowError(NullContextError);
  });

  it("recurses into group children via handler", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const scene = paintScene(100, 100, "#fff", [
      paintGroup([paintRect(1, 2, 10, 10), paintEllipse(5, 5, 3, 3)]),
    ]);
    vm.execute(scene, log);
    expect(log).toEqual([
      "clear:#fff",
      "group:start",
      "rect:1,2",
      "ellipse:5,5",
      "group:end",
    ]);
  });

  it("recurses into layer children via handler", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const scene = paintScene(100, 100, "#fff", [
      paintLayer([paintRect(0, 0, 50, 50)]),
    ]);
    vm.execute(scene, log);
    expect(log).toContain("layer:start");
    expect(log).toContain("layer:end");
    expect(log).toContain("rect:0,0");
  });

  it("recurses into clip children via handler", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const scene = paintScene(100, 100, "#fff", [
      paintClip(0, 0, 400, 300, [paintRect(10, 10, 50, 50)]),
    ]);
    vm.execute(scene, log);
    expect(log).toContain("clip:400x300");
    expect(log).toContain("rect:10,10");
  });

  it("throws UnknownInstructionError for an unregistered kind in the scene", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    const scene = paintScene(100, 100, "#fff", [paintRect(0, 0, 10, 10)]);
    expect(() => vm.execute(scene, [])).toThrowError(UnknownInstructionError);
  });
});

// ============================================================================
// patch()
// ============================================================================

describe("patch()", () => {
  it("falls back to execute() when no callbacks provided", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const old = paintScene(100, 100, "#fff", [paintRect(0, 0, 10, 10)]);
    const next = paintScene(100, 100, "#fff", [paintRect(0, 0, 20, 20)]);
    vm.patch(old, next, log);
    expect(log).toContain("rect:0,0"); // execute() called
  });

  it("calls onDelete for ids in old but not in new", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const old = paintScene(100, 100, "#fff", [
      paintRect(0, 0, 10, 10, { id: "bar-1" }),
      paintRect(20, 0, 10, 10, { id: "bar-2" }),
    ]);
    const next = paintScene(100, 100, "#fff", [
      paintRect(20, 0, 10, 10, { id: "bar-2" }),
    ]);
    const deleted: string[] = [];
    vm.patch(old, next, log, {
      onDelete: (instr) => {
        if (instr.id) deleted.push(instr.id);
      },
    });
    expect(deleted).toEqual(["bar-1"]);
  });

  it("calls onUpdate when an identified instruction changes", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const old = paintScene(100, 100, "#fff", [
      paintRect(0, 0, 10, 10, { id: "box", fill: "#blue" }),
    ]);
    const next = paintScene(100, 100, "#fff", [
      paintRect(0, 0, 10, 10, { id: "box", fill: "#red" }),
    ]);
    let updateCalled = false;
    vm.patch(old, next, log, {
      onUpdate: (oldI, newI) => {
        updateCalled = true;
        expect(oldI.id).toBe("box");
        expect(newI.id).toBe("box");
      },
    });
    expect(updateCalled).toBe(true);
  });

  it("skips identical identified instructions", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const rect = paintRect(0, 0, 10, 10, { id: "static-box", fill: "#fff" });
    const old = paintScene(100, 100, "#fff", [rect]);
    const next = paintScene(100, 100, "#fff", [
      paintRect(0, 0, 10, 10, { id: "static-box", fill: "#fff" }),
    ]);
    const updateCalled = vi.fn();
    vm.patch(old, next, log, { onUpdate: updateCalled });
    expect(updateCalled).not.toHaveBeenCalled();
  });

  it("calls onInsert for new instructions at positions beyond old length", () => {
    const { vm } = makeVM();
    const log: TestCtx = [];
    const old = paintScene(100, 100, "#fff", [paintRect(0, 0, 10, 10)]);
    const next = paintScene(100, 100, "#fff", [
      paintRect(0, 0, 10, 10),
      paintEllipse(50, 50, 20, 20), // new at position 1
    ]);
    const inserted: number[] = [];
    vm.patch(old, next, log, {
      onInsert: (_instr, pos) => inserted.push(pos),
    });
    expect(inserted).toEqual([1]);
  });

  it("throws NullContextError for null context", () => {
    const { vm } = makeVM();
    const scene = paintScene(100, 100, "#fff", []);
    expect(() =>
      vm.patch(scene, scene, null as unknown as TestCtx),
    ).toThrowError(NullContextError);
  });
});

// ============================================================================
// export()
// ============================================================================

describe("export()", () => {
  it("throws ExportNotSupportedError when no exportFn is provided", () => {
    const vm = new PaintVM<TestCtx>(vi.fn());
    expect(() =>
      vm.export(paintScene(100, 100, "#fff", [])),
    ).toThrowError(ExportNotSupportedError);
  });

  it("calls exportFn and returns PixelContainer", () => {
    const mockPixels: PixelContainer = {
      width: 100,
      height: 100,
      channels: 4,
      bit_depth: 8,
      pixels: new Uint8Array(100 * 100 * 4),
    };
    const exportFn = vi.fn(() => mockPixels);
    const vm = new PaintVM<TestCtx>(vi.fn(), exportFn);
    const scene = paintScene(100, 100, "#fff", []);
    const result = vm.export(scene);
    expect(result).toBe(mockPixels);
    expect(exportFn).toHaveBeenCalledOnce();
  });

  it("passes scale option to exportFn", () => {
    const exportFn = vi.fn(
      (_scene: PaintScene, _vm: unknown, opts: { scale: number }) => ({
        width: Math.round(100 * opts.scale),
        height: Math.round(100 * opts.scale),
        channels: 4 as const,
        bit_depth: 8 as const,
        pixels: new Uint8Array(4),
      }),
    );
    const vm = new PaintVM<TestCtx>(vi.fn(), exportFn as never);
    const scene = paintScene(100, 100, "#fff", []);
    const result = vm.export(scene, { scale: 2 });
    expect(result.width).toBe(200);
    expect(result.height).toBe(200);
  });

  it("defaults scale to 1.0, channels to 4, bit_depth to 8, color_space to srgb", () => {
    const exportFn = vi.fn(
      (_scene: PaintScene, _vm: unknown, opts: { scale: number; channels: number; bit_depth: number; color_space: string }) => {
        expect(opts.scale).toBe(1.0);
        expect(opts.channels).toBe(4);
        expect(opts.bit_depth).toBe(8);
        expect(opts.color_space).toBe("srgb");
        return {
          width: 1, height: 1, channels: 4 as const, bit_depth: 8 as const,
          pixels: new Uint8Array(4),
        };
      },
    );
    const vm = new PaintVM<TestCtx>(vi.fn(), exportFn as never);
    vm.export(paintScene(1, 1, "#fff", []));
  });
});

// ============================================================================
// deepEqual()
// ============================================================================

describe("deepEqual()", () => {
  it("returns true for identical primitives", () => {
    expect(deepEqual(1, 1)).toBe(true);
    expect(deepEqual("hello", "hello")).toBe(true);
    expect(deepEqual(true, true)).toBe(true);
    expect(deepEqual(null, null)).toBe(true);
    expect(deepEqual(undefined, undefined)).toBe(true);
  });

  it("returns false for different primitives", () => {
    expect(deepEqual(1, 2)).toBe(false);
    expect(deepEqual("a", "b")).toBe(false);
    expect(deepEqual(true, false)).toBe(false);
    expect(deepEqual(null, undefined)).toBe(false);
  });

  it("returns true for identical arrays", () => {
    expect(deepEqual([1, 2, 3], [1, 2, 3])).toBe(true);
    expect(deepEqual([], [])).toBe(true);
  });

  it("returns false for arrays of different length", () => {
    expect(deepEqual([1, 2], [1, 2, 3])).toBe(false);
  });

  it("returns false for arrays with different elements", () => {
    expect(deepEqual([1, 2, 3], [1, 2, 4])).toBe(false);
  });

  it("returns true for identical plain objects", () => {
    expect(deepEqual({ a: 1, b: "x" }, { a: 1, b: "x" })).toBe(true);
  });

  it("returns false for objects with different values", () => {
    expect(deepEqual({ a: 1 }, { a: 2 })).toBe(false);
  });

  it("returns false for objects with different keys", () => {
    expect(deepEqual({ a: 1 }, { b: 1 })).toBe(false);
  });

  it("returns true for deeply nested identical structures", () => {
    const a = { kind: "rect", x: 10, y: 20, fill: "#fff", nested: { arr: [1, 2] } };
    const b = { kind: "rect", x: 10, y: 20, fill: "#fff", nested: { arr: [1, 2] } };
    expect(deepEqual(a, b)).toBe(true);
  });

  it("returns false for deeply nested structures that differ", () => {
    const a = { nested: { arr: [1, 2, 3] } };
    const b = { nested: { arr: [1, 2, 4] } };
    expect(deepEqual(a, b)).toBe(false);
  });

  it("returns false when comparing array to object", () => {
    expect(deepEqual([1, 2], { 0: 1, 1: 2 })).toBe(false);
  });

  it("returns true for identical PaintRect instructions", () => {
    const r1 = paintRect(0, 0, 100, 50, { fill: "#fff", id: "box" });
    const r2 = paintRect(0, 0, 100, 50, { fill: "#fff", id: "box" });
    expect(deepEqual(r1, r2)).toBe(true);
  });

  it("returns false for PaintRect instructions with different fill", () => {
    const r1 = paintRect(0, 0, 100, 50, { fill: "#fff" });
    const r2 = paintRect(0, 0, 100, 50, { fill: "#000" });
    expect(deepEqual(r1, r2)).toBe(false);
  });
});

// ============================================================================
// Error classes
// ============================================================================

describe("error classes", () => {
  it("UnknownInstructionError has correct name and message", () => {
    const e = new UnknownInstructionError("svg:marker");
    expect(e.name).toBe("UnknownInstructionError");
    expect(e.kind).toBe("svg:marker");
    expect(e.message).toContain("svg:marker");
    expect(e).toBeInstanceOf(Error);
  });

  it("DuplicateHandlerError has correct name and message", () => {
    const e = new DuplicateHandlerError("rect");
    expect(e.name).toBe("DuplicateHandlerError");
    expect(e.kind).toBe("rect");
    expect(e.message).toContain("rect");
    expect(e).toBeInstanceOf(Error);
  });

  it("ExportNotSupportedError has correct name", () => {
    const e = new ExportNotSupportedError("terminal");
    expect(e.name).toBe("ExportNotSupportedError");
    expect(e.message).toContain("terminal");
    expect(e).toBeInstanceOf(Error);
  });

  it("NullContextError has correct name", () => {
    const e = new NullContextError();
    expect(e.name).toBe("NullContextError");
    expect(e).toBeInstanceOf(Error);
  });
});
