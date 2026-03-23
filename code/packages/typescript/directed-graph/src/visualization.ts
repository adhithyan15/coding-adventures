/**
 * visualization.ts -- Graph Visualization in Multiple Formats
 * ===========================================================
 *
 * This module converts directed graphs into human-readable text formats.
 * It supports three output formats, each serving a different purpose:
 *
 * 1. **DOT format** (Graphviz) -- the industry standard for graph
 *    visualization. Paste the output into https://dreampuf.github.io/GraphvizOnline/
 *    or pipe it to `dot -Tpng` to get a rendered image.
 *
 * 2. **Mermaid format** -- a lightweight alternative that renders directly
 *    in GitHub Markdown, Notion, and many other tools. Wrap the output in
 *    a ```mermaid code fence and it just works.
 *
 * 3. **ASCII table** -- a plain-text representation for terminal output.
 *    For labeled graphs, this produces a transition table (like an FSM
 *    state table). For unlabeled graphs, it produces an adjacency list.
 *
 * Why three formats?
 * ------------------
 *
 * Each format has a sweet spot:
 *
 * - DOT is the most powerful: it supports node shapes, colors, subgraphs,
 *   and precise layout control. Use it when you need publication-quality
 *   diagrams or complex styling.
 *
 * - Mermaid is the most convenient: it renders inline in documentation
 *   without any external tools. Use it for README files and wikis.
 *
 * - ASCII tables are the most portable: they work everywhere, including
 *   log files, emails, and terminals without Unicode support. Use them
 *   for debugging and quick inspection.
 *
 * Type detection
 * --------------
 *
 * We use `instanceof LabeledDirectedGraph` to distinguish between the two
 * graph types at runtime. This works because LabeledDirectedGraph is a
 * concrete class, not an interface — JavaScript's prototype chain gives us
 * reliable runtime type checking here.
 *
 * When a LabeledDirectedGraph is detected, we extract edge labels and
 * include them in the output. When a plain Graph is detected, we omit
 * label information entirely.
 */

import { Graph } from "./graph.js";
import { LabeledDirectedGraph } from "./labeled-graph.js";

// ---------------------------------------------------------------------------
// DOT format options
// ---------------------------------------------------------------------------

/**
 * Options for DOT output.
 *
 * - `name`: The graph name (appears as `digraph <name> { ... }`).
 *   Defaults to "G", which is the conventional placeholder name.
 *
 * - `nodeAttrs`: Per-node DOT attributes. For example, to make a node
 *   a doublecircle (indicating an accepting state in an FSM), pass:
 *   `new Map([["unlocked", { shape: "doublecircle" }]])`.
 *
 * - `initial`: If set, adds an invisible start node with an arrow to
 *   this node. This is the standard way to mark the initial state in
 *   FSM diagrams.
 *
 * - `rankdir`: Layout direction. "LR" means left-to-right, "TB" means
 *   top-to-bottom. Defaults to "LR" because horizontal layouts tend to
 *   be more readable for state machines and dependency chains.
 */
export interface DotOptions {
  name?: string;
  nodeAttrs?: Map<string, Record<string, string>>;
  initial?: string;
  rankdir?: "LR" | "TB";
}

// ---------------------------------------------------------------------------
// Mermaid format options
// ---------------------------------------------------------------------------

/**
 * Options for Mermaid output.
 *
 * - `direction`: Flow direction. "LR" for left-to-right, "TD" for
 *   top-down. Defaults to "LR".
 *
 * - `initial`: If set, adds an invisible start marker pointing to this
 *   node, using Mermaid's circle node syntax.
 */
export interface MermaidOptions {
  direction?: "LR" | "TD";
  initial?: string;
}

// ---------------------------------------------------------------------------
// Helper: collect edge labels for a labeled graph
// ---------------------------------------------------------------------------
//
// For labeled graphs, we need to know what labels exist between each pair
// of nodes. The LabeledDirectedGraph stores labels internally, but we
// need them grouped by (from, to) pair for rendering.
//
// We return a Map keyed by "from\0to" (matching the internal convention)
// where the value is the sorted list of labels joined with ", ".

/**
 * Given a labeled graph, build a lookup from (from, to) to a combined
 * label string like "coin, push".
 *
 * We sort the labels alphabetically so the output is deterministic --
 * important for tests and diffing.
 */
function collectEdgeLabels(
  graph: LabeledDirectedGraph
): Map<string, string> {
  const result = new Map<string, string>();
  const edges = graph.edges(); // returns [from, to, label][] sorted

  // Group labels by (from, to) pair.
  const grouped = new Map<string, string[]>();
  for (const [from, to, label] of edges) {
    const key = `${from}\0${to}`;
    if (!grouped.has(key)) {
      grouped.set(key, []);
    }
    grouped.get(key)!.push(label);
  }

  // Join each group into a comma-separated string.
  for (const [key, labels] of grouped) {
    result.set(key, labels.sort().join(", "));
  }

  return result;
}

// ---------------------------------------------------------------------------
// Helper: format DOT attributes
// ---------------------------------------------------------------------------

/**
 * Convert a Record of DOT attributes to the DOT attribute string format.
 *
 * Example: { shape: "circle", color: "red" } → `[shape=circle, color=red]`
 *
 * DOT attributes are key=value pairs inside square brackets. String values
 * are quoted if they contain special characters, but simple identifiers
 * like "circle" don't need quotes. We quote everything for safety.
 */
function formatDotAttrs(attrs: Record<string, string>): string {
  const parts = Object.entries(attrs)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([key, value]) => `${key}=${value}`);
  return `[${parts.join(", ")}]`;
}

// ===========================================================================
// toDot -- Graphviz DOT format
// ===========================================================================
//
// The DOT language is the standard input format for Graphviz, the most
// widely used graph visualization toolkit. A DOT file describes a graph
// using a simple text syntax:
//
//   digraph G {
//       A -> B;
//       B -> C;
//   }
//
// For labeled graphs, edges get label attributes:
//
//   A -> B [label="coin"];
//
// If multiple labels exist on the same edge pair, we combine them:
//
//   A -> B [label="coin, push"];
//
// The `initial` option adds a special invisible node that points to the
// initial state, which is the standard way to draw FSM diagrams:
//
//   "" [shape=none];
//   "" -> locked;

/**
 * Convert a graph to Graphviz DOT format.
 *
 * Works with both `Graph` and `LabeledDirectedGraph`. For labeled graphs,
 * edge labels are included as DOT edge attributes.
 *
 * @param graph - The graph to convert
 * @param options - DOT rendering options
 * @returns A string containing valid DOT source code
 *
 * @example
 * ```typescript
 * const g = new Graph();
 * g.addEdge("A", "B");
 * console.log(toDot(g));
 * // digraph G {
 * //     rankdir=LR;
 * //     A;
 * //     B;
 * //     A -> B;
 * // }
 * ```
 */
export function toDot(
  graph: Graph | LabeledDirectedGraph,
  options?: DotOptions
): string {
  // Apply defaults.
  const name = options?.name ?? "G";
  const rankdir = options?.rankdir ?? "LR";
  const nodeAttrs = options?.nodeAttrs ?? new Map();
  const initial = options?.initial;

  // Detect whether this is a labeled graph.
  const isLabeled = graph instanceof LabeledDirectedGraph;
  const edgeLabels = isLabeled
    ? collectEdgeLabels(graph as LabeledDirectedGraph)
    : new Map<string, string>();

  // Build the DOT output line by line.
  const lines: string[] = [];
  lines.push(`digraph ${name} {`);
  lines.push(`    rankdir=${rankdir};`);

  // If there's an initial state, add the invisible start node.
  // The empty string "" is a common DOT convention for invisible nodes.
  if (initial !== undefined) {
    lines.push(`    "" [shape=none];`);
    lines.push(`    "" -> ${initial};`);
  }

  // Emit node declarations (sorted for deterministic output).
  const nodes = isLabeled
    ? (graph as LabeledDirectedGraph).nodes()
    : (graph as Graph).nodes();
  const sortedNodes = [...nodes].sort();

  for (const node of sortedNodes) {
    if (nodeAttrs.has(node)) {
      lines.push(`    ${node} ${formatDotAttrs(nodeAttrs.get(node)!)};`);
    } else {
      lines.push(`    ${node};`);
    }
  }

  // Emit edges (sorted for deterministic output).
  if (isLabeled) {
    // For labeled graphs, we need to group edges by (from, to) pair
    // and combine their labels.
    const labeledGraph = graph as LabeledDirectedGraph;
    const structuralEdges: [string, string][] = [];
    const seen = new Set<string>();

    for (const [from, to] of labeledGraph.edges()) {
      const key = `${from}\0${to}`;
      if (!seen.has(key)) {
        seen.add(key);
        structuralEdges.push([from, to]);
      }
    }

    // Sort structural edges deterministically.
    structuralEdges.sort((a, b) => {
      if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
      return a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0;
    });

    for (const [from, to] of structuralEdges) {
      const key = `${from}\0${to}`;
      const label = edgeLabels.get(key);
      if (label) {
        lines.push(`    ${from} -> ${to} [label="${label}"];`);
      } else {
        lines.push(`    ${from} -> ${to};`);
      }
    }
  } else {
    // For unlabeled graphs, just emit each edge.
    const plainGraph = graph as Graph;
    const edges = plainGraph.edges();
    edges.sort((a, b) => {
      if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
      return a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0;
    });

    for (const [from, to] of edges) {
      lines.push(`    ${from} -> ${to};`);
    }
  }

  lines.push("}");
  return lines.join("\n");
}

// ===========================================================================
// toMermaid -- Mermaid flowchart format
// ===========================================================================
//
// Mermaid is a JavaScript-based diagramming tool that renders directly
// in Markdown. The syntax for a flowchart is:
//
//   graph LR
//       A --> B
//       B --> C
//
// For labeled edges:
//
//   A -->|coin| B
//
// Mermaid is popular because GitHub, GitLab, and many documentation tools
// render it natively -- no external tools needed.

/**
 * Convert a graph to Mermaid flowchart format.
 *
 * Works with both `Graph` and `LabeledDirectedGraph`. For labeled graphs,
 * edge labels are included using Mermaid's `-->|label|` syntax.
 *
 * @param graph - The graph to convert
 * @param options - Mermaid rendering options
 * @returns A string containing valid Mermaid source code
 *
 * @example
 * ```typescript
 * const g = new Graph();
 * g.addEdge("A", "B");
 * console.log(toMermaid(g));
 * // graph LR
 * //     A --> B
 * ```
 */
export function toMermaid(
  graph: Graph | LabeledDirectedGraph,
  options?: MermaidOptions
): string {
  const direction = options?.direction ?? "LR";
  const initial = options?.initial;

  const isLabeled = graph instanceof LabeledDirectedGraph;
  const edgeLabels = isLabeled
    ? collectEdgeLabels(graph as LabeledDirectedGraph)
    : new Map<string, string>();

  const lines: string[] = [];
  lines.push(`graph ${direction}`);

  // If there's an initial state, add a start marker.
  // We use Mermaid's circle syntax (( )) for the invisible start node.
  if (initial !== undefined) {
    lines.push(`    _start_(( )) --> ${initial}`);
  }

  if (isLabeled) {
    const labeledGraph = graph as LabeledDirectedGraph;
    const structuralEdges: [string, string][] = [];
    const seen = new Set<string>();

    for (const [from, to] of labeledGraph.edges()) {
      const key = `${from}\0${to}`;
      if (!seen.has(key)) {
        seen.add(key);
        structuralEdges.push([from, to]);
      }
    }

    structuralEdges.sort((a, b) => {
      if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
      return a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0;
    });

    for (const [from, to] of structuralEdges) {
      const key = `${from}\0${to}`;
      const label = edgeLabels.get(key);
      if (label) {
        lines.push(`    ${from} -->|${label}| ${to}`);
      } else {
        lines.push(`    ${from} --> ${to}`);
      }
    }
  } else {
    const plainGraph = graph as Graph;
    const edges = plainGraph.edges();
    edges.sort((a, b) => {
      if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
      return a[1] < b[1] ? -1 : a[1] > b[1] ? 1 : 0;
    });

    for (const [from, to] of edges) {
      lines.push(`    ${from} --> ${to}`);
    }
  }

  return lines.join("\n");
}

// ===========================================================================
// toAsciiTable -- Plain text table
// ===========================================================================
//
// For labeled graphs, we produce a transition table — the kind you'd see
// in a textbook when describing a finite state machine:
//
//   State      | coin      | push
//   -----------+-----------+----------
//   locked     | unlocked  | locked
//   unlocked   | unlocked  | locked
//
// Each row is a state (node), each column is a label (input symbol), and
// each cell is the destination state. If a transition doesn't exist, the
// cell shows "-" (a dash).
//
// If a state has multiple destinations for the same label (which shouldn't
// happen in a DFA but can happen in our general labeled graph), we join
// them with ", ".
//
// For unlabeled graphs, we produce a simpler adjacency list:
//
//   Node    | Successors
//   --------+-----------
//   A       | B, C
//   B       | D
//   C       | D
//   D       | -

/**
 * Convert a graph to an ASCII table string.
 *
 * For `LabeledDirectedGraph`: produces a transition table where rows are
 * nodes, columns are unique labels, and cells are destination nodes.
 *
 * For `Graph`: produces an adjacency list where rows are nodes and
 * the second column lists all successors.
 *
 * @param graph - The graph to convert
 * @returns A formatted ASCII table string
 *
 * @example
 * ```typescript
 * const g = new Graph();
 * g.addEdge("A", "B");
 * g.addEdge("A", "C");
 * console.log(toAsciiTable(g));
 * // Node | Successors
 * // -----+-----------
 * // A    | B, C
 * // B    | -
 * // C    | -
 * ```
 */
export function toAsciiTable(
  graph: Graph | LabeledDirectedGraph
): string {
  const isLabeled = graph instanceof LabeledDirectedGraph;

  if (isLabeled) {
    return labeledAsciiTable(graph as LabeledDirectedGraph);
  } else {
    return unlabeledAsciiTable(graph as Graph);
  }
}

// ---------------------------------------------------------------------------
// Helper: labeled graph ASCII transition table
// ---------------------------------------------------------------------------
//
// Building a transition table requires three steps:
//
// 1. Collect all unique labels (these become column headers).
// 2. For each node, for each label, find which nodes are reachable
//    via that label (these become cell values).
// 3. Calculate column widths and format everything with alignment.
//
// The column width calculation ensures everything lines up nicely:
//
//   width = max(header.length, max(cell_values.length for that column))
//
// We pad each cell to its column width with spaces.

function labeledAsciiTable(graph: LabeledDirectedGraph): string {
  const nodes = [...graph.nodes()].sort();
  const edges = graph.edges(); // [from, to, label][] sorted

  // Step 1: Collect all unique labels, sorted alphabetically.
  const labelSet = new Set<string>();
  for (const [, , label] of edges) {
    labelSet.add(label);
  }
  const labels = [...labelSet].sort();

  // Handle edge case: no labels means no columns beyond "State".
  if (labels.length === 0) {
    // Just show nodes with no transition columns.
    const header = "State";
    const maxWidth = Math.max(header.length, ...nodes.map((n) => n.length));
    const lines: string[] = [];
    lines.push(header.padEnd(maxWidth));
    lines.push("-".repeat(maxWidth));
    for (const node of nodes) {
      lines.push(node.padEnd(maxWidth));
    }
    return lines.join("\n");
  }

  // Step 2: Build the transition map.
  // transitions[node][label] = sorted list of destination nodes.
  const transitions = new Map<string, Map<string, string[]>>();
  for (const node of nodes) {
    transitions.set(node, new Map());
    for (const label of labels) {
      transitions.get(node)!.set(label, []);
    }
  }

  for (const [from, to, label] of edges) {
    const nodeMap = transitions.get(from);
    if (nodeMap) {
      const existing = nodeMap.get(label) ?? [];
      if (!existing.includes(to)) {
        existing.push(to);
        existing.sort();
      }
      nodeMap.set(label, existing);
    }
  }

  // Step 3: Calculate column widths.
  //
  // The first column is "State", and subsequent columns are label names.
  // Each column must be wide enough for:
  //   - The header text
  //   - The longest cell value in that column
  const stateColWidth = Math.max(
    "State".length,
    ...nodes.map((n) => n.length)
  );

  const labelColWidths = labels.map((label) => {
    const cellWidths = nodes.map((node) => {
      const dests = transitions.get(node)!.get(label)!;
      return dests.length > 0 ? dests.join(", ").length : 1; // "-" is 1 char
    });
    return Math.max(label.length, ...cellWidths);
  });

  // Step 4: Build the formatted table.
  const lines: string[] = [];

  // Header row: "State      | coin      | push"
  const headerParts = ["State".padEnd(stateColWidth)];
  for (let i = 0; i < labels.length; i++) {
    headerParts.push(labels[i].padEnd(labelColWidths[i]));
  }
  lines.push(headerParts.join(" | "));

  // Separator row: "-----------+-----------+----------"
  const sepParts = ["-".repeat(stateColWidth)];
  for (let i = 0; i < labels.length; i++) {
    sepParts.push("-".repeat(labelColWidths[i]));
  }
  lines.push(sepParts.join("-+-"));

  // Data rows: "locked     | unlocked  | locked"
  for (const node of nodes) {
    const rowParts = [node.padEnd(stateColWidth)];
    for (let i = 0; i < labels.length; i++) {
      const dests = transitions.get(node)!.get(labels[i])!;
      const cellText = dests.length > 0 ? dests.join(", ") : "-";
      rowParts.push(cellText.padEnd(labelColWidths[i]));
    }
    lines.push(rowParts.join(" | "));
  }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Helper: unlabeled graph ASCII adjacency list
// ---------------------------------------------------------------------------
//
// For unlabeled graphs, the table is simpler: just two columns.
// "Node" and "Successors".

function unlabeledAsciiTable(graph: Graph): string {
  const nodes = [...graph.nodes()].sort();

  // Build successor lists for each node.
  const successorMap = new Map<string, string>();
  for (const node of nodes) {
    const succs = graph.successors(node);
    succs.sort();
    successorMap.set(node, succs.length > 0 ? succs.join(", ") : "-");
  }

  // Calculate column widths.
  const nodeColWidth = Math.max(
    "Node".length,
    ...nodes.map((n) => n.length)
  );
  const succColWidth = Math.max(
    "Successors".length,
    ...nodes.map((n) => successorMap.get(n)!.length)
  );

  // Build the table.
  const lines: string[] = [];
  lines.push(
    `${"Node".padEnd(nodeColWidth)} | ${"Successors".padEnd(succColWidth)}`
  );
  lines.push(`${"-".repeat(nodeColWidth)}-+-${"-".repeat(succColWidth)}`);

  for (const node of nodes) {
    lines.push(
      `${node.padEnd(nodeColWidth)} | ${successorMap.get(node)!.padEnd(succColWidth)}`
    );
  }

  return lines.join("\n");
}
