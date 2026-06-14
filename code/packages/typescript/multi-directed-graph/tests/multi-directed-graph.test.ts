import { describe, expect, it } from "vitest";

import {
  DuplicateEdgeIdError,
  EdgeNotFoundError,
  MultiDirectedGraph,
  MultiDirectedGraphCycleError,
  NodeNotFoundError,
} from "../src/index.js";

describe("MultiDirectedGraph", () => {
  it("adds nodes and merges node metadata", () => {
    const graph = new MultiDirectedGraph();

    graph.addNode("input", { "nn.op": "input" });
    graph.addNode("input", { "nn.shape": "[1]" });

    expect(graph.nodes()).toEqual(["input"]);
    expect(graph.nodeProperties("input")).toEqual({
      "nn.op": "input",
      "nn.shape": "[1]",
    });
  });

  it("stores parallel directed edges with stable IDs", () => {
    const graph = new MultiDirectedGraph();

    const e0 = graph.addEdge("A", "B", 0.25, { channel: "left" }, "edge-a");
    const e1 = graph.addEdge("A", "B", 0.75, { channel: "right" }, "edge-b");

    expect(e0).toBe("edge-a");
    expect(e1).toBe("edge-b");
    expect(graph.edgesBetween("A", "B").map((edge) => edge.id)).toEqual([
      "edge-a",
      "edge-b",
    ]);
    expect(graph.successors("A")).toEqual(["B"]);
    expect(graph.predecessors("B")).toEqual(["A"]);
  });

  it("supports generic node values", () => {
    const graph = new MultiDirectedGraph<number>();

    const edgeId = graph.addEdge(1, 2, 3.5);

    expect(graph.nodes()).toEqual([1, 2]);
    expect(graph.edge(edgeId)).toEqual({
      id: edgeId,
      from: 1,
      to: 2,
      weight: 3.5,
    });
    expect(graph.successors(1)).toEqual([2]);
    expect(graph.predecessors(2)).toEqual([1]);
  });

  it("keeps edge properties and weights synchronized", () => {
    const graph = new MultiDirectedGraph();

    const edgeId = graph.addEdge("x0", "sum", 0.5, {
      "nn.trainable": true,
    });

    expect(graph.edgeProperties(edgeId)).toEqual({
      "nn.trainable": true,
      weight: 0.5,
    });

    graph.setEdgeProperty(edgeId, "weight", 0.75);
    expect(graph.edgeWeight(edgeId)).toBe(0.75);
    expect(graph.edgeProperties(edgeId).weight).toBe(0.75);

    graph.removeEdgeProperty(edgeId, "weight");
    expect(graph.edgeWeight(edgeId)).toBe(1.0);
    expect(graph.edgeProperties(edgeId).weight).toBe(1.0);
  });

  it("stores graph properties", () => {
    const graph = new MultiDirectedGraph();

    graph.setGraphProperty("nn.name", "tiny-model");
    graph.setGraphProperty("nn.version", "0");
    expect(graph.graphProperties()).toEqual({
      "nn.name": "tiny-model",
      "nn.version": "0",
    });

    graph.removeGraphProperty("nn.version");
    expect(graph.graphProperties()).toEqual({ "nn.name": "tiny-model" });
  });

  it("removes incident edges and metadata when removing a node", () => {
    const graph = new MultiDirectedGraph();

    const e0 = graph.addEdge("A", "B", 1.0, { role: "out" });
    const e1 = graph.addEdge("C", "A", 2.0, { role: "in" });

    graph.removeNode("A");

    expect(graph.hasNode("A")).toBe(false);
    expect(graph.hasEdge(e0)).toBe(false);
    expect(graph.hasEdge(e1)).toBe(false);
    expect(() => graph.edgeProperties(e0)).toThrow(EdgeNotFoundError);
    expect(() => graph.nodeProperties("A")).toThrow(NodeNotFoundError);
  });

  it("topologically sorts with parallel edge counts", () => {
    const graph = new MultiDirectedGraph();

    graph.addEdge("A", "B");
    graph.addEdge("A", "B");
    graph.addEdge("B", "C");

    expect(graph.topologicalSort()).toEqual(["A", "B", "C"]);
    expect(graph.independentGroups()).toEqual([["A"], ["B"], ["C"]]);
    expect(graph.hasCycle()).toBe(false);
  });

  it("detects cycles", () => {
    const graph = new MultiDirectedGraph();

    graph.addEdge("A", "B");
    graph.addEdge("B", "A");

    expect(graph.hasCycle()).toBe(true);
    expect(() => graph.topologicalSort()).toThrow(MultiDirectedGraphCycleError);
  });

  it("controls self-loops", () => {
    expect(() => new MultiDirectedGraph().addEdge("A", "A")).toThrow(
      "Self-loops are not allowed"
    );

    const graph = new MultiDirectedGraph({ allowSelfLoops: true });
    const edgeId = graph.addEdge("A", "A");
    expect(graph.edge(edgeId)).toEqual({
      id: edgeId,
      from: "A",
      to: "A",
      weight: 1,
    });
  });

  it("rejects duplicate edge IDs", () => {
    const graph = new MultiDirectedGraph();

    graph.addEdge("A", "B", 1.0, {}, "edge");

    expect(() => graph.addEdge("A", "B", 1.0, {}, "edge")).toThrow(
      DuplicateEdgeIdError
    );
  });

  it("returns useful string summaries", () => {
    const graph = new MultiDirectedGraph();
    graph.addEdge("A", "B");

    expect(graph.toString()).toBe("MultiDirectedGraph(nodes=2, edges=1)");
  });
});
