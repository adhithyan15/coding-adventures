// ============================================================================
// BinarySearchTree.swift — an immutable (persistent) binary search tree
// ============================================================================
//
// A binary search tree keeps its elements ordered: every value in a node's left
// subtree is smaller, every value on the right is larger. That invariant makes
// search, insert, and delete O(h) — O(log n) when the tree is balanced — and
// an in-order walk yields the elements sorted.
//
// This `BST` is **immutable**: `insert` and `delete` return a *new* tree and
// leave the original untouched, matching the reference `binary-search-tree`
// package. Under the hood the tree is an `indirect enum` (a value type), so
// unchanged subtrees are shared structurally between old and new versions — the
// same observable behaviour as the reference's clone-on-write, done cheaply.
//
// Each node caches its subtree `size`, which powers the order-statistic queries
// `kthSmallest(_:)` and `rank(_:)` in O(h).

/// An immutable binary search tree of `Comparable` elements.
public struct BST<Element: Comparable> {

    // A node is either empty or an interior node caching its subtree size.
    private indirect enum Node {
        case empty
        case node(value: Element, left: Node, right: Node, size: Int)

        var size: Int {
            if case let .node(_, _, _, s) = self { return s }
            return 0
        }
    }

    private let root: Node

    private init(_ root: Node) { self.root = root }

    /// Create an empty tree.
    public init() { self.root = .empty }

    /// Build a balanced tree from an already-sorted array (the middle element
    /// becomes each subtree's root).
    public static func fromSorted(_ array: [Element]) -> BST {
        BST(buildBalanced(ArraySlice(array)))
    }

    // ── Immutable mutation ───────────────────────────────────────────────────

    /// Return a new tree with `value` inserted (a no-op if already present).
    public func insert(_ value: Element) -> BST { BST(Self.insert(root, value)) }

    /// Return a new tree with `value` removed (a no-op if absent).
    public func delete(_ value: Element) -> BST { BST(Self.delete(root, value)) }

    // ── Lookup ───────────────────────────────────────────────────────────────

    /// The stored element equal to `value`, or `nil`.
    public func search(_ value: Element) -> Element? {
        var current = root
        while case let .node(v, l, r, _) = current {
            if value < v { current = l }
            else if value > v { current = r }
            else { return v }
        }
        return nil
    }

    /// Whether `value` is present.
    public func contains(_ value: Element) -> Bool { search(value) != nil }

    /// The smallest element, or `nil` when empty.
    public func minValue() -> Element? {
        var current = root
        var result: Element?
        while case let .node(v, l, _, _) = current {
            result = v
            current = l
        }
        return result
    }

    /// The largest element, or `nil` when empty.
    public func maxValue() -> Element? {
        var current = root
        var result: Element?
        while case let .node(v, _, r, _) = current {
            result = v
            current = r
        }
        return result
    }

    /// The largest stored element strictly less than `value`, or `nil`.
    public func predecessor(_ value: Element) -> Element? {
        var current = root
        var best: Element?
        while case let .node(v, l, r, _) = current {
            if v < value {
                best = v
                current = r
            } else {
                current = l
            }
        }
        return best
    }

    /// The smallest stored element strictly greater than `value`, or `nil`.
    public func successor(_ value: Element) -> Element? {
        var current = root
        var best: Element?
        while case let .node(v, l, r, _) = current {
            if v > value {
                best = v
                current = l
            } else {
                current = r
            }
        }
        return best
    }

    /// The `k`-th smallest element (1-based), or `nil` if out of range.
    public func kthSmallest(_ k: Int) -> Element? {
        guard k >= 1 else { return nil }
        var current = root
        var target = k
        while case let .node(v, l, r, _) = current {
            let leftSize = l.size
            if target == leftSize + 1 { return v }
            if target <= leftSize { current = l }
            else { current = r; target -= leftSize + 1 }
        }
        return nil
    }

    /// The number of stored elements strictly less than `value`.
    public func rank(_ value: Element) -> Int {
        var current = root
        var acc = 0
        while case let .node(v, l, r, _) = current {
            if value < v { current = l }
            else if value > v { acc += l.size + 1; current = r }
            else { return acc + l.size }
        }
        return acc
    }

    // ── Whole-tree views ─────────────────────────────────────────────────────

    /// The elements in ascending order.
    public func toSortedArray() -> [Element] {
        var out: [Element] = []
        Self.inorder(root, &out)
        return out
    }

    /// Whether the tree satisfies the BST ordering invariant.
    public func isValid() -> Bool { Self.validate(root, min: nil, max: nil) }

    /// The height of the tree: `-1` for an empty tree, `0` for a single node.
    public var height: Int { Self.height(root) }

    /// The number of stored elements.
    public var count: Int { root.size }

    /// Whether the tree stores no elements.
    public var isEmpty: Bool { count == 0 }

    // ── Internal recursive machinery ─────────────────────────────────────────

    private static func make(_ value: Element, _ left: Node, _ right: Node) -> Node {
        .node(value: value, left: left, right: right, size: 1 + left.size + right.size)
    }

    private static func insert(_ node: Node, _ value: Element) -> Node {
        switch node {
        case .empty:
            return .node(value: value, left: .empty, right: .empty, size: 1)
        case let .node(v, l, r, _):
            if value < v { return make(v, insert(l, value), r) }
            if value > v { return make(v, l, insert(r, value)) }
            return node // equal: unchanged
        }
    }

    private static func delete(_ node: Node, _ value: Element) -> Node {
        guard case let .node(v, l, r, _) = node else { return .empty }
        if value < v { return make(v, delete(l, value), r) }
        if value > v { return make(v, l, delete(r, value)) }
        // Found `v`. Replace by successor (min of right subtree) when needed.
        switch (l, r) {
        case (.empty, .empty): return .empty
        case (_, .empty): return l
        case (.empty, _): return r
        default:
            let (newRight, successor) = extractMin(r)
            return make(successor, l, newRight)
        }
    }

    /// Remove and return the smallest value of a non-empty subtree.
    private static func extractMin(_ node: Node) -> (Node, Element) {
        guard case let .node(v, l, r, _) = node else {
            preconditionFailure("extractMin on empty node")
        }
        if case .empty = l { return (r, v) }
        let (newLeft, minValue) = extractMin(l)
        return (make(v, newLeft, r), minValue)
    }

    private static func inorder(_ node: Node, _ out: inout [Element]) {
        guard case let .node(v, l, r, _) = node else { return }
        inorder(l, &out)
        out.append(v)
        inorder(r, &out)
    }

    private static func height(_ node: Node) -> Int {
        guard case let .node(_, l, r, _) = node else { return -1 }
        return 1 + Swift.max(height(l), height(r))
    }

    private static func validate(_ node: Node, min: Element?, max: Element?) -> Bool {
        guard case let .node(v, l, r, _) = node else { return true }
        if let lo = min, v <= lo { return false }
        if let hi = max, v >= hi { return false }
        return validate(l, min: min, max: v) && validate(r, min: v, max: max)
    }

    private static func buildBalanced(_ values: ArraySlice<Element>) -> Node {
        guard !values.isEmpty else { return .empty }
        let mid = values.startIndex + values.count / 2
        let left = buildBalanced(values[values.startIndex..<mid])
        let right = buildBalanced(values[(mid + 1)..<values.endIndex])
        return make(values[mid], left, right)
    }
}

extension BST: CustomStringConvertible {
    public var description: String { "BST(count=\(count), sorted=\(toSortedArray()))" }
}
