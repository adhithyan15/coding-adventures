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
} from "../src/index.js";

const representations = [
  GraphRepr.ADJACENCY_LIST,
  GraphRepr.ADJACENCY_MATRIX,
] as const;

function makeGraph(repr: GraphRepr): Graph<string> {
  const graph = new Graph<string>(repr);
  graph.addEdge("London", "Paris", 300);
  graph.addEdge("London", "Amsterdam", 520);
  graph.addEdge("Paris", "Berlin", 878);
  graph.addEdge("Amsterdam", "Berlin", 655);
  graph.addEdge("Amsterdam", "Brussels", 180);
  return graph;
}

function makeTriangle(repr: GraphRepr): Graph<string> {
  const graph = new Graph<string>(repr);
  graph.addEdge("A", "B");
  graph.addEdge("B", "C");
  graph.addEdge("C", "A");
  return graph;
}

function makePath(repr: GraphRepr): Graph<string> {
  const graph = new Graph<string>(repr);
  graph.addEdge("A", "B");
  graph.addEdge("B", "C");
  return graph;
}

describe("construction", () => {
  it("defaults to adjacency list", () => {
    expect(new Graph<string>().repr).toBe(GraphRepr.ADJACENCY_LIST);
  });

  for (const repr of representations) {
    it(`tracks empty state for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      expect(graph.size).toBe(0);
      expect(graph.nodes()).toEqual(new Set());
    });
  }
});

describe("node operations", () => {
  for (const repr of representations) {
    it(`adds and removes nodes for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A");
      graph.addNode("B");
      expect(graph.hasNode("A")).toBe(true);
      expect(graph.size).toBe(2);

      graph.removeNode("A");
      expect(graph.hasNode("A")).toBe(false);
      expect(graph.hasNode("B")).toBe(true);
      expect(graph.size).toBe(1);
    });

    it(`removing a missing node throws for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      expect(() => graph.removeNode("missing")).toThrow(/Node not found/);
    });
  }
});

describe("edge operations", () => {
  for (const repr of representations) {
    it(`creates undirected edges for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 2.5);
      expect(graph.hasEdge("A", "B")).toBe(true);
      expect(graph.hasEdge("B", "A")).toBe(true);
      expect(graph.edgeWeight("A", "B")).toBe(2.5);
      expect(graph.edgeWeight("B", "A")).toBe(2.5);
    });

    it(`keeps a self-loop visible as a neighbor for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "A");
      expect(graph.hasEdge("A", "A")).toBe(true);
      expect(graph.neighbors("A")).toEqual(new Set(["A"]));
    });

    it(`removes edges without deleting nodes for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.removeEdge("A", "B");
      expect(graph.hasNode("A")).toBe(true);
      expect(graph.hasNode("B")).toBe(true);
      expect(graph.hasEdge("A", "B")).toBe(false);
    });

    it(`deduplicates edges() for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 1);
      graph.addEdge("B", "C", 2);
      expect(graph.edges()).toHaveLength(2);
    });
  }
});

describe("property bags", () => {
  for (const repr of representations) {
    it(`stores graph properties for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.setGraphProperty("name", "city-map");
      graph.setGraphProperty("version", 1);
      expect(graph.graphProperties()).toEqual({
        name: "city-map",
        version: 1,
      });

      graph.removeGraphProperty("version");
      expect(graph.graphProperties()).toEqual({ name: "city-map" });
    });

    it(`stores and merges node properties for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A", { kind: "input" });
      graph.addNode("A", { trainable: false });
      graph.setNodeProperty("A", "slot", 0);
      expect(graph.nodeProperties("A")).toEqual({
        kind: "input",
        trainable: false,
        slot: 0,
      });

      const copy = graph.nodeProperties("A");
      copy.kind = "mutated";
      expect(graph.nodeProperties("A").kind).toBe("input");

      graph.removeNodeProperty("A", "slot");
      expect(graph.nodeProperties("A")).toEqual({
        kind: "input",
        trainable: false,
      });
    });

    it(`stores edge properties and treats weight as canonical for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 2.5, { role: "distance" });

      expect(graph.edgeProperties("A", "B")).toEqual({
        role: "distance",
        weight: 2.5,
      });
      expect(graph.edgeProperties("B", "A")).toEqual({
        role: "distance",
        weight: 2.5,
      });

      graph.setEdgeProperty("B", "A", "weight", 7);
      expect(graph.edgeWeight("A", "B")).toBe(7);
      expect(graph.edgeProperties("A", "B").weight).toBe(7);

      graph.setEdgeProperty("A", "B", "trainable", true);
      graph.removeEdgeProperty("A", "B", "role");
      expect(graph.edgeProperties("A", "B")).toEqual({
        trainable: true,
        weight: 7,
      });
    });

    it(`removes node and edge properties with their structure for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A", { kind: "input" });
      graph.addEdge("A", "B", 3, { role: "data" });

      graph.removeEdge("A", "B");
      expect(() => graph.edgeProperties("A", "B")).toThrow(/Edge not found/);

      graph.addEdge("A", "B", 3, { role: "data" });
      graph.removeNode("A");
      expect(() => graph.nodeProperties("A")).toThrow(/Node not found/);
      expect(() => graph.edgeProperties("A", "B")).toThrow(/Edge not found/);
    });
  }
});

describe("neighborhood queries", () => {
  for (const repr of representations) {
    it(`returns neighbors, weights, and degree for ${repr}`, () => {
      const graph = makeGraph(repr);
      expect(graph.neighbors("Amsterdam")).toEqual(
        new Set(["London", "Berlin", "Brussels"])
      );
      expect(graph.degree("Amsterdam")).toBe(3);
      expect(graph.neighborsWeighted("Amsterdam").get("London")).toBe(520);
      expect(graph.neighborsWeighted("Amsterdam").get("Brussels")).toBe(180);
    });
  }
});

describe("traversals", () => {
  for (const repr of representations) {
    it(`runs BFS over reachable nodes for ${repr}`, () => {
      expect(bfs(makePath(repr), "A")).toEqual(["A", "B", "C"]);
    });

    it(`runs DFS over reachable nodes for ${repr}`, () => {
      expect(dfs(makePath(repr), "A")).toEqual(["A", "B", "C"]);
    });

    it(`limits traversal to reachable nodes for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      expect(new Set(bfs(graph, "A"))).toEqual(new Set(["A", "B"]));
      expect(new Set(dfs(graph, "A"))).toEqual(new Set(["A", "B"]));
    });
  }
});

describe("connectivity", () => {
  for (const repr of representations) {
    it(`detects connected graphs for ${repr}`, () => {
      expect(isConnected(makeGraph(repr))).toBe(true);
    });

    it(`detects disconnected graphs for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      expect(isConnected(graph)).toBe(false);
    });

    it(`finds connected components for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addEdge("B", "C");
      graph.addEdge("D", "E");
      graph.addNode("F");
      const components = connectedComponents(graph);
      expect(components).toHaveLength(3);
      expect(components).toContainEqual(new Set(["A", "B", "C"]));
      expect(components).toContainEqual(new Set(["D", "E"]));
      expect(components).toContainEqual(new Set(["F"]));
    });
  }
});

describe("cycle detection", () => {
  for (const repr of representations) {
    it(`finds a cycle in a triangle for ${repr}`, () => {
      expect(hasCycle(makeTriangle(repr))).toBe(true);
    });

    it(`reports no cycle in a path for ${repr}`, () => {
      expect(hasCycle(makePath(repr))).toBe(false);
    });
  }
});

describe("shortest path", () => {
  for (const repr of representations) {
    it(`finds unweighted shortest path for ${repr}`, () => {
      expect(shortestPath(makePath(repr), "A", "C")).toEqual(["A", "B", "C"]);
    });

    it(`prefers lower total weight for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 1);
      graph.addEdge("B", "D", 10);
      graph.addEdge("A", "C", 3);
      graph.addEdge("C", "D", 3);
      expect(shortestPath(graph, "A", "D")).toEqual(["A", "C", "D"]);
    });

    it(`returns empty when no path exists for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A");
      graph.addNode("B");
      expect(shortestPath(graph, "A", "B")).toEqual([]);
    });

    it(`handles the city example for ${repr}`, () => {
      expect(shortestPath(makeGraph(repr), "London", "Berlin")).toEqual([
        "London",
        "Amsterdam",
        "Berlin",
      ]);
    });
  }
});

describe("minimum spanning tree", () => {
  for (const repr of representations) {
    it(`returns V-1 edges for ${repr}`, () => {
      const graph = makeGraph(repr);
      expect(minimumSpanningTree(graph).size).toBe(graph.size - 1);
    });

    it(`picks the cheapest edges in a triangle for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 1);
      graph.addEdge("B", "C", 2);
      graph.addEdge("C", "A", 4);
      const total = Array.from(minimumSpanningTree(graph)).reduce(
        (sum, [, , weight]) => sum + weight,
        0
      );
      expect(total).toBe(3);
    });

    it(`throws on disconnected graphs for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      expect(() => minimumSpanningTree(graph)).toThrow(/not connected/);
    });
  }
});

describe("edge cases", () => {
  for (const repr of representations) {
    it(`supports numeric nodes for ${repr}`, () => {
      const graph = new Graph<number>(repr);
      graph.addEdge(1, 2);
      graph.addEdge(2, 3);
      expect(shortestPath(graph, 1, 3)).toEqual([1, 2, 3]);
    });

    it(`supports tuple-like array nodes for ${repr}`, () => {
      const origin: readonly [number, number] = [0, 0];
      const north: readonly [number, number] = [0, 1];
      const east: readonly [number, number] = [1, 1];
      const graph = new Graph<readonly [number, number]>(repr);
      graph.addEdge(origin, north);
      graph.addEdge(north, east);
      expect(isConnected(graph)).toBe(true);
    });
  }

  it("handles a 1000-node sparse path graph quickly enough", () => {
    const graph = new Graph<number>(GraphRepr.ADJACENCY_LIST);
    for (let i = 0; i < 999; i++) {
      graph.addEdge(i, i + 1);
    }

    expect(graph.size).toBe(1000);
    expect(isConnected(graph)).toBe(true);
    expect(hasCycle(graph)).toBe(false);
  });
});
