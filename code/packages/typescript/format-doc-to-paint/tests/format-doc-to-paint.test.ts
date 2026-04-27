import { describe, expect, it } from "vitest";
import { concat, group, hardline, indent, layoutDoc, line, softline, text } from "@coding-adventures/format-doc";
import { renderToAscii } from "@coding-adventures/paint-vm-ascii";
import { VERSION, docLayoutToPaintScene, docToPaintScene } from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("docLayoutToPaintScene()", () => {
  it("converts layout spans into glyph runs", () => {
    const layout = layoutDoc(concat([text("ab"), hardline(), text("c")]), {
      printWidth: 80,
    });

    const scene = docLayoutToPaintScene(layout);
    expect(scene.width).toBe(2);
    expect(scene.height).toBe(2);
    expect(scene.instructions).toHaveLength(2);
    expect(scene.instructions[0]).toMatchObject({
      kind: "glyph_run",
      glyphs: [
        { glyph_id: "a".codePointAt(0)!, x: 0, y: 0 },
        { glyph_id: "b".codePointAt(0)!, x: 1, y: 0 },
      ],
    });
  });
});

describe("docToPaintScene()", () => {
  it("supports the full Doc -> LayoutTree -> PaintScene -> ASCII pipeline", () => {
    const doc = group(
      concat([
        text("foo("),
        indent(concat([softline(), text("bar,"), line(), text("baz")])),
        softline(),
        text(")"),
      ])
    );

    const scene = docToPaintScene(doc, { printWidth: 10, indentWidth: 2 });
    expect(renderToAscii(scene, { scaleX: 1, scaleY: 1 })).toBe("foo(\n  bar,\n  baz\n)");
  });

  it("keeps flat groups on one line through the ASCII backend", () => {
    const doc = group(
      concat([
        text("["),
        indent(concat([softline(), text("a,"), line(), text("b")])),
        softline(),
        text("]"),
      ])
    );

    const scene = docToPaintScene(doc, { printWidth: 40 });
    expect(renderToAscii(scene, { scaleX: 1, scaleY: 1 })).toBe("[a, b]");
  });
});
