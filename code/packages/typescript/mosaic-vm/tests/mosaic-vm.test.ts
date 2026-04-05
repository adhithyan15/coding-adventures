/**
 * Tests for the MosaicVM (TypeScript).
 *
 * The VM is a pure traversal driver — it has no output format of its own.
 * These tests use a **recording renderer** that captures all method calls
 * in order, then assert on the call log. This verifies traversal order,
 * value normalization, and slot context management without generating any
 * real code.
 *
 * Test Categories
 * ---------------
 *
 *   1. **Traversal order** — beginComponent/endComponent, beginNode/endNode
 *   2. **Property resolution** — string, number, bool, dimension, color, enum
 *   3. **Slot ref resolution** — component slots, loop variables
 *   4. **Slot references as children** — renderSlotChild calls
 *   5. **When blocks** — beginWhen/endWhen with children
 *   6. **Each blocks** — beginEach/endEach, loop scope
 *   7. **Color parsing** — #rgb, #rrggbb, #rrggbbaa
 *   8. **Dimension parsing** — dp, sp, %
 *   9. **MosaicVMError** — unknown slot reference
 */

import { describe, it, expect } from "vitest";
import { MosaicVM, MosaicVMError } from "../src/vm.js";
import { analyzeMosaic } from "@coding-adventures/mosaic-analyzer";
import type { MosaicRenderer, MosaicEmitResult, ResolvedProperty, SlotContext } from "../src/types.js";
import type { MosaicSlot, MosaicType } from "@coding-adventures/mosaic-analyzer";

// ============================================================================
// Recording Renderer
// ============================================================================

/**
 * A mock renderer that records all method calls.
 *
 * Each call is appended to `calls` as a structured object with a `method` name
 * and relevant arguments. Tests can then assert on the call log to verify
 * traversal order and argument values.
 */
type Call =
  | { method: "beginComponent"; name: string; slotCount: number }
  | { method: "endComponent" }
  | { method: "beginNode"; tag: string; isPrimitive: boolean; propNames: string[] }
  | { method: "endNode"; tag: string }
  | { method: "renderSlotChild"; slotName: string }
  | { method: "beginWhen"; slotName: string }
  | { method: "endWhen" }
  | { method: "beginEach"; slotName: string; itemName: string }
  | { method: "endEach" };

class RecordingRenderer implements MosaicRenderer {
  calls: Call[] = [];
  resolvedProps: ResolvedProperty[] = [];

  beginComponent(name: string, slots: MosaicSlot[]): void {
    this.calls.push({ method: "beginComponent", name, slotCount: slots.length });
  }
  endComponent(): void {
    this.calls.push({ method: "endComponent" });
  }
  emit(): MosaicEmitResult {
    return { files: [{ filename: "out.txt", content: "test" }] };
  }
  beginNode(tag: string, isPrimitive: boolean, properties: ResolvedProperty[], _ctx: SlotContext): void {
    this.calls.push({ method: "beginNode", tag, isPrimitive, propNames: properties.map((p) => p.name) });
    this.resolvedProps.push(...properties);
  }
  endNode(tag: string): void {
    this.calls.push({ method: "endNode", tag });
  }
  renderSlotChild(slotName: string, _slotType: MosaicType, _ctx: SlotContext): void {
    this.calls.push({ method: "renderSlotChild", slotName });
  }
  beginWhen(slotName: string, _ctx: SlotContext): void {
    this.calls.push({ method: "beginWhen", slotName });
  }
  endWhen(): void {
    this.calls.push({ method: "endWhen" });
  }
  beginEach(slotName: string, itemName: string, _elementType: MosaicType, _ctx: SlotContext): void {
    this.calls.push({ method: "beginEach", slotName, itemName });
  }
  endEach(): void {
    this.calls.push({ method: "endEach" });
  }
}

function run(source: string): { renderer: RecordingRenderer; result: MosaicEmitResult } {
  const ir = analyzeMosaic(source);
  const vm = new MosaicVM(ir);
  const renderer = new RecordingRenderer();
  const result = vm.run(renderer);
  return { renderer, result };
}

function methods(renderer: RecordingRenderer): string[] {
  return renderer.calls.map((c) => c.method);
}

// ============================================================================
// 1. Traversal Order
// ============================================================================

describe("traversal order", () => {
  it("calls beginComponent and endComponent once", () => {
    const { renderer } = run("component X { Row {} }");
    const calls = methods(renderer);
    expect(calls.filter((m) => m === "beginComponent")).toHaveLength(1);
    expect(calls.filter((m) => m === "endComponent")).toHaveLength(1);
  });

  it("beginComponent comes before any node calls", () => {
    const { renderer } = run("component X { Row {} }");
    const calls = methods(renderer);
    expect(calls.indexOf("beginComponent")).toBeLessThan(calls.indexOf("beginNode"));
  });

  it("endComponent comes after all node calls", () => {
    const { renderer } = run("component X { Row {} }");
    const calls = methods(renderer);
    expect(calls.indexOf("endComponent")).toBeGreaterThan(calls.lastIndexOf("endNode"));
  });

  it("beginComponent receives correct name and slot count", () => {
    const { renderer } = run("component Profile { slot name: text; Row {} }");
    const bc = renderer.calls[0] as { method: "beginComponent"; name: string; slotCount: number };
    expect(bc.name).toBe("Profile");
    expect(bc.slotCount).toBe(1);
  });

  it("emits beginNode / endNode pairs for root", () => {
    const { renderer } = run("component X { Column {} }");
    expect(methods(renderer)).toContain("beginNode");
    expect(methods(renderer)).toContain("endNode");
  });

  it("nested nodes: outer beginNode comes before inner, outer endNode after inner", () => {
    const { renderer } = run("component X { Column { Row {} } }");
    const calls = methods(renderer);
    const begins = calls.map((m, i) => ({ m, i })).filter((x) => x.m === "beginNode");
    const ends = calls.map((m, i) => ({ m, i })).filter((x) => x.m === "endNode");
    // beginNode(Column) before beginNode(Row)
    expect(begins[0].i).toBeLessThan(begins[1].i);
    // endNode(Row) before endNode(Column)
    expect(ends[0].i).toBeLessThan(ends[1].i);
  });

  it("run() returns the renderer emit() result", () => {
    const { result } = run("component X { Row {} }");
    expect(result.files).toHaveLength(1);
    expect(result.files[0].filename).toBe("out.txt");
  });
});

// ============================================================================
// 2. Property Resolution
// ============================================================================

describe("property resolution", () => {
  const resolveProp = (propDef: string) => {
    const { renderer } = run(`component X { Row { ${propDef} } }`);
    return renderer.resolvedProps[0];
  };

  it("string property", () => {
    const p = resolveProp('label: "hello";');
    expect(p.name).toBe("label");
    expect(p.value).toEqual({ kind: "string", value: "hello" });
  });

  it("number property", () => {
    const p = resolveProp("opacity: 0;");
    expect(p.value).toEqual({ kind: "number", value: 0 });
  });

  it("bool property (true)", () => {
    const p = resolveProp("disabled: true;");
    expect(p.value).toEqual({ kind: "bool", value: true });
  });

  it("bool property (false)", () => {
    const p = resolveProp("disabled: false;");
    expect(p.value).toEqual({ kind: "bool", value: false });
  });

  it("ident property becomes string kind", () => {
    // Bare identifiers (e.g., 'align: center;') are folded into { kind: "string" }
    const p = resolveProp("align: center;");
    expect(p.value.kind).toBe("string");
    expect((p.value as { kind: "string"; value: string }).value).toBe("center");
  });

  it("enum property", () => {
    const p = resolveProp("style: heading.small;");
    expect(p.value).toEqual({ kind: "enum", namespace: "heading", member: "small" });
  });
});

// ============================================================================
// 3. Color Parsing
// ============================================================================

describe("color parsing", () => {
  const resolveColor = (hex: string) => {
    const { renderer } = run(`component X { Row { bg: ${hex}; } }`);
    return renderer.resolvedProps[0].value as { kind: "color"; r: number; g: number; b: number; a: number };
  };

  it("parses #rrggbb (6-digit hex)", () => {
    const c = resolveColor("#2563eb");
    expect(c.kind).toBe("color");
    expect(c.r).toBe(0x25);
    expect(c.g).toBe(0x63);
    expect(c.b).toBe(0xeb);
    expect(c.a).toBe(255);
  });

  it("parses #fff (3-digit hex, doubles each digit)", () => {
    const c = resolveColor("#fff");
    expect(c.kind).toBe("color");
    expect(c.r).toBe(255);
    expect(c.g).toBe(255);
    expect(c.b).toBe(255);
    expect(c.a).toBe(255);
  });

  it("parses #000000ff (8-digit hex with full alpha)", () => {
    const c = resolveColor("#000000ff");
    expect(c.r).toBe(0);
    expect(c.g).toBe(0);
    expect(c.b).toBe(0);
    expect(c.a).toBe(255);
  });

  it("parses #00000000 (8-digit hex with zero alpha)", () => {
    const c = resolveColor("#00000000");
    expect(c.a).toBe(0);
  });
});

// ============================================================================
// 4. Dimension Parsing
// ============================================================================

describe("dimension parsing", () => {
  const resolveDim = (dimProp: string) => {
    const { renderer } = run(`component X { Row { padding: ${dimProp}; } }`);
    return renderer.resolvedProps[0].value as { kind: "dimension"; value: number; unit: string };
  };

  it("dp unit", () => {
    const d = resolveDim("16dp");
    expect(d.kind).toBe("dimension");
    expect(d.value).toBe(16);
    expect(d.unit).toBe("dp");
  });

  it("sp unit", () => {
    const d = resolveDim("14sp");
    expect(d.value).toBe(14);
    expect(d.unit).toBe("sp");
  });

  it("percent unit", () => {
    const d = resolveDim("100%");
    expect(d.value).toBe(100);
    expect(d.unit).toBe("%");
  });

  it("fractional dp", () => {
    const d = resolveDim("1.5dp");
    expect(d.value).toBe(1.5);
  });
});

// ============================================================================
// 5. Slot Ref Resolution
// ============================================================================

describe("slot ref resolution", () => {
  it("component slot ref gets slotType and isLoopVar=false", () => {
    const { renderer } = run(`
      component X { slot title: text; Text { content: @title; } }
    `);
    const slotRefProp = renderer.resolvedProps.find((p) => p.name === "content")!;
    expect(slotRefProp.value.kind).toBe("slot_ref");
    const rv = slotRefProp.value as { kind: "slot_ref"; slotType: MosaicType; isLoopVar: boolean };
    expect(rv.slotType).toEqual({ kind: "text" });
    expect(rv.isLoopVar).toBe(false);
  });

  it("loop variable ref gets isLoopVar=true", () => {
    const { renderer } = run(`
      component X {
        slot items: list<text>;
        Column {
          each @items as item {
            Text { content: @item; }
          }
        }
      }
    `);
    const contentProp = renderer.resolvedProps.find((p) => p.name === "content")!;
    const rv = contentProp.value as { kind: "slot_ref"; isLoopVar: boolean };
    expect(rv.kind).toBe("slot_ref");
    expect(rv.isLoopVar).toBe(true);
  });
});

// ============================================================================
// 6. Slot References as Children
// ============================================================================

describe("slot references as children", () => {
  it("@slot; child calls renderSlotChild", () => {
    const { renderer } = run(`
      component X { slot header: node; Column { @header; } }
    `);
    expect(methods(renderer)).toContain("renderSlotChild");
    const call = renderer.calls.find((c) => c.method === "renderSlotChild") as
      { method: "renderSlotChild"; slotName: string };
    expect(call.slotName).toBe("header");
  });
});

// ============================================================================
// 7. When Blocks
// ============================================================================

describe("when blocks", () => {
  it("calls beginWhen and endWhen", () => {
    const { renderer } = run(`
      component X {
        slot show: bool;
        Column { when @show { Text {} } }
      }
    `);
    expect(methods(renderer)).toContain("beginWhen");
    expect(methods(renderer)).toContain("endWhen");
  });

  it("beginWhen has correct slotName", () => {
    const { renderer } = run(`
      component X {
        slot show: bool;
        Column { when @show { Text {} } }
      }
    `);
    const call = renderer.calls.find((c) => c.method === "beginWhen") as
      { method: "beginWhen"; slotName: string };
    expect(call.slotName).toBe("show");
  });

  it("children of when block are traversed between beginWhen and endWhen", () => {
    const { renderer } = run(`
      component X {
        slot show: bool;
        Column { when @show { Row {} } }
      }
    `);
    const calls = methods(renderer);
    const whenStart = calls.indexOf("beginWhen");
    const whenEnd = calls.indexOf("endWhen");
    const nodeInBetween = calls.slice(whenStart + 1, whenEnd).some((m) => m === "beginNode");
    expect(nodeInBetween).toBe(true);
  });
});

// ============================================================================
// 8. Each Blocks
// ============================================================================

describe("each blocks", () => {
  it("calls beginEach and endEach", () => {
    const { renderer } = run(`
      component X {
        slot items: list<text>;
        Column { each @items as item { Text {} } }
      }
    `);
    expect(methods(renderer)).toContain("beginEach");
    expect(methods(renderer)).toContain("endEach");
  });

  it("beginEach has correct slotName and itemName", () => {
    const { renderer } = run(`
      component X {
        slot items: list<text>;
        Column { each @items as item { Text {} } }
      }
    `);
    const call = renderer.calls.find((c) => c.method === "beginEach") as
      { method: "beginEach"; slotName: string; itemName: string };
    expect(call.slotName).toBe("items");
    expect(call.itemName).toBe("item");
  });
});

// ============================================================================
// 9. MosaicVMError
// ============================================================================

describe("MosaicVMError", () => {
  it("MosaicVMError has correct name", () => {
    const err = new MosaicVMError("test");
    expect(err.name).toBe("MosaicVMError");
  });

  it("is an instance of Error", () => {
    const err = new MosaicVMError("test");
    expect(err instanceof Error).toBe(true);
  });
});
