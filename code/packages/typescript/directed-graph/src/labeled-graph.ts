/**
 * labeled-graph.ts -- A Directed Graph Where Edges Carry Labels
 * ==============================================================
 *
 * What is a labeled graph?
 * ------------------------
 *
 * A regular directed graph says "there IS an edge from A to B."  A labeled
 * directed graph says "there is an edge from A to B *with label L*."
 * Multiple labels can exist on the same pair of nodes, and the same label
 * can appear on different edges.
 *
 * Why do we need this?
 * --------------------
 *
 * Labeled graphs are essential for modeling systems where the *kind* of
 * relationship matters, not just its existence.  The canonical example is
 * a finite state machine (FSM):
 *
 *   - Nodes are states (e.g. "locked", "unlocked")
 *   - Edge labels are input symbols (e.g. "coin", "push")
 *   - The edge ("locked", "unlocked", "coin") means:
 *     "when in state 'locked' and input is 'coin', go to 'unlocked'"
 *
 * Self-loops are allowed because an FSM state can transition back to
 * itself (e.g. inserting a coin in an already-unlocked turnstile).
 *
 * Design: composition over inheritance
 * -------------------------------------
 *
 * Rather than inheriting from Graph, we *wrap* a Graph instance.  This
 * is the "composition" pattern — LabeledDirectedGraph HAS-A Graph rather
 * than IS-A Graph.  We do this because:
 *
 *   1. We need to set `allowSelfLoops: true` on the inner graph.
 *   2. The edge semantics are different — we store (from, to, label)
 *      triples, not just (from, to) pairs.
 *   3. We can still delegate algorithms like topologicalSort directly
 *      to the inner graph, since those only care about node connectivity.
 *
 * Internal storage
 * ----------------
 *
 * The labels are stored in a Map keyed by a composite string:
 *
 *   key = `${fromNode}\0${toNode}`
 *
 * We use the null byte (\0) as a separator because it cannot appear in
 * normal string node names, guaranteeing no collisions.  Each key maps
 * to a Set<string> of labels on that edge.
 *
 * The inner Graph handles node storage and adjacency.  The labels Map
 * adds the extra dimension of *which* labels exist on each edge.
 */

import {
  Graph,
  CycleError,
  NodeNotFoundError,
  EdgeNotFoundError,
} from "./graph.js";

// Re-export errors so consumers can import from labeled-graph directly.
export { CycleError, NodeNotFoundError, EdgeNotFoundError };

/**
 * Build the composite key for the labels map.
 *
 * We use the null byte as separator because it's guaranteed not to
 * appear in normal string node names.  This pattern matches the
 * state-machine package's approach.
 */
function edgeKey(fromNode: string, toNode: string): string {
  return `${fromNode}\0${toNode}`;
}

/**
 * A directed graph where every edge carries one or more string labels.
 *
 * Think of it as a multi-labeled multigraph: the same (from, to) pair
 * can have multiple labels, and self-loops are allowed.
 *
 * Example (turnstile FSM):
 *
 *     const fsm = new LabeledDirectedGraph();
 *     fsm.addEdge("locked", "unlocked", "coin");
 *     fsm.addEdge("locked", "locked", "push");
 *     fsm.addEdge("unlocked", "locked", "push");
 *     fsm.addEdge("unlocked", "unlocked", "coin");
 *
 *     fsm.successors("locked", "coin");  // ["unlocked"]
 *     fsm.labels("locked", "unlocked");  // Set { "coin" }
 */
export class LabeledDirectedGraph {
  // ------------------------------------------------------------------
  // Internal storage
  // ------------------------------------------------------------------

  /** The underlying graph that stores structural edges and nodes. */
  private readonly _graph: Graph;

  /**
   * Maps "fromNode\0toNode" -> Set of labels on that edge.
   *
   * This is the "extra dimension" that turns a plain graph into a
   * labeled graph.  The inner Graph tracks connectivity; this Map
   * tracks what labels each connection carries.
   */
  private readonly _labels: Map<string, Set<string>>;

  constructor() {
    this._graph = new Graph({ allowSelfLoops: true });
    this._labels = new Map();
  }

  // ------------------------------------------------------------------
  // Node operations — delegated to the inner graph
  // ------------------------------------------------------------------

  /**
   * Add a node to the graph.  No-op if it already exists.
   */
  addNode(node: string): void {
    this._graph.addNode(node);
  }

  /**
   * Remove a node and ALL edges (with all labels) touching it.
   *
   * We must clean up the labels Map entries for any edge involving
   * this node — both outgoing and incoming, plus any self-loop.
   *
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  removeNode(node: string): void {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    // Clean up label entries for outgoing edges.
    for (const succ of this._graph.successors(node)) {
      this._labels.delete(edgeKey(node, succ));
    }

    // Clean up label entries for incoming edges.
    for (const pred of this._graph.predecessors(node)) {
      this._labels.delete(edgeKey(pred, node));
    }

    // Clean up self-loop (may already be deleted above, but safe to call).
    this._labels.delete(edgeKey(node, node));

    this._graph.removeNode(node);
  }

  /**
   * Return true if the node exists in the graph.
   */
  hasNode(node: string): boolean {
    return this._graph.hasNode(node);
  }

  /**
   * Return a list of all nodes in the graph.
   */
  nodes(): string[] {
    return this._graph.nodes();
  }

  /**
   * Return the number of nodes in the graph.
   */
  get size(): number {
    return this._graph.size;
  }

  // ------------------------------------------------------------------
  // Labeled edge operations
  // ------------------------------------------------------------------

  /**
   * Add a labeled edge from `fromNode` to `toNode` with `label`.
   *
   * Both nodes are implicitly created if they don't exist.  If the
   * exact same (from, to, label) triple already exists, this is a
   * no-op (Set deduplication on the label set).
   *
   * Adding a second label to the same (from, to) pair creates a
   * multi-labeled edge — the underlying graph still has one structural
   * edge, but the labels Set grows.
   */
  addEdge(fromNode: string, toNode: string, label: string): void {
    // Ensure the structural edge exists in the inner graph.
    if (!this._graph.hasEdge(fromNode, toNode)) {
      this._graph.addEdge(fromNode, toNode);
    }

    const key = edgeKey(fromNode, toNode);
    if (!this._labels.has(key)) {
      this._labels.set(key, new Set());
    }
    this._labels.get(key)!.add(label);
  }

  /**
   * Remove a specific labeled edge.
   *
   * If this was the last label on the (from, to) pair, the structural
   * edge is also removed from the inner graph.
   *
   * Throws `NodeNotFoundError` if either node doesn't exist.
   * Throws `EdgeNotFoundError` if the (from, to, label) triple doesn't exist.
   */
  removeEdge(fromNode: string, toNode: string, label: string): void {
    if (!this._graph.hasNode(fromNode)) {
      throw new NodeNotFoundError(fromNode);
    }
    if (!this._graph.hasNode(toNode)) {
      throw new NodeNotFoundError(toNode);
    }

    const key = edgeKey(fromNode, toNode);
    const labelSet = this._labels.get(key);
    if (!labelSet || !labelSet.has(label)) {
      throw new EdgeNotFoundError(fromNode, toNode);
    }

    labelSet.delete(label);

    // If no labels remain, remove the structural edge too.
    if (labelSet.size === 0) {
      this._labels.delete(key);
      this._graph.removeEdge(fromNode, toNode);
    }
  }

  /**
   * Check whether an edge exists.
   *
   * - Without label: returns true if ANY edge from -> to exists.
   * - With label: returns true if the exact (from, to, label) triple exists.
   */
  hasEdge(fromNode: string, toNode: string, label?: string): boolean {
    if (label === undefined) {
      return this._graph.hasEdge(fromNode, toNode);
    }
    const key = edgeKey(fromNode, toNode);
    const labelSet = this._labels.get(key);
    return labelSet !== undefined && labelSet.has(label);
  }

  /**
   * Return all edges as [from, to, label] triples, sorted for
   * deterministic output.
   *
   * If a pair (A, B) has labels {"x", "y"}, we return two triples:
   *   ["A", "B", "x"] and ["A", "B", "y"]
   */
  edges(): [string, string, string][] {
    const result: [string, string, string][] = [];
    for (const [key, labelSet] of this._labels) {
      const parts = key.split("\0");
      const from = parts[0];
      const to = parts[1];
      for (const label of labelSet) {
        result.push([from, to, label]);
      }
    }
    result.sort((a, b) => {
      if (a[0] !== b[0]) return a[0] < b[0] ? -1 : 1;
      if (a[1] !== b[1]) return a[1] < b[1] ? -1 : 1;
      return a[2] < b[2] ? -1 : a[2] > b[2] ? 1 : 0;
    });
    return result;
  }

  /**
   * Return the Set of labels on the edge from `fromNode` to `toNode`.
   *
   * Returns an empty Set if no edge exists between the two nodes.
   *
   * Throws `NodeNotFoundError` if either node doesn't exist.
   */
  labels(fromNode: string, toNode: string): Set<string> {
    if (!this._graph.hasNode(fromNode)) {
      throw new NodeNotFoundError(fromNode);
    }
    if (!this._graph.hasNode(toNode)) {
      throw new NodeNotFoundError(toNode);
    }

    const key = edgeKey(fromNode, toNode);
    const labelSet = this._labels.get(key);
    return labelSet ? new Set(labelSet) : new Set();
  }

  // ------------------------------------------------------------------
  // Neighbor queries with optional label filtering
  // ------------------------------------------------------------------

  /**
   * Return successors of `node`, optionally filtered by `label`.
   *
   * - Without label: returns ALL direct successors.
   * - With label: returns only successors reachable via an edge
   *   carrying that specific label.
   *
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  successors(node: string, label?: string): string[] {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    if (label === undefined) {
      return this._graph.successors(node);
    }

    const result: string[] = [];
    for (const succ of this._graph.successors(node)) {
      const key = edgeKey(node, succ);
      const labelSet = this._labels.get(key);
      if (labelSet && labelSet.has(label)) {
        result.push(succ);
      }
    }
    return result.sort();
  }

  /**
   * Return predecessors of `node`, optionally filtered by `label`.
   *
   * - Without label: returns ALL direct predecessors.
   * - With label: returns only predecessors connected via an edge
   *   carrying that specific label.
   *
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  predecessors(node: string, label?: string): string[] {
    if (!this._graph.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }

    if (label === undefined) {
      return this._graph.predecessors(node);
    }

    const result: string[] = [];
    for (const pred of this._graph.predecessors(node)) {
      const key = edgeKey(pred, node);
      const labelSet = this._labels.get(key);
      if (labelSet && labelSet.has(label)) {
        result.push(pred);
      }
    }
    return result.sort();
  }

  // ------------------------------------------------------------------
  // Algorithm delegation
  // ------------------------------------------------------------------
  // Graph algorithms operate on the structural topology (which nodes
  // are connected to which), not on labels.  We delegate directly to
  // the inner Graph.

  /**
   * Return a topological ordering of all nodes.
   * Throws `CycleError` if the graph contains a cycle.
   */
  topologicalSort(): string[] {
    return this._graph.topologicalSort();
  }

  /**
   * Return true if the graph contains at least one cycle.
   */
  hasCycle(): boolean {
    return this._graph.hasCycle();
  }

  /**
   * Return all nodes reachable downstream from `node`.
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  transitiveClosure(node: string): Set<string> {
    return this._graph.transitiveClosure(node);
  }
}
