import { describe, expect, it } from "vitest";
import {
  paintClip,
  paintGroup,
  paintLine,
  paintRect,
  paintScene,
} from "@coding-adventures/paint-instructions";
import { VERSION, createAsciiContext, createAsciiVM, renderToAscii } from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("renderToAscii()", () => {
  const opts = { scaleX: 1, scaleY: 1 };

  it("draws a stroked rectangle", () => {
    const scene = paintScene(5, 3, "#fff", [
      paintRect(0, 0, 4, 2, { fill: "transparent", stroke: "#000", stroke_width: 1 }),
    ]);
    expect(renderToAscii(scene, opts)).toBe("┌───┐\n│   │\n└───┘");
  });

  it("fills a rectangle with block characters", () => {
    const scene = paintScene(3, 2, "#fff", [
      paintRect(0, 0, 2, 1, { fill: "#000" }),
    ]);
    expect(renderToAscii(scene, opts)).toContain("█");
  });

  it("renders horizontal and vertical lines with intersections", () => {
    const scene = paintScene(5, 3, "#fff", [
      paintLine(0, 1, 4, 1, "#000"),
      paintLine(2, 0, 2, 2, "#000"),
    ]);
    const lines = renderToAscii(scene, opts).split("\n");
    expect(lines[0]![2]).toBe("│");
    expect(lines[1]![2]).toBe("┼");
    expect(lines[2]![2]).toBe("│");
  });

  it("renders glyph runs as direct characters", () => {
    const scene = paintScene(5, 1, "#fff", [
      {
        kind: "glyph_run",
        glyphs: [
          { glyph_id: "H".codePointAt(0)!, x: 0, y: 0 },
          { glyph_id: "i".codePointAt(0)!, x: 1, y: 0 },
        ],
        font_ref: "mono",
        font_size: 12,
      },
    ]);
    expect(renderToAscii(scene, opts)).toBe("Hi");
  });

  it("replaces unsafe terminal control glyphs", () => {
    const scene = paintScene(2, 1, "#fff", [
      {
        kind: "glyph_run",
        glyphs: [
          { glyph_id: 0x1b, x: 0, y: 0 },
          { glyph_id: "A".codePointAt(0)!, x: 1, y: 0 },
        ],
        font_ref: "mono",
        font_size: 12,
      },
    ]);
    expect(renderToAscii(scene, opts)).toBe("?A");
  });

  it("clips child output", () => {
    const scene = paintScene(10, 1, "#fff", [
      paintClip(0, 0, 3, 1, [
        {
          kind: "glyph_run",
          glyphs: [
            { glyph_id: "H".codePointAt(0)!, x: 0, y: 0 },
            { glyph_id: "e".codePointAt(0)!, x: 1, y: 0 },
            { glyph_id: "l".codePointAt(0)!, x: 2, y: 0 },
            { glyph_id: "l".codePointAt(0)!, x: 3, y: 0 },
            { glyph_id: "o".codePointAt(0)!, x: 4, y: 0 },
          ],
          font_ref: "mono",
          font_size: 12,
        },
      ]),
    ]);
    expect(renderToAscii(scene, opts)).toBe("Hel");
  });

  it("recurses through plain groups and layers", () => {
    const scene = paintScene(5, 1, "#fff", [
      paintGroup([
        {
          kind: "layer",
          children: [
            {
              kind: "glyph_run",
              glyphs: [
                { glyph_id: "A".codePointAt(0)!, x: 0, y: 0 },
                { glyph_id: "B".codePointAt(0)!, x: 1, y: 0 },
              ],
              font_ref: "mono",
              font_size: 12,
            },
          ],
        },
        {
          kind: "glyph_run",
          glyphs: [
            { glyph_id: "C".codePointAt(0)!, x: 3, y: 0 },
            { glyph_id: "D".codePointAt(0)!, x: 4, y: 0 },
          ],
          font_ref: "mono",
          font_size: 12,
        },
      ]),
    ]);
    expect(renderToAscii(scene, opts)).toBe("AB CD");
  });

  it("rejects transformed groups", () => {
    const scene = paintScene(5, 1, "#fff", [
      paintGroup([], { transform: [1, 0, 0, 1, 1, 0] }),
    ]);
    expect(() => renderToAscii(scene, opts)).toThrow(/transformed groups/);
  });
});

describe("createAsciiVM()", () => {
  it("executes through the PaintVM interface", () => {
    const vm = createAsciiVM({ scaleX: 1, scaleY: 1 });
    const ctx = createAsciiContext();
    const scene = paintScene(2, 1, "#fff", [
      {
        kind: "glyph_run",
        glyphs: [{ glyph_id: "O".codePointAt(0)!, x: 0, y: 0 }],
        font_ref: "mono",
        font_size: 12,
      },
    ]);
    vm.execute(scene, ctx);
    expect(ctx.buffer.toString()).toBe("O");
  });
});
