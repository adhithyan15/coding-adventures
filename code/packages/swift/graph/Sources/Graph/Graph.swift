import Foundation

public enum GraphRepr: String, CaseIterable, Sendable {
    case adjacencyList = "adjacency_list"
    case adjacencyMatrix = "adjacency_matrix"
}

public typealias WeightedEdge = (String, String, Double)

public enum GraphPropertyValue: Equatable, Sendable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case null
}

public typealias GraphPropertyBag = [String: GraphPropertyValue]

public struct NodeNotFoundError: Error, CustomStringConvertible {
    public let node: String
    public var description: String { "Node not found: '\(node)'" }
}

public struct EdgeNotFoundError: Error, CustomStringConvertible {
    public let left: String
    public let right: String
    public var description: String { "Edge not found: '\(left)' -- '\(right)'" }
}

public struct NotConnectedError: Error, CustomStringConvertible {
    public var description: String { "graph is not connected" }
}

public struct Graph: Sendable {
    public let repr: GraphRepr
    private var adj: [String: [String: Double]]
    private var nodeList: [String]
    private var nodeIndex: [String: Int]
    private var matrix: [[Double?]]
    private var graphPropertiesStore: GraphPropertyBag
    private var nodePropertiesStore: [String: GraphPropertyBag]
    private var edgePropertiesStore: [String: GraphPropertyBag]

    public init(repr: GraphRepr = .adjacencyList) {
        self.repr = repr
        self.adj = [:]
        self.nodeList = []
        self.nodeIndex = [:]
        self.matrix = []
        self.graphPropertiesStore = [:]
        self.nodePropertiesStore = [:]
        self.edgePropertiesStore = [:]
    }

    public var size: Int {
        repr == .adjacencyList ? adj.count : nodeList.count
    }

    public mutating func addNode(_ node: String, properties: GraphPropertyBag = [:]) {
        if repr == .adjacencyList {
            if adj[node] == nil {
                adj[node] = [:]
            }
            mergeNodeProperties(node, properties)
            return
        }

        if nodeIndex[node] != nil {
            mergeNodeProperties(node, properties)
            return
        }

        let index = nodeList.count
        nodeList.append(node)
        nodeIndex[node] = index
        for row in matrix.indices {
            matrix[row].append(nil)
        }
        matrix.append(Array(repeating: nil, count: index + 1))
        mergeNodeProperties(node, properties)
    }

    public mutating func removeNode(_ node: String) throws {
        if repr == .adjacencyList {
            guard let neighbors = adj[node] else {
                throw NodeNotFoundError(node: node)
            }
            for neighbor in neighbors.keys {
                adj[neighbor]?.removeValue(forKey: node)
                edgePropertiesStore.removeValue(forKey: edgeKey(node, neighbor))
            }
            adj.removeValue(forKey: node)
            nodePropertiesStore.removeValue(forKey: node)
            return
        }

        guard let index = nodeIndex[node] else {
            throw NodeNotFoundError(node: node)
        }

        for other in nodeList {
            edgePropertiesStore.removeValue(forKey: edgeKey(node, other))
        }
        nodePropertiesStore.removeValue(forKey: node)
        nodeIndex.removeValue(forKey: node)
        nodeList.remove(at: index)
        matrix.remove(at: index)
        for row in matrix.indices {
            matrix[row].remove(at: index)
        }
        nodeIndex = Dictionary(uniqueKeysWithValues: nodeList.enumerated().map { ($1, $0) })
    }

    public func hasNode(_ node: String) -> Bool {
        repr == .adjacencyList ? adj[node] != nil : nodeIndex[node] != nil
    }

    public func nodes() -> [String] {
        let values = repr == .adjacencyList ? Array(adj.keys) : nodeList
        return values.sorted()
    }

    public mutating func addEdge(_ left: String, _ right: String, weight: Double = 1.0, properties: GraphPropertyBag = [:]) {
        addNode(left)
        addNode(right)

        if repr == .adjacencyList {
            adj[left, default: [:]][right] = weight
            adj[right, default: [:]][left] = weight
            mergeEdgeProperties(left, right, weight: weight, properties: properties)
            return
        }

        let leftIndex = nodeIndex[left]!
        let rightIndex = nodeIndex[right]!
        matrix[leftIndex][rightIndex] = weight
        matrix[rightIndex][leftIndex] = weight
        mergeEdgeProperties(left, right, weight: weight, properties: properties)
    }

    public mutating func removeEdge(_ left: String, _ right: String) throws {
        if repr == .adjacencyList {
            guard adj[left]?[right] != nil else {
                throw EdgeNotFoundError(left: left, right: right)
            }
            adj[left]?.removeValue(forKey: right)
            adj[right]?.removeValue(forKey: left)
            edgePropertiesStore.removeValue(forKey: edgeKey(left, right))
            return
        }

        guard let leftIndex = nodeIndex[left], let rightIndex = nodeIndex[right], matrix[leftIndex][rightIndex] != nil else {
            throw EdgeNotFoundError(left: left, right: right)
        }

        matrix[leftIndex][rightIndex] = nil
        matrix[rightIndex][leftIndex] = nil
        edgePropertiesStore.removeValue(forKey: edgeKey(left, right))
    }

    public func hasEdge(_ left: String, _ right: String) -> Bool {
        if repr == .adjacencyList {
            return adj[left]?[right] != nil
        }
        guard let leftIndex = nodeIndex[left], let rightIndex = nodeIndex[right] else {
            return false
        }
        return matrix[leftIndex][rightIndex] != nil
    }

    public func edges() -> [WeightedEdge] {
        var result: [WeightedEdge] = []

        if repr == .adjacencyList {
            var seen = Set<String>()
            for (left, neighbors) in adj {
                for (right, weight) in neighbors {
                    let ordered = canonical(left, right)
                    let key = "\(ordered.0)\u{0}\(ordered.1)"
                    if seen.insert(key).inserted {
                        result.append((ordered.0, ordered.1, weight))
                    }
                }
            }
        } else {
            guard !nodeList.isEmpty else { return [] }
            for row in 0..<nodeList.count {
                for col in row..<nodeList.count {
                    if let weight = matrix[row][col] {
                        result.append((nodeList[row], nodeList[col], weight))
                    }
                }
            }
        }

        return result.sorted { left, right in
            if left.2 != right.2 { return left.2 < right.2 }
            if left.0 != right.0 { return left.0 < right.0 }
            return left.1 < right.1
        }
    }

    public func edgeWeight(_ left: String, _ right: String) throws -> Double {
        if repr == .adjacencyList {
            guard let weight = adj[left]?[right] else {
                throw EdgeNotFoundError(left: left, right: right)
            }
            return weight
        }

        guard let leftIndex = nodeIndex[left], let rightIndex = nodeIndex[right], let weight = matrix[leftIndex][rightIndex] else {
            throw EdgeNotFoundError(left: left, right: right)
        }
        return weight
    }

    public func neighbors(of node: String) throws -> [String] {
        if repr == .adjacencyList {
            guard let neighbors = adj[node] else {
                throw NodeNotFoundError(node: node)
            }
            return neighbors.keys.sorted()
        }

        guard let index = nodeIndex[node] else {
            throw NodeNotFoundError(node: node)
        }
        var result: [String] = []
        for col in 0..<nodeList.count {
            if matrix[index][col] != nil {
                result.append(nodeList[col])
            }
        }
        return result.sorted()
    }

    public func neighborsWeighted(of node: String) throws -> [String: Double] {
        if repr == .adjacencyList {
            guard let neighbors = adj[node] else {
                throw NodeNotFoundError(node: node)
            }
            return neighbors
        }

        guard let index = nodeIndex[node] else {
            throw NodeNotFoundError(node: node)
        }
        var result: [String: Double] = [:]
        for col in 0..<nodeList.count {
            if let weight = matrix[index][col] {
                result[nodeList[col]] = weight
            }
        }
        return result
    }

    public func degree(of node: String) throws -> Int {
        try neighbors(of: node).count
    }

    public func graphProperties() -> GraphPropertyBag {
        graphPropertiesStore
    }

    public mutating func setGraphProperty(_ key: String, value: GraphPropertyValue) {
        graphPropertiesStore[key] = value
    }

    public mutating func removeGraphProperty(_ key: String) {
        graphPropertiesStore.removeValue(forKey: key)
    }

    public func nodeProperties(_ node: String) throws -> GraphPropertyBag {
        guard hasNode(node) else {
            throw NodeNotFoundError(node: node)
        }
        return nodePropertiesStore[node] ?? [:]
    }

    public mutating func setNodeProperty(_ node: String, _ key: String, value: GraphPropertyValue) throws {
        guard hasNode(node) else {
            throw NodeNotFoundError(node: node)
        }
        nodePropertiesStore[node, default: [:]][key] = value
    }

    public mutating func removeNodeProperty(_ node: String, _ key: String) throws {
        guard hasNode(node) else {
            throw NodeNotFoundError(node: node)
        }
        nodePropertiesStore[node]?.removeValue(forKey: key)
    }

    public func edgeProperties(_ left: String, _ right: String) throws -> GraphPropertyBag {
        guard hasEdge(left, right) else {
            throw EdgeNotFoundError(left: left, right: right)
        }
        var properties = edgePropertiesStore[edgeKey(left, right)] ?? [:]
        properties["weight"] = .number(try edgeWeight(left, right))
        return properties
    }

    public mutating func setEdgeProperty(_ left: String, _ right: String, _ key: String, value: GraphPropertyValue) throws {
        guard hasEdge(left, right) else {
            throw EdgeNotFoundError(left: left, right: right)
        }

        if key == "weight" {
            guard case let .number(weight) = value else {
                throw EdgeNotFoundError(left: "weight", right: "numeric property")
            }
            setEdgeWeight(left, right, weight: weight)
        }

        edgePropertiesStore[edgeKey(left, right), default: [:]][key] = value
    }

    public mutating func removeEdgeProperty(_ left: String, _ right: String, _ key: String) throws {
        guard hasEdge(left, right) else {
            throw EdgeNotFoundError(left: left, right: right)
        }

        if key == "weight" {
            setEdgeWeight(left, right, weight: 1.0)
            edgePropertiesStore[edgeKey(left, right), default: [:]]["weight"] = .number(1.0)
            return
        }

        edgePropertiesStore[edgeKey(left, right)]?.removeValue(forKey: key)
    }

    private mutating func mergeNodeProperties(_ node: String, _ properties: GraphPropertyBag) {
        nodePropertiesStore[node, default: [:]].merge(properties) { _, next in next }
    }

    private mutating func mergeEdgeProperties(_ left: String, _ right: String, weight: Double, properties: GraphPropertyBag) {
        var next = edgePropertiesStore[edgeKey(left, right)] ?? [:]
        next.merge(properties) { _, value in value }
        next["weight"] = .number(weight)
        edgePropertiesStore[edgeKey(left, right)] = next
    }

    private mutating func setEdgeWeight(_ left: String, _ right: String, weight: Double) {
        if repr == .adjacencyList {
            adj[left, default: [:]][right] = weight
            adj[right, default: [:]][left] = weight
            return
        }

        let leftIndex = nodeIndex[left]!
        let rightIndex = nodeIndex[right]!
        matrix[leftIndex][rightIndex] = weight
        matrix[rightIndex][leftIndex] = weight
    }
}

public func bfs(_ graph: Graph, start: String) throws -> [String] {
    guard graph.hasNode(start) else { throw NodeNotFoundError(node: start) }
    var queue = [start]
    var visited: Set<String> = [start]
    var result: [String] = []
    var index = 0

    while index < queue.count {
        let node = queue[index]
        index += 1
        result.append(node)
        for neighbor in try graph.neighbors(of: node) where !visited.contains(neighbor) {
            visited.insert(neighbor)
            queue.append(neighbor)
        }
    }

    return result
}

public func dfs(_ graph: Graph, start: String) throws -> [String] {
    guard graph.hasNode(start) else { throw NodeNotFoundError(node: start) }
    var stack = [start]
    var visited = Set<String>()
    var result: [String] = []

    while let node = stack.popLast() {
        if visited.contains(node) { continue }
        visited.insert(node)
        result.append(node)
        for neighbor in try graph.neighbors(of: node).reversed() where !visited.contains(neighbor) {
            stack.append(neighbor)
        }
    }

    return result
}

public func isConnected(_ graph: Graph) -> Bool {
    guard let start = graph.nodes().first else { return true }
    return (try? bfs(graph, start: start).count) == graph.size
}

public func connectedComponents(_ graph: Graph) -> [[String]] {
    var remaining = Set(graph.nodes())
    var result: [[String]] = []

    while let start = remaining.sorted().first {
        let component = (try? bfs(graph, start: start)) ?? []
        result.append(component)
        for node in component {
            remaining.remove(node)
        }
    }

    return result
}

public func hasCycle(_ graph: Graph) -> Bool {
    var visited = Set<String>()
    for start in graph.nodes() where !visited.contains(start) {
        if visitCycle(graph, start, parent: nil, visited: &visited) {
            return true
        }
    }
    return false
}

public func shortestPath(_ graph: Graph, start: String, finish: String) -> [String] {
    guard graph.hasNode(start), graph.hasNode(finish) else { return [] }
    if start == finish { return [start] }

    if graph.edges().allSatisfy({ $0.2 == 1.0 }) {
        return bfsShortestPath(graph, start: start, finish: finish)
    }
    return dijkstraShortestPath(graph, start: start, finish: finish)
}

public func minimumSpanningTree(_ graph: Graph) throws -> [WeightedEdge] {
    if graph.size <= 1 || graph.edges().isEmpty {
        return []
    }
    guard isConnected(graph) else { throw NotConnectedError() }

    var unionFind = UnionFind(nodes: graph.nodes())
    var result: [WeightedEdge] = []

    for edge in graph.edges() {
        if unionFind.find(edge.0) != unionFind.find(edge.1) {
            unionFind.union(edge.0, edge.1)
            result.append(edge)
        }
    }

    return result
}

private func canonical(_ left: String, _ right: String) -> (String, String) {
    left <= right ? (left, right) : (right, left)
}

private func edgeKey(_ left: String, _ right: String) -> String {
    let ordered = canonical(left, right)
    return "\(ordered.0)\u{0}\(ordered.1)"
}

private func visitCycle(_ graph: Graph, _ node: String, parent: String?, visited: inout Set<String>) -> Bool {
    visited.insert(node)
    for neighbor in (try? graph.neighbors(of: node)) ?? [] {
        if !visited.contains(neighbor) {
            if visitCycle(graph, neighbor, parent: node, visited: &visited) {
                return true
            }
        } else if neighbor != parent {
            return true
        }
    }
    return false
}

private func bfsShortestPath(_ graph: Graph, start: String, finish: String) -> [String] {
    var queue = [start]
    var parents: [String: String?] = [start: nil]
    var index = 0

    while index < queue.count {
        let node = queue[index]
        index += 1
        if node == finish { break }

        for neighbor in (try? graph.neighbors(of: node)) ?? [] where parents[neighbor] == nil {
            parents[neighbor] = node
            queue.append(neighbor)
        }
    }

    guard parents.keys.contains(finish) else { return [] }
    return reconstructPath(parents, finish: finish, start: start)
}

private func dijkstraShortestPath(_ graph: Graph, start: String, finish: String) -> [String] {
    var distances = Dictionary(uniqueKeysWithValues: graph.nodes().map { ($0, Double.infinity) })
    distances[start] = 0.0
    var parents: [String: String] = [:]
    var queue: [(Double, String)] = [(0.0, start)]

    while !queue.isEmpty {
        queue.sort { lhs, rhs in lhs.0 == rhs.0 ? lhs.1 < rhs.1 : lhs.0 < rhs.0 }
        let (distance, node) = queue.removeFirst()

        if distance > distances[node, default: .infinity] { continue }
        if node == finish { break }

        let weighted = (try? graph.neighborsWeighted(of: node)) ?? [:]
        for (neighbor, weight) in weighted {
            let next = distance + weight
            if next < distances[neighbor, default: .infinity] {
                distances[neighbor] = next
                parents[neighbor] = node
                queue.append((next, neighbor))
            }
        }
    }

    guard distances[finish, default: .infinity].isFinite else { return [] }
    return reconstructPath(parents.mapValues { Optional($0) }, finish: finish, start: start)
}

private func reconstructPath(_ parents: [String: String?], finish: String, start: String) -> [String] {
    var result: [String] = []
    var current: String? = finish

    while let node = current {
        result.append(node)
        current = parents[node] ?? nil
    }

    result.reverse()
    return result.first == start ? result : []
}

private struct UnionFind {
    var parent: [String: String]
    var rank: [String: Int]

    init(nodes: [String]) {
        self.parent = Dictionary(uniqueKeysWithValues: nodes.map { ($0, $0) })
        self.rank = Dictionary(uniqueKeysWithValues: nodes.map { ($0, 0) })
    }

    mutating func find(_ node: String) -> String {
        let current = parent[node]!
        if current == node { return node }
        let root = find(current)
        parent[node] = root
        return root
    }

    mutating func union(_ left: String, _ right: String) {
        var leftRoot = find(left)
        var rightRoot = find(right)
        if leftRoot == rightRoot { return }

        let leftRank = rank[leftRoot, default: 0]
        let rightRank = rank[rightRoot, default: 0]
        if leftRank < rightRank {
            swap(&leftRoot, &rightRoot)
        }

        parent[rightRoot] = leftRoot
        if leftRank == rightRank {
            rank[leftRoot, default: 0] += 1
        }
    }
}
