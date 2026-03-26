import { describe, expect, it } from "vitest";
import { createScene, drawGroup, drawRect, drawText } from "@coding-adventures/draw-instructions";
import { VERSION, renderSvg } from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("renderSvg()", () => {
  it("renders a complete svg document", () => {
    const scene = createScene(100, 50, [drawRect(10, 10, 20, 30)], {
      metadata: { label: "demo" },
    });

    const svg = renderSvg(scene);
    expect(svg).toContain("<svg");
    expect(svg).toContain('aria-label="demo"');
    expect(svg).toContain('<rect x="10" y="10" width="20" height="30"');
  });

  it("renders text and escapes values", () => {
    const scene = createScene(100, 50, [drawText(50, 40, "A&B")]);
    const svg = renderSvg(scene);
    expect(svg).toContain("A&amp;B");
  });

  it("renders groups recursively", () => {
    const scene = createScene(100, 50, [
      drawGroup([drawRect(1, 2, 3, 4)], { layer: "bars" }),
    ]);
    const svg = renderSvg(scene);
    expect(svg).toContain("<g");
    expect(svg).toContain('data-layer="bars"');
  });
});
