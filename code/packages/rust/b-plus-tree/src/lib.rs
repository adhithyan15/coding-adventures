//! # B+ Tree (DT12)
//!
//! A B+ tree is a variant of the B-tree invented in the 1970s (attributed to
//! D. E. Knuth, Rudolf Bayer, and others) that has become the de-facto index
//! structure for relational databases and filesystems.
//!
//! ## How it differs from a plain B-tree
//!
//! ```text
//!                   ┌─────────────────────────────────────┐
//!                   │  B-tree vs B+ tree                  │
//!                   ├─────────────────┬───────────────────┤
//!                   │ B-tree          │ B+ tree           │
//!                   ├─────────────────┼───────────────────┤
//!                   │ Values at every │ Values ONLY in    │
//!                   │ level           │ leaf nodes        │
//!                   ├─────────────────┼───────────────────┤
//!                   │ No leaf linking │ Leaves form a     │
//!                   │                 │ sorted linked list│
//!                   ├─────────────────┼───────────────────┤
//!                   │ Range query:    │ Range query:      │
//!                   │ complex in-     │ walk leaf list    │
//!                   │ order walk      │ from start point  │
//!                   └─────────────────┴───────────────────┘
//! ```
//!
//! In a B+ tree:
//!
//! - **Internal nodes** hold only *separator keys* and *child pointers*.  They
//!   never hold user values.
//! - **Leaf nodes** hold both keys and their associated values.  The separator
//!   key in an internal node is a *copy* of the smallest key in the right
//!   subtree.
//! - **Leaf linked list**: every leaf has a `next` pointer to the next leaf,
//!   forming a sorted singly-linked list.  This makes sequential scans and
//!   range queries O(k) (no tree traversal needed after finding the start).
//!
//! ## Structural diagram
//!
//! ```text
//!              [30 | 60]            ← internal node (only separators)
//!             /    |    \
//!      [10|20]  [40|50]  [70|80]   ← internal nodes
//!      / | \    / | \    / | \
//!  L1  L2  L3  L4  L5  L6  L7    ← leaf nodes  L1→L2→L3→L4→L5→L6→L7→∅
//!                                                (linked list)
//! ```
//!
//! ## Safety note on raw pointers
//!
//! The `BPlusLeaf::next` field is a `*mut BPlusLeaf<K,V>` raw pointer.
//!
//! **Safety invariant**: Every leaf node is owned (transitively) by the
//! `BPlusTree` through the `root` field.  Raw pointers in `next` chains point
//! only to leaves that are owned by the same `BPlusTree`.  The `BPlusTree`
//! never transfers ownership of a leaf outside itself (no `Arc`, no `Rc`,
//! no `Box` escape) and never deallocates a leaf except through the tree's
//! own drop logic.  Therefore, as long as a caller holds a shared reference
//! `&BPlusTree`, all leaf raw pointers are valid.  The tree never exposes
//! raw pointers to callers — all iteration uses safe Rust references.
//!
//! This pattern (owner keeps all heap objects alive while raw pointers form
//! an intrusive structure over them) is a classic Rust unsafe pattern and is
//! sound here because:
//!  1. `BPlusTree` is `!Send` and `!Sync` (raw pointer is not Send/Sync).
//!  2. Mutation only happens through `&mut BPlusTree`, which gives exclusive
//!     access — no aliased mutable references can coexist with the raw pointers.
//!  3. Every leaf is heap-allocated via `Box` before its raw pointer is taken,
//!     so pointer stability is guaranteed until the `Box` is dropped.

use std::ptr;

// ---------------------------------------------------------------------------
// Internal node
// ---------------------------------------------------------------------------

/// An internal (non-leaf) node in the B+ tree.
///
/// Internal nodes hold separator keys that route searches.  The `i`-th child
/// subtree contains all keys `< keys[i]`, and the last child contains all
/// keys `≥ keys[last]`.
///
/// More precisely, for a node with `n` keys and `n+1` children:
///
/// ```text
///  children[0]   children[1]   …   children[n]
///      ↑             ↑                  ↑
///  keys < k[0]  k[0] ≤ keys < k[1] … k[n-1] ≤ keys
/// ```
struct BPlusInternal<K, V> {
    keys: Vec<K>,
    children: Vec<Box<BPlusNode<K, V>>>,
}

// ---------------------------------------------------------------------------
// Leaf node
// ---------------------------------------------------------------------------

/// A leaf node in the B+ tree.
///
/// Leaves hold the actual (key, value) data.  They are linked together in
/// sorted order via the `next` raw pointer to allow O(1) start-of-scan and
/// O(k) range traversal.
///
/// # Memory layout
///
/// ```text
///   BPlusLeaf          BPlusLeaf          BPlusLeaf
///  ┌──────────┐       ┌──────────┐       ┌──────────┐
///  │ keys     │       │ keys     │       │ keys     │
///  │ values   │  next │ values   │  next │ values   │  next
///  │ next ────┼──────►│ next ────┼──────►│ next ────┼──────► null
///  └──────────┘       └──────────┘       └──────────┘
/// ```
struct BPlusLeaf<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
    /// Raw pointer to the next leaf in key order.
    ///
    /// # Safety
    ///
    /// This pointer is valid as long as the owning `BPlusTree` is alive.
    /// The tree maintains full ownership of all leaves through `root`.
    /// We never expose this pointer; the tree's own methods dereference it
    /// only while holding `&self` or `&mut self` on the tree.
    next: *mut BPlusLeaf<K, V>,
}

// ---------------------------------------------------------------------------
// Node enum
// ---------------------------------------------------------------------------

/// A node in the B+ tree — either internal or a leaf.
enum BPlusNode<K, V> {
    Internal(BPlusInternal<K, V>),
    Leaf(BPlusLeaf<K, V>),
}

impl<K, V> BPlusNode<K, V> {
    fn as_leaf(&self) -> &BPlusLeaf<K, V> {
        match self {
            BPlusNode::Leaf(l) => l,
            _ => panic!("expected leaf node"),
        }
    }

    fn as_leaf_mut(&mut self) -> &mut BPlusLeaf<K, V> {
        match self {
            BPlusNode::Leaf(l) => l,
            _ => panic!("expected leaf node"),
        }
    }

    fn as_internal(&self) -> &BPlusInternal<K, V> {
        match self {
            BPlusNode::Internal(n) => n,
            _ => panic!("expected internal node"),
        }
    }

    fn as_internal_mut(&mut self) -> &mut BPlusInternal<K, V> {
        match self {
            BPlusNode::Internal(n) => n,
            _ => panic!("expected internal node"),
        }
    }

    fn is_leaf(&self) -> bool {
        matches!(self, BPlusNode::Leaf(_))
    }

    fn key_count(&self) -> usize {
        match self {
            BPlusNode::Leaf(l) => l.keys.len(),
            BPlusNode::Internal(n) => n.keys.len(),
        }
    }

}

// ---------------------------------------------------------------------------
// Insert result
// ---------------------------------------------------------------------------

/// What an insert into a subtree can return to the caller.
enum InsertResult<K, V> {
    /// No split — the tree was merely updated in place (or a new key added).
    /// The bool indicates whether size grew.
    Done(bool),
    /// The node split.  The caller must insert `(sep_key, right_child)` into
    /// its own key/child arrays.
    Split {
        sep_key: K,
        right_child: Box<BPlusNode<K, V>>,
        grew: bool,
    },
}

// ---------------------------------------------------------------------------
// Delete result
// ---------------------------------------------------------------------------

enum DeleteResult {
    /// Key was not found.
    NotFound,
    /// Key was found and removed.  The bool is true if the node is now
    /// under-full and the caller may need to fix it.
    Removed { underfull: bool },
}

// ---------------------------------------------------------------------------
// Public B+ Tree
// ---------------------------------------------------------------------------

/// A fully-featured B+ tree with minimum degree `t`.
///
/// # Difference from `BTree`
///
/// - All values are stored only in leaf nodes.
/// - Internal nodes contain copies of separator keys for routing.
/// - Leaves are linked together forming a sorted linked list, enabling O(k)
///   range scans starting from any point.
///
/// # Example
///
/// ```
/// use coding_adventures_b_plus_tree::BPlusTree;
///
/// let mut tree: BPlusTree<i32, &str> = BPlusTree::new(2);
/// tree.insert(10, "ten");
/// tree.insert(5, "five");
/// tree.insert(20, "twenty");
///
/// assert_eq!(tree.search(&10), Some(&"ten"));
///
/// let scan = tree.range_scan(&5, &15);
/// let keys: Vec<i32> = scan.iter().map(|(k, _)| **k).collect();
/// assert_eq!(keys, vec![5, 10]);
///
/// assert!(tree.is_valid());
/// ```
pub struct BPlusTree<K: Ord + Clone, V: Clone> {
    root: Box<BPlusNode<K, V>>,
    /// Raw pointer to the first (leftmost) leaf, for O(1) full-scan start.
    ///
    /// # Safety
    ///
    /// Valid as long as `self` is alive; always points to a leaf owned by
    /// `root` (transitively).
    first_leaf: *mut BPlusLeaf<K, V>,
    /// Minimum degree.  Every non-root node has between t-1 and 2t-1 keys.
    t: usize,
    size: usize,
}

// SAFETY: BPlusTree owns all nodes via Box.  The raw pointers are only
// ever accessed through &self / &mut self.  We implement Send/Sync manually
// so the type can be used in typical single-threaded contexts.
// However, we conservatively do NOT implement Send+Sync because the raw
// pointer aliases into Box-owned data.  The standard library's Vec/Box are
// Send+Sync when K+V are, but we keep the conservative approach here since
// we hold raw pointers.

impl<K: Ord + Clone, V: Clone> BPlusTree<K, V> {
    /// Create a new empty B+ tree with minimum degree `t` (clamped to ≥ 2).
    pub fn new(t: usize) -> Self {
        let t = t.max(2);
        // Start with a single empty leaf.
        let mut leaf = Box::new(BPlusNode::Leaf(BPlusLeaf {
            keys: Vec::new(),
            values: Vec::new(),
            next: ptr::null_mut(),
        }));
        let first_leaf: *mut BPlusLeaf<K, V> = leaf.as_leaf_mut() as *mut BPlusLeaf<K, V>;
        BPlusTree {
            root: leaf,
            first_leaf,
            t,
            size: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    /// Find the leaf that would contain `key` and search it.
    pub fn search(&self, key: &K) -> Option<&V> {
        let leaf = self.find_leaf(key);
        // SAFETY: leaf pointer is valid (owned by root).
        let leaf = unsafe { &*leaf };
        match leaf.keys.binary_search_by(|k| k.cmp(key)) {
            Ok(i) => Some(&leaf.values[i]),
            Err(_) => None,
        }
    }

    /// Walk internal nodes to find the leaf that should contain `key`.
    fn find_leaf(&self, key: &K) -> *mut BPlusLeaf<K, V> {
        let mut node: *const BPlusNode<K, V> = &*self.root;
        loop {
            // SAFETY: node is always a valid pointer owned by root.
            match unsafe { &*node } {
                BPlusNode::Leaf(l) => {
                    return l as *const _ as *mut _;
                }
                BPlusNode::Internal(n) => {
                    // find_leaf: separator key is the smallest key in the
                    // right subtree.  We want the first child whose subtree
                    // could contain key.  `partition_point(|k| k <= key)`
                    // gives the number of separators ≤ key, which equals the
                    // child index to follow (clamped to the last child).
                    let ci = n.keys.partition_point(|k| k <= key);
                    let ci = ci.min(n.children.len() - 1);
                    node = &*n.children[ci];
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Insert
    // -----------------------------------------------------------------------

    /// Insert `(key, value)` into the tree.
    ///
    /// If `key` already exists, its value is updated (size unchanged).
    ///
    /// # Leaf split rule
    ///
    /// When a leaf overflows (reaches `2t` keys after insert), it splits:
    ///
    /// ```text
    /// Full leaf: [k0 | k1 | … | k_{2t-1}]   (2t keys after insert)
    ///
    /// Left leaf:  [k0 … k_{t-1}]
    /// Right leaf: [k_t … k_{2t-1}]
    ///
    /// Separator promoted to parent: k_t   (copy of right leaf's FIRST key)
    /// ```
    ///
    /// Note: the separator stays in the right leaf — it is only *copied*
    /// to the parent, not moved.  This means all values live in leaves.
    pub fn insert(&mut self, key: K, value: V) {
        let t = self.t;
        let grew;
        let maybe_split = Self::insert_node(&mut self.root, key, value, t);
        match maybe_split {
            InsertResult::Done(g) => {
                grew = g;
            }
            InsertResult::Split {
                sep_key,
                right_child,
                grew: g,
            } => {
                grew = g;
                // Root split — create a new root.
                let old_root =
                    std::mem::replace(&mut self.root, Box::new(BPlusNode::Internal(BPlusInternal {
                        keys: vec![sep_key],
                        children: Vec::new(),
                    })));
                let new_root = self.root.as_internal_mut();
                new_root.children.push(old_root);
                new_root.children.push(right_child);
            }
        }
        if grew {
            self.size += 1;
        }
        // Update first_leaf in case the leftmost leaf changed.
        self.first_leaf = Self::leftmost_leaf_ptr(&mut self.root);
    }

    /// Recursively insert into a subtree rooted at `node`.
    fn insert_node(node: &mut Box<BPlusNode<K, V>>, key: K, value: V, t: usize) -> InsertResult<K, V> {
        if node.is_leaf() {
            Self::insert_leaf(node, key, value, t)
        } else {
            Self::insert_internal(node, key, value, t)
        }
    }

    fn insert_leaf(node: &mut Box<BPlusNode<K, V>>, key: K, value: V, t: usize) -> InsertResult<K, V> {
        let leaf = node.as_leaf_mut();
        let pos = leaf.keys.binary_search_by(|k| k.cmp(&key));
        match pos {
            Ok(i) => {
                // Key already exists — update value.
                leaf.values[i] = value;
                InsertResult::Done(false)
            }
            Err(i) => {
                leaf.keys.insert(i, key);
                leaf.values.insert(i, value);
                // Check if we need to split (leaf now has 2t keys).
                if leaf.keys.len() >= 2 * t {
                    let split_point = t; // right leaf starts at index t
                    let right_keys: Vec<K> = leaf.keys.drain(split_point..).collect();
                    let right_values: Vec<V> = leaf.values.drain(split_point..).collect();
                    let sep_key = right_keys[0].clone(); // separator = first key of right leaf

                    // Build the right leaf and link it into the chain.
                    let old_next = leaf.next;
                    let mut right_leaf = Box::new(BPlusNode::Leaf(BPlusLeaf {
                        keys: right_keys,
                        values: right_values,
                        next: old_next,
                    }));
                    // Point left leaf's next to the new right leaf.
                    let right_ptr: *mut BPlusLeaf<K, V> =
                        right_leaf.as_leaf_mut() as *mut BPlusLeaf<K, V>;
                    node.as_leaf_mut().next = right_ptr;

                    InsertResult::Split {
                        sep_key,
                        right_child: right_leaf,
                        grew: true,
                    }
                } else {
                    InsertResult::Done(true)
                }
            }
        }
    }

    fn insert_internal(node: &mut Box<BPlusNode<K, V>>, key: K, value: V, t: usize) -> InsertResult<K, V> {
        let ci = {
            let internal = node.as_internal();
            let ci = internal.keys.partition_point(|k| k <= &key);
            ci.min(internal.children.len() - 1)
        };

        let result = Self::insert_node(&mut node.as_internal_mut().children[ci], key, value, t);

        match result {
            InsertResult::Done(grew) => InsertResult::Done(grew),
            InsertResult::Split { sep_key, right_child, grew } => {
                // Insert sep_key and right_child into this internal node.
                let internal = node.as_internal_mut();
                // Find where to insert the separator.
                let pos = internal.keys.partition_point(|k| k < &sep_key);
                internal.keys.insert(pos, sep_key);
                internal.children.insert(pos + 1, right_child);

                // Check if this internal node needs splitting too.
                if internal.keys.len() >= 2 * t {
                    let mid = t - 1; // index of separator to promote
                    let promote_key = internal.keys[mid].clone();
                    let right_keys: Vec<K> = internal.keys.drain(mid..).collect();
                    let right_keys_trimmed: Vec<K> = right_keys.into_iter().skip(1).collect(); // remove promoted key
                    let right_children: Vec<Box<BPlusNode<K, V>>> =
                        internal.children.drain(mid + 1..).collect();
                    // Truncate left node's key list.
                    // After drain, left has 0..mid keys already.

                    let right_internal = Box::new(BPlusNode::Internal(BPlusInternal {
                        keys: right_keys_trimmed,
                        children: right_children,
                    }));
                    InsertResult::Split {
                        sep_key: promote_key,
                        right_child: right_internal,
                        grew,
                    }
                } else {
                    InsertResult::Done(grew)
                }
            }
        }
    }

    /// Walk to the leftmost leaf and return its raw pointer.
    fn leftmost_leaf_ptr(node: &mut Box<BPlusNode<K, V>>) -> *mut BPlusLeaf<K, V> {
        match &mut **node {
            BPlusNode::Leaf(l) => l as *mut _,
            BPlusNode::Internal(n) => Self::leftmost_leaf_ptr(&mut n.children[0]),
        }
    }

    // -----------------------------------------------------------------------
    // Delete
    // -----------------------------------------------------------------------

    /// Delete the entry with `key`.
    ///
    /// Returns `true` if the key was found and removed.
    ///
    /// # Algorithm
    ///
    /// We use a recursive top-down approach.  After removing a key from a
    /// leaf, if the leaf is underfull (< t-1 keys), we try to borrow from a
    /// sibling; if that fails, we merge.  Internal nodes are fixed after
    /// returning from recursion (bottom-up rebalancing).
    pub fn delete(&mut self, key: &K) -> bool {
        let t = self.t;
        let result = Self::delete_node(&mut self.root, key, t, true);
        match result {
            DeleteResult::NotFound => false,
            DeleteResult::Removed { .. } => {
                // If root is an internal node with no keys (after a merge that
                // reduced it), shrink the tree.
                if !self.root.is_leaf() && self.root.as_internal().keys.is_empty() {
                    let old_root = std::mem::replace(
                        &mut self.root,
                        // Temporary placeholder; immediately overwritten.
                        Box::new(BPlusNode::Leaf(BPlusLeaf {
                            keys: Vec::new(),
                            values: Vec::new(),
                            next: ptr::null_mut(),
                        })),
                    );
                    let mut children = match *old_root {
                        BPlusNode::Internal(n) => n.children,
                        _ => unreachable!(),
                    };
                    self.root = children.remove(0);
                }
                self.size -= 1;
                self.first_leaf = Self::leftmost_leaf_ptr(&mut self.root);
                true
            }
        }
    }

    fn delete_node(node: &mut Box<BPlusNode<K, V>>, key: &K, t: usize, is_root: bool) -> DeleteResult {
        if node.is_leaf() {
            let leaf = node.as_leaf_mut();
            match leaf.keys.binary_search_by(|k| k.cmp(key)) {
                Ok(i) => {
                    leaf.keys.remove(i);
                    leaf.values.remove(i);
                    let min_keys = if is_root { 0 } else { t - 1 };
                    let underfull = leaf.keys.len() < min_keys;
                    DeleteResult::Removed { underfull }
                }
                Err(_) => DeleteResult::NotFound,
            }
        } else {
            let ci = {
                let internal = node.as_internal();
                let ci = internal.keys.partition_point(|k| k <= key);
                ci.min(internal.children.len() - 1)
            };

            let result = Self::delete_node(&mut node.as_internal_mut().children[ci], key, t, false);

            match result {
                DeleteResult::NotFound => DeleteResult::NotFound,
                DeleteResult::Removed { underfull } => {
                    if underfull {
                        Self::fix_underfull(node, ci, t);
                    } else {
                        // Update separator key if we deleted from the leftmost
                        // position of the child.
                        Self::maybe_update_separator(node, ci);
                    }
                    let min_keys = if is_root { 0 } else { t - 1 };
                    let underfull_now = node.as_internal().keys.len() < min_keys;
                    DeleteResult::Removed { underfull: underfull_now }
                }
            }
        }
    }

    /// After a delete, if a separator key in the parent became stale (because
    /// the deleted key was the smallest in its subtree), update it.
    fn maybe_update_separator(node: &mut Box<BPlusNode<K, V>>, ci: usize) {
        let internal = node.as_internal_mut();
        if ci > 0 && ci <= internal.keys.len() {
            // The separator internal.keys[ci-1] should equal the smallest key
            // in children[ci].  Refresh it.
            if let Some(new_sep) = Self::leftmost_key(&internal.children[ci]) {
                internal.keys[ci - 1] = new_sep;
            }
        }
    }

    fn leftmost_key(node: &Box<BPlusNode<K, V>>) -> Option<K> {
        match node.as_ref() {
            BPlusNode::Leaf(l) => l.keys.first().cloned(),
            BPlusNode::Internal(n) => Self::leftmost_key(&n.children[0]),
        }
    }

    /// Rebalance after `children[ci]` became underfull.
    fn fix_underfull(node: &mut Box<BPlusNode<K, V>>, ci: usize, t: usize) {
        let n_children = node.as_internal().children.len();

        // Try borrowing from left sibling.
        if ci > 0 && node.as_internal().children[ci - 1].key_count() >= t {
            Self::borrow_from_left_sibling(node, ci);
            return;
        }
        // Try borrowing from right sibling.
        if ci + 1 < n_children && node.as_internal().children[ci + 1].key_count() >= t {
            Self::borrow_from_right_sibling(node, ci);
            return;
        }
        // Merge.
        if ci > 0 {
            Self::merge_with_left(node, ci);
        } else {
            Self::merge_with_right(node, ci);
        }
    }

    fn borrow_from_left_sibling(node: &mut Box<BPlusNode<K, V>>, ci: usize) {
        let internal = node.as_internal_mut();
        if internal.children[ci].is_leaf() {
            // For leaves, take the last key/value from the left sibling and
            // prepend to current leaf.  Update the separator.
            let left = internal.children[ci - 1].as_leaf_mut();
            let borrow_key = left.keys.pop().unwrap();
            let borrow_val = left.values.pop().unwrap();

            let right = internal.children[ci].as_leaf_mut();
            right.keys.insert(0, borrow_key);
            right.values.insert(0, borrow_val);

            // Separator (ci-1) = new first key of right child.
            internal.keys[ci - 1] = right.keys[0].clone();
        } else {
            // For internal nodes, rotate via the separator.
            let sep = internal.keys[ci - 1].clone();
            let left_last_child = internal.children[ci - 1].as_internal_mut().children.pop().unwrap();
            let left_last_key = internal.children[ci - 1].as_internal_mut().keys.pop().unwrap();

            internal.keys[ci - 1] = left_last_key;

            let right = internal.children[ci].as_internal_mut();
            right.keys.insert(0, sep);
            right.children.insert(0, left_last_child);
        }
    }

    fn borrow_from_right_sibling(node: &mut Box<BPlusNode<K, V>>, ci: usize) {
        let internal = node.as_internal_mut();
        if internal.children[ci].is_leaf() {
            let right = internal.children[ci + 1].as_leaf_mut();
            let borrow_key = right.keys.remove(0);
            let borrow_val = right.values.remove(0);

            let left = internal.children[ci].as_leaf_mut();
            left.keys.push(borrow_key);
            left.values.push(borrow_val);

            // Separator (ci) = new first key of right child (children[ci+1]).
            internal.keys[ci] = internal.children[ci + 1].as_leaf().keys[0].clone();
        } else {
            let sep = internal.keys[ci].clone();
            let right_first_key = internal.children[ci + 1].as_internal_mut().keys.remove(0);
            let right_first_child = internal.children[ci + 1].as_internal_mut().children.remove(0);

            internal.keys[ci] = right_first_key;

            let left = internal.children[ci].as_internal_mut();
            left.keys.push(sep);
            left.children.push(right_first_child);
        }
    }

    fn merge_with_left(node: &mut Box<BPlusNode<K, V>>, ci: usize) {
        // Merge children[ci-1] (left) and children[ci] (right = underfull).
        let internal = node.as_internal_mut();
        let right = internal.children.remove(ci);
        let sep_key = internal.keys.remove(ci - 1);
        let left = &mut internal.children[ci - 1];

        match (*right, left.as_mut()) {
            (BPlusNode::Leaf(mut r_leaf), BPlusNode::Leaf(l_leaf)) => {
                l_leaf.keys.append(&mut r_leaf.keys);
                l_leaf.values.append(&mut r_leaf.values);
                l_leaf.next = r_leaf.next; // skip the merged-away right leaf
            }
            (BPlusNode::Internal(mut r_int), BPlusNode::Internal(l_int)) => {
                l_int.keys.push(sep_key);
                l_int.keys.append(&mut r_int.keys);
                l_int.children.append(&mut r_int.children);
            }
            _ => panic!("merge type mismatch"),
        }
    }

    fn merge_with_right(node: &mut Box<BPlusNode<K, V>>, ci: usize) {
        // Merge children[ci] (left = underfull) and children[ci+1] (right).
        let internal = node.as_internal_mut();
        let right = internal.children.remove(ci + 1);
        let sep_key = internal.keys.remove(ci);
        let left = &mut internal.children[ci];

        match (*right, left.as_mut()) {
            (BPlusNode::Leaf(mut r_leaf), BPlusNode::Leaf(l_leaf)) => {
                l_leaf.keys.append(&mut r_leaf.keys);
                l_leaf.values.append(&mut r_leaf.values);
                l_leaf.next = r_leaf.next;
            }
            (BPlusNode::Internal(mut r_int), BPlusNode::Internal(l_int)) => {
                l_int.keys.push(sep_key);
                l_int.keys.append(&mut r_int.keys);
                l_int.children.append(&mut r_int.children);
            }
            _ => panic!("merge type mismatch"),
        }
    }

    // -----------------------------------------------------------------------
    // Scans and queries
    // -----------------------------------------------------------------------

    /// Return all `(key, value)` pairs with `low <= key <= high`, in sorted
    /// order.
    ///
    /// Uses the leaf linked list for efficient traversal after finding the
    /// start leaf.
    pub fn range_scan<'a>(&'a self, low: &K, high: &K) -> Vec<(&'a K, &'a V)> {
        let mut out = Vec::new();
        // Find the first leaf that might contain `low`.
        let leaf_ptr = self.find_leaf(low);
        let mut cur: *const BPlusLeaf<K, V> = leaf_ptr;
        loop {
            if cur.is_null() {
                break;
            }
            // SAFETY: cur is valid (leaf is owned by root, and linked list
            // pointers are always kept in sync with the ownership chain).
            let leaf = unsafe { &*cur };
            let mut done = true;
            for (k, v) in leaf.keys.iter().zip(leaf.values.iter()) {
                if k > high {
                    break;
                }
                if k >= low {
                    out.push((k, v));
                    done = false;
                }
            }
            // If all keys in this leaf were < low, continue to next leaf.
            if done && !out.is_empty() {
                break;
            }
            cur = leaf.next;
        }
        out
    }

    /// Return all `(key, value)` pairs in sorted order by walking the leaf
    /// linked list from `first_leaf`.
    ///
    /// This is O(n) and requires no tree traversal beyond the first leaf,
    /// making it ideal for full table scans.
    pub fn full_scan<'a>(&'a self) -> Vec<(&'a K, &'a V)> {
        let mut out = Vec::new();
        let mut cur: *const BPlusLeaf<K, V> = self.first_leaf;
        loop {
            if cur.is_null() {
                break;
            }
            // SAFETY: same invariant as in range_scan.
            let leaf = unsafe { &*cur };
            for (k, v) in leaf.keys.iter().zip(leaf.values.iter()) {
                out.push((k, v));
            }
            cur = leaf.next;
        }
        out
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Return `true` if `key` is in the tree.
    pub fn contains(&self, key: &K) -> bool {
        self.search(key).is_some()
    }

    /// Return a reference to the smallest key, or `None` if empty.
    pub fn min_key(&self) -> Option<&K> {
        // SAFETY: first_leaf is always valid if size > 0.
        if self.size == 0 {
            return None;
        }
        let leaf = unsafe { &*self.first_leaf };
        leaf.keys.first()
    }

    /// Return a reference to the largest key, or `None` if empty.
    pub fn max_key(&self) -> Option<&K> {
        if self.size == 0 {
            return None;
        }
        Self::rightmost_key(&self.root)
    }

    fn rightmost_key(node: &Box<BPlusNode<K, V>>) -> Option<&K> {
        match node.as_ref() {
            BPlusNode::Leaf(l) => l.keys.last(),
            BPlusNode::Internal(n) => Self::rightmost_key(n.children.last()?),
        }
    }

    /// Return the number of entries.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Return `true` if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Return the height of the tree (0 = root is a leaf).
    pub fn height(&self) -> usize {
        Self::node_height(&self.root)
    }

    fn node_height(node: &Box<BPlusNode<K, V>>) -> usize {
        match node.as_ref() {
            BPlusNode::Leaf(_) => 0,
            BPlusNode::Internal(n) => 1 + Self::node_height(&n.children[0]),
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate all B+ tree structural invariants.
    ///
    /// Checks:
    ///  1. All leaves are at the same depth.
    ///  2. Key counts are within bounds for every node.
    ///  3. Keys within each node are sorted.
    ///  4. Internal node children count = keys.len() + 1.
    ///  5. Leaf linked list contains all keys exactly once in sorted order.
    ///  6. `size` matches the actual key count.
    pub fn is_valid(&self) -> bool {
        // Structural tree check.
        let (_leaf_depth, tree_ok, tree_count) = Self::validate_node(&self.root, self.t, true, 0);
        if !tree_ok {
            return false;
        }

        // Verify size counter.
        if tree_count != self.size {
            return false;
        }

        // Verify leaf linked list.
        let mut list_count = 0usize;
        let mut prev_key: Option<K> = None;
        let mut cur: *const BPlusLeaf<K, V> = self.first_leaf;
        loop {
            if cur.is_null() {
                break;
            }
            // SAFETY: leaf pointers are valid.
            let leaf = unsafe { &*cur };
            for k in &leaf.keys {
                if let Some(ref pk) = prev_key {
                    if pk >= k {
                        return false; // not strictly sorted
                    }
                }
                prev_key = Some(k.clone());
                list_count += 1;
            }
            cur = leaf.next;
        }
        list_count == self.size
    }

    fn validate_node(
        node: &Box<BPlusNode<K, V>>,
        t: usize,
        is_root: bool,
        depth: usize,
    ) -> (usize, bool, usize) {
        let min_keys = if is_root { 0 } else { t - 1 };
        let max_keys = 2 * t - 1;

        match node.as_ref() {
            BPlusNode::Leaf(l) => {
                if l.keys.len() > max_keys {
                    return (depth, false, 0);
                }
                if !is_root && l.keys.len() < min_keys {
                    return (depth, false, 0);
                }
                // Keys sorted.
                for i in 1..l.keys.len() {
                    if l.keys[i - 1] >= l.keys[i] {
                        return (depth, false, 0);
                    }
                }
                (depth, true, l.keys.len())
            }
            BPlusNode::Internal(n) => {
                if n.keys.len() > max_keys {
                    return (depth, false, 0);
                }
                if !is_root && n.keys.len() < min_keys {
                    return (depth, false, 0);
                }
                if n.children.len() != n.keys.len() + 1 {
                    return (depth, false, 0);
                }
                // Keys sorted.
                for i in 1..n.keys.len() {
                    if n.keys[i - 1] >= n.keys[i] {
                        return (depth, false, 0);
                    }
                }
                let mut leaf_depth: Option<usize> = None;
                let mut total = 0;
                for child in &n.children {
                    let (d, ok, cnt) = Self::validate_node(child, t, false, depth + 1);
                    if !ok {
                        return (depth, false, 0);
                    }
                    match leaf_depth {
                        None => leaf_depth = Some(d),
                        Some(ld) => {
                            if ld != d {
                                return (depth, false, 0);
                            }
                        }
                    }
                    total += cnt;
                }
                (leaf_depth.unwrap_or(depth), true, total)
            }
        }
    }

    // -----------------------------------------------------------------------
    // Iterator support
    // -----------------------------------------------------------------------

    /// Return an iterator over `(key, value)` pairs in sorted order.
    ///
    /// Implemented by walking the leaf linked list.
    pub fn iter(&self) -> BPlusTreeIter<'_, K, V> {
        let first_leaf = self.first_leaf as *const BPlusLeaf<K, V>;
        let leaf_ref = if first_leaf.is_null() {
            None
        } else {
            // SAFETY: first_leaf is valid.
            Some(unsafe { &*first_leaf })
        };
        BPlusTreeIter {
            current_leaf: leaf_ref,
            key_index: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Iterator
// ---------------------------------------------------------------------------

/// An iterator over `(&K, &V)` pairs of a `BPlusTree`, in ascending key order.
///
/// Constructed by `BPlusTree::iter()`.  Walks the leaf linked list, yielding
/// one entry at a time without any tree traversal.
pub struct BPlusTreeIter<'a, K, V> {
    current_leaf: Option<&'a BPlusLeaf<K, V>>,
    key_index: usize,
}

impl<'a, K: Ord + Clone, V: Clone> Iterator for BPlusTreeIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let leaf = self.current_leaf?;
            if self.key_index < leaf.keys.len() {
                let item = (&leaf.keys[self.key_index], &leaf.values[self.key_index]);
                self.key_index += 1;
                return Some(item);
            }
            // Move to the next leaf.
            // SAFETY: leaf.next is valid (same invariant as before).
            self.current_leaf = if leaf.next.is_null() {
                None
            } else {
                Some(unsafe { &*leaf.next })
            };
            self.key_index = 0;
        }
    }
}

// ---------------------------------------------------------------------------
// IntoIterator
// ---------------------------------------------------------------------------

/// Consuming iterator for `BPlusTree` — collects all entries in sorted order.
///
/// Because we cannot trivially walk raw pointers while consuming the tree
/// (ownership is complex with raw pointers), we materialise all entries via
/// `full_scan` first.
pub struct BPlusTreeIntoIter<K, V> {
    entries: std::vec::IntoIter<(K, V)>,
}

impl<K: Ord + Clone, V: Clone> IntoIterator for BPlusTree<K, V> {
    type Item = (K, V);
    type IntoIter = BPlusTreeIntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        // Collect references, then clone (K: Clone, V: Clone).
        let entries: Vec<(K, V)> = self
            .full_scan()
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        BPlusTreeIntoIter {
            entries: entries.into_iter(),
        }
    }
}

impl<K: Ord + Clone, V: Clone> Iterator for BPlusTreeIntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.entries.next()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: collect the leaf linked list and return all keys in order.
    fn leaf_list_keys(tree: &BPlusTree<i32, i32>) -> Vec<i32> {
        let mut out = Vec::new();
        let mut cur = tree.first_leaf as *const BPlusLeaf<i32, i32>;
        loop {
            if cur.is_null() {
                break;
            }
            // SAFETY: test helper, tree is alive.
            let leaf = unsafe { &*cur };
            out.extend_from_slice(&leaf.keys);
            cur = leaf.next;
        }
        out
    }

    // -- Basic ops --

    #[test]
    fn empty_tree() {
        let t: BPlusTree<i32, i32> = BPlusTree::new(2);
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
        assert_eq!(t.height(), 0);
        assert!(t.is_valid());
        assert_eq!(t.min_key(), None);
        assert_eq!(t.max_key(), None);
        assert_eq!(t.search(&1), None);
        assert!(!t.contains(&1));
    }

    #[test]
    fn single_insert_and_search() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        t.insert(42, 420);
        assert_eq!(t.len(), 1);
        assert_eq!(t.search(&42), Some(&420));
        assert!(t.contains(&42));
        assert!(t.is_valid());
        assert_eq!(t.min_key(), Some(&42));
        assert_eq!(t.max_key(), Some(&42));
    }

    #[test]
    fn duplicate_key_updates_value() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        t.insert(5, 50);
        t.insert(5, 500);
        assert_eq!(t.len(), 1);
        assert_eq!(t.search(&5), Some(&500));
        assert!(t.is_valid());
    }

    #[test]
    fn delete_from_empty_returns_false() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        assert!(!t.delete(&1));
    }

    #[test]
    fn delete_missing_key_returns_false() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        t.insert(1, 1);
        assert!(!t.delete(&99));
        assert!(t.is_valid());
    }

    #[test]
    fn delete_only_key() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        t.insert(7, 70);
        assert!(t.delete(&7));
        assert!(t.is_empty());
        assert!(t.is_valid());
    }

    // -- Leaf linked list invariant --

    #[test]
    fn leaf_list_sorted_after_inserts() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in [10, 3, 7, 1, 5, 15, 20] {
            t.insert(i, i);
        }
        let keys = leaf_list_keys(&t);
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "leaf list must be sorted");
        // Each key appears exactly once.
        let unique: std::collections::HashSet<i32> = keys.iter().cloned().collect();
        assert_eq!(unique.len(), keys.len());
        assert!(t.is_valid());
    }

    #[test]
    fn leaf_list_sorted_after_deletes() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in 0..20 {
            t.insert(i, i);
        }
        for i in (0..20).step_by(2) {
            t.delete(&i);
            assert!(t.is_valid(), "invalid after delete {i}");
            let keys = leaf_list_keys(&t);
            let mut sorted = keys.clone();
            sorted.sort();
            assert_eq!(keys, sorted, "leaf list not sorted after delete {i}");
        }
    }

    #[test]
    fn leaf_list_every_key_once_after_mixed_ops() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(3);
        for i in 0..50 {
            t.insert(i, i * 2);
        }
        for i in (0..50).step_by(3) {
            t.delete(&i);
        }
        let keys = leaf_list_keys(&t);
        let mut expected: Vec<i32> = (0..50).filter(|i| i % 3 != 0).collect();
        expected.sort();
        assert_eq!(keys, expected);
        assert!(t.is_valid());
    }

    // -- Full scan --

    #[test]
    fn full_scan_returns_sorted_entries() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in [5, 2, 8, 1, 9, 3, 7] {
            t.insert(i, i * 10);
        }
        let scan: Vec<i32> = t.full_scan().iter().map(|(k, _)| **k).collect();
        assert_eq!(scan, vec![1, 2, 3, 5, 7, 8, 9]);
    }

    #[test]
    fn full_scan_empty_tree() {
        let t: BPlusTree<i32, i32> = BPlusTree::new(2);
        assert_eq!(t.full_scan(), vec![]);
    }

    // -- Range scan --

    #[test]
    fn range_scan_basic() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in 1..=10 {
            t.insert(i, i * 100);
        }
        let r = t.range_scan(&3, &7);
        let keys: Vec<i32> = r.iter().map(|(k, _)| **k).collect();
        assert_eq!(keys, vec![3, 4, 5, 6, 7]);
    }

    #[test]
    fn range_scan_empty_result() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in [1, 5, 10] {
            t.insert(i, i);
        }
        let r = t.range_scan(&6, &9);
        assert!(r.is_empty());
    }

    #[test]
    fn range_scan_full_range() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(3);
        for i in 0..20 {
            t.insert(i, i);
        }
        let r = t.range_scan(&0, &19);
        assert_eq!(r.len(), 20);
    }

    // -- Iterator and IntoIterator --

    #[test]
    fn iter_sorted() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in [30, 10, 20, 40] {
            t.insert(i, i);
        }
        let collected: Vec<i32> = t.iter().map(|(k, _)| *k).collect();
        assert_eq!(collected, vec![10, 20, 30, 40]);
    }

    #[test]
    fn into_iter_sorted() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in 0..10 {
            t.insert(i, i);
        }
        let entries: Vec<(i32, i32)> = t.into_iter().collect();
        let keys: Vec<i32> = entries.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, (0..10).collect::<Vec<_>>());
    }

    // -- Different t values --

    #[test]
    fn t2_operations() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in 0..30 {
            t.insert(i, i);
        }
        assert_eq!(t.len(), 30);
        assert!(t.is_valid());
        for i in 0..15 {
            t.delete(&i);
            assert!(t.is_valid(), "invalid after delete {i}");
        }
        assert_eq!(t.len(), 15);
    }

    #[test]
    fn t3_operations() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(3);
        for i in (0..60).rev() {
            t.insert(i, i * 2);
        }
        assert!(t.is_valid());
        assert_eq!(t.search(&30), Some(&60));
        for i in (0..60).step_by(2) {
            t.delete(&i);
        }
        assert!(t.is_valid());
        assert_eq!(t.len(), 30);
    }

    #[test]
    fn t5_operations() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(5);
        for i in 0..100 {
            t.insert(i, i);
        }
        assert!(t.is_valid());
        for i in 50..100 {
            t.delete(&i);
        }
        assert!(t.is_valid());
        assert_eq!(t.len(), 50);
    }

    // -- min/max --

    #[test]
    fn min_max_after_ops() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in [5, 1, 9, 3, 7] {
            t.insert(i, i);
        }
        assert_eq!(t.min_key(), Some(&1));
        assert_eq!(t.max_key(), Some(&9));
        t.delete(&1);
        assert_eq!(t.min_key(), Some(&3));
        t.delete(&9);
        assert_eq!(t.max_key(), Some(&7));
        assert!(t.is_valid());
    }

    // -- is_valid after every op --

    #[test]
    fn is_valid_after_every_insert_and_delete() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        assert!(t.is_valid());
        for i in 0..25 {
            t.insert(i, i);
            assert!(t.is_valid(), "invalid after insert {i}");
        }
        for i in (0..25).step_by(2) {
            t.delete(&i);
            assert!(t.is_valid(), "invalid after delete {i}");
        }
    }

    // -- Large scale --

    #[test]
    fn ten_thousand_keys_t2() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        for i in 0..10_000 {
            t.insert(i, i * 2);
        }
        assert_eq!(t.len(), 10_000);
        assert!(t.is_valid());

        // Verify all keys via search.
        for i in 0..10_000 {
            assert_eq!(t.search(&i), Some(&(i * 2)), "missing {i}");
        }

        // Verify leaf list.
        let keys = leaf_list_keys(&t);
        assert_eq!(keys, (0..10_000).collect::<Vec<_>>());

        // Delete half.
        for i in (0..10_000).step_by(2) {
            t.delete(&i);
        }
        assert_eq!(t.len(), 5_000);
        assert!(t.is_valid());

        let keys = leaf_list_keys(&t);
        let expected: Vec<i32> = (0..10_000).filter(|i| i % 2 != 0).collect();
        assert_eq!(keys, expected);
    }

    #[test]
    fn ten_thousand_keys_t5() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(5);
        for i in (0..10_000).rev() {
            t.insert(i, i);
        }
        assert!(t.is_valid());
        let scan: Vec<i32> = t.full_scan().iter().map(|(k, _)| **k).collect();
        assert_eq!(scan, (0..10_000).collect::<Vec<_>>());
    }

    // -- height grows with size --

    #[test]
    fn height_grows_with_inserts() {
        let mut t: BPlusTree<i32, i32> = BPlusTree::new(2);
        assert_eq!(t.height(), 0);
        for i in 0..10 {
            t.insert(i, i);
        }
        assert!(t.height() >= 1);
        assert!(t.is_valid());
    }
}
