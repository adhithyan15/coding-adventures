import DirectedGraph

/// A rooted tree backed by a DirectedGraph.
public class Tree {
    private var _graph: Graph
    private let _root: String
    
    public init(root: String) {
        self._graph = Graph()
        self._graph.addNode(root)
        self._root = root
    }
    
    public var root: String { _root }
    
    public func addChild(parent: String, child: String) throws {
        guard _graph.hasNode(parent) else { throw TreeError.nodeNotFound(parent) }
        guard !_graph.hasNode(child) else { throw TreeError.duplicateNode(child) }
        try _graph.addEdge(from: parent, to: child)
    }
    
    public func removeSubtree(node: String) throws {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        guard node != _root else { throw TreeError.rootRemoval }
        
        let toRemove = _collectSubtreeNodes(node: node)
        for n in toRemove.reversed() {
            try _graph.removeNode(n)
        }
    }
    
    private func _collectSubtreeNodes(node: String) -> [String] {
        var result: [String] = []
        var queue = [node]
        var head = 0
        
        while head < queue.count {
            let current = queue[head]
            head += 1
            result.append(current)
            
            let successors = (try? _graph.successors(of: current)) ?? []
            for child in successors.sorted() {
                queue.append(child)
            }
        }
        return result
    }
    
    public func parent(of node: String) throws -> String? {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        let preds = (try? _graph.predecessors(of: node)) ?? []
        return preds.isEmpty ? nil : preds[0]
    }
    
    public func children(of node: String) throws -> [String] {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        return (try? _graph.successors(of: node))?.sorted() ?? []
    }
    
    public func siblings(of node: String) throws -> [String] {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        guard let p = try parent(of: node) else { return [] }
        let c = try children(of: p)
        return c.filter { $0 != node }
    }
    
    public func isLeaf(_ node: String) throws -> Bool {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        return (try children(of: node)).isEmpty
    }
    
    public func isRoot(_ node: String) throws -> Bool {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        return node == _root
    }
    
    public func depth(of node: String) throws -> Int {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        var d = 0
        var current = node
        while current != _root {
            guard let p = try parent(of: current) else { break }
            current = p
            d += 1
        }
        return d
    }
    
    public func height() -> Int {
        var maxDepth = 0
        var queue: [(String, Int)] = [(_root, 0)]
        var head = 0
        while head < queue.count {
            let (current, d) = queue[head]
            head += 1
            if d > maxDepth { maxDepth = d }
            let successors = (try? _graph.successors(of: current)) ?? []
            for child in successors.sorted() {
                queue.append((child, d + 1))
            }
        }
        return maxDepth
    }
    
    public var size: Int { _graph.size }
    
    public func nodes() -> [String] { _graph.nodes() }
    
    public func leaves() -> [String] {
        _graph.nodes().filter { (try? _graph.successors(of: $0))?.isEmpty ?? false }.sorted()
    }
    
    public func hasNode(_ node: String) -> Bool { _graph.hasNode(node) }
    
    // Traversals
    public func preorder() -> [String] {
        var result: [String] = []
        var stack = [_root]
        while let node = stack.popLast() {
            result.append(node)
            let successors = (try? _graph.successors(of: node)) ?? []
            stack.append(contentsOf: successors.sorted().reversed())
        }
        return result
    }
    
    public func postorder() -> [String] {
        var result: [String] = []
        _postorderRecursive(node: _root, result: &result)
        return result
    }
    
    private func _postorderRecursive(node: String, result: inout [String]) {
        let successors = (try? _graph.successors(of: node)) ?? []
        for child in successors.sorted() {
            _postorderRecursive(node: child, result: &result)
        }
        result.append(node)
    }
    
    public func levelOrder() -> [String] {
        var result: [String] = []
        var queue = [_root]
        var head = 0
        while head < queue.count {
            let node = queue[head]
            head += 1
            result.append(node)
            let successors = (try? _graph.successors(of: node)) ?? []
            queue.append(contentsOf: successors.sorted())
        }
        return result
    }
    
    public func pathTo(node: String) throws -> [String] {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        var path: [String] = []
        var current: String? = node
        while let curr = current {
            path.append(curr)
            current = try parent(of: curr)
        }
        return path.reversed()
    }
    
    public func lca(a: String, b: String) throws -> String {
        guard _graph.hasNode(a) else { throw TreeError.nodeNotFound(a) }
        guard _graph.hasNode(b) else { throw TreeError.nodeNotFound(b) }
        
        let pathA = try pathTo(node: a)
        let pathB = try pathTo(node: b)
        
        var lcaNode = _root
        for (na, nb) in zip(pathA, pathB) {
            if na == nb {
                lcaNode = na
            } else {
                break
            }
        }
        return lcaNode
    }
    
    public func subtree(of node: String) throws -> Tree {
        guard _graph.hasNode(node) else { throw TreeError.nodeNotFound(node) }
        let newTree = Tree(root: node)
        var queue = [node]
        var head = 0
        while head < queue.count {
            let current = queue[head]
            head += 1
            let successors = (try? _graph.successors(of: current)) ?? []
            for child in successors.sorted() {
                try? newTree.addChild(parent: current, child: child)
                queue.append(child)
            }
        }
        return newTree
    }
    
    public func toAscii() -> String {
        var lines: [String] = []
        _asciiRecursive(node: _root, prefix: "", childPrefix: "", lines: &lines)
        return lines.joined(separator: "\n")
    }
    
    private func _asciiRecursive(node: String, prefix: String, childPrefix: String, lines: inout [String]) {
        lines.append(prefix + node)
        let children = (try? _graph.successors(of: node))?.sorted() ?? []
        for (i, child) in children.enumerated() {
            if i < children.count - 1 {
                _asciiRecursive(node: child, prefix: childPrefix + "├── ", childPrefix: childPrefix + "│   ", lines: &lines)
            } else {
                _asciiRecursive(node: child, prefix: childPrefix + "└── ", childPrefix: childPrefix + "    ", lines: &lines)
            }
        }
    }
    
    public var graph: Graph { _graph }
}

extension Tree: CustomStringConvertible {
    public var description: String {
        return "Tree(root=\(_root), size=\(size))"
    }
}
