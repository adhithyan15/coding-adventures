import { describe, it, expect } from "vitest";
import {
  size_fixed,
  size_fill,
  size_wrap,
  edges_all,
  edges_xy,
  edges_zero,
  rgba,
  rgb,
  color_transparent,
  font_spec,
  font_bold,
  font_italic,
  constraints_fixed,
  constraints_width,
  constraints_unconstrained,
  constraints_shrink,
  node,
  leaf_text,
  leaf_image,
  container,
  positioned,
} from "../src/index.js";

import type { TextContent, ImageContent } from "../src/index.js";

// ============================================================================
// SizeValue builders
// ============================================================================

describe("size_fixed", () => {
  it("produces a fixed kind with the given value", () => {
    const s = size_fixed(200);
    expect(s.kind).toBe("fixed");
    if (s.kind === "fixed") expect(s.value).toBe(200);
  });

  it("accepts zero", () => {
    const s = size_fixed(0);
    expect(s.kind).toBe("fixed");
    if (s.kind === "fixed") expect(s.value).toBe(0);
  });

  it("accepts fractional values", () => {
    const s = size_fixed(16.5);
    expect(s.kind).toBe("fixed");
    if (s.kind === "fixed") expect(s.value).toBe(16.5);
  });
});

describe("size_fill", () => {
  it("produces a fill kind", () => {
    expect(size_fill()).toEqual({ kind: "fill" });
  });

  it("returns a new object each call (not shared reference)", () => {
    expect(size_fill()).not.toBe(size_fill());
  });
});

describe("size_wrap", () => {
  it("produces a wrap kind", () => {
    expect(size_wrap()).toEqual({ kind: "wrap" });
  });
});

// ============================================================================
// Edges builders
// ============================================================================

describe("edges_all", () => {
  it("sets all four sides to the same value", () => {
    const e = edges_all(12);
    expect(e).toEqual({ top: 12, right: 12, bottom: 12, left: 12 });
  });

  it("works with zero", () => {
    expect(edges_all(0)).toEqual({ top: 0, right: 0, bottom: 0, left: 0 });
  });
});

describe("edges_xy", () => {
  it("sets x on left/right and y on top/bottom", () => {
    const e = edges_xy(16, 8);
    expect(e).toEqual({ top: 8, right: 16, bottom: 8, left: 16 });
  });

  it("mirrors CSS padding: y x ordering", () => {
    // CSS: padding: 4px 20px → top/bottom=4, left/right=20
    const e = edges_xy(20, 4);
    expect(e.top).toBe(4);
    expect(e.bottom).toBe(4);
    expect(e.left).toBe(20);
    expect(e.right).toBe(20);
  });
});

describe("edges_zero", () => {
  it("returns all zeros", () => {
    expect(edges_zero()).toEqual({ top: 0, right: 0, bottom: 0, left: 0 });
  });
});

// ============================================================================
// Color builders
// ============================================================================

describe("rgba", () => {
  it("stores all four components", () => {
    const c = rgba(255, 128, 0, 200);
    expect(c).toEqual({ r: 255, g: 128, b: 0, a: 200 });
  });

  it("accepts black fully transparent", () => {
    expect(rgba(0, 0, 0, 0)).toEqual({ r: 0, g: 0, b: 0, a: 0 });
  });
});

describe("rgb", () => {
  it("sets alpha to 255", () => {
    const c = rgb(100, 150, 200);
    expect(c).toEqual({ r: 100, g: 150, b: 200, a: 255 });
  });

  it("white is rgb(255,255,255)", () => {
    expect(rgb(255, 255, 255)).toEqual({ r: 255, g: 255, b: 255, a: 255 });
  });
});

describe("color_transparent", () => {
  it("returns rgba(0,0,0,0)", () => {
    expect(color_transparent()).toEqual({ r: 0, g: 0, b: 0, a: 0 });
  });
});

// ============================================================================
// FontSpec builders
// ============================================================================

describe("font_spec", () => {
  it("sets family and size with defaults", () => {
    const f = font_spec("Arial", 16);
    expect(f.family).toBe("Arial");
    expect(f.size).toBe(16);
    expect(f.weight).toBe(400);
    expect(f.italic).toBe(false);
    expect(f.lineHeight).toBe(1.2);
  });

  it("accepts empty family string (system default)", () => {
    const f = font_spec("", 14);
    expect(f.family).toBe("");
  });
});

describe("font_bold", () => {
  it("sets weight to 700 without mutating original", () => {
    const orig = font_spec("Helvetica", 12);
    const bold = font_bold(orig);
    expect(bold.weight).toBe(700);
    expect(orig.weight).toBe(400); // original unchanged
  });

  it("preserves all other fields", () => {
    const orig = font_spec("Times New Roman", 18);
    const bold = font_bold(orig);
    expect(bold.family).toBe("Times New Roman");
    expect(bold.size).toBe(18);
    expect(bold.italic).toBe(false);
    expect(bold.lineHeight).toBe(1.2);
  });
});

describe("font_italic", () => {
  it("sets italic to true without mutating original", () => {
    const orig = font_spec("Arial", 14);
    const italic = font_italic(orig);
    expect(italic.italic).toBe(true);
    expect(orig.italic).toBe(false);
  });

  it("can be chained with font_bold", () => {
    const boldItalic = font_italic(font_bold(font_spec("Arial", 14)));
    expect(boldItalic.weight).toBe(700);
    expect(boldItalic.italic).toBe(true);
  });

  it("preserves family and size", () => {
    const orig = font_spec("Courier", 20);
    const italic = font_italic(orig);
    expect(italic.family).toBe("Courier");
    expect(italic.size).toBe(20);
  });
});

// ============================================================================
// Constraints builders
// ============================================================================

describe("constraints_fixed", () => {
  it("creates fixed width and height", () => {
    const c = constraints_fixed(800, 600);
    expect(c).toEqual({ minWidth: 0, maxWidth: 800, minHeight: 0, maxHeight: 600 });
  });

  it("accepts non-integer values", () => {
    const c = constraints_fixed(375.5, 812.5);
    expect(c.maxWidth).toBe(375.5);
    expect(c.maxHeight).toBe(812.5);
  });
});

describe("constraints_width", () => {
  it("fixes width, leaves height unconstrained", () => {
    const c = constraints_width(400);
    expect(c.maxWidth).toBe(400);
    expect(c.maxHeight).toBe(Infinity);
    expect(c.minHeight).toBe(0);
  });
});

describe("constraints_unconstrained", () => {
  it("returns all Infinity max constraints", () => {
    const c = constraints_unconstrained();
    expect(c.maxWidth).toBe(Infinity);
    expect(c.maxHeight).toBe(Infinity);
    expect(c.minWidth).toBe(0);
    expect(c.minHeight).toBe(0);
  });
});

describe("constraints_shrink", () => {
  it("reduces max by dw and dh", () => {
    const c = constraints_fixed(800, 600);
    const shrunk = constraints_shrink(c, 32, 16);
    expect(shrunk.maxWidth).toBe(768);
    expect(shrunk.maxHeight).toBe(584);
  });

  it("clamps to zero when shrink exceeds available", () => {
    const c = constraints_fixed(10, 10);
    const shrunk = constraints_shrink(c, 100, 100);
    expect(shrunk.maxWidth).toBe(0);
    expect(shrunk.maxHeight).toBe(0);
  });

  it("also shrinks minWidth and minHeight", () => {
    const c = { minWidth: 50, maxWidth: 200, minHeight: 30, maxHeight: 100 };
    const shrunk = constraints_shrink(c, 20, 10);
    expect(shrunk.minWidth).toBe(30);
    expect(shrunk.minHeight).toBe(20);
  });

  it("clamps minWidth to zero", () => {
    const c = { minWidth: 5, maxWidth: 100, minHeight: 0, maxHeight: 100 };
    const shrunk = constraints_shrink(c, 50, 0);
    expect(shrunk.minWidth).toBe(0);
  });
});

// ============================================================================
// LayoutNode builders
// ============================================================================

const sampleText: TextContent = {
  kind: "text",
  value: "Hello world",
  font: font_spec("Arial", 16),
  color: rgb(0, 0, 0),
  maxLines: null,
  textAlign: "start",
};

const sampleImage: ImageContent = {
  kind: "image",
  src: "https://example.com/img.png",
  fit: "contain",
};

describe("node", () => {
  it("creates a node with all defaults", () => {
    const n = node({});
    expect(n.content).toBeNull();
    expect(n.children).toEqual([]);
    expect(n.width).toBeNull();
    expect(n.height).toBeNull();
    expect(n.padding).toBeNull();
    expect(n.margin).toBeNull();
    expect(n.ext).toEqual({});
  });

  it("accepts id", () => {
    const n = node({ id: "root" });
    expect(n.id).toBe("root");
  });

  it("accepts ext", () => {
    const n = node({ ext: { flex: { direction: "row" } } });
    expect(n.ext["flex"]).toEqual({ direction: "row" });
  });
});

describe("leaf_text", () => {
  it("creates a leaf with text content", () => {
    const n = leaf_text(sampleText);
    expect(n.content).toEqual(sampleText);
    expect(n.children).toEqual([]);
  });

  it("defaults to wrap size", () => {
    const n = leaf_text(sampleText);
    expect(n.width).toEqual({ kind: "wrap" });
    expect(n.height).toEqual({ kind: "wrap" });
  });

  it("opts can override width", () => {
    const n = leaf_text(sampleText, { width: size_fixed(100) });
    expect(n.width).toEqual({ kind: "fixed", value: 100 });
    expect(n.height).toEqual({ kind: "wrap" }); // height still wrap
  });

  it("opts can add ext", () => {
    const n = leaf_text(sampleText, { ext: { block: { display: "inline" } } });
    expect(n.ext["block"]).toEqual({ display: "inline" });
  });
});

describe("leaf_image", () => {
  it("creates a leaf with image content", () => {
    const n = leaf_image(sampleImage);
    expect(n.content).toEqual(sampleImage);
    expect(n.children).toEqual([]);
  });

  it("defaults to wrap size", () => {
    const n = leaf_image(sampleImage);
    expect(n.width).toEqual({ kind: "wrap" });
    expect(n.height).toEqual({ kind: "wrap" });
  });

  it("opts can set fixed size", () => {
    const n = leaf_image(sampleImage, {
      width: size_fixed(200),
      height: size_fixed(200),
    });
    expect(n.width).toEqual({ kind: "fixed", value: 200 });
    expect(n.height).toEqual({ kind: "fixed", value: 200 });
  });
});

describe("container", () => {
  it("creates a container with children", () => {
    const child = leaf_text(sampleText);
    const c = container([child]);
    expect(c.content).toBeNull();
    expect(c.children).toHaveLength(1);
    expect(c.children[0]).toBe(child);
  });

  it("defaults to null width and height", () => {
    const c = container([]);
    expect(c.width).toBeNull();
    expect(c.height).toBeNull();
  });

  it("can have fill width and wrap height", () => {
    const c = container([], { width: size_fill(), height: size_wrap() });
    expect(c.width).toEqual({ kind: "fill" });
    expect(c.height).toEqual({ kind: "wrap" });
  });

  it("accepts padding and margin", () => {
    const c = container([], {
      padding: edges_all(16),
      margin: edges_xy(0, 8),
    });
    expect(c.padding).toEqual({ top: 16, right: 16, bottom: 16, left: 16 });
    expect(c.margin).toEqual({ top: 8, right: 0, bottom: 8, left: 0 });
  });
});

// ============================================================================
// positioned builder
// ============================================================================

describe("positioned", () => {
  it("creates a positioned node with resolved geometry", () => {
    const p = positioned(10, 20, 100, 50, {});
    expect(p.x).toBe(10);
    expect(p.y).toBe(20);
    expect(p.width).toBe(100);
    expect(p.height).toBe(50);
  });

  it("defaults content to null and children to []", () => {
    const p = positioned(0, 0, 80, 40, {});
    expect(p.content).toBeNull();
    expect(p.children).toEqual([]);
  });

  it("accepts content", () => {
    const p = positioned(0, 0, 80, 20, { content: sampleText });
    expect(p.content).toEqual(sampleText);
  });

  it("accepts children", () => {
    const child = positioned(0, 0, 50, 20, {});
    const parent = positioned(0, 0, 100, 40, { children: [child] });
    expect(parent.children).toHaveLength(1);
    expect(parent.children[0]).toBe(child);
  });

  it("accepts ext", () => {
    const p = positioned(0, 0, 100, 50, { ext: { paint: { backgroundColor: rgb(255, 0, 0) } } });
    expect(p.ext["paint"]).toBeDefined();
  });
});

// ============================================================================
// Discriminated union type guards
// ============================================================================

describe("SizeValue discriminant", () => {
  it("fixed can be narrowed by kind", () => {
    const s = size_fixed(42);
    if (s.kind === "fixed") {
      // TypeScript narrowing: value exists here
      expect(s.value).toBe(42);
    } else {
      throw new Error("should be fixed");
    }
  });

  it("fill and wrap have no value field", () => {
    const fill = size_fill();
    const wrap = size_wrap();
    expect("value" in fill).toBe(false);
    expect("value" in wrap).toBe(false);
  });
});

describe("NodeContent discriminant", () => {
  it("text content has kind 'text'", () => {
    expect(sampleText.kind).toBe("text");
  });

  it("image content has kind 'image'", () => {
    expect(sampleImage.kind).toBe("image");
  });
});

// ============================================================================
// Immutability of builders
// ============================================================================

describe("builders return new objects", () => {
  it("font_bold does not mutate original", () => {
    const orig = font_spec("Arial", 14);
    font_bold(orig);
    expect(orig.weight).toBe(400);
  });

  it("font_italic does not mutate original", () => {
    const orig = font_spec("Arial", 14);
    font_italic(orig);
    expect(orig.italic).toBe(false);
  });

  it("constraints_shrink does not mutate original", () => {
    const orig = constraints_fixed(200, 200);
    constraints_shrink(orig, 50, 50);
    expect(orig.maxWidth).toBe(200);
  });
});
