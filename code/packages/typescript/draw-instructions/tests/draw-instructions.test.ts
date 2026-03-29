import { describe, expect, it } from "vitest";
import {
  VERSION,
  createScene,
  drawClip,
  drawGroup,
  drawLine,
  drawRect,
  drawText,
  renderWith,
} from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("helpers", () => {
  it("creates a rectangle instruction", () => {
    expect(drawRect(1, 2, 3, 4, "#111111")).toEqual({
      kind: "rect",
      x: 1,
      y: 2,
      width: 3,
      height: 4,
      fill: "#111111",
      metadata: undefined,
    });
  });

  it("creates a text instruction with defaults", () => {
    expect(drawText(10, 20, "hello")).toMatchObject({
      kind: "text",
      x: 10,
      y: 20,
      value: "hello",
      fill: "#000000",
      fontFamily: "monospace",
      fontSize: 16,
      align: "middle",
    });
  });

  it("creates a grouped scene", () => {
    const group = drawGroup([drawRect(0, 0, 5, 5)]);
    const scene = createScene(100, 50, [group], { metadata: { kind: "demo" } });

    expect(scene).toMatchObject({
      width: 100,
      height: 50,
      background: "#ffffff",
      metadata: { kind: "demo" },
    });
  });

  it("creates a rectangle with stroke options", () => {
    expect(
      drawRect(0, 0, 10, 10, "#fff", { stroke: "#f00", strokeWidth: 2 }),
    ).toMatchObject({
      kind: "rect",
      fill: "#fff",
      stroke: "#f00",
      strokeWidth: 2,
    });
  });

  it("creates a rectangle with metadata (backward compat)", () => {
    const rect = drawRect(0, 0, 10, 10, "#000", { bar: true });
    expect(rect.metadata).toEqual({ bar: true });
    expect(rect.stroke).toBeUndefined();
  });

  it("creates a line instruction", () => {
    expect(drawLine(0, 0, 100, 0, "#333", 2)).toMatchObject({
      kind: "line",
      x1: 0,
      y1: 0,
      x2: 100,
      y2: 0,
      stroke: "#333",
      strokeWidth: 2,
    });
  });

  it("creates a line with defaults", () => {
    const line = drawLine(10, 20, 30, 40);
    expect(line.stroke).toBe("#000000");
    expect(line.strokeWidth).toBe(1);
  });

  it("creates a clip instruction", () => {
    const inner = drawRect(5, 5, 10, 10);
    const clip = drawClip(0, 0, 50, 50, [inner]);
    expect(clip).toMatchObject({
      kind: "clip",
      x: 0,
      y: 0,
      width: 50,
      height: 50,
    });
    expect(clip.children).toHaveLength(1);
    expect(clip.children[0]!.kind).toBe("rect");
  });

  it("creates text with fontWeight", () => {
    const text = drawText(10, 20, "Header", { fontWeight: "bold" });
    expect(text.fontWeight).toBe("bold");
  });

  it("creates text with default fontWeight undefined", () => {
    const text = drawText(10, 20, "Normal");
    expect(text.fontWeight).toBeUndefined();
  });
});

describe("renderWith()", () => {
  it("delegates to the provided renderer", () => {
    const scene = createScene(10, 10, []);
    const output = renderWith(scene, {
      render(input) {
        return `${input.width}x${input.height}`;
      },
    });

    expect(output).toBe("10x10");
  });
});
