import { describe, expect, it } from "vitest";
import {
  graphDiagram,
  graphEdge,
  graphNode,
  resolveStyle,
} from "../src/index.js";

describe("diagram-ir builders", () => {
  it("builds a graph diagram with defaults", () => {
    const diagram = graphDiagram(
      [graphNode("A"), graphNode("B")],
      [graphEdge("A", "B")],
    );

    expect(diagram.kind).toBe("graph");
    expect(diagram.direction).toBe("tb");
    expect(diagram.nodes[0].label.text).toBe("A");
    expect(diagram.edges[0].kind).toBe("directed");
  });

  it("resolves styles with sensible defaults", () => {
    const style = resolveStyle({ fill: "#ff0000" });

    expect(style.fill).toBe("#ff0000");
    expect(style.stroke).toBe("#1f2937");
    expect(style.fontSize).toBe(14);
  });
});
