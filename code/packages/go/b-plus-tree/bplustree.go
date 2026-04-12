// Package bplustree provides a generic B+ tree data structure.
//
// # What is a B+ Tree?
//
// A B+ tree is a refinement of the B-tree invented in the 1970s.  It is the
// dominant index structure used by relational databases (MySQL InnoDB, PostgreSQL,
// SQLite) and file systems (NTFS, ext4, HFS+).
//
// The key insight: **only leaves store values**.  Internal nodes store only
// keys that act as a routing guide.  A routing key R in an internal node means
// "keys ≥ R go right; keys < R go left".  All leaves are linked in a chain,
// enabling extremely efficient range scans — you find the first matching leaf,
// then walk the chain until you pass the range end.
//
// # B-Tree vs B+ Tree
//
// | Feature | B-Tree | B+ Tree |
// |---------|--------|---------|
// | Values stored | at every node | only at leaves |
// | Leaf scan | O(n log n) — must re-traverse | O(n) — walk the linked list |
// | Space efficiency | values anywhere | denser internal nodes |
// | Common use | general-purpose | databases, file systems |
//
// # Routing Convention (critical for correctness)
//
// An internal node with keys [k0, k1, ..., k_{n-1}] and n+1 children:
//
//	children[0] holds keys < k0
//	children[1] holds keys where k0 ≤ key < k1
//	children[2] holds keys where k1 ≤ key < k2
//	...
//	children[n] holds keys ≥ k_{n-1}
//
// To find which child to descend into for a given key K:
//   - Find the first i where keys[i] > K  (strictly greater)
//   - Descend into children[i]
//
// This is called "upper-bound" routing.  It differs from B-tree routing
// (which uses "lower-bound") because in a B+ tree, key copies in internal nodes
// are routing sentinels — the actual key lives in the leaf.
//
// # ASCII Diagram — B+ Tree with t=2
//
//	Internal level:       [30]
//	                     /    \
//	Internal level:  [20]     [40,50]
//	                 /  \     / | \
//	Leaf level:    [10][20,25][35][45][55]
//	                ↓    ↓     ↓   ↓   ↓
//	                linked list (sorted, left to right)
//
// Every leaf holds actual key-value pairs.
// Internal nodes hold only routing sentinels (copies of real keys).
//
// # Splitting Rules (critical difference from B-tree)
//
// When a leaf node fills up (2t-1 keys), it is split:
//
//  1. The upper half goes into a new right leaf.
//  2. The right leaf's first key is COPIED into the parent as a separator.
//     The key stays in the right leaf — it is NOT removed.
//
// When an internal node fills up, it is split just like a B-tree:
//  1. The upper half goes into a new right internal node.
//  2. The median key is MOVED to the parent (not kept in children).
//
// # This Implementation
//
// We use a Go interface (bplusNode) to represent both node types.
// Go generics let the tree work with any key type via a user-supplied
// less function.
//
// The firstLeaf pointer is maintained at all times for O(1) FullScan start.
//
// Example usage:
//
//	t := bplustree.New[int, string](2, func(a, b int) bool { return a < b })
//	t.Insert(10, "ten")
//	t.Insert(20, "twenty")
//	v, ok := t.Search(10)         // "ten", true
//	pairs := t.RangeScan(5, 15)   // [{10 ten}]
//	all   := t.FullScan()         // [{10 ten} {20 twenty}]
package bplustree

import (
	"errors"
	"fmt"
)

// ---------------------------------------------------------------------------
// KeyValue — a key-value pair returned by scans
// ---------------------------------------------------------------------------

// KeyValue pairs a key with its associated value.
type KeyValue[K any, V any] struct {
	Key   K
	Value V
}

// String renders the pair for debugging.
func (kv KeyValue[K, V]) String() string {
	return fmt.Sprintf("{%v:%v}", kv.Key, kv.Value)
}

// ---------------------------------------------------------------------------
// Node interface and concrete types
// ---------------------------------------------------------------------------

// bplusNode is the interface that both internal nodes and leaf nodes implement.
type bplusNode[K any, V any] interface {
	isLeaf() bool
}

// bplusInternal is an internal (non-leaf) node.
//
// Routing invariant (upper-bound routing):
//
//	children[0]: keys <  keys[0]
//	children[i]: keys[i-1] ≤ keys < keys[i]   (1 ≤ i < len(keys))
//	children[n]: keys ≥ keys[n-1]             (n = len(keys))
//
// To route key K: find i = first index where keys[i] > K; descend children[i].
type bplusInternal[K any, V any] struct {
	keys     []K
	children []bplusNode[K, V]
}

func (n *bplusInternal[K, V]) isLeaf() bool { return false }

// bplusLeaf is a leaf node.
//
// Stores key-value pairs sorted by key.  A next pointer links to the
// adjacent right leaf, forming the leaf linked list used for range scans.
type bplusLeaf[K any, V any] struct {
	keys   []K
	values []V
	next   *bplusLeaf[K, V]
}

func (n *bplusLeaf[K, V]) isLeaf() bool { return true }

// ---------------------------------------------------------------------------
// BPlusTree — the public struct
// ---------------------------------------------------------------------------

// BPlusTree is a generic B+ tree of minimum degree t.
//
// The type parameters are:
//   - K: the key type, ordered via the less function.
//   - V: the value type, unrestricted.
//
// Create with New.
type BPlusTree[K any, V any] struct {
	root      bplusNode[K, V]
	firstLeaf *bplusLeaf[K, V]
	t         int
	size      int
	less      func(a, b K) bool
}

// New creates an empty B+ tree with minimum degree t and comparison function.
//
// t must be ≥ 2.  less must define a strict total order.
func New[K any, V any](t int, less func(a, b K) bool) *BPlusTree[K, V] {
	if t < 2 {
		panic(fmt.Sprintf("bplustree: minimum degree t must be ≥ 2, got %d", t))
	}
	return &BPlusTree[K, V]{t: t, less: less}
}

// ---------------------------------------------------------------------------
// Key comparison helpers
// ---------------------------------------------------------------------------

// equal returns true when a == b (neither less than the other).
func (tr *BPlusTree[K, V]) equal(a, b K) bool {
	return !tr.less(a, b) && !tr.less(b, a)
}

// routeIndex returns the child index to descend into for the given key.
//
// This uses upper-bound routing: find the first i where keys[i] > key.
// All keys in children[i] satisfy keys[i-1] ≤ key < keys[i].
//
// Examples with keys=[10, 20, 30]:
//
//	routeIndex(5)  → 0  (children[0], since 5 < 10)
//	routeIndex(10) → 1  (children[1], since 10 == keys[0] and 10 < 20)
//	routeIndex(15) → 1  (children[1], since 10 ≤ 15 < 20)
//	routeIndex(30) → 3  (children[3], since 30 ≥ 30)
//	routeIndex(99) → 3  (children[3], since 99 ≥ 30)
func (tr *BPlusTree[K, V]) routeIndex(keys []K, key K) int {
	i := 0
	for i < len(keys) && !tr.less(key, keys[i]) {
		// key >= keys[i]  →  go right
		i++
	}
	return i
}

// findInLeaf returns the index of key in leaf.keys, or (idx, false) if not found.
// Uses lower-bound search: first i where keys[i] >= key.
func (tr *BPlusTree[K, V]) findInLeaf(leaf *bplusLeaf[K, V], key K) (int, bool) {
	i := 0
	for i < len(leaf.keys) && tr.less(leaf.keys[i], key) {
		i++
	}
	if i < len(leaf.keys) && tr.equal(leaf.keys[i], key) {
		return i, true
	}
	return i, false
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

// Search looks up a key and returns its value and true if found.
//
// Time complexity: O(t · log_t n)
func (tr *BPlusTree[K, V]) Search(key K) (V, bool) {
	if tr.root == nil {
		var zero V
		return zero, false
	}
	leaf := tr.findLeaf(key)
	idx, found := tr.findInLeaf(leaf, key)
	if found {
		return leaf.values[idx], true
	}
	var zero V
	return zero, false
}

// findLeaf descends the internal nodes to find the leaf where key belongs.
// Uses upper-bound routing at each internal node.
func (tr *BPlusTree[K, V]) findLeaf(key K) *bplusLeaf[K, V] {
	node := tr.root
	for !node.isLeaf() {
		internal := node.(*bplusInternal[K, V])
		i := tr.routeIndex(internal.keys, key)
		node = internal.children[i]
	}
	return node.(*bplusLeaf[K, V])
}

// Contains returns true if the key exists in the tree.
func (tr *BPlusTree[K, V]) Contains(key K) bool {
	_, ok := tr.Search(key)
	return ok
}

// ---------------------------------------------------------------------------
// Insert
// ---------------------------------------------------------------------------

// Insert adds the key-value pair to the tree.  If the key already exists,
// its value is updated.
//
// We use proactive top-down splitting: before descending into a full node,
// we split it.  This keeps insertion as a single downward pass.
//
// Time complexity: O(t · log_t n)
func (tr *BPlusTree[K, V]) Insert(key K, value V) {
	if tr.root == nil {
		// Create the very first leaf.
		leaf := &bplusLeaf[K, V]{}
		tr.root = leaf
		tr.firstLeaf = leaf
	}

	// If root is full, split it — growing the tree upward.
	if tr.nodeIsFull(tr.root) {
		newRoot := &bplusInternal[K, V]{
			children: []bplusNode[K, V]{tr.root},
		}
		tr.splitChild(newRoot, 0)
		tr.root = newRoot
	}

	tr.insertNonFull(tr.root, key, value)
}

// nodeIsFull returns true if the node holds 2t-1 keys (maximum).
// Both node types use the same formula; we extract the key count via keyCount.
func (tr *BPlusTree[K, V]) nodeIsFull(node bplusNode[K, V]) bool {
	return tr.keyCount(node) == 2*tr.t-1
}

// insertNonFull inserts into the subtree rooted at node, which is not full.
// On the way down, any full child is split proactively.
func (tr *BPlusTree[K, V]) insertNonFull(node bplusNode[K, V], key K, value V) {
	if node.isLeaf() {
		leaf := node.(*bplusLeaf[K, V])
		idx, found := tr.findInLeaf(leaf, key)
		if found {
			// Key already exists — update value.
			leaf.values[idx] = value
			return
		}
		// Insert at position idx.
		leaf.keys = append(leaf.keys, key)
		leaf.values = append(leaf.values, value)
		copy(leaf.keys[idx+1:], leaf.keys[idx:])
		copy(leaf.values[idx+1:], leaf.values[idx:])
		leaf.keys[idx] = key
		leaf.values[idx] = value
		tr.size++
		return
	}

	internal := node.(*bplusInternal[K, V])
	// Upper-bound routing: find the child to descend into.
	i := tr.routeIndex(internal.keys, key)

	child := internal.children[i]
	if tr.nodeIsFull(child) {
		tr.splitChild(internal, i)
		// After the split, internal.keys[i] is the new separator.
		// Routing: if key >= separator, go to the right child (i+1).
		if !tr.less(key, internal.keys[i]) {
			i++
		}
	}
	tr.insertNonFull(internal.children[i], key, value)
}

// splitChild splits the full child at position i in parent.
//
// Leaf split — separator is COPIED:
//
//	Before (t=2, leaf=[10,20,30]):
//	  parent.children[i] = [10, 20, 30]
//
//	After:
//	  left  = [10]           (first t keys)
//	  right = [20, 30]       (remaining 2t-1 - t keys)
//	  parent.keys[i] = 20    ← copy of right.keys[0]
//	  parent.children[i]   = left
//	  parent.children[i+1] = right
//
// Internal split — median is MOVED:
//
//	Before (t=2, internal.keys=[10,20,30], 4 children):
//
//	After:
//	  left.keys  = [10]      (first t-1 keys)
//	  right.keys = [30]      (last t-1 keys, after median removed)
//	  parent.keys[i] = 20    ← moved median
func (tr *BPlusTree[K, V]) splitChild(parent *bplusInternal[K, V], i int) {
	child := parent.children[i]
	if child.isLeaf() {
		tr.splitLeaf(parent, i, child.(*bplusLeaf[K, V]))
	} else {
		tr.splitInternal(parent, i, child.(*bplusInternal[K, V]))
	}
}

// splitLeaf splits a full leaf at position i in parent.
//
// The leaf is cut at index t:
//   - left  = keys[0 .. t-1]     (t keys)
//   - right = keys[t .. 2t-2]    (t-1 keys)
//
// The separator pushed up = right.keys[0] (copied, not moved).
// The leaf linked list is updated.
func (tr *BPlusTree[K, V]) splitLeaf(parent *bplusInternal[K, V], i int, leaf *bplusLeaf[K, V]) {
	t := tr.t
	splitIdx := t

	right := &bplusLeaf[K, V]{
		keys:   make([]K, len(leaf.keys)-splitIdx),
		values: make([]V, len(leaf.values)-splitIdx),
		next:   leaf.next,
	}
	copy(right.keys, leaf.keys[splitIdx:])
	copy(right.values, leaf.values[splitIdx:])

	leaf.keys = leaf.keys[:splitIdx]
	leaf.values = leaf.values[:splitIdx]
	leaf.next = right

	// Separator = right.keys[0] (copy).
	sep := right.keys[0]

	// Insert sep into parent at position i.
	parent.keys = append(parent.keys, sep)
	copy(parent.keys[i+1:], parent.keys[i:])
	parent.keys[i] = sep

	parent.children = append(parent.children, nil)
	copy(parent.children[i+2:], parent.children[i+1:])
	parent.children[i+1] = right
}

// splitInternal splits a full internal node at position i in parent.
// The median key (index t-1) is moved to parent.
func (tr *BPlusTree[K, V]) splitInternal(parent *bplusInternal[K, V], i int, node *bplusInternal[K, V]) {
	t := tr.t
	medIdx := t - 1

	right := &bplusInternal[K, V]{
		keys:     make([]K, len(node.keys)-medIdx-1),
		children: make([]bplusNode[K, V], len(node.children)-medIdx-1),
	}
	copy(right.keys, node.keys[medIdx+1:])
	copy(right.children, node.children[medIdx+1:])

	sep := node.keys[medIdx]

	node.keys = node.keys[:medIdx]
	node.children = node.children[:medIdx+1]

	parent.keys = append(parent.keys, sep)
	copy(parent.keys[i+1:], parent.keys[i:])
	parent.keys[i] = sep

	parent.children = append(parent.children, nil)
	copy(parent.children[i+2:], parent.children[i+1:])
	parent.children[i+1] = right
}

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

// Delete removes the key from the tree.  If the key does not exist, Delete
// is a no-op.
//
// All actual key-value pairs live in leaves.  We descend to the correct leaf
// and remove the entry.  On the way down we proactively "fill" any deficient
// child (one that would violate the minimum occupancy after deletion) using
// rotations or merges.  This eliminates backtracking.
//
// Time complexity: O(t · log_t n)
func (tr *BPlusTree[K, V]) Delete(key K) {
	if tr.root == nil {
		return
	}
	tr.deleteAt(tr.root, key)

	// Shrink root if it became an empty internal node.
	if !tr.root.isLeaf() {
		internal := tr.root.(*bplusInternal[K, V])
		if len(internal.keys) == 0 {
			tr.root = internal.children[0]
		}
	}
	// If root is now an empty leaf, clear the tree.
	if tr.root.isLeaf() {
		leaf := tr.root.(*bplusLeaf[K, V])
		if len(leaf.keys) == 0 {
			tr.root = nil
			tr.firstLeaf = nil
		}
	}
}

// minKeys returns the minimum allowed keys for a non-root node.
func (tr *BPlusTree[K, V]) minKeys() int {
	return tr.t - 1
}

// deleteAt recursively deletes key from the subtree at node.
//
// Invariant going in: node itself has more than the minimum keys (so we can
// safely merge/rotate its children without making node deficient).
// Exception: node == tr.root, which is exempt from the minimum key rule.
func (tr *BPlusTree[K, V]) deleteAt(node bplusNode[K, V], key K) {
	if node.isLeaf() {
		leaf := node.(*bplusLeaf[K, V])
		idx, found := tr.findInLeaf(leaf, key)
		if !found {
			return
		}
		leaf.keys = append(leaf.keys[:idx], leaf.keys[idx+1:]...)
		leaf.values = append(leaf.values[:idx], leaf.values[idx+1:]...)
		tr.size--
		return
	}

	internal := node.(*bplusInternal[K, V])
	i := tr.routeIndex(internal.keys, key)
	child := internal.children[i]

	// Fill the child proactively if it has the minimum number of keys.
	// After deletion it would drop below the minimum, violating the invariant.
	// Exception: if child is a leaf with exactly t-1 keys AND there is room
	// to borrow/merge, we must fill it now.
	//
	// The threshold is: if keyCount(child) <= t-1 (= minKeys()), fill it.
	// This ensures the child has at least t keys after we descend, so deletion
	// leaves it at t-1 (still valid).
	if tr.keyCount(child) <= tr.minKeys() {
		tr.fill(internal, i)
		// After fill the tree structure may have shifted (a merge collapses
		// two children into one).  Re-route.
		i = tr.routeIndex(internal.keys, key)
		// Guard: if all children merged, i may be out of bounds.
		if i >= len(internal.children) {
			i = len(internal.children) - 1
		}
		child = internal.children[i]
	}

	tr.deleteAt(child, key)
}

// keyCount returns the number of keys in a node.
// Every concrete bplusNode type exposes a keys slice; we dispatch on the type.
func (tr *BPlusTree[K, V]) keyCount(node bplusNode[K, V]) int {
	if node.isLeaf() {
		return len(node.(*bplusLeaf[K, V]).keys)
	}
	return len(node.(*bplusInternal[K, V]).keys)
}

// fill ensures internal.children[i] has more than minKeys() keys.
// Tries (in order): borrow from left, borrow from right, merge.
func (tr *BPlusTree[K, V]) fill(internal *bplusInternal[K, V], i int) {
	min := tr.minKeys()
	hasLeft := i > 0 && tr.keyCount(internal.children[i-1]) > min
	hasRight := i < len(internal.children)-1 && tr.keyCount(internal.children[i+1]) > min

	if hasLeft {
		tr.rotateRight(internal, i)
	} else if hasRight {
		tr.rotateLeft(internal, i)
	} else if i < len(internal.children)-1 {
		tr.merge(internal, i) // merge children[i] and children[i+1]
	} else {
		tr.merge(internal, i-1) // merge children[i-1] and children[i]
	}
}

// rotateRight borrows a key from children[i-1] (left sibling) and prepends
// it to children[i].
//
// For leaves:
//   - Move last key of left sibling to front of child.
//   - Update parent separator: parent.keys[i-1] = child.keys[0].
//
// For internal nodes:
//   - Standard B-tree right rotation: push parent separator down, pull last
//     key of left sibling up.
func (tr *BPlusTree[K, V]) rotateRight(parent *bplusInternal[K, V], i int) {
	child := parent.children[i]
	leftSib := parent.children[i-1]

	if child.isLeaf() {
		leftLeaf := leftSib.(*bplusLeaf[K, V])
		childLeaf := child.(*bplusLeaf[K, V])

		last := len(leftLeaf.keys) - 1
		bKey := leftLeaf.keys[last]
		bVal := leftLeaf.values[last]
		leftLeaf.keys = leftLeaf.keys[:last]
		leftLeaf.values = leftLeaf.values[:last]

		// Prepend to child.
		childLeaf.keys = append(childLeaf.keys, bKey)
		copy(childLeaf.keys[1:], childLeaf.keys)
		childLeaf.keys[0] = bKey
		childLeaf.values = append(childLeaf.values, bVal)
		copy(childLeaf.values[1:], childLeaf.values)
		childLeaf.values[0] = bVal

		// Update separator: first key of child.
		parent.keys[i-1] = childLeaf.keys[0]
	} else {
		leftInt := leftSib.(*bplusInternal[K, V])
		childInt := child.(*bplusInternal[K, V])

		// Push parent separator down to front of child.
		childInt.keys = append(childInt.keys, parent.keys[i-1])
		copy(childInt.keys[1:], childInt.keys)
		childInt.keys[0] = parent.keys[i-1]
		// Move last child pointer of leftSib to front of child.
		childInt.children = append(childInt.children, nil)
		copy(childInt.children[1:], childInt.children)
		childInt.children[0] = leftInt.children[len(leftInt.children)-1]
		// Pull last key of leftSib up to parent.
		parent.keys[i-1] = leftInt.keys[len(leftInt.keys)-1]
		leftInt.keys = leftInt.keys[:len(leftInt.keys)-1]
		leftInt.children = leftInt.children[:len(leftInt.children)-1]
	}
}

// rotateLeft borrows a key from children[i+1] (right sibling) and appends
// it to children[i].
func (tr *BPlusTree[K, V]) rotateLeft(parent *bplusInternal[K, V], i int) {
	child := parent.children[i]
	rightSib := parent.children[i+1]

	if child.isLeaf() {
		rightLeaf := rightSib.(*bplusLeaf[K, V])
		childLeaf := child.(*bplusLeaf[K, V])

		bKey := rightLeaf.keys[0]
		bVal := rightLeaf.values[0]
		rightLeaf.keys = rightLeaf.keys[1:]
		rightLeaf.values = rightLeaf.values[1:]

		childLeaf.keys = append(childLeaf.keys, bKey)
		childLeaf.values = append(childLeaf.values, bVal)

		// Update separator: first key of right sibling.
		parent.keys[i] = rightLeaf.keys[0]
	} else {
		rightInt := rightSib.(*bplusInternal[K, V])
		childInt := child.(*bplusInternal[K, V])

		childInt.keys = append(childInt.keys, parent.keys[i])
		childInt.children = append(childInt.children, rightInt.children[0])

		parent.keys[i] = rightInt.keys[0]
		rightInt.keys = rightInt.keys[1:]
		rightInt.children = rightInt.children[1:]
	}
}

// merge merges children[i+1] into children[i] and removes the separator
// from parent.keys[i].
//
// For leaves: concatenate keys/values, update next pointer, remove separator.
// For internals: pull the separator down, concatenate, remove separator.
func (tr *BPlusTree[K, V]) merge(parent *bplusInternal[K, V], i int) {
	left := parent.children[i]
	right := parent.children[i+1]

	if left.isLeaf() {
		leftLeaf := left.(*bplusLeaf[K, V])
		rightLeaf := right.(*bplusLeaf[K, V])

		leftLeaf.keys = append(leftLeaf.keys, rightLeaf.keys...)
		leftLeaf.values = append(leftLeaf.values, rightLeaf.values...)
		leftLeaf.next = rightLeaf.next
	} else {
		leftInt := left.(*bplusInternal[K, V])
		rightInt := right.(*bplusInternal[K, V])
		sep := parent.keys[i]

		leftInt.keys = append(leftInt.keys, sep)
		leftInt.keys = append(leftInt.keys, rightInt.keys...)
		leftInt.children = append(leftInt.children, rightInt.children...)
	}

	// Remove separator and right child from parent.
	parent.keys = append(parent.keys[:i], parent.keys[i+1:]...)
	parent.children = append(parent.children[:i+1], parent.children[i+2:]...)
}

// ---------------------------------------------------------------------------
// MinKey / MaxKey
// ---------------------------------------------------------------------------

// MinKey returns the smallest key — O(1) via firstLeaf.
func (tr *BPlusTree[K, V]) MinKey() (K, error) {
	if tr.firstLeaf == nil || len(tr.firstLeaf.keys) == 0 {
		var zero K
		return zero, errors.New("bplustree: tree is empty")
	}
	return tr.firstLeaf.keys[0], nil
}

// MaxKey returns the largest key — O(log n) via rightmost leaf.
func (tr *BPlusTree[K, V]) MaxKey() (K, error) {
	if tr.root == nil {
		var zero K
		return zero, errors.New("bplustree: tree is empty")
	}
	node := tr.root
	for !node.isLeaf() {
		internal := node.(*bplusInternal[K, V])
		node = internal.children[len(internal.children)-1]
	}
	leaf := node.(*bplusLeaf[K, V])
	// Invariant: the tree is non-empty (root != nil), so the rightmost leaf
	// must have at least one key.  This is guaranteed by Delete shrinking
	// the root when it becomes empty.
	return leaf.keys[len(leaf.keys)-1], nil
}

// ---------------------------------------------------------------------------
// Scans
// ---------------------------------------------------------------------------

// RangeScan returns all key-value pairs where low ≤ key ≤ high, in order.
//
// Algorithm:
//  1. findLeaf(low) — descend to the first leaf that might contain low. O(log n)
//  2. Walk the leaf linked list until key > high.                        O(k)
//
// Total: O(log n + k) where k is the number of results.
func (tr *BPlusTree[K, V]) RangeScan(low, high K) []KeyValue[K, V] {
	if tr.root == nil {
		return nil
	}
	var result []KeyValue[K, V]
	leaf := tr.findLeaf(low)
	for leaf != nil {
		for i, k := range leaf.keys {
			if tr.less(high, k) {
				return result
			}
			if !tr.less(k, low) {
				result = append(result, KeyValue[K, V]{Key: k, Value: leaf.values[i]})
			}
		}
		leaf = leaf.next
	}
	return result
}

// FullScan returns all key-value pairs in sorted order by walking the leaf list.
//
// Time complexity: O(n)  (no tree traversal needed after firstLeaf)
func (tr *BPlusTree[K, V]) FullScan() []KeyValue[K, V] {
	var result []KeyValue[K, V]
	leaf := tr.firstLeaf
	for leaf != nil {
		for i, k := range leaf.keys {
			result = append(result, KeyValue[K, V]{Key: k, Value: leaf.values[i]})
		}
		leaf = leaf.next
	}
	return result
}

// ---------------------------------------------------------------------------
// Len, Height
// ---------------------------------------------------------------------------

// Len returns the total number of key-value pairs.
func (tr *BPlusTree[K, V]) Len() int {
	return tr.size
}

// Height returns the height of the tree (levels minus one).
// Empty tree: -1.  Root leaf only: 0.
func (tr *BPlusTree[K, V]) Height() int {
	if tr.root == nil {
		return -1
	}
	h := 0
	node := tr.root
	for !node.isLeaf() {
		h++
		internal := node.(*bplusInternal[K, V])
		node = internal.children[0]
	}
	return h
}

// ---------------------------------------------------------------------------
// IsValid
// ---------------------------------------------------------------------------

// IsValid checks all B+ tree structural invariants.
//
// Invariants checked:
//  1. All leaves at same depth.
//  2. Each non-root node has t-1 ≤ len(keys) ≤ 2t-1.
//  3. Root has 1 ≤ len(keys) ≤ 2t-1 (if non-empty, non-leaf).
//  4. Keys sorted within each node.
//  5. Correct child count in internal nodes.
//  6. Leaf linked list is in ascending order.
//  7. Total key count matches tr.size.
//  8. firstLeaf points to the actual leftmost leaf.
func (tr *BPlusTree[K, V]) IsValid() bool {
	if tr.root == nil {
		return tr.size == 0 && tr.firstLeaf == nil
	}
	var counted int
	leafDepth := -1
	var prevLeaf *bplusLeaf[K, V]
	if !tr.validateNode(tr.root, true, 0, &leafDepth, &counted, &prevLeaf) {
		return false
	}
	if counted != tr.size {
		return false
	}
	// Verify firstLeaf == actual leftmost leaf.
	node := tr.root
	for !node.isLeaf() {
		internal := node.(*bplusInternal[K, V])
		node = internal.children[0]
	}
	return node.(*bplusLeaf[K, V]) == tr.firstLeaf
}

func (tr *BPlusTree[K, V]) validateNode(
	node bplusNode[K, V],
	isRoot bool,
	depth int,
	leafDepth *int,
	counted *int,
	prevLeaf **bplusLeaf[K, V],
) bool {
	t := tr.t

	if node.isLeaf() {
		leaf := node.(*bplusLeaf[K, V])
		n := len(leaf.keys)

		// Key count.
		if !isRoot && n < t-1 {
			return false
		}
		if n > 2*t-1 {
			return false
		}

		// Uniform leaf depth.
		if *leafDepth == -1 {
			*leafDepth = depth
		} else if *leafDepth != depth {
			return false
		}

		// Sorted keys within leaf.
		for i := 1; i < n; i++ {
			if !tr.less(leaf.keys[i-1], leaf.keys[i]) {
				return false
			}
		}

		// Leaf chain order.
		if *prevLeaf != nil && n > 0 && len((*prevLeaf).keys) > 0 {
			lastPrev := (*prevLeaf).keys[len((*prevLeaf).keys)-1]
			if !tr.less(lastPrev, leaf.keys[0]) {
				return false
			}
		}
		*prevLeaf = leaf
		*counted += n
		return true
	}

	internal := node.(*bplusInternal[K, V])
	n := len(internal.keys)

	// Key count for internal nodes.
	if isRoot {
		if n < 1 || n > 2*t-1 {
			return false
		}
	} else {
		if n < t-1 || n > 2*t-1 {
			return false
		}
	}

	// Sorted keys.
	for i := 1; i < n; i++ {
		if !tr.less(internal.keys[i-1], internal.keys[i]) {
			return false
		}
	}

	// Child count.
	if len(internal.children) != n+1 {
		return false
	}

	for _, child := range internal.children {
		if !tr.validateNode(child, false, depth+1, leafDepth, counted, prevLeaf) {
			return false
		}
	}
	return true
}
