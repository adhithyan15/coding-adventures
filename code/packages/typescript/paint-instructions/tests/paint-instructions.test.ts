import { describe, expect, it } from "vitest";
import {
  VERSION,
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
  type PaintInstruction,
  type PaintScene,
  type PaintRect,
  type PaintEllipse,
  type PaintPath,
  type PaintGroup,
  type PaintLayer,
  type PaintLine,
  type PaintClip,
  type PaintGradient,
  type PaintImage,
  type PixelContainer,
  type ImageCodec,
  type PathCommand,
  type Transform2D,
  type FilterEffect,
  type BlendMode,
} from "../src/index.js";

// ============================================================================
// VERSION
// ============================================================================

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ============================================================================
// paintRect builder
// ============================================================================

describe("paintRect", () => {
  it("creates a minimal rect with only required fields", () => {
    const r = paintRect(10, 20, 100, 50);
    expect(r.kind).toBe("rect");
    expect(r.x).toBe(10);
    expect(r.y).toBe(20);
    expect(r.width).toBe(100);
    expect(r.height).toBe(50);
    expect(r.fill).toBeUndefined();
    expect(r.stroke).toBeUndefined();
  });

  it("creates a rect with fill and stroke", () => {
    const r = paintRect(0, 0, 200, 100, {
      fill: "#2563eb",
      stroke: "#ffffff",
      stroke_width: 2,
    });
    expect(r.fill).toBe("#2563eb");
    expect(r.stroke).toBe("#ffffff");
    expect(r.stroke_width).toBe(2);
  });

  it("creates a rect with corner radius", () => {
    const r = paintRect(0, 0, 100, 50, { corner_radius: 8 });
    expect(r.corner_radius).toBe(8);
  });

  it("creates a rect with an id for patch() tracking", () => {
    const r = paintRect(0, 0, 50, 50, { id: "my-rect" });
    expect(r.id).toBe("my-rect");
  });

  it("creates a rect with metadata", () => {
    const r = paintRect(0, 0, 50, 50, {
      metadata: { source: "chart-bar-3", layer: "foreground" },
    });
    expect(r.metadata).toEqual({ source: "chart-bar-3", layer: "foreground" });
  });

  it("kind discriminant is always 'rect'", () => {
    const r = paintRect(0, 0, 1, 1);
    // TypeScript narrows the union via the kind field
    const instr: PaintInstruction = r;
    if (instr.kind === "rect") {
      expect(instr.x).toBe(0);
    }
  });
});

// ============================================================================
// paintEllipse builder
// ============================================================================

describe("paintEllipse", () => {
  it("creates an ellipse with center and radii", () => {
    const e = paintEllipse(100, 80, 60, 40);
    expect(e.kind).toBe("ellipse");
    expect(e.cx).toBe(100);
    expect(e.cy).toBe(80);
    expect(e.rx).toBe(60);
    expect(e.ry).toBe(40);
  });

  it("creates a circle by setting rx === ry", () => {
    const c = paintEllipse(50, 50, 30, 30, { fill: "#ef4444" });
    expect(c.rx).toBe(c.ry);
    expect(c.fill).toBe("#ef4444");
  });

  it("supports stroke options", () => {
    const e = paintEllipse(0, 0, 50, 25, { stroke: "#333", stroke_width: 1.5 });
    expect(e.stroke).toBe("#333");
    expect(e.stroke_width).toBe(1.5);
  });
});

// ============================================================================
// paintPath builder
// ============================================================================

describe("paintPath", () => {
  it("creates a path with move_to and line_to", () => {
    const commands: PathCommand[] = [
      { kind: "move_to", x: 0, y: 0 },
      { kind: "line_to", x: 100, y: 0 },
      { kind: "line_to", x: 50, y: 86 },
      { kind: "close" },
    ];
    const p = paintPath(commands, { fill: "#ef4444" });
    expect(p.kind).toBe("path");
    expect(p.commands).toHaveLength(4);
    expect(p.commands[0]).toEqual({ kind: "move_to", x: 0, y: 0 });
    expect(p.commands[3]).toEqual({ kind: "close" });
    expect(p.fill).toBe("#ef4444");
  });

  it("creates a path with cubic bezier", () => {
    const commands: PathCommand[] = [
      { kind: "move_to", x: 0, y: 50 },
      { kind: "cubic_to", cx1: 25, cy1: 0, cx2: 75, cy2: 100, x: 100, y: 50 },
    ];
    const p = paintPath(commands);
    expect(p.commands[1].kind).toBe("cubic_to");
  });

  it("supports fill_rule, stroke_cap, stroke_join", () => {
    const p = paintPath([], {
      fill_rule: "evenodd",
      stroke: "#000",
      stroke_cap: "round",
      stroke_join: "bevel",
    });
    expect(p.fill_rule).toBe("evenodd");
    expect(p.stroke_cap).toBe("round");
    expect(p.stroke_join).toBe("bevel");
  });
});

// ============================================================================
// paintLine builder
// ============================================================================

describe("paintLine", () => {
  it("creates a line between two points", () => {
    const l = paintLine(0, 50, 200, 50, "#9ca3af");
    expect(l.kind).toBe("line");
    expect(l.x1).toBe(0);
    expect(l.y1).toBe(50);
    expect(l.x2).toBe(200);
    expect(l.y2).toBe(50);
    expect(l.stroke).toBe("#9ca3af");
  });

  it("supports stroke_width and stroke_cap", () => {
    const l = paintLine(0, 0, 100, 100, "#333", {
      stroke_width: 3,
      stroke_cap: "round",
    });
    expect(l.stroke_width).toBe(3);
    expect(l.stroke_cap).toBe("round");
  });
});

// ============================================================================
// paintGroup builder
// ============================================================================

describe("paintGroup", () => {
  it("creates a group with children", () => {
    const children = [paintRect(0, 0, 50, 50)];
    const g = paintGroup(children);
    expect(g.kind).toBe("group");
    expect(g.children).toHaveLength(1);
    expect(g.children[0].kind).toBe("rect");
  });

  it("applies a translation transform", () => {
    const transform: Transform2D = [1, 0, 0, 1, 100, 50]; // translate(100, 50)
    const g = paintGroup([], { transform });
    expect(g.transform).toEqual([1, 0, 0, 1, 100, 50]);
  });

  it("applies opacity", () => {
    const g = paintGroup([paintRect(0, 0, 100, 100)], { opacity: 0.5 });
    expect(g.opacity).toBe(0.5);
  });

  it("supports nested groups", () => {
    const inner = paintGroup([paintRect(0, 0, 10, 10)]);
    const outer = paintGroup([inner]);
    expect(outer.children[0].kind).toBe("group");
    if (outer.children[0].kind === "group") {
      expect(outer.children[0].children[0].kind).toBe("rect");
    }
  });
});

// ============================================================================
// paintLayer builder
// ============================================================================

describe("paintLayer", () => {
  it("creates a layer with children", () => {
    const l = paintLayer([paintEllipse(50, 50, 30, 30, { fill: "#3b82f6" })]);
    expect(l.kind).toBe("layer");
    expect(l.children).toHaveLength(1);
  });

  it("applies a blur filter", () => {
    const l = paintLayer([paintRect(0, 0, 100, 100)], {
      filters: [{ kind: "blur", radius: 10 }],
    });
    expect(l.filters).toHaveLength(1);
    expect(l.filters![0]).toEqual({ kind: "blur", radius: 10 });
  });

  it("applies a drop_shadow filter", () => {
    const filter: FilterEffect = {
      kind: "drop_shadow",
      dx: 4,
      dy: 4,
      blur: 8,
      color: "rgba(0,0,0,0.4)",
    };
    const l = paintLayer([], { filters: [filter] });
    expect(l.filters![0].kind).toBe("drop_shadow");
  });

  it("applies multiple filters in order", () => {
    const l = paintLayer([], {
      filters: [
        { kind: "blur", radius: 5 },
        { kind: "brightness", amount: 1.2 },
        { kind: "saturate", amount: 1.5 },
      ],
    });
    expect(l.filters).toHaveLength(3);
    expect(l.filters![0].kind).toBe("blur");
    expect(l.filters![1].kind).toBe("brightness");
    expect(l.filters![2].kind).toBe("saturate");
  });

  it("sets blend mode", () => {
    const l = paintLayer([], { blend_mode: "multiply" });
    expect(l.blend_mode).toBe("multiply");
  });

  it("all BlendMode values are valid strings", () => {
    const modes: BlendMode[] = [
      "normal", "multiply", "screen", "overlay", "darken", "lighten",
      "color_dodge", "color_burn", "hard_light", "soft_light",
      "difference", "exclusion", "hue", "saturation", "color", "luminosity",
    ];
    for (const mode of modes) {
      const l = paintLayer([], { blend_mode: mode });
      expect(l.blend_mode).toBe(mode);
    }
  });

  it("applies opacity (separate from filter opacity)", () => {
    const l = paintLayer([], { opacity: 0.7 });
    expect(l.opacity).toBe(0.7);
  });

  it("applies a color_matrix filter with 20 values", () => {
    // Identity matrix: [1,0,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0]
    const identity = [1,0,0,0,0, 0,1,0,0,0, 0,0,1,0,0, 0,0,0,1,0];
    const l = paintLayer([], {
      filters: [{ kind: "color_matrix", matrix: identity }],
    });
    const f = l.filters![0];
    expect(f.kind).toBe("color_matrix");
    if (f.kind === "color_matrix") {
      expect(f.matrix).toHaveLength(20);
    }
  });
});

// ============================================================================
// paintClip builder
// ============================================================================

describe("paintClip", () => {
  it("creates a clip with a rect boundary and children", () => {
    const c = paintClip(0, 0, 400, 300, [
      paintRect(-50, -50, 600, 500, { fill: "#e0f2fe" }),
    ]);
    expect(c.kind).toBe("clip");
    expect(c.x).toBe(0);
    expect(c.y).toBe(0);
    expect(c.width).toBe(400);
    expect(c.height).toBe(300);
    expect(c.children).toHaveLength(1);
  });
});

// ============================================================================
// paintGradient builder
// ============================================================================

describe("paintGradient", () => {
  it("creates a linear gradient", () => {
    const g = paintGradient(
      "linear",
      [
        { offset: 0, color: "#3b82f6" },
        { offset: 1, color: "#8b5cf6" },
      ],
      { id: "blue-purple", x1: 0, y1: 0, x2: 400, y2: 0 },
    );
    expect(g.kind).toBe("gradient");
    expect(g.gradient_kind).toBe("linear");
    expect(g.stops).toHaveLength(2);
    expect(g.stops[0]).toEqual({ offset: 0, color: "#3b82f6" });
    expect(g.id).toBe("blue-purple");
    expect(g.x2).toBe(400);
  });

  it("creates a radial gradient", () => {
    const g = paintGradient(
      "radial",
      [
        { offset: 0, color: "#ffffff" },
        { offset: 1, color: "#3b82f6" },
      ],
      { cx: 200, cy: 150, r: 100 },
    );
    expect(g.gradient_kind).toBe("radial");
    expect(g.cx).toBe(200);
    expect(g.r).toBe(100);
  });
});

// ============================================================================
// paintImage builder
// ============================================================================

describe("paintImage", () => {
  it("creates an image from a URI string", () => {
    const img = paintImage(50, 50, 300, 200, "file:///assets/logo.png");
    expect(img.kind).toBe("image");
    expect(img.x).toBe(50);
    expect(img.src).toBe("file:///assets/logo.png");
  });

  it("creates an image from a data URL", () => {
    const img = paintImage(0, 0, 100, 100, "data:image/png;base64,iVBOR");
    expect(typeof img.src).toBe("string");
  });

  it("creates an image from a PixelContainer (zero-copy path)", () => {
    const pixels: PixelContainer = {
      width: 100,
      height: 100,
      data: new Uint8Array(100 * 100 * 4),
    };
    const img = paintImage(0, 0, 100, 100, pixels);
    expect(typeof img.src).toBe("object");
    if (typeof img.src === "object") {
      expect(img.src.width).toBe(100);
      expect(img.src.data.length).toBe(100 * 100 * 4);
    }
  });

  it("supports opacity", () => {
    const img = paintImage(0, 0, 100, 100, "file:///x.png", { opacity: 0.8 });
    expect(img.opacity).toBe(0.8);
  });
});

// ============================================================================
// paintText builder
// ============================================================================

describe("paintText", () => {
  it("creates a text instruction with the required fields", () => {
    const t = paintText(10, 20, "Hello", "canvas:Helvetica@16", 16, "#111");
    expect(t.kind).toBe("text");
    expect(t.x).toBe(10);
    expect(t.y).toBe(20);
    expect(t.text).toBe("Hello");
    expect(t.font_ref).toBe("canvas:Helvetica@16");
    expect(t.font_size).toBe(16);
    expect(t.fill).toBe("#111");
  });

  it("passes through optional cluster_positions", () => {
    const t = paintText(0, 0, "Hi", "canvas:Arial@16", 16, "#000", {
      cluster_positions: [{ cluster: 0, x: 0 }, { cluster: 1, x: 8 }],
    });
    expect(t.cluster_positions).toHaveLength(2);
    expect(t.cluster_positions?.[1].x).toBe(8);
  });

  it("passes through optional metadata and id from PaintBase", () => {
    const t = paintText(0, 0, "X", "canvas:Arial@16", 16, "#000", {
      id: "tag-1",
      metadata: { "layout:align": "start" },
    });
    expect(t.id).toBe("tag-1");
    expect(t.metadata?.["layout:align"]).toBe("start");
  });
});

// ============================================================================
// paintScene builder
// ============================================================================

describe("paintScene", () => {
  it("creates a scene with viewport and instructions", () => {
    const scene = paintScene(800, 600, "#ffffff", [
      paintRect(0, 0, 800, 600, { fill: "#f8fafc" }),
    ]);
    expect(scene.width).toBe(800);
    expect(scene.height).toBe(600);
    expect(scene.background).toBe("#ffffff");
    expect(scene.instructions).toHaveLength(1);
  });

  it("creates a transparent scene for compositing", () => {
    const scene = paintScene(400, 300, "transparent", []);
    expect(scene.background).toBe("transparent");
    expect(scene.instructions).toHaveLength(0);
  });

  it("accepts an optional id and metadata", () => {
    const scene = paintScene(100, 100, "#000", [], {
      id: "dashboard-scene-v2",
      metadata: { frame: 42 },
    });
    expect(scene.id).toBe("dashboard-scene-v2");
    expect(scene.metadata).toEqual({ frame: 42 });
  });
});

// ============================================================================
// PixelContainer — structural correctness
// ============================================================================

describe("PixelContainer", () => {
  it("holds RGBA8 pixel data with correct byte length", () => {
    const w = 4;
    const h = 2;
    const pixels: PixelContainer = {
      width: w,
      height: h,
      data: new Uint8Array(w * h * 4), // 32 bytes
    };
    expect(pixels.data.byteLength).toBe(32);
  });

  it("pixel offset formula: (row * width + col) * 4", () => {
    const w = 3;
    const h = 2;
    const raw = new Uint8Array(w * h * 4);
    // Set pixel at (row=1, col=2) to red
    const offset = (1 * w + 2) * 4;
    raw[offset + 0] = 255; // R
    raw[offset + 1] = 0;   // G
    raw[offset + 2] = 0;   // B
    raw[offset + 3] = 255; // A
    const pixels: PixelContainer = { width: w, height: h, data: raw };
    expect(pixels.data[offset]).toBe(255);
    expect(pixels.data[offset + 1]).toBe(0);
  });
});

// ============================================================================
// ImageCodec — interface shape (structural, no runtime impl needed)
// ============================================================================

describe("ImageCodec interface", () => {
  it("can be implemented with a stub codec", () => {
    const stubCodec: ImageCodec = {
      mime_type: "image/stub",
      encode(pixels: PixelContainer): Uint8Array {
        // Stub: just return empty bytes
        return new Uint8Array(0);
      },
      decode(bytes: Uint8Array): PixelContainer {
        // Stub: return a 1x1 transparent pixel
        return {
          width: 1,
          height: 1,
          channels: 4,
          bit_depth: 8,
          pixels: new Uint8Array(4),
        };
      },
    };
    expect(stubCodec.mime_type).toBe("image/stub");
    const encoded = stubCodec.encode({
      width: 1,
      height: 1,
      channels: 4,
      bit_depth: 8,
      pixels: new Uint8Array(4),
    });
    expect(encoded).toBeInstanceOf(Uint8Array);
    const decoded = stubCodec.decode(new Uint8Array(0));
    expect(decoded.width).toBe(1);
  });
});

// ============================================================================
// PaintInstruction union — discriminant routing
// ============================================================================

describe("PaintInstruction union discriminant", () => {
  const allKinds: PaintInstruction[] = [
    paintRect(0, 0, 10, 10),
    paintEllipse(5, 5, 5, 5),
    paintPath([{ kind: "move_to", x: 0, y: 0 }]),
    { kind: "glyph_run", glyphs: [], font_ref: "Inter", font_size: 16 },
    paintGroup([]),
    paintLayer([]),
    paintLine(0, 0, 10, 0, "#000"),
    paintClip(0, 0, 100, 100, []),
    paintGradient("linear", [{ offset: 0, color: "#000" }]),
    paintImage(0, 0, 10, 10, "data:image/png;base64,"),
  ];

  const expectedKinds = [
    "rect", "ellipse", "path", "glyph_run", "group",
    "layer", "line", "clip", "gradient", "image",
  ];

  it("covers all 10 instruction kinds", () => {
    expect(allKinds.map(i => i.kind)).toEqual(expectedKinds);
  });

  it("switch on kind narrows type correctly", () => {
    let rectCount = 0;
    for (const instr of allKinds) {
      if (instr.kind === "rect") {
        rectCount++;
        expect(instr.x).toBe(0);
      }
    }
    expect(rectCount).toBe(1);
  });
});

// ============================================================================
// FilterEffect union — all filter kinds
// ============================================================================

describe("FilterEffect union", () => {
  it("covers all filter effect kinds", () => {
    const filters: FilterEffect[] = [
      { kind: "blur", radius: 5 },
      { kind: "drop_shadow", dx: 2, dy: 2, blur: 4, color: "#000" },
      { kind: "color_matrix", matrix: new Array(20).fill(0) },
      { kind: "brightness", amount: 1.2 },
      { kind: "contrast", amount: 0.8 },
      { kind: "saturate", amount: 1.5 },
      { kind: "hue_rotate", angle: 90 },
      { kind: "invert", amount: 1.0 },
      { kind: "opacity", amount: 0.5 },
    ];
    expect(filters.map(f => f.kind)).toEqual([
      "blur", "drop_shadow", "color_matrix", "brightness",
      "contrast", "saturate", "hue_rotate", "invert", "opacity",
    ]);
  });
});
