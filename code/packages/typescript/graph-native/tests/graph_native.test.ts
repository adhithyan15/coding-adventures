import { describe, expect, it } from "vitest";
import {
  bfs,
  connectedComponents,
  dfs,
  Graph,
  GraphRepr,
  hasCycle,
  isConnected,
  minimumSpanningTree,
  shortestPath,
} from "../index.js";

const representations = [
  GraphRepr.ADJACENCY_LIST,
  GraphRepr.ADJACENCY_MATRIX,
] as const;

function makeGraph(repr: (typeof GraphRepr)[keyof typeof GraphRepr]): Graph {
  const graph = new Graph(repr);
  graph.addEdge("London", "Paris", 300);
  graph.addEdge("London", "Amsterdam", 520);
  graph.addEdge("Paris", "Berlin", 878);
  graph.addEdge("Amsterdam", "Berlin", 655);
  graph.addEdge("Amsterdam", "Brussels", 180);
  return graph;
}

describe("graph-native construction", () => {
  it("defaults to adjacency list", () => {
    expect(new Graph().repr).toBe(GraphRepr.ADJACENCY_LIST);
  });

  for (const repr of representations) {
    it(`tracks nodes and edges for ${repr}`, () => {
      const graph = new Graph(repr);
      graph.addNode("A");
      graph.addEdge("A", "B", 2.5);

      expect(graph.size).toBe(2);
      expect(graph.hasNode("A")).toBe(true);
      expect(graph.hasEdge("A", "B")).toBe(true);
      expect(graph.hasEdge("B", "A")).toBe(true);
      expect(graph.edgeWeight("A", "B")).toBe(2.5);
      expect(graph.edges()).toEqual([["A", "B", 2.5]]);
    });
  }
});

describe("graph-native queries", () => {
  for (const repr of representations) {
    it(`returns neighbors, weights, and degree for ${repr}`, () => {
      const graph = makeGraph(repr);
      expect(graph.neighbors("Amsterdam")).toEqual(
        new Set(["London", "Berlin", "Brussels"])
      );
      expect(graph.neighborsWeighted("Amsterdam")).toEqual(
        new Map([
          ["Berlin", 655],
          ["Brussels", 180],
          ["London", 520],
        ])
      );
      expect(graph.degree("Amsterdam")).toBe(3);
    });

    it(`runs traversal and connectivity algorithms for ${repr}`, () => {
      const graph = makeGraph(repr);
      expect(bfs(graph, "London")).toEqual([
        "London",
        "Amsterdam",
        "Paris",
        "Berlin",
        "Brussels",
      ]);
      expect(dfs(graph, "London")).toEqual([
        "London",
        "Amsterdam",
        "Berlin",
        "Paris",
        "Brussels",
      ]);
      expect(isConnected(graph)).toBe(true);
    });
  }
});

describe("graph-native higher-level algorithms", () => {
  for (const repr of representations) {
    it(`finds components and cycles for ${repr}`, () => {
      const graph = new Graph(repr);
      graph.addEdge("A", "B");
      graph.addEdge("B", "C");
      graph.addEdge("C", "A");
      graph.addEdge("D", "E");

      const components = connectedComponents(graph);
      expect(components).toContainEqual(new Set(["A", "B", "C"]));
      expect(components).toContainEqual(new Set(["D", "E"]));
      expect(hasCycle(graph)).toBe(true);
    });

    it(`finds shortest paths and MSTs for ${repr}`, () => {
      const graph = makeGraph(repr);
      expect(shortestPath(graph, "London", "Berlin")).toEqual([
        "London",
        "Amsterdam",
        "Berlin",
      ]);
      const mst = minimumSpanningTree(graph);
      expect(mst.size).toBe(graph.size - 1);
      const total = Array.from(mst).reduce((sum, [, , weight]) => sum + weight, 0);
      expect(total).toBe(1655);
    });

    it(`throws on disconnected MST for ${repr}`, () => {
      const graph = new Graph(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      expect(() => minimumSpanningTree(graph)).toThrow(/not connected/);
    });
  }
});

describe("graph-native mutations", () => {
  it("removes edges and nodes while preserving remaining nodes", () => {
    const graph = new Graph();
    graph.addEdge("A", "B");
    graph.removeEdge("A", "B");
    expect(graph.hasNode("A")).toBe(true);
    expect(graph.hasNode("B")).toBe(true);
    expect(graph.hasEdge("A", "B")).toBe(false);

    graph.removeNode("B");
    expect(graph.hasNode("B")).toBe(false);
  });
});
