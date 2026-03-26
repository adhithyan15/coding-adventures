import { describe, expect, it } from "vitest";
import {
  VERSION,
  createScene,
  drawGroup,
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
