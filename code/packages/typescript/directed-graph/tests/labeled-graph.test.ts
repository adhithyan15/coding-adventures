/**
 * labeled-graph.test.ts -- Tests for LabeledDirectedGraph
 * ========================================================
 *
 * A LabeledDirectedGraph extends a regular directed graph with labels on
 * edges.  Think of it like a state machine: nodes are states, and each
 * edge carries a label (the input symbol that triggers the transition).
 *
 * We organize tests into logical groups:
 *   1.  Empty graph behaviour
 *   2.  Node operations (add, remove, query)
 *   3.  Adding labeled edges
 *   4.  Removing labeled edges
 *   5.  Edge queries (hasEdge, edges, labels)
 *   6.  Multi-label edges (same pair, different labels)
 *   7.  Self-loops (a state transitioning to itself)
 *   8.  Successors and predecessors with label filtering
 *   9.  Algorithm delegation (topologicalSort, hasCycle, etc.)
 *   10. Error conditions
 *   11. Complex graph scenarios
 *   12. State machine modeling (integration test)
 */

import { describe, it, expect } from "vitest";
import {
  LabeledDirectedGraph,
  CycleError,
  NodeNotFoundError,
  EdgeNotFoundError,
} from "../src/labeled-graph.js";
import { Graph } from "../src/graph.js";

// ======================================================================
// 1. Empty Graph
// ======================================================================

describe("LabeledDirectedGraph: Empty Graph", () => {
  it("has no nodes", () => {
    const g = new LabeledDirectedGraph();
    expect(g.nodes()).toEqual([]);
    expect(g.size).toBe(0);
  });

  it("has no edges", () => {
    const g = new LabeledDirectedGraph();
    expect(g.edges()).toEqual([]);
  });

  it("hasNode returns false for any node", () => {
    const g = new LabeledDirectedGraph();
    expect(g.hasNode("A")).toBe(false);
  });

  it("hasEdge returns false for any edge", () => {
    const g = new LabeledDirectedGraph();
    expect(g.hasEdge("A", "B")).toBe(false);
    expect(g.hasEdge("A", "B", "x")).toBe(false);
  });
});

// ======================================================================
// 2. Node Operations
// ======================================================================

describe("LabeledDirectedGraph: Node Operations", () => {
  it("addNode makes it present", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    expect(g.hasNode("A")).toBe(true);
    expect(g.size).toBe(1);
  });

  it("addNode is idempotent", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    g.addNode("A");
    expect(g.size).toBe(1);
  });

  it("removeNode makes it absent", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    g.removeNode("A");
    expect(g.hasNode("A")).toBe(false);
    expect(g.size).toBe(0);
  });

  it("removeNode cleans up outgoing edges", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.removeNode("A");
    expect(g.hasEdge("A", "B")).toBe(false);
    expect(g.edges()).toEqual([]);
  });

  it("removeNode cleans up incoming edges", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.removeNode("B");
    expect(g.hasEdge("A", "B")).toBe(false);
    expect(g.edges()).toEqual([]);
  });

  it("removeNode cleans up self-loop", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    g.removeNode("A");
    expect(g.hasNode("A")).toBe(false);
    expect(g.edges()).toEqual([]);
  });

  it("nodes list is returned", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("C");
    g.addNode("A");
    g.addNode("B");
    expect(g.nodes().sort()).toEqual(["A", "B", "C"]);
  });
});

// ======================================================================
// 3. Adding Labeled Edges
// ======================================================================

describe("LabeledDirectedGraph: Adding Edges", () => {
  it("addEdge creates both nodes", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(true);
    expect(g.size).toBe(2);
  });

  it("addEdge creates the structural edge", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasEdge("A", "B")).toBe(true);
  });

  it("addEdge creates the labeled edge", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasEdge("A", "B", "x")).toBe(true);
  });

  it("duplicate edge is idempotent", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "x");
    expect(g.edges()).toEqual([["A", "B", "x"]]);
  });

  it("reverse edge does not exist", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasEdge("B", "A")).toBe(false);
    expect(g.hasEdge("B", "A", "x")).toBe(false);
  });
});

// ======================================================================
// 4. Removing Labeled Edges
// ======================================================================

describe("LabeledDirectedGraph: Removing Edges", () => {
  it("removeEdge removes the labeled edge", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.removeEdge("A", "B", "x");
    expect(g.hasEdge("A", "B", "x")).toBe(false);
    expect(g.hasEdge("A", "B")).toBe(false);
  });

  it("removeEdge keeps nodes", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.removeEdge("A", "B", "x");
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("B")).toBe(true);
  });

  it("removing one label keeps other label", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    g.removeEdge("A", "B", "x");
    expect(g.hasEdge("A", "B", "x")).toBe(false);
    expect(g.hasEdge("A", "B", "y")).toBe(true);
    expect(g.hasEdge("A", "B")).toBe(true);
  });

  it("removing last label removes structural edge", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.removeEdge("A", "B", "x");
    expect(g.hasEdge("A", "B")).toBe(false);
  });
});

// ======================================================================
// 5. Edge Queries
// ======================================================================

describe("LabeledDirectedGraph: Edge Queries", () => {
  it("hasEdge without label checks structural edge", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasEdge("A", "B")).toBe(true);
  });

  it("hasEdge with matching label", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasEdge("A", "B", "x")).toBe(true);
  });

  it("hasEdge with wrong label", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasEdge("A", "B", "y")).toBe(false);
  });

  it("hasEdge missing source", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("B");
    expect(g.hasEdge("A", "B")).toBe(false);
  });

  it("edges returns triples", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.edges()).toEqual([["A", "B", "x"]]);
  });

  it("edges sorted deterministically", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("B", "C", "y");
    g.addEdge("A", "B", "x");
    expect(g.edges()).toEqual([
      ["A", "B", "x"],
      ["B", "C", "y"],
    ]);
  });

  it("labels returns set of labels", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    expect(g.labels("A", "B")).toEqual(new Set(["x", "y"]));
  });

  it("labels returns empty set for no edge", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    g.addNode("B");
    expect(g.labels("A", "B")).toEqual(new Set());
  });

  it("labels returns a copy", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    const labels = g.labels("A", "B");
    labels.add("sneaky");
    expect(g.hasEdge("A", "B", "sneaky")).toBe(false);
  });
});

// ======================================================================
// 6. Multi-Label Edges
// ======================================================================

describe("LabeledDirectedGraph: Multi-Label Edges", () => {
  it("two labels on same pair", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    expect(g.hasEdge("A", "B", "x")).toBe(true);
    expect(g.hasEdge("A", "B", "y")).toBe(true);
  });

  it("three labels on same pair", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    g.addEdge("A", "B", "z");
    expect(g.labels("A", "B")).toEqual(new Set(["x", "y", "z"]));
  });

  it("multi-label edges list", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    expect(g.edges()).toEqual([
      ["A", "B", "x"],
      ["A", "B", "y"],
    ]);
  });

  it("multi-label does not duplicate nodes", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    expect(g.size).toBe(2);
  });

  it("same label on different pairs", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("C", "D", "x");
    expect(g.hasEdge("A", "B", "x")).toBe(true);
    expect(g.hasEdge("C", "D", "x")).toBe(true);
  });
});

// ======================================================================
// 7. Self-Loops
// ======================================================================

describe("LabeledDirectedGraph: Self-Loops", () => {
  it("self-loop is allowed", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    expect(g.hasEdge("A", "A", "loop")).toBe(true);
  });

  it("self-loop appears in edges", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    expect(g.edges()).toEqual([["A", "A", "loop"]]);
  });

  it("self-loop node is own successor", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    expect(g.successors("A")).toContain("A");
  });

  it("self-loop node is own predecessor", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    expect(g.predecessors("A")).toContain("A");
  });

  it("self-loop with other edges", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "self");
    g.addEdge("A", "B", "other");
    expect(g.hasEdge("A", "A", "self")).toBe(true);
    expect(g.hasEdge("A", "B", "other")).toBe(true);
  });

  it("self-loop multi-label", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "x");
    g.addEdge("A", "A", "y");
    expect(g.labels("A", "A")).toEqual(new Set(["x", "y"]));
  });

  it("remove self-loop", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    g.removeEdge("A", "A", "loop");
    expect(g.hasEdge("A", "A")).toBe(false);
  });

  it("self-loop labels", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "loop");
    expect(g.labels("A", "A")).toEqual(new Set(["loop"]));
  });
});

// ======================================================================
// 8. Successors and Predecessors with Label Filtering
// ======================================================================

describe("LabeledDirectedGraph: Neighbors", () => {
  it("successors without filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "C", "y");
    expect(g.successors("A").sort()).toEqual(["B", "C"]);
  });

  it("successors with label filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "C", "y");
    expect(g.successors("A", "x")).toEqual(["B"]);
  });

  it("successors label filter no match", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.successors("A", "z")).toEqual([]);
  });

  it("predecessors without filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "C", "x");
    g.addEdge("B", "C", "y");
    expect(g.predecessors("C").sort()).toEqual(["A", "B"]);
  });

  it("predecessors with label filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "C", "x");
    g.addEdge("B", "C", "y");
    expect(g.predecessors("C", "x")).toEqual(["A"]);
  });

  it("predecessors label filter no match", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "C", "x");
    expect(g.predecessors("C", "z")).toEqual([]);
  });

  it("successors multi-label filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    g.addEdge("A", "C", "x");
    expect(g.successors("A", "x")).toEqual(["B", "C"]);
  });

  it("predecessors multi-label filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "C", "x");
    g.addEdge("A", "C", "y");
    g.addEdge("B", "C", "x");
    expect(g.predecessors("C", "x")).toEqual(["A", "B"]);
  });

  it("self-loop in successors with filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "self");
    g.addEdge("A", "B", "other");
    expect(g.successors("A", "self")).toEqual(["A"]);
  });

  it("self-loop in predecessors with filter", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "self");
    g.addEdge("B", "A", "other");
    expect(g.predecessors("A", "self")).toEqual(["A"]);
  });
});

// ======================================================================
// 9. Algorithm Delegation
// ======================================================================

describe("LabeledDirectedGraph: Algorithms", () => {
  it("topologicalSort linear", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("B", "C", "y");
    expect(g.topologicalSort()).toEqual(["A", "B", "C"]);
  });

  it("topologicalSort empty", () => {
    const g = new LabeledDirectedGraph();
    expect(g.topologicalSort()).toEqual([]);
  });

  it("hasCycle false", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(g.hasCycle()).toBe(false);
  });

  it("hasCycle true", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("B", "A", "y");
    expect(g.hasCycle()).toBe(true);
  });

  it("hasCycle with self-loop", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "A", "self");
    expect(g.hasCycle()).toBe(true);
  });

  it("transitiveClosure linear", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("B", "C", "y");
    expect(g.transitiveClosure("A")).toEqual(new Set(["B", "C"]));
    expect(g.transitiveClosure("B")).toEqual(new Set(["C"]));
    expect(g.transitiveClosure("C")).toEqual(new Set());
  });

  it("transitiveClosure nonexistent node throws", () => {
    const g = new LabeledDirectedGraph();
    expect(() => g.transitiveClosure("X")).toThrow(NodeNotFoundError);
  });
});

// ======================================================================
// 10. Error Conditions
// ======================================================================

describe("LabeledDirectedGraph: Errors", () => {
  it("removeNode missing throws", () => {
    const g = new LabeledDirectedGraph();
    expect(() => g.removeNode("Z")).toThrow(NodeNotFoundError);
  });

  it("removeEdge missing source throws", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("B");
    expect(() => g.removeEdge("A", "B", "x")).toThrow(NodeNotFoundError);
  });

  it("removeEdge missing target throws", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    expect(() => g.removeEdge("A", "B", "x")).toThrow(NodeNotFoundError);
  });

  it("removeEdge missing label throws", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    expect(() => g.removeEdge("A", "B", "y")).toThrow(EdgeNotFoundError);
  });

  it("removeEdge no edge throws", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    g.addNode("B");
    expect(() => g.removeEdge("A", "B", "x")).toThrow(EdgeNotFoundError);
  });

  it("labels missing source throws", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("B");
    expect(() => g.labels("A", "B")).toThrow(NodeNotFoundError);
  });

  it("labels missing target throws", () => {
    const g = new LabeledDirectedGraph();
    g.addNode("A");
    expect(() => g.labels("A", "B")).toThrow(NodeNotFoundError);
  });

  it("successors missing node throws", () => {
    const g = new LabeledDirectedGraph();
    expect(() => g.successors("Z")).toThrow(NodeNotFoundError);
  });

  it("predecessors missing node throws", () => {
    const g = new LabeledDirectedGraph();
    expect(() => g.predecessors("Z")).toThrow(NodeNotFoundError);
  });

  it("successors with label missing node throws", () => {
    const g = new LabeledDirectedGraph();
    expect(() => g.successors("Z", "x")).toThrow(NodeNotFoundError);
  });

  it("predecessors with label missing node throws", () => {
    const g = new LabeledDirectedGraph();
    expect(() => g.predecessors("Z", "x")).toThrow(NodeNotFoundError);
  });
});

// ======================================================================
// 11. Complex Graph Scenarios
// ======================================================================

describe("LabeledDirectedGraph: Complex Scenarios", () => {
  it("diamond with labels", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "left");
    g.addEdge("A", "C", "right");
    g.addEdge("B", "D", "merge");
    g.addEdge("C", "D", "merge");
    expect(g.size).toBe(4);
    expect(g.topologicalSort()).toEqual(["A", "B", "C", "D"]);
  });

  it("remove node from middle of chain", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("B", "C", "y");
    g.removeNode("B");
    expect(g.hasNode("B")).toBe(false);
    expect(g.hasNode("A")).toBe(true);
    expect(g.hasNode("C")).toBe(true);
    expect(g.edges()).toEqual([]);
  });

  it("remove hub node", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("C", "B", "y");
    g.addEdge("B", "D", "z");
    g.removeNode("B");
    expect(g.size).toBe(3);
    expect(g.edges()).toEqual([]);
  });

  it("many labels on same pair", () => {
    const g = new LabeledDirectedGraph();
    const labels: string[] = [];
    for (let i = 1; i <= 10; i++) {
      const label = `label_${i}`;
      labels.push(label);
      g.addEdge("A", "B", label);
    }
    expect(g.labels("A", "B")).toEqual(new Set(labels));
    expect(g.edges().length).toBe(10);
  });

  it("remove all labels one by one", () => {
    const g = new LabeledDirectedGraph();
    g.addEdge("A", "B", "x");
    g.addEdge("A", "B", "y");
    g.addEdge("A", "B", "z");
    g.removeEdge("A", "B", "x");
    g.removeEdge("A", "B", "y");
    expect(g.hasEdge("A", "B", "z")).toBe(true);
    g.removeEdge("A", "B", "z");
    expect(g.hasEdge("A", "B")).toBe(false);
  });
});

// ======================================================================
// 12. State Machine Modeling (Integration Test)
// ======================================================================
//
// A turnstile has two states: "locked" and "unlocked".
// - In "locked" state:
//   - "coin" input -> transitions to "unlocked"
//   - "push" input -> stays "locked" (self-loop)
// - In "unlocked" state:
//   - "coin" input -> stays "unlocked" (self-loop)
//   - "push" input -> transitions to "locked"

describe("LabeledDirectedGraph: Turnstile FSM", () => {
  function makeTurnstile(): LabeledDirectedGraph {
    const fsm = new LabeledDirectedGraph();
    fsm.addEdge("locked", "unlocked", "coin");
    fsm.addEdge("locked", "locked", "push");
    fsm.addEdge("unlocked", "locked", "push");
    fsm.addEdge("unlocked", "unlocked", "coin");
    return fsm;
  }

  it("has 2 states", () => {
    const fsm = makeTurnstile();
    expect(fsm.size).toBe(2);
  });

  it("has correct nodes", () => {
    const fsm = makeTurnstile();
    expect(fsm.nodes().sort()).toEqual(["locked", "unlocked"]);
  });

  it("has 4 edges", () => {
    const fsm = makeTurnstile();
    expect(fsm.edges()).toEqual([
      ["locked", "locked", "push"],
      ["locked", "unlocked", "coin"],
      ["unlocked", "locked", "push"],
      ["unlocked", "unlocked", "coin"],
    ]);
  });

  it("coin from locked goes to unlocked", () => {
    const fsm = makeTurnstile();
    expect(fsm.successors("locked", "coin")).toEqual(["unlocked"]);
  });

  it("push from locked stays locked", () => {
    const fsm = makeTurnstile();
    expect(fsm.successors("locked", "push")).toEqual(["locked"]);
  });

  it("coin from unlocked stays unlocked", () => {
    const fsm = makeTurnstile();
    expect(fsm.successors("unlocked", "coin")).toEqual(["unlocked"]);
  });

  it("push from unlocked goes to locked", () => {
    const fsm = makeTurnstile();
    expect(fsm.successors("unlocked", "push")).toEqual(["locked"]);
  });

  it("labels locked to unlocked", () => {
    const fsm = makeTurnstile();
    expect(fsm.labels("locked", "unlocked")).toEqual(new Set(["coin"]));
  });

  it("labels locked to locked", () => {
    const fsm = makeTurnstile();
    expect(fsm.labels("locked", "locked")).toEqual(new Set(["push"]));
  });

  it("has cycle (expected for FSM)", () => {
    const fsm = makeTurnstile();
    expect(fsm.hasCycle()).toBe(true);
  });

  it("predecessors of locked", () => {
    const fsm = makeTurnstile();
    expect(fsm.predecessors("locked").sort()).toEqual(["locked", "unlocked"]);
  });

  it("predecessors of locked by push", () => {
    const fsm = makeTurnstile();
    expect(fsm.predecessors("locked", "push")).toEqual([
      "locked",
      "unlocked",
    ]);
  });

  it("predecessors of locked by coin", () => {
    const fsm = makeTurnstile();
    expect(fsm.predecessors("locked", "coin")).toEqual([]);
  });
});

// ======================================================================
// 13. Self-Loop Flag Tests on Base Graph
// ======================================================================
// These test that the allowSelfLoops flag works on the base Graph
// through the LabeledDirectedGraph (which always enables it).

describe("Graph allowSelfLoops flag (via labeled-graph tests)", () => {
  it("default Graph rejects self-loops", () => {
    const g = new Graph();
    expect(() => g.addEdge("A", "A")).toThrow("Self-loops are not allowed");
  });

  it("Graph with allowSelfLoops: true accepts self-loops", () => {
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    expect(g.hasEdge("A", "A")).toBe(true);
  });

  it("allowSelfLoops property", () => {
    const g1 = new Graph();
    expect(g1.allowSelfLoops).toBe(false);
    const g2 = new Graph({ allowSelfLoops: true });
    expect(g2.allowSelfLoops).toBe(true);
  });

  it("self-loop node is own successor and predecessor", () => {
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    expect(g.successors("A")).toContain("A");
    expect(g.predecessors("A")).toContain("A");
  });

  it("self-loop appears in edges", () => {
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    expect(g.edges()).toEqual([["A", "A"]]);
  });

  it("self-loop with other edges", () => {
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.addEdge("A", "B");
    expect(g.hasEdge("A", "A")).toBe(true);
    expect(g.hasEdge("A", "B")).toBe(true);
  });

  it("remove self-loop", () => {
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.removeEdge("A", "A");
    expect(g.hasEdge("A", "A")).toBe(false);
    expect(g.hasNode("A")).toBe(true);
  });

  it("remove node with self-loop", () => {
    const g = new Graph({ allowSelfLoops: true });
    g.addEdge("A", "A");
    g.addEdge("A", "B");
    g.removeNode("A");
    expect(g.hasNode("A")).toBe(false);
    expect(g.hasNode("B")).toBe(true);
    expect(g.edges()).toEqual([]);
  });
});
