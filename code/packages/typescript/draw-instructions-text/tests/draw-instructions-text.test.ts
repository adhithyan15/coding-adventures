import { describe, expect, it } from "vitest";
import {
  createScene,
  drawClip,
  drawGroup,
  drawLine,
  drawRect,
  drawText,
  renderWith,
} from "@coding-adventures/draw-instructions";
import { VERSION, TEXT_RENDERER, renderText, createTextRenderer } from "../src/index.js";

describe("VERSION", () => {
  it("is 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("renderText()", () => {
  // Use a simple 1:1 scale for easy reasoning in tests
  const opts = { scaleX: 1, scaleY: 1 };

  describe("stroked rectangles", () => {
    it("draws a box with corners and edges", () => {
      const scene = createScene(5, 3, [
        drawRect(0, 0, 4, 2, "transparent", { stroke: "#000", strokeWidth: 1 }),
      ]);
      const result = renderText(scene, opts);

      // Expected:
      // ┌───┐
      // │   │
      // └───┘
      expect(result).toBe(
        "┌───┐\n" +
        "│   │\n" +
        "└───┘",
      );
    });
  });

  describe("filled rectangles", () => {
    it("fills with block characters", () => {
      const scene = createScene(3, 2, [
        drawRect(0, 0, 2, 1, "#000"),
      ]);
      const result = renderText(scene, opts);

      // Expected:
      // ███
      // ██
      expect(result).toContain("█");
    });
  });

  describe("horizontal lines", () => {
    it("draws a horizontal line", () => {
      const scene = createScene(5, 1, [
        drawLine(0, 0, 4, 0, "#000", 1),
      ]);
      const result = renderText(scene, opts);
      expect(result).toBe("─────");
    });
  });

  describe("vertical lines", () => {
    it("draws a vertical line", () => {
      const scene = createScene(1, 3, [
        drawLine(0, 0, 0, 2, "#000", 1),
      ]);
      const result = renderText(scene, opts);
      expect(result).toBe("│\n│\n│");
    });
  });

  describe("line intersections", () => {
    it("crossing lines produce a cross character", () => {
      // A horizontal line at y=1 and vertical line at x=2, crossing at (2,1)
      const scene = createScene(5, 3, [
        drawLine(0, 1, 4, 1, "#000", 1),
        drawLine(2, 0, 2, 2, "#000", 1),
      ]);
      const result = renderText(scene, opts);

      const lines = result.split("\n");
      // Row 0: "  │"  (vertical line at col 2)
      expect(lines[0]![2]).toBe("│");
      // Row 1: "──┼──" (cross at col 2)
      expect(lines[1]![2]).toBe("┼");
      // Row 2: "  │"
      expect(lines[2]![2]).toBe("│");
    });
  });

  describe("box with internal lines (table grid)", () => {
    it("produces a table-like grid", () => {
      // A 7x3 box with a horizontal divider at y=1
      const scene = createScene(7, 3, [
        drawRect(0, 0, 6, 2, "transparent", { stroke: "#000", strokeWidth: 1 }),
        drawLine(0, 1, 6, 1, "#000", 1),
      ]);
      const result = renderText(scene, opts);

      // Expected:
      // ┌─────┐
      // ├─────┤
      // └─────┘
      // The horizontal line at y=1 meets the left edge (├) and right edge (┤)
      const lines = result.split("\n");
      expect(lines[0]).toBe("┌─────┐");
      expect(lines[1]![0]).toBe("├");
      expect(lines[1]![6]).toBe("┤");
      expect(lines[2]).toBe("└─────┘");
    });
  });

  describe("text rendering", () => {
    it("writes text at the specified position", () => {
      const scene = createScene(10, 1, [
        drawText(0, 0, "Hello", { align: "start" }),
      ]);
      const result = renderText(scene, opts);
      expect(result).toBe("Hello");
    });

    it("centers text with middle alignment", () => {
      const scene = createScene(10, 1, [
        drawText(5, 0, "Hi", { align: "middle" }),
      ]);
      const result = renderText(scene, opts);
      // "Hi" centered at col 5: starts at col 4
      expect(result[4]).toBe("H");
      expect(result[5]).toBe("i");
    });

    it("right-aligns text with end alignment", () => {
      const scene = createScene(10, 1, [
        drawText(9, 0, "End", { align: "end" }),
      ]);
      const result = renderText(scene, opts);
      // "End" ending at col 9: starts at col 6
      expect(result[6]).toBe("E");
      expect(result[7]).toBe("n");
      expect(result[8]).toBe("d");
    });
  });

  describe("text inside a box", () => {
    it("renders text inside a stroked rectangle", () => {
      const scene = createScene(12, 3, [
        drawRect(0, 0, 11, 2, "transparent", { stroke: "#000", strokeWidth: 1 }),
        drawText(1, 1, "Hello", { align: "start" }),
      ]);
      const result = renderText(scene, opts);

      const lines = result.split("\n");
      expect(lines[0]).toBe("┌──────────┐");
      expect(lines[1]).toBe("│Hello     │");
      expect(lines[2]).toBe("└──────────┘");
    });
  });

  describe("clips", () => {
    it("clips text that extends beyond the region", () => {
      const scene = createScene(10, 1, [
        drawClip(0, 0, 3, 1, [
          drawText(0, 0, "Hello World", { align: "start" }),
        ]),
      ]);
      const result = renderText(scene, opts);
      // Only first 3 chars should appear
      expect(result).toBe("Hel");
    });
  });

  describe("groups", () => {
    it("recurses into group children", () => {
      const scene = createScene(5, 1, [
        drawGroup([
          drawText(0, 0, "AB", { align: "start" }),
          drawText(3, 0, "CD", { align: "start" }),
        ]),
      ]);
      const result = renderText(scene, opts);
      expect(result).toBe("AB CD");
    });
  });

  describe("table demo", () => {
    it("renders a complete table with headers and data", () => {
      // A small 2-column table: "Name" (6 wide) | "Age" (4 wide)
      // 6 rows: border, header text, divider, data row 1, data row 2, border
      const scene = createScene(13, 6, [
        // Outer border
        drawRect(0, 0, 12, 5, "transparent", { stroke: "#000", strokeWidth: 1 }),
        // Vertical divider at x=6
        drawLine(6, 0, 6, 5, "#000", 1),
        // Horizontal divider at y=2 (below headers)
        drawLine(0, 2, 12, 2, "#000", 1),
        // Header text (row 1, inside the box)
        drawText(1, 1, "Name", { align: "start" }),
        drawText(7, 1, "Age", { align: "start" }),
        // Data row 1 (row 3)
        drawText(1, 3, "Alice", { align: "start" }),
        drawText(7, 3, "30", { align: "start" }),
        // Data row 2 (row 4)
        drawText(1, 4, "Bob", { align: "start" }),
        drawText(7, 4, "25", { align: "start" }),
      ]);
      const result = renderText(scene, opts);

      const lines = result.split("\n");
      expect(lines[0]).toBe("┌─────┬─────┐");
      expect(lines[1]).toContain("Name");
      expect(lines[1]).toContain("Age");
      expect(lines[2]![0]).toBe("├");
      expect(lines[2]![6]).toBe("┼");
      expect(lines[2]![12]).toBe("┤");
      expect(lines[3]).toContain("Alice");
      expect(lines[3]).toContain("30");
      expect(lines[4]).toContain("Bob");
      expect(lines[4]).toContain("25");
      expect(lines[5]).toBe("└─────┴─────┘");
    });
  });

  describe("scale factor", () => {
    it("maps pixel coordinates to characters using scale", () => {
      // Default scale: 8px/col, 16px/row
      // A rect at (0,0) with width=80 height=32 → 10 cols, 2 rows
      // That's a 3-row box: row 0 (top), row 1 (middle), row 2 (bottom)
      const scene = createScene(88, 48, [
        drawRect(0, 0, 80, 32, "transparent", { stroke: "#000", strokeWidth: 1 }),
      ]);
      const result = renderText(scene); // default scale

      const lines = result.split("\n");
      expect(lines).toHaveLength(3); // rows 0, 1, 2
      expect(lines[0]![0]).toBe("┌");
      expect(lines[2]![0]).toBe("└");
    });

    it("respects custom scale factor", () => {
      const renderer = createTextRenderer({ scaleX: 4, scaleY: 4 });
      const scene = createScene(12, 8, [
        drawLine(0, 0, 12, 0, "#000", 1),
      ]);
      const result = renderer.render(scene);
      // 12px / 4 = 3 cols → "───" (4 chars: 0,1,2,3)
      expect(result).toContain("─");
    });
  });

  describe("renderWith integration", () => {
    it("works with renderWith from draw-instructions", () => {
      const scene = createScene(5, 1, [
        drawText(0, 0, "OK", { align: "start" }),
      ]);
      const result = renderWith(scene, createTextRenderer({ scaleX: 1, scaleY: 1 }));
      expect(result).toBe("OK");
    });
  });

  describe("empty scene", () => {
    it("returns empty string for empty scene", () => {
      const scene = createScene(0, 0, []);
      const result = renderText(scene, opts);
      expect(result).toBe("");
    });
  });

  describe("transparent rect is not rendered", () => {
    it("does not draw anything for transparent fill with no stroke", () => {
      const scene = createScene(5, 3, [
        drawRect(0, 0, 4, 2, "transparent"),
      ]);
      const result = renderText(scene, opts);
      // All spaces, trimmed to empty
      expect(result).toBe("");
    });
  });
});
