import assert from "node:assert/strict";
import { describe, it } from "node:test";
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

    assert.equal(diagram.kind, "graph");
    assert.equal(diagram.direction, "tb");
    assert.equal(diagram.nodes[0].label.text, "A");
    assert.equal(diagram.edges[0].kind, "directed");
  });

  it("resolves styles with sensible defaults", () => {
    const style = resolveStyle({ fill: "#ff0000" });

    assert.equal(style.fill, "#ff0000");
    assert.equal(style.stroke, "#1f2937");
    assert.equal(style.fontSize, 14);
  });
});
