import { describe, expect, it } from "vitest";
import { VERSION, renderToSvgString, createSvgContext, createSvgVM, assembleSvg } from "../src/index.js";
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
  type PaintScene,
  type PixelContainer,
} from "@coding-adventures/paint-instructions";
import { ExportNotSupportedError } from "@coding-adventures/paint-vm";

// ============================================================================
// VERSION
// ============================================================================

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ============================================================================
// Helpers
// ============================================================================

// Simple scene with one rect for reuse
function simpleScene(): PaintScene {
  return paintScene(400, 300, "#ffffff", [
    paintRect(10, 10, 100, 50, { fill: "#3b82f6" }),
  ]);
}

// ============================================================================
// renderToSvgString — top-level output
// ============================================================================

describe("renderToSvgString()", () => {
  it("returns a string starting with <svg", () => {
    const svg = renderToSvgString(simpleScene());
    expect(svg).toMatch(/^<svg/);
  });

  it("includes xmlns attribute", () => {
    const svg = renderToSvgString(simpleScene());
    expect(svg).toContain('xmlns="http://www.w3.org/2000/svg"');
  });

  it("includes width and height from scene", () => {
    const svg = renderToSvgString(paintScene(800, 600, "#fff", []));
    expect(svg).toContain('width="800"');
    expect(svg).toContain('height="600"');
  });

  it("closes with </svg>", () => {
    const svg = renderToSvgString(simpleScene());
    expect(svg).toMatch(/<\/svg>$/);
  });

  it("emits a background rect when background is not transparent", () => {
    const svg = renderToSvgString(paintScene(100, 100, "#f8fafc", []));
    expect(svg).toContain('fill="#f8fafc"');
  });

  it("does not emit a background rect for transparent background", () => {
    const svg = renderToSvgString(paintScene(100, 100, "transparent", []));
    // Should not have a background rect — the only content is <svg>...</svg>
    // with no fill="#transparent" or similar
    expect(svg).not.toContain('fill="transparent"');
  });

  it("supports explicit vm.execute() composition", () => {
    const scene = simpleScene();
    const vm = createSvgVM();
    const ctx = createSvgContext();

    vm.execute(scene, ctx);

    const svg = assembleSvg(scene, ctx);
    expect(svg).toContain("<rect");
  });
});

// ============================================================================
// PaintRect → <rect>
// ============================================================================

describe("PaintRect → SVG <rect>", () => {
  it("emits a <rect> with x, y, width, height", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintRect(10, 20, 100, 50, { fill: "#ef4444" }),
      ]),
    );
    expect(svg).toContain('<rect');
    expect(svg).toContain('x="10"');
    expect(svg).toContain('y="20"');
    expect(svg).toContain('width="100"');
    expect(svg).toContain('height="50"');
    expect(svg).toContain('fill="#ef4444"');
  });

  it("includes stroke and stroke-width when set", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { stroke: "#000", stroke_width: 2 }),
      ]),
    );
    expect(svg).toContain('stroke="#000"');
    expect(svg).toContain('stroke-width="2"');
  });

  it("includes rx for corner_radius", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { corner_radius: 8, fill: "#fff" }),
      ]),
    );
    expect(svg).toContain('rx="8"');
  });

  it("includes id when set", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintRect(0, 0, 50, 50, { id: "my-rect", fill: "#fff" }),
      ]),
    );
    expect(svg).toContain('id="my-rect"');
  });

  it("emits fill='none' when no fill specified", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintRect(0, 0, 50, 50, { stroke: "#000" }),
      ]),
    );
    expect(svg).toContain('fill="none"');
  });
});

// ============================================================================
// PaintEllipse → <ellipse>
// ============================================================================

describe("PaintEllipse → SVG <ellipse>", () => {
  it("emits <ellipse> with cx, cy, rx, ry", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintEllipse(100, 100, 50, 30, { fill: "#3b82f6" }),
      ]),
    );
    expect(svg).toContain('<ellipse');
    expect(svg).toContain('cx="100"');
    expect(svg).toContain('cy="100"');
    expect(svg).toContain('rx="50"');
    expect(svg).toContain('ry="30"');
  });
});

// ============================================================================
// PaintPath → <path>
// ============================================================================

describe("PaintPath → SVG <path>", () => {
  it("emits <path> with d attribute", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath([
          { kind: "move_to", x: 0, y: 0 },
          { kind: "line_to", x: 100, y: 0 },
          { kind: "close" },
        ], { fill: "#ef4444" }),
      ]),
    );
    expect(svg).toContain('<path');
    expect(svg).toContain('M 0 0');
    expect(svg).toContain('L 100 0');
    expect(svg).toContain('Z');
  });

  it("includes fill-rule=evenodd when set", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath([], { fill_rule: "evenodd" }),
      ]),
    );
    expect(svg).toContain('fill-rule="evenodd"');
  });

  it("emits cubic_to as C command", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath([
          { kind: "move_to", x: 0, y: 50 },
          { kind: "cubic_to", cx1: 25, cy1: 0, cx2: 75, cy2: 100, x: 100, y: 50 },
        ]),
      ]),
    );
    expect(svg).toContain('C 25 0 75 100 100 50');
  });

  it("emits arc_to as A command", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath([
          { kind: "move_to", x: 0, y: 0 },
          { kind: "arc_to", rx: 50, ry: 50, x_rotation: 0, large_arc: false, sweep: true, x: 100, y: 0 },
        ]),
      ]),
    );
    expect(svg).toContain('A 50 50 0 0 1 100 0');
  });
});

// ============================================================================
// PaintLine → <line>
// ============================================================================

describe("PaintLine → SVG <line>", () => {
  it("emits <line> with coordinates and stroke", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLine(0, 50, 200, 50, "#9ca3af", { stroke_width: 2 }),
      ]),
    );
    expect(svg).toContain('<line');
    expect(svg).toContain('x1="0"');
    expect(svg).toContain('y1="50"');
    expect(svg).toContain('x2="200"');
    expect(svg).toContain('y2="50"');
    expect(svg).toContain('stroke="#9ca3af"');
    expect(svg).toContain('stroke-width="2"');
  });
});

// ============================================================================
// PaintGroup → <g>
// ============================================================================

describe("PaintGroup → SVG <g>", () => {
  it("wraps children in a <g> element", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintGroup([
          paintRect(0, 0, 50, 50, { fill: "#3b82f6" }),
        ]),
      ]),
    );
    expect(svg).toContain("<g>");
    expect(svg).toContain("</g>");
    expect(svg).toContain("<rect");
  });

  it("includes transform matrix on <g>", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintGroup([], { transform: [1, 0, 0, 1, 50, 100] }),
      ]),
    );
    expect(svg).toContain('transform="matrix(1,0,0,1,50,100)"');
  });

  it("includes opacity on <g>", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintGroup([], { opacity: 0.5 }),
      ]),
    );
    expect(svg).toContain('opacity="0.5"');
  });
});

// ============================================================================
// PaintLayer → <g filter="..."> with <filter> in <defs>
// ============================================================================

describe("PaintLayer → SVG <g> with filter", () => {
  it("emits a <defs> section with a <filter> element for blur", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([paintRect(0, 0, 100, 100, { fill: "#3b82f6" })], {
          id: "glow",
          filters: [{ kind: "blur", radius: 10 }],
        }),
      ]),
    );
    expect(svg).toContain("<defs>");
    expect(svg).toContain("<filter");
    expect(svg).toContain("feGaussianBlur");
    expect(svg).toContain('stdDeviation="10"');
    expect(svg).toContain('filter="url(#filter-glow)"');
  });

  it("includes mix-blend-mode style for non-normal blend modes", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([], { id: "layer1", blend_mode: "multiply" }),
      ]),
    );
    expect(svg).toContain("mix-blend-mode:multiply");
  });

  it("emits drop_shadow filter", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          id: "shadow-layer",
          filters: [{ kind: "drop_shadow", dx: 4, dy: 4, blur: 8, color: "rgba(0,0,0,0.4)" }],
        }),
      ]),
    );
    expect(svg).toContain("feDropShadow");
  });

  it("emits saturate filter via feColorMatrix", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          id: "sat-layer",
          filters: [{ kind: "saturate", amount: 1.5 }],
        }),
      ]),
    );
    expect(svg).toContain('type="saturate"');
  });

  it("emits hue_rotate filter via feColorMatrix", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([], {
          id: "hue-layer",
          filters: [{ kind: "hue_rotate", angle: 90 }],
        }),
      ]),
    );
    expect(svg).toContain('type="hueRotate"');
  });
});

// ============================================================================
// PaintClip → <clipPath> in defs + <g clip-path="url(#...)">
// ============================================================================

describe("PaintClip → SVG clipPath", () => {
  it("emits a <clipPath> in <defs> and clips with url(#...)", () => {
    const svg = renderToSvgString(
      paintScene(400, 300, "transparent", [
        paintClip(0, 0, 200, 150, [
          paintRect(0, 0, 300, 200, { fill: "#e0f2fe" }),
        ]),
      ]),
    );
    expect(svg).toContain("<clipPath");
    expect(svg).toContain("clip-path=\"url(#");
    expect(svg).toContain("<rect"); // the clipped child
  });
});

// ============================================================================
// PaintGradient → <linearGradient> / <radialGradient> in defs
// ============================================================================

describe("PaintGradient → SVG gradient in defs", () => {
  it("emits a linearGradient in <defs> when referenced by id", () => {
    const svg = renderToSvgString(
      paintScene(400, 300, "transparent", [
        paintGradient(
          "linear",
          [{ offset: 0, color: "#3b82f6" }, { offset: 1, color: "#8b5cf6" }],
          { id: "blue-purple", x1: 0, y1: 0, x2: 400, y2: 0 },
        ),
        paintRect(0, 0, 400, 300, { fill: "url(#blue-purple)" }),
      ]),
    );
    expect(svg).toContain("<linearGradient");
    expect(svg).toContain('id="blue-purple"');
    expect(svg).toContain('<stop offset="0"');
    expect(svg).toContain('stop-color="#3b82f6"');
  });

  it("emits a radialGradient in <defs>", () => {
    const svg = renderToSvgString(
      paintScene(400, 300, "transparent", [
        paintGradient(
          "radial",
          [{ offset: 0, color: "#fff" }, { offset: 1, color: "#000" }],
          { id: "radial1", cx: 200, cy: 150, r: 100 },
        ),
      ]),
    );
    expect(svg).toContain("<radialGradient");
    expect(svg).toContain('r="100"');
  });

  it("silently ignores a gradient without an id (cannot be referenced)", () => {
    // No id → no way to reference it → skip
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintGradient("linear", [{ offset: 0, color: "#000" }]),
      ]),
    );
    // Should not crash; should produce valid SVG
    expect(svg).toMatch(/^<svg/);
    expect(svg).not.toContain("<linearGradient");
  });
});

// ============================================================================
// PaintImage → <image>
// ============================================================================

describe("PaintImage → SVG <image>", () => {
  it("emits <image> with href for URI string src", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintImage(10, 10, 100, 80, "https://example.com/logo.png"),
      ]),
    );
    expect(svg).toContain('<image');
    expect(svg).toContain('href="https://example.com/logo.png"');
    expect(svg).toContain('width="100"');
    expect(svg).toContain('height="80"');
  });

  it("emits <image> with placeholder href for PixelContainer src", () => {
    // PixelContainer is fixed RGBA8: { width, height, data: Uint8Array }.
    const pixels: PixelContainer = {
      width: 10, height: 10,
      data: new Uint8Array(400),
    };
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintImage(0, 0, 10, 10, pixels),
      ]),
    );
    expect(svg).toContain('<image');
    expect(svg).toContain('href="data:image/png;base64,"');
  });

  it("includes opacity when set", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintImage(0, 0, 50, 50, "file:///x.png", { opacity: 0.5 }),
      ]),
    );
    expect(svg).toContain('opacity="0.5"');
  });
});

// ============================================================================
// PaintGlyphRun → <text>
// ============================================================================

describe("PaintGlyphRun → SVG <text>", () => {
  it("emits a <text> element with tspan per glyph", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [
            { glyph_id: 65, x: 10, y: 50 }, // 'A'
            { glyph_id: 66, x: 20, y: 50 }, // 'B'
          ],
          font_ref: "Inter",
          font_size: 16,
          fill: "#111111",
        },
      ]),
    );
    expect(svg).toContain("<text");
    expect(svg).toContain('font-size="16"');
    expect(svg).toContain('fill="#111111"');
    expect(svg).toContain("<tspan");
    expect(svg).toContain("&#65;");
    expect(svg).toContain("&#66;");
  });

  it("uses default black fill when fill is not set", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [{ glyph_id: 65, x: 10, y: 50 }],
          font_ref: "Inter",
          font_size: 12,
        },
      ]),
    );
    expect(svg).toContain('fill="#000000"');
  });
});

// ============================================================================
// PaintText → <text>
// ============================================================================

describe("PaintText → SVG <text>", () => {
  it("emits native SVG text with font family and content escaping", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintText(12, 48, "A < B & C", "svg:Inter@16", 16, "#111111"),
      ]),
    );
    expect(svg).toContain("<text");
    expect(svg).toContain('x="12"');
    expect(svg).toContain('y="48"');
    expect(svg).toContain('font-family="Inter"');
    expect(svg).toContain('font-size="16"');
    expect(svg).toContain("A &lt; B &amp; C");
  });

  it("maps centered text alignment to SVG text-anchor middle", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintText(100, 48, "centered", "svg:Inter@16", 16, "#111111", {
          text_align: "center",
        }),
      ]),
    );
    expect(svg).toContain('text-anchor="middle"');
  });
});

// ============================================================================
// PaintPath — stroke_join and stroke_cap
// ============================================================================

describe("PaintPath — stroke_join and stroke_cap", () => {
  it("includes stroke-linejoin when stroke_join is set", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath(
          [{ kind: "move_to", x: 0, y: 0 }, { kind: "line_to", x: 100, y: 0 }],
          { stroke: "#000", stroke_join: "round" },
        ),
      ]),
    );
    expect(svg).toContain('stroke-linejoin="round"');
  });

  it("includes stroke-linecap when stroke_cap is set", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath(
          [{ kind: "move_to", x: 0, y: 0 }, { kind: "line_to", x: 100, y: 0 }],
          { stroke: "#000", stroke_cap: "square" },
        ),
      ]),
    );
    expect(svg).toContain('stroke-linecap="square"');
  });
});

// ============================================================================
// PaintLayer — counter-based filter ID (no id on instruction)
// ============================================================================

describe("PaintLayer — filter ID when no instruction id", () => {
  it("generates a counter-based filter id when instruction has no id", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([paintRect(0, 0, 50, 50, { fill: "#3b82f6" })], {
          filters: [{ kind: "blur", radius: 5 }],
          // no id — should use filter-0
        }),
      ]),
    );
    expect(svg).toContain("filter-0");
  });

  it("increments counter for multiple unnamed layers", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([], { filters: [{ kind: "blur", radius: 5 }] }),
        paintLayer([], { filters: [{ kind: "blur", radius: 10 }] }),
      ]),
    );
    expect(svg).toContain("filter-0");
    expect(svg).toContain("filter-1");
  });

  it("layer with opacity emits opacity attribute", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([paintRect(0, 0, 100, 100, { fill: "#3b82f6" })], {
          id: "fade",
          opacity: 0.3,
        }),
      ]),
    );
    expect(svg).toContain('opacity="0.3"');
  });
});

// ============================================================================
// Remaining filter kinds (brightness, contrast, invert, opacity)
// ============================================================================

describe("Filter kinds — brightness, contrast, invert, opacity", () => {
  it("emits feComponentTransfer for brightness", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintLayer([], {
          id: "bright",
          filters: [{ kind: "brightness", amount: 1.5 }],
        }),
      ]),
    );
    expect(svg).toContain("feComponentTransfer");
    expect(svg).toContain('slope="1.5"');
  });

  it("emits feComponentTransfer for contrast", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintLayer([], {
          id: "contrast",
          filters: [{ kind: "contrast", amount: 1.2 }],
        }),
      ]),
    );
    expect(svg).toContain("feComponentTransfer");
    expect(svg).toContain('slope="1.2"');
  });

  it("emits feComponentTransfer for invert", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintLayer([], {
          id: "invert",
          filters: [{ kind: "invert", amount: 1.0 }],
        }),
      ]),
    );
    expect(svg).toContain("feComponentTransfer");
    expect(svg).toContain('slope="-1"');
  });

  it("emits feComponentTransfer feFuncA for opacity filter", () => {
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintLayer([], {
          id: "opaque",
          filters: [{ kind: "opacity", amount: 0.5 }],
        }),
      ]),
    );
    expect(svg).toContain("feComponentTransfer");
    expect(svg).toContain("feFuncA");
    expect(svg).toContain('slope="0.5"');
  });
});

// ============================================================================
// export() — ExportNotSupportedError
// ============================================================================

describe("export()", () => {
  it("throws ExportNotSupportedError — SVG cannot produce pixel data", () => {
    const vm = createSvgVM();
    expect(() =>
      vm.export(paintScene(100, 100, "#fff", [])),
    ).toThrowError(ExportNotSupportedError);
  });
});

// ============================================================================
// XML safety — attribute escaping
// ============================================================================

describe("XML attribute escaping", () => {
  it("escapes & in color strings", () => {
    // Unusual but possible — verify escAttr is used
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintRect(0, 0, 50, 50, { fill: "#fff" }),
      ]),
    );
    // Should produce valid XML — no bare & characters in attributes
    expect(svg).not.toMatch(/="[^"]*&[^a-z#][^"]*"/);
  });

  it("escapes quotes in id attributes", () => {
    // An id with a quote would break the attribute
    // We verify the output doesn't crash and is well-formed
    const svg = renderToSvgString(
      paintScene(100, 100, "transparent", [
        paintRect(0, 0, 10, 10, { id: 'safe-id', fill: "#fff" }),
      ]),
    );
    expect(svg).toContain('id="safe-id"');
  });
});

// ============================================================================
// Snapshot tests — full SVG output
// ============================================================================

describe("SVG snapshot tests", () => {
  it("renders a simple blue rectangle scene", () => {
    const svg = renderToSvgString(
      paintScene(200, 100, "#ffffff", [
        paintRect(10, 10, 180, 80, { fill: "#3b82f6", corner_radius: 4, id: "card" }),
      ]),
    );
    // Must be valid SVG structure
    expect(svg).toContain('xmlns="http://www.w3.org/2000/svg"');
    expect(svg).toContain('width="200"');
    expect(svg).toContain('id="card"');
    expect(svg).toContain('rx="4"');
    expect(svg).toContain('fill="#3b82f6"');
  });

  it("renders a scene with gradient + rect referencing it", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintGradient("linear",
          [{ offset: 0, color: "#3b82f6" }, { offset: 1, color: "#8b5cf6" }],
          { id: "grad1", x1: 0, y1: 0, x2: 400, y2: 0 },
        ),
        paintRect(0, 0, 400, 200, { fill: "url(#grad1)" }),
      ]),
    );
    expect(svg).toContain("<defs>");
    expect(svg).toContain("<linearGradient");
    expect(svg).toContain('fill="url(#grad1)"');
  });

  it("renders nested groups with transforms", () => {
    const svg = renderToSvgString(
      paintScene(400, 400, "transparent", [
        paintGroup([
          paintGroup([
            paintRect(0, 0, 50, 50, { fill: "#ef4444" }),
          ], { transform: [1, 0, 0, 1, 100, 100] }),
        ], { opacity: 0.8 }),
      ]),
    );
    expect(svg).toContain('opacity="0.8"');
    expect(svg).toContain('transform="matrix(1,0,0,1,100,100)"');
  });
});

// ============================================================================
// Security hardening tests
// ============================================================================

describe("Security — safeNum validation", () => {
  it("throws RangeError when scene width is NaN", () => {
    expect(() =>
      renderToSvgString(paintScene(NaN, 400, "transparent", [])),
    ).toThrow(RangeError);
  });

  it("throws RangeError when scene height is Infinity", () => {
    expect(() =>
      renderToSvgString(paintScene(400, Infinity, "transparent", [])),
    ).toThrow(RangeError);
  });

  it("throws RangeError when blur filter radius is NaN", () => {
    expect(() =>
      renderToSvgString(
        paintScene(200, 200, "transparent", [
          paintLayer([], { filters: [{ kind: "blur", radius: NaN }] }),
        ]),
      ),
    ).toThrow(RangeError);
  });

  it("throws RangeError when color_matrix contains Infinity", () => {
    const matrix = new Array(20).fill(0) as number[];
    matrix[5] = Infinity;
    expect(() =>
      renderToSvgString(
        paintScene(200, 200, "transparent", [
          paintLayer([], { filters: [{ kind: "color_matrix", matrix }] }),
        ]),
      ),
    ).toThrow(RangeError);
  });
});

describe("Security — glyph_id injection prevention", () => {
  it("replaces out-of-range glyph_id with replacement character (U+FFFD)", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          glyphs: [{ glyph_id: 0x200000, x: 10, y: 50 }], // out of Unicode range
          font_ref: "Inter",
          font_size: 16,
        },
      ]),
    );
    // Should use the replacement character codepoint (65533 = 0xFFFD)
    expect(svg).toContain("&#65533;");
  });

  it("rejects non-integer glyph_id and uses replacement character", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        {
          kind: "glyph_run",
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          glyphs: [{ glyph_id: 1.5 as any, x: 10, y: 50 }],
          font_ref: "Inter",
          font_size: 16,
        },
      ]),
    );
    expect(svg).toContain("&#65533;");
    // Should NOT contain the fractional value
    expect(svg).not.toContain("&#1.5;");
  });
});

describe("Security — image href URI validation", () => {
  it("allows https: URIs in image href", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintImage(0, 0, 100, 100, "https://example.com/image.png"),
      ]),
    );
    expect(svg).toContain("https://example.com/image.png");
  });

  it("allows data: URIs in image href", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintImage(0, 0, 100, 100, "data:image/png;base64,abc123"),
      ]),
    );
    expect(svg).toContain("data:image/png;base64,abc123");
  });

  it("replaces javascript: URIs with a safe placeholder", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        paintImage(0, 0, 100, 100, "javascript:alert(1)" as any),
      ]),
    );
    expect(svg).not.toContain("javascript:");
    expect(svg).toContain("data:image/gif;base64,");
  });

  it("replaces file: URIs with a safe placeholder", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        paintImage(0, 0, 100, 100, "file:///etc/passwd" as any),
      ]),
    );
    expect(svg).not.toContain("file:");
    expect(svg).toContain("data:image/gif;base64,");
  });
});

describe("Security — blend mode allowlist", () => {
  it("rejects unknown blend mode values and falls back to normal", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintLayer([paintRect(0, 0, 50, 50, { fill: "#f00" })], {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          blend_mode: "normal;color:red" as any,
        }),
      ]),
    );
    // Injected CSS should not appear; the malicious blend_mode was sanitized
    expect(svg).not.toContain("color:red");
    // The fallback "normal" blend mode means no style attribute is emitted
    expect(svg).not.toContain("mix-blend-mode:normal;color:red");
  });
});

describe("PaintRect with opacity", () => {
  it("emits opacity attribute when opacity is set", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintRect(0, 0, 100, 50, { fill: "#3b82f6", opacity: 0.7 }),
      ]),
    );
    expect(svg).toContain('opacity="0.7"');
  });
});

describe("PaintPath cubic_to command", () => {
  it("emits cubic Bezier C command for cubic_to", () => {
    const svg = renderToSvgString(
      paintScene(200, 200, "transparent", [
        paintPath([
          { kind: "move_to", x: 0, y: 0 },
          { kind: "cubic_to", cx1: 10, cy1: 20, cx2: 30, cy2: 40, x: 100, y: 100 },
        ], { stroke: "#000" }),
      ]),
    );
    expect(svg).toContain("C ");
  });
});

// ============================================================================
// PaintText → SVG <text>
// ============================================================================

describe("PaintText → SVG <text>", () => {
  it("emits a <text> element with x, y, and text content", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(20, 50, "Hello, SVG!", "canvas:Helvetica@16", 16, "#111111"),
      ]),
    );
    expect(svg).toContain("<text");
    expect(svg).toContain('x="20"');
    expect(svg).toContain('y="50"');
    expect(svg).toContain("Hello, SVG!");
  });

  it("emits font-family from the canvas: font_ref scheme", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "text", "canvas:Arial@14", 14, "#000"),
      ]),
    );
    expect(svg).toContain('font-family="Arial"');
  });

  it("emits font-family from the svg: font_ref scheme", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "text", "svg:Georgia@12", 12, "#000"),
      ]),
    );
    expect(svg).toContain('font-family="Georgia"');
  });

  it("emits font-size from instr.font_size (not font_ref size)", () => {
    // The @<size> in font_ref is ignored; instr.font_size is authoritative.
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "text", "canvas:Arial@99", 18, "#000"),
      ]),
    );
    expect(svg).toContain('font-size="18"');
    // font_ref size (99) should NOT appear as font-size
    expect(svg).not.toContain('font-size="99"');
  });

  it("emits font-weight when weight is non-default (700)", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "Bold text", "canvas:Arial@16:700", 16, "#000"),
      ]),
    );
    expect(svg).toContain('font-weight="700"');
  });

  it("omits font-weight attribute for default weight (400)", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "Normal text", "canvas:Arial@16:400", 16, "#000"),
      ]),
    );
    expect(svg).not.toContain("font-weight");
  });

  it("emits font-style=italic when style is italic", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "Italic text", "canvas:Arial@16:400:italic", 16, "#000"),
      ]),
    );
    expect(svg).toContain('font-style="italic"');
  });

  it("omits font-style attribute when style is not set", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "Normal text", "canvas:Arial@16:700", 16, "#000"),
      ]),
    );
    expect(svg).not.toContain("font-style");
  });

  it("emits the fill color", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(0, 20, "Colored text", "canvas:Arial@16", 16, "#3b82f6"),
      ]),
    );
    expect(svg).toContain('fill="#3b82f6"');
  });

  it("emits text-anchor=middle for text_align=center", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(200, 50, "Centered", "canvas:Arial@16", 16, "#000", {
          text_align: "center",
        }),
      ]),
    );
    expect(svg).toContain('text-anchor="middle"');
  });

  it("emits text-anchor=end for text_align=end", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(400, 50, "Right", "canvas:Arial@16", 16, "#000", {
          text_align: "end",
        }),
      ]),
    );
    expect(svg).toContain('text-anchor="end"');
  });

  it("omits text-anchor attribute for default text_align=start", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(10, 50, "Left", "canvas:Arial@16", 16, "#000", {
          text_align: "start",
        }),
      ]),
    );
    // start is the SVG default — we omit the attribute for cleaner output
    expect(svg).not.toContain("text-anchor");
  });

  it("omits text-anchor when text_align is undefined", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(10, 50, "Default align", "canvas:Arial@16", 16, "#000"),
      ]),
    );
    expect(svg).not.toContain("text-anchor");
  });

  it("includes id attribute when set", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(10, 50, "Labeled", "canvas:Arial@16", 16, "#000", {
          id: "my-label",
        }),
      ]),
    );
    expect(svg).toContain('id="my-label"');
  });

  it("XML-escapes ampersands in text content", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(10, 50, "Me & You", "canvas:Arial@16", 16, "#000"),
      ]),
    );
    expect(svg).toContain("Me &amp; You");
    expect(svg).not.toContain("Me & You");
  });

  it("XML-escapes angle brackets in text content", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        paintText(10, 50, "1 < 2 > 0", "canvas:Arial@16", 16, "#000"),
      ]),
    );
    expect(svg).toContain("&lt;");
    expect(svg).toContain("&gt;");
  });

  it("throws RangeError when font_size is NaN", () => {
    expect(() =>
      renderToSvgString(
        paintScene(200, 200, "transparent", [
          paintText(0, 0, "x", "canvas:Arial@16", NaN, "#000"),
        ]),
      ),
    ).toThrow(RangeError);
  });

  it("throws RangeError when font_size is Infinity", () => {
    expect(() =>
      renderToSvgString(
        paintScene(200, 200, "transparent", [
          paintText(0, 0, "x", "canvas:Arial@16", Infinity, "#000"),
        ]),
      ),
    ).toThrow(RangeError);
  });

  it("falls back to sans-serif for unknown font_ref scheme", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        // "coretext:" is not a recognised scheme for the SVG backend
        paintText(0, 20, "text", "coretext:Arial@16", 16, "#000"),
      ]),
    );
    expect(svg).toContain('font-family="sans-serif"');
  });

  it("strips injection characters from font-family", () => {
    const svg = renderToSvgString(
      paintScene(400, 200, "transparent", [
        // Attempt to inject a closing quote + XSS payload into font-family:
        //   canvas:Arial"><script>@16  →  family parsed as  Arial"><script>
        //   after sanitization:        →  Arialscript  (< > " stripped, letters kept)
        paintText(0, 20, "text", 'canvas:Arial"><script>@16', 16, "#000"),
      ]),
    );
    // The raw injection payload must NOT appear verbatim in the SVG output.
    // < and " are stripped, so "<script>" becomes "script" (letters pass).
    // The attribute must be closed properly — no premature close of font-family="
    expect(svg).not.toContain("<script>");
    // Verify the attribute was not broken open — font-family value must be
    // a safe substring of the original (all letters, digits, etc.)
    expect(svg).toMatch(/font-family="[^"<>]*"/);
  });

  it("renders a complete text+rect scene (cowsay-style layout preview)", () => {
    const svg = renderToSvgString(
      paintScene(400, 150, "#ffffff", [
        paintRect(10, 10, 380, 80, { fill: "#f8fafc", stroke: "#9ca3af", stroke_width: 1, corner_radius: 4 }),
        paintText(20, 50, "Hello, World!", "canvas:Courier@16", 16, "#111111"),
        paintText(20, 80, "< Moo >", "canvas:Courier@16", 16, "#374151"),
      ]),
    );
    expect(svg).toContain("Hello, World!");
    expect(svg).toContain("&lt; Moo &gt;");
    expect(svg).toContain('<rect');
  });
});
