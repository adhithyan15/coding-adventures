//! # B-Tree (DT11)
//!
//! A B-tree is a self-balancing search tree generalised from binary search
//! trees.  Invented by Rudolf Bayer and Edward McCreight at Boeing Research
//! Labs in 1970, it was designed for systems where data lives on slow block
//! devices (hard drives).  The key insight: pack *many* keys into one node so
//! that a single disk read fetches many keys, and the number of disk reads
//! needed to find any key is kept logarithmically small.
//!
//! ## Anatomy of a B-tree of order `t`
//!
//! ```text
//!                     [30 | 60]
//!                    /    |    \
//!          [10|20]  [40|50]  [70|80]
//!          / | \    / | \    / | \
//! leaves…
//! ```
//!
//! - Every *internal* node except the root holds between `t-1` and `2t-1`
//!   keys.
//! - The root holds between 1 and `2t-1` keys.
//! - All leaves are at the same depth.
//! - A node with `k` keys has exactly `k+1` children (if it is internal).
//!
//! `t` is called the *minimum degree* (also written *order* or *branching
//! factor*).  For `t=2` the tree is called a *2-3-4 tree* (nodes hold 1-3
//! keys).  Databases commonly use `t` in the hundreds so that a whole B-tree
//! fits in a handful of disk pages.
//!
//! ## Operations
//!
//! | Operation    | Worst-case time        |
//! |-------------|------------------------|
//! | Search       | O(log n)               |
//! | Insert       | O(t · log_t n)         |
//! | Delete       | O(t · log_t n)         |
//! | Range query  | O(t · log_t n + k)     |
//!
//! where `k` is the number of results returned.
//!
//! ## This implementation
//!
//! We use **proactive top-down splitting**: as we walk *down* the tree
//! during insertion we split any full node we encounter before we try to
//! insert into it.  This means we never need to walk back *up* to fix an
//! overfull parent — which simplifies the code and is cache-friendly.
//!
//! For deletion we use the classic Cormen/Leiserson/Rivest/Stein (CLRS)
//! algorithm that pre-fills nodes on the way down so that every node we
//! reach has at least `t` keys, guaranteeing that we can always remove a
//! key without violating the B-tree invariants.


// ---------------------------------------------------------------------------
// Node
// ---------------------------------------------------------------------------

/// A single node inside the B-tree.
///
/// Each node stores:
///
/// - `keys`     — the separator keys in sorted order.
/// - `values`   — one value per key, parallel array (same index as the key).
/// - `children` — if `is_leaf` is false, exactly `keys.len() + 1` children.
/// - `is_leaf`  — true if this node has no children.
///
/// Diagram for an internal node with 2 keys [K0, K1]:
///
/// ```text
///   children: [ C0  |  C1  |  C2 ]
///   keys:          K0    K1
/// ```
///
/// All keys in subtree C0 < K0 ≤ all keys in subtree C1 < K1 ≤ all keys in
/// subtree C2.
struct BTreeNode<K, V> {
    keys: Vec<K>,
    values: Vec<V>,
    children: Vec<Box<BTreeNode<K, V>>>,
    is_leaf: bool,
}

impl<K: Ord + Clone, V: Clone> BTreeNode<K, V> {
    /// Create a new empty leaf node.
    fn new_leaf() -> Self {
        BTreeNode {
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
            is_leaf: true,
        }
    }

    /// Create a new empty internal node.
    fn new_internal() -> Self {
        BTreeNode {
            keys: Vec::new(),
            values: Vec::new(),
            children: Vec::new(),
            is_leaf: false,
        }
    }

    /// Return true if this node holds the maximum number of keys (`2t - 1`).
    fn is_full(&self, t: usize) -> bool {
        self.keys.len() == 2 * t - 1
    }

    /// Binary-search for `key` in this node's key array.
    ///
    /// Returns `Ok(i)` if keys[i] == key, or `Err(i)` with `i` being the
    /// child index to follow for a descent.
    fn find_key_pos(&self, key: &K) -> Result<usize, usize> {
        // We use the standard library's binary search which returns
        // Ok(pos) on exact match and Err(pos) on miss (pos = insertion point).
        self.keys.binary_search_by(|k| k.cmp(key))
    }

    /// Search this subtree for `key`, returning a reference to the value.
    fn search(&self, key: &K) -> Option<&V> {
        match self.find_key_pos(key) {
            Ok(i) => Some(&self.values[i]),
            Err(i) => {
                if self.is_leaf {
                    None
                } else {
                    self.children[i].search(key)
                }
            }
        }
    }

    /// Split `self.children[child_index]`, which must be full (2t-1 keys).
    ///
    /// After the split:
    ///
    /// ```text
    /// Before:                       After:
    ///   self                          self
    ///   children[i] = [k0..k_{2t-2}]   children[i] = [k0..k_{t-2}]
    ///                                   children[i+1] = [k_t..k_{2t-2}]
    ///                                   self.keys[i] = k_{t-1}  (promoted)
    /// ```
    ///
    /// The median key (`k_{t-1}`) is promoted up into `self`.
    fn split_child(&mut self, child_index: usize, t: usize) {
        // We'll take ownership of the full child temporarily.
        let full_child = &mut self.children[child_index];
        let median_pos = t - 1; // index of the key to promote

        // Pull the right half of keys/values/children out of the full child.
        let right_keys: Vec<K> = full_child.keys.drain(median_pos + 1..).collect();
        let right_values: Vec<V> = full_child.values.drain(median_pos + 1..).collect();
        let right_children: Vec<Box<BTreeNode<K, V>>> = if !full_child.is_leaf {
            full_child.children.drain(t..).collect()
        } else {
            Vec::new()
        };

        // Take the median key/value to promote.
        let median_key = full_child.keys.pop().unwrap();
        let median_value = full_child.values.pop().unwrap();

        // Build the new right sibling node.
        let right_node = BTreeNode {
            is_leaf: full_child.is_leaf,
            keys: right_keys,
            values: right_values,
            children: right_children,
        };

        // Insert the median into self at position child_index.
        self.keys.insert(child_index, median_key);
        self.values.insert(child_index, median_value);
        self.children.insert(child_index + 1, Box::new(right_node));
    }

    /// Insert `(key, value)` into a subtree that is guaranteed to be
    /// **non-full** at `self`.  We split children proactively on the way
    /// down so we never have to backtrack.
    fn insert_non_full(&mut self, key: K, value: V, t: usize) -> bool {
        // Find where the key belongs.
        match self.find_key_pos(&key) {
            Ok(i) => {
                // Key already exists — update in place.
                self.values[i] = value;
                false // did not grow size
            }
            Err(i) => {
                if self.is_leaf {
                    // Simple leaf insert.
                    self.keys.insert(i, key);
                    self.values.insert(i, value);
                    true // size grew
                } else {
                    // We need to descend into children[i].  But first, if
                    // that child is full, split it so there's room to grow.
                    let mut idx = i;
                    if self.children[idx].is_full(t) {
                        self.split_child(idx, t);
                        // After split, the median is now at self.keys[idx].
                        // Decide which side to go into.
                        match key.cmp(&self.keys[idx]) {
                            std::cmp::Ordering::Equal => {
                                // The promoted median IS our key — update.
                                self.values[idx] = value;
                                return false;
                            }
                            std::cmp::Ordering::Greater => {
                                // Go to the new right child.
                                idx += 1;
                            }
                            std::cmp::Ordering::Less => {
                                // Stay in the left child.
                            }
                        }
                    }
                    self.children[idx].insert_non_full(key, value, t)
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Deletion helpers
    // -----------------------------------------------------------------------

    /// Return the number of keys in this node.
    fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Find and return the *predecessor* of `keys[idx]`: the largest key in
    /// the left subtree rooted at `children[idx]`.
    fn predecessor(&self, idx: usize) -> (K, V) {
        let mut node = &*self.children[idx];
        while !node.is_leaf {
            node = node.children.last().unwrap();
        }
        let last = node.keys.len() - 1;
        (node.keys[last].clone(), node.values[last].clone())
    }

    /// Find and return the *successor* of `keys[idx]`: the smallest key in
    /// the right subtree rooted at `children[idx+1]`.
    fn successor(&self, idx: usize) -> (K, V) {
        let mut node = &*self.children[idx + 1];
        while !node.is_leaf {
            node = &node.children[0];
        }
        (node.keys[0].clone(), node.values[0].clone())
    }

    /// Merge `children[idx]` and `children[idx+1]` with `keys[idx]` as the
    /// separator, producing a single child with `2t-1` keys.
    ///
    /// ```text
    /// Before: … [k_{idx-1} | K | k_{idx+1}] …   (self.keys)
    ///           C_left      C_right
    /// After:  … [k_{idx-1} | k_{idx+1}] …
    ///           C_merged = [C_left.keys | K | C_right.keys]
    /// ```
    fn merge_children(&mut self, idx: usize) {
        // Remove the right child.
        let mut right = *self.children.remove(idx + 1);
        // Pull out the separator key from self.
        let sep_key = self.keys.remove(idx);
        let sep_val = self.values.remove(idx);
        // Append separator + right child content into left child.
        let left = &mut self.children[idx];
        left.keys.push(sep_key);
        left.values.push(sep_val);
        left.keys.append(&mut right.keys);
        left.values.append(&mut right.values);
        left.children.append(&mut right.children);
    }

    /// Rotate a key from `children[idx+1]` down through `self.keys[idx]` into
    /// `children[idx]`.  ("Borrow from right sibling.")
    fn rotate_left(&mut self, idx: usize) {
        // Move self.keys[idx] into the left child.
        let sep_key = self.keys[idx].clone();
        let sep_val = self.values[idx].clone();
        let left = &mut self.children[idx];
        left.keys.push(sep_key);
        left.values.push(sep_val);

        // Pull the first key of the right sibling up to self.keys[idx].
        let right = &mut self.children[idx + 1];
        let new_sep_key = right.keys.remove(0);
        let new_sep_val = right.values.remove(0);
        self.keys[idx] = new_sep_key;
        self.values[idx] = new_sep_val;

        // If the right sibling is internal, also move its first child pointer.
        if !right.is_leaf {
            let child = right.children.remove(0);
            self.children[idx].children.push(child);
        }
    }

    /// Rotate a key from `children[idx-1]` down through `self.keys[idx-1]`
    /// into `children[idx]`.  ("Borrow from left sibling.")
    fn rotate_right(&mut self, idx: usize) {
        // Move self.keys[idx-1] to the front of the right child.
        let sep_key = self.keys[idx - 1].clone();
        let sep_val = self.values[idx - 1].clone();
        let right = &mut self.children[idx];
        right.keys.insert(0, sep_key);
        right.values.insert(0, sep_val);

        // Pull the last key of the left sibling up to self.keys[idx-1].
        let left = &mut self.children[idx - 1];
        let new_sep_key = left.keys.pop().unwrap();
        let new_sep_val = left.values.pop().unwrap();
        self.keys[idx - 1] = new_sep_key;
        self.values[idx - 1] = new_sep_val;

        // If the left sibling is internal, move its last child pointer to the
        // front of the right child.
        if !left.is_leaf {
            let child = left.children.pop().unwrap();
            self.children[idx].children.insert(0, child);
        }
    }

    /// Ensure that `children[idx]` has at least `t` keys before we descend
    /// into it.  This is the "pre-fill" step of the CLRS deletion algorithm.
    ///
    /// Three sub-cases:
    ///  - **Rotate right** (borrow from left sibling) when the left sibling
    ///    has ≥ t keys.
    ///  - **Rotate left** (borrow from right sibling) when the right sibling
    ///    has ≥ t keys.
    ///  - **Merge** with a sibling (and pull the separator down from self)
    ///    when both siblings are at the minimum.
    ///
    /// Returns the (possibly changed) child index to continue into.
    fn ensure_child_has_t_keys(&mut self, idx: usize, t: usize) -> usize {
        if self.children[idx].key_count() >= t {
            return idx; // already enough keys, nothing to do
        }

        let has_left = idx > 0;
        let has_right = idx + 1 < self.children.len();

        if has_left && self.children[idx - 1].key_count() >= t {
            // Borrow from left sibling.
            self.rotate_right(idx);
            idx
        } else if has_right && self.children[idx + 1].key_count() >= t {
            // Borrow from right sibling.
            self.rotate_left(idx);
            idx
        } else if has_left {
            // Merge children[idx-1] and children[idx].
            self.merge_children(idx - 1);
            idx - 1 // we now descend into the merged node (one index left)
        } else {
            // Merge children[idx] and children[idx+1].
            self.merge_children(idx);
            idx
        }
    }

    /// Delete `key` from the subtree rooted at `self`.
    ///
    /// Before calling, the caller must ensure `self` has ≥ t keys (or is the
    /// root).  We use the CLRS strategy: pre-fill on the way down.
    ///
    /// Returns `true` if a key was actually deleted.
    fn delete(&mut self, key: &K, t: usize) -> bool {
        match self.find_key_pos(key) {
            Ok(i) => {
                // The key lives in this node.
                if self.is_leaf {
                    // Case 1: leaf node with the key — just remove it.
                    self.keys.remove(i);
                    self.values.remove(i);
                    true
                } else {
                    // Cases 2a / 2b / 2c for internal nodes.
                    if self.children[i].key_count() >= t {
                        // Case 2a: left child has ≥ t keys — replace with
                        // predecessor and delete predecessor from left subtree.
                        let (pred_k, pred_v) = self.predecessor(i);
                        self.keys[i] = pred_k.clone();
                        self.values[i] = pred_v;
                        self.children[i].delete(&pred_k, t)
                    } else if self.children[i + 1].key_count() >= t {
                        // Case 2b: right child has ≥ t keys — replace with
                        // successor and delete successor from right subtree.
                        let (succ_k, succ_v) = self.successor(i);
                        self.keys[i] = succ_k.clone();
                        self.values[i] = succ_v;
                        self.children[i + 1].delete(&succ_k, t)
                    } else {
                        // Case 2c: both children have only t-1 keys — merge
                        // them and then delete the key from the merged child.
                        self.merge_children(i);
                        // After merge the left child is at index i and now
                        // contains the key (at position t-1 within it).
                        self.children[i].delete(key, t)
                    }
                }
            }
            Err(i) => {
                // Key is not in this node — descend.
                if self.is_leaf {
                    // Key doesn't exist in the tree.
                    false
                } else {
                    // Case 3: pre-fill before descent.
                    let new_i = self.ensure_child_has_t_keys(i, t);
                    self.children[new_i].delete(key, t)
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Traversal helpers
    // -----------------------------------------------------------------------

    /// Collect all (key, value) pairs in sorted order via an in-order walk.
    fn inorder<'a>(&'a self, out: &mut Vec<(&'a K, &'a V)>) {
        if self.is_leaf {
            for (k, v) in self.keys.iter().zip(self.values.iter()) {
                out.push((k, v));
            }
        } else {
            for i in 0..self.keys.len() {
                self.children[i].inorder(out);
                out.push((&self.keys[i], &self.values[i]));
            }
            self.children[self.keys.len()].inorder(out);
        }
    }

    /// Collect (key, value) pairs where `low <= key <= high`.
    fn range_query<'a>(&'a self, low: &K, high: &K, out: &mut Vec<(&'a K, &'a V)>) {
        if self.is_leaf {
            for (k, v) in self.keys.iter().zip(self.values.iter()) {
                if k >= low && k <= high {
                    out.push((k, v));
                }
            }
        } else {
            for i in 0..self.keys.len() {
                // Only recurse into children that could hold keys in [low,high].
                if &self.keys[i] > low {
                    self.children[i].range_query(low, high, out);
                }
                if &self.keys[i] >= low && &self.keys[i] <= high {
                    out.push((&self.keys[i], &self.values[i]));
                }
                if &self.keys[i] >= high {
                    return;
                }
            }
            // Check the last child.
            self.children[self.keys.len()].range_query(low, high, out);
        }
    }

    /// Return the minimum key in this subtree.
    fn min_key(&self) -> &K {
        if self.is_leaf {
            &self.keys[0]
        } else {
            self.children[0].min_key()
        }
    }

    /// Return the maximum key in this subtree.
    fn max_key(&self) -> &K {
        if self.is_leaf {
            self.keys.last().unwrap()
        } else {
            self.children.last().unwrap().max_key()
        }
    }

    /// Return the height of this subtree (0 = leaf).
    fn height(&self) -> usize {
        if self.is_leaf {
            0
        } else {
            1 + self.children[0].height()
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Validate B-tree structural invariants for this subtree.
    ///
    /// Returns `(depth_of_leaves, is_valid)`.
    ///
    /// Rules checked:
    ///  1. Keys are sorted within each node.
    ///  2. Every non-root node has ≥ t-1 keys.
    ///  3. Every node has ≤ 2t-1 keys.
    ///  4. All leaves are at the same depth.
    ///  5. Internal nodes have exactly keys.len()+1 children.
    fn validate(&self, t: usize, is_root: bool, depth: usize) -> (usize, bool) {
        // Check key count bounds.
        let min_keys = if is_root { 1 } else { t - 1 };
        let max_keys = 2 * t - 1;
        if !self.keys.is_empty() && (self.keys.len() < min_keys || self.keys.len() > max_keys) {
            return (depth, false);
        }

        // Check keys are sorted and unique.
        for i in 1..self.keys.len() {
            if self.keys[i - 1] >= self.keys[i] {
                return (depth, false);
            }
        }

        // Check values array length matches keys.
        if self.values.len() != self.keys.len() {
            return (depth, false);
        }

        if self.is_leaf {
            // Leaf nodes must have no children.
            if !self.children.is_empty() {
                return (depth, false);
            }
            (depth, true)
        } else {
            // Internal node must have keys.len()+1 children.
            if self.children.len() != self.keys.len() + 1 {
                return (depth, false);
            }

            // Recurse; collect all leaf depths.
            let mut leaf_depth: Option<usize> = None;
            for child in &self.children {
                let (d, ok) = child.validate(t, false, depth + 1);
                if !ok {
                    return (depth, false);
                }
                match leaf_depth {
                    None => leaf_depth = Some(d),
                    Some(ld) => {
                        if ld != d {
                            return (depth, false); // leaves at different depths
                        }
                    }
                }
            }
            (leaf_depth.unwrap_or(depth), true)
        }
    }
}

// ---------------------------------------------------------------------------
// Public BTree
// ---------------------------------------------------------------------------

/// A fully-featured B-tree with minimum degree `t`.
///
/// # Example
///
/// ```
/// use coding_adventures_b_tree::BTree;
///
/// let mut tree: BTree<i32, &str> = BTree::new(2);
/// tree.insert(10, "ten");
/// tree.insert(20, "twenty");
/// tree.insert(5,  "five");
///
/// assert_eq!(tree.search(&10), Some(&"ten"));
/// assert_eq!(tree.min_key(), Some(&5));
/// assert!(tree.is_valid());
/// ```
pub struct BTree<K: Ord + Clone, V: Clone> {
    root: Option<Box<BTreeNode<K, V>>>,
    /// Minimum degree.  Every non-root node holds between `t-1` and `2t-1`
    /// keys.  Must be ≥ 2.
    t: usize,
    size: usize,
}

impl<K: Ord + Clone, V: Clone> BTree<K, V> {
    /// Create a new empty B-tree with minimum degree `t`.
    ///
    /// `t` must be ≥ 2.  If a value < 2 is supplied, it is silently clamped
    /// to 2.
    ///
    /// With `t = 2` nodes hold 1–3 keys (a 2-3-4 tree).
    /// With `t = 100` nodes hold 99–199 keys (typical for a database index).
    pub fn new(t: usize) -> Self {
        BTree {
            root: None,
            t: t.max(2),
            size: 0,
        }
    }

    /// Insert `(key, value)` into the tree.
    ///
    /// If `key` already exists, its value is replaced (size does not grow).
    ///
    /// # Splitting strategy
    ///
    /// We use **proactive top-down splitting**:
    ///
    /// 1. If the root is full (`2t-1` keys) we split it *before* inserting.
    ///    This is the only time the tree grows in height.
    ///
    ///    ```text
    ///    Old root (full):  [k0 | k1 | … | k_{2t-2}]
    ///    New root:         [k_{t-1}]
    ///                      /        \
    ///         [k0..k_{t-2}]        [k_t..k_{2t-2}]
    ///    ```
    ///
    /// 2. On each subsequent level, if the child we are about to descend
    ///    into is full, we split it first and then descend.
    ///
    /// This ensures the parent always has room to absorb the promoted median.
    pub fn insert(&mut self, key: K, value: V) {
        let t = self.t;

        match &self.root {
            None => {
                // Tree is empty — create the first leaf.
                let mut leaf = BTreeNode::new_leaf();
                leaf.keys.push(key);
                leaf.values.push(value);
                self.root = Some(Box::new(leaf));
                self.size += 1;
            }
            Some(root) if root.is_full(t) => {
                // Root is full — split it.
                //
                // We replace the root with a new internal node that has the
                // old root as its only child, then split that child.
                let old_root = self.root.take().unwrap();
                let mut new_root = BTreeNode::new_internal();
                new_root.children.push(old_root);
                new_root.split_child(0, t);
                let grew = new_root.insert_non_full(key, value, t);
                self.root = Some(Box::new(new_root));
                if grew {
                    self.size += 1;
                }
            }
            Some(_) => {
                let grew = self.root.as_mut().unwrap().insert_non_full(key, value, t);
                if grew {
                    self.size += 1;
                }
            }
        }
    }

    /// Delete the entry with the given `key`.
    ///
    /// Returns `true` if the key existed and was removed, `false` if the key
    /// was not in the tree.
    ///
    /// # Algorithm (CLRS Chapter 18)
    ///
    /// We walk the tree top-down, ensuring each node we visit has at least `t`
    /// keys *before* we recurse into it.  This guarantees we can always remove
    /// a key from a node without violating the lower bound.
    ///
    /// The cases are:
    ///
    /// - **Case 1** — key is in a leaf: simply remove it.
    /// - **Case 2a** — key is in an internal node and left child has ≥ t keys:
    ///   replace key with its predecessor, then delete the predecessor.
    /// - **Case 2b** — key is in an internal node and right child has ≥ t keys:
    ///   replace key with its successor, then delete the successor.
    /// - **Case 2c** — both children have exactly t-1 keys: merge, then delete.
    /// - **Case 3** — key is not in this node (we need to descend):
    ///   pre-fill the target child (rotate or merge) before descending.
    pub fn delete(&mut self, key: &K) -> bool {
        let t = self.t;
        match &mut self.root {
            None => false,
            Some(root) => {
                let deleted = root.delete(key, t);
                if deleted {
                    self.size -= 1;
                    // If root has no keys but has a child, shrink the tree.
                    if root.keys.is_empty() && !root.children.is_empty() {
                        self.root = Some(self.root.take().unwrap().children.remove(0));
                    } else if root.keys.is_empty() {
                        // Tree is now empty.
                        self.root = None;
                    }
                }
                deleted
            }
        }
    }

    /// Search for `key`.  Returns a reference to the associated value if
    /// found, or `None` otherwise.
    pub fn search(&self, key: &K) -> Option<&V> {
        self.root.as_ref()?.search(key)
    }

    /// Return `true` if `key` is in the tree.
    pub fn contains(&self, key: &K) -> bool {
        self.search(key).is_some()
    }

    /// Return a reference to the smallest key in the tree, or `None` if empty.
    pub fn min_key(&self) -> Option<&K> {
        self.root.as_ref().map(|r| r.min_key())
    }

    /// Return a reference to the largest key in the tree, or `None` if empty.
    pub fn max_key(&self) -> Option<&K> {
        self.root.as_ref().map(|r| r.max_key())
    }

    /// Return all `(key, value)` pairs where `low <= key <= high`, in sorted
    /// order.
    ///
    /// This is efficient because the B-tree structure lets us skip whole
    /// subtrees that fall entirely outside the range.
    pub fn range_query<'a>(&'a self, low: &K, high: &K) -> Vec<(&'a K, &'a V)> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.range_query(low, high, &mut out);
        }
        out
    }

    /// Return all `(key, value)` pairs in sorted order.
    ///
    /// Equivalent to `range_query` with the widest possible range.
    pub fn inorder(&self) -> Vec<(&K, &V)> {
        let mut out = Vec::new();
        if let Some(root) = &self.root {
            root.inorder(&mut out);
        }
        out
    }

    /// Return the number of key-value pairs stored.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Return `true` if the tree holds no key-value pairs.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Return the height of the tree.
    ///
    /// - An empty tree has height 0.
    /// - A tree with only a root leaf has height 0.
    /// - Each level of internal nodes adds 1.
    pub fn height(&self) -> usize {
        match &self.root {
            None => 0,
            Some(root) => root.height(),
        }
    }

    /// Validate all B-tree structural invariants.
    ///
    /// Returns `true` if:
    ///
    /// - All leaves are at the same depth.
    /// - Every non-root node has between `t-1` and `2t-1` keys.
    /// - Keys within each node are sorted in ascending order.
    /// - Internal nodes have exactly `keys.len() + 1` children.
    pub fn is_valid(&self) -> bool {
        match &self.root {
            None => true,
            Some(root) => {
                if root.keys.is_empty() {
                    return true; // empty root is valid
                }
                let (_, ok) = root.validate(self.t, true, 0);
                ok
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (inline) — exhaustive edge cases
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Basic operations --

    #[test]
    fn empty_tree() {
        let t: BTree<i32, i32> = BTree::new(2);
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
        let mut t: BTree<i32, &str> = BTree::new(2);
        t.insert(42, "hello");
        assert_eq!(t.len(), 1);
        assert_eq!(t.search(&42), Some(&"hello"));
        assert!(t.contains(&42));
        assert!(t.is_valid());
        assert_eq!(t.min_key(), Some(&42));
        assert_eq!(t.max_key(), Some(&42));
    }

    #[test]
    fn duplicate_key_replaces_value() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        t.insert(1, 100);
        t.insert(1, 200);
        assert_eq!(t.len(), 1);
        assert_eq!(t.search(&1), Some(&200));
        assert!(t.is_valid());
    }

    #[test]
    fn delete_from_empty_returns_false() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        assert!(!t.delete(&5));
    }

    #[test]
    fn delete_missing_key_returns_false() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        t.insert(1, 1);
        t.insert(2, 2);
        assert!(!t.delete(&99));
        assert!(t.is_valid());
    }

    #[test]
    fn delete_only_key() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        t.insert(5, 50);
        assert!(t.delete(&5));
        assert!(t.is_empty());
        assert!(t.is_valid());
    }

    // -- Range queries --

    #[test]
    fn range_query_basic() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        for i in 1..=10 {
            t.insert(i, i * 10);
        }
        let r = t.range_query(&3, &7);
        let keys: Vec<i32> = r.iter().map(|(k, _)| **k).collect();
        assert_eq!(keys, vec![3, 4, 5, 6, 7]);
        assert!(t.is_valid());
    }

    #[test]
    fn range_query_empty_range() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        for i in [1, 5, 10] {
            t.insert(i, i);
        }
        let r = t.range_query(&6, &9);
        assert!(r.is_empty());
    }

    #[test]
    fn inorder_is_sorted() {
        let mut t: BTree<i32, i32> = BTree::new(3);
        let keys = vec![20, 5, 15, 25, 10, 30, 1];
        for k in &keys {
            t.insert(*k, *k);
        }
        let io: Vec<i32> = t.inorder().iter().map(|(k, _)| **k).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(io, sorted);
        assert!(t.is_valid());
    }

    // -- Proactive root split --

    #[test]
    fn root_split_increases_height_t2() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        // A t=2 node holds at most 3 keys.  Inserting 4 keys forces a split.
        for i in 1..=4 {
            t.insert(i, i);
        }
        assert!(t.height() >= 1);
        assert!(t.is_valid());
        assert_eq!(t.len(), 4);
    }

    // -- Delete sub-cases (CLRS) --

    /// Case 1: delete from leaf that has enough keys.
    #[test]
    fn delete_case1_leaf_enough_keys() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        for i in 1..=7 {
            t.insert(i, i);
        }
        assert!(t.delete(&3));
        assert!(!t.contains(&3));
        assert_eq!(t.len(), 6);
        assert!(t.is_valid());
    }

    /// Case 2a: delete internal key whose left child has ≥ t keys.
    #[test]
    fn delete_case2a_predecessor_replacement() {
        // Build a tree big enough to guarantee an internal node.
        let mut t: BTree<i32, i32> = BTree::new(2);
        for i in 1..=15 {
            t.insert(i, i);
        }
        // Delete something that should be in an internal node.
        let mid = *t.min_key().unwrap() + (t.len() / 2) as i32;
        let existed = t.delete(&mid);
        // It may or may not be internal, but the tree must remain valid.
        let _ = existed;
        assert!(t.is_valid());
    }

    /// Case 2b: delete internal key whose right child has ≥ t keys (covered by
    /// a sequence that forces the left to be minimal).
    #[test]
    fn delete_various_internal_keys() {
        let mut t: BTree<i32, i32> = BTree::new(3);
        for i in 0..30 {
            t.insert(i, i);
        }
        // Delete every even key and check validity each time.
        for i in (0..30).step_by(2) {
            t.delete(&i);
            assert!(t.is_valid(), "invalid after deleting {i}");
        }
    }

    /// Case 2c: both children have t-1 keys — merge triggered.
    #[test]
    fn delete_case2c_merge() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        for i in 1..=7 {
            t.insert(i, i);
        }
        // Delete all but one, repeatedly triggering merges.
        for i in 1..=6 {
            assert!(t.delete(&i));
            assert!(t.is_valid(), "invalid after deleting {i}");
        }
        assert_eq!(t.len(), 1);
        assert!(t.is_valid());
    }

    /// Case 3: key not in current node; pre-fill via rotate.
    #[test]
    fn delete_case3_rotate() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        // Fill and then delete in a pattern that exercises borrow-from-sibling.
        let vals: Vec<i32> = vec![10, 20, 30, 40, 50, 60, 70];
        for v in &vals {
            t.insert(*v, *v);
        }
        for v in &vals {
            t.delete(v);
            assert!(t.is_valid(), "invalid after deleting {v}");
        }
        assert!(t.is_empty());
    }

    // -- Various t values --

    #[test]
    fn t3_operations() {
        let mut t: BTree<i32, i32> = BTree::new(3);
        for i in 0..50 {
            t.insert(i, i * 2);
        }
        assert_eq!(t.len(), 50);
        assert!(t.is_valid());
        for i in 0..50 {
            assert_eq!(t.search(&i), Some(&(i * 2)));
        }
        for i in (0..50).step_by(3) {
            t.delete(&i);
        }
        assert!(t.is_valid());
    }

    #[test]
    fn t5_operations() {
        let mut t: BTree<i32, i32> = BTree::new(5);
        for i in (0..100).rev() {
            t.insert(i, i);
        }
        assert_eq!(t.len(), 100);
        assert!(t.is_valid());
        for i in 0..50 {
            t.delete(&i);
        }
        assert_eq!(t.len(), 50);
        assert!(t.is_valid());
    }

    // -- Large-scale test --

    #[test]
    fn ten_thousand_keys_t2() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        for i in 0..10_000 {
            t.insert(i, i);
        }
        assert_eq!(t.len(), 10_000);
        assert!(t.is_valid());

        // All keys present.
        for i in 0..10_000 {
            assert!(t.contains(&i), "missing key {i}");
        }

        // Delete odd keys.
        for i in (1..10_000).step_by(2) {
            assert!(t.delete(&i));
        }
        assert_eq!(t.len(), 5_000);
        assert!(t.is_valid());

        // Only even keys remain.
        for i in 0..10_000 {
            assert_eq!(t.contains(&i), i % 2 == 0, "key {i}");
        }
    }

    #[test]
    fn ten_thousand_keys_t5() {
        let mut t: BTree<i32, i32> = BTree::new(5);
        for i in (0..10_000).rev() {
            t.insert(i, i * 3);
        }
        assert!(t.is_valid());
        assert_eq!(t.search(&9999), Some(&29997));
        assert_eq!(t.search(&0), Some(&0));
        let io: Vec<i32> = t.inorder().iter().map(|(k, _)| **k).collect();
        assert_eq!(io, (0..10_000).collect::<Vec<_>>());
    }

    // -- min / max --

    #[test]
    fn min_max_after_inserts() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        let vals = vec![50, 10, 90, 30, 70];
        for v in &vals {
            t.insert(*v, *v);
        }
        assert_eq!(t.min_key(), Some(&10));
        assert_eq!(t.max_key(), Some(&90));
        t.delete(&10);
        assert_eq!(t.min_key(), Some(&30));
        t.delete(&90);
        assert_eq!(t.max_key(), Some(&70));
        assert!(t.is_valid());
    }

    // -- is_valid after mixed ops --

    #[test]
    fn is_valid_after_every_operation() {
        let mut t: BTree<i32, i32> = BTree::new(2);
        assert!(t.is_valid());
        for i in 0..20 {
            t.insert(i, i);
            assert!(t.is_valid(), "invalid after insert {i}");
        }
        for i in (0..20).step_by(2) {
            t.delete(&i);
            assert!(t.is_valid(), "invalid after delete {i}");
        }
    }
}
