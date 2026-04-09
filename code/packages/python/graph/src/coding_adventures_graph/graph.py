"""
Undirected Graph Implementation from Scratch
==============================================

A graph is the most general data structure for representing pairwise relationships
between things. This module provides both adjacency list and adjacency matrix
representations with a comprehensive set of algorithms.

## Core Concepts

Nodes (vertices): the entities in the graph
Edges: connections between pairs of nodes
Weight: optional cost/distance on each edge
Degree: number of edges connected to a node
Path: sequence of nodes connected by edges
Cycle: a path that starts and ends at the same node
Connected: every node can reach every other node via some path

## Two Representations

Adjacency List (default):
  - Space: O(V + E), efficient for sparse graphs
  - Edge lookup: O(degree(v))
  - Best for: most real-world graphs (sparse)

Adjacency Matrix:
  - Space: O(V²), efficient for dense graphs
  - Edge lookup: O(1)
  - Best for: dense graphs, complete graphs, game boards

The rule of thumb: use adjacency list by default. Switch to matrix only if
the graph is very dense (E > V²/4) AND you need O(1) edge lookup.

## Nodes

Nodes can be any hashable type: strings, integers, tuples. They are stored
as-is with no internal ID mapping. This keeps the API simple and memory usage
proportional to the actual data.

## Weighted Edges

By default, every edge has weight 1.0. The weight represents cost, distance,
strength of connection, or any numeric attribute of the relationship.

Algorithms that care about weights (Dijkstra's shortest path, MST) use them.
Algorithms that don't (BFS, DFS, cycle detection) treat all edges as equal.
"""

from __future__ import annotations

from collections import deque
from enum import Enum
from typing import Generic, Optional, TypeVar

T = TypeVar("T")


class GraphRepr(Enum):
    """Graph representation strategy."""
    ADJACENCY_LIST = "adjacency_list"
    ADJACENCY_MATRIX = "adjacency_matrix"


class NodeNotFoundError(Exception):
    """Raised when trying to operate on a node that doesn't exist."""
    pass


class EdgeNotFoundError(Exception):
    """Raised when trying to operate on an edge that doesn't exist."""
    pass


class Graph(Generic[T]):
    """
    An undirected, optionally weighted graph.

    Supports two internal representations (adjacency list or matrix)
    with identical public APIs. All algorithms work on either representation
    without modification.

    Example:
        g = Graph()
        g.add_edge("A", "B", weight=5)
        g.add_edge("B", "C")
        print(g.neighbors("B"))  # frozenset({"A", "C"})
        print(bfs(g, "A"))       # ["A", "B", "C"]
    """

    def __init__(self, repr: GraphRepr = GraphRepr.ADJACENCY_LIST) -> None:
        """
        Initialize a new graph with the given representation.

        Args:
            repr: Either ADJACENCY_LIST (default, sparse) or ADJACENCY_MATRIX (dense).
        """
        self._repr = repr

        if repr == GraphRepr.ADJACENCY_LIST:
            # For adjacency list: map from node to map of neighbors to weights
            # Structure: {node: {neighbor: weight, ...}, ...}
            self._nodes_adj_list: set[T] = set()
            self._adj: dict[T, dict[T, float]] = {}
        else:
            # For adjacency matrix: maintain ordered node list and dense matrix
            # Structure:
            #   _nodes_matrix: ordered list of nodes for index mapping
            #   _index: reverse mapping from node to its row/col index
            #   _matrix: V×V matrix where 0.0 means no edge, positive means weight
            self._nodes_matrix: list[T] = []
            self._index: dict[T, int] = {}
            self._matrix: list[list[float]] = []

    # ─── Node Operations ────────────────────────────────────────────────────

    def add_node(self, node: T) -> None:
        """
        Add a node to the graph. If the node already exists, this is a no-op.

        Time complexity:
            - Adjacency list: O(1)
            - Adjacency matrix: O(V²) due to matrix resizing
        """
        if self._repr == GraphRepr.ADJACENCY_LIST:
            if node not in self._nodes_adj_list:
                self._nodes_adj_list.add(node)
                self._adj[node] = {}
        else:
            if node not in self._index:
                idx = len(self._nodes_matrix)
                self._nodes_matrix.append(node)
                self._index[node] = idx
                # Expand matrix: add new row and new column
                for row in self._matrix:
                    row.append(0.0)
                self._matrix.append([0.0] * (idx + 1))

    def remove_node(self, node: T) -> None:
        """
        Remove a node and all its incident edges from the graph.

        Raises:
            NodeNotFoundError: if the node doesn't exist.

        Time complexity:
            - Adjacency list: O(degree(node) + V) to clean up reverse edges
            - Adjacency matrix: O(V²) to rebuild matrix
        """
        if not self.has_node(node):
            raise NodeNotFoundError(f"Node {node} not found in graph")

        if self._repr == GraphRepr.ADJACENCY_LIST:
            # Remove all edges involving this node
            neighbors = list(self._adj[node].keys())
            for neighbor in neighbors:
                if neighbor in self._adj:
                    self._adj[neighbor].pop(node, None)
            # Remove the node itself
            del self._adj[node]
            self._nodes_adj_list.discard(node)
        else:
            # Matrix representation: remove row and column
            idx = self._index[node]
            # Remove from index mapping
            del self._index[node]
            self._nodes_matrix.pop(idx)
            # Reindex all nodes after this one
            for i in range(idx, len(self._nodes_matrix)):
                self._index[self._nodes_matrix[i]] = i
            # Remove row and column from matrix
            self._matrix.pop(idx)
            for row in self._matrix:
                row.pop(idx)

    def has_node(self, node: T) -> bool:
        """Check if a node exists in the graph. O(1)."""
        if self._repr == GraphRepr.ADJACENCY_LIST:
            return node in self._nodes_adj_list
        else:
            return node in self._index

    def nodes(self) -> frozenset[T]:
        """Return all nodes in the graph. O(V)."""
        if self._repr == GraphRepr.ADJACENCY_LIST:
            return frozenset(self._nodes_adj_list)
        else:
            return frozenset(self._nodes_matrix)

    # ─── Edge Operations ────────────────────────────────────────────────────

    def add_edge(self, u: T, v: T, weight: float = 1.0) -> None:
        """
        Add an undirected edge between u and v with the given weight.

        If either node doesn't exist, it is created. If the edge already exists,
        its weight is updated.

        Args:
            u: First endpoint (will be created if doesn't exist)
            v: Second endpoint (will be created if doesn't exist)
            weight: Edge weight (default 1.0)

        Time complexity:
            - Adjacency list: O(1) amortized
            - Adjacency matrix: O(1) if both nodes exist, O(V²) if growth needed
        """
        # Ensure both nodes exist
        if not self.has_node(u):
            self.add_node(u)
        if not self.has_node(v):
            self.add_node(v)

        if self._repr == GraphRepr.ADJACENCY_LIST:
            # Add both directions (undirected)
            self._adj[u][v] = weight
            self._adj[v][u] = weight
        else:
            # Matrix representation
            u_idx = self._index[u]
            v_idx = self._index[v]
            self._matrix[u_idx][v_idx] = weight
            self._matrix[v_idx][u_idx] = weight

    def remove_edge(self, u: T, v: T) -> None:
        """
        Remove the edge between u and v.

        Raises:
            EdgeNotFoundError: if the edge doesn't exist.
            NodeNotFoundError: if either node doesn't exist.

        Time complexity:
            - Adjacency list: O(1)
            - Adjacency matrix: O(1)
        """
        if not self.has_node(u):
            raise NodeNotFoundError(f"Node {u} not found")
        if not self.has_node(v):
            raise NodeNotFoundError(f"Node {v} not found")
        if not self.has_edge(u, v):
            raise EdgeNotFoundError(f"Edge ({u}, {v}) not found")

        if self._repr == GraphRepr.ADJACENCY_LIST:
            del self._adj[u][v]
            del self._adj[v][u]
        else:
            u_idx = self._index[u]
            v_idx = self._index[v]
            self._matrix[u_idx][v_idx] = 0.0
            self._matrix[v_idx][u_idx] = 0.0

    def has_edge(self, u: T, v: T) -> bool:
        """
        Check if an edge exists between u and v.

        Time complexity:
            - Adjacency list: O(degree(u))
            - Adjacency matrix: O(1)
        """
        if not self.has_node(u) or not self.has_node(v):
            return False

        if self._repr == GraphRepr.ADJACENCY_LIST:
            return v in self._adj[u]
        else:
            u_idx = self._index[u]
            v_idx = self._index[v]
            return self._matrix[u_idx][v_idx] > 0.0

    def edge_weight(self, u: T, v: T) -> float:
        """
        Get the weight of the edge between u and v.

        Returns:
            The edge weight (positive float).

        Raises:
            NodeNotFoundError: if either node doesn't exist.
            EdgeNotFoundError: if the edge doesn't exist.

        Time complexity:
            - Adjacency list: O(degree(u))
            - Adjacency matrix: O(1)
        """
        if not self.has_node(u):
            raise NodeNotFoundError(f"Node {u} not found")
        if not self.has_node(v):
            raise NodeNotFoundError(f"Node {v} not found")
        if not self.has_edge(u, v):
            raise EdgeNotFoundError(f"Edge ({u}, {v}) not found")

        if self._repr == GraphRepr.ADJACENCY_LIST:
            return self._adj[u][v]
        else:
            u_idx = self._index[u]
            v_idx = self._index[v]
            return self._matrix[u_idx][v_idx]

    def edges(self) -> frozenset[tuple[T, T, float]]:
        """
        Return all edges in the graph as (u, v, weight) tuples.

        In an undirected graph, each edge is represented exactly once
        (we normalize to u < v to avoid duplication).

        Time complexity: O(V + E)
        """
        result = set()

        if self._repr == GraphRepr.ADJACENCY_LIST:
            for u in self._adj:
                for v, weight in self._adj[u].items():
                    # Normalize: only include each edge once
                    if (u, v, weight) not in result and (v, u, weight) not in result:
                        result.add((u, v, weight))
        else:
            # Matrix representation
            for i, u in enumerate(self._nodes_matrix):
                for j in range(i + 1, len(self._nodes_matrix)):
                    v = self._nodes_matrix[j]
                    weight = self._matrix[i][j]
                    if weight > 0.0:
                        result.add((u, v, weight))

        return frozenset(result)

    # ─── Neighborhood Queries ───────────────────────────────────────────────

    def neighbors(self, node: T) -> frozenset[T]:
        """
        Get all neighbors (adjacent nodes) of the given node.

        Raises:
            NodeNotFoundError: if the node doesn't exist.

        Time complexity:
            - Adjacency list: O(degree(node))
            - Adjacency matrix: O(V)
        """
        if not self.has_node(node):
            raise NodeNotFoundError(f"Node {node} not found")

        if self._repr == GraphRepr.ADJACENCY_LIST:
            return frozenset(self._adj[node].keys())
        else:
            idx = self._index[node]
            neighbors_set = set()
            for j, weight in enumerate(self._matrix[idx]):
                if weight > 0.0 and j != idx:
                    neighbors_set.add(self._nodes_matrix[j])
            return frozenset(neighbors_set)

    def neighbors_weighted(self, node: T) -> dict[T, float]:
        """
        Get all neighbors of the node with their edge weights.

        Returns:
            A dict mapping neighbor -> weight

        Raises:
            NodeNotFoundError: if the node doesn't exist.

        Time complexity:
            - Adjacency list: O(degree(node))
            - Adjacency matrix: O(V)
        """
        if not self.has_node(node):
            raise NodeNotFoundError(f"Node {node} not found")

        if self._repr == GraphRepr.ADJACENCY_LIST:
            return dict(self._adj[node])
        else:
            idx = self._index[node]
            result = {}
            for j, weight in enumerate(self._matrix[idx]):
                if weight > 0.0 and j != idx:
                    result[self._nodes_matrix[j]] = weight
            return result

    def degree(self, node: T) -> int:
        """
        Get the degree (number of neighbors) of a node.

        Raises:
            NodeNotFoundError: if the node doesn't exist.

        Time complexity: O(1) for adjacency list, O(V) for matrix.
        """
        if not self.has_node(node):
            raise NodeNotFoundError(f"Node {node} not found")

        if self._repr == GraphRepr.ADJACENCY_LIST:
            return len(self._adj[node])
        else:
            idx = self._index[node]
            return sum(1 for w in self._matrix[idx] if w > 0.0)

    def __len__(self) -> int:
        """Return the number of nodes in the graph. O(1)."""
        if self._repr == GraphRepr.ADJACENCY_LIST:
            return len(self._nodes_adj_list)
        else:
            return len(self._nodes_matrix)

    def __contains__(self, node: T) -> bool:
        """Check if a node exists in the graph. O(1)."""
        return self.has_node(node)

    def __repr__(self) -> str:
        """Return a string representation of the graph."""
        return (
            f"Graph(nodes={len(self)}, edges={len(self.edges())}, "
            f"repr={self._repr.value})"
        )


# ─── Pure Function Algorithms ───────────────────────────────────────────────


def bfs(graph: Graph[T], start: T) -> list[T]:
    """
    Breadth-First Search: explore nodes level by level from start.

    Think of dropping a stone in a pond — ripples spread outward one ring at a time.
    BFS finds the shortest path (fewest edges) in unweighted graphs and is useful
    for exploring "nearby" nodes before distant ones.

    Algorithm:
    1. Initialize visited set with start node
    2. Enqueue start node
    3. While queue is not empty:
       a. Dequeue node and add to result
       b. For each unvisited neighbor, mark visited and enqueue

    Example:
        Graph: A—B—C, A—D
        BFS from A: [A, B, D, C]

    Args:
        graph: The graph to search
        start: Starting node

    Returns:
        List of nodes in BFS order

    Raises:
        NodeNotFoundError: if start node doesn't exist

    Time: O(V + E)
    Space: O(V) for visited set and queue
    """
    if not graph.has_node(start):
        raise NodeNotFoundError(f"Start node {start} not found")

    visited = {start}
    queue = deque([start])
    result = []

    while queue:
        node = queue.popleft()
        result.append(node)

        for neighbor in graph.neighbors(node):
            if neighbor not in visited:
                visited.add(neighbor)
                queue.append(neighbor)

    return result


def dfs(graph: Graph[T], start: T) -> list[T]:
    """
    Depth-First Search: go as deep as possible down one path before backtracking.

    Think of exploring a maze — walk forward until you hit a dead end, then back up
    and try a different turn. DFS is useful for cycle detection, topological sorting,
    finding connected components, and maze solving.

    Algorithm (iterative with explicit stack):
    1. Initialize empty visited set
    2. Push start node onto stack
    3. While stack is not empty:
       a. Pop node; if already visited, skip
       b. Mark visited and add to result
       c. For each unvisited neighbor, push onto stack

    Example:
        Graph: A—B—C, B—D
        DFS from A: [A, B, D, C] or [A, B, C, D] (depends on neighbor order)

    Args:
        graph: The graph to search
        start: Starting node

    Returns:
        List of nodes in DFS order

    Raises:
        NodeNotFoundError: if start node doesn't exist

    Time: O(V + E)
    Space: O(V) for visited set and stack
    """
    if not graph.has_node(start):
        raise NodeNotFoundError(f"Start node {start} not found")

    visited = set()
    stack = [start]
    result = []

    while stack:
        node = stack.pop()
        if node in visited:
            continue
        visited.add(node)
        result.append(node)

        # Push unvisited neighbors onto stack
        for neighbor in sorted(graph.neighbors(node), key=str):
            if neighbor not in visited:
                stack.append(neighbor)

    return result


def is_connected(graph: Graph[T]) -> bool:
    """
    Check if the graph is connected (every node can reach every other node).

    Algorithm:
    1. If graph is empty, it's vacuously connected
    2. Run BFS/DFS from any node
    3. Return True if all nodes were visited

    Example:
        Graph: A—B—C, D—E  (two components)
        is_connected(graph) = False

        Graph: A—B—C—D  (one component)
        is_connected(graph) = True

    Time: O(V + E)
    Space: O(V)
    """
    if len(graph) == 0:
        return True

    # Start from any node
    start = next(iter(graph.nodes()))
    visited = set(bfs(graph, start))
    return len(visited) == len(graph)


def connected_components(graph: Graph[T]) -> list[frozenset[T]]:
    """
    Find all connected components in the graph.

    A connected component is a maximal set of nodes where every node can reach
    every other node. If the graph has isolated clusters (islands), each cluster
    is a component.

    Algorithm:
    1. Start with all nodes unvisited
    2. While unvisited nodes remain:
       a. Pick any unvisited node as start
       b. Run BFS to find all reachable nodes
       c. Add reachable nodes as a component
       d. Remove from unvisited

    Example:
        Graph: A—B—C, D—E, F
        Components: [{A, B, C}, {D, E}, {F}]

    Time: O(V + E)
    Space: O(V) for visited set and components
    """
    unvisited = set(graph.nodes())
    components = []

    while unvisited:
        start = next(iter(unvisited))
        component = set(bfs(graph, start))
        components.append(frozenset(component))
        unvisited -= component

    return components


def has_cycle(graph: Graph[T]) -> bool:
    """
    Check if the graph has a cycle.

    An undirected graph has a cycle if DFS finds a "back edge" — an edge to a
    node already in the visited set, excluding the parent node (the node we
    came from). We must exclude the parent because every edge appears twice
    in an undirected graph.

    Algorithm (recursive DFS):
    1. Maintain visited set
    2. For each unvisited node, start a DFS with parent=None
    3. In DFS: mark node visited, then for each neighbor:
       - If not visited, recurse with neighbor as node and current as parent
       - If visited and not parent, we found a back edge → cycle!

    Example:
        Graph: A—B—C—A     (triangle)
        has_cycle(graph) = True

        Graph: A—B—C       (path)
        has_cycle(graph) = False

    Time: O(V + E)
    Space: O(V) for recursion stack and visited set
    """
    visited = set()

    def dfs_cycle(node: T, parent: Optional[T]) -> bool:
        """DFS helper that detects cycles."""
        visited.add(node)
        for neighbor in graph.neighbors(node):
            if neighbor not in visited:
                if dfs_cycle(neighbor, node):
                    return True
            elif neighbor != parent:
                # Back edge: neighbor visited and not our parent
                return True
        return False

    # Check all nodes (in case graph has multiple components)
    for node in graph.nodes():
        if node not in visited:
            if dfs_cycle(node, None):
                return True

    return False


def shortest_path(graph: Graph[T], start: T, end: T) -> list[T]:
    """
    Find the shortest path between start and end nodes.

    For unweighted graphs or graphs with uniform weights, uses BFS.
    For weighted graphs, uses Dijkstra's algorithm.

    Detects weights: if any edge has weight != 1.0, uses Dijkstra.

    Algorithm (BFS for unweighted):
    1. Track parent of each node during BFS
    2. When reaching end, trace back through parents to start
    3. Reverse the path to get start → end order

    Algorithm (Dijkstra for weighted):
    1. Initialize distances: start=0, others=infinity
    2. Use min-heap to always expand closest unvisited node
    3. For each neighbor, update distance if we found a shorter path
    4. Trace back through parent map

    Example (unweighted):
        Graph: A—B—C, B—D
        shortest_path(graph, A, D) = [A, B, D]

    Example (weighted):
        Graph: A—B (weight 1), B—C (weight 10), A—C (weight 2)
        shortest_path(graph, A, C) = [A, C] (total weight 2, not 11)

    Args:
        graph: The graph to search
        start: Starting node
        end: Target node

    Returns:
        List of nodes forming the shortest path (empty if no path exists)

    Raises:
        NodeNotFoundError: if start or end doesn't exist

    Time:
        - Unweighted: O(V + E)
        - Weighted (Dijkstra): O((V + E) log V) with binary heap
    Space: O(V)
    """
    if not graph.has_node(start):
        raise NodeNotFoundError(f"Start node {start} not found")
    if not graph.has_node(end):
        raise NodeNotFoundError(f"End node {end} not found")

    # Check if graph has weighted edges
    has_weights = any(weight != 1.0 for _, _, weight in graph.edges())

    if not has_weights:
        # BFS for unweighted graphs
        parent = {start: None}
        queue = deque([start])

        while queue:
            node = queue.popleft()
            if node == end:
                break

            for neighbor in graph.neighbors(node):
                if neighbor not in parent:
                    parent[neighbor] = node
                    queue.append(neighbor)

        # Trace back from end to start
        if end not in parent:
            return []

        path = []
        current = end
        while current is not None:
            path.append(current)
            current = parent[current]

        return list(reversed(path))
    else:
        # Dijkstra's algorithm for weighted graphs
        import heapq

        # Initialize distances
        dist = {node: float("inf") for node in graph.nodes()}
        dist[start] = 0
        parent = {}

        # Min-heap: (distance, node)
        pq = [(0, start)]

        while pq:
            d, node = heapq.heappop(pq)

            # Skip if this is a stale entry
            if d > dist[node]:
                continue

            # Stop if we reached the end
            if node == end:
                break

            # Relax edges
            for neighbor, weight in graph.neighbors_weighted(node).items():
                new_dist = dist[node] + weight
                if new_dist < dist[neighbor]:
                    dist[neighbor] = new_dist
                    parent[neighbor] = node
                    heapq.heappush(pq, (new_dist, neighbor))

        # Trace back from end to start
        if end not in parent and start != end:
            return []

        if start == end:
            return [start]

        path = []
        current = end
        while current is not None:
            path.append(current)
            current = parent.get(current)

        return list(reversed(path))


class UnionFind:
    """
    Union-Find (Disjoint Set Union) data structure for Kruskal's algorithm.

    Efficiently tracks which nodes belong to the same connected component.

    Key operations:
    - find(x): return the representative of x's set
    - union(x, y): merge the sets containing x and y

    Optimization: path compression — when finding a root, update all nodes
    along the path to point directly to the root. This makes future finds
    nearly O(1).

    Time: O(E log E) for sorting + O(E · α(V)) for E union/find ops
    where α is the inverse Ackermann function (effectively O(1) in practice)
    """

    def __init__(self, nodes: list[T]) -> None:
        """Initialize UnionFind with all nodes as separate sets."""
        self.parent: dict[T, T] = {node: node for node in nodes}
        self.rank: dict[T, int] = {node: 0 for node in nodes}

    def find(self, x: T) -> T:
        """
        Find the root representative of x's set (with path compression).

        Path compression: after finding the root, update x to point directly
        to it. This speeds up future finds from O(log V) to nearly O(1).
        """
        if self.parent[x] != x:
            self.parent[x] = self.find(self.parent[x])
        return self.parent[x]

    def union(self, x: T, y: T) -> bool:
        """
        Merge the sets containing x and y.

        Returns True if x and y were in different sets (merge happened).
        Returns False if they were already in the same set.

        Uses union by rank to keep trees shallow.
        """
        root_x = self.find(x)
        root_y = self.find(y)

        if root_x == root_y:
            return False

        # Union by rank: attach smaller tree to larger
        if self.rank[root_x] < self.rank[root_y]:
            self.parent[root_x] = root_y
        elif self.rank[root_x] > self.rank[root_y]:
            self.parent[root_y] = root_x
        else:
            self.parent[root_y] = root_x
            self.rank[root_x] += 1

        return True


def minimum_spanning_tree(graph: Graph[T]) -> frozenset[tuple[T, T, float]]:
    """
    Find the Minimum Spanning Tree (MST) of a connected graph.

    A spanning tree is a subset of edges that connects all nodes with no cycles.
    A minimum spanning tree minimizes the total edge weight.

    Real-world use case: you need to lay cables to connect 5 cities. A spanning
    tree means every city is reachable. A minimum spanning tree minimizes total
    cable length.

    Algorithm (Kruskal's):
    1. Sort all edges by weight (ascending)
    2. Initialize UnionFind with all nodes
    3. For each edge in sorted order:
       - If the edge connects two different components, add it
       - Otherwise, skip it (would create a cycle)
    4. Stop after adding V-1 edges (complete tree)

    Example:
        Graph edges: (A,C,1), (C,D,2), (A,B,3), (B,E,4), (D,E,5)
        Sorted:      (A,C,1), (C,D,2), (A,B,3), (B,E,4), (D,E,5)

        MST construction:
        1. Add (A,C,1) → {A,C}
        2. Add (C,D,2) → {A,C,D}
        3. Add (A,B,3) → {A,C,D,B}
        4. Add (B,E,4) → {A,C,D,B,E} ← 4 edges for 5 nodes, done!
        5. Skip (D,E,5) ← would create cycle

        MST edges: {(A,C,1), (C,D,2), (A,B,3), (B,E,4)}, total weight = 10

    Args:
        graph: A connected undirected graph

    Returns:
        Set of (u, v, weight) tuples forming the MST

    Raises:
        ValueError: if graph is empty or disconnected

    Time: O(E log E) for sorting + O(E α(V)) for union-find ops
    Space: O(V + E)
    """
    if len(graph) == 0:
        return frozenset()

    if not is_connected(graph):
        raise ValueError("Graph must be connected to have a spanning tree")

    # Sort edges by weight
    edges = sorted(graph.edges(), key=lambda e: e[2])

    # Initialize UnionFind
    uf = UnionFind(list(graph.nodes()))

    result = set()
    for u, v, weight in edges:
        if uf.union(u, v):
            result.add((u, v, weight))
        if len(result) == len(graph) - 1:
            break

    return frozenset(result)
