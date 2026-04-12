// Package btree provides a generic B-tree data structure.
//
// # What is a B-Tree?
//
// A B-tree is a self-balancing search tree designed for systems that read and
// write large blocks of data — primarily databases and file systems.  Rudolf
// Bayer and Edward McCreight invented it at Boeing Research Labs in 1970.  The
// "B" most likely stands for "Bayer", "Boeing", or "Balanced" — the inventors
// never confirmed.
//
// Unlike a binary search tree (BST), which allows exactly 2 children per node,
// a B-tree node can hold many keys and children.  This matters enormously for
// disk-based storage: reading from disk is 100,000× slower than reading from
// RAM.  If the tree is short and fat instead of tall and thin, you need far
// fewer disk reads to answer a query.
//
// # The B-Tree Invariants
//
// A B-tree of minimum degree t satisfies all of these at all times:
//
//  1. Every node holds at most 2t-1 keys and at most 2t children.
//  2. Every node (except the root) holds at least t-1 keys and at least t
//     children (when it is an internal node).
//  3. The root holds at least 1 key (when the tree is non-empty).
//  4. All leaves are at the same depth.
//  5. Keys within a node are sorted in ascending order.
//  6. For internal nodes: all keys in children[i] are strictly between
//     keys[i-1] and keys[i].
//
// Think of t like a branching factor.  t=2 gives a 2-3-4 tree (each node
// has 1, 2, or 3 keys and 2, 3, or 4 children).  Databases often use
// t=100 or larger — a three-level tree then holds 100^3 = 1,000,000 entries.
//
// # ASCII Diagram — B-tree with t=2
//
//	        [30]
//	       /    \
//	   [10,20]  [40,50]
//	   / | \    / | \
//	 [5][15][25][35][45][55]
//
// All leaves are at depth 2.  The root has 1 key and 2 children.
// Internal nodes have 2 keys and 3 children.
// Leaves hold actual key-value pairs.
//
// # This Implementation
//
// We use Go generics so the tree works with any ordered key type via a
// user-supplied less function.  Values are also generic — a value can be
// anything from an integer to a complex struct.
//
// We use proactive top-down splitting during insertion: when we encounter a
// full node on the path to the insertion leaf, we split it immediately before
// descending.  This means insertion is a single downward pass — no backtracking
// needed.
//
// Example usage:
//
//	t := btree.New[int, string](2, func(a, b int) bool { return a < b })
//	t.Insert(10, "ten")
//	t.Insert(20, "twenty")
//	v, ok := t.Search(10)   // v = "ten", ok = true
//	fmt.Println(t.Inorder()) // [{10 ten} {20 twenty}]
package btree

import (
	"errors"
	"fmt"
)

// ---------------------------------------------------------------------------
// KeyValue — a key-value pair returned by range queries and traversals
// ---------------------------------------------------------------------------

// KeyValue pairs a key with its associated value.  Returned by Inorder and
// RangeQuery so callers can iterate over the tree's contents.
type KeyValue[K any, V any] struct {
	Key   K
	Value V
}

// String renders the pair as "Key:Value" for debugging.
func (kv KeyValue[K, V]) String() string {
	return fmt.Sprintf("{%v:%v}", kv.Key, kv.Value)
}

// ---------------------------------------------------------------------------
// Internal node type
// ---------------------------------------------------------------------------

// bTreeNode is one node in the B-tree.
//
// Each node stores up to 2t-1 keys and the corresponding values.  Internal
// nodes additionally store 2t child pointers.  Leaf nodes have no children
// (isLeaf = true, len(children) = 0).
//
// Key ordering invariant (for internal node):
//   - keys[0] separates children[0] from children[1]
//   - keys[1] separates children[1] from children[2]
//   - ...
//   - keys[i] separates children[i] from children[i+1]
//
// Diagram for a node with keys [10, 20, 30]:
//
//	children[0] | 10 | children[1] | 20 | children[2] | 30 | children[3]
//	   (< 10)               (10–20)              (20–30)              (> 30)
type bTreeNode[K any, V any] struct {
	keys     []K
	values   []V
	children []*bTreeNode[K, V]
	isLeaf   bool
}

// isFull returns true when this node holds 2t-1 keys (no more can be added
// without first splitting the node).
func (n *bTreeNode[K, V]) isFull(t int) bool {
	return len(n.keys) == 2*t-1
}

// ---------------------------------------------------------------------------
// BTree — the public struct
// ---------------------------------------------------------------------------

// BTree is a generic B-tree of minimum degree t.
//
// The type parameters are:
//   - K: the key type.  Must be totally ordered via the less function.
//   - V: the value type.  Any Go type is allowed.
//
// Create with New.  All methods are safe to call on a zero-size tree.
type BTree[K any, V any] struct {
	root *bTreeNode[K, V]
	t    int              // minimum degree (every non-root node has ≥ t-1 keys)
	size int              // total number of key-value pairs
	less func(a, b K) bool // strict ordering: less(a,b) iff a < b
}

// New creates an empty B-tree with the given minimum degree t and comparison
// function less.
//
// t must be ≥ 2 (t=1 would allow 0-key nodes, which is degenerate).
// less must define a strict total order:
//   - Irreflexivity: !less(a, a)
//   - Transitivity:  less(a,b) && less(b,c) ⟹ less(a,c)
//   - Asymmetry:     less(a,b) ⟹ !less(b,a)
//
// Example:
//
//	tree := btree.New[int, string](2, func(a, b int) bool { return a < b })
func New[K any, V any](t int, less func(a, b K) bool) *BTree[K, V] {
	if t < 2 {
		panic(fmt.Sprintf("btree: minimum degree t must be ≥ 2, got %d", t))
	}
	return &BTree[K, V]{t: t, less: less}
}

// ---------------------------------------------------------------------------
// Helpers: key comparison
// ---------------------------------------------------------------------------

// equal returns true when a == b according to the less function.
// Since less defines a strict total order: a == b ⟺ !less(a,b) && !less(b,a).
func (tr *BTree[K, V]) equal(a, b K) bool {
	return !tr.less(a, b) && !tr.less(b, a)
}

// findKeyIndex returns the smallest index i in node.keys such that
// node.keys[i] >= key.  If all keys are strictly less, returns len(node.keys).
//
// This is the standard B-tree binary-search step that tells us which child
// to descend into (or where to insert a new key).
func (tr *BTree[K, V]) findKeyIndex(node *bTreeNode[K, V], key K) int {
	// Linear scan is fine for small t; for large t, binary search would help.
	i := 0
	for i < len(node.keys) && tr.less(node.keys[i], key) {
		i++
	}
	return i
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

// Search looks up a key and returns its value and true if found, or the zero
// value of V and false otherwise.
//
// Time complexity: O(t · log_t n)  (t keys checked per node, log_t n levels)
//
// Example:
//
//	v, ok := tree.Search(42)
func (tr *BTree[K, V]) Search(key K) (V, bool) {
	return tr.searchNode(tr.root, key)
}

func (tr *BTree[K, V]) searchNode(node *bTreeNode[K, V], key K) (V, bool) {
	if node == nil {
		var zero V
		return zero, false
	}
	i := tr.findKeyIndex(node, key)
	// Did we land exactly on this key?
	if i < len(node.keys) && tr.equal(node.keys[i], key) {
		return node.values[i], true
	}
	// Not found here; if leaf, give up.
	if node.isLeaf {
		var zero V
		return zero, false
	}
	// Recurse into the appropriate child.
	return tr.searchNode(node.children[i], key)
}

// Contains returns true if the key exists in the tree.
func (tr *BTree[K, V]) Contains(key K) bool {
	_, ok := tr.Search(key)
	return ok
}

// ---------------------------------------------------------------------------
// Insert
// ---------------------------------------------------------------------------

// Insert adds the key-value pair to the tree.  If the key already exists,
// its value is updated to the new value.
//
// We use proactive top-down splitting: before descending into a full child,
// we split it.  This guarantees there is always room for the new key at the
// leaf level without any backtracking.
//
// Time complexity: O(t · log_t n)
//
// Example:
//
//	tree.Insert(10, "ten")
func (tr *BTree[K, V]) Insert(key K, value V) {
	if tr.root == nil {
		// First insertion: create root as a leaf.
		tr.root = &bTreeNode[K, V]{isLeaf: true}
	}

	// If the root itself is full, split it before descending.
	// This is the only place the tree grows in height.
	//
	// Splitting the root:
	//   1. Create a new (empty) root node.
	//   2. Make the old root the sole child of the new root.
	//   3. Split the old root, promoting its median key to the new root.
	//
	// Before split (root full, t=2, keys=[10,20,30]):
	//
	//	   [10,20,30]   ← old root (full)
	//
	// After split:
	//
	//	      [20]       ← new root
	//	     /    \
	//	  [10]   [30]
	if tr.root.isFull(tr.t) {
		newRoot := &bTreeNode[K, V]{
			isLeaf:   false,
			children: []*bTreeNode[K, V]{tr.root},
		}
		tr.splitChild(newRoot, 0)
		tr.root = newRoot
	}

	tr.insertNonFull(tr.root, key, value)
}

// insertNonFull inserts into a subtree rooted at node, which is guaranteed
// not to be full.  On the way down, any full child is split proactively.
func (tr *BTree[K, V]) insertNonFull(node *bTreeNode[K, V], key K, value V) {
	i := tr.findKeyIndex(node, key)

	// If the key already exists in this node, update the value.
	if i < len(node.keys) && tr.equal(node.keys[i], key) {
		node.values[i] = value
		return
	}

	if node.isLeaf {
		// Insert key/value at position i, shifting everything right.
		node.keys = append(node.keys, key)     // grow by 1
		node.values = append(node.values, value)
		copy(node.keys[i+1:], node.keys[i:])
		copy(node.values[i+1:], node.values[i:])
		node.keys[i] = key
		node.values[i] = value
		tr.size++
		return
	}

	// Internal node: recurse into children[i].
	// But first, if children[i] is full, split it so we never descend into
	// a full node (top-down splitting invariant).
	if node.children[i].isFull(tr.t) {
		tr.splitChild(node, i)
		// After splitting, the median of the old children[i] is now node.keys[i].
		// Decide which of the two new children to descend into.
		if tr.less(node.keys[i], key) {
			i++
		} else if tr.equal(node.keys[i], key) {
			// The split median IS the key we want to insert — update it.
			node.values[i] = value
			return
		}
	}

	tr.insertNonFull(node.children[i], key, value)
}

// splitChild splits the i-th child of parent, which must be full (2t-1 keys).
//
// Splitting promotes the median key (at index t-1) to the parent, and
// creates a new sibling node holding the upper half (t-1 keys).
//
// Before (t=2, child=[10,20,30]):
//
//	parent: [... | ... ]  children: [..., child, ...]
//
// After (median=20 promoted):
//
//	parent: [... | 20 | ... ]  children: [..., left=[10], right=[30], ...]
//
// The child is split at position t-1:
//   - left  (original child): keys[0 .. t-2]  + children[0 .. t-1]
//   - right (new sibling):    keys[t .. 2t-2] + children[t .. 2t-1]
//   - median: keys[t-1] is promoted to the parent
func (tr *BTree[K, V]) splitChild(parent *bTreeNode[K, V], i int) {
	t := tr.t
	child := parent.children[i]
	medianIdx := t - 1

	// Create the right sibling with the upper half of child's keys.
	right := &bTreeNode[K, V]{
		isLeaf: child.isLeaf,
		keys:   make([]K, t-1),
		values: make([]V, t-1),
	}
	copy(right.keys, child.keys[t:])
	copy(right.values, child.values[t:])

	if !child.isLeaf {
		right.children = make([]*bTreeNode[K, V], t)
		copy(right.children, child.children[t:])
	}

	// The median key/value moves up to the parent.
	medianKey := child.keys[medianIdx]
	medianVal := child.values[medianIdx]

	// Truncate the left child to t-1 keys.
	child.keys = child.keys[:medianIdx]
	child.values = child.values[:medianIdx]
	if !child.isLeaf {
		child.children = child.children[:t]
	}

	// Insert the median into the parent at position i.
	parent.keys = append(parent.keys, medianKey)
	copy(parent.keys[i+1:], parent.keys[i:])
	parent.keys[i] = medianKey

	parent.values = append(parent.values, medianVal)
	copy(parent.values[i+1:], parent.values[i:])
	parent.values[i] = medianVal

	// Insert the right child into the parent's child list at position i+1.
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
// B-tree deletion has three main cases depending on where the key lives:
//
//  1. Key is in a leaf node → remove it directly.
//  2. Key is in an internal node → replace it with the in-order predecessor
//     (max of left subtree) or in-order successor (min of right subtree),
//     then delete from the child.
//  3. Key is not in the current node → find the appropriate child and recurse,
//     but first ensure the child has at least t keys (by rotating or merging)
//     so deletion never leaves a node with too few keys.
//
// We guarantee every node we descend into has at least t keys (except the
// root, which can have as few as 1).  This eliminates backtracking.
//
// Time complexity: O(t · log_t n)
func (tr *BTree[K, V]) Delete(key K) {
	if tr.root == nil {
		return
	}
	tr.delete(tr.root, key)

	// If the root becomes empty after deletion (can happen when a merge
	// collapses the root's only child back into the root), shrink the tree.
	if len(tr.root.keys) == 0 {
		if tr.root.isLeaf {
			tr.root = nil
		} else {
			tr.root = tr.root.children[0]
		}
	}
}

// delete is the recursive helper for Delete.  It assumes node has at least t
// keys (enforced by the caller via fill before recursion), except for the root.
func (tr *BTree[K, V]) delete(node *bTreeNode[K, V], key K) {
	t := tr.t
	i := tr.findKeyIndex(node, key)
	found := i < len(node.keys) && tr.equal(node.keys[i], key)

	if found {
		if node.isLeaf {
			// Case 1: Key is in a leaf → just remove it.
			//
			// Before: [..., key, ...]
			// After:  [..., ...]
			node.keys = append(node.keys[:i], node.keys[i+1:]...)
			node.values = append(node.values[:i], node.values[i+1:]...)
			tr.size--
			return
		}

		// Case 2: Key is in an internal node.
		left := node.children[i]
		right := node.children[i+1]

		if len(left.keys) >= t {
			// Case 2a: The left child has at least t keys.
			// Replace node.keys[i] with the in-order predecessor (max of left),
			// then delete that predecessor from the left subtree.
			//
			//    [... | K | ...]          [... | pred | ...]
			//         /  \         →           /
			//   [..., pred]               [...] (pred removed)
			predKey, predVal := tr.maxKey(left)
			node.keys[i] = predKey
			node.values[i] = predVal
			tr.delete(left, predKey)
		} else if len(right.keys) >= t {
			// Case 2b: The right child has at least t keys.
			// Replace with the in-order successor (min of right), then delete.
			//
			//    [... | K | ...]          [... | succ | ...]
			//         /  \         →                  \
			//             [succ, ...]                 [...] (succ removed)
			succKey, succVal := tr.minKey(right)
			node.keys[i] = succKey
			node.values[i] = succVal
			tr.delete(right, succKey)
		} else {
			// Case 2c: Both children have exactly t-1 keys.
			// Merge right into left (with K as the separator), then delete K
			// from the merged node.
			//
			// Before:  node = [..., K, ...]
			//          left = [L1, L2], right = [R1, R2]  (t-1=1 keys each for t=2)
			//
			// After merge: left = [L1, L2, K, R1, R2]
			//              node.children[i] = left
			//              node.keys[i] removed
			tr.mergeChildren(node, i)
			tr.delete(node.children[i], key)
		}
		return
	}

	// Case 3: Key not in this node; recurse into the appropriate child.
	if node.isLeaf {
		// Key does not exist in the tree.
		return
	}

	// Before descending, ensure the child has at least t keys.
	// If it has only t-1 keys, we "fill" it by borrowing from a sibling or
	// merging with a sibling.
	if len(node.children[i].keys) < t {
		tr.fill(node, i)
		// After fill the tree structure may have shifted; re-find the key index.
		// A merge at i-1 means i should decrease by 1.
		i = tr.findKeyIndex(node, key)
		if i < len(node.keys) && tr.equal(node.keys[i], key) {
			// The key ended up in the parent after a rotation — handle it.
			tr.delete(node, key)
			return
		}
	}
	tr.delete(node.children[i], key)
}

// fill ensures node.children[i] has at least t keys.
// It tries (in order):
//  1. Rotate from the left sibling (children[i-1] has ≥ t keys).
//  2. Rotate from the right sibling (children[i+1] has ≥ t keys).
//  3. Merge children[i] with a sibling.
func (tr *BTree[K, V]) fill(node *bTreeNode[K, V], i int) {
	t := tr.t
	leftSiblingHasExtra := i > 0 && len(node.children[i-1].keys) >= t
	rightSiblingHasExtra := i < len(node.children)-1 && len(node.children[i+1].keys) >= t

	if leftSiblingHasExtra {
		tr.rotateRight(node, i)
	} else if rightSiblingHasExtra {
		tr.rotateLeft(node, i)
	} else if i < len(node.children)-1 {
		tr.mergeChildren(node, i)
	} else {
		tr.mergeChildren(node, i-1)
	}
}

// rotateRight borrows the last key from children[i-1] (left sibling) and
// prepends it to children[i] via the parent separator at keys[i-1].
//
// Before (parent key P, left sibling L, right child C):
//
//	parent: [..., P, ...]
//	L = [A, B, X]    C = [D, E]
//
// After:
//
//	parent: [..., X, ...]
//	L = [A, B]       C = [P, D, E]
func (tr *BTree[K, V]) rotateRight(parent *bTreeNode[K, V], i int) {
	child := parent.children[i]
	leftSib := parent.children[i-1]

	// Shift all of child's keys right by 1 to make room for the parent separator.
	child.keys = append(child.keys, parent.keys[i-1])
	copy(child.keys[1:], child.keys[:len(child.keys)-1])
	child.keys[0] = parent.keys[i-1]

	child.values = append(child.values, parent.values[i-1])
	copy(child.values[1:], child.values[:len(child.values)-1])
	child.values[0] = parent.values[i-1]

	// Pull the last key from the left sibling up to the parent.
	last := len(leftSib.keys) - 1
	parent.keys[i-1] = leftSib.keys[last]
	parent.values[i-1] = leftSib.values[last]
	leftSib.keys = leftSib.keys[:last]
	leftSib.values = leftSib.values[:last]

	// If internal nodes, move the last child of leftSib to the front of child.
	if !child.isLeaf {
		child.children = append(child.children, nil)
		copy(child.children[1:], child.children[:len(child.children)-1])
		child.children[0] = leftSib.children[len(leftSib.children)-1]
		leftSib.children = leftSib.children[:len(leftSib.children)-1]
	}
}

// rotateLeft borrows the first key from children[i+1] (right sibling) and
// appends it to children[i] via the parent separator at keys[i].
//
// Before (parent key P, child C, right sibling R):
//
//	parent: [..., P, ...]
//	C = [A, B]    R = [X, D, E]
//
// After:
//
//	parent: [..., X, ...]
//	C = [A, B, P]    R = [D, E]
func (tr *BTree[K, V]) rotateLeft(parent *bTreeNode[K, V], i int) {
	child := parent.children[i]
	rightSib := parent.children[i+1]

	// Append the parent separator to the end of child.
	child.keys = append(child.keys, parent.keys[i])
	child.values = append(child.values, parent.values[i])

	// Pull the first key from right sibling up to the parent.
	parent.keys[i] = rightSib.keys[0]
	parent.values[i] = rightSib.values[0]
	rightSib.keys = rightSib.keys[1:]
	rightSib.values = rightSib.values[1:]

	// If internal nodes, move the first child of rightSib to the end of child.
	if !child.isLeaf {
		child.children = append(child.children, rightSib.children[0])
		rightSib.children = rightSib.children[1:]
	}
}

// mergeChildren merges children[i+1] into children[i], pulling down the
// separator key at parent.keys[i].
//
// After the merge, children[i+1] is removed and parent.keys[i] is removed.
// The merged node has 2t-1 keys (left t-1 + separator 1 + right t-1).
//
// Before (t=2, separator=20, left=[10], right=[30]):
//
//	parent = [20]
//	children: [[10], [30]]
//
// After merge:
//
//	parent = []
//	children: [[10, 20, 30]]
func (tr *BTree[K, V]) mergeChildren(parent *bTreeNode[K, V], i int) {
	left := parent.children[i]
	right := parent.children[i+1]
	sep := parent.keys[i]
	sepVal := parent.values[i]

	// Pull the separator key down into the left child.
	left.keys = append(left.keys, sep)
	left.values = append(left.values, sepVal)

	// Append all of right's keys/values/children to left.
	left.keys = append(left.keys, right.keys...)
	left.values = append(left.values, right.values...)
	if !left.isLeaf {
		left.children = append(left.children, right.children...)
	}

	// Remove the separator from the parent and the right child pointer.
	parent.keys = append(parent.keys[:i], parent.keys[i+1:]...)
	parent.values = append(parent.values[:i], parent.values[i+1:]...)
	parent.children = append(parent.children[:i+1], parent.children[i+2:]...)
}

// maxKey returns the maximum key in the subtree rooted at node.
func (tr *BTree[K, V]) maxKey(node *bTreeNode[K, V]) (K, V) {
	for !node.isLeaf {
		node = node.children[len(node.children)-1]
	}
	last := len(node.keys) - 1
	return node.keys[last], node.values[last]
}

// minKey returns the minimum key in the subtree rooted at node.
func (tr *BTree[K, V]) minKey(node *bTreeNode[K, V]) (K, V) {
	for !node.isLeaf {
		node = node.children[0]
	}
	return node.keys[0], node.values[0]
}

// ---------------------------------------------------------------------------
// MinKey / MaxKey
// ---------------------------------------------------------------------------

// MinKey returns the smallest key in the tree.
// Returns an error if the tree is empty.
func (tr *BTree[K, V]) MinKey() (K, error) {
	if tr.root == nil {
		var zero K
		return zero, errors.New("btree: tree is empty")
	}
	k, _ := tr.minKey(tr.root)
	return k, nil
}

// MaxKey returns the largest key in the tree.
// Returns an error if the tree is empty.
func (tr *BTree[K, V]) MaxKey() (K, error) {
	if tr.root == nil {
		var zero K
		return zero, errors.New("btree: tree is empty")
	}
	k, _ := tr.maxKey(tr.root)
	return k, nil
}

// ---------------------------------------------------------------------------
// Traversals
// ---------------------------------------------------------------------------

// Inorder returns all key-value pairs in ascending key order.
//
// An in-order traversal of a B-tree visits the leftmost subtree, then the
// first key, then the next subtree, etc. — giving sorted order.
//
// Time complexity: O(n)
func (tr *BTree[K, V]) Inorder() []KeyValue[K, V] {
	var result []KeyValue[K, V]
	tr.inorderNode(tr.root, &result)
	return result
}

func (tr *BTree[K, V]) inorderNode(node *bTreeNode[K, V], result *[]KeyValue[K, V]) {
	if node == nil {
		return
	}
	for i := range node.keys {
		if !node.isLeaf {
			tr.inorderNode(node.children[i], result)
		}
		*result = append(*result, KeyValue[K, V]{Key: node.keys[i], Value: node.values[i]})
	}
	if !node.isLeaf {
		tr.inorderNode(node.children[len(node.keys)], result)
	}
}

// RangeQuery returns all key-value pairs where low ≤ key ≤ high, in order.
//
// Traverses only the parts of the tree where keys might be in range.
//
// Example:
//
//	pairs := tree.RangeQuery(10, 30) // returns keys 10, 15, 20, 25, 30
func (tr *BTree[K, V]) RangeQuery(low, high K) []KeyValue[K, V] {
	var result []KeyValue[K, V]
	tr.rangeNode(tr.root, low, high, &result)
	return result
}

func (tr *BTree[K, V]) rangeNode(node *bTreeNode[K, V], low, high K, result *[]KeyValue[K, V]) {
	if node == nil {
		return
	}
	// An in-order traversal that skips subtrees entirely out of range.
	//
	// For each position i in [0, len(keys)):
	//   children[i] contains all keys < keys[i]
	//   keys[i] is the separator
	//   children[i+1] contains all keys > keys[i]  (handled in next iteration)
	//
	// We recurse into children[i] before emitting keys[i], mirroring the
	// standard in-order pattern, but skip the subtree if it can't overlap [low,high].
	for i := range node.keys {
		// Recurse into children[i] only if keys[i] > low
		// (meaning children[i] might hold keys >= low).
		// We also skip if high < smallest possible key in children[i], but that
		// requires knowing the subtree bounds; the simpler approach is to always
		// recurse left and let the depth stop early when all keys < low.
		if !node.isLeaf {
			// Prune: if keys[i] <= low then ALL keys in children[i] are < low.
			// We still want the subtree at children[i] if low could be there.
			// Actually: children[i] holds keys < keys[i]. If keys[i] <= low, then
			// children[i] only has keys < low → nothing in range. Skip it.
			// But: keys[i] == low means children[i] has keys < low → skip.
			// And: keys[i] > low means children[i] might have keys in [low, keys[i]) → recurse.
			if tr.less(low, node.keys[i]) || tr.equal(low, node.keys[i]) {
				tr.rangeNode(node.children[i], low, high, result)
			}
		}
		// Emit keys[i] if it falls in [low, high].
		if !tr.less(node.keys[i], low) && !tr.less(high, node.keys[i]) {
			*result = append(*result, KeyValue[K, V]{Key: node.keys[i], Value: node.values[i]})
		}
		// Stop entirely if we've passed high.
		if tr.less(high, node.keys[i]) {
			return
		}
	}
	// Rightmost child: holds all keys > last key. Recurse if not prunable.
	if !node.isLeaf {
		tr.rangeNode(node.children[len(node.keys)], low, high, result)
	}
}

// ---------------------------------------------------------------------------
// Size, Height, Len
// ---------------------------------------------------------------------------

// Len returns the number of key-value pairs in the tree.
func (tr *BTree[K, V]) Len() int {
	return tr.size
}

// Height returns the height of the tree (number of levels minus one).
// An empty tree has height -1.  A tree with only a root leaf has height 0.
func (tr *BTree[K, V]) Height() int {
	if tr.root == nil {
		return -1
	}
	h := 0
	node := tr.root
	for !node.isLeaf {
		h++
		node = node.children[0]
	}
	return h
}

// ---------------------------------------------------------------------------
// IsValid — structural invariant checker
// ---------------------------------------------------------------------------

// IsValid checks all B-tree invariants and returns true if the tree is
// structurally correct.  Useful for testing and debugging.
//
// Checks:
//  1. Root has at most 2t-1 keys.
//  2. Root has at least 1 key (if non-empty, non-leaf).
//  3. Every non-root node has t-1 ≤ len(keys) ≤ 2t-1.
//  4. Keys within each node are sorted.
//  5. All leaves are at the same depth.
//  6. Internal nodes have exactly len(keys)+1 children.
//  7. Total key count matches tr.size.
func (tr *BTree[K, V]) IsValid() bool {
	if tr.root == nil {
		return tr.size == 0
	}
	if len(tr.root.keys) == 0 {
		return tr.size == 0
	}

	var counted int
	leafDepth := -1
	ok := tr.validateNode(tr.root, true, 0, &leafDepth, &counted)
	if !ok {
		return false
	}
	return counted == tr.size
}

func (tr *BTree[K, V]) validateNode(node *bTreeNode[K, V], isRoot bool, depth int, leafDepth *int, counted *int) bool {
	t := tr.t
	n := len(node.keys)

	// Check key count bounds.
	if isRoot {
		if n > 2*t-1 {
			return false
		}
	} else {
		if n < t-1 || n > 2*t-1 {
			return false
		}
	}

	// Check keys are sorted within this node.
	for i := 1; i < n; i++ {
		if !tr.less(node.keys[i-1], node.keys[i]) {
			return false
		}
	}

	if node.isLeaf {
		// All leaves must be at the same depth.
		if *leafDepth == -1 {
			*leafDepth = depth
		} else if *leafDepth != depth {
			return false
		}
		*counted += n
		return true
	}

	// Internal node must have exactly n+1 children.
	if len(node.children) != n+1 {
		return false
	}

	// Recurse into children.
	for _, child := range node.children {
		if !tr.validateNode(child, false, depth+1, leafDepth, counted) {
			return false
		}
	}
	*counted += n
	return true
}
