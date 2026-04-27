import { describe, expect, it } from "vitest";
import { parseDotToGraphDiagram } from "@coding-adventures/dot-parser";
import { layoutGraphDiagram } from "@coding-adventures/diagram-layout-graph";
import { diagramToPaint } from "../src/index.js";

describe("diagramToPaint", () => {
  it("lowers a layouted diagram to a paint scene", () => {
    const diagram = parseDotToGraphDiagram(`
      digraph Demo {
        rankdir=LR;
        A [label="Start"];
        B [shape=diamond];
        A -> B [label="next"];
      }
    `);

    const layout = layoutGraphDiagram(diagram);
    const scene = diagramToPaint(layout);

    expect(scene.width).toBeGreaterThan(0);
    expect(scene.height).toBeGreaterThan(0);
    expect(scene.instructions.some((instruction) => instruction.kind === "path")).toBe(true);
    expect(scene.instructions.some((instruction) => instruction.kind === "text")).toBe(true);
    expect(scene.instructions.some((instruction) => instruction.kind === "rect" || instruction.kind === "ellipse")).toBe(true);
  });
});
