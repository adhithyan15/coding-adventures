// index.d.ts -- TypeScript type definitions for the native addon
// ===============================================================
//
// These type definitions describe the JavaScript API exposed by the Rust
// native addon. They provide IntelliSense, type checking, and documentation
// for TypeScript users.
//
// The actual implementation is in src/lib.rs (Rust). These types must be
// kept in sync with the Rust #[napi] annotations manually.
//
// Error types:
// - CycleError: thrown when topological sort or independent groups encounters a cycle.
//   The error message starts with "CycleError:".
// - NodeNotFoundError: thrown when an operation references a nonexistent node.
//   The error message starts with "NodeNotFoundError:".
// - EdgeNotFoundError: thrown when removeEdge references a nonexistent edge.
//   The error message starts with "EdgeNotFoundError:".
// - SelfLoopError: thrown when addEdge is called with fromNode === toNode.
//   The error message starts with "SelfLoopError:".

/**
 * A directed graph backed by Rust's directed-graph crate.
 *
 * This class provides the same API as the pure TypeScript `Graph` class from
 * `@coding-adventures/directed-graph`, but the algorithms run in Rust for
 * better performance on large graphs.
 *
 * @example
 * ```typescript
 * const g = new DirectedGraph();
 * g.addEdge("compile", "link");
 * g.addEdge("link", "package");
 *
 * g.topologicalSort();    // ["compile", "link", "package"]
 * g.independentGroups();  // [["compile"], ["link"], ["package"]]
 * ```
 */
export class DirectedGraph {
  /** Create a new empty directed graph. */
  constructor();

  // -- Node operations ---------------------------------------------------

  /** Add a node to the graph. If the node already exists, this is a no-op. */
  addNode(node: string): void;

  /**
   * Remove a node and all its edges from the graph.
   * @throws {Error} NodeNotFoundError if the node does not exist.
   */
  removeNode(node: string): void;

  /** Check whether a node exists in the graph. */
  hasNode(node: string): boolean;

  /** Return a sorted list of all nodes in the graph. */
  nodes(): string[];

  // -- Edge operations ---------------------------------------------------

  /**
   * Add a directed edge from `fromNode` to `toNode`.
   * Both nodes are created if they don't exist yet.
   * @throws {Error} SelfLoopError if fromNode === toNode.
   */
  addEdge(fromNode: string, toNode: string): void;

  /**
   * Remove a directed edge from `fromNode` to `toNode`.
   * @throws {Error} EdgeNotFoundError if the edge does not exist.
   */
  removeEdge(fromNode: string, toNode: string): void;

  /** Check whether a directed edge exists from `fromNode` to `toNode`. */
  hasEdge(fromNode: string, toNode: string): boolean;

  /**
   * Return a list of all edges as `[from, to]` pairs, sorted.
   * Each edge is a two-element array: `[fromNode, toNode]`.
   */
  edges(): [string, string][];

  // -- Neighbor queries --------------------------------------------------

  /**
   * Return the predecessors (parents) of a node -- nodes that point TO it.
   * @throws {Error} NodeNotFoundError if the node does not exist.
   */
  predecessors(node: string): string[];

  /**
   * Return the successors (children) of a node -- nodes it points TO.
   * @throws {Error} NodeNotFoundError if the node does not exist.
   */
  successors(node: string): string[];

  // -- Graph properties --------------------------------------------------

  /** Return the number of nodes in the graph. */
  size(): number;

  /** Return the number of edges in the graph. */
  edgeCount(): number;

  /**
   * Return a human-readable string representation of the graph.
   * Format: `DirectedGraph(nodes=N, edges=M)`
   */
  toStringRepr(): string;

  // -- Algorithms --------------------------------------------------------

  /**
   * Return a topological ordering of the graph (Kahn's algorithm).
   * Every node appears after all of its dependencies.
   * @throws {Error} CycleError if the graph contains a cycle.
   */
  topologicalSort(): string[];

  /** Check whether the graph contains a cycle (DFS with 3-color marking). */
  hasCycle(): boolean;

  /**
   * Return all nodes reachable from `node` (transitive closure).
   * Does NOT include the node itself. Returns a sorted array.
   * @throws {Error} NodeNotFoundError if the node does not exist.
   */
  transitiveClosure(node: string): string[];

  /**
   * Given a list of changed nodes, return all nodes that are transitively
   * affected (the changed nodes plus everything that depends on them).
   * Unknown nodes are silently ignored.
   */
  affectedNodes(changed: string[]): string[];

  /**
   * Partition the graph into independent groups (parallel execution levels).
   * Level 0 contains nodes with no dependencies. Level 1 depends only on
   * level 0. And so on. Nodes within the same level can be executed in parallel.
   * @throws {Error} CycleError if the graph contains a cycle.
   */
  independentGroups(): string[][];
}
