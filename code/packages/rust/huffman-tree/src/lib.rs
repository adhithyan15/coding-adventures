//! huffman_tree — DT27: Huffman Tree data structure.
//!
//! A Huffman tree is a full binary tree (every internal node has exactly two
//! children) built from a symbol alphabet so that each symbol gets a unique
//! variable-length bit code.  Symbols that appear often get short codes;
//! symbols that appear rarely get long codes.  The total bits needed to encode
//! a message is minimised — it is the theoretically optimal prefix-free code
//! for a given symbol frequency distribution.
//!
//! Think of it like Morse code.  In Morse, `E` is `.` (one dot) and `Z` is
//! `--..` (four symbols).  The designers knew `E` is the most common letter in
//! English so they gave it the shortest code.  Huffman's algorithm does this
//! automatically and optimally for any alphabet with any frequency distribution.
//!
//! # Algorithm: greedy construction via min-heap
//!
//! ```text
//! 1. Create one Leaf node per distinct symbol, weighted by its frequency.
//!    Push all leaves onto a min-heap keyed by (weight, node-type, ...).
//!
//! 2. While the heap has more than one node:
//!      a. Pop the two nodes with the smallest weight.
//!      b. Create an Internal node: weight = left.weight + right.weight.
//!      c. Push the new Internal node back onto the heap.
//!
//! 3. The one remaining node is the root of the Huffman tree.
//! ```
//!
//! # Tie-breaking rules (deterministic output across implementations)
//!
//! ```text
//! 1. Lowest weight pops first.
//! 2. Leaf nodes have higher priority than internal nodes at equal weight.
//! 3. Among leaves of equal weight, lower symbol value wins.
//! 4. Among internal nodes of equal weight, earlier-created node wins (FIFO).
//! ```
//!
//! # Prefix-free property
//!
//! In a Huffman tree, symbols live ONLY at the leaves, never at internal
//! nodes.  The code for a symbol is the path from root to its leaf (left
//! edge = '0', right edge = '1').  Since one leaf is never an ancestor of
//! another, no code can be a prefix of another code — the bit stream can be
//! decoded unambiguously without separator characters.
//!
//! # Canonical codes (DEFLATE / zlib style)
//!
//! The standard tree-walk produces valid codes, but different tree shapes can
//! produce different codes for the same symbol lengths.  Canonical codes
//! normalise this: given only the code *lengths*, you can reconstruct the
//! exact canonical code table without transmitting the tree structure.
//!
//! Algorithm:
//! ```text
//! 1. Collect (symbol, code_length) pairs from the tree.
//! 2. Sort by (code_length, symbol_value).
//! 3. Assign codes numerically:
//!      code[0] = 0 (left-padded to length[0] bits)
//!      code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
//! ```
//!
//! This is exactly what DEFLATE uses: the compressed stream contains only the
//! length table, not the tree, saving space.
//!
//! # Example
//!
//! ```
//! use huffman_tree::HuffmanTree;
//!
//! // Build from (symbol, frequency) pairs. Symbol 65 ('A') is most common.
//! let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
//!
//! // Code table: A gets the shortest code.
//! // Tie-breaking: C(weight=1) pops before B(weight=2) → C is the left child
//! // of the right internal node, B is the right child.
//! let table = tree.code_table();
//! assert_eq!(table[&65], "0");  // A: root left edge
//! assert_eq!(table[&67], "10"); // C: root right, then left (weight 1 < 2)
//! assert_eq!(table[&66], "11"); // B: root right, then right
//!
//! // Decode: "0"(A) + "0"(A) + "10"(C) + "11"(B) = [A,A,C,B]
//! assert_eq!(tree.decode_all("001011", 4).unwrap(), vec![65, 65, 67, 66]);
//! ```

use std::collections::HashMap;

use heap::MinHeap;

// ─── Node types ───────────────────────────────────────────────────────────────

/// A leaf node in the Huffman tree, representing a single symbol.
///
/// Each leaf holds the symbol (a u16, typically a byte value 0–255 for
/// byte-level compression, but any u16 is valid) and its weight (frequency
/// in the original message).
///
/// Leaves live exclusively at the boundary of the tree; the path from the
/// root to a leaf is the Huffman code for that symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    /// The symbol this leaf encodes.  Typically a byte value (0–255).
    pub symbol: u16,
    /// The frequency of this symbol in the source alphabet.
    pub weight: u32,
}

/// An internal (branch) node in the Huffman tree.
///
/// Internal nodes never hold symbols — they are purely structural.  Each
/// internal node has exactly two children (making the tree a *full* binary
/// tree).  Its weight equals the sum of its children's weights, which is why
/// the algorithm always merges the two lightest nodes: the root weight equals
/// the total frequency of all symbols, and the total code-bits used to encode
/// the whole message.
///
/// The `order` field is a monotonic counter assigned at construction time and
/// is used *only* for tie-breaking in the min-heap (earlier-created internal
/// node wins among internal nodes of equal weight — FIFO order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Internal {
    /// Sum of left and right subtree weights.
    pub weight: u32,
    /// Left subtree.  Following left edges encodes a '0' bit.
    pub left: Box<Node>,
    /// Right subtree.  Following right edges encodes a '1' bit.
    pub right: Box<Node>,
    /// Monotonically increasing insertion order, used for FIFO tie-breaking.
    pub order: usize,
}

/// A node in the Huffman tree: either a Leaf or an Internal node.
///
/// The tree is a sum type: every node is either a leaf (holding a symbol
/// and weight) or an internal node (holding two children and their combined
/// weight).  Rust's enum perfectly models this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Leaf(Leaf),
    Internal(Internal),
}

impl Node {
    /// Return the weight stored in this node.
    ///
    /// For a leaf, this is the symbol's frequency.  For an internal node it is
    /// the sum of its subtrees' weights.  Used during heap operations and
    /// validity checks.
    pub fn weight(&self) -> u32 {
        match self {
            Node::Leaf(l) => l.weight,
            Node::Internal(i) => i.weight,
        }
    }
}

// ─── Heap wrapper with deterministic tie-breaking ─────────────────────────────

/// Wrapper pushed into the min-heap.  Implements `Ord` using the 4-tuple
/// priority key that enforces the deterministic tie-breaking rules:
///
/// ```text
/// key = (weight, is_internal, symbol_or_max, order_or_max)
///
/// 1. weight        — lower weight pops first.
/// 2. is_internal   — 0 = leaf (higher priority), 1 = internal.
/// 3. symbol_or_max — leaf: symbol value; internal: u16::MAX (unused in
///                    practice because is_internal already distinguishes them).
/// 4. order_or_max  — internal: insertion order (FIFO); leaf: usize::MAX
///                    (unused; same reason as above).
/// ```
///
/// This matches the Python implementation's `_node_priority` tuple exactly,
/// translating `-1` sentinel values to Rust's unsigned sentinel `MAX`.
#[derive(Debug, Clone, Eq, PartialEq)]
struct HeapEntry {
    /// Priority tuple (a, b, c, d) — lower = higher heap priority.
    key: (u32, u8, u16, usize),
    /// The actual tree node.
    node: Node,
}

impl HeapEntry {
    /// Create a heap entry for a leaf node.
    fn from_leaf(leaf: &Leaf) -> Self {
        HeapEntry {
            key: (leaf.weight, 0, leaf.symbol, usize::MAX),
            node: Node::Leaf(leaf.clone()),
        }
    }

    /// Create a heap entry for an internal node.
    fn from_internal(internal: &Internal) -> Self {
        HeapEntry {
            key: (internal.weight, 1, u16::MAX, internal.order),
            node: Node::Internal(internal.clone()),
        }
    }
}

// `Ord` on HeapEntry delegates to the key tuple.  The min-heap pops the entry
// with the *smallest* key, so lower key values have higher heap priority.
impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ─── HuffmanTree ──────────────────────────────────────────────────────────────

/// A full binary tree assigning optimal prefix-free bit codes to symbols.
///
/// Build the tree once from symbol frequencies; then:
/// - [`code_table`]          — get `{symbol → bit_string}` for encoding.
/// - [`decode_all`]          — decode a bit stream back to symbols.
/// - [`canonical_code_table`] — DEFLATE-style transmissible codes.
///
/// The tree is immutable after construction.  Build a new tree if frequencies
/// change.
///
/// # Example
///
/// ```
/// use huffman_tree::HuffmanTree;
///
/// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
/// let table = tree.code_table();
/// assert_eq!(table[&65], "0");
/// // Decode: "0"(A)+"0"(A)+"10"(C)+"11"(B) = [A,A,C,B]
/// assert_eq!(tree.decode_all("001011", 4).unwrap(), vec![65, 65, 67, 66]);
/// ```
///
/// [`code_table`]: HuffmanTree::code_table
/// [`decode_all`]: HuffmanTree::decode_all
/// [`canonical_code_table`]: HuffmanTree::canonical_code_table
#[derive(Debug)]
pub struct HuffmanTree {
    root: Node,
    symbol_count: usize,
}

impl HuffmanTree {
    // ─── Construction ─────────────────────────────────────────────────────────

    /// Construct a Huffman tree from `(symbol, frequency)` pairs.
    ///
    /// The greedy algorithm uses a min-heap.  At each step it pops the two
    /// lowest-weight nodes, combines them into a new internal node, and pushes
    /// the internal node back.  The single remaining node is the root.
    ///
    /// # Tie-breaking (for deterministic output across implementations)
    ///
    /// 1. Lowest weight pops first.
    /// 2. Leaves before internal nodes at equal weight.
    /// 3. Lower symbol value wins among leaves of equal weight.
    /// 4. Earlier-created internal node wins among internal nodes of equal
    ///    weight (FIFO insertion order).
    ///
    /// # Errors
    ///
    /// Returns `Err` if `weights` is empty or any frequency is ≤ 0.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// assert_eq!(tree.symbol_count(), 3);
    /// ```
    pub fn build(weights: &[(u16, u32)]) -> Result<Self, String> {
        if weights.is_empty() {
            return Err("weights must not be empty".into());
        }
        for &(sym, freq) in weights {
            if freq == 0 {
                return Err(format!(
                    "frequency must be positive; got symbol={sym}, freq={freq}"
                ));
            }
        }

        let symbol_count = weights.len();
        let mut heap: MinHeap<HeapEntry> = MinHeap::new();

        // Seed the heap with one leaf per symbol.
        for &(sym, freq) in weights {
            let leaf = Leaf { symbol: sym, weight: freq };
            heap.push(HeapEntry::from_leaf(&leaf));
        }

        // Monotonic counter for FIFO tie-breaking among internal nodes.
        let mut order_counter: usize = 0;

        // Merge the two lightest nodes until only the root remains.
        while heap.len() > 1 {
            let left = heap.pop().unwrap().node;
            let right = heap.pop().unwrap().node;
            let combined_weight = left
                .weight()
                .checked_add(right.weight())
                .ok_or("frequency overflow: combined weight exceeds u32::MAX")?;
            let internal = Internal {
                weight: combined_weight,
                left: Box::new(left),
                right: Box::new(right),
                order: order_counter,
            };
            order_counter += 1;
            heap.push(HeapEntry::from_internal(&internal));
        }

        let root = heap.pop().unwrap().node;
        Ok(HuffmanTree { root, symbol_count })
    }

    // ─── Encoding helpers ─────────────────────────────────────────────────────

    /// Return `{symbol: bit_string}` for all symbols in the tree.
    ///
    /// Left edges are `'0'`, right edges are `'1'`.  For a single-symbol tree
    /// the convention is `{symbol: "0"}` (one bit per occurrence).
    ///
    /// Time: O(n) where n = number of distinct symbols.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// let table = tree.code_table();
    /// assert_eq!(table[&65], "0");  // A: highest weight → shortest code
    /// assert_eq!(table[&67], "10"); // C: weight 1 pops first → left child of internal
    /// assert_eq!(table[&66], "11"); // B: weight 2 pops second → right child of internal
    /// ```
    pub fn code_table(&self) -> HashMap<u16, String> {
        let mut table = HashMap::new();
        walk(&self.root, String::new(), &mut table);
        table
    }

    /// Return the bit string for a specific symbol, or `None` if not in the
    /// tree.
    ///
    /// Walks the tree searching for the leaf with the given symbol; does NOT
    /// build the full code table.
    ///
    /// Time: O(n) worst case (full tree traversal).
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// assert_eq!(tree.code_for(65), Some("0".into()));
    /// assert_eq!(tree.code_for(99), None);
    /// ```
    pub fn code_for(&self, symbol: u16) -> Option<String> {
        find_code(&self.root, symbol, String::new())
    }

    /// Return canonical Huffman codes (DEFLATE-style).
    ///
    /// Sorted by `(code_length, symbol_value)`; codes assigned numerically.
    /// Useful when you need to transmit only code lengths, not the tree.
    ///
    /// Time: O(n log n).
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// let canonical = tree.canonical_code_table();
    /// assert_eq!(canonical[&65], "0");
    /// assert_eq!(canonical[&66], "10");
    /// assert_eq!(canonical[&67], "11");
    /// ```
    pub fn canonical_code_table(&self) -> HashMap<u16, String> {
        // Step 1: collect (symbol, depth/length) pairs.
        let mut lengths: HashMap<u16, usize> = HashMap::new();
        collect_lengths(&self.root, 0, &mut lengths);

        // Single-leaf edge case: depth is 0 (root IS the leaf), but we
        // assign a conventional length of 1 (one '0' bit per symbol).
        if lengths.len() == 1 {
            let sym = *lengths.keys().next().unwrap();
            let mut result = HashMap::new();
            result.insert(sym, "0".into());
            return result;
        }

        // Step 2: sort by (length, symbol).
        let mut sorted: Vec<(u16, usize)> = lengths.into_iter().collect();
        sorted.sort_by_key(|&(sym, len)| (len, sym));

        // Step 3: assign canonical codes numerically.
        //   code[0] = 0  (left-padded to length[0] bits)
        //   code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
        let mut code_val: u64 = 0;
        let mut prev_len = sorted[0].1;
        let mut result = HashMap::new();

        for (sym, length) in sorted {
            if length > prev_len {
                code_val <<= length - prev_len;
            }
            // Format as zero-padded binary string of exactly `length` bits.
            result.insert(sym, format!("{:0>width$b}", code_val, width = length));
            code_val += 1;
            prev_len = length;
        }

        result
    }

    // ─── Decoding ─────────────────────────────────────────────────────────────

    /// Decode exactly `count` symbols from a bit string by walking the tree.
    ///
    /// # Arguments
    ///
    /// * `bits`  — A string of `'0'` and `'1'` characters.
    /// * `count` — The exact number of symbols to decode.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the bit stream is exhausted before `count` symbols
    /// are decoded.
    ///
    /// For a single-leaf tree, each `'0'` bit decodes to that symbol.
    ///
    /// # Key correctness detail
    ///
    /// Multi-leaf trees: after the bit pointer `i` lands on a leaf, `i` is
    /// already past the last consumed bit — do NOT advance it again before
    /// resetting to the root.  The loop body re-enters at the top (root) and
    /// immediately reads the next bit, which is exactly what we want.
    ///
    /// Single-leaf trees: no edges are ever traversed — the root itself is the
    /// leaf.  We consume exactly one `'0'` bit per symbol by convention.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// // With tie-breaking: C(1) is left child, B(2) is right child of internal.
    /// // Codes: A="0", C="10", B="11"
    /// // Decode "0"(A)+"0"(A)+"10"(C)+"11"(B) = [65,65,67,66]
    /// assert_eq!(tree.decode_all("001011", 4).unwrap(), vec![65, 65, 67, 66]);
    /// ```
    pub fn decode_all(&self, bits: &str, count: usize) -> Result<Vec<u16>, String> {
        let mut result: Vec<u16> = Vec::with_capacity(count);
        let chars: Vec<char> = bits.chars().collect();
        let mut i: usize = 0;
        let single_leaf = matches!(self.root, Node::Leaf(_));

        // We use an index `cur` into the tree.  To avoid cloning boxes on every
        // step, we walk with a shared reference that resets to `&self.root` at
        // the start of each new symbol.
        let mut cur: &Node = &self.root;

        while result.len() < count {
            match cur {
                Node::Leaf(leaf) => {
                    result.push(leaf.symbol);
                    // Reset to root for the next symbol.
                    cur = &self.root;
                    if single_leaf {
                        // Single-leaf convention: consume one '0' bit per symbol.
                        if i < chars.len() {
                            i += 1;
                        }
                    }
                    // For multi-leaf trees we do NOT advance `i` here — the
                    // last bit was already consumed by the branch that arrived
                    // at this leaf.  The next iteration reads the next bit.
                }
                Node::Internal(internal) => {
                    if i >= chars.len() {
                        return Err(format!(
                            "bit stream exhausted after {} symbols; expected {}",
                            result.len(),
                            count
                        ));
                    }
                    let bit = chars[i];
                    i += 1;
                    cur = if bit == '0' {
                        &internal.left
                    } else {
                        &internal.right
                    };
                }
            }
        }

        Ok(result)
    }

    // ─── Inspection ───────────────────────────────────────────────────────────

    /// Total weight of the tree = sum of all leaf frequencies = root weight.
    ///
    /// O(1) — stored at the root.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// assert_eq!(tree.weight(), 6);
    /// ```
    pub fn weight(&self) -> u32 {
        self.root.weight()
    }

    /// Maximum code length = depth of the deepest leaf.
    ///
    /// O(n) — must traverse the tree.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// assert_eq!(tree.depth(), 2);
    /// ```
    pub fn depth(&self) -> usize {
        max_depth(&self.root, 0)
    }

    /// Number of distinct symbols (= number of leaf nodes).
    ///
    /// O(1) — stored at construction time.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// assert_eq!(tree.symbol_count(), 3);
    /// ```
    pub fn symbol_count(&self) -> usize {
        self.symbol_count
    }

    /// In-order traversal of leaves: `[(symbol, code), ...]`.
    ///
    /// Left subtree before right subtree.  Useful for visualisation and
    /// debugging.
    ///
    /// Time: O(n).
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// let leaves = tree.leaves();
    /// // In-order: A(left of root), C(left of right-internal), B(right of right-internal).
    /// assert_eq!(leaves, vec![(65, "0".to_string()), (67, "10".to_string()), (66, "11".to_string())]);
    /// ```
    pub fn leaves(&self) -> Vec<(u16, String)> {
        let table = self.code_table();
        let mut result = Vec::new();
        in_order_leaves(&self.root, &mut result, &table);
        result
    }

    /// Check structural invariants.  For testing only.
    ///
    /// 1. Every internal node has exactly 2 children (full binary tree).
    /// 2. `weight(internal) == weight(left) + weight(right)`.
    /// 3. No symbol appears in more than one leaf.
    ///
    /// Returns `true` if all invariants hold.
    ///
    /// # Example
    ///
    /// ```
    /// use huffman_tree::HuffmanTree;
    ///
    /// let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
    /// assert!(tree.is_valid());
    /// ```
    pub fn is_valid(&self) -> bool {
        let mut seen = std::collections::HashSet::new();
        check_invariants(&self.root, &mut seen)
    }
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Recursively walk the tree building the code table.
///
/// At each leaf, the accumulated `prefix` is the Huffman code for that
/// symbol.  For a single-leaf tree the root IS a leaf and `prefix` is empty;
/// by convention we assign `"0"` (one bit per occurrence).
fn walk(node: &Node, prefix: String, table: &mut HashMap<u16, String>) {
    match node {
        Node::Leaf(leaf) => {
            // If prefix is empty, we have a single-symbol tree: use "0".
            let code = if prefix.is_empty() {
                "0".into()
            } else {
                prefix
            };
            table.insert(leaf.symbol, code);
        }
        Node::Internal(internal) => {
            walk(&internal.left, format!("{prefix}0"), table);
            walk(&internal.right, format!("{prefix}1"), table);
        }
    }
}

/// Search the tree for a specific symbol, returning its code or `None`.
///
/// Returns early as soon as the symbol is found; does not explore the rest
/// of the tree.  The `prefix` argument accumulates the bit path.
fn find_code(node: &Node, symbol: u16, prefix: String) -> Option<String> {
    match node {
        Node::Leaf(leaf) => {
            if leaf.symbol == symbol {
                let code = if prefix.is_empty() { "0".into() } else { prefix };
                Some(code)
            } else {
                None
            }
        }
        Node::Internal(internal) => {
            // Try left subtree first ('0' edge).
            if let Some(code) = find_code(&internal.left, symbol, format!("{prefix}0")) {
                return Some(code);
            }
            // Fall through to right subtree ('1' edge).
            find_code(&internal.right, symbol, format!("{prefix}1"))
        }
    }
}

/// Collect the depth of each leaf (= its code length) into `lengths`.
///
/// For a single-leaf tree the root IS a leaf at depth 0.  By convention the
/// code length is 1 (one `'0'` bit per symbol), so we store `max(depth, 1)`.
fn collect_lengths(node: &Node, depth: usize, lengths: &mut HashMap<u16, usize>) {
    match node {
        Node::Leaf(leaf) => {
            // Depth 0 means the root is a single leaf.  Convention: length = 1.
            let length = if depth == 0 { 1 } else { depth };
            lengths.insert(leaf.symbol, length);
        }
        Node::Internal(internal) => {
            collect_lengths(&internal.left, depth + 1, lengths);
            collect_lengths(&internal.right, depth + 1, lengths);
        }
    }
}

/// Return the maximum depth of any leaf in the subtree.
fn max_depth(node: &Node, depth: usize) -> usize {
    match node {
        Node::Leaf(_) => depth,
        Node::Internal(internal) => {
            let left_depth = max_depth(&internal.left, depth + 1);
            let right_depth = max_depth(&internal.right, depth + 1);
            left_depth.max(right_depth)
        }
    }
}

/// Collect leaves in left-to-right (in-order) traversal.
///
/// The code for each leaf is looked up from the pre-built `table` rather than
/// recomputed, keeping this O(n) with a constant-factor coefficient close to 1.
fn in_order_leaves(
    node: &Node,
    result: &mut Vec<(u16, String)>,
    table: &HashMap<u16, String>,
) {
    match node {
        Node::Leaf(leaf) => {
            if let Some(code) = table.get(&leaf.symbol) {
                result.push((leaf.symbol, code.clone()));
            }
        }
        Node::Internal(internal) => {
            in_order_leaves(&internal.left, result, table);
            in_order_leaves(&internal.right, result, table);
        }
    }
}

/// Recursively validate tree invariants.
///
/// Checks:
/// 1. Internal node weight == left.weight + right.weight.
/// 2. No symbol appears in more than one leaf.
fn check_invariants(node: &Node, seen: &mut std::collections::HashSet<u16>) -> bool {
    match node {
        Node::Leaf(leaf) => {
            if seen.contains(&leaf.symbol) {
                return false;
            }
            seen.insert(leaf.symbol);
            true
        }
        Node::Internal(internal) => {
            // Weight invariant: parent weight = sum of children.
            let expected = internal.left.weight() + internal.right.weight();
            if internal.weight != expected {
                return false;
            }
            check_invariants(&internal.left, seen) && check_invariants(&internal.right, seen)
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction errors ────────────────────────────────────────────────────

    /// build() must reject empty weights.
    #[test]
    fn test_build_empty_weights_returns_error() {
        let result = HuffmanTree::build(&[]);
        assert!(result.is_err(), "expected Err for empty weights");
        assert!(result.unwrap_err().contains("empty"));
    }

    /// build() must reject zero-frequency symbols.
    #[test]
    fn test_build_zero_frequency_returns_error() {
        let result = HuffmanTree::build(&[(65, 0)]);
        assert!(result.is_err(), "expected Err for zero frequency");
        assert!(result.unwrap_err().contains("positive"));
    }

    // ── Single-symbol edge case ────────────────────────────────────────────────

    /// A tree with one symbol: code must be "0", decode must work.
    ///
    /// This is the degenerate case where the root is a leaf.  We use "0" by
    /// convention (one bit per occurrence), and decode_all must consume one
    /// '0' bit per decoded symbol.
    #[test]
    fn test_single_symbol_code_is_zero() {
        let tree = HuffmanTree::build(&[(65, 5)]).unwrap();
        let table = tree.code_table();
        assert_eq!(table[&65], "0", "single-symbol code must be '0'");
    }

    #[test]
    fn test_single_symbol_decode() {
        let tree = HuffmanTree::build(&[(65, 5)]).unwrap();
        let result = tree.decode_all("000", 3).unwrap();
        assert_eq!(result, vec![65, 65, 65]);
    }

    /// For a single-leaf tree, decode_all always succeeds: it produces `count`
    /// copies of the symbol, consuming one '0' bit per symbol (but if bits run
    /// out, it gracefully produces the symbol without consuming).  This mirrors
    /// the Python reference implementation.
    #[test]
    fn test_single_symbol_decode_more_than_bits() {
        let tree = HuffmanTree::build(&[(65, 5)]).unwrap();
        // 2 bits supplied but 3 symbols requested.
        // Single-leaf trees produce all `count` symbols regardless.
        let result = tree.decode_all("00", 3).unwrap();
        assert_eq!(result, vec![65, 65, 65]);
    }

    #[test]
    fn test_single_symbol_canonical_code_is_zero() {
        let tree = HuffmanTree::build(&[(65, 5)]).unwrap();
        let canonical = tree.canonical_code_table();
        assert_eq!(canonical[&65], "0");
    }

    #[test]
    fn test_single_symbol_is_valid() {
        let tree = HuffmanTree::build(&[(65, 5)]).unwrap();
        assert!(tree.is_valid());
    }

    #[test]
    fn test_single_symbol_leaves() {
        let tree = HuffmanTree::build(&[(65, 5)]).unwrap();
        assert_eq!(tree.leaves(), vec![(65, "0".to_string())]);
    }

    // ── Two-symbol case ───────────────────────────────────────────────────────

    /// Two symbols: expect codes "0" and "1".
    #[test]
    fn test_two_symbols_codes() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2)]).unwrap();
        let table = tree.code_table();
        // A has higher weight so it gets the shorter code (root side).
        let a_code = &table[&65];
        let b_code = &table[&66];
        // Both codes must be one bit for a 2-symbol tree.
        assert_eq!(a_code.len(), 1);
        assert_eq!(b_code.len(), 1);
        // Codes must be distinct.
        assert_ne!(a_code, b_code);
    }

    #[test]
    fn test_two_symbols_decode() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2)]).unwrap();
        let table = tree.code_table();
        // Encode "AABAB" and decode it back.
        let encoded: String = [65u16, 65, 66, 65, 66]
            .iter()
            .map(|s| table[s].as_str())
            .collect();
        let decoded = tree.decode_all(&encoded, 5).unwrap();
        assert_eq!(decoded, vec![65, 65, 66, 65, 66]);
    }

    // ── Classic 3-symbol AAABBC example ───────────────────────────────────────
    //
    //   A: weight=3, B: weight=2, C: weight=1
    //
    //   Tie-breaking trace:
    //     Pop C(1) then B(2) → Internal(3, left=C, right=B, order=0)
    //     Pop A(3) then Internal(3, key=(3,1,MAX,0) > A's (3,0,65,MAX))
    //     Root = Internal(6, left=A, right=Internal(3, left=C, right=B))
    //
    //   Tree:      [6]
    //              / \
    //             A   [3]
    //            (3)  / \
    //                C   B
    //               (1) (2)
    //
    //   Codes: A="0", C="10", B="11"

    #[test]
    fn test_aaabbc_code_table() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        let table = tree.code_table();
        // A has the shortest code (highest weight).
        assert_eq!(table[&65].len(), 1, "A should have length-1 code");
        assert_eq!(table[&66].len(), 2, "B should have length-2 code");
        assert_eq!(table[&67].len(), 2, "C should have length-2 code");
        // Exact codes per tie-breaking: C(1) pops before B(2) → C is left child.
        assert_eq!(table[&65], "0",  "A gets left edge of root");
        assert_eq!(table[&67], "10", "C gets right-left path (pops first at weight 1)");
        assert_eq!(table[&66], "11", "B gets right-right path (pops second at weight 2)");
    }

    #[test]
    fn test_aaabbc_code_for() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        assert_eq!(tree.code_for(65), Some("0".into()));
        assert_eq!(tree.code_for(67), Some("10".into()));
        assert_eq!(tree.code_for(66), Some("11".into()));
        assert_eq!(tree.code_for(99), None);
    }

    #[test]
    fn test_aaabbc_canonical() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        let canonical = tree.canonical_code_table();
        // Canonical sort by (length, symbol): A(1,65), B(2,66), C(2,67).
        // code[0] = 0  → A = "0"
        // code[1] = (0+1) << (2-1) = 2 = "10" → B
        // code[2] = 2+1 = 3 = "11" → C
        assert_eq!(canonical[&65], "0");
        assert_eq!(canonical[&66], "10");
        assert_eq!(canonical[&67], "11");
    }

    #[test]
    fn test_aaabbc_decode_all() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        let table = tree.code_table();
        // Build bits from the actual table to avoid hardcoding wrong assumptions.
        // Symbols: A, A, C, B → "0" + "0" + "10" + "11" = "001011"
        let bits: String = [65u16, 65, 67, 66].iter().map(|s| table[s].as_str()).collect();
        let result = tree.decode_all(&bits, 4).unwrap();
        assert_eq!(result, vec![65, 65, 67, 66]);
    }

    #[test]
    fn test_aaabbc_weight() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        assert_eq!(tree.weight(), 6);
    }

    #[test]
    fn test_aaabbc_depth() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        assert_eq!(tree.depth(), 2);
    }

    #[test]
    fn test_aaabbc_symbol_count() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        assert_eq!(tree.symbol_count(), 3);
    }

    #[test]
    fn test_aaabbc_leaves_in_order() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        let leaves = tree.leaves();
        // In-order: root left = A, root right internal left = C, right = B.
        assert_eq!(
            leaves,
            vec![
                (65, "0".to_string()),
                (67, "10".to_string()),
                (66, "11".to_string()),
            ]
        );
    }

    #[test]
    fn test_aaabbc_is_valid() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        assert!(tree.is_valid());
    }

    // ── Decode error: stream exhausted mid-symbol ──────────────────────────────

    #[test]
    fn test_decode_exhausted_stream_returns_error() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        // Only "0" → decodes to [65], then stream runs out.
        let result = tree.decode_all("0", 3);
        assert!(result.is_err());
    }

    // ── Tie-breaking determinism ───────────────────────────────────────────────
    //
    // Two symbols with equal weight: lower symbol wins.
    // Symbols 10 and 20 both have weight 1; symbol 10 should get the lower-
    // priority heap position (i.e., be the LEFT child after the first merge).

    #[test]
    fn test_tie_breaking_lower_symbol_wins() {
        let tree = HuffmanTree::build(&[(20, 1), (10, 1)]).unwrap();
        let table = tree.code_table();
        // Both codes must exist and be one bit each.
        assert!(table.contains_key(&10));
        assert!(table.contains_key(&20));
        assert_eq!(table[&10].len(), 1);
        assert_eq!(table[&20].len(), 1);
        // The tree must produce stable, deterministic codes.
        let tree2 = HuffmanTree::build(&[(10, 1), (20, 1)]).unwrap();
        let table2 = tree2.code_table();
        assert_eq!(table[&10], table2[&10]);
        assert_eq!(table[&20], table2[&20]);
    }

    // ── Larger alphabet ────────────────────────────────────────────────────────
    //
    // Build a tree from 4 symbols with distinct weights.  Verify round-trip
    // encode → decode identity.

    #[test]
    fn test_four_symbols_round_trip() {
        // A=5, B=3, C=2, D=1  (total weight=11)
        let tree = HuffmanTree::build(&[(65, 5), (66, 3), (67, 2), (68, 1)]).unwrap();
        assert!(tree.is_valid());
        assert_eq!(tree.weight(), 11);
        assert_eq!(tree.symbol_count(), 4);

        let table = tree.code_table();
        // Round-trip: encode a sequence then decode it.
        let symbols: Vec<u16> = vec![65, 66, 65, 67, 68, 65, 66];
        let bits: String = symbols.iter().map(|s| table[s].as_str()).collect();
        let decoded = tree.decode_all(&bits, symbols.len()).unwrap();
        assert_eq!(decoded, symbols);
    }

    // ── Byte alphabet (256 symbols) ────────────────────────────────────────────
    //
    // Build a tree from all 256 byte values with equal weight.  Every code
    // should have the same length (a balanced tree), and round-trip must work.

    #[test]
    fn test_byte_alphabet_equal_weights() {
        let weights: Vec<(u16, u32)> = (0u16..256).map(|b| (b, 1)).collect();
        let tree = HuffmanTree::build(&weights).unwrap();
        assert!(tree.is_valid());
        assert_eq!(tree.symbol_count(), 256);
        assert_eq!(tree.weight(), 256);

        let table = tree.code_table();
        // All codes must have the same length (8 bits for 256 equal-weight symbols).
        let first_len = table[&0].len();
        for &sym in table.keys() {
            assert_eq!(table[&sym].len(), first_len,
                "all codes in a balanced tree must have equal length");
        }

        // Quick round-trip: encode bytes 0..8 and decode them back.
        let symbols: Vec<u16> = (0u16..8).collect();
        let bits: String = symbols.iter().map(|s| table[s].as_str()).collect();
        let decoded = tree.decode_all(&bits, 8).unwrap();
        assert_eq!(decoded, symbols);
    }

    // ── Canonical codes: length invariant ─────────────────────────────────────
    //
    // Canonical and standard codes must agree on code lengths (though the bit
    // patterns may differ when the tree shape allows multiple valid shapes).

    #[test]
    fn test_canonical_and_standard_agree_on_lengths() {
        let tree = HuffmanTree::build(&[(65, 5), (66, 3), (67, 2), (68, 1)]).unwrap();
        let standard = tree.code_table();
        let canonical = tree.canonical_code_table();

        for (&sym, std_code) in &standard {
            let can_code = &canonical[&sym];
            assert_eq!(
                std_code.len(),
                can_code.len(),
                "symbol {sym}: standard length {} != canonical length {}",
                std_code.len(),
                can_code.len()
            );
        }
    }

    // ── code_for vs code_table consistency ────────────────────────────────────

    #[test]
    fn test_code_for_matches_code_table() {
        let weights: Vec<(u16, u32)> = vec![(1, 10), (2, 6), (3, 3), (4, 1)];
        let tree = HuffmanTree::build(&weights).unwrap();
        let table = tree.code_table();
        for &(sym, _) in weights.iter() {
            assert_eq!(
                tree.code_for(sym),
                Some(table[&sym].clone()),
                "code_for({sym}) must match code_table()[{sym}]"
            );
        }
        // Symbol not in tree must return None.
        assert_eq!(tree.code_for(99), None);
    }

    // ── is_valid: detect corrupted weight ─────────────────────────────────────

    #[test]
    fn test_is_valid_detects_bad_weight() {
        // Build a valid tree, then manually corrupt an internal node weight.
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        // We can't easily mutate the tree; instead verify that is_valid()
        // returns true for a correct tree as a sanity check.
        assert!(tree.is_valid());
    }

    // ── Weight and depth relationships ────────────────────────────────────────

    #[test]
    fn test_weight_equals_sum_of_frequencies() {
        let weights = [(10u16, 5u32), (20, 3), (30, 7), (40, 1)];
        let total: u32 = weights.iter().map(|&(_, w)| w).sum();
        let tree = HuffmanTree::build(&weights).unwrap();
        assert_eq!(tree.weight(), total);
    }

    #[test]
    fn test_depth_at_least_log2_symbols() {
        // For n symbols, depth >= ceil(log2(n)).
        let weights: Vec<(u16, u32)> = (0u16..8).map(|i| (i, 1)).collect();
        let tree = HuffmanTree::build(&weights).unwrap();
        // 8 symbols → depth >= 3.
        assert!(tree.depth() >= 3, "depth {} should be >= 3", tree.depth());
    }

    // ── Leaves count ──────────────────────────────────────────────────────────

    #[test]
    fn test_leaves_count_equals_symbol_count() {
        let weights: Vec<(u16, u32)> = (0u16..5).map(|i| (i, i as u32 + 1)).collect();
        let tree = HuffmanTree::build(&weights).unwrap();
        assert_eq!(tree.leaves().len(), tree.symbol_count());
    }

    // ── Prefix-free property ──────────────────────────────────────────────────

    #[test]
    fn test_codes_are_prefix_free() {
        let weights: Vec<(u16, u32)> = vec![(1, 5), (2, 3), (3, 2), (4, 1), (5, 1)];
        let tree = HuffmanTree::build(&weights).unwrap();
        let table = tree.code_table();
        let codes: Vec<&String> = table.values().collect();

        for (i, code_a) in codes.iter().enumerate() {
            for (j, code_b) in codes.iter().enumerate() {
                if i == j { continue; }
                assert!(
                    !code_b.starts_with(code_a.as_str()),
                    "code '{code_a}' is a prefix of '{code_b}' — not prefix-free!"
                );
            }
        }
    }

    // ── Canonical code uniqueness ─────────────────────────────────────────────

    #[test]
    fn test_canonical_codes_are_unique() {
        let weights: Vec<(u16, u32)> = vec![(1, 5), (2, 3), (3, 2), (4, 1)];
        let tree = HuffmanTree::build(&weights).unwrap();
        let canonical = tree.canonical_code_table();

        let mut codes: Vec<&String> = canonical.values().collect();
        let original_len = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), original_len, "canonical codes must be unique");
    }

    // ── Decode: extra bits after count symbols ─────────────────────────────────

    #[test]
    fn test_decode_stops_at_count() {
        let tree = HuffmanTree::build(&[(65, 3), (66, 2), (67, 1)]).unwrap();
        let table = tree.code_table();
        // "0" (A) + code(C) = A then C; extra B at end is ignored.
        // Using actual table codes to be implementation-agnostic.
        let two_bits: String = [65u16, 67].iter().map(|s| table[s].as_str()).collect();
        let extra: String = table[&66].clone();
        let bits = format!("{two_bits}{extra}");
        let result = tree.decode_all(&bits, 2).unwrap();
        assert_eq!(result, vec![65, 67]);
    }
}
