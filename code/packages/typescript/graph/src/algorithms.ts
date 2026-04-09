/**
 * algorithms.ts — Pure Graph Algorithms
 * =====================================
 *
 * All functions here are pure — they take a Graph as input and return a result.
 * They never mutate the graph.  They work identically on both ADJACENCY_LIST and
 * ADJACENCY_MATRIX graphs because they only call the Graph's public API.
 *
 * Algorithms provided:
 *   bfs                   — breadth-first traversal
 *   dfs                   — depth-first traversal
 *   isConnected           — does every node reach every other?
 *   connectedComponents   — find all isolated clusters
 *   hasCycle              — does the graph contain a cycle?
 *   shortestPath          — fewest-hops or lowest-weight path
 *   minimumSpanningTree   — cheapest set of edges connecting all nodes
 */

import { Graph } from "./graph";

/**
 * BFS — Breadth-First Search
 *
 * BFS explores a graph level-by-level: first all nodes 1 hop from start,
 * then all 2 hops, then 3 hops, etc.  Picture a stone dropped in water:
 * the ripple rings expand outward one at a time.
 *
 *   Queue  (FIFO): nodes to visit, oldest first.
 *   Visited set:   prevents revisiting nodes and infinite loops.
 */
export function bfs(graph: Graph, start: string): string[] {
  /**
   * Return nodes reachable from start in breadth-first order.
   *
   * Nodes not reachable from start (in a disconnected graph) are excluded.
   *
   * Time: O(V + E).  Space: O(V) for the visited set and queue.
   */
  const visited = new Set<string>([start]);
  const queue: string[] = [start];
  const result: string[] = [];

  while (queue.length > 0) {
    const node = queue.shift()!;
    result.push(node);
    // Sort neighbours for deterministic output
    const neighbours = graph.neighbors(node).sort();
    for (const neighbour of neighbours) {
      if (!visited.has(neighbour)) {
        visited.add(neighbour);
        queue.push(neighbour);
      }
    }
  }

  return result;
}

/**
 * DFS — Depth-First Search
 *
 * DFS explores as far as possible down each branch before backtracking.
 * Think of solving a maze: go straight until you hit a dead end, back up,
 * try the next turn.
 *
 * We use an explicit stack instead of recursion to avoid stack overflow
 * on deep graphs.
 */
export function dfs(graph: Graph, start: string): string[] {
  /**
   * Return nodes reachable from start in depth-first order.
   *
   * Nodes not reachable from start (in a disconnected graph) are excluded.
   *
   * Time: O(V + E).  Space: O(V) for the visited set and stack.
   */
  const visited = new Set<string>();
  const stack: string[] = [start];
  const result: string[] = [];

  while (stack.length > 0) {
    const node = stack.pop()!;
    if (visited.has(node)) {
      continue;
    }
    visited.add(node);
    result.push(node);

    // Reverse-sort so that when we push all neighbours the first (alphabetically)
    // is on top — this makes output deterministic
    const neighbours = graph.neighbors(node).sort().reverse();
    for (const neighbour of neighbours) {
      if (!visited.has(neighbour)) {
        stack.push(neighbour);
      }
    }
  }

  return result;
}

/**
 * Is Connected
 *
 * A graph is connected if every node can reach every other node.
 * One BFS from any starting node visits ALL nodes iff the graph is connected.
 */
export function isConnected(graph: Graph): boolean {
  /**
   * Return true if every node can reach every other node.
   *
   * An empty graph is vacuously connected (True).
   * A single-node graph is trivially connected (True).
   *
   * Time: O(V + E).
   */
  const nodes = graph.nodes();
  if (nodes.length === 0) {
    return true;
  }
  return bfs(graph, nodes[0]).length === nodes.length;
}

/**
 * Connected Components
 *
 * When a graph is disconnected it consists of several isolated clusters called
 * "connected components".  Think of an archipelago: ships can travel between
 * ports on the same island, but cannot cross to a different island.
 *
 * Algorithm: repeatedly BFS from any unvisited node, collecting each component.
 */
export function connectedComponents(graph: Graph): Set<string>[] {
  /**
   * Return a list of connected components, each as a Set of nodes.
   *
   * Time: O(V + E).
   */
  const unvisited = new Set(graph.nodes());
  const components: Set<string>[] = [];

  while (unvisited.size > 0) {
    const start = unvisited.values().next().value;
    const component = new Set(bfs(graph, start));
    components.push(component);
    for (const node of component) {
      unvisited.delete(node);
    }
  }

  return components;
}

/**
 * Has Cycle
 *
 * An undirected graph has a cycle if DFS finds a "back edge" — an edge to a
 * node already in the visited set that is NOT the node we came from.
 *
 * The "not our parent" check is essential: in an undirected graph every edge
 * appears twice (u→v and v→u).  Without the parent check, the return edge
 * would always look like a back edge and every edge would falsely indicate a
 * cycle.
 */
export function hasCycle(graph: Graph): boolean {
  /**
   * Return true if the graph contains any cycle.
   *
   * Uses iterative DFS (avoids stack overflow on large graphs).
   *
   * Key insight: an undirected graph has a cycle iff DFS finds a "back edge" —
   * an edge to an already-visited node that is NOT the node we came from.
   * The parent check prevents counting the return edge (u→v, v→u) as a cycle.
   *
   * Time: O(V + E).
   */
  const visited = new Set<string>();

  for (const start of graph.nodes()) {
    if (visited.has(start)) {
      continue;
    }

    // Stack holds [node, parent] pairs
    const stack: Array<[string, string | null]> = [[start, null]];

    while (stack.length > 0) {
      const [node, par] = stack.pop()!;

      if (visited.has(node)) {
        continue;
      }

      visited.add(node);

      for (const neighbour of graph.neighbors(node)) {
        if (!visited.has(neighbour)) {
          stack.push([neighbour, node]);
        } else if (neighbour !== par) {
          // Back edge: visited neighbour that isn't our parent → cycle
          return true;
        }
      }
    }
  }

  return false;
}

/**
 * Shortest Path
 *
 * Two strategies depending on edge weights:
 *
 *   All weights equal (or all 1.0):
 *     BFS finds the shortest path in O(V + E).  BFS naturally explores
 *     nodes in order of hop-count, so the first time it reaches the
 *     destination it has found the shortest route.
 *
 *   Variable weights (Dijkstra's algorithm):
 *     A priority queue (min-heap) always expands the cheapest unvisited
 *     node.  Think of water flowing: it always takes the cheapest path
 *     available, spreading outward until it reaches the destination.
 */
export function shortestPath(graph: Graph, start: string, end: string): string[] {
  /**
   * Return the shortest (lowest-weight) path from start to end.
   *
   * Returns an empty array if no path exists.
   *
   * For unweighted graphs (all weights 1.0) uses BFS — O(V + E).
   * For weighted graphs uses Dijkstra's algorithm — O((V + E) log V).
   */
  if (start === end) {
    return graph.nodes().includes(start) ? [start] : [];
  }

  // Decide strategy: BFS if all weights are 1.0, else Dijkstra
  const allUnit = graph.edges().every(([_, __, w]) => w === 1.0);

  if (allUnit) {
    return bfsPath(graph, start, end);
  }
  return dijkstra(graph, start, end);
}

function bfsPath(graph: Graph, start: string, end: string): string[] {
  /**
   * BFS shortest path (for unweighted graphs).
   */
  const parent = new Map<string, string | null>([[start, null]]);
  const queue: string[] = [start];

  while (queue.length > 0) {
    const node = queue.shift()!;
    if (node === end) {
      break;
    }
    for (const neighbour of graph.neighbors(node)) {
      if (!parent.has(neighbour)) {
        parent.set(neighbour, node);
        queue.push(neighbour);
      }
    }
  }

  if (!parent.has(end)) {
    return [];
  }

  // Trace back from end to start via parent pointers
  const path: string[] = [];
  let cur: string | null = end;
  while (cur !== null) {
    path.push(cur);
    cur = parent.get(cur) ?? null;
  }
  path.reverse();
  return path;
}

function dijkstra(graph: Graph, start: string, end: string): string[] {
  /**
   * Dijkstra's algorithm for weighted shortest path.
   */
  const INF = Number.MAX_VALUE;
  const dist = new Map<string, number>();
  const parent = new Map<string, string | null>();

  for (const node of graph.nodes()) {
    dist.set(node, INF);
  }
  dist.set(start, 0);

  // Min-heap entries: [distance, node]
  // Using a simple array and sorting for simplicity (not optimal, but correct)
  const heap: Array<[number, string]> = [[0, start]];

  while (heap.length > 0) {
    // Sort to get min element
    heap.sort((a, b) => a[0] - b[0]);
    const [d, node] = heap.shift()!;

    if (d > dist.get(node)!) {
      continue; // Stale entry
    }

    if (node === end) {
      break;
    }

    for (const [neighbour, weight] of graph.neighborsWeighted(node).entries()) {
      const newDist = dist.get(node)! + weight;
      if (newDist < dist.get(neighbour)!) {
        dist.set(neighbour, newDist);
        parent.set(neighbour, node);
        heap.push([newDist, neighbour]);
      }
    }
  }

  if (dist.get(end)! === INF) {
    return [];
  }

  // Trace back
  const path: string[] = [];
  let cur: string | null = end;
  while (cur !== null) {
    path.push(cur);
    cur = parent.get(cur) ?? null;
  }
  path.reverse();
  return path;
}

/**
 * Minimum Spanning Tree — Kruskal's algorithm + Union-Find
 *
 * A spanning tree connects all V nodes with exactly V-1 edges and no cycles.
 * The MINIMUM spanning tree does so with the lowest possible total weight.
 *
 * Real-world use: lay cables to connect N cities using the least total wire.
 *
 * Kruskal's algorithm:
 *   1. Sort all edges by weight (cheapest first).
 *   2. Greedily add each edge IF it doesn't create a cycle.
 *      Cycle check: use Union-Find.  If both endpoints are already in the
 *      same component, adding the edge would create a cycle — skip it.
 *   3. Stop when we have V-1 edges (the spanning tree is complete).
 */
export function minimumSpanningTree(
  graph: Graph
): Array<[string, string, number]> {
  /**
   * Return the minimum spanning tree as an array of [u, v, weight] triples.
   *
   * Throws an error if the graph is not connected (no spanning tree exists).
   *
   * Time: O(E log E) for sorting + O(E · α(V)) for Union-Find.
   */
  const allNodes = graph.nodes();
  if (allNodes.length === 0) {
    return [];
  }

  // Sort edges by weight
  const sortedEdges = graph.edges().sort((a, b) => a[2] - b[2]);

  const uf = new UnionFind(allNodes);
  const mst: Array<[string, string, number]> = [];

  for (const [u, v, w] of sortedEdges) {
    if (uf.find(u) !== uf.find(v)) {
      uf.union(u, v);
      mst.push([u, v, w]);
      if (mst.length === allNodes.length - 1) {
        break; // MST is complete
      }
    }
  }

  if (mst.length < allNodes.length - 1 && allNodes.length > 1) {
    throw new Error(
      "Graph is not connected — no spanning tree exists"
    );
  }

  return mst;
}

/**
 * Union-Find (helper for Kruskal's algorithm)
 *
 * Tracks which "component" (group) each node belongs to.
 * Two nodes are in the same component iff find(a) === find(b).
 *
 * Path compression:  When we walk up the parent chain to find the root,
 * we update every visited node to point DIRECTLY to the root.  This
 * "flattens" the tree and makes future finds very fast.
 */
class UnionFind {
  private parent: Map<string, string>;
  private rank: Map<string, number>;

  constructor(nodes: string[]) {
    this.parent = new Map();
    this.rank = new Map();
    for (const node of nodes) {
      this.parent.set(node, node);
      this.rank.set(node, 0);
    }
  }

  find(x: string): string {
    /**
     * Return the representative (root) of x's component.
     */
    if (this.parent.get(x) !== x) {
      this.parent.set(x, this.find(this.parent.get(x)!)); // path compression
    }
    return this.parent.get(x)!;
  }

  union(a: string, b: string): void {
    /**
     * Merge the components of a and b (union by rank).
     */
    let ra = this.find(a);
    let rb = this.find(b);
    if (ra === rb) {
      return;
    }
    // Attach the shorter tree under the taller tree
    if (this.rank.get(ra)! < this.rank.get(rb)!) {
      [ra, rb] = [rb, ra];
    }
    this.parent.set(rb, ra);
    if (this.rank.get(ra) === this.rank.get(rb)) {
      this.rank.set(ra, this.rank.get(ra)! + 1);
    }
  }
}
