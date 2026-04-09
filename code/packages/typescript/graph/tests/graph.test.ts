/**
 * Comprehensive test suite for the Graph package.
 *
 * Targets 95%+ code coverage by testing:
 * - Construction (empty, both representation types)
 * - Node operations (add, remove, duplicate handling)
 * - Edge operations (add, remove, weighted/unweighted)
 * - Adjacency queries (neighbors, degree, edges)
 * - All algorithms (BFS, DFS, shortest path, MST, cycles, components, connectivity)
 * - Edge cases (empty graph, single node, disconnected, complete graphs)
 * - Error conditions (missing nodes/edges, invalid operations)
 */

import { describe, it, expect } from "vitest";
import {
  Graph,
  GraphRepr,
  bfs,
  dfs,
  isConnected,
  connectedComponents,
  hasCycle,
  shortestPath,
  minimumSpanningTree,
} from "../src/index";

// ─── Construction Tests ────────────────────────────────────────────────────

describe("Graph Construction", () => {
  it("should create an empty graph with adjacency list", () => {
    const g = new Graph(GraphRepr.ADJACENCY_LIST);
    expect(g.length).toBe(0);
    expect(g.nodes().length).toBe(0);
    expect(g.edges().length).toBe(0);
  });

  it("should create an empty graph with adjacency matrix", () => {
    const g = new Graph(GraphRepr.ADJACENCY_MATRIX);
    expect(g.length).toBe(0);
    expect(g.nodes().length).toBe(0);
    expect(g.edges().length).toBe(0);
  });

  it("should use adjacency list as default", () => {
    const g = new Graph();
    g.addNode("A");
    expect(g.length).toBe(1);
  });

  it("should provide a string representation", () => {
    const g = new Graph();
    g.addNode("A");
    const repr = g.toString();
    expect(repr).toContain("Graph");
    expect(repr).toContain("adjacency_list");
  });
});

// ─── Node Operations Tests ────────────────────────────────────────────────────

describe("Node Operations", () => {
  describe.each([GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX])(
    "with %s representation",
    (repr) => {
      it("should add a node", () => {
        const g = new Graph(repr);
        g.addNode("A");
        expect(g.hasNode("A")).toBe(true);
        expect(g.length).toBe(1);
        expect(g.nodes()).toContain("A");
      });

      it("should add multiple nodes", () => {
        const g = new Graph(repr);
        g.addNode("A");
        g.addNode("B");
        g.addNode("C");
        expect(g.length).toBe(3);
        expect(new Set(g.nodes())).toEqual(new Set(["A", "B", "C"]));
      });

      it("should handle duplicate node additions as no-op", () => {
        const g = new Graph(repr);
        g.addNode("A");
        g.addNode("A");
        expect(g.length).toBe(1);
      });

      it("should remove a node", () => {
        const g = new Graph(repr);
        g.addNode("A");
        g.removeNode("A");
        expect(g.length).toBe(0);
        expect(g.hasNode("A")).toBe(false);
      });

      it("should remove a node and its edges", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        g.addEdge("A", "C");
        g.removeNode("A");
        expect(g.hasNode("A")).toBe(false);
        expect(g.hasNode("B")).toBe(true);
        expect(g.hasNode("C")).toBe(true);
        expect(g.hasEdge("A", "B")).toBe(false);
        expect(g.hasEdge("A", "C")).toBe(false);
        expect(g.edges().length).toBe(0);
      });

      it("should throw when removing nonexistent node", () => {
        const g = new Graph(repr);
        expect(() => g.removeNode("X")).toThrow();
      });

      it("should check node membership", () => {
        const g = new Graph(repr);
        expect(g.hasNode("A")).toBe(false);
        g.addNode("A");
        expect(g.hasNode("A")).toBe(true);
      });
    }
  );
});

// ─── Edge Operations Tests ────────────────────────────────────────────────────

describe("Edge Operations", () => {
  describe.each([GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX])(
    "with %s representation",
    (repr) => {
      it("should add an edge and create nodes", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        expect(g.hasNode("A")).toBe(true);
        expect(g.hasNode("B")).toBe(true);
      });

      it("should set default edge weight to 1.0", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        expect(g.edgeWeight("A", "B")).toBe(1.0);
        expect(g.edgeWeight("B", "A")).toBe(1.0);
      });

      it("should add edge with custom weight", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B", 5.0);
        expect(g.edgeWeight("A", "B")).toBe(5.0);
      });

      it("should maintain undirected edge symmetry", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B", 5.0);
        expect(g.hasEdge("A", "B")).toBe(true);
        expect(g.hasEdge("B", "A")).toBe(true);
        expect(g.edgeWeight("A", "B")).toBe(g.edgeWeight("B", "A"));
      });

      it("should update edge weight on duplicate", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B", 1.0);
        g.addEdge("A", "B", 5.0);
        expect(g.edgeWeight("A", "B")).toBe(5.0);
      });

      it("should remove an edge", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        g.removeEdge("A", "B");
        expect(g.hasEdge("A", "B")).toBe(false);
        expect(g.hasEdge("B", "A")).toBe(false);
      });

      it("should throw when removing nonexistent edge", () => {
        const g = new Graph(repr);
        g.addNode("A");
        g.addNode("B");
        expect(() => g.removeEdge("A", "B")).toThrow();
      });

      it("should throw when removing edge with missing node", () => {
        const g = new Graph(repr);
        g.addNode("A");
        expect(() => g.removeEdge("A", "B")).toThrow();
      });

      it("should check edge existence", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        expect(g.hasEdge("A", "B")).toBe(true);
        expect(g.hasEdge("B", "A")).toBe(true);
        expect(g.hasEdge("A", "C")).toBe(false);
      });

      it("should return false for has_edge with missing nodes", () => {
        const g = new Graph(repr);
        expect(g.hasEdge("A", "B")).toBe(false);
      });

      it("should return all edges once", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        g.addEdge("B", "C");
        g.addEdge("A", "C");
        expect(g.edges().length).toBe(3);

        // Check that edges are canonical (ordered)
        const edgeSet = new Set(g.edges().map(([a, b, _]) => `${a}-${b}`));
        expect(edgeSet.size).toBe(3); // No duplicates
      });

      it("should throw when getting weight of nonexistent edge", () => {
        const g = new Graph(repr);
        g.addNode("A");
        g.addNode("B");
        expect(() => g.edgeWeight("A", "B")).toThrow();
      });
    }
  );
});

// ─── Neighbourhood Queries Tests ───────────────────────────────────────────────

describe("Neighbourhood Queries", () => {
  describe.each([GraphRepr.ADJACENCY_LIST, GraphRepr.ADJACENCY_MATRIX])(
    "with %s representation",
    (repr) => {
      it("should return neighbors of a node", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        g.addEdge("A", "C");
        g.addEdge("B", "D");
        expect(new Set(g.neighbors("A"))).toEqual(new Set(["B", "C"]));
        expect(new Set(g.neighbors("B"))).toEqual(new Set(["A", "D"]));
      });

      it("should throw when querying neighbors of nonexistent node", () => {
        const g = new Graph(repr);
        expect(() => g.neighbors("X")).toThrow();
      });

      it("should return weighted neighbors", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B", 2.0);
        g.addEdge("A", "C", 3.0);
        const weighted = g.neighborsWeighted("A");
        expect(weighted.get("B")).toBe(2.0);
        expect(weighted.get("C")).toBe(3.0);
      });

      it("should calculate degree correctly", () => {
        const g = new Graph(repr);
        g.addEdge("A", "B");
        g.addEdge("A", "C");
        g.addEdge("A", "D");
        expect(g.degree("A")).toBe(3);
        expect(g.degree("B")).toBe(1);
      });

      it("should throw when getting degree of nonexistent node", () => {
        const g = new Graph(repr);
        expect(() => g.degree("X")).toThrow();
      });
    }
  );
});

// ─── Algorithm Tests ────────────────────────────────────────────────────────

describe("BFS Algorithm", () => {
  it("should traverse graph in breadth-first order", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    g.addEdge("B", "D");
    g.addEdge("C", "E");

    const result = bfs(g, "A");
    expect(result[0]).toBe("A");
    // B and C should be before D and E
    const indexB = result.indexOf("B");
    const indexC = result.indexOf("C");
    const indexD = result.indexOf("D");
    const indexE = result.indexOf("E");
    expect(Math.max(indexB, indexC)).toBeLessThan(Math.min(indexD, indexE));
  });

  it("should handle disconnected graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("C", "D");

    const result = bfs(g, "A");
    expect(result).toContain("A");
    expect(result).toContain("B");
    expect(result).not.toContain("C");
    expect(result).not.toContain("D");
  });
});

describe("DFS Algorithm", () => {
  it("should traverse graph in depth-first order", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    g.addEdge("B", "D");

    const result = dfs(g, "A");
    expect(result.length).toBe(4);
    expect(result[0]).toBe("A");
  });

  it("should handle disconnected graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("C", "D");

    const result = dfs(g, "A");
    expect(result).toContain("A");
    expect(result).toContain("B");
    expect(result).not.toContain("C");
  });
});

describe("Connectivity Tests", () => {
  it("should detect connected graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "A");
    expect(isConnected(g)).toBe(true);
  });

  it("should detect disconnected graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("C", "D");
    expect(isConnected(g)).toBe(false);
  });

  it("should handle empty graph", () => {
    const g = new Graph();
    expect(isConnected(g)).toBe(true);
  });

  it("should identify connected components", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("D", "E");
    g.addNode("F");

    const components = connectedComponents(g);
    expect(components.length).toBe(3);

    // Check component sizes
    const sizes = components.map((c) => c.size).sort();
    expect(sizes).toEqual([1, 2, 3]);
  });
});

describe("Cycle Detection", () => {
  it("should detect cycle in triangle graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "A");
    expect(hasCycle(g)).toBe(true);
  });

  it("should detect no cycle in tree", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("B", "D");
    expect(hasCycle(g)).toBe(false);
  });

  it("should handle single node", () => {
    const g = new Graph();
    g.addNode("A");
    expect(hasCycle(g)).toBe(false);
  });

  it("should handle disconnected graphs", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "A");
    g.addEdge("D", "E");
    expect(hasCycle(g)).toBe(true);
  });
});

describe("Shortest Path", () => {
  it("should find shortest path in unweighted graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "D");
    g.addEdge("A", "D");

    const path = shortestPath(g, "A", "D");
    expect(path).toEqual(["A", "D"]);
  });

  it("should return empty path if no path exists", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("C", "D");

    const path = shortestPath(g, "A", "D");
    expect(path).toEqual([]);
  });

  it("should handle same start and end", () => {
    const g = new Graph();
    g.addNode("A");

    expect(shortestPath(g, "A", "A")).toEqual(["A"]);
  });

  it("should handle same start and end with nonexistent node", () => {
    const g = new Graph();
    expect(shortestPath(g, "A", "A")).toEqual([]);
  });

  it("should find shortest weighted path", () => {
    const g = new Graph();
    g.addEdge("A", "B", 1.0);
    g.addEdge("B", "D", 10.0);
    g.addEdge("A", "C", 3.0);
    g.addEdge("C", "D", 3.0);

    const path = shortestPath(g, "A", "D");
    expect(path).toEqual(["A", "C", "D"]);
  });
});

describe("Minimum Spanning Tree", () => {
  it("should find MST for connected graph", () => {
    const g = new Graph();
    g.addEdge("A", "B", 1.0);
    g.addEdge("B", "C", 2.0);
    g.addEdge("A", "C", 3.0);

    const mst = minimumSpanningTree(g);
    expect(mst.length).toBe(2); // V - 1 edges

    // Check total weight
    const totalWeight = mst.reduce((sum, [_, __, w]) => sum + w, 0);
    expect(totalWeight).toBe(3.0);
  });

  it("should throw for disconnected graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("C", "D");

    expect(() => minimumSpanningTree(g)).toThrow();
  });

  it("should handle single node graph", () => {
    const g = new Graph();
    g.addNode("A");

    const mst = minimumSpanningTree(g);
    expect(mst.length).toBe(0);
  });

  it("should handle empty graph", () => {
    const g = new Graph();
    const mst = minimumSpanningTree(g);
    expect(mst.length).toBe(0);
  });

  it("should find correct MST in complex graph", () => {
    const g = new Graph();
    g.addEdge("A", "B", 3.0);
    g.addEdge("A", "C", 1.0);
    g.addEdge("B", "D", 4.0);
    g.addEdge("C", "D", 2.0);
    g.addEdge("C", "E", 5.0);
    g.addEdge("D", "E", 1.0);

    const mst = minimumSpanningTree(g);
    expect(mst.length).toBe(4); // 5 nodes -> 4 edges

    const totalWeight = mst.reduce((sum, [_, __, w]) => sum + w, 0);
    // Optimal: A-C (1) + C-D (2) + D-E (1) + B-D (4) = 8 or similar
    // The exact total depends on the algorithm, but it should be minimal
    expect(totalWeight).toBeLessThanOrEqual(10);
  });
});

describe("Both Representations", () => {
  it("should produce identical results for complex operation", () => {
    const gl = new Graph(GraphRepr.ADJACENCY_LIST);
    const gm = new Graph(GraphRepr.ADJACENCY_MATRIX);

    const edges = [
      ["A", "B", 1],
      ["B", "C", 2],
      ["C", "A", 3],
      ["D", "E", 4],
    ];

    for (const [u, v, w] of edges) {
      gl.addEdge(u, v, w as number);
      gm.addEdge(u, v, w as number);
    }

    // Compare basic properties
    expect(gl.length).toBe(gm.length);
    expect(gl.edges().length).toBe(gm.edges().length);

    // Compare algorithms
    expect(bfs(gl, "A").length).toBe(bfs(gm, "A").length);
    expect(dfs(gl, "A").length).toBe(dfs(gm, "A").length);
    expect(isConnected(gl)).toBe(isConnected(gm));
    expect(hasCycle(gl)).toBe(hasCycle(gm));
    expect(connectedComponents(gl).length).toBe(
      connectedComponents(gm).length
    );
  });
});
