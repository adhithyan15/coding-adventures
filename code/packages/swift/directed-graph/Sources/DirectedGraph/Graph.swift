// ============================================================================
// Graph.swift — A Directed Graph Data Structure
// ============================================================================
//
// A directed graph stores **nodes** (vertices) and **edges** (arrows between
// nodes). Unlike an undirected graph where edges have no direction, a directed
// graph's edges go *from* one node *to* another — like one-way streets.
//
//     A ──→ B ──→ C
//     │           ▲
//     └───────────┘
//
// In this graph, A has edges to B and C, B has an edge to C, but there is
// no edge from C back to A or B.
//
// Directed graphs appear everywhere in computing:
//
// - **Build systems**: Package A depends on package B. Build B first.
// - **Task scheduling**: Task X must finish before task Y starts.
// - **Compiler passes**: Lexer → Parser → Type Checker → Code Generator.
// - **State machines**: State transitions in protocol handlers.
//
// This implementation uses **dual adjacency maps**: for each node, we store
// both its successors (outgoing edges) and predecessors (incoming edges).
// This doubles memory but gives O(1) lookups in both directions — essential
// for algorithms like topological sort and affected-node computation.
//
// ============================================================================

// MARK: - Error Types

/// Error thrown when a cycle is detected during topological sort.
///
/// A cycle means there is a circular dependency: A depends on B, B depends
/// on C, and C depends on A. No valid ordering exists in this case.
public struct CycleError: Error, CustomStringConvertible {
    /// The nodes forming the cycle, in order.
    public let cycle: [String]

    public var description: String {
        "Cycle detected: \(cycle.joined(separator: " → "))"
    }
}

/// Error thrown when an operation references a node that does not exist.
public struct NodeNotFoundError: Error, CustomStringConvertible {
    public let node: String

    public var description: String {
        "Node not found: '\(node)'"
    }
}

/// Error thrown when an operation references an edge that does not exist.
public struct EdgeNotFoundError: Error, CustomStringConvertible {
    public let fromNode: String
    public let toNode: String

    public var description: String {
        "Edge not found: '\(fromNode)' → '\(toNode)'"
    }
}

// MARK: - Graph

/// A directed graph with string-typed nodes.
///
/// Supports adding/removing nodes and edges, querying neighbors in both
/// directions, topological sorting, cycle detection, transitive closure,
/// and computation of affected nodes for incremental rebuilds.
///
/// Example:
///
///     var graph = Graph()
///     graph.addNode("A")
///     graph.addNode("B")
///     graph.addEdge(from: "A", to: "B")
///     print(graph.successors(of: "A"))  // ["B"]
///     print(graph.predecessors(of: "B"))  // ["A"]
///
public struct Graph: Sendable {

    // -----------------------------------------------------------------------
    // Storage
    // -----------------------------------------------------------------------

    /// Forward adjacency: node → set of successors.
    private var _forward: [String: Set<String>] = [:]

    /// Reverse adjacency: node → set of predecessors.
    private var _reverse: [String: Set<String>] = [:]

    /// Whether self-loops (edges from a node to itself) are permitted.
    public let allowSelfLoops: Bool

    // -----------------------------------------------------------------------
    // Initialization
    // -----------------------------------------------------------------------

    /// Create an empty directed graph.
    ///
    /// - Parameter allowSelfLoops: If `true`, a node may have an edge to
    ///   itself. Defaults to `false`.
    public init(allowSelfLoops: Bool = false) {
        self.allowSelfLoops = allowSelfLoops
    }

    // -----------------------------------------------------------------------
    // Node Operations
    // -----------------------------------------------------------------------

    /// The number of nodes in the graph.
    public var size: Int { _forward.count }

    /// Add a node. If the node already exists, this is a no-op.
    public mutating func addNode(_ node: String) {
        if _forward[node] == nil {
            _forward[node] = []
            _reverse[node] = []
        }
    }

    /// Remove a node and all its incident edges.
    ///
    /// - Throws: `NodeNotFoundError` if the node does not exist.
    public mutating func removeNode(_ node: String) throws {
        guard _forward[node] != nil else {
            throw NodeNotFoundError(node: node)
        }
        // Remove all outgoing edges.
        for successor in _forward[node]! {
            _reverse[successor]?.remove(node)
        }
        // Remove all incoming edges.
        for predecessor in _reverse[node]! {
            _forward[predecessor]?.remove(node)
        }
        _forward.removeValue(forKey: node)
        _reverse.removeValue(forKey: node)
    }

    /// Check whether the graph contains a node.
    public func hasNode(_ node: String) -> Bool {
        _forward[node] != nil
    }

    /// Return all nodes in the graph, sorted alphabetically.
    public func nodes() -> [String] {
        _forward.keys.sorted()
    }

    // -----------------------------------------------------------------------
    // Edge Operations
    // -----------------------------------------------------------------------

    /// Add a directed edge from one node to another.
    ///
    /// Both nodes are automatically created if they don't exist yet.
    /// If the edge already exists, this is a no-op.
    ///
    /// - Throws: `CycleError` if `allowSelfLoops` is false and `from == to`.
    public mutating func addEdge(from fromNode: String, to toNode: String) throws {
        if !allowSelfLoops && fromNode == toNode {
            throw CycleError(cycle: [fromNode, toNode])
        }
        addNode(fromNode)
        addNode(toNode)
        _forward[fromNode]!.insert(toNode)
        _reverse[toNode]!.insert(fromNode)
    }

    /// Remove a directed edge.
    ///
    /// - Throws: `EdgeNotFoundError` if the edge does not exist.
    public mutating func removeEdge(from fromNode: String, to toNode: String) throws {
        guard _forward[fromNode]?.contains(toNode) == true else {
            throw EdgeNotFoundError(fromNode: fromNode, toNode: toNode)
        }
        _forward[fromNode]!.remove(toNode)
        _reverse[toNode]!.remove(fromNode)
    }

    /// Check whether a directed edge exists.
    public func hasEdge(from fromNode: String, to toNode: String) -> Bool {
        _forward[fromNode]?.contains(toNode) == true
    }

    /// Return all edges as (from, to) pairs, sorted.
    public func edges() -> [(String, String)] {
        var result: [(String, String)] = []
        for (node, successors) in _forward {
            for successor in successors.sorted() {
                result.append((node, successor))
            }
        }
        return result.sorted { ($0.0, $0.1) < ($1.0, $1.1) }
    }

    // -----------------------------------------------------------------------
    // Neighbor Queries
    // -----------------------------------------------------------------------

    /// Return the successors of a node (nodes it points to), sorted.
    ///
    /// - Throws: `NodeNotFoundError` if the node does not exist.
    public func successors(of node: String) throws -> [String] {
        guard let succs = _forward[node] else {
            throw NodeNotFoundError(node: node)
        }
        return succs.sorted()
    }

    /// Return the predecessors of a node (nodes pointing to it), sorted.
    ///
    /// - Throws: `NodeNotFoundError` if the node does not exist.
    public func predecessors(of node: String) throws -> [String] {
        guard let preds = _reverse[node] else {
            throw NodeNotFoundError(node: node)
        }
        return preds.sorted()
    }

    // -----------------------------------------------------------------------
    // Algorithms
    // -----------------------------------------------------------------------

    /// Topological sort using Kahn's algorithm.
    ///
    /// Returns nodes in an order where every node appears before all nodes
    /// it has edges to. This is the order you should process dependencies:
    /// if A → B, then A appears before B.
    ///
    /// - Throws: `CycleError` if the graph contains a cycle.
    public func topologicalSort() throws -> [String] {
        // Kahn's algorithm:
        // 1. Find all nodes with no incoming edges (in-degree 0).
        // 2. Remove them from the graph, add to result.
        // 3. Repeat until no nodes remain. If nodes remain but all have
        //    incoming edges, the graph has a cycle.

        var inDegree: [String: Int] = [:]
        for node in _forward.keys {
            inDegree[node] = _reverse[node]?.count ?? 0
        }

        // Queue of nodes with in-degree 0 (sorted for deterministic output).
        var queue = inDegree.filter { $0.value == 0 }.map { $0.key }.sorted()
        var result: [String] = []

        while !queue.isEmpty {
            let node = queue.removeFirst()
            result.append(node)

            for successor in (_forward[node] ?? []).sorted() {
                inDegree[successor]! -= 1
                if inDegree[successor] == 0 {
                    // Insert in sorted position to maintain determinism.
                    let insertIdx = queue.firstIndex { $0 > successor } ?? queue.endIndex
                    queue.insert(successor, at: insertIdx)
                }
            }
        }

        if result.count != _forward.count {
            // Some nodes were never processed — they are in a cycle.
            let cycleNodes = _forward.keys.filter { !result.contains($0) }.sorted()
            throw CycleError(cycle: cycleNodes)
        }

        return result
    }

    /// Check whether the graph contains a cycle.
    public func hasCycle() -> Bool {
        do {
            _ = try topologicalSort()
            return false
        } catch {
            return true
        }
    }

    /// Compute the transitive closure of a node — all nodes reachable
    /// by following edges forward from the given node.
    ///
    /// - Throws: `NodeNotFoundError` if the node does not exist.
    public func transitiveClosure(of node: String) throws -> Set<String> {
        guard _forward[node] != nil else {
            throw NodeNotFoundError(node: node)
        }
        var visited = Set<String>()
        var stack = [node]
        while let current = stack.popLast() {
            if visited.contains(current) { continue }
            visited.insert(current)
            for successor in _forward[current] ?? [] {
                stack.append(successor)
            }
        }
        visited.remove(node) // Don't include the starting node itself.
        return visited
    }

    /// Compute the transitive dependents of a node — all nodes that
    /// can reach this node by following edges backward.
    ///
    /// - Throws: `NodeNotFoundError` if the node does not exist.
    public func transitiveDependents(of node: String) throws -> Set<String> {
        guard _reverse[node] != nil else {
            throw NodeNotFoundError(node: node)
        }
        var visited = Set<String>()
        var stack = [node]
        while let current = stack.popLast() {
            if visited.contains(current) { continue }
            visited.insert(current)
            for predecessor in _reverse[current] ?? [] {
                stack.append(predecessor)
            }
        }
        visited.remove(node)
        return visited
    }

    /// Compute independent groups — nodes that can be processed in parallel.
    ///
    /// Each group contains nodes whose dependencies are all satisfied by
    /// nodes in earlier groups. Within a group, all nodes are independent.
    ///
    /// - Throws: `CycleError` if the graph contains a cycle.
    public func independentGroups() throws -> [[String]] {
        var inDegree: [String: Int] = [:]
        for node in _forward.keys {
            inDegree[node] = _reverse[node]?.count ?? 0
        }

        var groups: [[String]] = []
        var remaining = inDegree

        while !remaining.isEmpty {
            let ready = remaining.filter { $0.value == 0 }.map { $0.key }.sorted()
            if ready.isEmpty {
                let cycleNodes = remaining.keys.sorted()
                throw CycleError(cycle: cycleNodes)
            }
            groups.append(ready)
            for node in ready {
                remaining.removeValue(forKey: node)
                for successor in _forward[node] ?? [] {
                    if remaining[successor] != nil {
                        remaining[successor]! -= 1
                    }
                }
            }
        }

        return groups
    }

    /// Compute all nodes affected by changes to the given set of nodes.
    ///
    /// Returns the changed nodes plus all nodes that transitively depend
    /// on any changed node (following edges backward from each changed node's
    /// dependents).
    public func affectedNodes(changed: Set<String>) -> Set<String> {
        var affected = changed
        for node in changed {
            if let dependents = try? transitiveDependents(of: node) {
                affected.formUnion(dependents)
            }
        }
        return affected
    }
}
