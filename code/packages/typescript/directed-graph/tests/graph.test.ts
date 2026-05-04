/**
 * graph.test.ts -- Tests for Graph Core Operations and Algorithms
 * ================================================================
 *
 * These tests cover both the fundamental data structure operations and the
 * algorithmic methods of the directed graph: adding/removing nodes and edges,
 * querying neighbors, error handling, topological sort, cycle detection,
 * transitive closure/dependents, independent groups, and affected nodes.
 *
 * The tests are organized from simplest to most complex, following a natural
 * progression that mirrors how you'd learn the API:
 *
 * 1. Empty graph behavior
 * 2. Single node operations
 * 3. Single edge operations
 * 4. Multi-node/edge operations
 * 5. Error conditions
 * 6. Edge cases (toString, etc.)
 * 7. Topological sort
 * 8. Cycle detection
 * 9. Transitive closure
 * 10. Transitive dependents
 * 11. Independent groups
 * 12. Affected nodes
 * 13. Real repo graph integration test
 *
 * Each test function is named to describe WHAT it tests and WHAT the expected
 * outcome is, so you can read the test names as a specification of the graph's
 * behavior.
 */

import { describe, it, expect } from "vitest";
import {
  Graph,
  CycleError,
  NodeNotFoundError,
  EdgeNotFoundError,
} from "../src/graph.js";

// ======================================================================
// Helper: build common graph shapes
// ======================================================================
// We use factory functions instead of fixtures so each test gets a fresh
// graph. This avoids any accidental state sharing between tests.

function makeLinearChain(): Graph {
  /** Build A -> B -> C -> D. */
  const g = new Graph();
  g.addEdge("A", "B");
  g.addEdge("B", "C");
  g.addEdge("C", "D");
  return g;
}

function makeDiamond(): Graph {
  /**
   * Build the diamond shape:
   *
   *       A
   *      / \
   *     B   C
   *      \ /
   *       D
   *
   * Edges: A->B, A->C, B->D, C->D
   */
  const g = new Graph();
  g.addEdge("A", "B");
  g.addEdge("A", "C");
  g.addEdge("B", "D");
  g.addEdge("C", "D");
  return g;
}

function makeCycle(): Graph {
  /** Build A -> B -> C -> A (a three-node cycle). */
  const g = new Graph();
  g.addEdge("A", "B");
  g.addEdge("B", "C");
  g.addEdge("C", "A");
  return g;
}

// ======================================================================
// 1. Empty Graph
// ======================================================================
// An empty graph has no nodes and no edges. All query methods should return
// empty collections rather than throwing errors.

describe("Empty Graph", () => {
  it("has no nodes", () => {
    /** An empty graph should report zero nodes. */
    const g = new Graph();
    expect(g.nodes()).toEqual([]);
    expect(g.size).toBe(0);
  });

  it("has no edges", () => {
    /** An empty graph should report zero edges. */
    const g = new Graph();
    expect(g.edges()).toEqual([]);
  });

  it("hasNode returns false for any node", () => {
    /** hasNode should return false for any node in an empty graph. */
    const g = new Graph();
    expect(g.hasNode("A")).toBe(false);
  });

  it("hasEdge returns false for any edge", () => {
    /** hasEdge should return false for any edge in an empty graph. */
    const g = new Graph();
    expect(g.hasEdge("A", "B")).toBe(false);
  });
});

// ======================================================================
// 2. Single Node
// ======================================================================
// Adding a single node is the simplest mutation. We test that it appears
// in the graph and can be removed.

describe("Single Node", () => {
  it("addNode makes it present", () => {
    /** After addNode, the node should be findable. */
    const g = new Graph();
    g.addNode("A");
    expect(g.hasNode("A")).toBe(true);
    expect(g.size).toBe(1);
    expect(g.nodes()).toEqual(["A"]);
  });

  it("removeNode makes it absent", () => {
    /** After removeNode, the node should be gone. */
    const g = new Graph();
    g.addNode("A");
    g.removeNode("A");
    expect(g.hasNode("A")).toBe(false);
    expect(g.size).toBe(0);
  });

  it("addNode is idempotent", () => {
    /** Adding the same node twice should not create duplicates. */
    const g = new Graph();
    g.addNode("A");
    g.addNode("A"); // Should be a no-op
    expect(g.size).toBe(1);
    expect(g.nodes()).toEqual(["A"]);
  });

  it("predecessors of isolated node is empty", () => {
    /** An isolated node has no predecessors. */
    const g = new Graph();
    g.addNode("A");
    expect(g.predecessors("A")).toEqual([]);
  });

  it("successors of isolated node is empty", () => {
    /** An isolated node has no successors. */
    const g = new Graph();
    g.addNode("A");
    expect(g.successors("A")).toEqual([]);
  });
});

// ======================================================================
// 3. Single Edge
// ======================================================================
// An edge implicitly adds both endpoints, so addEdge("A", "B") creates
// nodes A and B plus the edge A -> B.

describe("Single Edge", () => {
  it("addEdge creates both nodes", () => {
    /** addEdge should implicitly add both endpoint nodes. */
    const g = new Graph();
    g.addEdge("A", "B");
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(true);
    expect(g.size).toBe(2);
  });

  it("addEdge creates the edge", () => {
    /** After addEdge, hasEdge should return true. */
    const g = new Graph();
    g.addEdge("A", "B");
    expect(g.hasEdge("A", "B")).toBe(true);
    // The reverse direction should NOT exist.
    expect(g.hasEdge("B", "A")).toBe(false);
  });

  it("edges() returns the edge", () => {
    /** edges() should include the added edge. */
    const g = new Graph();
    g.addEdge("A", "B");
    expect(g.edges()).toEqual([["A", "B"]]);
  });

  it("predecessors and successors", () => {
    /** For edge A -> B: A's successor is B, B's predecessor is A. */
    const g = new Graph();
    g.addEdge("A", "B");
    expect(g.successors("A")).toEqual(["B"]);
    expect(g.predecessors("B")).toEqual(["A"]);
    // A has no predecessors, B has no successors.
    expect(g.predecessors("A")).toEqual([]);
    expect(g.successors("B")).toEqual([]);
  });

  it("removeEdge removes edge but keeps nodes", () => {
    /** removeEdge should delete the edge but keep both nodes. */
    const g = new Graph();
    g.addEdge("A", "B");
    g.removeEdge("A", "B");
    expect(g.hasEdge("A", "B")).toBe(false);
    // Both nodes should still exist.
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(true);
  });

  it("duplicate edge is idempotent", () => {
    /** Adding the same edge twice should not create duplicates. */
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("A", "B"); // Should be a no-op
    expect(g.edges()).toEqual([["A", "B"]]);
  });
});

describe("Property Bags and Weights", () => {
  it("stores graph, node, and directed edge metadata", () => {
    const g = new Graph();

    g.setGraphProperty("name", "neural-dag");
    g.setGraphProperty("version", 1);
    expect(g.graphProperties()).toEqual({
      name: "neural-dag",
      version: 1,
    });
    g.removeGraphProperty("version");
    expect(g.graphProperties()).toEqual({ name: "neural-dag" });

    g.addNode("input", { kind: "input" });
    g.addNode("input", { slot: 0 });
    expect(g.nodeProperties("input")).toEqual({
      kind: "input",
      slot: 0,
    });

    const nodeProperties = g.nodeProperties("input");
    nodeProperties.kind = "mutated";
    expect(g.nodeProperties("input").kind).toBe("input");

    g.addEdge("input", "sum", 0.5, { trainable: true });
    expect(g.edgeProperties("input", "sum")).toEqual({
      trainable: true,
      weight: 0.5,
    });
    expect(g.edgeWeight("input", "sum")).toBe(0.5);
    expect(g.edgesWeighted()).toEqual([["input", "sum", 0.5]]);

    g.setEdgeProperty("input", "sum", "weight", 0.75);
    expect(g.edgeWeight("input", "sum")).toBe(0.75);
    expect(g.successorsWeighted("input").get("sum")).toBe(0.75);

    g.removeEdgeProperty("input", "sum", "trainable");
    expect(g.edgeProperties("input", "sum")).toEqual({ weight: 0.75 });
  });

  it("keeps reverse edge properties independent", () => {
    const g = new Graph();

    g.addEdge("A", "B", 2, { role: "forward" });
    g.addEdge("B", "A", 3, { role: "reverse" });

    expect(g.edgeProperties("A", "B")).toEqual({
      role: "forward",
      weight: 2,
    });
    expect(g.edgeProperties("B", "A")).toEqual({
      role: "reverse",
      weight: 3,
    });

    g.setEdgeProperty("B", "A", "weight", 4);
    expect(g.edgeWeight("A", "B")).toBe(2);
    expect(g.edgeWeight("B", "A")).toBe(4);
  });

  it("removes properties with their graph structure", () => {
    const g = new Graph();

    g.addNode("A", { kind: "input" });
    g.addEdge("A", "B", 1.5, { trainable: true });

    g.removeEdge("A", "B");
    expect(() => g.edgeProperties("A", "B")).toThrow(EdgeNotFoundError);

    g.addEdge("A", "B", 1.5, { trainable: true });
    g.removeNode("A");
    expect(() => g.nodeProperties("A")).toThrow(NodeNotFoundError);
    expect(() => g.edgeProperties("A", "B")).toThrow(EdgeNotFoundError);
  });
});

// ======================================================================
// 4. Multi-Node Operations
// ======================================================================
// Test more complex graph structures to make sure the adjacency maps
// stay consistent.

describe("Multi-Node Operations", () => {
  it("removeNode cleans up edges", () => {
    /**
     * Removing a node should remove all its incoming and outgoing edges.
     *
     * Given A -> B -> C, removing B should leave A and C as isolated nodes
     * with no edges between them.
     */
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.removeNode("B");

    expect(g.hasNode("B")).toBe(false);
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("C")).toBe(true);
    expect(g.edges()).toEqual([]);
    expect(g.successors("A")).toEqual([]);
    expect(g.predecessors("C")).toEqual([]);
  });

  it("removeNode with multiple edges", () => {
    /**
     * Removing a hub node should clean up all connected edges.
     *
     * Given A -> B, C -> B, B -> D, removing B should leave A, C, D
     * with no edges.
     */
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("C", "B");
    g.addEdge("B", "D");
    g.removeNode("B");

    expect(g.size).toBe(3);
    expect(g.edges()).toEqual([]);
  });
});

// ======================================================================
// 5. Error Conditions
// ======================================================================
// These tests verify that the graph throws the right errors for
// invalid operations.

describe("Self-Loop Flag", () => {
  it("default Graph rejects self-loops", () => {
    /** By default, self-loops should be rejected. */
    const g = new Graph();
    expect(() => g.addEdge("A", "A")).toThrow("Self-loops are not allowed");
  });

  it("allowSelfLoops property defaults to false", () => {
    /** The allowSelfLoops getter should report the flag value. */
    const g = new Graph();
    expect(g.allowSelfLoops).toBe(false);
  });

  it("allowSelfLoops: true permits self-loops", () => {
    /** When the flag is true, self-loops should be accepted. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    expect(g.hasEdge("A", "A")).toBe(true);
    expect(g.allowSelfLoops).toBe(true);
  });

  it("self-loop node is own successor and predecessor", () => {
    /** A node with a self-loop should appear in its own neighbor lists. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    expect(g.successors("A")).toContain("A");
    expect(g.predecessors("A")).toContain("A");
  });

  it("self-loop appears in edges list", () => {
    /** Self-loop should be listed as an edge. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    expect(g.edges()).toEqual([["A", "A"]]);
  });

  it("self-loop with other edges", () => {
    /** Self-loop should coexist with normal edges. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.addEdge("A", "B");
    expect(g.hasEdge("A", "A")).toBe(true);
    expect(g.hasEdge("A", "B")).toBe(true);
    expect(g.size).toBe(2);
  });

  it("remove self-loop keeps node", () => {
    /** Removing a self-loop should keep the node. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.removeEdge("A", "A");
    expect(g.hasEdge("A", "A")).toBe(false);
    expect(g.hasNode("A")).toBe(true);
  });

  it("remove node with self-loop", () => {
    /** Removing a node should clean up its self-loop. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.addEdge("A", "B");
    g.removeNode("A");
    expect(g.hasNode("A")).toBe(false);
    expect(g.hasNode("B")).toBe(true);
    expect(g.edges()).toEqual([]);
  });

  it("duplicate self-loop is idempotent", () => {
    /** Adding the same self-loop twice should be a no-op. */
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.addEdge("A", "A");
    expect(g.edges()).toEqual([["A", "A"]]);
  });
});

describe("Error Conditions", () => {
  it("self-loop throws Error", () => {
    /** A self-loop (A -> A) should throw Error. */
    const g = new Graph();
    expect(() => g.addEdge("A", "A")).toThrow("Self-loops are not allowed");
  });

  it("remove nonexistent node throws NodeNotFoundError", () => {
    /** Removing a node that doesn't exist should throw NodeNotFoundError. */
    const g = new Graph();
    expect(() => g.removeNode("X")).toThrow(NodeNotFoundError);
  });

  it("NodeNotFoundError has node property", () => {
    /** NodeNotFoundError should carry the missing node value. */
    const g = new Graph();
    try {
      g.removeNode("X");
    } catch (e) {
      expect(e).toBeInstanceOf(NodeNotFoundError);
      expect((e as NodeNotFoundError).node).toBe("X");
    }
  });

  it("remove nonexistent edge throws EdgeNotFoundError", () => {
    /** Removing an edge that doesn't exist should throw EdgeNotFoundError. */
    const g = new Graph();
    g.addNode("A");
    g.addNode("B");
    expect(() => g.removeEdge("A", "B")).toThrow(EdgeNotFoundError);
  });

  it("EdgeNotFoundError has fromNode and toNode properties", () => {
    /** EdgeNotFoundError should carry both node values. */
    const g = new Graph();
    try {
      g.removeEdge("X", "Y");
    } catch (e) {
      expect(e).toBeInstanceOf(EdgeNotFoundError);
      expect((e as EdgeNotFoundError).fromNode).toBe("X");
      expect((e as EdgeNotFoundError).toNode).toBe("Y");
    }
  });

  it("predecessors of nonexistent node throws", () => {
    /** predecessors() should throw NodeNotFoundError for missing nodes. */
    const g = new Graph();
    expect(() => g.predecessors("X")).toThrow(NodeNotFoundError);
  });

  it("successors of nonexistent node throws", () => {
    /** successors() should throw NodeNotFoundError for missing nodes. */
    const g = new Graph();
    expect(() => g.successors("X")).toThrow(NodeNotFoundError);
  });
});

// ======================================================================
// 6. Edge Cases and toString
// ======================================================================

describe("Edge Cases", () => {
  it("toString shows counts", () => {
    /** toString should show node and edge counts. */
    const g = new Graph();
    g.addEdge("A", "B");
    expect(g.toString()).toBe("Graph(nodes=2, edges=1)");
  });
});

// ======================================================================
// 7. Topological Sort
// ======================================================================
// Kahn's algorithm should produce a valid ordering where every edge goes
// from earlier to later in the sequence.

describe("Topological Sort", () => {
  it("empty graph", () => {
    /** Topological sort of an empty graph is an empty list. */
    const g = new Graph();
    expect(g.topologicalSort()).toEqual([]);
  });

  it("single node", () => {
    /** A single node topo-sorts to a one-element list. */
    const g = new Graph();
    g.addNode("A");
    expect(g.topologicalSort()).toEqual(["A"]);
  });

  it("linear chain", () => {
    /**
     * A -> B -> C -> D must sort to exactly [A, B, C, D].
     *
     * In a linear chain, there's only one valid topological order.
     */
    const g = makeLinearChain();
    expect(g.topologicalSort()).toEqual(["A", "B", "C", "D"]);
  });

  it("diamond", () => {
    /**
     * The diamond A->{B,C}->D has multiple valid orderings.
     *
     * A must come first, D must come last. B and C can be in either order.
     * Our implementation sorts ties alphabetically, so we expect [A, B, C, D].
     */
    const g = makeDiamond();
    const result = g.topologicalSort();
    expect(result[0]).toBe("A");
    expect(result[result.length - 1]).toBe("D");
    expect(new Set(result.slice(1, 3))).toEqual(new Set(["B", "C"]));
  });

  it("cycle raises CycleError", () => {
    /** A graph with a cycle should throw CycleError on topological sort. */
    const g = makeCycle();
    try {
      g.topologicalSort();
      expect.unreachable("Should have thrown CycleError");
    } catch (e) {
      expect(e).toBeInstanceOf(CycleError);
      const ce = e as CycleError;
      // The error should include the cycle path.
      expect(ce.cycle.length).toBeGreaterThanOrEqual(3);
      // The first and last element of the cycle should be the same node.
      expect(ce.cycle[0]).toBe(ce.cycle[ce.cycle.length - 1]);
    }
  });

  it("disconnected components", () => {
    /**
     * Topological sort should handle disconnected components.
     *
     * If the graph has two separate chains X->Y and A->B, the sort should
     * include all four nodes in a valid order.
     */
    const g = new Graph();
    g.addEdge("X", "Y");
    g.addEdge("A", "B");
    const result = g.topologicalSort();
    expect(result.length).toBe(4);
    // X must come before Y, A must come before B.
    expect(result.indexOf("X")).toBeLessThan(result.indexOf("Y"));
    expect(result.indexOf("A")).toBeLessThan(result.indexOf("B"));
  });
});

// ======================================================================
// 8. Cycle Detection
// ======================================================================

describe("Cycle Detection", () => {
  it("empty graph has no cycle", () => {
    /** An empty graph has no cycle. */
    const g = new Graph();
    expect(g.hasCycle()).toBe(false);
  });

  it("linear chain has no cycle", () => {
    /** A linear chain is a DAG -- no cycles. */
    const g = makeLinearChain();
    expect(g.hasCycle()).toBe(false);
  });

  it("diamond has no cycle", () => {
    /** A diamond is a DAG -- no cycles. */
    const g = makeDiamond();
    expect(g.hasCycle()).toBe(false);
  });

  it("three-node cycle", () => {
    /** A -> B -> C -> A is a cycle. */
    const g = makeCycle();
    expect(g.hasCycle()).toBe(true);
  });

  it("cycle with tail", () => {
    /**
     * A graph with a cycle and a non-cyclic tail.
     *
     * X -> A -> B -> C -> A. The cycle is A->B->C->A, and X is a tail
     * leading into it.
     */
    const g = new Graph();
    g.addEdge("X", "A");
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "A");
    expect(g.hasCycle()).toBe(true);
  });
});

// ======================================================================
// 9. Transitive Closure
// ======================================================================
// transitiveClosure(node) returns all nodes reachable downstream.

describe("Transitive Closure", () => {
  it("linear chain from root", () => {
    /** From A in A->B->C->D, everything downstream is reachable. */
    const g = makeLinearChain();
    expect(g.transitiveClosure("A")).toEqual(new Set(["B", "C", "D"]));
  });

  it("linear chain from middle", () => {
    /** From B in A->B->C->D, only C and D are reachable. */
    const g = makeLinearChain();
    expect(g.transitiveClosure("B")).toEqual(new Set(["C", "D"]));
  });

  it("linear chain from leaf", () => {
    /** From D (a leaf), nothing is reachable. */
    const g = makeLinearChain();
    expect(g.transitiveClosure("D")).toEqual(new Set());
  });

  it("diamond from root", () => {
    /** From A in the diamond, B, C, and D are all reachable. */
    const g = makeDiamond();
    expect(g.transitiveClosure("A")).toEqual(new Set(["B", "C", "D"]));
  });

  it("diamond from middle", () => {
    /** From B in the diamond, only D is reachable. */
    const g = makeDiamond();
    expect(g.transitiveClosure("B")).toEqual(new Set(["D"]));
  });

  it("nonexistent node throws", () => {
    /** transitiveClosure on a missing node should throw. */
    const g = new Graph();
    expect(() => g.transitiveClosure("X")).toThrow(NodeNotFoundError);
  });

  it("isolated node has empty closure", () => {
    /** An isolated node has empty transitive closure. */
    const g = new Graph();
    g.addNode("A");
    expect(g.transitiveClosure("A")).toEqual(new Set());
  });
});

// ======================================================================
// 10. Transitive Dependents
// ======================================================================
// transitiveDependents(node) returns all nodes that depend on this node
// (walking edges backwards).

describe("Transitive Dependents", () => {
  it("linear chain from leaf", () => {
    /**
     * From D in A->B->C->D, everything upstream depends on D.
     *
     * D is the leaf. Nodes that depend on D are the ones that
     * point TO D, which is C. And C's dependents are B, and B's are A.
     * So transitiveDependents("D") = {A, B, C}.
     */
    const g = makeLinearChain();
    expect(g.transitiveDependents("D")).toEqual(new Set(["A", "B", "C"]));
  });

  it("linear chain from root", () => {
    /** From A (the root), nothing depends on A -- it has no predecessors. */
    const g = makeLinearChain();
    expect(g.transitiveDependents("A")).toEqual(new Set());
  });

  it("diamond from D", () => {
    /** From D in the diamond, A, B, and C all transitively depend on D. */
    const g = makeDiamond();
    expect(g.transitiveDependents("D")).toEqual(new Set(["A", "B", "C"]));
  });

  it("diamond from B", () => {
    /** From B in the diamond, only A depends on B. */
    const g = makeDiamond();
    expect(g.transitiveDependents("B")).toEqual(new Set(["A"]));
  });

  it("nonexistent node throws", () => {
    /** transitiveDependents on a missing node should throw. */
    const g = new Graph();
    expect(() => g.transitiveDependents("X")).toThrow(NodeNotFoundError);
  });
});

// ======================================================================
// 11. Independent Groups
// ======================================================================
// independentGroups partitions nodes into topological levels. Nodes at
// the same level can run in parallel.

describe("Independent Groups", () => {
  it("empty graph", () => {
    /** An empty graph has no groups. */
    const g = new Graph();
    expect(g.independentGroups()).toEqual([]);
  });

  it("single node", () => {
    /** A single node forms one group. */
    const g = new Graph();
    g.addNode("A");
    expect(g.independentGroups()).toEqual([["A"]]);
  });

  it("linear chain", () => {
    /**
     * A -> B -> C -> D has four levels, each with one node.
     *
     * No parallelism is possible because each node depends on the previous.
     */
    const g = makeLinearChain();
    expect(g.independentGroups()).toEqual([["A"], ["B"], ["C"], ["D"]]);
  });

  it("diamond has parallel middle", () => {
    /**
     * The diamond A->{B,C}->D should have B and C at the same level.
     *
     * Level 0: [A]      (no dependencies)
     * Level 1: [B, C]   (both depend only on A)
     * Level 2: [D]      (depends on both B and C)
     */
    const g = makeDiamond();
    const groups = g.independentGroups();
    expect(groups.length).toBe(3);
    expect(groups[0]).toEqual(["A"]);
    expect([...groups[1]].sort()).toEqual(["B", "C"]);
    expect(groups[2]).toEqual(["D"]);
  });

  it("two independent chains", () => {
    /**
     * Two disconnected chains should interleave at each level.
     *
     * A -> B and X -> Y should give:
     * Level 0: [A, X]   (both are roots)
     * Level 1: [B, Y]   (both depend on level-0 nodes)
     */
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("X", "Y");
    const groups = g.independentGroups();
    expect(groups.length).toBe(2);
    expect([...groups[0]].sort()).toEqual(["A", "X"]);
    expect([...groups[1]].sort()).toEqual(["B", "Y"]);
  });

  it("cycle raises CycleError", () => {
    /** independentGroups should throw CycleError on cyclic graphs. */
    const g = makeCycle();
    expect(() => g.independentGroups()).toThrow(CycleError);
  });

  it("wide graph", () => {
    /**
     * A graph where a root fans out to many children.
     *
     * ROOT -> {A, B, C, D, E}
     *
     * Level 0: [ROOT]
     * Level 1: [A, B, C, D, E]
     */
    const g = new Graph();
    for (const child of ["A", "B", "C", "D", "E"]) {
      g.addEdge("ROOT", child);
    }
    const groups = g.independentGroups();
    expect(groups.length).toBe(2);
    expect(groups[0]).toEqual(["ROOT"]);
    expect([...groups[1]].sort()).toEqual(["A", "B", "C", "D", "E"]);
  });
});

// ======================================================================
// 12. Affected Nodes
// ======================================================================
// affectedNodes(changed) = changed + all their transitive dependents.

describe("Affected Nodes", () => {
  it("change leaf affects everything upstream", () => {
    /**
     * Changing D in A->B->C->D affects all nodes.
     *
     * Our edge convention means the dependents of D walk backwards
     * through the reverse map: C depends on D, B depends on C, A on B.
     * So affectedNodes({D}) = {A, B, C, D}.
     */
    const g = makeLinearChain();
    expect(g.affectedNodes(new Set(["D"]))).toEqual(
      new Set(["A", "B", "C", "D"])
    );
  });

  it("change root affects only root", () => {
    /**
     * Changing A in A->B->C->D affects only A.
     *
     * A has no predecessors, so nothing depends on A.
     */
    const g = makeLinearChain();
    expect(g.affectedNodes(new Set(["A"]))).toEqual(new Set(["A"]));
  });

  it("change D in diamond", () => {
    /**
     * Changing D in the diamond affects A, B, C, and D.
     *
     * D has A, B, C as transitive dependents.
     */
    const g = makeDiamond();
    expect(g.affectedNodes(new Set(["D"]))).toEqual(
      new Set(["A", "B", "C", "D"])
    );
  });

  it("change A in diamond", () => {
    /**
     * Changing A in the diamond affects only A.
     *
     * A is the root; nothing depends on it.
     */
    const g = makeDiamond();
    expect(g.affectedNodes(new Set(["A"]))).toEqual(new Set(["A"]));
  });

  it("change multiple nodes", () => {
    /**
     * Changing B and C in the diamond affects A, B, C.
     *
     * B's dependents are {A}, C's dependents are {A}.
     * So affected = {B, C, A}.
     */
    const g = makeDiamond();
    expect(g.affectedNodes(new Set(["B", "C"]))).toEqual(
      new Set(["A", "B", "C"])
    );
  });

  it("change nonexistent node is ignored", () => {
    /** Nodes not in the graph should be silently ignored. */
    const g = makeDiamond();
    expect(g.affectedNodes(new Set(["Z"]))).toEqual(new Set());
  });

  it("mixed existing and nonexistent", () => {
    /** A mix of real and fake nodes should include only the real ones. */
    const g = makeDiamond();
    const result = g.affectedNodes(new Set(["A", "Z"]));
    expect(result.has("A")).toBe(true);
    expect(result.has("Z")).toBe(false);
  });
});

// ======================================================================
// 13. Real Repo Graph (21 Packages)
// ======================================================================
// This test models the actual dependency graph of the packages in the
// coding-adventures repository. It serves as an integration test to make
// sure all the algorithms work together on a realistic graph.

describe("Real Repo Graph", () => {
  /**
   * Build the actual dependency graph for the repository.
   *
   * The packages and their dependencies (A -> B means A depends on B):
   *
   * Layer 1: logic-gates (no deps)
   * Layer 2: arithmetic (depends on logic-gates)
   * Layer 3: grammar-tools (no deps)
   * Layer 4: lexer (depends on grammar-tools)
   * Layer 5: parser (depends on lexer, grammar-tools)
   * Layer 6: cpu-simulator (depends on arithmetic, logic-gates)
   *          intel4004-simulator (depends on arithmetic)
   *          pipeline (depends on parser, lexer)
   * Layer 7: assembler (depends on parser, grammar-tools)
   *          virtual-machine (depends on cpu-simulator)
   *          arm-simulator (depends on cpu-simulator, assembler)
   * Layer 8: jvm-simulator (depends on virtual-machine)
   *          clr-simulator (depends on virtual-machine)
   *          wasm-simulator (depends on virtual-machine)
   *          riscv-simulator (depends on cpu-simulator)
   * Layer 9: bytecode-compiler (depends on jvm-simulator, clr-simulator,
   *                             wasm-simulator, parser)
   *          html-renderer (depends on parser, lexer)
   *          jit-compiler (depends on virtual-machine, assembler)
   */
  function makeRepoGraph(): Graph {
    const g = new Graph();

    // Layer 1 -> 2
    g.addEdge("arithmetic", "logic-gates");

    // Layer 3 -> 4
    g.addEdge("lexer", "grammar-tools");

    // Layer 4 -> 5
    g.addEdge("parser", "lexer");
    g.addEdge("parser", "grammar-tools");

    // Layer 5 -> 6
    g.addEdge("cpu-simulator", "arithmetic");
    g.addEdge("cpu-simulator", "logic-gates");
    g.addEdge("intel4004-simulator", "arithmetic");
    g.addEdge("pipeline", "parser");
    g.addEdge("pipeline", "lexer");

    // Layer 6 -> 7
    g.addEdge("assembler", "parser");
    g.addEdge("assembler", "grammar-tools");
    g.addEdge("virtual-machine", "cpu-simulator");
    g.addEdge("arm-simulator", "cpu-simulator");
    g.addEdge("arm-simulator", "assembler");

    // Layer 7 -> 8
    g.addEdge("jvm-simulator", "virtual-machine");
    g.addEdge("clr-simulator", "virtual-machine");
    g.addEdge("wasm-simulator", "virtual-machine");
    g.addEdge("riscv-simulator", "cpu-simulator");

    // Layer 8 -> 9
    g.addEdge("bytecode-compiler", "jvm-simulator");
    g.addEdge("bytecode-compiler", "clr-simulator");
    g.addEdge("bytecode-compiler", "wasm-simulator");
    g.addEdge("bytecode-compiler", "parser");
    g.addEdge("html-renderer", "parser");
    g.addEdge("html-renderer", "lexer");
    g.addEdge("jit-compiler", "virtual-machine");
    g.addEdge("jit-compiler", "assembler");

    // Ruby-related packages
    g.addEdge("ruby-lexer", "grammar-tools");
    g.addEdge("ruby-parser", "ruby-lexer");
    g.addEdge("ruby-parser", "grammar-tools");

    // directed-graph has no dependencies on other packages
    g.addNode("directed-graph");

    return g;
  }

  it("has 21 packages", () => {
    /** The repository has 21 packages. */
    const g = makeRepoGraph();
    expect(g.size).toBe(21);
  });

  it("is acyclic", () => {
    /** The repo dependency graph should be a DAG. */
    const g = makeRepoGraph();
    expect(g.hasCycle()).toBe(false);
  });

  it("topological sort includes all packages", () => {
    /**
     * Topological sort should produce a valid ordering.
     *
     * We verify that for every edge (A -> B), A appears before B in the order.
     */
    const g = makeRepoGraph();
    const order = g.topologicalSort();
    expect(order.length).toBe(21);

    const position = new Map<string, number>();
    order.forEach((node, i) => position.set(node, i));

    for (const [fromNode, toNode] of g.edges()) {
      expect(position.get(fromNode)!).toBeLessThan(position.get(toNode)!);
    }
  });

  it("independent groups partition all packages", () => {
    /**
     * Independent groups should partition all 21 packages.
     *
     * The sum of all group sizes should be 21.
     */
    const g = makeRepoGraph();
    const groups = g.independentGroups();

    const allNodes = groups.flat();
    expect(allNodes.length).toBe(21);
    expect(new Set(allNodes)).toEqual(new Set(g.nodes()));

    // directed-graph has no dependencies, so it should be in the first
    // group (among the leaf consumers / items with no in-edges pointing to them).
    expect(groups[0]).toContain("directed-graph");

    // The last group should contain foundational packages.
    const lastGroup = groups[groups.length - 1];
    expect(
      lastGroup.includes("logic-gates") || lastGroup.includes("grammar-tools")
    ).toBe(true);
  });

  it("transitive closure of logic-gates is empty", () => {
    /** logic-gates has no dependencies, so its transitive closure is empty. */
    const g = makeRepoGraph();
    expect(g.transitiveClosure("logic-gates")).toEqual(new Set());
  });

  it("transitive dependents of logic-gates", () => {
    /**
     * Changing logic-gates should affect arithmetic and everything above.
     *
     * logic-gates is at the bottom. arithmetic depends on it, cpu-simulator
     * depends on arithmetic, etc.
     */
    const g = makeRepoGraph();
    const dependents = g.transitiveDependents("logic-gates");
    expect(dependents.has("arithmetic")).toBe(true);
    expect(dependents.has("cpu-simulator")).toBe(true);
    expect(dependents.has("virtual-machine")).toBe(true);
  });

  it("affected by grammar-tools change", () => {
    /** Changing grammar-tools should affect lexer, parser, and everything above. */
    const g = makeRepoGraph();
    const affected = g.affectedNodes(new Set(["grammar-tools"]));
    expect(affected.has("grammar-tools")).toBe(true);
    expect(affected.has("lexer")).toBe(true);
    expect(affected.has("parser")).toBe(true);
    expect(affected.has("assembler")).toBe(true);
    expect(affected.has("pipeline")).toBe(true);
  });

  it("affected by leaf change", () => {
    /**
     * Changing bytecode-compiler (a leaf) affects only itself.
     *
     * bytecode-compiler has no packages that depend on it.
     */
    const g = makeRepoGraph();
    const affected = g.affectedNodes(new Set(["bytecode-compiler"]));
    expect(affected).toEqual(new Set(["bytecode-compiler"]));
  });
});
