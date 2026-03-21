/**
 * graph.test.ts -- Tests for the native (Rust-backed) directed graph
 * ===================================================================
 *
 * These tests mirror the Python directed_graph_native test suite to ensure
 * the napi-rs extension provides identical behavior to the PyO3 extension.
 * If these tests pass, the native extension is a valid drop-in replacement
 * for the pure TypeScript directed-graph package.
 *
 * The tests are organized from simplest to most complex:
 *
 * 1. Node operations (add, remove, has, nodes, len)
 * 2. Edge operations (add, remove, has, edges, self-loops)
 * 3. Neighbor queries (predecessors, successors)
 * 4. Topological sort
 * 5. Cycle detection
 * 6. Transitive closure
 * 7. Affected nodes
 * 8. Independent groups
 * 9. String representation
 * 10. Real repo graph integration test
 *
 * Error handling:
 * - The Rust wrapper throws plain Error objects with descriptive messages.
 * - Error messages start with a type prefix: "CycleError:", "NodeNotFoundError:",
 *   "EdgeNotFoundError:", or "SelfLoopError:".
 * - Tests match on these prefixes to verify the correct error type.
 */

import { describe, it, expect } from "vitest";

// The native addon is loaded from the compiled .node file.
// The index.js ESM entry point uses createRequire to load the .node binary.
import { DirectedGraph } from "../index.js";

// ======================================================================
// Helper: error message matching
// ======================================================================
//
// Since napi-rs throws plain Error objects (not custom subclasses), we
// match on the error message prefix to verify the error type.

function expectErrorContaining(fn: () => void, substring: string): void {
  try {
    fn();
    expect.unreachable(`Expected error containing "${substring}"`);
  } catch (e: any) {
    expect(e.message).toContain(substring);
  }
}

// ======================================================================
// 1. Node Operations
// ======================================================================

describe("Node Operations", () => {
  it("add and has node", () => {
    /** After addNode, hasNode should return true for that node. */
    const g = new DirectedGraph();
    g.addNode("A");
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(false);
  });

  it("add duplicate node is noop", () => {
    /** Adding the same node twice should not create duplicates. */
    const g = new DirectedGraph();
    g.addNode("A");
    g.addNode("A");
    expect(g.size()).toBe(1);
  });

  it("remove node", () => {
    /** After removeNode, hasNode should return false. */
    const g = new DirectedGraph();
    g.addNode("A");
    g.removeNode("A");
    expect(g.hasNode("A")).toBe(false);
  });

  it("remove nonexistent node throws", () => {
    /** Removing a node that doesn't exist should throw NodeNotFoundError. */
    const g = new DirectedGraph();
    expectErrorContaining(() => g.removeNode("X"), "NodeNotFoundError");
  });

  it("remove node cleans up edges", () => {
    /** Removing a node should remove all its incoming and outgoing edges. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.removeNode("B");
    expect(g.hasEdge("A", "B")).toBe(false);
    expect(g.hasEdge("B", "C")).toBe(false);
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("C")).toBe(true);
  });

  it("nodes returns sorted list", () => {
    /** nodes() should return all nodes in alphabetical order. */
    const g = new DirectedGraph();
    g.addNode("C");
    g.addNode("A");
    g.addNode("B");
    expect(g.nodes()).toEqual(["A", "B", "C"]);
  });

  it("size reflects node count", () => {
    /** size() should track the number of nodes. */
    const g = new DirectedGraph();
    expect(g.size()).toBe(0);
    g.addNode("A");
    expect(g.size()).toBe(1);
    g.addNode("B");
    expect(g.size()).toBe(2);
  });
});

// ======================================================================
// 2. Edge Operations
// ======================================================================

describe("Edge Operations", () => {
  it("add and has edge", () => {
    /** After addEdge, hasEdge should return true for that direction only. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    expect(g.hasEdge("A", "B")).toBe(true);
    expect(g.hasEdge("B", "A")).toBe(false); // directed!
  });

  it("add edge creates nodes", () => {
    /** addEdge should implicitly create both endpoint nodes. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(true);
  });

  it("remove edge", () => {
    /** After removeEdge, hasEdge should return false but nodes remain. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.removeEdge("A", "B");
    expect(g.hasEdge("A", "B")).toBe(false);
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(true);
  });

  it("remove nonexistent edge throws", () => {
    /** Removing an edge that doesn't exist should throw EdgeNotFoundError. */
    const g = new DirectedGraph();
    g.addNode("A");
    g.addNode("B");
    expectErrorContaining(() => g.removeEdge("A", "B"), "EdgeNotFoundError");
  });

  it("self loop throws", () => {
    /** Self-loops (A -> A) should be rejected with SelfLoopError. */
    const g = new DirectedGraph();
    expectErrorContaining(() => g.addEdge("A", "A"), "SelfLoopError");
  });

  it("edges returns sorted list", () => {
    /** edges() should return all edges sorted lexicographically. */
    const g = new DirectedGraph();
    g.addEdge("B", "C");
    g.addEdge("A", "B");
    const edges = g.edges();
    expect(edges).toEqual([["A", "B"], ["B", "C"]]);
  });

  it("duplicate edge is idempotent", () => {
    /** Adding the same edge twice should not create duplicates. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("A", "B");
    expect(g.edges()).toEqual([["A", "B"]]);
  });
});

// ======================================================================
// 3. Neighbor Queries
// ======================================================================

describe("Neighbor Queries", () => {
  it("predecessors", () => {
    /** predecessors returns nodes that point TO the given node. */
    const g = new DirectedGraph();
    g.addEdge("A", "C");
    g.addEdge("B", "C");
    const preds = g.predecessors("C");
    expect(preds.sort()).toEqual(["A", "B"]);
  });

  it("successors", () => {
    /** successors returns nodes that the given node points TO. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    const succs = g.successors("A");
    expect(succs.sort()).toEqual(["B", "C"]);
  });

  it("predecessors of nonexistent node throws", () => {
    /** predecessors on a missing node should throw NodeNotFoundError. */
    const g = new DirectedGraph();
    expectErrorContaining(() => g.predecessors("X"), "NodeNotFoundError");
  });

  it("successors of nonexistent node throws", () => {
    /** successors on a missing node should throw NodeNotFoundError. */
    const g = new DirectedGraph();
    expectErrorContaining(() => g.successors("X"), "NodeNotFoundError");
  });

  it("predecessors of isolated node is empty", () => {
    /** An isolated node has no predecessors. */
    const g = new DirectedGraph();
    g.addNode("A");
    expect(g.predecessors("A")).toEqual([]);
  });

  it("successors of isolated node is empty", () => {
    /** An isolated node has no successors. */
    const g = new DirectedGraph();
    g.addNode("A");
    expect(g.successors("A")).toEqual([]);
  });
});

// ======================================================================
// 4. Topological Sort
// ======================================================================

describe("Topological Sort", () => {
  it("linear chain", () => {
    /** A -> B -> C must sort to [A, B, C]. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    expect(g.topologicalSort()).toEqual(["A", "B", "C"]);
  });

  it("diamond", () => {
    /** The diamond A->{B,C}->D: A first, D last, B and C in middle. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    g.addEdge("B", "D");
    g.addEdge("C", "D");
    const order = g.topologicalSort();
    expect(order[0]).toBe("A");
    expect(order[order.length - 1]).toBe("D");
    expect(new Set(order.slice(1, 3))).toEqual(new Set(["B", "C"]));
  });

  it("cycle throws CycleError", () => {
    /** A graph with a cycle should throw CycleError. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "A");
    expectErrorContaining(() => g.topologicalSort(), "CycleError");
  });

  it("empty graph", () => {
    /** Topological sort of an empty graph is an empty list. */
    const g = new DirectedGraph();
    expect(g.topologicalSort()).toEqual([]);
  });

  it("single node", () => {
    /** A single node topo-sorts to a one-element list. */
    const g = new DirectedGraph();
    g.addNode("A");
    expect(g.topologicalSort()).toEqual(["A"]);
  });

  it("disconnected components", () => {
    /** Disconnected components should all appear in the sort. */
    const g = new DirectedGraph();
    g.addEdge("X", "Y");
    g.addEdge("A", "B");
    const result = g.topologicalSort();
    expect(result.length).toBe(4);
    expect(result.indexOf("X")).toBeLessThan(result.indexOf("Y"));
    expect(result.indexOf("A")).toBeLessThan(result.indexOf("B"));
  });
});

// ======================================================================
// 5. Cycle Detection
// ======================================================================

describe("Cycle Detection", () => {
  it("no cycle in linear chain", () => {
    /** A linear chain is a DAG -- no cycles. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    expect(g.hasCycle()).toBe(false);
  });

  it("has cycle in two-node cycle", () => {
    /** A -> B -> A is a cycle. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "A");
    expect(g.hasCycle()).toBe(true);
  });

  it("empty graph has no cycle", () => {
    /** An empty graph trivially has no cycle. */
    const g = new DirectedGraph();
    expect(g.hasCycle()).toBe(false);
  });

  it("three-node cycle", () => {
    /** A -> B -> C -> A is a cycle. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "A");
    expect(g.hasCycle()).toBe(true);
  });
});

// ======================================================================
// 6. Transitive Closure
// ======================================================================

describe("Transitive Closure", () => {
  it("linear chain", () => {
    /** From A in A->B->C, reachable nodes are B and C. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    const closure = g.transitiveClosure("A");
    expect(new Set(closure)).toEqual(new Set(["B", "C"]));
  });

  it("diamond", () => {
    /** From A in the diamond, all of B, C, D are reachable. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    g.addEdge("B", "D");
    g.addEdge("C", "D");
    const closure = g.transitiveClosure("A");
    expect(new Set(closure)).toEqual(new Set(["B", "C", "D"]));
  });

  it("leaf node has empty closure", () => {
    /** A leaf node has no downstream nodes. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    expect(g.transitiveClosure("B")).toEqual([]);
  });

  it("nonexistent node throws", () => {
    /** transitiveClosure on a missing node should throw. */
    const g = new DirectedGraph();
    expectErrorContaining(() => g.transitiveClosure("X"), "NodeNotFoundError");
  });

  it("isolated node has empty closure", () => {
    /** An isolated node has empty transitive closure. */
    const g = new DirectedGraph();
    g.addNode("A");
    expect(g.transitiveClosure("A")).toEqual([]);
  });
});

// ======================================================================
// 7. Affected Nodes
// ======================================================================

describe("Affected Nodes", () => {
  it("single change propagates", () => {
    /** Changing A in A->B->C should affect A, B, C. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    const affected = new Set(g.affectedNodes(["A"]));
    expect(affected).toEqual(new Set(["A", "B", "C"]));
  });

  it("leaf change affects only itself", () => {
    /** Changing a leaf node affects only that node. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    const affected = new Set(g.affectedNodes(["B"]));
    expect(affected).toEqual(new Set(["B"]));
  });

  it("diamond change at root", () => {
    /** Changing A in the diamond affects all four nodes. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    g.addEdge("B", "D");
    g.addEdge("C", "D");
    const affected = new Set(g.affectedNodes(["A"]));
    expect(affected).toEqual(new Set(["A", "B", "C", "D"]));
  });

  it("unknown nodes are ignored", () => {
    /** Nodes not in the graph should be silently ignored. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    const affected = g.affectedNodes(["X"]);
    expect(affected).not.toContain("X");
  });

  it("mixed known and unknown nodes", () => {
    /** Only known nodes should appear in the result. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    const affected = new Set(g.affectedNodes(["A", "X"]));
    expect(affected.has("A")).toBe(true);
    expect(affected.has("X")).toBe(false);
  });
});

// ======================================================================
// 8. Independent Groups
// ======================================================================

describe("Independent Groups", () => {
  it("linear chain", () => {
    /** A -> B -> C should be three separate levels. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    expect(g.independentGroups()).toEqual([["A"], ["B"], ["C"]]);
  });

  it("diamond", () => {
    /** Diamond should have B and C in the same parallel level. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("A", "C");
    g.addEdge("B", "D");
    g.addEdge("C", "D");
    const groups = g.independentGroups();
    expect(groups.length).toBe(3);
    expect(groups[0]).toEqual(["A"]);
    expect([...groups[1]].sort()).toEqual(["B", "C"]);
    expect(groups[2]).toEqual(["D"]);
  });

  it("parallel roots", () => {
    /** Three isolated nodes should all be in one group. */
    const g = new DirectedGraph();
    g.addNode("A");
    g.addNode("B");
    g.addNode("C");
    const groups = g.independentGroups();
    expect(groups.length).toBe(1);
    expect([...groups[0]].sort()).toEqual(["A", "B", "C"]);
  });

  it("cycle throws CycleError", () => {
    /** independentGroups should throw CycleError on cyclic graphs. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("B", "A");
    expectErrorContaining(() => g.independentGroups(), "CycleError");
  });

  it("empty graph", () => {
    /** An empty graph should return no groups. */
    const g = new DirectedGraph();
    expect(g.independentGroups()).toEqual([]);
  });

  it("two independent chains", () => {
    /** Two disconnected chains should interleave at each level. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    g.addEdge("X", "Y");
    const groups = g.independentGroups();
    expect(groups.length).toBe(2);
    expect([...groups[0]].sort()).toEqual(["A", "X"]);
    expect([...groups[1]].sort()).toEqual(["B", "Y"]);
  });

  it("wide graph with fan-out", () => {
    /** A root fanning out to many children creates two levels. */
    const g = new DirectedGraph();
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
// 9. String Representation
// ======================================================================

describe("String Representation", () => {
  it("toStringRepr shows counts", () => {
    /** toStringRepr should show node and edge counts. */
    const g = new DirectedGraph();
    g.addEdge("A", "B");
    const repr = g.toStringRepr();
    expect(repr).toContain("DirectedGraph");
    expect(repr).toContain("nodes=2");
    expect(repr).toContain("edges=1");
  });

  it("edge count tracks edges", () => {
    /** edgeCount() should reflect the number of edges. */
    const g = new DirectedGraph();
    expect(g.edgeCount()).toBe(0);
    g.addEdge("A", "B");
    expect(g.edgeCount()).toBe(1);
    g.addEdge("B", "C");
    expect(g.edgeCount()).toBe(2);
  });
});

// ======================================================================
// 10. Real Repo Graph Integration Test
// ======================================================================
//
// This models the actual dependency graph of packages in the
// coding-adventures repository. It serves as a comprehensive integration
// test to ensure all algorithms work together on a realistic graph.

describe("Real Repo Graph", () => {
  function makeRepoGraph() {
    const g = new DirectedGraph();

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

    // directed-graph has no dependencies
    g.addNode("directed-graph");

    return g;
  }

  it("has 21 packages", () => {
    /** The repository has 21 packages. */
    const g = makeRepoGraph();
    expect(g.size()).toBe(21);
  });

  it("is acyclic", () => {
    /** The repo dependency graph should be a DAG. */
    const g = makeRepoGraph();
    expect(g.hasCycle()).toBe(false);
  });

  it("topological sort includes all packages", () => {
    /** Topological sort should produce a valid ordering of all 21 packages. */
    const g = makeRepoGraph();
    const order = g.topologicalSort();
    expect(order.length).toBe(21);

    // Verify ordering: for every edge, from must appear before to.
    const position = new Map<string, number>();
    order.forEach((node: string, i: number) => position.set(node, i));

    for (const [fromNode, toNode] of g.edges()) {
      expect(position.get(fromNode)!).toBeLessThan(position.get(toNode)!);
    }
  });

  it("independent groups partition all packages", () => {
    /** Independent groups should contain all 21 packages. */
    const g = makeRepoGraph();
    const groups = g.independentGroups();
    const allNodes = groups.flat();
    expect(allNodes.length).toBe(21);
    expect(new Set(allNodes)).toEqual(new Set(g.nodes()));
  });

  it("transitive closure of logic-gates is empty", () => {
    /** logic-gates has no dependencies. */
    const g = makeRepoGraph();
    expect(g.transitiveClosure("logic-gates")).toEqual([]);
  });

  it("affected by grammar-tools change", () => {
    /** Changing grammar-tools should affect lexer, parser, assembler, etc. */
    const g = makeRepoGraph();
    const affected = new Set(g.affectedNodes(["grammar-tools"]));
    expect(affected.has("grammar-tools")).toBe(true);
    expect(affected.has("lexer")).toBe(true);
    expect(affected.has("parser")).toBe(true);
    expect(affected.has("assembler")).toBe(true);
    expect(affected.has("pipeline")).toBe(true);
  });

  it("affected by leaf change", () => {
    /** Changing bytecode-compiler (a leaf) affects only itself. */
    const g = makeRepoGraph();
    const affected = new Set(g.affectedNodes(["bytecode-compiler"]));
    expect(affected).toEqual(new Set(["bytecode-compiler"]));
  });
});
