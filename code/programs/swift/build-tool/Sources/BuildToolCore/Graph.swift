import Foundation

public final class DirectedGraph {
    private var forward: [String: Set<String>] = [:]
    private var reverse: [String: Set<String>] = [:]

    public init() {}

    public func addNode(_ node: String) {
        if forward[node] == nil {
            forward[node] = []
        }
        if reverse[node] == nil {
            reverse[node] = []
        }
    }

    public func addEdge(from fromNode: String, to toNode: String) {
        addNode(fromNode)
        addNode(toNode)
        forward[fromNode, default: []].insert(toNode)
        reverse[toNode, default: []].insert(fromNode)
    }

    public func hasNode(_ node: String) -> Bool {
        forward[node] != nil
    }

    public func nodes() -> [String] {
        forward.keys.sorted()
    }

    public func successors(of node: String) -> [String] {
        Array(forward[node] ?? []).sorted()
    }

    public func predecessors(of node: String) -> [String] {
        Array(reverse[node] ?? []).sorted()
    }

    public func edges() -> [(String, String)] {
        var result: [(String, String)] = []
        for node in forward.keys.sorted() {
            for successor in (forward[node] ?? []).sorted() {
                result.append((node, successor))
            }
        }
        return result
    }

    public func transitiveClosure(from node: String) -> Set<String> {
        guard let initial = forward[node] else {
            return []
        }

        var visited = initial
        var stack = Array(initial)

        while let current = stack.popLast() {
            for successor in forward[current] ?? [] where !visited.contains(successor) {
                visited.insert(successor)
                stack.append(successor)
            }
        }

        return visited
    }

    public func transitivePrerequisites(of node: String) -> Set<String> {
        guard let initial = reverse[node] else {
            return []
        }

        var visited = initial
        var stack = Array(initial)

        while let current = stack.popLast() {
            for predecessor in reverse[current] ?? [] where !visited.contains(predecessor) {
                visited.insert(predecessor)
                stack.append(predecessor)
            }
        }

        return visited
    }

    public func affectedNodes(changed: Set<String>) -> Set<String> {
        var result = changed
        for node in changed {
            result.formUnion(transitiveClosure(from: node))
        }
        return result
    }

    public func independentGroups() throws -> [[String]] {
        var inDegree: [String: Int] = [:]
        for node in forward.keys {
            inDegree[node] = reverse[node]?.count ?? 0
        }

        var currentLevel = inDegree
            .filter { $0.value == 0 }
            .map(\.key)
            .sorted()

        var groups: [[String]] = []
        var processed = 0

        while !currentLevel.isEmpty {
            groups.append(currentLevel)
            processed += currentLevel.count
            var nextLevel = Set<String>()

            for node in currentLevel {
                for successor in forward[node] ?? [] {
                    let nextDegree = (inDegree[successor] ?? 0) - 1
                    inDegree[successor] = nextDegree
                    if nextDegree == 0 {
                        nextLevel.insert(successor)
                    }
                }
            }

            currentLevel = nextLevel.sorted()
        }

        if processed != forward.count {
            throw BuildToolError.cycleDetected
        }

        return groups
    }
}
