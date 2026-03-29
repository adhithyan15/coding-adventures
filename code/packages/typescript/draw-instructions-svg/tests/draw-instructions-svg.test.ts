import { describe, expect, it } from "vitest";
import { createScene, drawClip, drawGroup, drawLine, drawRect, drawText } from "@coding-adventures/draw-instructions";
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

  it("renders rect with stroke attributes", () => {
    const scene = createScene(100, 50, [
      drawRect(10, 10, 80, 30, "#fff", { stroke: "#f00", strokeWidth: 2 }),
    ]);
    const svg = renderSvg(scene);
    expect(svg).toContain('stroke="#f00"');
    expect(svg).toContain('stroke-width="2"');
  });

  it("omits stroke attributes when not set", () => {
    const scene = createScene(100, 50, [drawRect(10, 10, 80, 30)]);
    const svg = renderSvg(scene);
    expect(svg).not.toContain("stroke=");
  });

  it("renders line instructions", () => {
    const scene = createScene(100, 50, [drawLine(0, 25, 100, 25, "#333", 1)]);
    const svg = renderSvg(scene);
    expect(svg).toContain('<line x1="0" y1="25" x2="100" y2="25"');
    expect(svg).toContain('stroke="#333"');
    expect(svg).toContain('stroke-width="1"');
  });

  it("renders clip instructions with clipPath", () => {
    const scene = createScene(100, 50, [
      drawClip(10, 10, 80, 30, [drawText(20, 25, "Clipped")]),
    ]);
    const svg = renderSvg(scene);
    expect(svg).toContain("<clipPath");
    expect(svg).toContain('clip-path="url(#clip-1)"');
    expect(svg).toContain("Clipped");
  });

  it("renders text with font-weight when bold", () => {
    const scene = createScene(100, 50, [
      drawText(50, 25, "Bold", { fontWeight: "bold" }),
    ]);
    const svg = renderSvg(scene);
    expect(svg).toContain('font-weight="bold"');
  });

  it("omits font-weight for normal text", () => {
    const scene = createScene(100, 50, [drawText(50, 25, "Normal")]);
    const svg = renderSvg(scene);
    expect(svg).not.toContain("font-weight");
  });
});
