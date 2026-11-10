import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
import { isDeepStrictEqual } from "node:util";
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
    assert.equal(new Graph<string>().repr, GraphRepr.ADJACENCY_LIST);
  });

  for (const repr of representations) {
    it(`tracks empty state for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      assert.equal(graph.size, 0);
      assert.deepEqual(graph.nodes(), new Set());
    });
  }
});

describe("node operations", () => {
  for (const repr of representations) {
    it(`adds and removes nodes for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A");
      graph.addNode("B");
      assert.equal(graph.hasNode("A"), true);
      assert.equal(graph.size, 2);

      graph.removeNode("A");
      assert.equal(graph.hasNode("A"), false);
      assert.equal(graph.hasNode("B"), true);
      assert.equal(graph.size, 1);
    });

    it(`removing a missing node throws for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      assert.throws(() => graph.removeNode("missing"), /Node not found/);
    });
  }
});

describe("edge operations", () => {
  for (const repr of representations) {
    it(`creates undirected edges for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 2.5);
      assert.equal(graph.hasEdge("A", "B"), true);
      assert.equal(graph.hasEdge("B", "A"), true);
      assert.equal(graph.edgeWeight("A", "B"), 2.5);
      assert.equal(graph.edgeWeight("B", "A"), 2.5);
    });

    it(`keeps a self-loop visible as a neighbor for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "A");
      assert.equal(graph.hasEdge("A", "A"), true);
      assert.deepEqual(graph.neighbors("A"), new Set(["A"]));
    });

    it(`removes edges without deleting nodes for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.removeEdge("A", "B");
      assert.equal(graph.hasNode("A"), true);
      assert.equal(graph.hasNode("B"), true);
      assert.equal(graph.hasEdge("A", "B"), false);
    });

    it(`deduplicates edges() for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 1);
      graph.addEdge("B", "C", 2);
      assert.equal(graph.edges().length, 2);
    });
  }
});

describe("property bags", () => {
  for (const repr of representations) {
    it(`stores graph properties for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.setGraphProperty("name", "city-map");
      graph.setGraphProperty("version", 1);
      assert.deepEqual(graph.graphProperties(), {
        name: "city-map",
        version: 1,
      });

      graph.removeGraphProperty("version");
      assert.deepEqual(graph.graphProperties(), { name: "city-map" });
    });

    it(`stores and merges node properties for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A", { kind: "input" });
      graph.addNode("A", { trainable: false });
      graph.setNodeProperty("A", "slot", 0);
      assert.deepEqual(graph.nodeProperties("A"), {
        kind: "input",
        trainable: false,
        slot: 0,
      });

      const copy = graph.nodeProperties("A");
      copy.kind = "mutated";
      assert.equal(graph.nodeProperties("A").kind, "input");

      graph.removeNodeProperty("A", "slot");
      assert.deepEqual(graph.nodeProperties("A"), {
        kind: "input",
        trainable: false,
      });
    });

    it(`stores edge properties and treats weight as canonical for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 2.5, { role: "distance" });

      assert.deepEqual(graph.edgeProperties("A", "B"), {
        role: "distance",
        weight: 2.5,
      });
      assert.deepEqual(graph.edgeProperties("B", "A"), {
        role: "distance",
        weight: 2.5,
      });

      graph.setEdgeProperty("B", "A", "weight", 7);
      assert.equal(graph.edgeWeight("A", "B"), 7);
      assert.equal(graph.edgeProperties("A", "B").weight, 7);

      graph.setEdgeProperty("A", "B", "trainable", true);
      graph.removeEdgeProperty("A", "B", "role");
      assert.deepEqual(graph.edgeProperties("A", "B"), {
        trainable: true,
        weight: 7,
      });
    });

    it(`removes node and edge properties with their structure for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A", { kind: "input" });
      graph.addEdge("A", "B", 3, { role: "data" });

      graph.removeEdge("A", "B");
      assert.throws(() => graph.edgeProperties("A", "B"), /Edge not found/);

      graph.addEdge("A", "B", 3, { role: "data" });
      graph.removeNode("A");
      assert.throws(() => graph.nodeProperties("A"), /Node not found/);
      assert.throws(() => graph.edgeProperties("A", "B"), /Edge not found/);
    });
  }
});

describe("neighborhood queries", () => {
  for (const repr of representations) {
    it(`returns neighbors, weights, and degree for ${repr}`, () => {
      const graph = makeGraph(repr);
      assert.deepEqual(
        graph.neighbors("Amsterdam"),
        new Set(["London", "Berlin", "Brussels"])
      );
      assert.equal(graph.degree("Amsterdam"), 3);
      assert.equal(graph.neighborsWeighted("Amsterdam").get("London"), 520);
      assert.equal(graph.neighborsWeighted("Amsterdam").get("Brussels"), 180);
    });
  }
});

describe("traversals", () => {
  for (const repr of representations) {
    it(`runs BFS over reachable nodes for ${repr}`, () => {
      assert.deepEqual(bfs(makePath(repr), "A"), ["A", "B", "C"]);
    });

    it(`runs DFS over reachable nodes for ${repr}`, () => {
      assert.deepEqual(dfs(makePath(repr), "A"), ["A", "B", "C"]);
    });

    it(`limits traversal to reachable nodes for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      assert.deepEqual(new Set(bfs(graph, "A")), new Set(["A", "B"]));
      assert.deepEqual(new Set(dfs(graph, "A")), new Set(["A", "B"]));
    });
  }
});

describe("connectivity", () => {
  for (const repr of representations) {
    it(`detects connected graphs for ${repr}`, () => {
      assert.equal(isConnected(makeGraph(repr)), true);
    });

    it(`detects disconnected graphs for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      assert.equal(isConnected(graph), false);
    });

    it(`finds connected components for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addEdge("B", "C");
      graph.addEdge("D", "E");
      graph.addNode("F");
      const components = connectedComponents(graph);
      assert.equal(components.length, 3);
      assert.ok(components.some((value) => isDeepStrictEqual(value, new Set(["A", "B", "C"]))));
      assert.ok(components.some((value) => isDeepStrictEqual(value, new Set(["D", "E"]))));
      assert.ok(components.some((value) => isDeepStrictEqual(value, new Set(["F"]))));
    });
  }
});

describe("cycle detection", () => {
  for (const repr of representations) {
    it(`finds a cycle in a triangle for ${repr}`, () => {
      assert.equal(hasCycle(makeTriangle(repr)), true);
    });

    it(`reports no cycle in a path for ${repr}`, () => {
      assert.equal(hasCycle(makePath(repr)), false);
    });
  }
});

describe("shortest path", () => {
  for (const repr of representations) {
    it(`finds unweighted shortest path for ${repr}`, () => {
      assert.deepEqual(shortestPath(makePath(repr), "A", "C"), ["A", "B", "C"]);
    });

    it(`prefers lower total weight for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B", 1);
      graph.addEdge("B", "D", 10);
      graph.addEdge("A", "C", 3);
      graph.addEdge("C", "D", 3);
      assert.deepEqual(shortestPath(graph, "A", "D"), ["A", "C", "D"]);
    });

    it(`returns empty when no path exists for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addNode("A");
      graph.addNode("B");
      assert.deepEqual(shortestPath(graph, "A", "B"), []);
    });

    it(`handles the city example for ${repr}`, () => {
      assert.deepEqual(shortestPath(makeGraph(repr), "London", "Berlin"), [
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
      assert.equal(minimumSpanningTree(graph).size, graph.size - 1);
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
      assert.equal(total, 3);
    });

    it(`throws on disconnected graphs for ${repr}`, () => {
      const graph = new Graph<string>(repr);
      graph.addEdge("A", "B");
      graph.addNode("C");
      assert.throws(() => minimumSpanningTree(graph), /not connected/);
    });
  }
});

describe("edge cases", () => {
  for (const repr of representations) {
    it(`supports numeric nodes for ${repr}`, () => {
      const graph = new Graph<number>(repr);
      graph.addEdge(1, 2);
      graph.addEdge(2, 3);
      assert.deepEqual(shortestPath(graph, 1, 3), [1, 2, 3]);
    });

    it(`supports tuple-like array nodes for ${repr}`, () => {
      const origin: readonly [number, number] = [0, 0];
      const north: readonly [number, number] = [0, 1];
      const east: readonly [number, number] = [1, 1];
      const graph = new Graph<readonly [number, number]>(repr);
      graph.addEdge(origin, north);
      graph.addEdge(north, east);
      assert.equal(isConnected(graph), true);
    });
  }

  it("handles a 1000-node sparse path graph quickly enough", () => {
    const graph = new Graph<number>(GraphRepr.ADJACENCY_LIST);
    for (let i = 0; i < 999; i++) {
      graph.addEdge(i, i + 1);
    }

    assert.equal(graph.size, 1000);
    assert.equal(isConnected(graph), true);
    assert.equal(hasCycle(graph), false);
  });
});
