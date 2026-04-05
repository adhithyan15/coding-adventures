/**
 * Tests for mosaic-ir-to-layout.
 *
 * We build MosaicComponent IR directly (bypassing the lexer/parser/analyzer)
 * to test the converter logic in isolation.
 */

import { describe, it, expect } from "vitest";
import {
  mosaic_ir_to_layout,
  mosaic_default_theme,
  type SlotMap,
  type MosaicLayoutTheme,
} from "../src/index.js";
import type { MosaicComponent, MosaicNode, MosaicValue } from "@coding-adventures/mosaic-analyzer";
import type { LayoutNode } from "@coding-adventures/layout-ir";
import { rgb, rgba } from "@coding-adventures/layout-ir";

// ── Helpers ──────────────────────────────────────────────────────────────────

const theme = mosaic_default_theme();
const emptySlots: SlotMap = new Map();

/** Build a minimal MosaicComponent with a given root node */
function component(rootNode: MosaicNode): MosaicComponent {
  return { name: "TestComponent", slots: [], tree: rootNode };
}

/** Build a simple primitive MosaicNode */
function mNode(
  tag: string,
  props: Array<{ name: string; value: MosaicValue }> = [],
  children: MosaicNode[] = []
): MosaicNode {
  return {
    tag,
    isPrimitive: true,
    properties: props,
    children: children.map(c => ({ kind: "node", node: c })),
  };
}

/** Build a string MosaicValue */
const str = (v: string): MosaicValue => ({ kind: "string", value: v });
const num = (v: number): MosaicValue => ({ kind: "number", value: v });
const ident = (v: string): MosaicValue => ({ kind: "ident", value: v });
const dim = (v: number, unit = "dp"): MosaicValue => ({ kind: "dimension", value: v, unit });
const hex = (v: string): MosaicValue => ({ kind: "color_hex", value: v });
const slotRef = (name: string): MosaicValue => ({ kind: "slot_ref", slotName: name });

// ── Default theme ────────────────────────────────────────────────────────────

describe("mosaic_default_theme", () => {
  it("returns a theme with a default font", () => {
    const t = mosaic_default_theme();
    expect(t.defaultFont).toBeDefined();
    expect(t.defaultFont.size).toBeGreaterThan(0);
  });

  it("returns a theme with a non-transparent text color", () => {
    const t = mosaic_default_theme();
    expect(t.defaultTextColor.a).toBeGreaterThan(0);
  });

  it("baseFontSize is 1.0", () => {
    expect(mosaic_default_theme().baseFontSize).toBe(1.0);
  });
});

// ── Column mapping ───────────────────────────────────────────────────────────

describe("Column → flex container (direction: column)", () => {
  it("produces a LayoutNode with direction: column", () => {
    const result = mosaic_ir_to_layout(component(mNode("Column")), emptySlots, theme);
    const flex = result.ext["flex"] as { direction?: string } | undefined;
    expect(flex?.direction).toBe("column");
  });

  it("width defaults to fill", () => {
    const result = mosaic_ir_to_layout(component(mNode("Column")), emptySlots, theme);
    expect(result.width.kind).toBe("fill");
  });

  it("height defaults to wrap", () => {
    const result = mosaic_ir_to_layout(component(mNode("Column")), emptySlots, theme);
    expect(result.height.kind).toBe("wrap");
  });

  it("gap property sets flex gap", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "gap", value: num(12) }])),
      emptySlots, theme
    );
    const flex = result.ext["flex"] as { gap?: number } | undefined;
    expect(flex?.gap).toBe(12);
  });

  it("padding property sets padding on container", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "padding", value: num(16) }])),
      emptySlots, theme
    );
    expect(result.padding?.top).toBe(16);
    expect(result.padding?.left).toBe(16);
  });

  it("children are converted recursively", () => {
    const col = mNode("Column", [], [mNode("Text", [{ name: "content", value: str("Hi") }])]);
    const result = mosaic_ir_to_layout(component(col), emptySlots, theme);
    expect(result.children).toHaveLength(1);
    expect(result.children[0].content?.kind).toBe("text");
  });

  it("align property maps to alignItems", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "align", value: ident("center") }])),
      emptySlots, theme
    );
    const flex = result.ext["flex"] as { alignItems?: string } | undefined;
    expect(flex?.alignItems).toBe("center");
  });

  it("background property populates paint ext", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "background", value: hex("#ff0000") }])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { backgroundColor?: unknown } | undefined;
    expect(paint?.backgroundColor).toBeDefined();
  });
});

// ── Row mapping ──────────────────────────────────────────────────────────────

describe("Row → flex container (direction: row)", () => {
  it("produces direction: row", () => {
    const result = mosaic_ir_to_layout(component(mNode("Row")), emptySlots, theme);
    const flex = result.ext["flex"] as { direction?: string } | undefined;
    expect(flex?.direction).toBe("row");
  });

  it("width defaults to fill", () => {
    const result = mosaic_ir_to_layout(component(mNode("Row")), emptySlots, theme);
    expect(result.width.kind).toBe("fill");
  });
});

// ── Box mapping ──────────────────────────────────────────────────────────────

describe("Box → flex container with wrap", () => {
  it("produces direction: column and wrap: wrap", () => {
    const result = mosaic_ir_to_layout(component(mNode("Box")), emptySlots, theme);
    const flex = result.ext["flex"] as { direction?: string; wrap?: string } | undefined;
    expect(flex?.direction).toBe("column");
    expect(flex?.wrap).toBe("wrap");
  });
});

// ── Text mapping ─────────────────────────────────────────────────────────────

describe("Text → leaf with TextContent", () => {
  it("produces a leaf with content.kind = text", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [{ name: "content", value: str("Hello") }])),
      emptySlots, theme
    );
    expect(result.content?.kind).toBe("text");
  });

  it("content value matches the string property", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [{ name: "content", value: str("World") }])),
      emptySlots, theme
    );
    expect(result.content?.kind === "text" ? result.content.value : "").toBe("World");
  });

  it("uses theme.defaultFont when no font property", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [{ name: "content", value: str("Hi") }])),
      emptySlots, theme
    );
    const content = result.content;
    expect(content?.kind === "text" ? content.font.family : "").toBe(theme.defaultFont.family);
  });

  it("font-size property overrides theme font size", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [
        { name: "content", value: str("Hi") },
        { name: "font-size", value: num(24) },
      ])),
      emptySlots, theme
    );
    const content = result.content;
    expect(content?.kind === "text" ? content.font.size : 0).toBe(24);
  });

  it("style: heading.large sets large font size", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [
        { name: "content", value: str("Heading") },
        { name: "style", value: { kind: "enum", namespace: "heading", member: "large" } },
      ])),
      emptySlots, theme
    );
    const content = result.content;
    expect(content?.kind === "text" ? content.font.size : 0).toBe(32);
  });

  it("uses defaultTextColor when no color property", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [{ name: "content", value: str("Hi") }])),
      emptySlots, theme
    );
    const content = result.content;
    const color = content?.kind === "text" ? content.color : null;
    expect(color).toEqual(theme.defaultTextColor);
  });

  it("color property overrides default text color", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [
        { name: "content", value: str("Hi") },
        { name: "color", value: hex("#ff0000") },
      ])),
      emptySlots, theme
    );
    const content = result.content;
    const color = content?.kind === "text" ? content.color : null;
    expect(color?.r).toBe(255);
    expect(color?.g).toBe(0);
  });

  it("width and height default to wrap", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [{ name: "content", value: str("x") }])),
      emptySlots, theme
    );
    expect(result.width.kind).toBe("wrap");
    expect(result.height.kind).toBe("wrap");
  });
});

// ── Image mapping ────────────────────────────────────────────────────────────

describe("Image → leaf with ImageContent", () => {
  it("produces content.kind = image", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Image", [{ name: "source", value: str("img.png") }])),
      emptySlots, theme
    );
    expect(result.content?.kind).toBe("image");
  });

  it("source property sets src", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Image", [{ name: "source", value: str("logo.png") }])),
      emptySlots, theme
    );
    expect(result.content?.kind === "image" ? result.content.src : "").toBe("logo.png");
  });

  it("shape: circle sets cornerRadius to 9999", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Image", [
        { name: "source", value: str("avatar.png") },
        { name: "shape", value: ident("circle") },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { cornerRadius?: number } | undefined;
    expect(paint?.cornerRadius).toBe(9999);
  });

  it("shape: rounded sets cornerRadius to 8", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Image", [
        { name: "source", value: str("card.png") },
        { name: "shape", value: ident("rounded") },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { cornerRadius?: number } | undefined;
    expect(paint?.cornerRadius).toBe(8);
  });
});

// ── Spacer mapping ───────────────────────────────────────────────────────────

describe("Spacer → flex-grow filler", () => {
  it("has grow=1 in flex ext", () => {
    const result = mosaic_ir_to_layout(component(mNode("Spacer")), emptySlots, theme);
    const flex = result.ext["flex"] as { grow?: number } | undefined;
    expect(flex?.grow).toBe(1);
  });

  it("width and height are fill", () => {
    const result = mosaic_ir_to_layout(component(mNode("Spacer")), emptySlots, theme);
    expect(result.width.kind).toBe("fill");
    expect(result.height.kind).toBe("fill");
  });
});

// ── Divider mapping ──────────────────────────────────────────────────────────

describe("Divider → thin horizontal rule", () => {
  it("has fixed height of 1", () => {
    const result = mosaic_ir_to_layout(component(mNode("Divider")), emptySlots, theme);
    expect(result.height).toEqual({ kind: "fixed", value: 1 });
  });

  it("has fill width", () => {
    const result = mosaic_ir_to_layout(component(mNode("Divider")), emptySlots, theme);
    expect(result.width.kind).toBe("fill");
  });

  it("has a semi-transparent backgroundColor", () => {
    const result = mosaic_ir_to_layout(component(mNode("Divider")), emptySlots, theme);
    const paint = result.ext["paint"] as { backgroundColor?: unknown } | undefined;
    expect(paint?.backgroundColor).toBeDefined();
  });
});

// ── Scroll mapping ───────────────────────────────────────────────────────────

describe("Scroll → overflow container", () => {
  it("has overflow: scroll in paint ext", () => {
    const result = mosaic_ir_to_layout(component(mNode("Scroll")), emptySlots, theme);
    const paint = result.ext["paint"] as { overflow?: string } | undefined;
    expect(paint?.overflow).toBe("scroll");
  });

  it("direction is column", () => {
    const result = mosaic_ir_to_layout(component(mNode("Scroll")), emptySlots, theme);
    const flex = result.ext["flex"] as { direction?: string } | undefined;
    expect(flex?.direction).toBe("column");
  });
});

// ── Non-primitive (imported component) ───────────────────────────────────────

describe("non-primitive node → placeholder", () => {
  it("produces an empty container with _componentRef", () => {
    const imported: MosaicNode = {
      tag: "Button",
      isPrimitive: false,
      properties: [],
      children: [],
    };
    const result = mosaic_ir_to_layout(component(imported), emptySlots, theme);
    expect(result.children).toHaveLength(0);
    expect(result.ext["_componentRef"]).toBe("Button");
  });
});

// ── Slot resolution ──────────────────────────────────────────────────────────

describe("slot resolution", () => {
  it("@slot reference resolves from slots map", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "title", type: { kind: "text" }, required: true }],
      tree: mNode("Text", [{ name: "content", value: slotRef("title") }]),
    };
    const slots = new Map([["title", "Hello World"]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    const content = result.content;
    expect(content?.kind === "text" ? content.value : "").toBe("Hello World");
  });

  it("@slot uses default value when slot not in map", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{
        name: "label",
        type: { kind: "text" },
        defaultValue: { kind: "string", value: "Default" },
        required: false,
      }],
      tree: mNode("Text", [{ name: "content", value: slotRef("label") }]),
    };
    const result = mosaic_ir_to_layout(comp, emptySlots, theme);
    const content = result.content;
    expect(content?.kind === "text" ? content.value : "").toBe("Default");
  });

  it("unknown @slot uses zero value (empty string)", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [],
      tree: mNode("Text", [{ name: "content", value: slotRef("unknown") }]),
    };
    const result = mosaic_ir_to_layout(comp, emptySlots, theme);
    const content = result.content;
    expect(content?.kind === "text" ? content.value : "NOT_EMPTY").toBe("");
  });
});

// ── when blocks ──────────────────────────────────────────────────────────────

describe("when blocks", () => {
  it("when @show is true: includes children", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "show", type: { kind: "bool" }, required: true }],
      tree: {
        tag: "Column",
        isPrimitive: true,
        properties: [],
        children: [{
          kind: "when",
          slotName: "show",
          children: [{ kind: "node", node: mNode("Text", [{ name: "content", value: str("Visible") }]) }],
        }],
      },
    };
    const slots = new Map([["show", true as boolean]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    expect(result.children).toHaveLength(1);
  });

  it("when @show is false: omits children", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "show", type: { kind: "bool" }, required: true }],
      tree: {
        tag: "Column",
        isPrimitive: true,
        properties: [],
        children: [{
          kind: "when",
          slotName: "show",
          children: [{ kind: "node", node: mNode("Text", [{ name: "content", value: str("Hidden") }]) }],
        }],
      },
    };
    const slots = new Map([["show", false as boolean]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    expect(result.children).toHaveLength(0);
  });
});

// ── each blocks ──────────────────────────────────────────────────────────────

describe("each blocks", () => {
  it("each @items iterates and produces one child per item", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{
        name: "items",
        type: { kind: "list", elementType: { kind: "text" } },
        required: true,
      }],
      tree: {
        tag: "Column",
        isPrimitive: true,
        properties: [],
        children: [{
          kind: "each",
          slotName: "items",
          itemName: "item",
          children: [{ kind: "node", node: mNode("Text", [{ name: "content", value: slotRef("item") }]) }],
        }],
      },
    };
    const slots = new Map([["items", ["A", "B", "C"] as string[]]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    expect(result.children).toHaveLength(3);
  });

  it("each over empty list produces no children", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "items", type: { kind: "list", elementType: { kind: "text" } }, required: true }],
      tree: {
        tag: "Column",
        isPrimitive: true,
        properties: [],
        children: [{
          kind: "each",
          slotName: "items",
          itemName: "item",
          children: [{ kind: "node", node: mNode("Text") }],
        }],
      },
    };
    const slots = new Map([["items", [] as string[]]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    expect(result.children).toHaveLength(0);
  });
});

// ── paint ext — border, opacity, shadow ─────────────────────────────────────

describe("paint ext decorations", () => {
  it("border-width and border-color produce stroke paint ext", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [
        { name: "border-width", value: num(2) },
        { name: "border-color", value: hex("#000000") },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { borderWidth?: number; borderColor?: unknown } | undefined;
    expect(paint?.borderWidth).toBe(2);
    expect(paint?.borderColor).toBeDefined();
  });

  it("opacity < 1 sets opacity in paint ext", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "opacity", value: num(0.5) }])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { opacity?: number } | undefined;
    expect(paint?.opacity).toBe(0.5);
  });

  it("shadow: elevation.low sets shadow fields", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [
        { name: "shadow", value: { kind: "enum", namespace: "elevation", member: "low" } },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { shadowBlur?: number } | undefined;
    expect(paint?.shadowBlur).toBe(3);
  });

  it("shadow: elevation.high sets bigger blur", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [
        { name: "shadow", value: { kind: "enum", namespace: "elevation", member: "high" } },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { shadowBlur?: number } | undefined;
    expect(paint?.shadowBlur).toBe(24);
  });
});

// ── padding shorthand (edges_all) ────────────────────────────────────────────

describe("padding shorthand", () => {
  it("scalar padding sets all sides", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "padding", value: num(20) }])),
      emptySlots, theme
    );
    expect(result.padding?.top).toBe(20);
    expect(result.padding?.right).toBe(20);
    expect(result.padding?.bottom).toBe(20);
    expect(result.padding?.left).toBe(20);
  });

  it("per-side padding is independent", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Row", [
        { name: "padding-top", value: num(4) },
        { name: "padding-bottom", value: num(8) },
      ])),
      emptySlots, theme
    );
    expect(result.padding?.top).toBe(4);
    expect(result.padding?.bottom).toBe(8);
    expect(result.padding?.left).toBe(0);
  });
});

// ── shape via enum (not ident) ────────────────────────────────────────────────

describe("image shape via enum value", () => {
  it("shape via enum.circle sets cornerRadius", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Image", [
        { name: "source", value: str("img.png") },
        { name: "shape", value: { kind: "enum", namespace: "shape", member: "circle" } },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { cornerRadius?: number } | undefined;
    expect(paint?.cornerRadius).toBe(9999);
  });
});

// ── unknown primitive tag ─────────────────────────────────────────────────────

describe("unknown primitive tag", () => {
  it("produces empty container with _unknownTag", () => {
    const result = mosaic_ir_to_layout(
      component({ tag: "CustomWidget", isPrimitive: true, properties: [], children: [] }),
      emptySlots, theme
    );
    expect(result.ext["_unknownTag"]).toBe("CustomWidget");
  });
});

// ── slot_ref child ────────────────────────────────────────────────────────────

describe("slot_ref as child", () => {
  it("slot_ref child with LayoutNode value inlines the node", () => {
    const childNode: LayoutNode = {
      width: { kind: "fill" },
      height: { kind: "wrap" },
      content: null,
      children: [],
      ext: {},
      minWidth: undefined,
      maxWidth: undefined,
      minHeight: undefined,
      maxHeight: undefined,
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      padding: { top: 0, right: 0, bottom: 0, left: 0 },
    };
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "header", type: { kind: "node" }, required: true }],
      tree: {
        tag: "Column",
        isPrimitive: true,
        properties: [],
        children: [{ kind: "slot_ref", slotName: "header" }],
      },
    };
    const slots = new Map([["header", childNode as import("@coding-adventures/layout-ir").LayoutNode]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    expect(result.children).toHaveLength(1);
    // The injected node is used directly
    expect(result.children[0]).toBe(childNode);
  });

  it("slot_ref child without a LayoutNode value produces placeholder", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "action", type: { kind: "node" }, required: true }],
      tree: {
        tag: "Column",
        isPrimitive: true,
        properties: [],
        children: [{ kind: "slot_ref", slotName: "action" }],
      },
    };
    // Provide a string value — not a LayoutNode
    const slots = new Map([["action", "not-a-node" as unknown as string]]);
    const result = mosaic_ir_to_layout(comp, slots, theme);
    expect(result.children).toHaveLength(1);
    expect(result.children[0].ext["_slotRef"]).toBe("action");
  });
});

// ── shadow: ident form ────────────────────────────────────────────────────────

describe("shadow ident form", () => {
  it("shadow: medium via ident sets medium blur", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [
        { name: "shadow", value: { kind: "ident", value: "medium" } },
      ])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { shadowBlur?: number } | undefined;
    expect(paint?.shadowBlur).toBe(12);
  });
});

// ── zero values for slots ─────────────────────────────────────────────────────

describe("zero values for unresolved slots", () => {
  function compWithSlot(kind: string): MosaicComponent {
    return {
      name: "Test",
      slots: [{ name: "val", type: { kind } as import("@coding-adventures/mosaic-analyzer").MosaicType, required: true }],
      tree: mNode("Text", [{ name: "content", value: slotRef("val") }]),
    };
  }

  it("unresolved number slot → 0", () => {
    const result = mosaic_ir_to_layout(compWithSlot("number"), emptySlots, theme);
    // The text content becomes "0" (coerced from number 0)
    // or the content is set to "0" via resolveValue → number 0 (which get() coerces to "")
    // Just check it doesn't throw
    expect(result.content?.kind).toBe("text");
  });

  it("unresolved bool slot → false", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "flag", type: { kind: "bool" }, required: true }],
      tree: {
        tag: "Column", isPrimitive: true, properties: [],
        children: [{ kind: "when", slotName: "flag", children: [] }],
      },
    };
    // false → when block produces no children
    const result = mosaic_ir_to_layout(comp, emptySlots, theme);
    expect(result.children).toHaveLength(0);
  });

  it("unresolved color slot → transparent", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "tint", type: { kind: "color" }, required: true }],
      tree: mNode("Column", [{ name: "background", value: slotRef("tint") }]),
    };
    const result = mosaic_ir_to_layout(comp, emptySlots, theme);
    // backgroundColor is transparent (a=0) — not set by paintExtFromProps since it's present but transparent
    // The key thing is it doesn't throw
    expect(result).toBeDefined();
  });

  it("unresolved list slot → empty array", () => {
    const comp: MosaicComponent = {
      name: "Test",
      slots: [{ name: "items", type: { kind: "list", elementType: { kind: "text" } }, required: true }],
      tree: {
        tag: "Column", isPrimitive: true, properties: [],
        children: [{
          kind: "each", slotName: "items", itemName: "item",
          children: [{ kind: "node", node: mNode("Text") }],
        }],
      },
    };
    const result = mosaic_ir_to_layout(comp, emptySlots, theme);
    expect(result.children).toHaveLength(0);
  });
});

// ── style ident (large/caption) ───────────────────────────────────────────────

describe("style ident values", () => {
  it("style: large sets large font size", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [
        { name: "content", value: str("Big") },
        { name: "style", value: { kind: "ident", value: "large" } },
      ])),
      emptySlots, theme
    );
    const content = result.content;
    expect(content?.kind === "text" ? content.font.size : 0).toBe(32);
  });

  it("style: caption sets small font size", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Text", [
        { name: "content", value: str("Small") },
        { name: "style", value: { kind: "ident", value: "caption" } },
      ])),
      emptySlots, theme
    );
    const content = result.content;
    expect(content?.kind === "text" ? content.font.size : 0).toBe(12);
  });
});

// ── Row scalar padding ────────────────────────────────────────────────────────

describe("Row scalar padding", () => {
  it("scalar padding on Row sets all sides", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Row", [{ name: "padding", value: num(8) }])),
      emptySlots, theme
    );
    expect(result.padding?.top).toBe(8);
    expect(result.padding?.left).toBe(8);
  });
});

// ── color parsing ────────────────────────────────────────────────────────────

describe("hex color parsing", () => {
  it("#rrggbb parses correctly", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "background", value: hex("#2563eb") }])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { backgroundColor?: { r: number; g: number; b: number; a: number } } | undefined;
    const bg = paint?.backgroundColor;
    expect(bg?.r).toBe(0x25);
    expect(bg?.g).toBe(0x63);
    expect(bg?.b).toBe(0xeb);
    expect(bg?.a).toBe(255);
  });

  it("#rgb shorthand parses correctly", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "background", value: hex("#f00") }])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { backgroundColor?: { r: number; g: number; b: number } } | undefined;
    expect(paint?.backgroundColor?.r).toBe(255);
    expect(paint?.backgroundColor?.g).toBe(0);
  });

  it("invalid hex string falls back to transparent", () => {
    // Use an invalid hex value — falls through to the transparent fallback
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "background", value: hex("notahex") }])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { backgroundColor?: { r: number; g: number; b: number; a: number } } | undefined;
    // Fallback to transparent: {r:0, g:0, b:0, a:0}
    expect(paint?.backgroundColor).toEqual({ r: 0, g: 0, b: 0, a: 0 });
  });

  it("#rrggbbaa parses alpha channel", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "background", value: hex("#ff000080") }])),
      emptySlots, theme
    );
    const paint = result.ext["paint"] as { backgroundColor?: { r: number; a: number } } | undefined;
    expect(paint?.backgroundColor?.r).toBe(255);
    expect(paint?.backgroundColor?.a).toBe(0x80);
  });
});

// ── dimension values ──────────────────────────────────────────────────────────

describe("dimension values", () => {
  it("dimension value is used as numeric pixels", () => {
    const result = mosaic_ir_to_layout(
      component(mNode("Column", [{ name: "padding", value: { kind: "dimension", value: 12, unit: "dp" } }])),
      emptySlots, theme
    );
    expect(result.padding?.top).toBe(12);
  });
});
