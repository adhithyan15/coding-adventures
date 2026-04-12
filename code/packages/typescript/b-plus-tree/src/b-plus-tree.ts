/**
 * B+ Tree — DT12
 * ==============
 *
 * A B+ tree is the most common variant of B-trees used in databases. It
 * extends the B-tree with one crucial change: **all data (key-value pairs)
 * lives exclusively in the leaf nodes**, and the leaf nodes are connected in
 * a singly-linked list.
 *
 * ## B-tree vs B+ tree
 *
 * ```
 * B-tree:    every node can hold data
 *            [10|"a"] — [20|"b"] — [30|"c"]   (internal nodes hold values too)
 *
 * B+ tree:   internal nodes hold ONLY keys (routing / separator keys)
 *            internal: [20] — [40]
 *            leaves:   [10|"a"] ↔ [20|"b"] ↔ [30|"c"] ↔ [40|"d"]
 * ```
 *
 * ## Why B+ trees are preferred for databases
 *
 * 1. **Full table scans**: The leaf linked list enables O(n) sequential scans
 *    without any tree traversal. You find the first leaf and follow `next`
 *    pointers. Perfect for `SELECT * FROM table ORDER BY key`.
 *
 * 2. **Range scans**: `SELECT * WHERE key BETWEEN 100 AND 200` descends to the
 *    leaf for key 100, then walks the linked list until key > 200. Extremely
 *    cache-friendly — leaves are allocated sequentially in most implementations.
 *
 * 3. **Higher branching factor**: Since internal nodes hold only keys (no values),
 *    they are smaller and fit more entries per disk page → even shallower tree.
 *
 * ## Invariants
 *
 * Let `t` be the minimum degree.
 *
 * ### Internal nodes (`BPlusInternalNode`)
 * - Has `keys.length` separator keys and `children.length = keys.length + 1` children.
 * - Every non-root internal node has `t - 1 ≤ keys.length ≤ 2t - 1`.
 * - `keys[i]` is the separator between `children[i]` and `children[i+1]`:
 *   - All keys in `children[i]` are **strictly less than** `keys[i]`.
 *   - All keys in `children[i+1]` are **≥ `keys[i]`**.
 *
 * ### Leaf nodes (`BPlusLeafNode`)
 * - Has `keys.length = values.length` key-value pairs.
 * - Every non-root leaf has `t - 1 ≤ keys.length ≤ 2t - 1`.
 * - Keys are sorted: `keys[0] < keys[1] < ... < keys[n-1]`.
 * - `next` points to the next leaf in sorted order, or `null` if last.
 * - All leaves are at the same depth.
 *
 * ## Key insight: separator COPIED (not moved) on leaf split
 *
 * When a leaf splits, the separator key that goes up to the parent is
 * **COPIED** — it stays in the right leaf too. This is different from the
 * B-tree where the median key is MOVED up (removed from the child).
 *
 * ```
 * Leaf split (t=2, leaf has 3 keys = 2t-1):
 *
 *   Before: leaf = [10, 20, 30]
 *
 *   After:
 *     left leaf  = [10]
 *     right leaf = [20, 30]    ← 20 stays here
 *     separator 20 pushed up to parent
 * ```
 *
 * @module b-plus-tree
 */

// ---------------------------------------------------------------------------
// Node types
// ---------------------------------------------------------------------------

/**
 * An internal (non-leaf) node in the B+ tree.
 *
 * Internal nodes act as an index/routing layer. They store only separator keys,
 * not the actual data values. All actual data lives in the leaves.
 *
 * ```
 *   keys:     [20, 40]
 *   children: [leaf0, leaf1, leaf2]
 *
 *   leaf0 contains keys < 20
 *   leaf1 contains keys in [20, 40)
 *   leaf2 contains keys ≥ 40
 * ```
 */
export interface BPlusInternalNode<K, V> {
  /** Separator keys. Always `children.length - 1` of them. */
  keys: K[];
  /** Child pointers. Can be internal or leaf nodes. */
  children: Array<BPlusInternalNode<K, V> | BPlusLeafNode<K, V>>;
  readonly isLeaf: false;
}

/**
 * A leaf node in the B+ tree.
 *
 * Leaves are where all the actual data lives. They form a singly-linked list
 * in sorted key order via the `next` pointer.
 *
 * ```
 *   keys:   [10, 15, 18]
 *   values: ["a", "b", "c"]
 *   next → (next leaf node)
 * ```
 */
export interface BPlusLeafNode<K, V> {
  /** Sorted keys. Always same length as `values`. */
  keys: K[];
  /** Data values parallel to `keys`. */
  values: V[];
  /** Pointer to the next leaf in ascending key order. `null` if last leaf. */
  next: BPlusLeafNode<K, V> | null;
  readonly isLeaf: true;
}

/** Union type for any B+ tree node. */
type BPlusNode<K, V> = BPlusInternalNode<K, V> | BPlusLeafNode<K, V>;

// ---------------------------------------------------------------------------
// BPlusTree class
// ---------------------------------------------------------------------------

/**
 * A generic B+ tree keyed on `K` with associated values `V`.
 *
 * ### Quick start
 *
 * ```typescript
 * const tree = new BPlusTree<number, string>(2, (a, b) => a - b);
 * tree.insert(10, "ten");
 * tree.insert(20, "twenty");
 * tree.insert(5,  "five");
 *
 * console.log(tree.search(10));               // "ten"
 * console.log(tree.fullScan());               // [[5,"five"], [10,"ten"], [20,"twenty"]]
 * console.log(tree.rangeScan(5, 15));         // [[5,"five"], [10,"ten"]]
 *
 * for (const [k, v] of tree) {
 *   console.log(k, v);   // sorted iteration via Symbol.iterator
 * }
 * ```
 *
 * @typeParam K - Key type. Must be totally ordered via `compareFn`.
 * @typeParam V - Value type. Can be anything.
 */
export class BPlusTree<K, V> {
  /** The root — may be a leaf (empty or small tree) or internal node. */
  private root: BPlusNode<K, V>;

  /**
   * Pointer to the first (leftmost) leaf.
   * Gives O(1) start of a full scan or ascending iteration.
   */
  private firstLeaf: BPlusLeafNode<K, V>;

  /** Maintained count of key-value pairs. */
  private _size = 0;

  /**
   * Creates a new B+ tree.
   *
   * @param t          Minimum degree (default 2). Must be ≥ 2.
   * @param compareFn  Comparator: returns negative / 0 / positive.
   */
  constructor(
    private readonly t: number = 2,
    private readonly compareFn: (a: K, b: K) => number
  ) {
    if (t < 2) throw new RangeError("BPlusTree minimum degree t must be ≥ 2");
    const leaf = this.makeLeaf();
    this.root = leaf;
    this.firstLeaf = leaf;
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /** Number of key-value pairs stored. O(1). */
  get size(): number {
    return this._size;
  }

  /**
   * Returns the value associated with `key`, or `undefined` if not found.
   *
   * Descends through internal nodes to the correct leaf, then does a linear
   * scan. O(t · log_t n).
   */
  search(key: K): V | undefined {
    const leaf = this.findLeaf(key);
    const i = this.leafIndexOf(leaf, key);
    if (i >= 0) return leaf.values[i];
    return undefined;
  }

  /**
   * Returns `true` if `key` exists in the tree.
   */
  contains(key: K): boolean {
    return this.search(key) !== undefined;
  }

  /**
   * Inserts `key → value` into the tree.
   *
   * If `key` already exists, its value is updated (upsert semantics).
   *
   * Uses proactive top-down splitting: before descending into a full node,
   * split it. On leaf split, the separator is COPIED up (not moved).
   *
   * O(t · log_t n).
   */
  insert(key: K, value: V): void {
    // If root is full, split it → tree grows by one level
    if (this.nodeIsFull(this.root)) {
      const oldRoot = this.root;
      const newRoot = this.makeInternal();
      newRoot.children.push(oldRoot);
      this.splitChild(newRoot, 0);
      this.root = newRoot;
    }
    const wasNew = this.insertIntoNode(this.root, key, value);
    if (wasNew) this._size++;
  }

  /**
   * Removes `key` from the tree. Returns `true` if found, `false` otherwise.
   *
   * B+ tree deletion:
   * - Only leaf nodes hold data, so we always delete from a leaf.
   * - We ensure each node we visit has > t-1 keys before descending.
   *   If not, we borrow from a sibling or merge.
   *
   * O(t · log_t n).
   */
  delete(key: K): boolean {
    const found = this.deleteFromNode(this.root, key, null, -1);
    if (found) {
      this._size--;
      // Shrink tree if root has no keys but has a child
      if (!this.root.isLeaf && this.root.keys.length === 0) {
        this.root = (this.root as BPlusInternalNode<K, V>).children[0];
      }
      // Update firstLeaf in case we deleted from the very first leaf
      this.firstLeaf = this.getFirstLeaf(this.root);
    }
    return found;
  }

  /**
   * Returns the smallest key in the tree, or `undefined` if empty.
   *
   * O(1) — `firstLeaf` is maintained directly.
   */
  minKey(): K | undefined {
    if (this._size === 0) return undefined;
    return this.firstLeaf.keys[0];
  }

  /**
   * Returns the largest key in the tree, or `undefined` if empty.
   *
   * O(h) — follows rightmost path.
   */
  maxKey(): K | undefined {
    if (this._size === 0) return undefined;
    let node: BPlusNode<K, V> = this.root;
    while (!node.isLeaf) {
      const internal = node as BPlusInternalNode<K, V>;
      node = internal.children[internal.children.length - 1];
    }
    const leaf = node as BPlusLeafNode<K, V>;
    return leaf.keys[leaf.keys.length - 1];
  }

  /**
   * Returns all `[key, value]` pairs with `low ≤ key ≤ high`, in sorted order.
   *
   * Strategy:
   * 1. Find the leaf for `low` via `findLeaf`.
   * 2. Scan forward through leaves via the `next` pointer until `key > high`.
   *
   * O(log_t n + m) where m is the number of matching pairs — extremely
   * efficient for database-style range scans.
   *
   * @example
   * tree.rangeScan(100, 200);   // all entries between 100 and 200 inclusive
   */
  rangeScan(low: K, high: K): Array<[K, V]> {
    const result: Array<[K, V]> = [];
    let leaf: BPlusLeafNode<K, V> | null = this.findLeaf(low);

    while (leaf !== null) {
      for (let i = 0; i < leaf.keys.length; i++) {
        const cmpLow = this.compareFn(leaf.keys[i], low);
        const cmpHigh = this.compareFn(leaf.keys[i], high);
        if (cmpLow >= 0 && cmpHigh <= 0) {
          result.push([leaf.keys[i], leaf.values[i]]);
        }
        if (cmpHigh > 0) return result; // past the end of range
      }
      leaf = leaf.next;
    }
    return result;
  }

  /**
   * Returns all `[key, value]` pairs in ascending key order.
   *
   * Starts at `firstLeaf` and follows the `next` chain. O(n).
   *
   * This is equivalent to a full table scan in a database — no tree traversal
   * needed after the initial O(1) pointer dereference.
   *
   * @example
   * tree.fullScan();   // [[k1, v1], [k2, v2], ...]
   */
  fullScan(): Array<[K, V]> {
    const result: Array<[K, V]> = [];
    let leaf: BPlusLeafNode<K, V> | null = this.firstLeaf;
    while (leaf !== null) {
      for (let i = 0; i < leaf.keys.length; i++) {
        result.push([leaf.keys[i], leaf.values[i]]);
      }
      leaf = leaf.next;
    }
    return result;
  }

  /**
   * Returns the height of the tree.
   *
   * 0 = root is a leaf (empty or very small tree).
   * 1 = one level of internal nodes above the leaves.
   *
   * O(h) — follows the leftmost path.
   */
  height(): number {
    let h = 0;
    let node: BPlusNode<K, V> = this.root;
    while (!node.isLeaf) {
      h++;
      node = (node as BPlusInternalNode<K, V>).children[0];
    }
    return h;
  }

  /**
   * Validates all B+ tree invariants. Returns `true` if the tree is valid.
   *
   * Invariants checked:
   * 1. All leaves are at the same depth.
   * 2. Every non-root node has ≥ t − 1 keys.
   * 3. Every node has ≤ 2t − 1 keys.
   * 4. Keys within each node are strictly sorted.
   * 5. Internal node children count = keys count + 1.
   * 6. Leaf linked list is intact (forward walk matches in-order count).
   * 7. `size` equals the number of key-value pairs in the leaves.
   * 8. `firstLeaf` is the leftmost leaf.
   *
   * O(n).
   */
  isValid(): boolean {
    try {
      const h = this.height();
      let leafCount = 0;

      const check = (node: BPlusNode<K, V>, depth: number, minKey: K | null, maxKey: K | null): void => {
        if (node.keys.length > 2 * this.t - 1) throw new Error("over-full node");
        if (node !== this.root && node.keys.length < this.t - 1)
          throw new Error("under-full node");

        // Keys must be sorted
        for (let i = 1; i < node.keys.length; i++) {
          if (this.compareFn(node.keys[i - 1], node.keys[i]) >= 0)
            throw new Error("unsorted keys");
        }

        if (node.isLeaf) {
          const leaf = node as BPlusLeafNode<K, V>;
          if (depth !== h) throw new Error("leaf at wrong depth");
          if (leaf.keys.length !== leaf.values.length)
            throw new Error("key/value length mismatch in leaf");

          // Check bounds
          for (const k of leaf.keys) {
            if (minKey !== null && this.compareFn(k, minKey) < 0)
              throw new Error("leaf key below min bound");
            if (maxKey !== null && this.compareFn(k, maxKey) >= 0)
              throw new Error("leaf key above max bound");
          }

          leafCount += leaf.keys.length;
        } else {
          const internal = node as BPlusInternalNode<K, V>;
          if (internal.children.length !== internal.keys.length + 1)
            throw new Error("wrong children count");

          for (let i = 0; i < internal.children.length; i++) {
            const childMin = i === 0 ? minKey : internal.keys[i - 1];
            const childMax = i === internal.keys.length ? maxKey : internal.keys[i];
            check(internal.children[i], depth + 1, childMin, childMax);
          }
        }
      };

      check(this.root, 0, null, null);

      // Validate linked list integrity
      let linkedCount = 0;
      let leaf: BPlusLeafNode<K, V> | null = this.firstLeaf;
      let prevLastKey: K | null = null;
      while (leaf !== null) {
        linkedCount += leaf.keys.length;
        if (leaf.keys.length > 0 && prevLastKey !== null) {
          if (this.compareFn(prevLastKey, leaf.keys[0]) >= 0)
            throw new Error("linked list out of order");
        }
        if (leaf.keys.length > 0) {
          prevLastKey = leaf.keys[leaf.keys.length - 1];
        }
        leaf = leaf.next;
      }
      if (linkedCount !== leafCount) throw new Error("linked list count mismatch");
      if (leafCount !== this._size) throw new Error("size mismatch");

      // Validate firstLeaf
      if (this.firstLeaf !== this.getFirstLeaf(this.root))
        throw new Error("firstLeaf is wrong");

      return true;
    } catch {
      return false;
    }
  }

  /**
   * Implements `Symbol.iterator` so the tree can be used in `for...of` loops.
   *
   * Iterates in ascending key order via the leaf linked list. O(n) total.
   *
   * @example
   * for (const [key, value] of tree) {
   *   console.log(key, value);
   * }
   *
   * // Or spread into an array:
   * const all = [...tree];
   */
  [Symbol.iterator](): Iterator<[K, V]> {
    let leaf: BPlusLeafNode<K, V> | null = this.firstLeaf;
    let i = 0;
    return {
      next(): IteratorResult<[K, V]> {
        while (leaf !== null) {
          if (i < leaf.keys.length) {
            return { value: [leaf.keys[i], leaf.values[i++]], done: false };
          }
          leaf = leaf.next;
          i = 0;
        }
        return { value: undefined as unknown as [K, V], done: true };
      },
    };
  }

  // -------------------------------------------------------------------------
  // Private helpers — node factories
  // -------------------------------------------------------------------------

  /** Creates a new empty leaf node. */
  private makeLeaf(): BPlusLeafNode<K, V> {
    return { keys: [], values: [], next: null, isLeaf: true };
  }

  /** Creates a new empty internal node. */
  private makeInternal(): BPlusInternalNode<K, V> {
    return { keys: [], children: [], isLeaf: false };
  }

  /** Returns true if node has 2t-1 keys (full). */
  private nodeIsFull(node: BPlusNode<K, V>): boolean {
    return node.keys.length === 2 * this.t - 1;
  }

  // -------------------------------------------------------------------------
  // Private helpers — search
  // -------------------------------------------------------------------------

  /**
   * Descends the tree to find the leaf that would contain `key`.
   *
   * At each internal node, we use the separator keys to choose the correct
   * child:
   * - If `key < keys[i]`, descend into `children[i]`.
   * - Otherwise continue scanning right.
   */
  private findLeaf(key: K): BPlusLeafNode<K, V> {
    let node: BPlusNode<K, V> = this.root;
    while (!node.isLeaf) {
      const internal = node as BPlusInternalNode<K, V>;
      let i = 0;
      while (i < internal.keys.length && this.compareFn(key, internal.keys[i]) >= 0) {
        i++;
      }
      node = internal.children[i];
    }
    return node as BPlusLeafNode<K, V>;
  }

  /**
   * Returns the index of `key` in `leaf.keys`, or -1 if not found.
   */
  private leafIndexOf(leaf: BPlusLeafNode<K, V>, key: K): number {
    for (let i = 0; i < leaf.keys.length; i++) {
      if (this.compareFn(key, leaf.keys[i]) === 0) return i;
    }
    return -1;
  }

  /**
   * Follows leftmost child pointers to find the first leaf.
   */
  private getFirstLeaf(node: BPlusNode<K, V>): BPlusLeafNode<K, V> {
    while (!node.isLeaf) {
      node = (node as BPlusInternalNode<K, V>).children[0];
    }
    return node as BPlusLeafNode<K, V>;
  }

  // -------------------------------------------------------------------------
  // Private helpers — insert
  // -------------------------------------------------------------------------

  /**
   * Inserts `key → value` into `node`, which is guaranteed not to be full.
   *
   * Returns `true` if this was a new key, `false` if it was an update.
   */
  private insertIntoNode(node: BPlusNode<K, V>, key: K, value: V): boolean {
    if (node.isLeaf) {
      return this.insertIntoLeaf(node as BPlusLeafNode<K, V>, key, value);
    }

    const internal = node as BPlusInternalNode<K, V>;
    let i = internal.keys.length;
    while (i > 0 && this.compareFn(key, internal.keys[i - 1]) < 0) i--;

    // At internal nodes, separator keys don't hold data. The only way key
    // equals a separator is if that exact key was previously promoted as a
    // separator. In B+ trees, we don't update separators — the actual value
    // is in the leaf. We still need to descend to the right child.
    //
    // Descend into children[i]; if that child is full, split it first.
    let childIdx = i;
    if (this.compareFn(key, internal.keys[i > 0 ? i - 1 : 0]) >= 0 && i > 0) {
      childIdx = i;
    } else {
      childIdx = i;
    }

    if (this.nodeIsFull(internal.children[childIdx])) {
      this.splitChild(internal, childIdx);
      // After split, the new separator is at internal.keys[childIdx].
      // Choose the correct child to descend into.
      if (this.compareFn(key, internal.keys[childIdx]) >= 0) {
        childIdx++;
      }
    }

    return this.insertIntoNode(internal.children[childIdx], key, value);
  }

  /**
   * Inserts into a leaf node. Returns true if key is new, false if update.
   */
  private insertIntoLeaf(leaf: BPlusLeafNode<K, V>, key: K, value: V): boolean {
    // Find insertion position (maintaining sorted order)
    let i = 0;
    while (i < leaf.keys.length && this.compareFn(key, leaf.keys[i]) > 0) i++;

    if (i < leaf.keys.length && this.compareFn(key, leaf.keys[i]) === 0) {
      // Key exists — update value
      leaf.values[i] = value;
      return false;
    }

    // Insert at position i
    leaf.keys.splice(i, 0, key);
    leaf.values.splice(i, 0, value);
    return true;
  }

  /**
   * Splits the full child at `childIndex` of `parent`.
   *
   * ### B+ tree split rules (differ from B-tree)
   *
   * **If the child is a LEAF:**
   * ```
   *   Before: leaf = [a, b, c, d]  (2t-1 = 3 for t=2 → but let's use t=2 for illustration)
   *   Split at index t-1 = 1 (for t=2, each half gets t-1=1 keys minimum)
   *
   *   Left leaf  = [a, b]
   *   Right leaf = [c, d]          ← separator c STAYS in right leaf
   *   Separator c is COPIED to parent
   * ```
   *
   * **If the child is an INTERNAL node:**
   * ```
   *   Before: internal = [k0, k1, k2]  (2t-1 = 3)
   *   Median key = k1 (index t-1 = 1)
   *
   *   Left internal  = [k0]
   *   Right internal = [k2]    ← k1 is MOVED (not copied) to parent
   * ```
   *
   * This is the critical difference: leaf splits COPY the separator; internal
   * splits MOVE it.
   */
  private splitChild(parent: BPlusInternalNode<K, V>, childIndex: number): void {
    const child = parent.children[childIndex];
    const t = this.t;

    if (child.isLeaf) {
      // Leaf split: separator is COPIED (stays in right leaf)
      const leftLeaf = child as BPlusLeafNode<K, V>;
      const rightLeaf = this.makeLeaf();

      // Split point: right leaf gets keys from index t-1 onward
      // (so left gets t-1 keys, right gets t keys for 2t-1 total)
      rightLeaf.keys = leftLeaf.keys.splice(t - 1);
      rightLeaf.values = leftLeaf.values.splice(t - 1);

      // Maintain linked list: right.next = left.next, left.next = right
      rightLeaf.next = leftLeaf.next;
      leftLeaf.next = rightLeaf;

      // The separator key is the FIRST key of the right leaf (COPIED, not removed)
      const separator = rightLeaf.keys[0];

      // Insert separator and new child into parent
      parent.keys.splice(childIndex, 0, separator);
      parent.children.splice(childIndex + 1, 0, rightLeaf);
    } else {
      // Internal node split: median key is MOVED (not copied)
      const leftInternal = child as BPlusInternalNode<K, V>;
      const rightInternal = this.makeInternal();

      // Median index
      const medianIdx = t - 1;
      const separator = leftInternal.keys[medianIdx];

      // Right internal gets keys after median
      rightInternal.keys = leftInternal.keys.splice(medianIdx + 1);
      // Remove the median key itself from left
      leftInternal.keys.splice(medianIdx, 1);

      // Right internal gets the right half of children
      rightInternal.children = leftInternal.children.splice(t);

      // Insert separator and new child into parent
      parent.keys.splice(childIndex, 0, separator);
      parent.children.splice(childIndex + 1, 0, rightInternal);
    }
  }

  // -------------------------------------------------------------------------
  // Private helpers — delete
  // -------------------------------------------------------------------------

  /**
   * Deletes `key` from the subtree rooted at `node`.
   *
   * `parent` and `parentChildIndex` track how we got to this node, so we
   * can update the parent's separator keys when needed.
   *
   * Returns `true` if the key was found and deleted.
   *
   * B+ tree delete strategy:
   * - All deletions happen in leaves.
   * - Before descending into a child, ensure it has > t-1 keys (borrow or merge).
   * - After deleting from a leaf, if the deleted key was a separator in an
   *   ancestor, update that separator with the new first key of the leaf.
   */
  private deleteFromNode(
    node: BPlusNode<K, V>,
    key: K,
    parent: BPlusInternalNode<K, V> | null,
    parentChildIndex: number
  ): boolean {
    if (node.isLeaf) {
      const leaf = node as BPlusLeafNode<K, V>;
      const i = this.leafIndexOf(leaf, key);
      if (i < 0) return false;

      leaf.keys.splice(i, 1);
      leaf.values.splice(i, 1);

      // Update parent separator if we deleted the first key of this leaf
      if (i === 0 && parent !== null && parentChildIndex > 0 && leaf.keys.length > 0) {
        // The separator at parent.keys[parentChildIndex - 1] used to be `key`.
        // It should now be the new first key of this leaf.
        parent.keys[parentChildIndex - 1] = leaf.keys[0];
      }
      return true;
    }

    // Internal node: find the child to descend into
    const internal = node as BPlusInternalNode<K, V>;
    let i = 0;
    while (i < internal.keys.length && this.compareFn(key, internal.keys[i]) >= 0) {
      i++;
    }

    // Ensure child[i] has > t-1 keys before descending
    const child = internal.children[i];
    if (child.keys.length <= this.t - 1) {
      this.fixChild(internal, i);
      // After fixing, the tree structure may have changed (merge reduces child count).
      // Recompute index.
      // Simple approach: re-run the whole delete from this node.
      return this.deleteFromNode(node, key, parent, parentChildIndex);
    }

    return this.deleteFromNode(child, key, internal, i);
  }

  /**
   * Ensures that `parent.children[i]` has > t-1 keys by borrowing from a
   * sibling or merging with a sibling.
   *
   * This is the B+ tree analogue of CLRS Case 3.
   *
   * ### Borrow from left sibling (rotate right)
   *
   * ```
   * Before:
   *   parent.keys[i-1] = P
   *   left = [..., L]   child = [...]
   *
   * After:
   *   parent.keys[i-1] = L
   *   left = [...]      child = [L, ...]
   * ```
   *
   * ### Merge with left sibling
   *
   * ```
   * Before:
   *   parent.keys[i-1] = P
   *   left = [a, b]   child = [c]
   *
   * After:
   *   merged = [a, b, c]   (parent.keys[i-1] and child removed from parent)
   * ```
   */
  private fixChild(parent: BPlusInternalNode<K, V>, i: number): void {
    const child = parent.children[i];
    const leftSibling = i > 0 ? parent.children[i - 1] : null;
    const rightSibling = i < parent.children.length - 1 ? parent.children[i + 1] : null;

    if (leftSibling && leftSibling.keys.length > this.t - 1) {
      // Borrow from left sibling
      if (child.isLeaf) {
        const leftLeaf = leftSibling as BPlusLeafNode<K, V>;
        const childLeaf = child as BPlusLeafNode<K, V>;
        // Move last entry of left to front of child
        childLeaf.keys.unshift(leftLeaf.keys.pop()!);
        childLeaf.values.unshift(leftLeaf.values.pop()!);
        // Update separator: it's now the first key of the child
        parent.keys[i - 1] = childLeaf.keys[0];
      } else {
        const leftInternal = leftSibling as BPlusInternalNode<K, V>;
        const childInternal = child as BPlusInternalNode<K, V>;
        // For internal nodes: rotate via parent separator
        childInternal.keys.unshift(parent.keys[i - 1]);
        parent.keys[i - 1] = leftInternal.keys.pop()!;
        childInternal.children.unshift(leftInternal.children.pop()!);
      }
    } else if (rightSibling && rightSibling.keys.length > this.t - 1) {
      // Borrow from right sibling
      if (child.isLeaf) {
        const rightLeaf = rightSibling as BPlusLeafNode<K, V>;
        const childLeaf = child as BPlusLeafNode<K, V>;
        // Move first entry of right to end of child
        childLeaf.keys.push(rightLeaf.keys.shift()!);
        childLeaf.values.push(rightLeaf.values.shift()!);
        // Update separator
        parent.keys[i] = rightLeaf.keys[0];
      } else {
        const rightInternal = rightSibling as BPlusInternalNode<K, V>;
        const childInternal = child as BPlusInternalNode<K, V>;
        childInternal.keys.push(parent.keys[i]);
        parent.keys[i] = rightInternal.keys.shift()!;
        childInternal.children.push(rightInternal.children.shift()!);
      }
    } else {
      // Merge
      if (leftSibling) {
        // Merge child into left sibling
        this.mergeNodes(parent, i - 1);
      } else {
        // Merge right sibling into child
        this.mergeNodes(parent, i);
      }
    }
  }

  /**
   * Merges `parent.children[i+1]` into `parent.children[i]`.
   *
   * For leaves: concatenate keys/values, update linked list.
   * For internal nodes: pull down separator from parent, then concatenate.
   *
   * After merge, removes the separator and right child from parent.
   */
  private mergeNodes(parent: BPlusInternalNode<K, V>, i: number): void {
    const left = parent.children[i];
    const right = parent.children[i + 1];

    if (left.isLeaf) {
      const leftLeaf = left as BPlusLeafNode<K, V>;
      const rightLeaf = right as BPlusLeafNode<K, V>;
      leftLeaf.keys.push(...rightLeaf.keys);
      leftLeaf.values.push(...rightLeaf.values);
      leftLeaf.next = rightLeaf.next;
    } else {
      const leftInternal = left as BPlusInternalNode<K, V>;
      const rightInternal = right as BPlusInternalNode<K, V>;
      // Pull down separator from parent
      leftInternal.keys.push(parent.keys[i]);
      leftInternal.keys.push(...rightInternal.keys);
      leftInternal.children.push(...rightInternal.children);
    }

    // Remove separator and right child from parent
    parent.keys.splice(i, 1);
    parent.children.splice(i + 1, 1);
  }
}
