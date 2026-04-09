/**
 * graph.ts — Undirected Graph: An Undirected Network of Nodes and Edges
 * =====================================================================
 *
 * A graph G = (V, E) is a pair of sets:
 *
 *   V  — vertices (nodes): strings in this TypeScript implementation
 *   E  — edges: unordered pairs {u, v} — no direction, {u,v} == {v,u}
 *
 * Two Representations
 * -------------------
 * We support two internal representations, selectable at construction time:
 *
 *   ADJACENCY_LIST (default):
 *     A Map mapping each node to a Map of its neighbours with edge weights.
 *
 *         adj.get(u).get(v) = weight    (and adj.get(v).get(u) = weight for undirected)
 *
 *     Space: O(V + E)  — only stores existing edges.
 *     Edge lookup: O(degree(u))  — scan neighbour map.
 *     Best for SPARSE graphs (most real-world graphs).
 *
 *   ADJACENCY_MATRIX:
 *     A V×V boolean matrix where matrix[i][j] = true means an edge exists.
 *     Nodes are mapped to integer indices for row/column addressing.
 *     Weights stored in a separate Map.
 *
 *         matrix[idx(u)][idx(v)] = true, weights.get(u).get(v) = weight
 *
 *     Space: O(V²)  — allocates a slot for every possible edge.
 *     Edge lookup: O(1)  — single array read.
 *     Best for DENSE graphs or when O(1) edge lookup is critical.
 *
 * Both representations expose the same public API.  Every algorithm works
 * unchanged on either.
 *
 * Undirected Edge Symmetry
 * -------------------------
 * Because edges have no direction, every operation maintains symmetry:
 *
 *     addEdge(u, v, w)    stores   adj[u][v] = w  AND  adj[v][u] = w
 *     removeEdge(u, v)    removes  adj[u][v]       AND  adj[v][u]
 *     edges()             returns each edge ONCE as [min(u,v), max(u,v), w]
 *
 * This is the key invariant that makes Graph undirected.
 */

export enum GraphRepr {
  ADJACENCY_LIST = "adjacency_list",
  ADJACENCY_MATRIX = "adjacency_matrix",
}

export class Graph {
  private repr: GraphRepr;
  // Adjacency list representation
  private adj?: Map<string, Map<string, number>>;
  // Adjacency matrix representation
  private nodeList?: string[];
  private nodeIdx?: Map<string, number>;
  private matrix?: boolean[][];
  private weights?: Map<string, Map<string, number>>;

  /**
   * Create a new undirected graph.
   *
   * @param repr - Internal representation: ADJACENCY_LIST (default) or ADJACENCY_MATRIX
   */
  constructor(repr: GraphRepr = GraphRepr.ADJACENCY_LIST) {
    this.repr = repr;

    if (repr === GraphRepr.ADJACENCY_LIST) {
      this.adj = new Map();
    } else {
      this.nodeList = [];
      this.nodeIdx = new Map();
      this.matrix = [];
      this.weights = new Map();
    }
  }

  // ------------------------------------------------------------------
  // Node operations
  // ------------------------------------------------------------------

  /**
   * Add a node to the graph. No-op if the node already exists.
   */
  addNode(node: string): void {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      if (!this.adj!.has(node)) {
        this.adj!.set(node, new Map());
      }
    } else {
      if (!this.nodeIdx!.has(node)) {
        const idx = this.nodeList!.length;
        this.nodeList!.push(node);
        this.nodeIdx!.set(node, idx);
        this.weights!.set(node, new Map());

        // Add a new row and column of false values
        for (const row of this.matrix!) {
          row.push(false);
        }
        this.matrix!.push(new Array(idx + 1).fill(false));
      }
    }
  }

  /**
   * Remove a node and all edges incident to it.
   *
   * @throws Error if the node does not exist.
   */
  removeNode(node: string): void {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      if (!this.adj!.has(node)) {
        throw new Error(`Node not found: ${node}`);
      }
      // Remove all edges that touch this node
      for (const neighbour of this.adj!.get(node)!.keys()) {
        this.adj!.get(neighbour)!.delete(node);
      }
      this.adj!.delete(node);
    } else {
      if (!this.nodeIdx!.has(node)) {
        throw new Error(`Node not found: ${node}`);
      }
      const idx = this.nodeIdx!.get(node)!;
      this.nodeList!.splice(idx, 1);
      this.nodeIdx!.delete(node);

      // Update indices for nodes that shifted down
      for (let i = idx; i < this.nodeList!.length; i++) {
        this.nodeIdx!.set(this.nodeList![i], i);
      }

      // Remove the row
      this.matrix!.splice(idx, 1);

      // Remove the column from every remaining row
      for (const row of this.matrix!) {
        row.splice(idx, 1);
      }

      this.weights!.delete(node);
    }
  }

  /**
   * Return true if node is in the graph.
   */
  hasNode(node: string): boolean {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      return this.adj!.has(node);
    }
    return this.nodeIdx!.has(node);
  }

  /**
   * Return all nodes as a frozen array.
   */
  nodes(): string[] {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      return Array.from(this.adj!.keys());
    }
    return this.nodeList!.slice();
  }

  // ------------------------------------------------------------------
  // Edge operations
  // ------------------------------------------------------------------

  /**
   * Add an undirected edge between u and v with the given weight (default 1.0).
   *
   * Both nodes are added automatically if they do not already exist.
   * If the edge already exists its weight is updated.
   */
  addEdge(u: string, v: string, weight: number = 1.0): void {
    this.addNode(u);
    this.addNode(v);

    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      this.adj!.get(u)!.set(v, weight);
      this.adj!.get(v)!.set(u, weight);
    } else {
      const i = this.nodeIdx!.get(u)!;
      const j = this.nodeIdx!.get(v)!;
      this.matrix![i][j] = true;
      this.matrix![j][i] = true;
      this.weights!.get(u)!.set(v, weight);
      this.weights!.get(v)!.set(u, weight);
    }
  }

  /**
   * Remove the edge between u and v.
   *
   * @throws Error if either node or the edge does not exist.
   */
  removeEdge(u: string, v: string): void {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      if (!this.adj!.has(u) || !this.adj!.get(u)!.has(v)) {
        throw new Error(`Edge not found: (${u}, ${v})`);
      }
      this.adj!.get(u)!.delete(v);
      this.adj!.get(v)!.delete(u);
    } else {
      if (!this.nodeIdx!.has(u) || !this.nodeIdx!.has(v)) {
        throw new Error(`Node not found: (${u}, ${v})`);
      }
      const i = this.nodeIdx!.get(u)!;
      const j = this.nodeIdx!.get(v)!;
      if (!this.matrix![i][j]) {
        throw new Error(`Edge not found: (${u}, ${v})`);
      }
      this.matrix![i][j] = false;
      this.matrix![j][i] = false;
      this.weights!.get(u)!.delete(v);
      this.weights!.get(v)!.delete(u);
    }
  }

  /**
   * Return true if an edge exists between u and v.
   */
  hasEdge(u: string, v: string): boolean {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      return this.adj!.has(u) && this.adj!.get(u)!.has(v);
    }
    if (!this.nodeIdx!.has(u) || !this.nodeIdx!.has(v)) {
      return false;
    }
    const i = this.nodeIdx!.get(u)!;
    const j = this.nodeIdx!.get(v)!;
    return this.matrix![i][j];
  }

  /**
   * Return all edges as an array of [u, v, weight] triples.
   *
   * Each undirected edge appears exactly once. The two endpoint nodes
   * are ordered canonically (using string comparison).
   */
  edges(): Array<[string, string, number]> {
    const result: Array<[string, string, number]> = [];

    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      const seen = new Set<string>();
      for (const [u, neighbours] of this.adj!.entries()) {
        for (const [v, w] of neighbours.entries()) {
          // Canonical ordering: use string comparison
          const key = [u, v].sort().join(",");
          if (!seen.has(key)) {
            const [a, b] = u <= v ? [u, v] : [v, u];
            result.push([a, b, w]);
            seen.add(key);
          }
        }
      }
    } else {
      const n = this.nodeList!.length;
      for (let i = 0; i < n; i++) {
        for (let j = i + 1; j < n; j++) {
          if (this.matrix![i][j]) {
            const u = this.nodeList![i];
            const v = this.nodeList![j];
            const w = this.weights!.get(u)!.get(v)!;
            result.push([u, v, w]);
          }
        }
      }
    }

    return result;
  }

  /**
   * Return the weight of edge (u, v).
   *
   * @throws Error if the edge does not exist.
   */
  edgeWeight(u: string, v: string): number {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      if (!this.adj!.has(u) || !this.adj!.get(u)!.has(v)) {
        throw new Error(`Edge not found: (${u}, ${v})`);
      }
      return this.adj!.get(u)!.get(v)!;
    }
    if (!this.nodeIdx!.has(u) || !this.nodeIdx!.has(v)) {
      throw new Error(`Edge not found: (${u}, ${v})`);
    }
    const i = this.nodeIdx!.get(u)!;
    const j = this.nodeIdx!.get(v)!;
    if (!this.matrix![i][j]) {
      throw new Error(`Edge not found: (${u}, ${v})`);
    }
    return this.weights!.get(u)!.get(v)!;
  }

  // ------------------------------------------------------------------
  // Neighbourhood queries
  // ------------------------------------------------------------------

  /**
   * Return all neighbours of node.
   *
   * @throws Error if the node does not exist.
   */
  neighbors(node: string): string[] {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      if (!this.adj!.has(node)) {
        throw new Error(`Node not found: ${node}`);
      }
      return Array.from(this.adj!.get(node)!.keys());
    }
    if (!this.nodeIdx!.has(node)) {
      throw new Error(`Node not found: ${node}`);
    }
    const idx = this.nodeIdx!.get(node)!;
    const result: string[] = [];
    for (let j = 0; j < this.matrix![idx].length; j++) {
      if (this.matrix![idx][j]) {
        result.push(this.nodeList![j]);
      }
    }
    return result;
  }

  /**
   * Return {neighbour: weight} for all neighbours of node.
   *
   * @throws Error if the node does not exist.
   */
  neighborsWeighted(node: string): Map<string, number> {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      if (!this.adj!.has(node)) {
        throw new Error(`Node not found: ${node}`);
      }
      return new Map(this.adj!.get(node)!);
    }
    if (!this.nodeIdx!.has(node)) {
      throw new Error(`Node not found: ${node}`);
    }
    const idx = this.nodeIdx!.get(node)!;
    const result = new Map<string, number>();
    for (let j = 0; j < this.matrix![idx].length; j++) {
      if (this.matrix![idx][j]) {
        const neighbour = this.nodeList![j];
        const weight = this.weights!.get(node)!.get(neighbour)!;
        result.set(neighbour, weight);
      }
    }
    return result;
  }

  /**
   * Return the degree of node (number of incident edges).
   *
   * @throws Error if the node does not exist.
   */
  degree(node: string): number {
    return this.neighbors(node).length;
  }

  /**
   * Return the number of nodes in the graph.
   */
  get length(): number {
    if (this.repr === GraphRepr.ADJACENCY_LIST) {
      return this.adj!.size;
    }
    return this.nodeList!.length;
  }

  /**
   * String representation of the graph.
   */
  toString(): string {
    return `Graph(nodes=${this.length}, edges=${this.edges().length}, repr=${this.repr})`;
  }
}
