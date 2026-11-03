import assert from "node:assert/strict";
import { describe, it } from "node:test";

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

    assert.deepEqual(graph.nodes(), ["input"]);
    assert.deepEqual(graph.nodeProperties("input"), {
      "nn.op": "input",
      "nn.shape": "[1]",
    });
  });

  it("stores parallel directed edges with stable IDs", () => {
    const graph = new MultiDirectedGraph();

    const e0 = graph.addEdge("A", "B", 0.25, { channel: "left" }, "edge-a");
    const e1 = graph.addEdge("A", "B", 0.75, { channel: "right" }, "edge-b");

    assert.equal(e0, "edge-a");
    assert.equal(e1, "edge-b");
    assert.deepEqual(graph.edgesBetween("A", "B").map((edge) => edge.id), [
      "edge-a",
      "edge-b",
    ]);
    assert.deepEqual(graph.successors("A"), ["B"]);
    assert.deepEqual(graph.predecessors("B"), ["A"]);
  });

  it("supports generic node values", () => {
    const graph = new MultiDirectedGraph<number>();

    const edgeId = graph.addEdge(1, 2, 3.5);

    assert.deepEqual(graph.nodes(), [1, 2]);
    assert.deepEqual(graph.edge(edgeId), {
      id: edgeId,
      from: 1,
      to: 2,
      weight: 3.5,
    });
    assert.deepEqual(graph.successors(1), [2]);
    assert.deepEqual(graph.predecessors(2), [1]);
  });

  it("keeps edge properties and weights synchronized", () => {
    const graph = new MultiDirectedGraph();

    const edgeId = graph.addEdge("x0", "sum", 0.5, {
      "nn.trainable": true,
    });

    assert.deepEqual(graph.edgeProperties(edgeId), {
      "nn.trainable": true,
      weight: 0.5,
    });

    graph.setEdgeProperty(edgeId, "weight", 0.75);
    assert.equal(graph.edgeWeight(edgeId), 0.75);
    assert.equal(graph.edgeProperties(edgeId).weight, 0.75);

    graph.removeEdgeProperty(edgeId, "weight");
    assert.equal(graph.edgeWeight(edgeId), 1.0);
    assert.equal(graph.edgeProperties(edgeId).weight, 1.0);
  });

  it("stores graph properties", () => {
    const graph = new MultiDirectedGraph();

    graph.setGraphProperty("nn.name", "tiny-model");
    graph.setGraphProperty("nn.version", "0");
    assert.deepEqual(graph.graphProperties(), {
      "nn.name": "tiny-model",
      "nn.version": "0",
    });

    graph.removeGraphProperty("nn.version");
    assert.deepEqual(graph.graphProperties(), { "nn.name": "tiny-model" });
  });

  it("removes incident edges and metadata when removing a node", () => {
    const graph = new MultiDirectedGraph();

    const e0 = graph.addEdge("A", "B", 1.0, { role: "out" });
    const e1 = graph.addEdge("C", "A", 2.0, { role: "in" });

    graph.removeNode("A");

    assert.equal(graph.hasNode("A"), false);
    assert.equal(graph.hasEdge(e0), false);
    assert.equal(graph.hasEdge(e1), false);
    assert.throws(() => graph.edgeProperties(e0), EdgeNotFoundError);
    assert.throws(() => graph.nodeProperties("A"), NodeNotFoundError);
  });

  it("topologically sorts with parallel edge counts", () => {
    const graph = new MultiDirectedGraph();

    graph.addEdge("A", "B");
    graph.addEdge("A", "B");
    graph.addEdge("B", "C");

    assert.deepEqual(graph.topologicalSort(), ["A", "B", "C"]);
    assert.deepEqual(graph.independentGroups(), [["A"], ["B"], ["C"]]);
    assert.equal(graph.hasCycle(), false);
  });

  it("detects cycles", () => {
    const graph = new MultiDirectedGraph();

    graph.addEdge("A", "B");
    graph.addEdge("B", "A");

    assert.equal(graph.hasCycle(), true);
    assert.throws(() => graph.topologicalSort(), MultiDirectedGraphCycleError);
  });

  it("controls self-loops", () => {
    assert.throws(
      () => new MultiDirectedGraph().addEdge("A", "A"),
      /Self-loops are not allowed/,
    );

    const graph = new MultiDirectedGraph({ allowSelfLoops: true });
    const edgeId = graph.addEdge("A", "A");
    assert.deepEqual(graph.edge(edgeId), {
      id: edgeId,
      from: "A",
      to: "A",
      weight: 1,
    });
  });

  it("rejects duplicate edge IDs", () => {
    const graph = new MultiDirectedGraph();

    graph.addEdge("A", "B", 1.0, {}, "edge");

    assert.throws(
      () => graph.addEdge("A", "B", 1.0, {}, "edge"),
      DuplicateEdgeIdError,
    );
  });

  it("returns useful string summaries", () => {
    const graph = new MultiDirectedGraph();
    graph.addEdge("A", "B");

    assert.equal(graph.toString(), "MultiDirectedGraph(nodes=2, edges=1)");
  });
});
