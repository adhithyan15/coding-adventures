/// BPlusTree.swift — B+ Tree (DT12) implementation in Swift
/// ==========================================================
///
/// # How does a B+ Tree differ from a B-Tree?
///
/// The B+ Tree is a refinement of the B-Tree with two key structural changes:
///
/// 1. **Internal nodes hold only keys (no values).**
///    All (key, value) pairs live exclusively in the leaf nodes.
///    Internal nodes are purely a routing index.
///
/// 2. **Leaf nodes form a singly-linked list.**
///    Every leaf has a `next` pointer to the next leaf in key order.
///    Range scans walk this linked list without touching internal nodes.
///
/// ASCII diagram — B+ Tree with t=2, 5 entries:
///
///   Internal:       [3]
///                  /    \
///   Leaves: [1,2] ──▶ [3,4,5]
///           ↑↑         ↑↑↑
///        values        values
///
///   Key 3 appears in BOTH the internal node AND the right leaf.
///
/// # Leaf vs Internal nodes
///
///   Leaf:     keys[], values[], next pointer, isLeaf=true, children=[]
///   Internal: keys[], children[], isLeaf=false, values=[], next=nil
///
/// # Split rules
///
/// Leaf split (full leaf has 2t-1 keys):
///   Left  gets keys[0 ..< mid]
///   Right gets keys[mid ...]
///   The FIRST key of right is COPIED up into the parent as separator.
///   (It stays in the leaf — that's the B+ Tree property.)
///   Linked list: left.next = right, right.next = old left.next
///
/// Internal split (same as B-Tree):
///   Median key is MOVED up into the parent.
///
/// # Range scan
///
/// rangeScan(from:to:) walks the leaf linked list.  Two phases:
///   1. Find starting leaf (tree descent, O(log n)).
///   2. Walk linked list collecting keys ≤ high (O(k) for k results).
///
/// # Full scan
///
/// fullScan() starts at firstLeaf and walks the entire chain.  O(n).

// ─────────────────────────────────────────────────────────────────────────────
// BPlusNode
// ─────────────────────────────────────────────────────────────────────────────

final class BPlusNode<K: Comparable, V> {
    var keys: [K]
    var values: [V]           // non-empty only for leaf nodes
    var children: [BPlusNode<K, V>]  // non-empty only for internal nodes
    var isLeaf: Bool
    var next: BPlusNode<K, V>?

    init(isLeaf: Bool) {
        self.keys = []
        self.values = []
        self.children = []
        self.isLeaf = isLeaf
        self.next = nil
    }

    /// Find the first index where keys[i] >= key.
    func findKeyIndex(_ key: K) -> Int {
        var i = 0
        while i < keys.count && keys[i] < key { i += 1 }
        return i
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BPlusTree
// ─────────────────────────────────────────────────────────────────────────────

public final class BPlusTree<K: Comparable, V> {
    private let t: Int
    private var root: BPlusNode<K, V>
    private var firstLeaf: BPlusNode<K, V>
    private var _count: Int = 0

    public init(t: Int = 2) {
        precondition(t >= 2)
        self.t = t
        let leaf = BPlusNode<K, V>(isLeaf: true)
        self.root = leaf
        self.firstLeaf = leaf
    }

    public var count: Int { _count }
    public var height: Int { _height(root) }

    // ── Search ────────────────────────────────────────────────────────────────

    public func search(_ key: K) -> V? {
        let leaf = _findLeaf(key)
        let i = leaf.findKeyIndex(key)
        return (i < leaf.keys.count && leaf.keys[i] == key) ? leaf.values[i] : nil
    }

    public func contains(_ key: K) -> Bool { search(key) != nil }

    // ── Insert ────────────────────────────────────────────────────────────────

    public func insert(_ key: K, _ value: V) {
        if root.keys.count == 2 * t - 1 {
            let newRoot = BPlusNode<K, V>(isLeaf: false)
            newRoot.children.append(root)
            _splitChild(parent: newRoot, childIndex: 0)
            root = newRoot
        }
        let inserted = _insertNonFull(node: root, key: key, value: value)
        if inserted { _count += 1 }
        // Recompute firstLeaf in case a left split created a new leftmost leaf.
        firstLeaf = _leftmostLeaf(root)
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    @discardableResult
    public func delete(_ key: K) -> Bool {
        guard _count > 0 else { return false }
        let found = _delete(node: root, key: key)
        if found {
            _count -= 1
            if root.keys.isEmpty && !root.isLeaf { root = root.children[0] }
        }
        firstLeaf = _leftmostLeaf(root)
        return found
    }

    // ── Range / traversal ────────────────────────────────────────────────────

    public func minKey() -> K? { firstLeaf.keys.first }

    public func maxKey() -> K? {
        var leaf = firstLeaf
        while let next = leaf.next { leaf = next }
        return leaf.keys.last
    }

    /// Range scan using the leaf linked list.
    public func rangeScan(from low: K, to high: K) -> [(K, V)] {
        var result: [(K, V)] = []
        var leaf: BPlusNode<K, V>? = _findLeaf(low)
        while let l = leaf {
            for i in 0..<l.keys.count {
                if l.keys[i] > high { return result }
                if l.keys[i] >= low { result.append((l.keys[i], l.values[i])) }
            }
            leaf = l.next
        }
        return result
    }

    /// Full scan by walking the leaf linked list.
    public func fullScan() -> [(K, V)] {
        var result: [(K, V)] = []
        var leaf: BPlusNode<K, V>? = firstLeaf
        while let l = leaf {
            for i in 0..<l.keys.count { result.append((l.keys[i], l.values[i])) }
            leaf = l.next
        }
        return result
    }

    public func inorder() -> [(K, V)] { fullScan() }

    // ── Validation ────────────────────────────────────────────────────────────

    public func isValid() -> Bool {
        guard _isStructurallyValid(root, minKey: nil, maxKey: nil,
                                   expectedDepth: height, depth: 0, isRoot: true)
        else { return false }
        return _isLinkedListValid()
    }

    // =========================================================================
    // MARK: — Private implementation
    // =========================================================================

    private func _height(_ node: BPlusNode<K, V>) -> Int {
        node.isLeaf ? 0 : 1 + _height(node.children[0])
    }

    private func _leftmostLeaf(_ node: BPlusNode<K, V>) -> BPlusNode<K, V> {
        var n = node
        while !n.isLeaf { n = n.children[0] }
        return n
    }

    private func _findLeaf(_ key: K) -> BPlusNode<K, V> {
        var node = root
        while !node.isLeaf {
            // In a B+ Tree internal node, keys[i] is the minimum key in children[i+1].
            // So we go right as long as key >= keys[i].
            var i = 0
            while i < node.keys.count && key >= node.keys[i] { i += 1 }
            node = node.children[i]
        }
        return node
    }

    // ── Split ─────────────────────────────────────────────────────────────────

    private func _splitChild(parent: BPlusNode<K, V>, childIndex i: Int) {
        let child = parent.children[i]
        if child.isLeaf {
            _splitLeaf(parent: parent, leafIndex: i)
        } else {
            _splitInternal(parent: parent, internalIndex: i)
        }
    }

    private func _splitLeaf(parent: BPlusNode<K, V>, leafIndex i: Int) {
        let left = parent.children[i]
        let right = BPlusNode<K, V>(isLeaf: true)

        // Split at midpoint: left gets first half, right gets second half.
        let mid = left.keys.count / 2

        right.keys = Array(left.keys[mid...])
        right.values = Array(left.values[mid...])
        left.keys = Array(left.keys[..<mid])
        left.values = Array(left.values[..<mid])

        // Fix linked list.
        right.next = left.next
        left.next = right

        // The separator pushed into parent is the FIRST key of the right leaf.
        // It stays in the right leaf (B+ Tree property).
        let separator = right.keys[0]
        parent.keys.insert(separator, at: i)
        parent.children.insert(right, at: i + 1)
    }

    private func _splitInternal(parent: BPlusNode<K, V>, internalIndex i: Int) {
        let full = parent.children[i]
        let right = BPlusNode<K, V>(isLeaf: false)

        // Median key MOVES up into parent (same as B-Tree).
        let medianKey = full.keys[t - 1]

        right.keys = Array(full.keys[t...])
        right.children = Array(full.children[t...])
        full.keys = Array(full.keys[..<(t - 1)])
        full.children = Array(full.children[..<t])

        parent.keys.insert(medianKey, at: i)
        parent.children.insert(right, at: i + 1)
    }

    // ── Insert helper ─────────────────────────────────────────────────────────

    @discardableResult
    private func _insertNonFull(node: BPlusNode<K, V>, key: K, value: V) -> Bool {
        if node.isLeaf {
            let i = node.findKeyIndex(key)
            if i < node.keys.count && node.keys[i] == key {
                node.values[i] = value
                return false
            }
            node.keys.insert(key, at: i)
            node.values.insert(value, at: i)
            return true
        }

        // Find which child to descend into.
        var ci = 0
        while ci < node.keys.count && key >= node.keys[ci] { ci += 1 }

        // Pre-emptively split full child.
        if node.children[ci].keys.count == 2 * t - 1 {
            _splitChild(parent: node, childIndex: ci)
            // After split, decide which half to enter.
            // node.keys[ci] is the new separator.
            if ci < node.keys.count && key >= node.keys[ci] {
                ci += 1
            }
        }

        let safeCI = min(ci, node.children.count - 1)
        return _insertNonFull(node: node.children[safeCI], key: key, value: value)
    }

    // ── Delete ────────────────────────────────────────────────────────────────
    //
    // B+ Tree deletion:
    //   1. Walk down to the leaf containing the key.
    //   2. Before descending into any child, ensure it has ≥ t keys.
    //   3. Remove the key from the leaf.
    //   4. After removal, update internal separators if needed.
    //
    // We do NOT need to handle Cases 2a/2b/2c like B-Tree because values only
    // live in leaves.  Internal nodes just need their separators updated.

    /// Delete `key`.  Returns true if found.
    private func _delete(node: BPlusNode<K, V>, key: K) -> Bool {
        if node.isLeaf {
            let i = node.findKeyIndex(key)
            if i < node.keys.count && node.keys[i] == key {
                node.keys.remove(at: i)
                node.values.remove(at: i)
                return true
            }
            return false
        }

        // Find which child to descend into.
        var ci = 0
        while ci < node.keys.count && key >= node.keys[ci] { ci += 1 }
        ci = min(ci, node.children.count - 1)

        // Ensure child[ci] has ≥ t keys before descending.
        let newCI = _prepareChild(parent: node, index: ci)
        let safeCI = min(newCI, node.children.count - 1)

        return _delete(node: node.children[safeCI], key: key)
    }

    /// Ensure node.children[i] has ≥ t keys.
    /// Returns the (possibly shifted) index of the child.
    @discardableResult
    private func _prepareChild(parent: BPlusNode<K, V>, index i: Int) -> Int {
        let child = parent.children[i]
        guard child.keys.count < t else { return i }

        let hasLeft  = i > 0
        let hasRight = i < parent.children.count - 1

        if hasLeft && parent.children[i - 1].keys.count >= t {
            _borrowFromLeft(parent: parent, childIndex: i)
            return i
        } else if hasRight && parent.children[i + 1].keys.count >= t {
            _borrowFromRight(parent: parent, childIndex: i)
            return i
        } else if hasLeft {
            _mergeChildren(parent: parent, index: i - 1)
            return i - 1
        } else {
            _mergeChildren(parent: parent, index: i)
            return i
        }
    }

    private func _borrowFromLeft(parent: BPlusNode<K, V>, childIndex i: Int) {
        let child = parent.children[i]
        let left  = parent.children[i - 1]

        if child.isLeaf {
            // Move last key/value from left leaf to front of child leaf.
            let borrowedKey = left.keys.removeLast()
            let borrowedVal = left.values.removeLast()
            child.keys.insert(borrowedKey, at: 0)
            child.values.insert(borrowedVal, at: 0)
            // Update separator in parent to the new first key of child.
            parent.keys[i - 1] = child.keys[0]
        } else {
            // Internal borrow: rotate through parent.
            child.keys.insert(parent.keys[i - 1], at: 0)
            child.children.insert(left.children.removeLast(), at: 0)
            parent.keys[i - 1] = left.keys.removeLast()
        }
    }

    private func _borrowFromRight(parent: BPlusNode<K, V>, childIndex i: Int) {
        let child = parent.children[i]
        let right = parent.children[i + 1]

        if child.isLeaf {
            let borrowedKey = right.keys.removeFirst()
            let borrowedVal = right.values.removeFirst()
            child.keys.append(borrowedKey)
            child.values.append(borrowedVal)
            // Update separator in parent to new first key of right.
            parent.keys[i] = right.keys[0]
        } else {
            child.keys.append(parent.keys[i])
            child.children.append(right.children.removeFirst())
            parent.keys[i] = right.keys.removeFirst()
        }
    }

    private func _mergeChildren(parent: BPlusNode<K, V>, index i: Int) {
        let left  = parent.children[i]
        let right = parent.children[i + 1]

        if left.isLeaf {
            // Leaf merge: absorb right into left, fix linked list.
            left.keys.append(contentsOf: right.keys)
            left.values.append(contentsOf: right.values)
            left.next = right.next
        } else {
            // Internal merge: pull separator key down.
            left.keys.append(parent.keys[i])
            left.keys.append(contentsOf: right.keys)
            left.children.append(contentsOf: right.children)
        }

        parent.keys.remove(at: i)
        parent.children.remove(at: i + 1)
    }

    /// Update the separator keys in an internal node so they match the actual
    /// minimum key of each right subtree  (children[1], children[2], …).
    private func _updateSeparators(node: BPlusNode<K, V>) {
        guard !node.isLeaf else { return }
        for i in 0..<node.keys.count {
            if i + 1 < node.children.count {
                if let mk = _minKey(node.children[i + 1]) {
                    node.keys[i] = mk
                }
            }
        }
    }

    private func _minKey(_ node: BPlusNode<K, V>) -> K? {
        var n = node
        while !n.isLeaf {
            if n.children.isEmpty { return nil }
            n = n.children[0]
        }
        return n.keys.first
    }

    // ── Validation helpers ────────────────────────────────────────────────────

    private func _isStructurallyValid(
        _ node: BPlusNode<K, V>,
        minKey: K?,
        maxKey: K?,
        expectedDepth: Int,
        depth: Int,
        isRoot: Bool
    ) -> Bool {
        if node.isLeaf {
            if depth != expectedDepth { return false }
            if node.keys.count != node.values.count { return false }
            // Leaf keys must be sorted.
            if node.keys.count > 1 {
                for i in 1..<node.keys.count {
                    if node.keys[i] <= node.keys[i - 1] { return false }
                }
            }
        } else {
            if !node.values.isEmpty { return false }
            if node.children.count != node.keys.count + 1 { return false }
            // Internal node separators must be sorted.
            if node.keys.count > 1 {
                for i in 1..<node.keys.count {
                    if node.keys[i] <= node.keys[i - 1] { return false }
                }
            }
        }

        let minKeys = isRoot ? 0 : (t - 1)
        if node.keys.count < minKeys || node.keys.count > 2 * t - 1 { return false }

        // Check key count bounds (structural only, not value bounds for internal nodes
        // since B+ tree separators may be stale after deletion).
        if node.isLeaf {
            for k in node.keys {
                if let lo = minKey, k < lo { return false }
                if let hi = maxKey, k > hi { return false }
            }
        }

        if !node.isLeaf {
            // Recursively validate children using separator-based bounds.
            for i in 0...node.keys.count {
                let cMin: K? = i == 0 ? minKey : Optional(node.keys[i - 1])
                let cMax: K? = i == node.keys.count ? maxKey : Optional(node.keys[i])
                if !_isStructurallyValid(node.children[i], minKey: cMin, maxKey: cMax,
                                         expectedDepth: expectedDepth, depth: depth + 1,
                                         isRoot: false) { return false }
            }
        }
        return true
    }

    private func _isLinkedListValid() -> Bool {
        var total = 0
        var leaf: BPlusNode<K, V>? = firstLeaf
        var prevLast: K? = nil
        while let l = leaf {
            for i in 0..<l.keys.count {
                if i > 0 && l.keys[i] <= l.keys[i - 1] { return false }
                if let prev = prevLast, l.keys[i] <= prev { return false }
                prevLast = l.keys[i]
                total += 1
            }
            leaf = l.next
        }
        return total == _count
    }
}
