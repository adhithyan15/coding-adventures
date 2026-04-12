/// BTree.swift — B-Tree (DT11) implementation in Swift
/// ======================================================
///
/// # What is a B-Tree?
///
/// A B-Tree is a self-balancing search tree designed to work efficiently on
/// disk-based storage systems.  Unlike a binary search tree (which has at most
/// 2 children per node), a B-Tree node can hold many keys and many children.
/// This "wide and shallow" shape keeps the height tiny — even a tree with a
/// billion entries typically has a height of just 3 or 4.
///
/// Invented by Rudolf Bayer and Edward McCreight at Boeing Research Labs in 1970,
/// B-Trees are the foundation of almost every database and filesystem you will
/// ever use: SQLite, PostgreSQL, MySQL InnoDB, NTFS, HFS+, ext4 all rely on
/// B-Tree variants under the hood.
///
/// # The minimum-degree parameter t
///
/// Every B-Tree is parameterised by an integer t ≥ 2 called the **minimum degree**.
/// It governs how fat each node is:
///
///   • A non-root node must hold at least t-1 keys  (so it is "at least half full")
///   • Every node holds at most 2t-1 keys            (so it never overflows)
///   • A non-leaf node with k keys has exactly k+1 children
///
/// When t = 2 we get a **2-3-4 tree**: nodes hold 1, 2, or 3 keys.
///
/// # Invariants — the rules the tree must NEVER break
///
/// 1. All leaf nodes sit at the same depth  (the tree is perfectly balanced).
/// 2. Every node's keys are in strictly ascending order.
/// 3. Every non-root node has at least t-1 keys.
/// 4. Every node has at most 2t-1 keys.
/// 5. For an internal node with keys [k0, k1, …, km], the i-th child subtree
///    contains only keys x such that k_{i-1} < x < k_i.
///
/// # Insertion — proactive top-down splitting
///
/// We use the "single-pass" algorithm: as we walk down the tree searching for
/// the insertion point we pre-emptively split any node that is already full
/// (has 2t-1 keys).  This means we never have to backtrack upwards.
///
/// # Deletion — CLRS top-down approach
///
/// We use the CLRS (Introduction to Algorithms) top-down deletion algorithm.
/// Before recursing into any child C we ensure C has at least t keys (one more
/// than the minimum), so that if we need to delete a key from C, C can afford
/// to lose one key and still satisfy the invariant.
///
/// The key insight: we call `_deleteFromNode` only on nodes that already have
/// ≥ t keys (guaranteed by the caller), except for the root (which can have
/// as few as 1 key).
///
/// ASCII diagram — splitting a full node when t=2:
///
///        P: [10, 30]                      P: [10, 20, 30]
///           /   |   \          →              /  |   |   \
///          A  [15,20,25]  B               A  [15] [25]  B
///                                  (median 20 rises into P)

// ─────────────────────────────────────────────────────────────────────────────
// BTreeNode — the building block
// ─────────────────────────────────────────────────────────────────────────────

/// A single node in a B-Tree.
final class BTreeNode<K: Comparable, V> {
    var keys: [K]
    var values: [V]
    var children: [BTreeNode<K, V>]
    var isLeaf: Bool

    init(isLeaf: Bool) {
        self.keys = []
        self.values = []
        self.children = []
        self.isLeaf = isLeaf
    }

    /// Find the first index i where keys[i] >= key.
    func findKeyIndex(_ key: K) -> Int {
        var i = 0
        while i < keys.count && keys[i] < key { i += 1 }
        return i
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BTree — the public API
// ─────────────────────────────────────────────────────────────────────────────

/// A generic B-Tree that maps keys of type K to values of type V.
///
/// Usage example:
/// ```swift
/// var tree = BTree<Int, String>()
/// tree.insert(10, "ten")
/// tree.insert(20, "twenty")
/// let v = tree.search(10)   // → "ten"
/// let r = tree.rangeQuery(from: 5, to: 15)  // → [(10, "ten")]
/// ```
public final class BTree<K: Comparable, V> {
    private let t: Int
    private var root: BTreeNode<K, V>
    private var _count: Int = 0

    /// Create an empty B-Tree with the given minimum degree.
    /// - Parameter t: Minimum degree (default 2).  Must be ≥ 2.
    public init(t: Int = 2) {
        precondition(t >= 2, "Minimum degree t must be at least 2")
        self.t = t
        self.root = BTreeNode(isLeaf: true)
    }

    // ── Public properties ────────────────────────────────────────────────────

    public var count: Int { _count }
    public var height: Int { _height(node: root) }

    // ── Search ────────────────────────────────────────────────────────────────

    /// Return the value for `key`, or `nil` if the key is not present.
    public func search(_ key: K) -> V? {
        return _search(node: root, key: key)
    }

    /// Return true if `key` is present in the tree.
    public func contains(_ key: K) -> Bool {
        return search(key) != nil
    }

    // ── Insert ────────────────────────────────────────────────────────────────

    /// Insert or update the mapping key → value  (upsert semantics).
    public func insert(_ key: K, _ value: V) {
        if root.keys.count == 2 * t - 1 {
            let newRoot = BTreeNode<K, V>(isLeaf: false)
            newRoot.children.append(root)
            _splitChild(parent: newRoot, childIndex: 0)
            root = newRoot
        }
        let didInsert = _insertNonFull(node: root, key: key, value: value)
        if didInsert { _count += 1 }
    }

    // ── Delete ────────────────────────────────────────────────────────────────

    /// Remove the key from the tree.
    /// - Returns: `true` if the key was found and removed.
    @discardableResult
    public func delete(_ key: K) -> Bool {
        guard _count > 0 else { return false }
        let found = _delete(node: root, key: key)
        if found {
            _count -= 1
            if root.keys.isEmpty && !root.isLeaf {
                root = root.children[0]
            }
        }
        return found
    }

    // ── Range / traversal ────────────────────────────────────────────────────

    public func minKey() -> K? {
        guard !root.keys.isEmpty else { return nil }
        var node = root
        while !node.isLeaf { node = node.children[0] }
        return node.keys[0]
    }

    public func maxKey() -> K? {
        guard !root.keys.isEmpty else { return nil }
        var node = root
        while !node.isLeaf { node = node.children[node.children.count - 1] }
        return node.keys[node.keys.count - 1]
    }

    public func rangeQuery(from low: K, to high: K) -> [(K, V)] {
        var result: [(K, V)] = []
        _rangeQuery(node: root, low: low, high: high, result: &result)
        return result
    }

    public func inorder() -> [(K, V)] {
        var result: [(K, V)] = []
        _inorder(node: root, result: &result)
        return result
    }

    // ── Validation ────────────────────────────────────────────────────────────

    public func isValid() -> Bool {
        return _isValid(node: root, minKey: nil, maxKey: nil,
                        expectedDepth: height, currentDepth: 0, isRoot: true)
    }

    // =========================================================================
    // MARK: — Private implementation
    // =========================================================================

    // ── Search helper ─────────────────────────────────────────────────────────

    private func _search(node: BTreeNode<K, V>, key: K) -> V? {
        let i = node.findKeyIndex(key)
        if i < node.keys.count && node.keys[i] == key { return node.values[i] }
        if node.isLeaf { return nil }
        return _search(node: node.children[i], key: key)
    }

    // ── Split helper ──────────────────────────────────────────────────────────

    /// Split the i-th child of `parent` (which must be full — 2t-1 keys).
    private func _splitChild(parent: BTreeNode<K, V>, childIndex i: Int) {
        let fullChild = parent.children[i]
        let newChild = BTreeNode<K, V>(isLeaf: fullChild.isLeaf)

        let medianKey = fullChild.keys[t - 1]
        let medianValue = fullChild.values[t - 1]

        newChild.keys = Array(fullChild.keys[t...])
        newChild.values = Array(fullChild.values[t...])

        if !fullChild.isLeaf {
            newChild.children = Array(fullChild.children[t...])
            fullChild.children = Array(fullChild.children[..<t])
        }

        fullChild.keys = Array(fullChild.keys[..<(t - 1)])
        fullChild.values = Array(fullChild.values[..<(t - 1)])

        parent.keys.insert(medianKey, at: i)
        parent.values.insert(medianValue, at: i)
        parent.children.insert(newChild, at: i + 1)
    }

    // ── Insert helper ─────────────────────────────────────────────────────────

    @discardableResult
    private func _insertNonFull(node: BTreeNode<K, V>, key: K, value: V) -> Bool {
        let i = node.findKeyIndex(key)
        if i < node.keys.count && node.keys[i] == key {
            node.values[i] = value
            return false
        }
        if node.isLeaf {
            node.keys.insert(key, at: i)
            node.values.insert(value, at: i)
            return true
        }
        var ci = i
        if node.children[ci].keys.count == 2 * t - 1 {
            _splitChild(parent: node, childIndex: ci)
            // After the split, node.keys[ci] is the newly risen median.
            // Decide which of the two halves to descend into.
            if key > node.keys[ci] {
                ci += 1
            } else if key == node.keys[ci] {
                // The key IS the median that just rose into this node.
                node.values[ci] = value
                return false
            }
            // Note: if key < node.keys[ci], ci stays unchanged → go left half.
        }
        return _insertNonFull(node: node.children[ci], key: key, value: value)
    }

    // ── Delete helpers ────────────────────────────────────────────────────────
    //
    // CLRS top-down deletion.  The algorithm has three cases:
    //
    //  Case A: Key k is in node n, and n is a LEAF.  Remove directly.
    //
    //  Case B: Key k is in node n, and n is INTERNAL.
    //    B1: Left child child[i] has ≥ t keys.
    //        Let pred = the in-order predecessor of k (the max key in child[i]).
    //        Replace k with pred in n, then recursively delete pred from child[i].
    //    B2: Right child child[i+1] has ≥ t keys.
    //        Symmetric to B1 using the in-order successor.
    //    B3: Both children have t-1 keys.
    //        Merge child[i], k, and child[i+1] into child[i].
    //        Remove k from child[i] (which now has 2t-1 keys).
    //
    //  Case C: Key k is NOT in node n.  We must descend into some child[i].
    //    Before descending we ensure child[i] has ≥ t keys so that the
    //    recursive deletion cannot underflow it:
    //    C1: A sibling of child[i] has ≥ t keys → rotate through parent.
    //    C2: All siblings have t-1 keys → merge child[i] with a sibling.
    //
    // The helper `_prepareChild(parent:index:)` ensures the child at the given
    // index has ≥ t keys and returns the new index (which may shift left by 1
    // after a left-merge).

    /// Recursively delete `key` from the subtree rooted at `node`.
    ///
    /// INVARIANT (enforced by caller): `node` has ≥ t keys, OR `node` is the
    /// root.  This ensures that if we delete one key from `node` directly
    /// (Case A), node won't underflow.
    ///
    /// Returns true if the key was found anywhere in the subtree.
    private func _delete(node: BTreeNode<K, V>, key: K) -> Bool {
        let i = node.findKeyIndex(key)

        if i < node.keys.count && node.keys[i] == key {
            // ── Key is in THIS node ──────────────────────────────────────────

            if node.isLeaf {
                // Case A: direct removal from leaf.
                // Safe because caller guarantees node has ≥ t keys (or is root).
                node.keys.remove(at: i)
                node.values.remove(at: i)
                return true
            }

            // Internal node.  We need the left and right children of this key.
            let leftChild  = node.children[i]
            let rightChild = node.children[i + 1]

            if leftChild.keys.count >= t {
                // Case B1: Left child is fat enough.
                // Replace key with its in-order predecessor (max of leftChild
                // subtree), then delete that predecessor from leftChild.
                // Pre-fill leftChild's path using the same deletion descent.
                let (predKey, predVal) = _maxKeyValue(node: leftChild)
                node.keys[i]   = predKey
                node.values[i] = predVal
                // Delete predKey from leftChild.  leftChild has ≥ t keys so it
                // satisfies the invariant for the recursive call.
                _ = _deleteDescend(node: leftChild, key: predKey)
                return true

            } else if rightChild.keys.count >= t {
                // Case B2: Right child is fat enough.
                let (succKey, succVal) = _minKeyValue(node: rightChild)
                node.keys[i]   = succKey
                node.values[i] = succVal
                _ = _deleteDescend(node: rightChild, key: succKey)
                return true

            } else {
                // Case B3: Both children have exactly t-1 keys.
                // Merge: pull the key down, absorb rightChild into leftChild.
                _mergeChildren(parent: node, index: i)
                // The merged child is at children[i] and has 2t-1 keys.
                return _delete(node: node.children[i], key: key)
            }

        } else {
            // ── Key is NOT in this node — must descend ────────────────────────
            if node.isLeaf { return false }

            // Before descending into children[i], ensure it has ≥ t keys so
            // that deleting from it won't cause underflow.
            let childIdx = _prepareChild(parent: node, index: i)

            // After _prepareChild a rotation may have moved the key into the
            // parent.  Re-check before descending.
            let j = node.findKeyIndex(key)
            if j < node.keys.count && node.keys[j] == key {
                return _delete(node: node, key: key)
            }
            return _delete(node: node.children[min(childIdx, node.children.count - 1)], key: key)
        }
    }

    /// Delete `key` from the subtree rooted at `node`, pre-filling each node
    /// on the way down to ensure the ≥ t invariant.
    ///
    /// This is exactly `_delete` but used when we know `node` already has ≥ t
    /// keys (Case B1/B2 path).
    private func _deleteDescend(node: BTreeNode<K, V>, key: K) -> Bool {
        return _delete(node: node, key: key)
    }

    /// Ensure that `parent.children[index]` has ≥ t keys.
    /// Returns the new index of the child after possible restructuring.
    ///
    /// If the child already has ≥ t keys, return `index` unchanged.
    /// Otherwise:
    ///   C1a: Borrow from left sibling   → return `index`
    ///   C1b: Borrow from right sibling  → return `index`
    ///   C2a: Merge with left sibling    → return `index - 1` (merged node)
    ///   C2b: Merge with right sibling   → return `index`
    @discardableResult
    private func _prepareChild(parent: BTreeNode<K, V>, index i: Int) -> Int {
        let child = parent.children[i]
        guard child.keys.count < t else { return i }

        let hasLeft  = i > 0
        let hasRight = i < parent.children.count - 1

        if hasLeft && parent.children[i - 1].keys.count >= t {
            // C1a: rotate from left sibling.
            let left = parent.children[i - 1]
            // Push parent separator down to front of child.
            child.keys.insert(parent.keys[i - 1], at: 0)
            child.values.insert(parent.values[i - 1], at: 0)
            // Pull last key from left sibling up to parent.
            parent.keys[i - 1]   = left.keys.removeLast()
            parent.values[i - 1] = left.values.removeLast()
            // Transfer last child pointer if internal.
            if !left.isLeaf {
                child.children.insert(left.children.removeLast(), at: 0)
            }
            return i

        } else if hasRight && parent.children[i + 1].keys.count >= t {
            // C1b: rotate from right sibling.
            let right = parent.children[i + 1]
            child.keys.append(parent.keys[i])
            child.values.append(parent.values[i])
            parent.keys[i]   = right.keys.removeFirst()
            parent.values[i] = right.values.removeFirst()
            if !right.isLeaf {
                child.children.append(right.children.removeFirst())
            }
            return i

        } else if hasLeft {
            // C2a: merge with left sibling.
            // After merge the combined node sits at index i-1.
            _mergeChildren(parent: parent, index: i - 1)
            return i - 1

        } else {
            // C2b: merge with right sibling.
            _mergeChildren(parent: parent, index: i)
            return i
        }
    }

    /// Merge parent.children[i+1] into parent.children[i], pulling down
    /// parent.keys[i] as the separator.
    private func _mergeChildren(parent: BTreeNode<K, V>, index i: Int) {
        let left  = parent.children[i]
        let right = parent.children[i + 1]

        left.keys.append(parent.keys[i])
        left.values.append(parent.values[i])
        left.keys.append(contentsOf: right.keys)
        left.values.append(contentsOf: right.values)
        if !right.isLeaf {
            left.children.append(contentsOf: right.children)
        }
        parent.keys.remove(at: i)
        parent.values.remove(at: i)
        parent.children.remove(at: i + 1)
    }

    private func _maxKeyValue(node: BTreeNode<K, V>) -> (K, V) {
        var n = node
        while !n.isLeaf { n = n.children[n.children.count - 1] }
        return (n.keys[n.keys.count - 1], n.values[n.values.count - 1])
    }

    private func _minKeyValue(node: BTreeNode<K, V>) -> (K, V) {
        var n = node
        while !n.isLeaf { n = n.children[0] }
        return (n.keys[0], n.values[0])
    }

    // ── Traversal helpers ─────────────────────────────────────────────────────

    private func _inorder(node: BTreeNode<K, V>, result: inout [(K, V)]) {
        for i in 0..<node.keys.count {
            if !node.isLeaf { _inorder(node: node.children[i], result: &result) }
            result.append((node.keys[i], node.values[i]))
        }
        if !node.isLeaf { _inorder(node: node.children[node.keys.count], result: &result) }
    }

    private func _rangeQuery(node: BTreeNode<K, V>, low: K, high: K, result: inout [(K, V)]) {
        for i in 0..<node.keys.count {
            if !node.isLeaf && node.keys[i] > low {
                _rangeQuery(node: node.children[i], low: low, high: high, result: &result)
            }
            if node.keys[i] >= low && node.keys[i] <= high {
                result.append((node.keys[i], node.values[i]))
            }
        }
        if !node.isLeaf {
            let last = node.keys.count
            if node.keys[last - 1] < high {
                _rangeQuery(node: node.children[last], low: low, high: high, result: &result)
            }
        }
    }

    // ── Height / validation helpers ───────────────────────────────────────────

    private func _height(node: BTreeNode<K, V>) -> Int {
        if node.isLeaf { return 0 }
        return 1 + _height(node: node.children[0])
    }

    private func _isValid(
        node: BTreeNode<K, V>,
        minKey: K?,
        maxKey: K?,
        expectedDepth: Int,
        currentDepth: Int,
        isRoot: Bool
    ) -> Bool {
        if node.isLeaf && currentDepth != expectedDepth { return false }
        let minKeys = isRoot ? 0 : (t - 1)
        if node.keys.count < minKeys || node.keys.count > 2 * t - 1 { return false }
        if !node.isLeaf && node.children.count != node.keys.count + 1 { return false }
        for i in 0..<node.keys.count {
            if let lo = minKey, node.keys[i] <= lo { return false }
            if let hi = maxKey, node.keys[i] >= hi { return false }
            if i > 0 && node.keys[i] <= node.keys[i - 1] { return false }
        }
        if !node.isLeaf {
            for i in 0...node.keys.count {
                let cMin = i == 0 ? minKey : Optional(node.keys[i - 1])
                let cMax = i == node.keys.count ? maxKey : Optional(node.keys[i])
                if !_isValid(node: node.children[i], minKey: cMin, maxKey: cMax,
                             expectedDepth: expectedDepth, currentDepth: currentDepth + 1,
                             isRoot: false) { return false }
            }
        }
        return true
    }
}
