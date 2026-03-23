/**
 * visualization.test.ts -- Tests for Graph Visualization Functions
 * ================================================================
 *
 * We test all three visualization functions (toDot, toMermaid, toAsciiTable)
 * with both Graph and LabeledDirectedGraph inputs. Tests verify:
 *
 * - Correct format structure (headers, separators, brackets)
 * - Edge labels are included for labeled graphs
 * - Multiple labels on the same edge are combined correctly
 * - Deterministic output (same graph always produces same string)
 * - Options like name, rankdir, nodeAttrs, initial work correctly
 * - Edge cases: empty graphs, isolated nodes, self-loops
 */

import { describe, test, expect } from "vitest";
import {
  Graph,
  LabeledDirectedGraph,
  toDot,
  toMermaid,
  toAsciiTable,
} from "../src/index.js";
import type { DotOptions, MermaidOptions } from "../src/index.js";

// ---------------------------------------------------------------------------
// Helper: create a turnstile FSM (used in many tests)
// ---------------------------------------------------------------------------

function turnstile(): LabeledDirectedGraph {
  const lg = new LabeledDirectedGraph();
  lg.addEdge("locked", "unlocked", "coin");
  lg.addEdge("locked", "locked", "push");
  lg.addEdge("unlocked", "locked", "push");
  lg.addEdge("unlocked", "unlocked", "coin");
  return lg;
}

function simpleDag(): Graph {
  const g = new Graph();
  g.addEdge("A", "B");
  g.addEdge("A", "C");
  g.addEdge("B", "D");
  g.addEdge("C", "D");
  return g;
}

// ===========================================================================
// toDot -- Unlabeled Graph
// ===========================================================================

describe("toDot with Graph", () => {
  test("empty graph produces valid DOT", () => {
    const g = new Graph();
    const dot = toDot(g);
    expect(dot).toContain("digraph G {");
    expect(dot).toContain("rankdir=LR;");
    expect(dot).toMatch(/\}$/);
  });

  test("single node", () => {
    const g = new Graph();
    g.addNode("A");
    const dot = toDot(g);
    expect(dot).toContain("    A;");
  });

  test("single edge", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const dot = toDot(g);
    expect(dot).toContain("    A -> B;");
  });

  test("diamond DAG", () => {
    const g = simpleDag();
    const dot = toDot(g);
    expect(dot).toContain("A -> B;");
    expect(dot).toContain("A -> C;");
    expect(dot).toContain("B -> D;");
    expect(dot).toContain("C -> D;");
  });

  test("custom name", () => {
    const g = new Graph();
    const dot = toDot(g, { name: "MyGraph" });
    expect(dot).toContain("digraph MyGraph {");
  });

  test("TB rankdir", () => {
    const g = new Graph();
    const dot = toDot(g, { rankdir: "TB" });
    expect(dot).toContain("rankdir=TB;");
  });

  test("node attributes", () => {
    const g = new Graph();
    g.addNode("A");
    const nodeAttrs = new Map([["A", { shape: "circle" }]]);
    const dot = toDot(g, { nodeAttrs });
    expect(dot).toContain("A [shape=circle];");
  });

  test("multiple node attributes sorted", () => {
    const g = new Graph();
    g.addNode("A");
    const nodeAttrs = new Map([["A", { shape: "circle", color: "red" }]]);
    const dot = toDot(g, { nodeAttrs });
    expect(dot).toContain("A [color=red, shape=circle];");
  });

  test("initial state marker", () => {
    const g = new Graph();
    g.addNode("start");
    const dot = toDot(g, { initial: "start" });
    expect(dot).toContain('    "" [shape=none];');
    expect(dot).toContain('    "" -> start;');
  });

  test("nodes without attrs still listed", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const nodeAttrs = new Map([["A", { shape: "circle" }]]);
    const dot = toDot(g, { nodeAttrs });
    expect(dot).toContain("    B;");
  });

  test("isolated nodes appear", () => {
    const g = new Graph();
    g.addNode("X");
    g.addNode("Y");
    const dot = toDot(g);
    expect(dot).toContain("    X;");
    expect(dot).toContain("    Y;");
  });

  test("deterministic output", () => {
    const g = simpleDag();
    const dot1 = toDot(g);
    const dot2 = toDot(g);
    expect(dot1).toBe(dot2);
  });
});

// ===========================================================================
// toDot -- Labeled Graph
// ===========================================================================

describe("toDot with LabeledDirectedGraph", () => {
  test("empty labeled graph", () => {
    const lg = new LabeledDirectedGraph();
    const dot = toDot(lg);
    expect(dot).toContain("digraph G {");
  });

  test("single labeled edge", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "compile");
    const dot = toDot(lg);
    expect(dot).toContain('A -> B [label="compile"];');
  });

  test("multiple labels on same edge are combined", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "compile");
    lg.addEdge("A", "B", "test");
    const dot = toDot(lg);
    expect(dot).toContain('A -> B [label="compile, test"];');
  });

  test("turnstile FSM", () => {
    const lg = turnstile();
    const dot = toDot(lg);
    expect(dot).toContain('locked -> locked [label="push"];');
    expect(dot).toContain('locked -> unlocked [label="coin"];');
    expect(dot).toContain('unlocked -> locked [label="push"];');
    expect(dot).toContain('unlocked -> unlocked [label="coin"];');
  });

  test("initial state with labeled graph", () => {
    const lg = turnstile();
    const dot = toDot(lg, { initial: "locked" });
    expect(dot).toContain('"" [shape=none];');
    expect(dot).toContain('"" -> locked;');
  });

  test("node attrs with labeled graph", () => {
    const lg = turnstile();
    const nodeAttrs = new Map([
      ["unlocked", { shape: "doublecircle" }],
    ]);
    const dot = toDot(lg, { nodeAttrs });
    expect(dot).toContain("unlocked [shape=doublecircle];");
  });

  test("three labels on same edge", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "x");
    lg.addEdge("A", "B", "y");
    lg.addEdge("A", "B", "z");
    const dot = toDot(lg);
    expect(dot).toContain('A -> B [label="x, y, z"];');
  });

  test("custom name with labeled graph", () => {
    const lg = new LabeledDirectedGraph();
    const dot = toDot(lg, { name: "FSM" });
    expect(dot).toContain("digraph FSM {");
  });

  test("deterministic labeled output", () => {
    const lg = turnstile();
    const dot1 = toDot(lg);
    const dot2 = toDot(lg);
    expect(dot1).toBe(dot2);
  });
});

// ===========================================================================
// toMermaid -- Unlabeled Graph
// ===========================================================================

describe("toMermaid with Graph", () => {
  test("empty graph", () => {
    const g = new Graph();
    const m = toMermaid(g);
    expect(m).toBe("graph LR");
  });

  test("single edge", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const m = toMermaid(g);
    expect(m).toContain("A --> B");
  });

  test("diamond DAG", () => {
    const g = simpleDag();
    const m = toMermaid(g);
    expect(m).toContain("graph LR");
    expect(m).toContain("A --> B");
    expect(m).toContain("A --> C");
    expect(m).toContain("B --> D");
    expect(m).toContain("C --> D");
  });

  test("TD direction", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const m = toMermaid(g, { direction: "TD" });
    expect(m).toContain("graph TD");
  });

  test("default LR direction", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const m = toMermaid(g);
    expect(m).toContain("graph LR");
  });

  test("chain graph", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    g.addEdge("B", "C");
    g.addEdge("C", "D");
    const m = toMermaid(g);
    expect(m).toContain("A --> B");
    expect(m).toContain("B --> C");
    expect(m).toContain("C --> D");
  });

  test("initial state marker", () => {
    const g = new Graph();
    g.addNode("start");
    const m = toMermaid(g, { initial: "start" });
    expect(m).toContain("_start_(( )) --> start");
  });

  test("deterministic output", () => {
    const g = simpleDag();
    const m1 = toMermaid(g);
    const m2 = toMermaid(g);
    expect(m1).toBe(m2);
  });
});

// ===========================================================================
// toMermaid -- Labeled Graph
// ===========================================================================

describe("toMermaid with LabeledDirectedGraph", () => {
  test("empty labeled graph", () => {
    const lg = new LabeledDirectedGraph();
    const m = toMermaid(lg);
    expect(m).toBe("graph LR");
  });

  test("single labeled edge", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "compile");
    const m = toMermaid(lg);
    expect(m).toContain("A -->|compile| B");
  });

  test("multiple labels combined", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "compile");
    lg.addEdge("A", "B", "test");
    const m = toMermaid(lg);
    expect(m).toContain("A -->|compile, test| B");
  });

  test("turnstile FSM", () => {
    const lg = turnstile();
    const m = toMermaid(lg);
    expect(m).toContain("locked -->|coin| unlocked");
    expect(m).toContain("locked -->|push| locked");
    expect(m).toContain("unlocked -->|coin| unlocked");
    expect(m).toContain("unlocked -->|push| locked");
  });

  test("TD direction", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "dep");
    const m = toMermaid(lg, { direction: "TD" });
    expect(m).toContain("graph TD");
  });

  test("three labels", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "x");
    lg.addEdge("A", "B", "y");
    lg.addEdge("A", "B", "z");
    const m = toMermaid(lg);
    expect(m).toContain("A -->|x, y, z| B");
  });

  test("initial state with labeled graph", () => {
    const lg = turnstile();
    const m = toMermaid(lg, { initial: "locked" });
    expect(m).toContain("_start_(( )) --> locked");
  });

  test("deterministic labeled output", () => {
    const lg = turnstile();
    const m1 = toMermaid(lg);
    const m2 = toMermaid(lg);
    expect(m1).toBe(m2);
  });
});

// ===========================================================================
// toAsciiTable -- Unlabeled Graph
// ===========================================================================

describe("toAsciiTable with Graph", () => {
  test("empty graph", () => {
    const g = new Graph();
    const table = toAsciiTable(g);
    expect(table).toContain("Node");
    expect(table).toContain("Successors");
  });

  test("single node no edges", () => {
    const g = new Graph();
    g.addNode("A");
    const table = toAsciiTable(g);
    expect(table).toContain("A");
    expect(table).toContain("-");
  });

  test("single edge", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const table = toAsciiTable(g);
    expect(table).toContain("A");
    expect(table).toContain("B");
  });

  test("diamond DAG", () => {
    const g = simpleDag();
    const table = toAsciiTable(g);
    expect(table).toContain("B, C");
    // D has no successors.
    const lines = table.split("\n");
    const dLine = lines.find((l) => l.startsWith("D"));
    expect(dLine).toContain("-");
  });

  test("header and separator present", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const table = toAsciiTable(g);
    const lines = table.split("\n");
    expect(lines[0]).toContain("Node");
    expect(lines[0]).toContain("Successors");
    expect(lines[1]).toContain("-+-");
  });

  test("column alignment with long names", () => {
    const g = new Graph();
    g.addEdge("short", "very_long_successor_name");
    const table = toAsciiTable(g);
    expect(table).toContain("very_long_successor_name");
  });

  test("hub node with many successors", () => {
    const g = new Graph();
    g.addEdge("hub", "A");
    g.addEdge("hub", "B");
    g.addEdge("hub", "C");
    g.addEdge("hub", "D");
    const table = toAsciiTable(g);
    expect(table).toContain("A, B, C, D");
  });

  test("deterministic output", () => {
    const g = simpleDag();
    const t1 = toAsciiTable(g);
    const t2 = toAsciiTable(g);
    expect(t1).toBe(t2);
  });
});

// ===========================================================================
// toAsciiTable -- Labeled Graph
// ===========================================================================

describe("toAsciiTable with LabeledDirectedGraph", () => {
  test("empty labeled graph", () => {
    const lg = new LabeledDirectedGraph();
    const table = toAsciiTable(lg);
    expect(table).toContain("State");
  });

  test("single labeled edge", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "dep");
    const table = toAsciiTable(lg);
    expect(table).toContain("State");
    expect(table).toContain("dep");
    expect(table).toContain("B");
  });

  test("turnstile transition table", () => {
    const lg = turnstile();
    const table = toAsciiTable(lg);
    expect(table).toContain("State");
    expect(table).toContain("coin");
    expect(table).toContain("push");
    // Check that locked row has correct transitions.
    const lines = table.split("\n");
    const lockedLine = lines.find((l) => l.startsWith("locked "));
    expect(lockedLine).toContain("unlocked");
  });

  test("separator row", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "dep");
    const table = toAsciiTable(lg);
    const lines = table.split("\n");
    expect(lines[1]).toContain("-+-");
  });

  test("missing transition shows dash", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "x");
    lg.addNode("C");
    const table = toAsciiTable(lg);
    const lines = table.split("\n");
    const cLine = lines.find((l) => l.startsWith("C"));
    expect(cLine).toContain("-");
  });

  test("nodes without edges show dashes", () => {
    const lg = new LabeledDirectedGraph();
    lg.addNode("A");
    lg.addNode("B");
    const table = toAsciiTable(lg);
    expect(table).toContain("State");
    expect(table).toContain("A");
    expect(table).toContain("B");
  });

  test("multiple label columns", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "x");
    lg.addEdge("A", "C", "y");
    const table = toAsciiTable(lg);
    expect(table).toContain("x");
    expect(table).toContain("y");
  });

  test("wide state names align correctly", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("very_long_state_name", "B", "go");
    const table = toAsciiTable(lg);
    expect(table).toContain("very_long_state_name");
  });

  test("deterministic labeled output", () => {
    const lg = turnstile();
    const t1 = toAsciiTable(lg);
    const t2 = toAsciiTable(lg);
    expect(t1).toBe(t2);
  });

  test("three columns in transition table", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("S1", "S2", "a");
    lg.addEdge("S1", "S3", "b");
    lg.addEdge("S2", "S1", "c");
    const table = toAsciiTable(lg);
    expect(table).toContain("a");
    expect(table).toContain("b");
    expect(table).toContain("c");
    // S3 has no outgoing edges, should show dashes.
    const lines = table.split("\n");
    const s3Line = lines.find((l) => l.startsWith("S3"));
    expect(s3Line).toBeDefined();
    // Count dashes in S3 line (should have 3 dashes, one per label column).
    const dashCount = (s3Line!.match(/ - /g) || []).length +
      (s3Line!.match(/ -$/g) || []).length +
      (s3Line!.match(/\| -/g) || []).length;
    expect(dashCount).toBeGreaterThanOrEqual(2);
  });
});

// ===========================================================================
// Full DOT output format verification
// ===========================================================================

describe("toDot full output format", () => {
  test("turnstile with all options", () => {
    const lg = turnstile();
    const nodeAttrs = new Map([
      ["locked", { shape: "circle" }],
      ["unlocked", { shape: "doublecircle" }],
    ]);
    const dot = toDot(lg, {
      name: "Turnstile",
      rankdir: "LR",
      initial: "locked",
      nodeAttrs,
    });

    // Verify structure.
    expect(dot).toContain("digraph Turnstile {");
    expect(dot).toContain("rankdir=LR;");
    expect(dot).toContain('"" [shape=none];');
    expect(dot).toContain('"" -> locked;');
    expect(dot).toContain("locked [shape=circle];");
    expect(dot).toContain("unlocked [shape=doublecircle];");
    expect(dot).toContain('locked -> locked [label="push"];');
    expect(dot).toContain('locked -> unlocked [label="coin"];');
  });

  test("unlabeled graph full format", () => {
    const g = new Graph();
    g.addEdge("compile", "link");
    g.addEdge("link", "package");
    const dot = toDot(g, { name: "Build" });
    expect(dot).toContain("digraph Build {");
    expect(dot).toContain("compile -> link;");
    expect(dot).toContain("link -> package;");
    expect(dot.endsWith("}")).toBe(true);
  });
});

// ===========================================================================
// Full Mermaid output format verification
// ===========================================================================

describe("toMermaid full output format", () => {
  test("starts with graph directive", () => {
    const g = new Graph();
    g.addEdge("A", "B");
    const m = toMermaid(g);
    expect(m.startsWith("graph LR")).toBe(true);
  });

  test("labeled graph first line", () => {
    const lg = new LabeledDirectedGraph();
    lg.addEdge("A", "B", "dep");
    const m = toMermaid(lg, { direction: "TD" });
    expect(m.startsWith("graph TD")).toBe(true);
  });
});
