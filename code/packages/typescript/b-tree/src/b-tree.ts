/**
 * B-Tree — DT11
 * =============
 *
 * A B-tree is a self-balancing search tree generalisation of a binary search
 * tree (BST). Where a BST node holds exactly one key and has at most two
 * children, a B-tree node can hold many keys and many children.
 *
 * ## Why B-trees exist
 *
 * Hard drives and SSDs transfer data in large fixed-size "pages" (typically
 * 4 KB–64 KB). Reading a single byte still costs one full page-read. A BST
 * with a million nodes needs 20 levels of traversal, i.e. 20 separate page
 * reads. A B-tree with branching factor 100 needs only 3 levels, so 3 page
 * reads — a 7× win in I/O operations. This is why every serious database
 * (PostgreSQL, MySQL, SQLite, Oracle, SQL Server) stores its indexes as
 * B-trees or B+ trees.
 *
 * ## Terminology
 *
 * - **Minimum degree `t`**: the parameter that controls node size.
 *   - Every non-root node has at least `t − 1` keys.
 *   - Every node (including root) has at most `2t − 1` keys.
 *   - A full node has exactly `2t − 1` keys.
 * - **Internal node**: a node that has children.
 * - **Leaf node**: a node with no children (`isLeaf === true`).
 *
 * ## Node anatomy (t = 2, so each node has 1–3 keys)
 *
 * ```
 *      [ k1 | k2 | k3 ]       ← keys array (max 2t-1 = 3)
 *      /    |    |    \
 *    c0   c1   c2   c3         ← children array (max 2t = 4)
 * ```
 *
 * The children array has exactly one more element than the keys array. Child
 * `c[i]` holds keys strictly less than `keys[i]`, and `c[i+1]` holds keys
 * strictly greater than `keys[i]`.
 *
 * ## Invariants
 *
 * 1. All leaves are at the same depth.
 * 2. Every non-root node has ≥ t − 1 keys.
 * 3. Every node has ≤ 2t − 1 keys.
 * 4. Keys within a node are sorted in ascending order.
 * 5. Key ordering is respected across subtrees (BST property).
 *
 * ## Insert strategy — proactive top-down splitting
 *
 * The classic approach (Cormen et al., CLRS 4th ed., Section 18.3) splits full
 * nodes on the way DOWN during a single pass. This avoids a second upward pass
 * and keeps the algorithm cache-friendly. The rules are:
 *
 * 1. If the root is full, split it before descending. The tree grows taller.
 * 2. Before descending into any child, if that child is full, split it first.
 * 3. Insert the key into the appropriate leaf.
 *
 * ## Delete strategy
 *
 * Deletion requires more case analysis. For a key `k` in node `x`:
 *
 * **Case 1** — `k` is in a leaf. Just remove it.
 *
 * **Case 2** — `k` is in an internal node `x`.
 *   - 2a: Predecessor child `x.children[i]` has ≥ t keys. Replace `k` with
 *         its in-order predecessor `k'`, then delete `k'` recursively.
 *   - 2b: Successor child `x.children[i+1]` has ≥ t keys. Replace `k` with
 *         its in-order successor `k'`, then delete `k'` recursively.
 *   - 2c: Both children have only t−1 keys. Merge them (and `k`) into a
 *         single child of 2t−1 keys, then delete `k` from the merged child.
 *
 * **Case 3** — `k` is not present in the current node. Descend into child
 * `x.children[i]` that would contain `k`.
 *   - First guarantee that child has ≥ t keys:
 *   - 3a: If a sibling has ≥ t keys, rotate a key from parent down and a key
 *         from sibling up.
 *   - 3b: If both siblings have t−1 keys, merge with a sibling.
 *
 * @module b-tree
 */

// ---------------------------------------------------------------------------
// Node interface
// ---------------------------------------------------------------------------

/**
 * A single node in the B-tree.
 *
 * We store `keys` and `values` as parallel arrays: `keys[i]` maps to
 * `values[i]`. Children are stored in `children`; a leaf always has
 * `children === []`.
 *
 * Example (t = 2, three keys in one node):
 *
 * ```
 *   keys:     [10,  30,  50]
 *   values:   ['a', 'b', 'c']
 *   children: [n0, n1, n2, n3]   ← 4 children for 3 keys
 * ```
 */
export interface BTreeNode<K, V> {
  /** Sorted search keys. */
  keys: K[];
  /** Values parallel to keys — keys[i] ↔ values[i]. */
  values: V[];
  /** Child pointers. Length is always `keys.length + 1` for internal nodes, 0 for leaves. */
  children: BTreeNode<K, V>[];
  /** True when this node has no children. */
  isLeaf: boolean;
}

// ---------------------------------------------------------------------------
// BTree class
// ---------------------------------------------------------------------------

/**
 * A generic B-tree keyed on `K` with associated values `V`.
 *
 * ### Quick start
 *
 * ```typescript
 * const tree = new BTree<number, string>(2, (a, b) => a - b);
 * tree.insert(10, "ten");
 * tree.insert(20, "twenty");
 * tree.insert(5,  "five");
 *
 * console.log(tree.search(10));   // "ten"
 * console.log(tree.size);         // 3
 * console.log(tree.inorder());    // [[5, "five"], [10, "ten"], [20, "twenty"]]
 * ```
 *
 * ### Choosing `t`
 *
 * | `t` | Min keys/node | Max keys/node | Max children |
 * |-----|---------------|---------------|--------------|
 * |  2  |       1       |       3       |      4       |
 * |  3  |       2       |       5       |      6       |
 * |  5  |       4       |       9       |     10       |
 *
 * For in-memory use, `t = 2` or `t = 3` is fine. For disk-backed storage, pick
 * `t` so that one B-tree node fills exactly one disk page.
 *
 * @typeParam K - Key type. Must be totally ordered via `compareFn`.
 * @typeParam V - Value type. Can be anything.
 */
export class BTree<K, V> {
  /** The root node of the tree. May be a leaf when empty or tiny. */
  private root: BTreeNode<K, V>;

  /** Number of key-value pairs stored in the tree. */
  private _size = 0;

  /**
   * Creates a new B-tree.
   *
   * @param t          Minimum degree (default 2). Must be ≥ 2.
   * @param compareFn  A comparator: `(a, b) => negative | 0 | positive`.
   *                   Works just like `Array.prototype.sort`'s compareFn.
   *
   * @example
   * // Numeric keys
   * new BTree<number, string>(2, (a, b) => a - b);
   *
   * // String keys (lexicographic)
   * new BTree<string, number>(3, (a, b) => a.localeCompare(b));
   */
  constructor(
    private readonly t: number = 2,
    private readonly compareFn: (a: K, b: K) => number
  ) {
    if (t < 2) throw new RangeError("BTree minimum degree t must be ≥ 2");
    this.root = this.makeLeaf();
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * The number of key-value pairs stored.
   *
   * O(1) — maintained incrementally.
   */
  get size(): number {
    return this._size;
  }

  /**
   * Returns the value associated with `key`, or `undefined` if not found.
   *
   * Time complexity: O(t · log_t(n)) — one comparison per key per level.
   *
   * @example
   * tree.insert(42, "answer");
   * tree.search(42);   // "answer"
   * tree.search(99);   // undefined
   */
  search(key: K): V | undefined {
    return this.searchNode(this.root, key);
  }

  /**
   * Returns `true` if `key` exists in the tree.
   *
   * @example
   * tree.contains(42);   // true
   * tree.contains(999);  // false
   */
  contains(key: K): boolean {
    return this.search(key) !== undefined;
  }

  /**
   * Inserts `key → value` into the tree.
   *
   * If `key` already exists, its value is **updated** (upsert semantics).
   *
   * Uses proactive top-down splitting: full nodes are split on the way down
   * so no second upward pass is needed.
   *
   * Time complexity: O(t · log_t(n)).
   *
   * @example
   * tree.insert(1, "one");
   * tree.insert(1, "ONE");   // overwrites
   * tree.search(1);           // "ONE"
   */
  insert(key: K, value: V): void {
    // If the root is full (2t-1 keys), we must split it.
    // This is the only way the tree grows taller.
    //
    // Before split (t=2, root full with 3 keys):
    //
    //   [10 | 20 | 30]
    //
    // After split (median 20 becomes new root):
    //
    //        [20]
    //       /    \
    //    [10]   [30]
    //
    if (this.isFull(this.root)) {
      const oldRoot = this.root;
      const newRoot = this.makeInternal([]);
      newRoot.children.push(oldRoot);
      this.splitChild(newRoot, 0);
      this.root = newRoot;
    }
    const existed = this.insertNonFull(this.root, key, value);
    if (!existed) this._size++;
  }

  /**
   * Removes `key` from the tree. Returns `true` if the key was found and
   * removed, `false` if it did not exist.
   *
   * Implements all three CLRS deletion cases with sub-cases 2a/2b/2c and
   * 3a/3b.
   *
   * Time complexity: O(t · log_t(n)).
   *
   * @example
   * tree.insert(5, "five");
   * tree.delete(5);    // true
   * tree.delete(5);    // false (already gone)
   */
  delete(key: K): boolean {
    const existed = this.deleteKey(this.root, key);
    if (existed) {
      this._size--;
      // If the root has no keys but has a child (after a merge), shrink
      // the tree height by making that child the new root.
      if (this.root.keys.length === 0 && !this.root.isLeaf) {
        this.root = this.root.children[0];
      }
    }
    return existed;
  }

  /**
   * Returns the smallest key in the tree, or `undefined` if empty.
   *
   * O(h) where h = height.
   *
   * @example
   * tree.insert(30, "c"); tree.insert(10, "a"); tree.insert(20, "b");
   * tree.minKey();   // 10
   */
  minKey(): K | undefined {
    if (this._size === 0) return undefined;
    let node = this.root;
    while (!node.isLeaf) node = node.children[0];
    return node.keys[0];
  }

  /**
   * Returns the largest key in the tree, or `undefined` if empty.
   *
   * O(h) where h = height.
   *
   * @example
   * tree.insert(30, "c"); tree.insert(10, "a"); tree.insert(20, "b");
   * tree.maxKey();   // 30
   */
  maxKey(): K | undefined {
    if (this._size === 0) return undefined;
    let node = this.root;
    while (!node.isLeaf) node = node.children[node.children.length - 1];
    return node.keys[node.keys.length - 1];
  }

  /**
   * Returns all `[key, value]` pairs with `low ≤ key ≤ high`, in sorted order.
   *
   * Traverses the tree in-order, pruning subtrees outside the range.
   * O(t · log_t(n) + m) where m is the number of matching pairs.
   *
   * @example
   * // Insert 1–10, query [3, 7]
   * tree.rangeQuery(3, 7);   // [[3,v], [4,v], [5,v], [6,v], [7,v]]
   */
  rangeQuery(low: K, high: K): Array<[K, V]> {
    const result: Array<[K, V]> = [];
    this.rangeNode(this.root, low, high, result);
    return result;
  }

  /**
   * Returns all `[key, value]` pairs in ascending key order.
   *
   * O(n) — visits every node exactly once.
   *
   * @example
   * tree.inorder();   // [[k1, v1], [k2, v2], ...]
   */
  inorder(): Array<[K, V]> {
    const result: Array<[K, V]> = [];
    this.inorderNode(this.root, result);
    return result;
  }

  /**
   * Returns the height of the tree.
   *
   * Height is defined as the number of edges on the longest root-to-leaf path.
   * An empty tree (root is a leaf with 0 keys) has height 0.
   *
   * O(h) — follows the leftmost path.
   *
   * @example
   * // Fresh empty tree
   * tree.height();   // 0
   * // After many inserts that cause splits and a new root
   * tree.height();   // ≥ 1
   */
  height(): number {
    let h = 0;
    let node = this.root;
    while (!node.isLeaf) {
      h++;
      node = node.children[0];
    }
    return h;
  }

  /**
   * Validates all B-tree invariants. Returns `true` if the tree is valid.
   *
   * Invariants checked:
   * 1. All leaves are at the same depth.
   * 2. Every non-root node has ≥ t − 1 keys.
   * 3. Every node has ≤ 2t − 1 keys.
   * 4. Keys within each node are strictly sorted.
   * 5. Child ordering respects BST property.
   * 6. `size` equals the number of keys reachable from the root.
   *
   * O(n) — visits every node.
   *
   * @example
   * tree.isValid();   // true (after any sequence of inserts / deletes)
   */
  isValid(): boolean {
    try {
      const h = this.height();
      let count = 0;
      const check = (node: BTreeNode<K, V>, depth: number, minKey: K | null, maxKey: K | null): void => {
        // Invariant 3: node must not be over-full
        if (node.keys.length > 2 * this.t - 1) throw new Error("over-full node");

        // Invariant 2: non-root nodes must have ≥ t-1 keys
        if (node !== this.root && node.keys.length < this.t - 1)
          throw new Error("under-full node");

        // Invariant 4: keys within node are sorted
        for (let i = 1; i < node.keys.length; i++) {
          if (this.compareFn(node.keys[i - 1], node.keys[i]) >= 0)
            throw new Error("unsorted keys in node");
        }

        // Invariant 5: key ordering w.r.t. parent bounds
        for (const k of node.keys) {
          if (minKey !== null && this.compareFn(k, minKey) <= 0)
            throw new Error("key violates min bound");
          if (maxKey !== null && this.compareFn(k, maxKey) >= 0)
            throw new Error("key violates max bound");
        }

        count += node.keys.length;

        if (node.isLeaf) {
          // Invariant 1: all leaves at same depth
          if (depth !== h) throw new Error("leaf at wrong depth");
          if (node.children.length !== 0) throw new Error("leaf has children");
        } else {
          // Internal node: children count = keys count + 1
          if (node.children.length !== node.keys.length + 1)
            throw new Error("wrong number of children");
          for (let i = 0; i < node.children.length; i++) {
            const childMin = i === 0 ? minKey : node.keys[i - 1];
            const childMax = i === node.keys.length ? maxKey : node.keys[i];
            check(node.children[i], depth + 1, childMin, childMax);
          }
        }
      };
      check(this.root, 0, null, null);

      // Invariant 6: size matches actual count
      if (count !== this._size) throw new Error("size mismatch");
      return true;
    } catch {
      return false;
    }
  }

  // -------------------------------------------------------------------------
  // Private helpers — node factories
  // -------------------------------------------------------------------------

  /** Creates an empty leaf node. */
  private makeLeaf(): BTreeNode<K, V> {
    return { keys: [], values: [], children: [], isLeaf: true };
  }

  /** Creates an internal node with the given keys (children added later). */
  private makeInternal(keys: K[]): BTreeNode<K, V> {
    return { keys, values: [], children: [], isLeaf: false };
  }

  /** Returns true if `node` is full (has 2t − 1 keys). */
  private isFull(node: BTreeNode<K, V>): boolean {
    return node.keys.length === 2 * this.t - 1;
  }

  // -------------------------------------------------------------------------
  // Private helpers — search
  // -------------------------------------------------------------------------

  /**
   * Searches for `key` starting at `node`. Returns the value or `undefined`.
   *
   * Uses linear scan within each node. For large `t`, replace with binary
   * search for better constant factors — the asymptotic complexity is the same.
   */
  private searchNode(node: BTreeNode<K, V>, key: K): V | undefined {
    let i = 0;
    // Advance past keys strictly less than the target
    while (i < node.keys.length && this.compareFn(key, node.keys[i]) > 0) i++;

    if (i < node.keys.length && this.compareFn(key, node.keys[i]) === 0) {
      // Found the key in this node
      return node.values[i];
    }
    if (node.isLeaf) {
      // Key not found and no children to descend into
      return undefined;
    }
    // Descend into the appropriate child
    return this.searchNode(node.children[i], key);
  }

  // -------------------------------------------------------------------------
  // Private helpers — insert
  // -------------------------------------------------------------------------

  /**
   * Inserts `key → value` into a node that is guaranteed NOT to be full.
   *
   * Returns `true` if the key already existed (update case), `false` if new.
   *
   * Strategy:
   *   - If leaf: insert at the correct sorted position.
   *   - If internal: find the child to descend into; if that child is full,
   *     split it first; then descend.
   */
  private insertNonFull(node: BTreeNode<K, V>, key: K, value: V): boolean {
    let i = node.keys.length - 1;

    if (node.isLeaf) {
      // Find the insertion point and shift larger keys right
      // Example: keys = [1, 3, 5], inserting 4
      //   i starts at 2 (key=5), shift 5 right, i=1 (key=3), shift 3 right, i=0 (key=1), stop
      //   result: keys = [1, 3, 4, 5]

      // First check if key already exists (scan for exact match)
      for (let j = 0; j <= i; j++) {
        if (this.compareFn(key, node.keys[j]) === 0) {
          node.values[j] = value; // update
          return true;
        }
      }

      node.keys.push(null as unknown as K);
      node.values.push(null as unknown as V);

      while (i >= 0 && this.compareFn(key, node.keys[i]) < 0) {
        node.keys[i + 1] = node.keys[i];
        node.values[i + 1] = node.values[i];
        i--;
      }
      node.keys[i + 1] = key;
      node.values[i + 1] = value;
      return false;
    }

    // Internal node: find child index to descend into
    // Also check if key is already present in this internal node
    while (i >= 0 && this.compareFn(key, node.keys[i]) < 0) i--;

    if (i >= 0 && this.compareFn(key, node.keys[i]) === 0) {
      // Key is in this internal node
      node.values[i] = value;
      return true;
    }

    // Descend into child[i+1]; split it first if full
    i++;
    if (this.isFull(node.children[i])) {
      this.splitChild(node, i);
      // After split, the median key is now at node.keys[i].
      // Decide which of the two new children to descend into.
      if (this.compareFn(key, node.keys[i]) === 0) {
        node.values[i] = value;
        return true;
      }
      if (this.compareFn(key, node.keys[i]) > 0) i++;
    }
    return this.insertNonFull(node.children[i], key, value);
  }

  /**
   * Splits the `i`-th child of `parent`, which must be full (2t − 1 keys).
   *
   * Before split (t = 2, child has 3 keys [10, 20, 30]):
   *
   * ```
   *   parent: [... | 40 | ...]
   *            c[i] = [10 | 20 | 30]
   * ```
   *
   * After split (median key 20 moves up to parent):
   *
   * ```
   *   parent: [... | 20 | 40 | ...]
   *            c[i] = [10]   c[i+1] = [30]
   * ```
   *
   * The left child keeps keys 0..t-2, the right child gets keys t..2t-2.
   * The median key (index t-1) is promoted to the parent.
   *
   * @param parent  The parent node (must not be full).
   * @param i       The index of the child to split.
   */
  private splitChild(parent: BTreeNode<K, V>, i: number): void {
    const t = this.t;
    const child = parent.children[i];

    // The new right sibling takes the top-half keys
    const right: BTreeNode<K, V> = {
      keys: child.keys.splice(t, t - 1),    // keys t..2t-2
      values: child.values.splice(t, t - 1),
      children: child.isLeaf ? [] : child.children.splice(t),
      isLeaf: child.isLeaf,
    };

    // The median key (now the last key in child after splice) moves up
    const medianKey = child.keys.pop()!;
    const medianValue = child.values.pop()!;

    // Insert median into parent at position i
    parent.keys.splice(i, 0, medianKey);
    parent.values.splice(i, 0, medianValue);
    parent.children.splice(i + 1, 0, right);
  }

  // -------------------------------------------------------------------------
  // Private helpers — delete
  // -------------------------------------------------------------------------

  /**
   * Deletes `key` from the subtree rooted at `node`.
   *
   * Returns `true` if the key was found and deleted, `false` otherwise.
   *
   * This implements the CLRS B-tree deletion algorithm (Section 18.3).
   * The `node` passed in must always have ≥ t keys (except when it is the root),
   * so that we can give keys away without violating the B-tree property.
   */
  private deleteKey(node: BTreeNode<K, V>, key: K): boolean {
    let i = 0;
    // Find the index i such that keys[i] >= key
    while (i < node.keys.length && this.compareFn(key, node.keys[i]) > 0) i++;

    const found = i < node.keys.length && this.compareFn(key, node.keys[i]) === 0;

    if (node.isLeaf) {
      // Case 1: key is in a leaf → just remove it
      if (!found) return false;
      node.keys.splice(i, 1);
      node.values.splice(i, 1);
      return true;
    }

    if (found) {
      // Case 2: key is in this internal node
      return this.deleteFromInternal(node, i);
    }

    // Case 3: key is NOT in this node → it lives in child[i]
    // Before descending, make sure child[i] has ≥ t keys
    return this.deleteFromChild(node, i, key);
  }

  /**
   * Case 2 — key is at `node.keys[i]` in an internal node.
   *
   * Sub-cases:
   * - 2a: left child has ≥ t keys → replace with in-order predecessor
   * - 2b: right child has ≥ t keys → replace with in-order successor
   * - 2c: both children have t-1 keys → merge and delete from merged node
   */
  private deleteFromInternal(node: BTreeNode<K, V>, i: number): boolean {
    const leftChild = node.children[i];
    const rightChild = node.children[i + 1];

    if (leftChild.keys.length >= this.t) {
      // Case 2a: predecessor
      // Find the rightmost (largest) key in the left subtree
      const [predKey, predValue] = this.getRightmost(leftChild);
      node.keys[i] = predKey;
      node.values[i] = predValue;
      return this.deleteKey(leftChild, predKey);
    }

    if (rightChild.keys.length >= this.t) {
      // Case 2b: successor
      // Find the leftmost (smallest) key in the right subtree
      const [succKey, succValue] = this.getLeftmost(rightChild);
      node.keys[i] = succKey;
      node.values[i] = succValue;
      return this.deleteKey(rightChild, succKey);
    }

    // Case 2c: merge left + median + right into left child
    // left now has 2t-1 keys; delete from it
    const keyToDelete = node.keys[i];
    this.mergeChildren(node, i);
    return this.deleteKey(node.children[i], keyToDelete);
  }

  /**
   * Case 3 — key lives somewhere in `node.children[i]`.
   *
   * We must ensure `children[i]` has ≥ t keys before descending.
   * - 3a: A sibling has ≥ t keys → rotate
   * - 3b: Both siblings have t-1 keys → merge
   */
  private deleteFromChild(node: BTreeNode<K, V>, i: number, key: K): boolean {
    const child = node.children[i];

    if (child.keys.length >= this.t) {
      // Child already has enough keys — just descend
      return this.deleteKey(child, key);
    }

    // Child has only t-1 keys. Try to borrow from a sibling.
    const leftSibling = i > 0 ? node.children[i - 1] : null;
    const rightSibling = i < node.children.length - 1 ? node.children[i + 1] : null;

    if (leftSibling && leftSibling.keys.length >= this.t) {
      // Case 3a: rotate right (borrow from left sibling)
      //
      // Before:
      //   parent: [... | P | ...]
      //   left:   [... | L]   child: [C | ...]
      //
      // After:
      //   parent: [... | L | ...]
      //   left:   [...]         child: [P | C | ...]
      //
      child.keys.unshift(node.keys[i - 1]);
      child.values.unshift(node.values[i - 1]);
      node.keys[i - 1] = leftSibling.keys.pop()!;
      node.values[i - 1] = leftSibling.values.pop()!;
      if (!leftSibling.isLeaf) {
        child.children.unshift(leftSibling.children.pop()!);
      }
    } else if (rightSibling && rightSibling.keys.length >= this.t) {
      // Case 3a: rotate left (borrow from right sibling)
      child.keys.push(node.keys[i]);
      child.values.push(node.values[i]);
      node.keys[i] = rightSibling.keys.shift()!;
      node.values[i] = rightSibling.values.shift()!;
      if (!rightSibling.isLeaf) {
        child.children.push(rightSibling.children.shift()!);
      }
    } else if (leftSibling) {
      // Case 3b: merge child into left sibling
      this.mergeChildren(node, i - 1);
      // After merge, the child is gone; its content is in node.children[i-1]
      return this.deleteKey(node.children[i - 1], key);
    } else {
      // Case 3b: merge right sibling into child
      this.mergeChildren(node, i);
    }

    return this.deleteKey(node.children[i], key);
  }

  /**
   * Merges `node.children[i+1]` (right) into `node.children[i]` (left).
   *
   * The separator key `node.keys[i]` is pulled down into the merged node.
   *
   * Before (t = 2):
   * ```
   *   node.keys[i] = M
   *   left  = [a]
   *   right = [b]
   * ```
   *
   * After:
   * ```
   *   merged = [a, M, b]
   *   node loses key M and child right
   * ```
   */
  private mergeChildren(node: BTreeNode<K, V>, i: number): void {
    const left = node.children[i];
    const right = node.children[i + 1];

    // Pull the separator key down from parent
    left.keys.push(node.keys[i]);
    left.values.push(node.values[i]);

    // Append all keys/values/children from right
    left.keys.push(...right.keys);
    left.values.push(...right.values);
    left.children.push(...right.children);

    // Remove the separator key and right child from parent
    node.keys.splice(i, 1);
    node.values.splice(i, 1);
    node.children.splice(i + 1, 1);
  }

  /**
   * Returns [key, value] of the rightmost (largest) entry in the subtree
   * rooted at `node`.
   *
   * O(h) — follows rightmost pointers to the leaf.
   */
  private getRightmost(node: BTreeNode<K, V>): [K, V] {
    if (node.isLeaf) {
      const last = node.keys.length - 1;
      return [node.keys[last], node.values[last]];
    }
    return this.getRightmost(node.children[node.children.length - 1]);
  }

  /**
   * Returns [key, value] of the leftmost (smallest) entry in the subtree
   * rooted at `node`.
   *
   * O(h) — follows leftmost pointers to the leaf.
   */
  private getLeftmost(node: BTreeNode<K, V>): [K, V] {
    if (node.isLeaf) return [node.keys[0], node.values[0]];
    return this.getLeftmost(node.children[0]);
  }

  // -------------------------------------------------------------------------
  // Private helpers — traversal
  // -------------------------------------------------------------------------

  /** In-order traversal: accumulates [key, value] pairs in sorted order. */
  private inorderNode(node: BTreeNode<K, V>, result: Array<[K, V]>): void {
    if (node.isLeaf) {
      for (let i = 0; i < node.keys.length; i++) {
        result.push([node.keys[i], node.values[i]]);
      }
      return;
    }
    for (let i = 0; i < node.keys.length; i++) {
      this.inorderNode(node.children[i], result);
      result.push([node.keys[i], node.values[i]]);
    }
    this.inorderNode(node.children[node.children.length - 1], result);
  }

  /**
   * Range traversal: collects [key, value] pairs where `low ≤ key ≤ high`.
   *
   * Prunes subtrees that cannot contain keys in the range:
   * - Skip right subtrees if parent key > high
   * - Skip left subtrees if parent key < low
   */
  private rangeNode(
    node: BTreeNode<K, V>,
    low: K,
    high: K,
    result: Array<[K, V]>
  ): void {
    for (let i = 0; i < node.keys.length; i++) {
      const cmpLow = this.compareFn(node.keys[i], low);
      const cmpHigh = this.compareFn(node.keys[i], high);

      // Descend into left child if it could contain keys ≥ low
      if (!node.isLeaf && cmpLow > 0) {
        this.rangeNode(node.children[i], low, high, result);
      } else if (!node.isLeaf && cmpLow <= 0) {
        // The left subtree of keys[i] may have keys ≥ low
        this.rangeNode(node.children[i], low, high, result);
      }

      // Include this key if it's in range
      if (cmpLow >= 0 && cmpHigh <= 0) {
        result.push([node.keys[i], node.values[i]]);
      }

      // If this key > high, stop — nothing to the right can be in range
      if (cmpHigh > 0) return;
    }

    // Descend into the rightmost child
    if (!node.isLeaf) {
      this.rangeNode(node.children[node.children.length - 1], low, high, result);
    }
  }
}
