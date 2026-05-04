/**
 * graph.ts -- Directed Graph with Built-in Algorithms
 * ====================================================
 *
 * This module contains the entire directed graph implementation: the data structure,
 * mutation methods, query methods, and graph algorithms. We keep everything in one
 * class because the algorithms need intimate access to the internal adjacency maps,
 * and splitting them into a separate module would just add indirection without any
 * real benefit.
 *
 * Internal Storage
 * ----------------
 *
 * We maintain **two** adjacency maps:
 *
 * - `_forward.get(u)`  = set of nodes that `u` points TO   (successors / children)
 * - `_reverse.get(v)`  = set of nodes that point TO `v`     (predecessors / parents)
 *
 * Every node that exists in the graph has an entry in both maps, even if its
 * adjacency set is empty. This invariant lets us use `this._forward.has(node)`
 * as the canonical "does this node exist?" check, and it means we never need
 * to special-case missing keys.
 *
 * Why two maps? Because many of our algorithms need to walk edges in *both*
 * directions efficiently:
 *
 * - `topologicalSort` needs to find nodes with zero in-degree, which means
 *   checking `this._reverse.get(node)!.size === 0` -- O(1) with the reverse map.
 * - `transitiveDependents` walks *backwards* from a node, which is just a
 *   forward traversal on `_reverse`.
 * - `removeNode` needs to clean up both incoming and outgoing edges, which
 *   is O(degree) with both maps but would be O(E) with only one.
 *
 * The trade-off is that every `addEdge` and `removeEdge` must update both
 * maps, but that's O(1) per operation, so it's a good deal.
 *
 * Error Classes
 * -------------
 *
 * We define three custom error classes:
 *
 * - `CycleError` -- thrown when a topological sort is requested on a graph that
 *   contains a cycle. It stores the cycle path so the caller can report which
 *   nodes are involved.
 * - `NodeNotFoundError` -- thrown when an operation references a node that
 *   doesn't exist in the graph (e.g., `removeNode("X")` when X was never added).
 * - `EdgeNotFoundError` -- thrown when `removeEdge(u, v)` is called but
 *   the edge u -> v doesn't exist.
 */

// ---------------------------------------------------------------------------
// Custom error classes
// ---------------------------------------------------------------------------
// Each error carries enough context for the caller to produce a useful
// error message. We inherit from Error because these are expected,
// recoverable errors -- not bugs in the library.

/**
 * Thrown when a topological sort encounters a cycle.
 *
 * The `cycle` property contains a list of nodes forming the cycle,
 * starting and ending with the same node. For example, if the graph has
 * edges A -> B -> C -> A, the cycle might be `["A", "B", "C", "A"]`.
 */
export class CycleError extends Error {
  public readonly cycle: string[];

  constructor(message: string, cycle: string[]) {
    super(message);
    this.name = "CycleError";
    this.cycle = cycle;
  }
}

/**
 * Thrown when an operation references a node not in the graph.
 *
 * The `node` property contains the missing node value.
 */
export class NodeNotFoundError extends Error {
  public readonly node: string;

  constructor(node: string) {
    super(`Node not found: "${node}"`);
    this.name = "NodeNotFoundError";
    this.node = node;
  }
}

/**
 * Thrown when `removeEdge` targets a nonexistent edge.
 *
 * The `fromNode` and `toNode` properties identify the missing edge.
 */
export class EdgeNotFoundError extends Error {
  public readonly fromNode: string;
  public readonly toNode: string;

  constructor(fromNode: string, toNode: string) {
    super(`Edge not found: "${fromNode}" -> "${toNode}"`);
    this.name = "EdgeNotFoundError";
    this.fromNode = fromNode;
    this.toNode = toNode;
  }
}

export type GraphPropertyValue = string | number | boolean | null;
export type GraphPropertyBag = Record<string, GraphPropertyValue>;
export type WeightedEdge = readonly [string, string, number];

function edgeKey(fromNode: string, toNode: string): string {
  return `${fromNode}\0${toNode}`;
}

// ---------------------------------------------------------------------------
// The Graph class
// ---------------------------------------------------------------------------

/**
 * A directed graph backed by forward and reverse adjacency maps.
 *
 * The graph stores string-typed nodes. Edges are directed: `addEdge("A", "B")`
 * means A points to B, so B is a *successor* of A and A is a *predecessor* of B.
 *
 * Self-loops are disallowed -- `addEdge("A", "A")` throws an `Error`.
 * Duplicate nodes are silently ignored. Re-adding an edge updates its weight
 * and merges metadata into the existing edge property bag.
 *
 * Example:
 *
 *     const g = new Graph();
 *     g.addEdge("compile", "link");
 *     g.addEdge("link", "package");
 *     console.log(g.topologicalSort());   // ['compile', 'link', 'package']
 */
export class Graph {
  // ------------------------------------------------------------------
  // Internal storage
  // ------------------------------------------------------------------
  // We start with empty maps. The invariant is: every node that exists
  // in the graph has a key in BOTH _forward and _reverse.

  private readonly _forward: Map<string, Map<string, number>> = new Map();
  private readonly _reverse: Map<string, Map<string, number>> = new Map();
  private readonly _graphProperties: GraphPropertyBag = {};
  private readonly _nodeProperties: Map<string, GraphPropertyBag> = new Map();
  private readonly _edgeProperties: Map<string, GraphPropertyBag> = new Map();

  // ------------------------------------------------------------------
  // Self-loop control
  // ------------------------------------------------------------------
  // By default, self-loops (edges where fromNode === toNode) are
  // forbidden because they represent cycles of length 1, which break
  // topological sort and most DAG algorithms.
  //
  // However, some use cases — like labeled graphs modeling state
  // machines — genuinely need self-loops (e.g. a state that transitions
  // back to itself on certain inputs).
  //
  // Pass `{ allowSelfLoops: true }` to the constructor to permit them.

  private readonly _allowSelfLoops: boolean;

  constructor(options?: { allowSelfLoops?: boolean }) {
    this._allowSelfLoops = options?.allowSelfLoops ?? false;
  }

  /**
   * Returns true if self-loops are permitted in this graph.
   */
  get allowSelfLoops(): boolean {
    return this._allowSelfLoops;
  }

  // ------------------------------------------------------------------
  // Node operations
  // ------------------------------------------------------------------

  /**
   * Add a node to the graph. No-op if the node already exists.
   *
   * This is called implicitly by `addEdge`, so you only need to call
   * it directly for isolated nodes (nodes with no edges).
   */
  addNode(node: string, properties: GraphPropertyBag = {}): void {
    if (!this._forward.has(node)) {
      this._forward.set(node, new Map());
      this._reverse.set(node, new Map());
      this._nodeProperties.set(node, {});
    }
    this.mergeNodeProperties(node, properties);
  }

  /**
   * Remove a node and all its incoming/outgoing edges.
   *
   * Throws `NodeNotFoundError` if the node doesn't exist.
   *
   * This is O(in-degree + out-degree) because we need to update the
   * adjacency sets of all neighbors.
   */
  removeNode(node: string): void {
    if (!this._forward.has(node)) {
      throw new NodeNotFoundError(node);
    }

    // Clean up outgoing edges: for each successor, remove `node` from
    // that successor's reverse (predecessor) set.
    for (const successor of this._forward.get(node)!.keys()) {
      this._reverse.get(successor)!.delete(node);
      this._edgeProperties.delete(edgeKey(node, successor));
    }

    // Clean up incoming edges: for each predecessor, remove `node` from
    // that predecessor's forward (successor) set.
    for (const predecessor of this._reverse.get(node)!.keys()) {
      this._forward.get(predecessor)!.delete(node);
      this._edgeProperties.delete(edgeKey(predecessor, node));
    }

    // Finally, remove the node itself from both maps.
    this._forward.delete(node);
    this._reverse.delete(node);
    this._nodeProperties.delete(node);
  }

  /**
   * Return true if the node exists in the graph.
   */
  hasNode(node: string): boolean {
    return this._forward.has(node);
  }

  /**
   * Return a list of all nodes in the graph.
   *
   * The order follows Map insertion order, but we don't guarantee it.
   */
  nodes(): string[] {
    return Array.from(this._forward.keys());
  }

  // ------------------------------------------------------------------
  // Edge operations
  // ------------------------------------------------------------------

  /**
   * Add a directed edge from `fromNode` to `toNode`.
   *
   * Both nodes are implicitly added if they don't exist yet. This means
   * you can build a graph entirely with `addEdge` calls -- no need
   * to call `addNode` first.
   *
   * Throws `Error` if `fromNode === toNode` (self-loops are not allowed
   * in a DAG-oriented graph).
   *
   * Duplicate edges are silently ignored (Sets handle deduplication).
   */
  addEdge(
    fromNode: string,
    toNode: string,
    weight = 1.0,
    properties: GraphPropertyBag = {}
  ): void {
    if (fromNode === toNode && !this._allowSelfLoops) {
      throw new Error(
        `Self-loops are not allowed: "${fromNode}" -> "${toNode}"`
      );
    }
    this.validateWeight(weight);

    // Ensure both nodes exist (idempotent).
    this.addNode(fromNode);
    this.addNode(toNode);

    // Add the edge to both adjacency maps.
    this._forward.get(fromNode)!.set(toNode, weight);
    this._reverse.get(toNode)!.set(fromNode, weight);
    this.mergeEdgeProperties(fromNode, toNode, { ...properties, weight });
  }

  /**
   * Remove the directed edge from `fromNode` to `toNode`.
   *
   * Throws `EdgeNotFoundError` if the edge doesn't exist (including
   * if either node doesn't exist).
   */
  removeEdge(fromNode: string, toNode: string): void {
    if (
      !this._forward.has(fromNode) ||
      !this._forward.get(fromNode)!.has(toNode)
    ) {
      throw new EdgeNotFoundError(fromNode, toNode);
    }

    this._forward.get(fromNode)!.delete(toNode);
    this._reverse.get(toNode)!.delete(fromNode);
    this._edgeProperties.delete(edgeKey(fromNode, toNode));
  }

  /**
   * Return true if the directed edge fromNode -> toNode exists.
   */
  hasEdge(fromNode: string, toNode: string): boolean {
    return (
      this._forward.has(fromNode) &&
      this._forward.get(fromNode)!.has(toNode)
    );
  }

  /**
   * Return a list of all edges as [fromNode, toNode] tuples.
   *
   * The order is arbitrary.
   */
  edges(): [string, string][] {
    const result: [string, string][] = [];
    for (const [node, successors] of this._forward) {
      for (const successor of successors.keys()) {
        result.push([node, successor]);
      }
    }
    return result;
  }

  /**
   * Return a list of all directed edges as [fromNode, toNode, weight] tuples.
   */
  edgesWeighted(): WeightedEdge[] {
    const result: WeightedEdge[] = [];
    for (const [node, successors] of this._forward) {
      for (const [successor, weight] of successors) {
        result.push([node, successor, weight]);
      }
    }
    return result;
  }

  /**
   * Return the weight of the directed edge fromNode -> toNode.
   */
  edgeWeight(fromNode: string, toNode: string): number {
    const weight = this._forward.get(fromNode)?.get(toNode);
    if (weight === undefined) {
      throw new EdgeNotFoundError(fromNode, toNode);
    }
    return weight;
  }

  // ------------------------------------------------------------------
  // Property bags
  // ------------------------------------------------------------------

  /**
   * Return a copy of graph-level properties.
   */
  graphProperties(): GraphPropertyBag {
    return { ...this._graphProperties };
  }

  /**
   * Set one graph-level property.
   */
  setGraphProperty(key: string, value: GraphPropertyValue): void {
    this._graphProperties[key] = value;
  }

  /**
   * Remove one graph-level property if present.
   */
  removeGraphProperty(key: string): void {
    delete this._graphProperties[key];
  }

  /**
   * Return a copy of properties attached to a node.
   */
  nodeProperties(node: string): GraphPropertyBag {
    this.assertNode(node);
    return { ...(this._nodeProperties.get(node) ?? {}) };
  }

  /**
   * Set one property on a node.
   */
  setNodeProperty(
    node: string,
    key: string,
    value: GraphPropertyValue
  ): void {
    this.assertNode(node);
    this._nodeProperties.get(node)![key] = value;
  }

  /**
   * Remove one property from a node if present.
   */
  removeNodeProperty(node: string, key: string): void {
    this.assertNode(node);
    delete this._nodeProperties.get(node)![key];
  }

  /**
   * Return a copy of properties attached to directed edge fromNode -> toNode.
   */
  edgeProperties(fromNode: string, toNode: string): GraphPropertyBag {
    this.assertEdge(fromNode, toNode);
    const properties =
      this._edgeProperties.get(edgeKey(fromNode, toNode)) ?? {};
    return { ...properties, weight: this.edgeWeight(fromNode, toNode) };
  }

  /**
   * Set one property on directed edge fromNode -> toNode.
   * Setting "weight" also updates edgeWeight and weighted traversal state.
   */
  setEdgeProperty(
    fromNode: string,
    toNode: string,
    key: string,
    value: GraphPropertyValue
  ): void {
    this.assertEdge(fromNode, toNode);
    if (key === "weight") {
      if (typeof value !== "number" || Number.isNaN(value)) {
        throw new Error("Edge property 'weight' must be a number");
      }
      this.setEdgeWeight(fromNode, toNode, value);
    }
    this.mergeEdgeProperties(fromNode, toNode, { [key]: value });
  }

  /**
   * Remove one property from directed edge fromNode -> toNode if present.
   * Removing "weight" resets the edge weight to 1.0.
   */
  removeEdgeProperty(fromNode: string, toNode: string, key: string): void {
    this.assertEdge(fromNode, toNode);
    if (key === "weight") {
      this.setEdgeWeight(fromNode, toNode, 1.0);
      this.mergeEdgeProperties(fromNode, toNode, { weight: 1.0 });
      return;
    }
    delete this._edgeProperties.get(edgeKey(fromNode, toNode))?.[key];
  }

  // ------------------------------------------------------------------
  // Neighbor queries
  // ------------------------------------------------------------------

  /**
   * Return the direct predecessors (parents) of a node.
   *
   * These are the nodes that have an edge pointing TO this node.
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  predecessors(node: string): string[] {
    if (!this._reverse.has(node)) {
      throw new NodeNotFoundError(node);
    }
    return Array.from(this._reverse.get(node)!.keys());
  }

  /**
   * Return the direct successors (children) of a node.
   *
   * These are the nodes that this node points TO.
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  successors(node: string): string[] {
    if (!this._forward.has(node)) {
      throw new NodeNotFoundError(node);
    }
    return Array.from(this._forward.get(node)!.keys());
  }

  /**
   * Return successor weights as a Map of successor -> edge weight.
   */
  successorsWeighted(node: string): ReadonlyMap<string, number> {
    if (!this._forward.has(node)) {
      throw new NodeNotFoundError(node);
    }
    return new Map(this._forward.get(node)!);
  }

  // ------------------------------------------------------------------
  // Utility methods
  // ------------------------------------------------------------------

  /**
   * Return the number of nodes in the graph.
   */
  get size(): number {
    return this._forward.size;
  }

  /**
   * Return a string representation showing node and edge counts.
   */
  toString(): string {
    return `Graph(nodes=${this.size}, edges=${this.edges().length})`;
  }

  // ==================================================================
  // ALGORITHMS
  // ==================================================================
  // All algorithms are methods on the graph itself. This keeps the API
  // simple: you just call g.topologicalSort() instead of importing a
  // separate module.

  // ------------------------------------------------------------------
  // Topological Sort (Kahn's Algorithm)
  // ------------------------------------------------------------------
  //
  // Kahn's algorithm works by repeatedly removing nodes with zero
  // in-degree from the graph. The order in which we remove them is a
  // valid topological ordering.
  //
  // Why Kahn's instead of DFS-based? Two reasons:
  // 1. It naturally detects cycles (if we can't remove all nodes, there's
  //    a cycle).
  // 2. It's easier to modify for independentGroups (see below).
  //
  // Time complexity: O(V + E) where V = nodes, E = edges.

  /**
   * Return a topological ordering of all nodes.
   *
   * A topological ordering is a linear sequence where for every edge
   * u -> v, u appears before v. This only exists for DAGs (directed
   * acyclic graphs).
   *
   * Throws `CycleError` if the graph contains a cycle. The error
   * includes the cycle path.
   *
   * For an empty graph, returns an empty list.
   */
  topologicalSort(): string[] {
    // We work on copies of the in-degree counts so we don't mutate the
    // actual graph. This is a "virtual" removal -- we just decrement
    // counters instead of actually removing nodes.

    const inDegree = new Map<string, number>();
    for (const [node, preds] of this._reverse) {
      inDegree.set(node, preds.size);
    }

    // Start with all nodes that have zero in-degree (no dependencies).
    const queue: string[] = [];
    for (const [node, degree] of inDegree) {
      if (degree === 0) {
        queue.push(node);
      }
    }

    const result: string[] = [];

    while (queue.length > 0) {
      // Pick a node with zero in-degree. Sorting ensures deterministic
      // output when multiple nodes have zero in-degree.
      const node = queue.shift()!;
      result.push(node);

      // "Remove" this node by decrementing the in-degree of all its
      // successors. If any successor's in-degree drops to zero, it's
      // ready to be processed.
      const sortedSuccessors = Array.from(this._forward.get(node)!.keys()).sort();
      for (const successor of sortedSuccessors) {
        inDegree.set(successor, inDegree.get(successor)! - 1);
        if (inDegree.get(successor)! === 0) {
          queue.push(successor);
        }
      }
    }

    // If we couldn't process all nodes, there's a cycle. Find it using
    // DFS so we can report the actual cycle path.
    if (result.length !== this._forward.size) {
      const cycle = this._findCycle();
      throw new CycleError(
        `Graph contains a cycle: ${cycle.join(" -> ")}`,
        cycle
      );
    }

    return result;
  }

  // ------------------------------------------------------------------
  // Cycle Detection (DFS Three-Color Algorithm)
  // ------------------------------------------------------------------
  //
  // The three-color algorithm uses:
  // - WHITE (0): not yet visited
  // - GRAY  (1): currently being explored (on the recursion stack)
  // - BLACK (2): fully explored
  //
  // If we encounter a GRAY node during DFS, we've found a back edge,
  // which means there's a cycle.

  private static readonly WHITE = 0;
  private static readonly GRAY = 1;
  private static readonly BLACK = 2;

  /**
   * Return true if the graph contains at least one cycle.
   *
   * Uses DFS with three-color marking. This is O(V + E).
   */
  hasCycle(): boolean {
    const color = new Map<string, number>();
    for (const node of this._forward.keys()) {
      color.set(node, Graph.WHITE);
    }

    const dfs = (node: string): boolean => {
      /**Return true if a cycle is reachable from this node.*/
      color.set(node, Graph.GRAY);

      for (const successor of this._forward.get(node)!.keys()) {
        if (color.get(successor) === Graph.GRAY) {
          // Back edge found -- cycle!
          return true;
        }
        if (color.get(successor) === Graph.WHITE && dfs(successor)) {
          return true;
        }
      }

      color.set(node, Graph.BLACK);
      return false;
    };

    // We need to start DFS from every unvisited node because the graph
    // might not be connected.
    for (const node of this._forward.keys()) {
      if (color.get(node) === Graph.WHITE) {
        if (dfs(node)) {
          return true;
        }
      }
    }

    return false;
  }

  /**
   * Find and return a cycle path using DFS.
   *
   * This is a private helper used by `topologicalSort` to provide
   * a useful error message. It returns a list like [A, B, C, A] where
   * A -> B -> C -> A is the cycle.
   */
  private _findCycle(): string[] {
    const color = new Map<string, number>();
    const parent = new Map<string, string | null>();
    for (const node of this._forward.keys()) {
      color.set(node, Graph.WHITE);
      parent.set(node, null);
    }

    const dfs = (node: string): string[] | null => {
      color.set(node, Graph.GRAY);

      const sortedSuccessors = Array.from(this._forward.get(node)!.keys()).sort();
      for (const successor of sortedSuccessors) {
        if (color.get(successor) === Graph.GRAY) {
          // Found the cycle! Reconstruct the path.
          const cycle: string[] = [successor, node];
          let current: string | null = node;
          while (current !== successor) {
            current = parent.get(current!) ?? null;
            if (current === null) {
              break;
            }
            cycle.push(current);
          }
          cycle.reverse();
          cycle.push(successor); // Close the cycle
          return cycle;
        }
        if (color.get(successor) === Graph.WHITE) {
          parent.set(successor, node);
          const result = dfs(successor);
          if (result !== null) {
            return result;
          }
        }
      }

      color.set(node, Graph.BLACK);
      return null;
    };

    const sortedNodes = Array.from(this._forward.keys()).sort();
    for (const node of sortedNodes) {
      if (color.get(node) === Graph.WHITE) {
        const result = dfs(node);
        if (result !== null) {
          return result;
        }
      }
    }

    return []; // Should never reach here if called when a cycle exists
  }

  // ------------------------------------------------------------------
  // Transitive Closure
  // ------------------------------------------------------------------
  //
  // The transitive closure of a node is the set of all nodes reachable
  // from it by following edges forward. We use BFS because it's simple
  // and doesn't risk stack overflow on deep graphs.

  /**
   * Return all nodes reachable downstream from `node`.
   *
   * This follows edges in the forward direction. The starting node is
   * NOT included in the result (only the nodes it can reach).
   *
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  transitiveClosure(node: string): Set<string> {
    if (!this._forward.has(node)) {
      throw new NodeNotFoundError(node);
    }

    const visited = new Set<string>();
    const queue: string[] = Array.from(this._forward.get(node)!.keys());
    for (const n of queue) {
      visited.add(n);
    }

    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const successor of this._forward.get(current)!.keys()) {
        if (!visited.has(successor)) {
          visited.add(successor);
          queue.push(successor);
        }
      }
    }

    return visited;
  }

  // ------------------------------------------------------------------
  // Transitive Dependents (Reverse Transitive Closure)
  // ------------------------------------------------------------------
  //
  // This is the mirror of transitiveClosure: instead of asking "what
  // does this node depend on?", we ask "what depends on this node?"
  // We just walk the reverse adjacency map instead of the forward one.

  /**
   * Return all nodes that transitively depend on `node`.
   *
   * This follows edges in the REVERSE direction -- it finds everything
   * upstream that would be affected if `node` changed.
   *
   * The starting node is NOT included in the result.
   *
   * Throws `NodeNotFoundError` if the node doesn't exist.
   */
  transitiveDependents(node: string): Set<string> {
    if (!this._reverse.has(node)) {
      throw new NodeNotFoundError(node);
    }

    const visited = new Set<string>();
    const queue: string[] = Array.from(this._reverse.get(node)!.keys());
    for (const n of queue) {
      visited.add(n);
    }

    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const predecessor of this._reverse.get(current)!.keys()) {
        if (!visited.has(predecessor)) {
          visited.add(predecessor);
          queue.push(predecessor);
        }
      }
    }

    return visited;
  }

  // ------------------------------------------------------------------
  // Independent Groups (Parallel Execution Levels)
  // ------------------------------------------------------------------
  //
  // This is a modified version of Kahn's algorithm. Instead of pulling
  // nodes off the queue one at a time, we pull ALL zero-in-degree nodes
  // at once -- they form one "level" of independent tasks that can run
  // in parallel.
  //
  // For a linear chain A -> B -> C, we get [[A], [B], [C]] (fully serial).
  // For a diamond A -> B, A -> C, B -> D, C -> D, we get
  // [[A], [B, C], [D]] -- B and C can run in parallel.

  /**
   * Partition nodes into levels by topological depth.
   *
   * Each level contains nodes that have no dependencies on each other
   * and whose dependencies have all been satisfied by earlier levels.
   * Nodes within a level can be executed in parallel.
   *
   * Throws `CycleError` if the graph contains a cycle.
   *
   * Returns an empty list for an empty graph.
   */
  independentGroups(): string[][] {
    const inDegree = new Map<string, number>();
    for (const [node, preds] of this._reverse) {
      inDegree.set(node, preds.size);
    }

    // Collect the initial set of zero-in-degree nodes.
    let currentLevel: string[] = [];
    for (const [node, degree] of inDegree) {
      if (degree === 0) {
        currentLevel.push(node);
      }
    }
    currentLevel.sort();

    const groups: string[][] = [];
    let processed = 0;

    while (currentLevel.length > 0) {
      groups.push(currentLevel);
      processed += currentLevel.length;

      const nextLevelSet = new Set<string>();
      for (const node of currentLevel) {
        for (const successor of this._forward.get(node)!.keys()) {
          inDegree.set(successor, inDegree.get(successor)! - 1);
          if (inDegree.get(successor)! === 0) {
            nextLevelSet.add(successor);
          }
        }
      }

      currentLevel = Array.from(nextLevelSet).sort();
    }

    if (processed !== this._forward.size) {
      const cycle = this._findCycle();
      throw new CycleError(
        `Graph contains a cycle: ${cycle.join(" -> ")}`,
        cycle
      );
    }

    return groups;
  }

  // ------------------------------------------------------------------
  // Affected Nodes
  // ------------------------------------------------------------------
  //
  // Given a set of "changed" nodes, compute everything that is affected:
  // the changed nodes themselves plus all their transitive dependents.
  // This is useful in build systems to figure out what needs to be rebuilt.

  /**
   * Return the changed nodes plus all their transitive dependents.
   *
   * For each node in `changed`, we find everything that depends on it
   * (directly or transitively) and include it in the result. The changed
   * nodes themselves are always included.
   *
   * Nodes in `changed` that don't exist in the graph are silently
   * ignored (they might have been removed).
   */
  affectedNodes(changed: Set<string>): Set<string> {
    const result = new Set<string>();

    for (const node of changed) {
      if (this._forward.has(node)) {
        result.add(node);
        for (const dep of this.transitiveDependents(node)) {
          result.add(dep);
        }
      }
    }

    return result;
  }

  private assertNode(node: string): void {
    if (!this.hasNode(node)) {
      throw new NodeNotFoundError(node);
    }
  }

  private assertEdge(fromNode: string, toNode: string): void {
    if (!this.hasEdge(fromNode, toNode)) {
      throw new EdgeNotFoundError(fromNode, toNode);
    }
  }

  private mergeNodeProperties(
    node: string,
    properties: GraphPropertyBag
  ): void {
    const existing = this._nodeProperties.get(node);
    if (existing === undefined) {
      return;
    }
    Object.assign(existing, properties);
  }

  private mergeEdgeProperties(
    fromNode: string,
    toNode: string,
    properties: GraphPropertyBag
  ): void {
    const key = edgeKey(fromNode, toNode);
    const existing = this._edgeProperties.get(key) ?? {};
    Object.assign(existing, properties);
    this._edgeProperties.set(key, existing);
  }

  private validateWeight(weight: number): void {
    if (typeof weight !== "number" || Number.isNaN(weight)) {
      throw new Error("Edge weight must be a number");
    }
  }

  private setEdgeWeight(
    fromNode: string,
    toNode: string,
    weight: number
  ): void {
    this.validateWeight(weight);
    this._forward.get(fromNode)!.set(toNode, weight);
    this._reverse.get(toNode)!.set(fromNode, weight);
  }
}
