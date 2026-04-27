// ============================================================================
// Graph.swift — Undirected Weighted Graph
// ============================================================================
//
// A graph G = (V, E) is a collection of:
//   V — vertices (nodes): any Hashable type
//   E — edges: unordered pairs {u, v} with optional weights (default 1.0)
//
// Two Representations
// -------------------
// ADJACENCY_LIST (default):
//   A Dictionary mapping each node to a Dictionary of neighbours with weights.
//   Space: O(V + E) — only stores existing edges.
//   Edge lookup: O(degree) — scan neighbour dictionary.
//   Best for sparse graphs (most real-world graphs).
//
// ADJACENCY_MATRIX:
//   A V×V matrix where matrix[i][j] = weight (0.0 means no edge).
//   Nodes are mapped to integer indices for array addressing.
//   Space: O(V²) — allocates a slot for every possible edge.
//   Edge lookup: O(1) — single array access.
//   Best for dense graphs or when O(1) edge lookup is critical.
//
// Both representations expose the same public API.
// ============================================================================

import Foundation

// MARK: - Graph Representation Enum

public enum GraphRepr {
    case adjacencyList
    case adjacencyMatrix
}

// MARK: - Error Types

public enum GraphError: Error, Equatable {
    case nodeNotFound(String)
    case edgeNotFound(String, String)
    case graphNotConnected
    case invalidRepresentation
}

// MARK: - Graph Structure

public struct Graph<Node: Hashable & CustomStringConvertible> {
    // Internal representation type
    private let repr: GraphRepr

    // For adjacency list representation:
    // _adjList[u][v] = weight for edge {u, v}
    private var adjList: [Node: [Node: Double]] = [:]

    // For adjacency matrix representation:
    // _nodeList: ordered list of nodes for matrix indexing
    // _nodeIdx: node -> row/col index mapping
    // _matrix: V×V matrix (symmetric for undirected)
    private var nodeList: [Node] = []
    private var nodeIdx: [Node: Int] = [:]
    private var matrix: [[Double]] = []

    // MARK: - Initialization

    /// Create a new Graph with the specified representation.
    /// Default is adjacency list (better for sparse graphs).
    public init(repr: GraphRepr = .adjacencyList) {
        self.repr = repr

        if repr == .adjacencyList {
            self.adjList = [:]
        } else {
            self.nodeList = []
            self.nodeIdx = [:]
            self.matrix = []
        }
    }

    // MARK: - Node Operations

    /// Add a node to the graph. No-op if the node already exists.
    public mutating func addNode(_ node: Node) {
        if repr == .adjacencyList {
            if adjList[node] == nil {
                adjList[node] = [:]
            }
        } else {
            guard nodeIdx[node] == nil else { return }

            let idx = nodeList.count
            nodeList.append(node)
            nodeIdx[node] = idx

            // Add column to existing rows
            for i in 0..<matrix.count {
                matrix[i].append(0.0)
            }
            // Add new row
            matrix.append(Array(repeating: 0.0, count: idx + 1))
        }
    }

    /// Remove a node and all its incident edges.
    /// Raises GraphError.nodeNotFound if the node doesn't exist.
    public mutating func removeNode(_ node: Node) throws {
        if repr == .adjacencyList {
            guard adjList[node] != nil else {
                throw GraphError.nodeNotFound(node.description)
            }

            // Remove all edges touching this node
            for neighbour in adjList[node]!.keys {
                adjList[neighbour]?.removeValue(forKey: node)
            }
            adjList.removeValue(forKey: node)
        } else {
            guard let idx = nodeIdx[node] else {
                throw GraphError.nodeNotFound(node.description)
            }

            nodeIdx.removeValue(forKey: node)
            nodeList.remove(at: idx)

            // Update indices for nodes that shifted down
            for i in idx..<nodeList.count {
                nodeIdx[nodeList[i]] = i
            }

            // Remove row
            matrix.remove(at: idx)

            // Remove column from remaining rows
            for i in 0..<matrix.count {
                matrix[i].remove(at: idx)
            }
        }
    }

    /// Check if a node exists in the graph.
    public func hasNode(_ node: Node) -> Bool {
        if repr == .adjacencyList {
            return adjList[node] != nil
        } else {
            return nodeIdx[node] != nil
        }
    }

    /// Return all nodes in the graph.
    public var nodes: [Node] {
        if repr == .adjacencyList {
            return Array(adjList.keys)
        } else {
            return nodeList
        }
    }

    /// Return the number of nodes.
    public var count: Int {
        if repr == .adjacencyList {
            return adjList.count
        } else {
            return nodeList.count
        }
    }

    // MARK: - Edge Operations

    /// Add an undirected edge between u and v with the given weight.
    /// Both nodes are added automatically if they don't exist.
    public mutating func addEdge(_ u: Node, _ v: Node, weight: Double = 1.0) {
        addNode(u)
        addNode(v)

        if repr == .adjacencyList {
            adjList[u]![v] = weight
            adjList[v]![u] = weight
        } else {
            let i = nodeIdx[u]!
            let j = nodeIdx[v]!
            matrix[i][j] = weight
            matrix[j][i] = weight
        }
    }

    /// Remove the edge between u and v.
    /// Raises GraphError.edgeNotFound if either node or the edge doesn't exist.
    public mutating func removeEdge(_ u: Node, _ v: Node) throws {
        if repr == .adjacencyList {
            guard adjList[u] != nil, adjList[u]![v] != nil else {
                throw GraphError.edgeNotFound(u.description, v.description)
            }
            adjList[u]!.removeValue(forKey: v)
            adjList[v]!.removeValue(forKey: u)
        } else {
            guard let i = nodeIdx[u], let j = nodeIdx[v] else {
                throw GraphError.edgeNotFound(u.description, v.description)
            }
            guard matrix[i][j] != 0.0 else {
                throw GraphError.edgeNotFound(u.description, v.description)
            }
            matrix[i][j] = 0.0
            matrix[j][i] = 0.0
        }
    }

    /// Check if an edge exists between u and v.
    public func hasEdge(_ u: Node, _ v: Node) -> Bool {
        if repr == .adjacencyList {
            return adjList[u]?[v] != nil
        } else {
            guard let i = nodeIdx[u], let j = nodeIdx[v] else { return false }
            return matrix[i][j] != 0.0
        }
    }

    /// Return all edges as an array of (u, v, weight) tuples.
    /// Each undirected edge appears exactly once.
    public var edges: [(Node, Node, Double)] {
        var result: [(Node, Node, Double)] = []

        if repr == .adjacencyList {
            var seen = Set<String>()
            for u in adjList.keys {
                for (v, weight) in adjList[u]! {
                    let key = "\(min(u.description, v.description)),\(max(u.description, v.description))"
                    guard !seen.contains(key) else { continue }
                    seen.insert(key)
                    result.append((u, v, weight))
                }
            }
        } else {
            let n = nodeList.count
            for i in 0..<n {
                for j in (i + 1)..<n {
                    let weight = matrix[i][j]
                    if weight != 0.0 {
                        result.append((nodeList[i], nodeList[j], weight))
                    }
                }
            }
        }

        return result
    }

    /// Get the weight of the edge between u and v.
    /// Raises GraphError.edgeNotFound if the edge doesn't exist.
    public func edgeWeight(_ u: Node, _ v: Node) throws -> Double {
        if repr == .adjacencyList {
            guard let weight = adjList[u]?[v] else {
                throw GraphError.edgeNotFound(u.description, v.description)
            }
            return weight
        } else {
            guard let i = nodeIdx[u], let j = nodeIdx[v] else {
                throw GraphError.edgeNotFound(u.description, v.description)
            }
            let weight = matrix[i][j]
            guard weight != 0.0 else {
                throw GraphError.edgeNotFound(u.description, v.description)
            }
            return weight
        }
    }

    // MARK: - Neighbourhood Queries

    /// Return all neighbours of a node.
    /// Raises GraphError.nodeNotFound if the node doesn't exist.
    public func neighbors(_ node: Node) throws -> [Node] {
        if repr == .adjacencyList {
            guard let neighs = adjList[node] else {
                throw GraphError.nodeNotFound(node.description)
            }
            return Array(neighs.keys)
        } else {
            guard let idx = nodeIdx[node] else {
                throw GraphError.nodeNotFound(node.description)
            }
            var result: [Node] = []
            for j in 0..<matrix[idx].count {
                if matrix[idx][j] != 0.0 {
                    result.append(nodeList[j])
                }
            }
            return result
        }
    }

    /// Return weighted neighbours as a dictionary {neighbour: weight}.
    /// Raises GraphError.nodeNotFound if the node doesn't exist.
    public func neighborsWeighted(_ node: Node) throws -> [Node: Double] {
        if repr == .adjacencyList {
            guard let neighs = adjList[node] else {
                throw GraphError.nodeNotFound(node.description)
            }
            return neighs
        } else {
            guard let idx = nodeIdx[node] else {
                throw GraphError.nodeNotFound(node.description)
            }
            var result: [Node: Double] = [:]
            for j in 0..<matrix[idx].count {
                let weight = matrix[idx][j]
                if weight != 0.0 {
                    result[nodeList[j]] = weight
                }
            }
            return result
        }
    }

    /// Return the degree of a node (number of incident edges).
    /// Raises GraphError.nodeNotFound if the node doesn't exist.
    public func degree(_ node: Node) throws -> Int {
        return try neighbors(node).count
    }

    // MARK: - Computed Properties

    /// Check if the graph is connected (all nodes reachable from any node).
    public var isConnected: Bool {
        guard !nodes.isEmpty else { return true }
        let start = nodes[0]
        let reachable = (try? bfsPrivate(start)) ?? []
        return reachable.count == nodes.count
    }

    // MARK: - Algorithms

    /// Breadth-first search from start node.
    /// Returns nodes in BFS order.
    /// Time: O(V + E)
    public func bfs(_ start: Node) -> [Node] {
        guard hasNode(start) else { return [] }

        var visited = Set<Node>([start])
        var queue = [start]
        var result: [Node] = []

        while !queue.isEmpty {
            let node = queue.removeFirst()
            result.append(node)

            let neighbors = (try? self.neighbors(node)) ?? []
            for neighbor in neighbors.sorted(by: { $0.description < $1.description }) {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor)
                    queue.append(neighbor)
                }
            }
        }

        return result
    }

    /// Depth-first search from start node.
    /// Returns nodes in DFS order.
    /// Time: O(V + E)
    public func dfs(_ start: Node) -> [Node] {
        guard hasNode(start) else { return [] }

        var visited = Set<Node>()
        var stack = [start]
        var result: [Node] = []

        while !stack.isEmpty {
            let node = stack.removeLast()
            guard !visited.contains(node) else { continue }

            visited.insert(node)
            result.append(node)

            let neighbors = (try? self.neighbors(node)) ?? []
            for neighbor in neighbors.sorted(by: { $0.description > $1.description }) {
                if !visited.contains(neighbor) {
                    stack.append(neighbor)
                }
            }
        }

        return result
    }

    /// Find shortest (lowest-weight) path from start to end.
    /// Returns empty array if no path exists.
    /// Uses BFS for unweighted graphs, Dijkstra for weighted.
    /// Time: O(V + E) for BFS, O((V + E) log V) for Dijkstra
    public func shortestPath(_ start: Node, _ end: Node) -> [Node] {
        guard start != end || hasNode(start) else { return [] }
        guard start != end else { return [start] }

        // Check if all weights are 1.0
        let allUnit = edges.allSatisfy { $0.2 == 1.0 }

        if allUnit {
            return bfsPath(start, end)
        } else {
            return dijkstra(start, end)
        }
    }

    /// Check if the graph contains a cycle.
    /// Uses iterative DFS.
    /// Time: O(V + E)
    public func hasCycle() -> Bool {
        var visited = Set<Node>()

        for start in nodes {
            guard !visited.contains(start) else { continue }

            var stack: [(Node, Node?)] = [(start, nil)]

            while !stack.isEmpty {
                let (node, parent) = stack.removeLast()
                guard !visited.contains(node) else { continue }

                visited.insert(node)

                let neighbors = (try? self.neighbors(node)) ?? []
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        stack.append((neighbor, node))
                    } else if neighbor != parent {
                        // Back edge to visited node that isn't parent -> cycle
                        return true
                    }
                }
            }
        }

        return false
    }

    /// Check if the graph is bipartite (2-colorable).
    /// Time: O(V + E)
    public func isBipartite() -> Bool {
        var color: [Node: Int] = [:]

        for start in nodes {
            guard color[start] == nil else { continue }

            var queue = [start]
            color[start] = 0

            while !queue.isEmpty {
                let node = queue.removeFirst()
                let nodeColor = color[node]!

                let neighbors = (try? self.neighbors(node)) ?? []
                for neighbor in neighbors {
                    if color[neighbor] == nil {
                        color[neighbor] = 1 - nodeColor
                        queue.append(neighbor)
                    } else if color[neighbor] == nodeColor {
                        // Adjacent nodes have same color -> not bipartite
                        return false
                    }
                }
            }
        }

        return true
    }

    /// Find the minimum spanning tree using Kruskal's algorithm.
    /// Raises GraphError.graphNotConnected if graph is not connected.
    /// Time: O(E log E)
    public func minimumSpanningTree() throws -> [(Node, Node, Double)] {
        guard !nodes.isEmpty else { return [] }

        let sortedEdges = edges.sorted { $0.2 < $1.2 }
        var uf = UnionFind<Node>()

        for node in nodes {
            uf.makeSet(node)
        }

        var mst: [(Node, Node, Double)] = []

        for (u, v, w) in sortedEdges {
            if uf.find(u) != uf.find(v) {
                uf.union(u, v)
                mst.append((u, v, w))
                if mst.count == nodes.count - 1 {
                    break
                }
            }
        }

        if mst.count < nodes.count - 1 && nodes.count > 1 {
            throw GraphError.graphNotConnected
        }

        return mst
    }

    // MARK: - Private Helpers

    private func bfsPath(_ start: Node, _ end: Node) -> [Node] {
        var parent: [Node: Node?] = [start: nil]
        var queue = [start]

        while !queue.isEmpty {
            let node = queue.removeFirst()
            if node == end {
                break
            }

            let neighbors = (try? self.neighbors(node)) ?? []
            for neighbor in neighbors {
                if parent[neighbor] == nil {
                    parent[neighbor] = node
                    queue.append(neighbor)
                }
            }
        }

        guard parent[end] != nil else { return [] }

        // Trace back from end to start
        var path: [Node] = []
        var cur: Node? = end
        while cur != nil {
            path.insert(cur!, at: 0)
            cur = parent[cur!]!
        }

        return path
    }

    private func dijkstra(_ start: Node, _ end: Node) -> [Node] {
        var dist: [Node: Double] = [:]
        var parent: [Node: Node?] = [:]

        for node in nodes {
            dist[node] = Double.infinity
        }
        dist[start] = 0

        var heap: [(Node, Double)] = [(start, 0)]

        while !heap.isEmpty {
            heap.sort { $0.1 < $1.1 }
            let (node, d) = heap.removeFirst()

            guard d <= (dist[node] ?? Double.infinity) else { continue }
            if node == end { break }

            let neighbors = (try? neighborsWeighted(node)) ?? [:]
            for (neighbor, weight) in neighbors {
                let newDist = dist[node]! + weight
                if newDist < (dist[neighbor] ?? Double.infinity) {
                    dist[neighbor] = newDist
                    parent[neighbor] = node
                    heap.append((neighbor, newDist))
                }
            }
        }

        guard (dist[end] ?? Double.infinity) < Double.infinity else { return [] }

        // Trace back
        var path: [Node] = []
        var cur: Node? = end
        while cur != nil {
            path.insert(cur!, at: 0)
            cur = parent[cur!] ?? nil
        }

        return path
    }

    private func bfsPrivate(_ start: Node) throws -> [Node] {
        var visited = Set<Node>([start])
        var queue = [start]
        var result: [Node] = []

        while !queue.isEmpty {
            let node = queue.removeFirst()
            result.append(node)

            let neighbors = try self.neighbors(node)
            for neighbor in neighbors.sorted(by: { $0.description < $1.description }) {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor)
                    queue.append(neighbor)
                }
            }
        }

        return result
    }

    // MARK: - Helper: Union-Find for MST

    private struct UnionFind<Element: Hashable> {
        private var parent: [Element: Element] = [:]
        private var rank: [Element: Int] = [:]

        mutating func makeSet(_ x: Element) {
            parent[x] = x
            rank[x] = 0
        }

        mutating func find(_ x: Element) -> Element {
            guard let p = parent[x], p != x else { return x }
            parent[x] = find(p)
            return parent[x]!
        }

        mutating func union(_ a: Element, _ b: Element) {
            var ra = find(a)
            var rb = find(b)

            guard ra != rb else { return }

            if (rank[ra] ?? 0) < (rank[rb] ?? 0) {
                (ra, rb) = (rb, ra)
            }
            parent[rb] = ra
            if rank[ra] == rank[rb] {
                rank[ra] = (rank[ra] ?? 0) + 1
            }
        }
    }
}
