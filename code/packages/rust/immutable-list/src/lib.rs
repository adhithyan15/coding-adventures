// lib.rs -- ImmutableList: A Persistent Vector via 32-Way Trie with Tail Buffer
// =============================================================================
//
// An immutable list is a persistent data structure -- once created, it never
// changes. Every "modification" (push, set, pop) returns a *new* list, leaving
// the original intact. This sounds wasteful, but the trick is **structural
// sharing**: the new list reuses most of the old list's memory, only allocating
// new nodes along the path that changed.
//
// Why does this matter?
//
// 1. **Safety.** When data can't be mutated, entire categories of bugs vanish:
//    no iterator invalidation, no data races between threads, no spooky
//    action-at-a-distance where one function modifies a list another is reading.
//
// 2. **Concurrency.** Immutable data is inherently thread-safe. No locks,
//    mutexes, or atomic operations needed for reading. Multiple threads share
//    the same list with zero synchronization overhead.
//
// 3. **Time travel.** Every version persists. You can keep a reference to "the
//    list before the last 50 pushes" and it's still valid, still O(1) to access.
//    This enables undo/redo, transactional semantics, and snapshotting for free.
//
//
// The Design: 32-Way Trie with Tail Buffer
// -----------------------------------------
//
// This implementation follows Clojure's PersistentVector (designed by Rich
// Hickey, based on Phil Bagwell's Hash Array Mapped Tries). It combines two
// ideas for near-constant-time operations:
//
// 1. **A wide trie (32-way branching tree).** Every internal node has up to 32
//    children, every leaf has up to 32 elements. A tree holding 1 million
//    elements is only 4 levels deep (32^4 = 1,048,576). Index lookup walks at
//    most 4 levels -- effectively O(1) for any practical size.
//
// 2. **A tail buffer.** The last 32 elements live in a flat Vec outside the
//    trie. Most `push` operations just append to this buffer -- no tree
//    traversal needed. Only when the tail fills (every 32nd push) does it get
//    promoted into the trie as a new leaf. That means ~97% of pushes are a
//    simple Vec append.
//
//
// Cache Locality: Why 32?
// -----------------------
//
// 32 elements per node means each node fits in 1-2 CPU cache lines (a cache
// line is typically 64 bytes). When the CPU loads a node, all 32 children or
// elements come with it. Compare this to a binary tree where each node holds
// just 2 pointers and every access is a cache miss.
//
//     Depth vs. capacity:
//
//     Depth 1:  32^1 =            32 elements
//     Depth 2:  32^2 =         1,024 elements
//     Depth 3:  32^3 =        32,768 elements
//     Depth 4:  32^4 =     1,048,576 elements
//     Depth 5:  32^5 =    33,554,432 elements
//     Depth 6:  32^6 = 1,073,741,824 elements
//
//     A depth-4 trie holds over a million elements. For most applications,
//     you'll never exceed depth 5 or 6. This is why O(log32 n) is effectively
//     O(1) -- the "log" never exceeds single digits.

use std::fmt;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
//
// The branching factor is 32, chosen for cache-line alignment and minimal tree
// depth. Every formula in this module derives from these three constants:
//
//   BRANCH_FACTOR = 32  -- how many children per node / elements per leaf
//   BITS = 5            -- log2(32), the number of index bits consumed per level
//   MASK = 0x1F = 31    -- a 5-bit mask: 0b11111
//
// These are analogous to the BITS_PER_WORD constant in bitset -- a fundamental
// parameter that shapes every operation.

/// Number of children per internal node, and elements per leaf.
const BRANCH_FACTOR: usize = 32;

/// Number of index bits consumed per trie level. 2^5 = 32.
const BITS: u32 = 5;

/// Bitmask for extracting the lowest 5 bits: 0b11111 = 31.
const MASK: usize = BRANCH_FACTOR - 1;

// ---------------------------------------------------------------------------
// Node: The building block of the trie
// ---------------------------------------------------------------------------
//
// The trie has two kinds of nodes:
//
// - **Internal** nodes hold up to 32 children, each an Arc<Node>. Children can
//   be None (empty slot) or Some(Arc<Node>). Internal nodes exist only to route
//   index lookups to the correct leaf.
//
// - **Leaf** nodes hold up to 32 elements (String values). Elements can be
//   None (empty slot) or Some(String).
//
// Why Option<Arc<Node>> instead of just Arc<Node>? Because not every node is
// fully populated. A list with 33 elements has one internal node with only 2
// children (not 32). The remaining 30 slots are None.
//
//     Internal node with 3 children:
//
//     ┌──────────────────────────────────────────────────────────────┐
//     │ children: [Some, Some, Some, None, None, ..., None]         │
//     │            [0]   [1]   [2]   [3]   [4]        [31]         │
//     └──────────────────────────────────────────────────────────────┘
//
//     Leaf node with 32 elements (full):
//
//     ┌──────────────────────────────────────────────────────────────┐
//     │ elements: [Some("a"), Some("b"), ..., Some("z")]            │
//     │            [0]        [1]              [31]                  │
//     └──────────────────────────────────────────────────────────────┘

#[derive(Clone, Debug)]
enum Node {
    /// An internal node routes index lookups to the correct child subtree.
    /// Each child slot is either None (empty) or Some(Arc<Node>).
    Internal {
        children: Vec<Option<Arc<Node>>>,
    },

    /// A leaf node stores actual elements. Each slot is either None (empty)
    /// or Some(String) containing the element value.
    Leaf {
        elements: Vec<Option<String>>,
    },
}

impl Node {
    /// Create a new empty internal node with 32 None slots.
    ///
    /// This is the starting point for a fresh trie level. Slots get filled
    /// as leaves are promoted from the tail buffer.
    fn new_internal() -> Self {
        Node::Internal {
            children: vec![None; BRANCH_FACTOR],
        }
    }

    /// Create a new leaf node from a Vec of strings.
    ///
    /// The input values are placed into the first `values.len()` slots,
    /// and the remaining slots (up to 32) are left as None.
    ///
    /// This is used during tail promotion: the full tail buffer becomes a
    /// leaf node in the trie.
    fn new_leaf_from_values(values: &[String]) -> Self {
        let mut elements = Vec::with_capacity(BRANCH_FACTOR);
        for v in values {
            elements.push(Some(v.clone()));
        }
        // Pad remaining slots with None up to BRANCH_FACTOR.
        while elements.len() < BRANCH_FACTOR {
            elements.push(None);
        }
        Node::Leaf { elements }
    }
}

// ---------------------------------------------------------------------------
// ImmutableList: The public-facing persistent vector
// ---------------------------------------------------------------------------
//
// The ImmutableList struct holds four fields:
//
//   root  -- Arc<Node> pointing to the root of the trie
//   tail  -- Vec<String> holding the last block of up to 32 elements
//   len   -- total number of elements (trie + tail)
//   shift -- tree depth * BITS (controls bit partitioning)
//
// Memory layout example for a list with 67 elements:
//
//     ┌─────────────────────────────────────────────────┐
//     │ root: Arc ──► Internal                          │
//     │                  │       │                       │
//     │                  ▼       ▼                       │
//     │              Leaf 0   Leaf 1                     │
//     │              (e0-31)  (e32-63)                   │
//     │                                                  │
//     │ tail: ["e64", "e65", "e66"]                      │
//     │ len: 67                                          │
//     │ shift: 5  (depth 1)                              │
//     └─────────────────────────────────────────────────┘
//
// The tail is a plain Vec<String> (not inside the trie). This is the key
// optimization: most pushes just append to the tail without touching the trie.
//
// shift = 0 means the trie is empty (all elements in tail, or list is empty).
// shift = 5 means depth 1 (root children are leaves).
// shift = 10 means depth 2 (root -> internal -> leaf).
// And so on. In general: shift = depth * 5.

/// A persistent (immutable) vector using a 32-way trie with structural sharing.
///
/// Every mutation operation (`push`, `set`, `pop`) returns a **new** list,
/// leaving the original unchanged. The new and old lists share most of their
/// internal structure via `Arc` reference counting, making mutations
/// efficient -- O(log32 n) new nodes, which is effectively O(1) for practical
/// sizes.
///
/// Elements are `String` values. Future versions may support generic types.
///
/// # Examples
///
/// ```
/// use immutable_list::ImmutableList;
///
/// // Create a list and push elements
/// let empty = ImmutableList::new();
/// let one = empty.push("hello".to_string());
/// let two = one.push("world".to_string());
///
/// // Original lists are unchanged
/// assert_eq!(empty.len(), 0);
/// assert_eq!(one.len(), 1);
/// assert_eq!(two.len(), 2);
///
/// // Index access
/// assert_eq!(two.get(0), Some("hello"));
/// assert_eq!(two.get(1), Some("world"));
///
/// // Set returns a new list
/// let modified = two.set(0, "hi".to_string());
/// assert_eq!(modified.get(0), Some("hi"));
/// assert_eq!(two.get(0), Some("hello")); // original unchanged
///
/// // Pop returns (new_list, removed_element)
/// let (popped, val) = two.pop();
/// assert_eq!(val, "world");
/// assert_eq!(popped.len(), 1);
/// ```
#[derive(Clone)]
pub struct ImmutableList {
    /// The root of the trie. For lists with <= 32 elements, this is an empty
    /// internal node because all elements live in the tail buffer.
    root: Arc<Node>,

    /// The tail buffer: the last block of up to 32 elements, stored outside
    /// the trie for fast push/pop. This is where ~97% of pushes go.
    tail: Vec<String>,

    /// Total number of elements in the list (trie + tail combined).
    len: usize,

    /// Bit shift for the root level: depth * BITS. Controls how many bits
    /// of the index to extract at the root level during lookups.
    ///
    /// shift = 0  -> trie is empty (all in tail, or empty list)
    /// shift = 5  -> depth 1 (root children are leaves)
    /// shift = 10 -> depth 2 (root -> internal -> leaf)
    shift: u32,
}

// ---------------------------------------------------------------------------
// Core operations
// ---------------------------------------------------------------------------

impl ImmutableList {
    // =======================================================================
    // Constructor: new()
    // =======================================================================
    //
    // Creates an empty list. The root is an empty internal node (all children
    // None), the tail is empty, length is 0, and shift is 5 (depth 1).
    //
    // Why shift=5 and not 0? Because when we promote the first tail into the
    // trie, we need root.children to be ready to accept a leaf. With shift=5,
    // the root is at depth 1, meaning its children are leaves. This avoids a
    // special case in push_tail.
    //
    // However, for the truly empty case we also need to handle `shift` correctly
    // during tail_offset calculation. See tail_offset() below.

    /// Create a new, empty immutable list.
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let list = ImmutableList::new();
    /// assert!(list.is_empty());
    /// assert_eq!(list.len(), 0);
    /// ```
    pub fn new() -> Self {
        ImmutableList {
            root: Arc::new(Node::new_internal()),
            tail: Vec::new(),
            len: 0,
            shift: BITS,
        }
    }

    // =======================================================================
    // Constructor: from_slice()
    // =======================================================================
    //
    // Builds a list from a slice of strings by pushing each element one at a
    // time. This is straightforward and leverages the push logic (including
    // tail promotion) that we've already implemented.
    //
    // A more optimal approach would build the trie bottom-up (directly creating
    // full leaf nodes from chunks of 32), but the iterative approach is correct
    // and simpler to verify.

    /// Create an immutable list from a slice of strings.
    ///
    /// Equivalent to calling `push` for each element in order, but expressed
    /// as a single constructor.
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    /// let list = ImmutableList::from_slice(&items);
    /// assert_eq!(list.len(), 3);
    /// assert_eq!(list.get(0), Some("a"));
    /// assert_eq!(list.get(2), Some("c"));
    /// ```
    pub fn from_slice(items: &[String]) -> Self {
        let mut list = ImmutableList::new();
        for item in items {
            list = list.push(item.clone());
        }
        list
    }

    // =======================================================================
    // len() and is_empty()
    // =======================================================================

    /// Return the number of elements in the list.
    ///
    /// This is O(1) -- the length is stored as a field, not computed by
    /// traversing the trie.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return true if the list has zero elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // =======================================================================
    // tail_offset(): Where does the trie end and the tail begin?
    // =======================================================================
    //
    // The tail buffer holds the last block of elements. To know whether a given
    // index is in the trie or the tail, we compute `tail_offset`:
    //
    //     tail_offset = len - tail.len()
    //
    // Elements at indices [0, tail_offset) are in the trie.
    // Elements at indices [tail_offset, len) are in the tail.
    //
    // Example: a list with 67 elements and a tail of 3:
    //     tail_offset = 67 - 3 = 64
    //     Indices 0-63 are in the trie, indices 64-66 are in the tail.

    /// Compute the index where the tail buffer starts.
    ///
    /// All elements with index < tail_offset() are in the trie.
    /// All elements with index >= tail_offset() are in the tail.
    fn tail_offset(&self) -> usize {
        if self.len < BRANCH_FACTOR {
            // When the list has fewer than 32 elements, everything is in the
            // tail. The trie is empty. tail_offset is 0.
            0
        } else {
            // General case: tail holds the last (len % 32) elements, or 32
            // if len is a multiple of 32. But we can compute it more simply:
            // tail_offset = len - tail.len()
            self.len - self.tail.len()
        }
    }

    // =======================================================================
    // get(index): Index Lookup via Bit Partitioning
    // =======================================================================
    //
    // To find element at index `i`:
    //
    // 1. Check if i is in the tail buffer (i >= tail_offset).
    //    If so, return tail[i - tail_offset]. This is O(1).
    //
    // 2. Otherwise, descend the trie using bit partitioning:
    //    At each level, extract 5 bits from the index to select a child.
    //
    //    child_index = (i >> current_shift) & MASK
    //
    //    Start at shift (the root's shift), decrement by BITS at each level,
    //    until we reach shift=0 (the leaf level).
    //
    //    Example: get(1000) in a depth-2 trie (shift=10):
    //
    //    1000 in binary (15 bits): 00000_11111_01000
    //
    //    Level 0 (root, shift=10): (1000 >> 10) & 0x1F =  0 -> children[0]
    //    Level 1 (shift=5):        (1000 >>  5) & 0x1F = 31 -> children[31]
    //    Level 2 (leaf, shift=0):  (1000 >>  0) & 0x1F =  8 -> elements[8]
    //
    //    The index is essentially a base-32 number, and each digit selects
    //    a child at one level of the trie.

    /// Retrieve the element at `index`, or `None` if out of bounds.
    ///
    /// Time complexity: O(log32 n), which is effectively O(1) for practical
    /// sizes (at most 6-7 levels for billions of elements).
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let list = ImmutableList::new()
    ///     .push("a".to_string())
    ///     .push("b".to_string());
    /// assert_eq!(list.get(0), Some("a"));
    /// assert_eq!(list.get(1), Some("b"));
    /// assert_eq!(list.get(2), None);
    /// ```
    pub fn get(&self, index: usize) -> Option<&str> {
        // Bounds check.
        if index >= self.len {
            return None;
        }

        let tail_off = self.tail_offset();

        // Fast path: element is in the tail buffer.
        if index >= tail_off {
            return Some(&self.tail[index - tail_off]);
        }

        // Slow path: descend the trie via bit partitioning.
        //
        // We walk from the root down to the leaf, extracting 5 bits of the
        // index at each level to choose which child to follow.
        let mut node = &*self.root;
        let mut level = self.shift;

        loop {
            match node {
                Node::Internal { children } => {
                    // Extract 5 bits at the current level to select a child.
                    let child_idx = (index >> level) & MASK;
                    match &children[child_idx] {
                        Some(child) => {
                            node = child;
                            level -= BITS;
                        }
                        None => return None, // Shouldn't happen for valid indices.
                    }
                }
                Node::Leaf { elements } => {
                    // At the leaf level (shift=0), extract the lowest 5 bits.
                    let elem_idx = index & MASK;
                    return elements[elem_idx].as_deref();
                }
            }
        }
    }

    // =======================================================================
    // push(value): Append to the End
    // =======================================================================
    //
    // Push has two paths:
    //
    // **Fast path (~97% of calls):** If the tail has room (< 32 elements),
    // just clone the tail and append. No trie modification. O(1).
    //
    //     push("X") when tail = ["a", "b", "c"]:
    //     -> new tail = ["a", "b", "c", "X"]
    //     -> same root (Arc::clone, reference count++)
    //     -> new len = old len + 1
    //
    // **Slow path (every 32nd push):** The tail is full (32 elements). We must:
    //   1. Promote the current tail into the trie as a new leaf node.
    //   2. Create a new tail containing only the pushed element.
    //
    //     push("X") when tail is full:
    //     -> promote old tail as Leaf in the trie
    //     -> new tail = ["X"]
    //     -> new root = modified trie with the new leaf
    //     -> new len = old len + 1
    //
    // Tail promotion may trigger a tree growth if the trie is full at its
    // current depth. See push_tail() for details.

    /// Append an element to the end of the list. Returns a new list.
    ///
    /// The original list is unchanged (structural sharing).
    ///
    /// Time: O(1) amortized. ~97% of calls are a simple tail append.
    /// The remaining ~3% promote the tail into the trie: O(log32 n).
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let a = ImmutableList::new().push("hello".to_string());
    /// let b = a.push("world".to_string());
    /// assert_eq!(a.len(), 1); // a is unchanged
    /// assert_eq!(b.len(), 2);
    /// ```
    pub fn push(&self, value: String) -> ImmutableList {
        // Fast path: tail has room. Just append to a cloned tail.
        if self.tail.len() < BRANCH_FACTOR {
            let mut new_tail = self.tail.clone();
            new_tail.push(value);
            return ImmutableList {
                root: Arc::clone(&self.root),
                tail: new_tail,
                len: self.len + 1,
                shift: self.shift,
            };
        }

        // Slow path: tail is full (32 elements). Promote it to the trie.
        //
        // Step 1: Convert the full tail into a leaf node.
        let tail_leaf = Arc::new(Node::new_leaf_from_values(&self.tail));

        // Step 2: Insert the tail leaf into the trie. This may grow the tree
        // if all leaf slots at the current depth are full.
        let (new_root, new_shift) = self.push_tail(self.len, self.shift, &self.root, tail_leaf);

        // Step 3: Start a fresh tail with just the new element.
        ImmutableList {
            root: new_root,
            tail: vec![value],
            len: self.len + 1,
            shift: new_shift,
        }
    }

    // =======================================================================
    // push_tail(): Insert a leaf into the trie
    // =======================================================================
    //
    // This is the core of tail promotion. We need to insert a new leaf node
    // at the rightmost position in the trie. There are three cases:
    //
    // 1. **Room at the current level:** The rightmost internal node at the
    //    bottom level has an empty child slot. Insert the leaf there.
    //
    // 2. **Current level is full, but tree has room:** Walk down the rightmost
    //    path, creating new internal nodes as needed, until we find room.
    //
    // 3. **Tree is completely full:** The trie can't hold any more leaves at
    //    its current depth. We must grow: create a new root with the old root
    //    as its first child, and insert the leaf into the new root's second
    //    subtree. This increases shift by BITS.
    //
    // In all cases, we use **path copying**: we clone the internal nodes along
    // the insertion path and share everything else via Arc.
    //
    //     Before (depth 1, 2 leaves, inserting 3rd):
    //
    //     root ──► [Internal: child[0]=Leaf0, child[1]=Leaf1, child[2]=None...]
    //
    //     After:
    //
    //     new_root ──► [Internal': child[0]=Leaf0, child[1]=Leaf1, child[2]=NewLeaf...]
    //                   (Leaf0 and Leaf1 are shared via Arc)

    /// Insert a leaf node into the trie at the next available rightmost position.
    ///
    /// Returns (new_root, new_shift). The shift may increase if the tree grows.
    fn push_tail(
        &self,
        count: usize,
        shift: u32,
        parent: &Arc<Node>,
        tail_node: Arc<Node>,
    ) -> (Arc<Node>, u32) {
        // Check if the tree needs to grow a new level.
        //
        // The tree is full when we have exactly 32^depth leaves worth of
        // elements in the trie. The trie portion holds (count - tail.len())
        // elements, but at the point push_tail is called, count = self.len
        // (before incrementing). The number of elements in the trie is
        // count - BRANCH_FACTOR (since the full tail has BRANCH_FACTOR elements
        // that are being promoted).
        //
        // Actually, the simpler check: can the root accommodate another child
        // at the rightmost path? We check if the index of the new leaf
        // overflows the current root capacity.
        //
        // Capacity at current depth = 1 << (shift + BITS) = 32^(depth+1).
        // But we need to count only trie elements. The trie holds elements
        // [0, tail_offset). After promotion, the new tail_offset would be
        // `count` (the old tail is now in the trie).
        //
        // The tree needs to grow if: count >> BITS > (1 << shift)
        // i.e., the number of leaves exceeds what one root can hold.

        let trie_elements = count; // All `count` elements will be in the trie after promotion

        // Does the current root have room? At depth=shift/BITS, the root can
        // hold up to 32 children. The index of the new leaf at the root level
        // is (trie_elements >> shift). If this equals 32, we've overflowed.
        if (trie_elements >> BITS) > (1u64 << shift as u64) as usize {
            // Tree overflow! Create a new root one level higher.
            //
            //     old root ──► [Internal with 32 full children]
            //
            //     new root ──► [Internal']
            //                    child[0] = old root
            //                    child[1] = new subtree containing tail_node
            //
            let new_shift = shift + BITS;
            let mut new_children = vec![None; BRANCH_FACTOR];
            new_children[0] = Some(Arc::clone(parent));

            // Create a path of empty internal nodes down to the leaf level.
            let new_subtree = self.new_path(shift, tail_node);
            new_children[1] = Some(new_subtree);

            return (
                Arc::new(Node::Internal {
                    children: new_children,
                }),
                new_shift,
            );
        }

        // The root has room. Walk down the rightmost path and insert.
        let new_root = self.do_push_tail(count, shift, parent, tail_node);
        (new_root, shift)
    }

    /// Recursively descend the trie to insert a leaf at the rightmost position.
    ///
    /// This performs path copying: nodes on the insertion path are cloned,
    /// siblings are shared via Arc.
    fn do_push_tail(
        &self,
        count: usize,
        level: u32,
        parent: &Arc<Node>,
        tail_node: Arc<Node>,
    ) -> Arc<Node> {
        match &**parent {
            Node::Internal { children } => {
                // Find the child index for the new leaf at this level.
                // The index tells us which subtree the new leaf belongs in.
                let subidx = ((count - 1) >> level) & MASK;

                // Clone the children array (path copying).
                let mut new_children = children.clone();

                if level == BITS {
                    // We're at the bottom internal level. The child at subidx
                    // should be the new leaf node directly.
                    new_children[subidx] = Some(tail_node);
                } else {
                    // We're above the bottom level. Recurse into the child.
                    match &children[subidx] {
                        Some(child) => {
                            // Child exists -- recurse into it.
                            let new_child =
                                self.do_push_tail(count, level - BITS, child, tail_node);
                            new_children[subidx] = Some(new_child);
                        }
                        None => {
                            // No child at this index yet. Create a new path
                            // of internal nodes down to the leaf level.
                            let new_path = self.new_path(level - BITS, tail_node);
                            new_children[subidx] = Some(new_path);
                        }
                    }
                }

                Arc::new(Node::Internal {
                    children: new_children,
                })
            }
            Node::Leaf { .. } => {
                // We should never reach a leaf during push_tail traversal.
                // The traversal stops at the bottom internal level (level == BITS).
                panic!("push_tail reached a leaf node unexpectedly");
            }
        }
    }

    /// Create a path of empty internal nodes from `level` down to BITS, ending
    /// with the given tail_node as the leaf.
    ///
    /// This is used when inserting a leaf into a part of the trie that doesn't
    /// exist yet (e.g., when the tree grows a new level, or when we need to
    /// create intermediate nodes for a sparse subtree).
    ///
    /// ```text
    /// new_path(level=10, leaf) creates:
    ///
    /// Internal (level 10)
    ///     child[0] = Internal (level 5)
    ///         child[0] = leaf (level 0)
    /// ```
    fn new_path(&self, level: u32, node: Arc<Node>) -> Arc<Node> {
        if level == 0 {
            // We're at the leaf level. Return the node as-is.
            return node;
        }

        // Create an internal node and put the result of recursing one level
        // down into child[0].
        let mut children = vec![None; BRANCH_FACTOR];
        children[0] = Some(self.new_path(level - BITS, node));
        Arc::new(Node::Internal { children })
    }

    // =======================================================================
    // set(index, value): Replace an Element via Path Copying
    // =======================================================================
    //
    // To replace element at index `i`:
    //
    // 1. If `i` is in the tail, clone the tail and replace the element.
    //
    // 2. If `i` is in the trie, walk from root to the leaf containing `i`,
    //    cloning each node on the path (path copying). At the leaf, replace
    //    the element. All sibling nodes are shared via Arc.
    //
    //     set(33, "NEW") on a list with 64 elements:
    //
    //     BEFORE:
    //     root ──► [Internal: child[0]=Leaf0, child[1]=Leaf1]
    //
    //     AFTER:
    //     new_root ──► [Internal': child[0]=Leaf0 (shared), child[1]=Leaf1' (new)]
    //                   Leaf1' has elements[1] = "NEW"
    //
    //     Only 2 new nodes: Internal' and Leaf1'. Everything else is shared.

    /// Replace the element at `index` with `value`. Returns a new list.
    ///
    /// Panics if `index >= len()`.
    ///
    /// Time: O(log32 n) -- path copying creates at most `depth + 1` new nodes.
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let a = ImmutableList::from_slice(&["x".to_string(), "y".to_string()]);
    /// let b = a.set(1, "z".to_string());
    /// assert_eq!(a.get(1), Some("y")); // original unchanged
    /// assert_eq!(b.get(1), Some("z"));
    /// ```
    pub fn set(&self, index: usize, value: String) -> ImmutableList {
        assert!(index < self.len, "index out of bounds: {} >= {}", index, self.len);

        let tail_off = self.tail_offset();

        // If the index is in the tail buffer, just clone and modify the tail.
        if index >= tail_off {
            let mut new_tail = self.tail.clone();
            new_tail[index - tail_off] = value;
            return ImmutableList {
                root: Arc::clone(&self.root),
                tail: new_tail,
                len: self.len,
                shift: self.shift,
            };
        }

        // Index is in the trie. Path-copy from root to the target leaf.
        let new_root = self.do_set(self.shift, &self.root, index, value);
        ImmutableList {
            root: new_root,
            tail: self.tail.clone(),
            len: self.len,
            shift: self.shift,
        }
    }

    /// Recursively descend the trie, path-copying nodes, to set an element.
    fn do_set(&self, level: u32, node: &Arc<Node>, index: usize, value: String) -> Arc<Node> {
        match &**node {
            Node::Internal { children } => {
                let subidx = (index >> level) & MASK;
                let mut new_children = children.clone();
                if let Some(child) = &children[subidx] {
                    new_children[subidx] = Some(self.do_set(level - BITS, child, index, value));
                }
                Arc::new(Node::Internal {
                    children: new_children,
                })
            }
            Node::Leaf { elements } => {
                let elem_idx = index & MASK;
                let mut new_elements = elements.clone();
                new_elements[elem_idx] = Some(value);
                Arc::new(Node::Leaf {
                    elements: new_elements,
                })
            }
        }
    }

    // =======================================================================
    // pop(): Remove the Last Element
    // =======================================================================
    //
    // Pop is the mirror of push:
    //
    // **Fast path:** If the tail has more than 1 element, clone the tail and
    // remove the last element. No trie modification. O(1).
    //
    // **Slow path (when tail has exactly 1 element):** After removing the last
    // tail element, the tail would be empty. We need to pull the rightmost
    // leaf from the trie to become the new tail. This may shrink the tree if
    // the root ends up with only one child.
    //
    //     pop() when tail = ["x", "y", "z"]:
    //     -> new tail = ["x", "y"]
    //     -> same root (Arc::clone)
    //     -> returns ("z", new_list)
    //
    //     pop() when tail = ["z"] (single element):
    //     -> pull rightmost leaf from trie as new tail
    //     -> possibly shrink tree depth
    //     -> returns ("z", new_list)

    /// Remove the last element. Returns (new_list, removed_element).
    ///
    /// Panics if the list is empty.
    ///
    /// Time: O(1) amortized. ~97% of calls just truncate the tail.
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let list = ImmutableList::new()
    ///     .push("a".to_string())
    ///     .push("b".to_string());
    /// let (popped, val) = list.pop();
    /// assert_eq!(val, "b");
    /// assert_eq!(popped.len(), 1);
    /// assert_eq!(list.len(), 2); // original unchanged
    /// ```
    pub fn pop(&self) -> (ImmutableList, String) {
        assert!(self.len > 0, "cannot pop from an empty list");

        // Special case: list has exactly 1 element. Pop to empty.
        if self.len == 1 {
            let val = self.tail[0].clone();
            return (ImmutableList::new(), val);
        }

        // Fast path: tail has more than 1 element. Just truncate.
        if self.tail.len() > 1 {
            let val = self.tail.last().unwrap().clone();
            let new_tail = self.tail[..self.tail.len() - 1].to_vec();
            return (
                ImmutableList {
                    root: Arc::clone(&self.root),
                    tail: new_tail,
                    len: self.len - 1,
                    shift: self.shift,
                },
                val,
            );
        }

        // Slow path: tail has exactly 1 element. After removal, we need to
        // pull the rightmost leaf from the trie as the new tail.
        let val = self.tail[0].clone();

        // Find the rightmost leaf in the trie and extract it.
        let new_tail = self.rightmost_leaf(&self.root, self.shift);

        // Remove the rightmost leaf from the trie (via path copying).
        let (new_root, new_shift) = self.pop_tail(self.len - 1, self.shift, &self.root);

        (
            ImmutableList {
                root: new_root,
                tail: new_tail,
                len: self.len - 1,
                shift: new_shift,
            },
            val,
        )
    }

    /// Extract the rightmost leaf's elements as a Vec<String>.
    ///
    /// This walks the rightmost path of the trie to find the last leaf node,
    /// then collects its non-None elements into a Vec.
    fn rightmost_leaf(&self, node: &Arc<Node>, level: u32) -> Vec<String> {
        match &**node {
            Node::Internal { children } => {
                // Find the rightmost non-None child.
                let mut idx = BRANCH_FACTOR - 1;
                while idx > 0 && children[idx].is_none() {
                    idx -= 1;
                }
                match &children[idx] {
                    Some(child) => self.rightmost_leaf(child, level - BITS),
                    None => Vec::new(),
                }
            }
            Node::Leaf { elements } => {
                // Collect all non-None elements.
                elements
                    .iter()
                    .filter_map(|e| e.clone())
                    .collect()
            }
        }
    }

    /// Remove the rightmost leaf from the trie (via path copying).
    ///
    /// Returns (new_root, new_shift). The shift may decrease if the root ends
    /// up with only its first child remaining after removal.
    fn pop_tail(
        &self,
        count: usize,
        level: u32,
        node: &Arc<Node>,
    ) -> (Arc<Node>, u32) {
        match &**node {
            Node::Internal { children } => {
                // Find the index of the rightmost leaf at this level.
                let subidx = ((count - 1) >> level) & MASK;

                if level == BITS {
                    // At the bottom internal level. Remove the child at subidx.
                    let mut new_children = children.clone();
                    new_children[subidx] = None;

                    // If this internal node is now completely empty (only when
                    // subidx == 0), the parent should remove us too. But the
                    // root handles this at the top level by checking for shrink.
                    if subidx == 0 {
                        // The trie is now empty at this level.
                        return (Arc::new(Node::new_internal()), BITS);
                    }

                    return (
                        Arc::new(Node::Internal {
                            children: new_children,
                        }),
                        level,
                    );
                }

                // Above the bottom level. Recurse into the child.
                if let Some(child) = &children[subidx] {
                    let (new_child, _) = self.pop_tail(count, level - BITS, child);

                    // Check if the child became empty (i.e., it's an internal
                    // node with all None children after removal).
                    let child_is_empty = match &*new_child {
                        Node::Internal { children: c } => c.iter().all(|x| x.is_none()),
                        _ => false,
                    };

                    let mut new_children = children.clone();
                    if child_is_empty && subidx > 0 {
                        new_children[subidx] = None;
                    } else if child_is_empty && subidx == 0 {
                        // Root's only subtree is now empty.
                        return (Arc::new(Node::new_internal()), BITS);
                    } else {
                        new_children[subidx] = Some(new_child);
                    }

                    // Check if the root should shrink. If only child[0] exists
                    // and the tree is deeper than 1, we can remove a level.
                    let should_shrink = level > BITS
                        && new_children[1..].iter().all(|c| c.is_none())
                        && new_children[0].is_some();

                    if should_shrink {
                        let sole_child = new_children[0].as_ref().unwrap().clone();
                        return (sole_child, level - BITS);
                    }

                    return (
                        Arc::new(Node::Internal {
                            children: new_children,
                        }),
                        level,
                    );
                }

                // Shouldn't reach here for valid tries.
                (Arc::clone(node), level)
            }
            Node::Leaf { .. } => {
                // Shouldn't reach a leaf during pop_tail -- we stop at level BITS.
                panic!("pop_tail reached leaf unexpectedly");
            }
        }
    }

    // =======================================================================
    // iter(): Iterate Over All Elements
    // =======================================================================
    //
    // Iteration visits every element in index order. We iterate through the
    // trie's leaves left-to-right, then through the tail buffer. The iterator
    // state is a simple index counter -- each call to next() does a get().
    //
    // A more efficient iterator would maintain a stack of trie nodes to avoid
    // re-traversing the trie from root for each element. But the simple
    // approach is correct and the trie is so shallow (at most ~6 levels) that
    // the overhead is minimal.

    /// Return an iterator over all elements in order.
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let list = ImmutableList::from_slice(&[
    ///     "a".to_string(), "b".to_string(), "c".to_string(),
    /// ]);
    /// let collected: Vec<&str> = list.iter().collect();
    /// assert_eq!(collected, vec!["a", "b", "c"]);
    /// ```
    pub fn iter(&self) -> ImmutableListIter<'_> {
        ImmutableListIter {
            list: self,
            index: 0,
        }
    }

    // =======================================================================
    // to_vec(): Collect into a plain Vec
    // =======================================================================

    /// Collect all elements into a Vec<String>.
    ///
    /// Time: O(n). Allocates a new Vec and copies all elements.
    ///
    /// ```
    /// use immutable_list::ImmutableList;
    /// let items = vec!["x".to_string(), "y".to_string()];
    /// let list = ImmutableList::from_slice(&items);
    /// assert_eq!(list.to_vec(), items);
    /// ```
    pub fn to_vec(&self) -> Vec<String> {
        self.iter().map(|s| s.to_string()).collect()
    }
}

// ---------------------------------------------------------------------------
// Default trait: ImmutableList::default() creates an empty list
// ---------------------------------------------------------------------------

impl Default for ImmutableList {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Iterator
// ---------------------------------------------------------------------------
//
// The iterator is a simple wrapper that calls get(index) for each element.
// This is O(log32 n) per call, making full iteration O(n * log32 n).
//
// For practical purposes this is fine: log32(1,000,000) is about 4, so full
// iteration of a million-element list does ~4M lookups instead of 1M. A
// stack-based iterator could achieve O(n) total, but the constant factor of
// the trie means the difference is negligible.

/// Iterator over the elements of an ImmutableList.
pub struct ImmutableListIter<'a> {
    list: &'a ImmutableList,
    index: usize,
}

impl<'a> Iterator for ImmutableListIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.len() {
            let val = self.list.get(self.index);
            self.index += 1;
            val
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.list.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for ImmutableListIter<'a> {}

// ---------------------------------------------------------------------------
// PartialEq and Eq
// ---------------------------------------------------------------------------
//
// Two lists are equal if they have the same length and all corresponding
// elements are equal. We compare element-by-element using the iterators.
//
// Note: two lists can be structurally different internally (different trie
// shapes from different construction histories) but still be equal if they
// contain the same elements in the same order.

impl PartialEq for ImmutableList {
    fn eq(&self, other: &Self) -> bool {
        // Short-circuit: different lengths are never equal.
        if self.len != other.len {
            return false;
        }

        // Check if they're the exact same object (same Arc pointer for root
        // and same tail). This is a fast identity check.
        if Arc::ptr_eq(&self.root, &other.root) && self.tail == other.tail {
            return true;
        }

        // Element-by-element comparison.
        self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl Eq for ImmutableList {}

// ---------------------------------------------------------------------------
// Display: human-readable representation
// ---------------------------------------------------------------------------
//
// Prints the list as: ImmutableList[elem0, elem1, elem2, ...]
// For long lists, this shows all elements. A production implementation might
// truncate after a certain number, but for educational purposes showing
// everything is more useful.

impl fmt::Display for ImmutableList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ImmutableList[")?;
        for (i, elem) in self.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", elem)?;
        }
        write!(f, "]")
    }
}

// ---------------------------------------------------------------------------
// Debug: shows internal structure for debugging
// ---------------------------------------------------------------------------
//
// The derived Debug on the fields would show the raw trie nodes, which is
// overwhelming. Instead we show a summary: length, shift, tail size, and
// the elements.

impl fmt::Debug for ImmutableList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImmutableList")
            .field("len", &self.len)
            .field("shift", &self.shift)
            .field("tail_len", &self.tail.len())
            .field("elements", &self.to_vec())
            .finish()
    }
}

// ===========================================================================
// Tests
// ===========================================================================
//
// The test suite covers:
//
// 1. Correctness: empty list, push, get, set, pop, iter, to_vec, from_slice
// 2. Structural sharing: push/set/pop preserve originals, clone independence
// 3. Boundary cases: 32, 33, 1024, 1025, 32768 elements
// 4. Edge cases: pop to empty, alternating push/pop
// 5. Performance smoke tests: 100K elements, clone is O(1)
//
// Each test group is prefixed with a comment explaining what's being verified
// and why the boundary matters.

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Basic operations: empty list, push, get, len, is_empty
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_list() {
        // An empty list should have len=0, is_empty=true, and return None
        // for any index access.
        let list = ImmutableList::new();
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
        assert_eq!(list.get(0), None);
        assert_eq!(list.get(100), None);
    }

    #[test]
    fn test_push_single() {
        // Pushing a single element: len becomes 1, get(0) returns it.
        let list = ImmutableList::new().push("hello".to_string());
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list.get(0), Some("hello"));
        assert_eq!(list.get(1), None);
    }

    #[test]
    fn test_push_multiple() {
        // Push several elements and verify each is at the correct index.
        let mut list = ImmutableList::new();
        for i in 0..10 {
            list = list.push(format!("item_{}", i));
        }
        assert_eq!(list.len(), 10);
        for i in 0..10 {
            assert_eq!(list.get(i), Some(format!("item_{}", i).as_str()));
        }
    }

    // -----------------------------------------------------------------------
    // Tail boundary: 32 elements (exactly fills the tail, no trie needed)
    // -----------------------------------------------------------------------
    //
    // With 32 elements, everything fits in the tail buffer. The trie's root
    // is an empty internal node. This is the maximum size before the first
    // tail promotion occurs.

    #[test]
    fn test_exactly_32_elements() {
        let mut list = ImmutableList::new();
        for i in 0..32 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 32);

        // Verify all elements accessible.
        for i in 0..32 {
            assert_eq!(list.get(i), Some(format!("e{}", i).as_str()));
        }

        // All elements should be in the tail (tail.len() == 32).
        assert_eq!(list.tail.len(), 32);
    }

    // -----------------------------------------------------------------------
    // First tail promotion: 33 elements (tail overflows, first leaf created)
    // -----------------------------------------------------------------------
    //
    // At 33 elements, the first 32 elements in the tail get promoted into the
    // trie as a leaf node, and element 32 (0-indexed) goes into a fresh tail.
    //
    // Before push of element 32:
    //     tail = [e0, e1, ..., e31]  (32 elements, FULL)
    //     trie = empty
    //
    // After push of element 32:
    //     tail = [e32]
    //     trie = root -> [Leaf: e0-e31]

    #[test]
    fn test_33_elements_first_promotion() {
        let mut list = ImmutableList::new();
        for i in 0..33 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 33);

        // The tail should have 1 element (the 33rd).
        assert_eq!(list.tail.len(), 1);
        assert_eq!(list.tail[0], "e32");

        // All elements accessible.
        for i in 0..33 {
            assert_eq!(list.get(i), Some(format!("e{}", i).as_str()));
        }
    }

    // -----------------------------------------------------------------------
    // Larger lists: 100, 1000, 1024, 1025 elements
    // -----------------------------------------------------------------------

    #[test]
    fn test_100_elements() {
        let mut list = ImmutableList::new();
        for i in 0..100 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 100);
        for i in 0..100 {
            assert_eq!(list.get(i), Some(format!("e{}", i).as_str()));
        }
    }

    #[test]
    fn test_1000_elements() {
        let mut list = ImmutableList::new();
        for i in 0..1000 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 1000);

        // Spot check various indices.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(31), Some("e31"));
        assert_eq!(list.get(32), Some("e32"));
        assert_eq!(list.get(500), Some("e500"));
        assert_eq!(list.get(999), Some("e999"));
        assert_eq!(list.get(1000), None);
    }

    // 1024 elements = 32 * 32 = fills depth-1 trie completely.
    // All 32 leaves are full, each with 32 elements.

    #[test]
    fn test_1024_elements_full_depth1() {
        let mut list = ImmutableList::new();
        for i in 0..1024 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 1024);

        // The tail should have 32 elements (the last leaf's worth).
        assert_eq!(list.tail.len(), 32);

        // Verify boundaries.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(1023), Some("e1023"));
    }

    // 1025 elements: the 1025th push promotes a full tail into the trie as
    // the 32nd leaf (filling depth-1 completely). The trie depth is still 1.
    //
    // Depth increase to 2 happens when we need a 33rd leaf, which occurs at
    // 1057 elements (32 full leaves in trie = 1024, + tail of 32, + 1 more).

    #[test]
    fn test_1025_elements() {
        let mut list = ImmutableList::new();
        for i in 0..1025 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 1025);

        // At 1025 elements: trie has 32 leaves (1024 elements), tail has 1.
        // Shift is still 5 (depth 1) because the root's 32 slots are full
        // but no overflow has occurred yet.
        assert_eq!(list.shift, BITS);

        // Verify all elements.
        for i in 0..1025 {
            assert_eq!(
                list.get(i),
                Some(format!("e{}", i).as_str()),
                "Failed at index {}",
                i
            );
        }
    }

    // 1057 elements triggers depth increase from 1 to 2.
    // At 1056 elements, the trie has 32 full leaves (1024 elements) and
    // the tail has 32 elements. The 1057th push tries to promote the tail
    // as leaf #33, which overflows the root's 32 slots, forcing a new level.

    #[test]
    fn test_1057_elements_depth_increase() {
        let mut list = ImmutableList::new();
        for i in 0..1057 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 1057);

        // Shift should now be 10 (depth 2).
        assert_eq!(list.shift, 10);

        // Verify all elements.
        for i in 0..1057 {
            assert_eq!(
                list.get(i),
                Some(format!("e{}", i).as_str()),
                "Failed at index {}",
                i
            );
        }
    }

    // -----------------------------------------------------------------------
    // set(): Replace an element with path copying
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_in_tail() {
        // Set an element that's in the tail buffer.
        let list = ImmutableList::from_slice(&[
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
        ]);
        let modified = list.set(1, "B".to_string());

        assert_eq!(modified.get(0), Some("a"));
        assert_eq!(modified.get(1), Some("B"));
        assert_eq!(modified.get(2), Some("c"));

        // Original unchanged.
        assert_eq!(list.get(1), Some("b"));
    }

    #[test]
    fn test_set_in_trie() {
        // Set an element in the trie (not in tail).
        let mut list = ImmutableList::new();
        for i in 0..64 {
            list = list.push(format!("e{}", i));
        }

        // Element 10 is in the trie (in the first leaf).
        let modified = list.set(10, "NEW_10".to_string());
        assert_eq!(modified.get(10), Some("NEW_10"));
        assert_eq!(list.get(10), Some("e10")); // original unchanged

        // All other elements unchanged in the modified version.
        for i in 0..64 {
            if i != 10 {
                assert_eq!(modified.get(i), Some(format!("e{}", i).as_str()));
            }
        }
    }

    #[test]
    fn test_set_at_boundaries() {
        let mut list = ImmutableList::new();
        for i in 0..100 {
            list = list.push(format!("e{}", i));
        }

        // Set first element.
        let m1 = list.set(0, "FIRST".to_string());
        assert_eq!(m1.get(0), Some("FIRST"));

        // Set element at tail boundary.
        let tail_start = list.tail_offset();
        let m2 = list.set(tail_start, "TAIL_START".to_string());
        assert_eq!(m2.get(tail_start), Some("TAIL_START"));

        // Set last element.
        let m3 = list.set(99, "LAST".to_string());
        assert_eq!(m3.get(99), Some("LAST"));
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn test_set_out_of_bounds() {
        let list = ImmutableList::new().push("x".to_string());
        list.set(1, "y".to_string());
    }

    // -----------------------------------------------------------------------
    // Structural sharing verification
    // -----------------------------------------------------------------------
    //
    // These tests verify that modifying one version of a list does NOT affect
    // other versions. This is the core guarantee of persistent data structures.

    #[test]
    fn test_push_preserves_original() {
        let a = ImmutableList::new().push("x".to_string());
        let b = a.push("y".to_string());
        let c = a.push("z".to_string());

        assert_eq!(a.len(), 1);
        assert_eq!(a.get(0), Some("x"));

        assert_eq!(b.len(), 2);
        assert_eq!(b.get(1), Some("y"));

        assert_eq!(c.len(), 2);
        assert_eq!(c.get(1), Some("z"));
    }

    #[test]
    fn test_set_preserves_original() {
        let a = ImmutableList::from_slice(&["a".to_string(), "b".to_string(), "c".to_string()]);
        let b = a.set(0, "X".to_string());

        assert_eq!(a.get(0), Some("a")); // original unchanged
        assert_eq!(b.get(0), Some("X"));
    }

    #[test]
    fn test_pop_preserves_original() {
        let a = ImmutableList::from_slice(&["a".to_string(), "b".to_string(), "c".to_string()]);
        let (b, val) = a.pop();

        assert_eq!(val, "c");
        assert_eq!(a.len(), 3); // original unchanged
        assert_eq!(b.len(), 2);
        assert_eq!(a.get(2), Some("c"));
    }

    #[test]
    fn test_clone_independence() {
        // Clone a list, modify the clone, verify the original is unchanged.
        let a = ImmutableList::from_slice(&["a".to_string(), "b".to_string()]);
        let b = a.clone();

        // Modify b via push.
        let c = b.push("c".to_string());

        assert_eq!(a.len(), 2); // a unchanged
        assert_eq!(b.len(), 2); // b unchanged (push returns new list)
        assert_eq!(c.len(), 3);
    }

    #[test]
    fn test_deep_structural_sharing() {
        // Build a large list, clone it, push to the clone.
        // Both should remain valid and independent.
        let mut list = ImmutableList::new();
        for i in 0..10_000 {
            list = list.push(format!("e{}", i));
        }

        let clone = list.clone();
        let extended = clone.push("extra".to_string());

        assert_eq!(list.len(), 10_000);
        assert_eq!(clone.len(), 10_000);
        assert_eq!(extended.len(), 10_001);

        // Verify a few elements in each.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(9999), Some("e9999"));
        assert_eq!(clone.get(5000), Some("e5000"));
        assert_eq!(extended.get(10_000), Some("extra"));
    }

    // -----------------------------------------------------------------------
    // pop(): Correctness and edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_pop_single_element() {
        let list = ImmutableList::new().push("only".to_string());
        let (popped, val) = list.pop();
        assert_eq!(val, "only");
        assert_eq!(popped.len(), 0);
        assert!(popped.is_empty());
    }

    #[test]
    fn test_pop_at_tail_boundary() {
        // Build a list with 33 elements (1 in tail after promotion).
        // Popping should leave 32 elements with a full tail pulled from trie.
        let mut list = ImmutableList::new();
        for i in 0..33 {
            list = list.push(format!("e{}", i));
        }

        let (popped, val) = list.pop();
        assert_eq!(val, "e32");
        assert_eq!(popped.len(), 32);

        // All remaining elements should be accessible.
        for i in 0..32 {
            assert_eq!(
                popped.get(i),
                Some(format!("e{}", i).as_str()),
                "Failed at index {} after pop",
                i
            );
        }
    }

    #[test]
    fn test_pop_to_empty() {
        // Push 100 elements, then pop all 100.
        let mut list = ImmutableList::new();
        for i in 0..100 {
            list = list.push(format!("e{}", i));
        }

        for i in (0..100).rev() {
            let (new_list, val) = list.pop();
            assert_eq!(val, format!("e{}", i));
            list = new_list;
        }

        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    #[should_panic(expected = "cannot pop from an empty list")]
    fn test_pop_empty_panics() {
        let list = ImmutableList::new();
        list.pop();
    }

    #[test]
    fn test_alternating_push_pop() {
        // Push 5, pop 3, push 5, pop 3. Verify state after each phase.
        let mut list = ImmutableList::new();

        // Push 5
        for i in 0..5 {
            list = list.push(format!("a{}", i));
        }
        assert_eq!(list.len(), 5);

        // Pop 3
        for _ in 0..3 {
            let (new_list, _) = list.pop();
            list = new_list;
        }
        assert_eq!(list.len(), 2);
        assert_eq!(list.get(0), Some("a0"));
        assert_eq!(list.get(1), Some("a1"));

        // Push 5 more
        for i in 0..5 {
            list = list.push(format!("b{}", i));
        }
        assert_eq!(list.len(), 7);

        // Pop 3
        for _ in 0..3 {
            let (new_list, _) = list.pop();
            list = new_list;
        }
        assert_eq!(list.len(), 4);
        assert_eq!(list.get(0), Some("a0"));
        assert_eq!(list.get(1), Some("a1"));
        assert_eq!(list.get(2), Some("b0"));
        assert_eq!(list.get(3), Some("b1"));
    }

    // -----------------------------------------------------------------------
    // iter() and to_vec()
    // -----------------------------------------------------------------------

    #[test]
    fn test_iter_empty() {
        let list = ImmutableList::new();
        let collected: Vec<&str> = list.iter().collect();
        assert!(collected.is_empty());
    }

    #[test]
    fn test_iter_correctness() {
        let mut list = ImmutableList::new();
        for i in 0..50 {
            list = list.push(format!("e{}", i));
        }

        let collected: Vec<&str> = list.iter().collect();
        assert_eq!(collected.len(), 50);
        for (i, val) in collected.iter().enumerate() {
            assert_eq!(*val, format!("e{}", i));
        }
    }

    #[test]
    fn test_iter_exact_size() {
        let list = ImmutableList::from_slice(&["a".to_string(), "b".to_string(), "c".to_string()]);
        let iter = list.iter();
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn test_to_vec_roundtrip() {
        let items: Vec<String> = (0..100).map(|i| format!("item_{}", i)).collect();
        let list = ImmutableList::from_slice(&items);
        assert_eq!(list.to_vec(), items);
    }

    // -----------------------------------------------------------------------
    // from_slice()
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_slice_empty() {
        let list = ImmutableList::from_slice(&[]);
        assert!(list.is_empty());
    }

    #[test]
    fn test_from_slice_large() {
        let items: Vec<String> = (0..1000).map(|i| format!("x{}", i)).collect();
        let list = ImmutableList::from_slice(&items);
        assert_eq!(list.len(), 1000);
        for i in 0..1000 {
            assert_eq!(list.get(i), Some(format!("x{}", i).as_str()));
        }
    }

    // -----------------------------------------------------------------------
    // Equality
    // -----------------------------------------------------------------------

    #[test]
    fn test_equality_same_elements() {
        let a = ImmutableList::from_slice(&["x".to_string(), "y".to_string()]);
        let b = ImmutableList::from_slice(&["x".to_string(), "y".to_string()]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_equality_different_elements() {
        let a = ImmutableList::from_slice(&["x".to_string(), "y".to_string()]);
        let b = ImmutableList::from_slice(&["x".to_string(), "z".to_string()]);
        assert_ne!(a, b);
    }

    #[test]
    fn test_equality_different_lengths() {
        let a = ImmutableList::from_slice(&["x".to_string()]);
        let b = ImmutableList::from_slice(&["x".to_string(), "y".to_string()]);
        assert_ne!(a, b);
    }

    #[test]
    fn test_equality_empty_lists() {
        let a = ImmutableList::new();
        let b = ImmutableList::new();
        assert_eq!(a, b);
    }

    #[test]
    fn test_equality_constructed_differently() {
        // Build the same list two different ways. They should be equal.
        let a = ImmutableList::from_slice(&["a".to_string(), "b".to_string(), "c".to_string()]);
        let b = ImmutableList::new()
            .push("a".to_string())
            .push("b".to_string())
            .push("c".to_string());
        assert_eq!(a, b);
    }

    // -----------------------------------------------------------------------
    // Display and Debug
    // -----------------------------------------------------------------------

    #[test]
    fn test_display() {
        let list = ImmutableList::from_slice(&["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(format!("{}", list), "ImmutableList[a, b, c]");
    }

    #[test]
    fn test_display_empty() {
        let list = ImmutableList::new();
        assert_eq!(format!("{}", list), "ImmutableList[]");
    }

    #[test]
    fn test_debug() {
        let list = ImmutableList::from_slice(&["x".to_string()]);
        let debug_str = format!("{:?}", list);
        assert!(debug_str.contains("ImmutableList"));
        assert!(debug_str.contains("len: 1"));
    }

    // -----------------------------------------------------------------------
    // Stress tests: large lists, depth transitions
    // -----------------------------------------------------------------------

    #[test]
    fn test_100k_elements() {
        // Push 100,000 elements and verify random access.
        let mut list = ImmutableList::new();
        for i in 0..100_000 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 100_000);

        // Spot check.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(31), Some("e31"));
        assert_eq!(list.get(32), Some("e32"));
        assert_eq!(list.get(1023), Some("e1023"));
        assert_eq!(list.get(1024), Some("e1024"));
        assert_eq!(list.get(32767), Some("e32767"));
        assert_eq!(list.get(32768), Some("e32768"));
        assert_eq!(list.get(99_999), Some("e99999"));
    }

    #[test]
    fn test_set_on_large_list() {
        // Build a large list, set several elements, verify correctness.
        let mut list = ImmutableList::new();
        for i in 0..1025 {
            list = list.push(format!("e{}", i));
        }

        let modified = list.set(0, "ZERO".to_string());
        let modified = modified.set(500, "FIVE_HUNDRED".to_string());
        let modified = modified.set(1024, "LAST".to_string());

        assert_eq!(modified.get(0), Some("ZERO"));
        assert_eq!(modified.get(500), Some("FIVE_HUNDRED"));
        assert_eq!(modified.get(1024), Some("LAST"));

        // Original unchanged.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(500), Some("e500"));
        assert_eq!(list.get(1024), Some("e1024"));
    }

    #[test]
    fn test_pop_across_depth_boundary() {
        // Build 1057 elements (depth 2), pop back to trigger depth decrease.
        let mut list = ImmutableList::new();
        for i in 0..1057 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.shift, 10); // depth 2

        // Pop elements until we're back to 1024 (depth should shrink back).
        for i in (1024..1057).rev() {
            let (new_list, val) = list.pop();
            assert_eq!(val, format!("e{}", i));
            list = new_list;
        }
        assert_eq!(list.len(), 1024);

        // Verify elements are still correct.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(1023), Some("e1023"));
    }

    // -----------------------------------------------------------------------
    // Performance smoke test: clone should be O(1)
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_is_cheap() {
        // Build a large list and clone it many times.
        // If clone were O(n), this would be very slow.
        let mut list = ImmutableList::new();
        for i in 0..100_000 {
            list = list.push(format!("e{}", i));
        }

        // Clone 1000 times. If each clone were O(n), this would copy
        // 100 million elements. With O(1) clone, it's instant.
        let mut clones = Vec::new();
        for _ in 0..1000 {
            clones.push(list.clone());
        }

        // Verify the original and a clone are equal.
        assert_eq!(list, clones[0]);
        assert_eq!(clones[999].len(), 100_000);
    }

    // -----------------------------------------------------------------------
    // 32768 elements: fills depth-2 trie completely (32^3 = 32768)
    // -----------------------------------------------------------------------

    #[test]
    fn test_32768_elements() {
        let mut list = ImmutableList::new();
        for i in 0..32_768 {
            list = list.push(format!("e{}", i));
        }
        assert_eq!(list.len(), 32_768);

        // Spot check boundaries.
        assert_eq!(list.get(0), Some("e0"));
        assert_eq!(list.get(1023), Some("e1023"));
        assert_eq!(list.get(1024), Some("e1024"));
        assert_eq!(list.get(32_767), Some("e32767"));
    }

    // -----------------------------------------------------------------------
    // Default trait
    // -----------------------------------------------------------------------

    #[test]
    fn test_default() {
        let list: ImmutableList = Default::default();
        assert!(list.is_empty());
    }
}
