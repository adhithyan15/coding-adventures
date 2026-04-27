import { describe, expect, it } from "vitest";
import {
  graphDiagram,
  graphEdge,
  graphNode,
} from "@coding-adventures/diagram-ir";
import { layoutGraphDiagram } from "../src/index.js";

describe("layoutGraphDiagram", () => {
  it("assigns layered positions for a simple DAG", () => {
    const diagram = graphDiagram(
      [graphNode("A"), graphNode("B"), graphNode("C")],
      [graphEdge("A", "B"), graphEdge("B", "C")],
      { direction: "lr" },
    );

    const layout = layoutGraphDiagram(diagram);
    const a = layout.nodes.find((node) => node.id === "A")!;
    const b = layout.nodes.find((node) => node.id === "B")!;
    const c = layout.nodes.find((node) => node.id === "C")!;

    expect(a.x).toBeLessThan(b.x);
    expect(b.x).toBeLessThan(c.x);
    expect(layout.edges).toHaveLength(2);
    expect(layout.width).toBeGreaterThan(0);
    expect(layout.height).toBeGreaterThan(0);
  });

  it("creates explicit self-loop geometry", () => {
    const diagram = graphDiagram([graphNode("A")], [graphEdge("A", "A")]);
    const layout = layoutGraphDiagram(diagram);

    expect(layout.edges[0].points.length).toBeGreaterThan(2);
  });

  it("falls back deterministically for cycles", () => {
    const diagram = graphDiagram(
      [graphNode("A"), graphNode("B")],
      [graphEdge("A", "B"), graphEdge("B", "A")],
      { direction: "tb" },
    );

    const layout = layoutGraphDiagram(diagram);
    expect(layout.nodes[0].y).not.toBe(layout.nodes[1].y);
  });
});
