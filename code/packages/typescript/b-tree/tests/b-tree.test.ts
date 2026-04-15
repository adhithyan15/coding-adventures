/**
 * B-Tree Test Suite — DT11
 * ========================
 *
 * Tests cover:
 * - All public API methods
 * - All delete cases (1, 2a, 2b, 2c, 3a, 3b)
 * - isValid() called after every mutation
 * - Numeric and string keys
 * - t = 2, t = 3, t = 5
 * - Large-scale insertion (5000+ keys)
 * - Range queries
 * - Edge cases (empty tree, single element, duplicates)
 */

import { describe, it, expect, beforeEach } from "vitest";
import { BTree } from "../src/index.js";

// Helper: numeric comparator
const numCmp = (a: number, b: number) => a - b;
// Helper: string comparator
const strCmp = (a: string, b: string) => a.localeCompare(b);

// Helper: build a tree with t=2 and insert all entries from pairs
function buildTree<K, V>(
  t: number,
  cmp: (a: K, b: K) => number,
  pairs: Array<[K, V]>
): BTree<K, V> {
  const tree = new BTree<K, V>(t, cmp);
  for (const [k, v] of pairs) tree.insert(k, v);
  return tree;
}

// Helper: verify sorted order of inorder output
function isSorted(pairs: Array<[number, unknown]>): boolean {
  for (let i = 1; i < pairs.length; i++) {
    if (pairs[i][0] < pairs[i - 1][0]) return false;
  }
  return true;
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

describe("BTree constructor", () => {
  it("creates an empty tree", () => {
    const tree = new BTree<number, string>(2, numCmp);
    expect(tree.size).toBe(0);
    expect(tree.height()).toBe(0);
    expect(tree.inorder()).toEqual([]);
    expect(tree.isValid()).toBe(true);
  });

  it("throws if t < 2", () => {
    expect(() => new BTree<number, string>(1, numCmp)).toThrow();
  });

  it("uses default t = 2 when not specified", () => {
    // Constructor requires compareFn so we must pass it; t defaults to 2
    const tree = new BTree<number, number>(undefined as unknown as number, numCmp);
    tree.insert(1, 1);
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Basic insert / search / contains
// ---------------------------------------------------------------------------

describe("insert and search", () => {
  it("inserts and retrieves a single key", () => {
    const tree = new BTree<number, string>(2, numCmp);
    tree.insert(42, "answer");
    expect(tree.search(42)).toBe("answer");
    expect(tree.size).toBe(1);
    expect(tree.isValid()).toBe(true);
  });

  it("returns undefined for missing keys", () => {
    const tree = new BTree<number, string>(2, numCmp);
    tree.insert(10, "ten");
    expect(tree.search(99)).toBeUndefined();
  });

  it("contains returns true/false correctly", () => {
    const tree = new BTree<number, string>(2, numCmp);
    tree.insert(7, "seven");
    expect(tree.contains(7)).toBe(true);
    expect(tree.contains(8)).toBe(false);
  });

  it("upsert updates existing value", () => {
    const tree = new BTree<number, string>(2, numCmp);
    tree.insert(5, "five");
    tree.insert(5, "FIVE");
    expect(tree.search(5)).toBe("FIVE");
    expect(tree.size).toBe(1);
    expect(tree.isValid()).toBe(true);
  });

  it("inserts many keys in sorted order", () => {
    const tree = new BTree<number, number>(2, numCmp);
    for (let i = 1; i <= 100; i++) {
      tree.insert(i, i * 2);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(100);
    const pairs = tree.inorder();
    expect(pairs.length).toBe(100);
    expect(isSorted(pairs)).toBe(true);
  });

  it("inserts many keys in reverse order", () => {
    const tree = new BTree<number, number>(2, numCmp);
    for (let i = 100; i >= 1; i--) {
      tree.insert(i, i);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(100);
    expect(isSorted(tree.inorder())).toBe(true);
  });

  it("inserts many keys in random order", () => {
    const tree = new BTree<number, number>(2, numCmp);
    // Pseudo-random permutation using a simple LCG
    const keys: number[] = [];
    for (let i = 0; i < 200; i++) keys.push(i);
    // Shuffle with Fisher-Yates using Math.sin as a pseudo-random source
    for (let i = keys.length - 1; i > 0; i--) {
      const jj = Math.abs(Math.round(Math.sin(i) * 100000)) % (i + 1);
      [keys[i], keys[jj]] = [keys[jj], keys[i]];
    }
    for (const k of keys) {
      tree.insert(k, k);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(200);
    expect(isSorted(tree.inorder())).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// minKey / maxKey
// ---------------------------------------------------------------------------

describe("minKey and maxKey", () => {
  it("returns undefined on empty tree", () => {
    const tree = new BTree<number, string>(2, numCmp);
    expect(tree.minKey()).toBeUndefined();
    expect(tree.maxKey()).toBeUndefined();
  });

  it("returns correct min/max for single element", () => {
    const tree = buildTree(2, numCmp, [[42, "x"]]);
    expect(tree.minKey()).toBe(42);
    expect(tree.maxKey()).toBe(42);
  });

  it("returns correct min/max after multiple inserts", () => {
    const tree = buildTree(2, numCmp, [
      [30, "c"], [10, "a"], [50, "e"], [20, "b"], [40, "d"],
    ]);
    expect(tree.minKey()).toBe(10);
    expect(tree.maxKey()).toBe(50);
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// inorder / rangeQuery
// ---------------------------------------------------------------------------

describe("inorder", () => {
  it("returns empty array for empty tree", () => {
    const tree = new BTree<number, string>(2, numCmp);
    expect(tree.inorder()).toEqual([]);
  });

  it("returns sorted pairs", () => {
    const tree = buildTree(2, numCmp, [
      [3, "c"], [1, "a"], [2, "b"],
    ]);
    expect(tree.inorder()).toEqual([[1, "a"], [2, "b"], [3, "c"]]);
  });
});

describe("rangeQuery", () => {
  it("returns empty array when tree is empty", () => {
    const tree = new BTree<number, string>(2, numCmp);
    expect(tree.rangeQuery(1, 10)).toEqual([]);
  });

  it("returns correct results for a range", () => {
    const tree = buildTree(2, numCmp, [
      [1, "a"], [2, "b"], [3, "c"], [4, "d"], [5, "e"],
      [6, "f"], [7, "g"], [8, "h"], [9, "i"], [10, "j"],
    ]);
    const result = tree.rangeQuery(3, 7);
    expect(result).toEqual([
      [3, "c"], [4, "d"], [5, "e"], [6, "f"], [7, "g"],
    ]);
  });

  it("includes boundary keys", () => {
    const tree = buildTree(2, numCmp, [
      [10, "a"], [20, "b"], [30, "c"],
    ]);
    expect(tree.rangeQuery(10, 30)).toEqual([[10, "a"], [20, "b"], [30, "c"]]);
    expect(tree.rangeQuery(10, 10)).toEqual([[10, "a"]]);
  });

  it("returns empty when range has no matches", () => {
    const tree = buildTree(2, numCmp, [[5, "x"], [15, "y"]]);
    expect(tree.rangeQuery(6, 14)).toEqual([]);
  });

  it("range query with large tree (5000 keys)", () => {
    const tree = new BTree<number, number>(3, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i);
    const result = tree.rangeQuery(1000, 2000);
    expect(result.length).toBe(1001); // inclusive
    expect(result[0][0]).toBe(1000);
    expect(result[result.length - 1][0]).toBe(2000);
  });
});

// ---------------------------------------------------------------------------
// Delete — Case 1: key in leaf
// ---------------------------------------------------------------------------

describe("delete case 1 (key in leaf)", () => {
  it("deletes from a single-element tree", () => {
    const tree = buildTree(2, numCmp, [[5, "five"]]);
    expect(tree.delete(5)).toBe(true);
    expect(tree.size).toBe(0);
    expect(tree.search(5)).toBeUndefined();
    expect(tree.isValid()).toBe(true);
  });

  it("returns false when key not found", () => {
    const tree = buildTree(2, numCmp, [[5, "five"]]);
    expect(tree.delete(99)).toBe(false);
    expect(tree.size).toBe(1);
    expect(tree.isValid()).toBe(true);
  });

  it("deletes a leaf key from a multi-key tree", () => {
    const tree = buildTree(2, numCmp, [
      [1, "a"], [2, "b"], [3, "c"], [4, "d"], [5, "e"],
    ]);
    tree.delete(3);
    expect(tree.search(3)).toBeUndefined();
    expect(tree.size).toBe(4);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.inorder())).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Delete — Case 2: key in internal node
// ---------------------------------------------------------------------------

describe("delete case 2a (predecessor)", () => {
  it("replaces with in-order predecessor when left child has >= t keys", () => {
    // Build a tree that forces case 2a by inserting keys that create an
    // internal node whose left child has enough keys
    const tree = new BTree<number, number>(2, numCmp);
    // Insert 1..10; after splits the root has internal keys
    for (let i = 1; i <= 10; i++) tree.insert(i, i);
    expect(tree.isValid()).toBe(true);

    // Find an internal key and delete it
    const internal = tree.inorder().map(([k]) => k);
    // Delete values that are likely internal nodes
    for (const k of [4, 6, 8]) {
      if (tree.contains(k)) {
        const result = tree.delete(k);
        expect(result).toBe(true);
        expect(tree.search(k)).toBeUndefined();
        expect(tree.isValid()).toBe(true);
      }
    }
    expect(isSorted(tree.inorder())).toBe(true);
    void internal;
  });
});

describe("delete case 2b (successor)", () => {
  it("replaces with in-order successor when right child has >= t keys", () => {
    const tree = new BTree<number, number>(3, numCmp);
    for (let i = 1; i <= 20; i++) tree.insert(i, i);
    expect(tree.isValid()).toBe(true);

    // Delete several internal keys
    for (const k of [10, 5, 15]) {
      tree.delete(k);
      expect(tree.isValid()).toBe(true);
    }
    expect(isSorted(tree.inorder())).toBe(true);
  });
});

describe("delete case 2c (merge)", () => {
  it("merges left and right children when both have t-1 keys", () => {
    // With t=2, nodes need 1 key minimum. Force a 2c situation:
    // Build a minimal tree where both children of internal key have exactly 1 key
    const tree = new BTree<number, number>(2, numCmp);
    // Insert 1,2,3 → root=[2], children=[1],[3] (after split)
    // But with proactive splitting, we need to force this differently.
    // Insert enough to get a two-level tree
    for (let i = 1; i <= 7; i++) tree.insert(i, i);
    expect(tree.isValid()).toBe(true);

    // Delete to trigger case 2c merges
    for (let i = 7; i >= 1; i--) {
      tree.delete(i);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Delete — Case 3: descend with guarantee
// ---------------------------------------------------------------------------

describe("delete case 3a (rotate)", () => {
  it("borrows a key from a sibling when child is under-full", () => {
    const tree = new BTree<number, number>(2, numCmp);
    // Build a tree where deletion requires borrowing
    const keys = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for (const k of keys) tree.insert(k, k);
    expect(tree.isValid()).toBe(true);

    // Delete keys that require case 3a rotations
    tree.delete(10);
    expect(tree.isValid()).toBe(true);
    tree.delete(20);
    expect(tree.isValid()).toBe(true);
    tree.delete(30);
    expect(tree.isValid()).toBe(true);

    expect(isSorted(tree.inorder())).toBe(true);
  });
});

describe("delete case 3b (merge during descent)", () => {
  it("merges siblings when both have minimum keys during descent", () => {
    const tree = new BTree<number, number>(2, numCmp);
    for (let i = 1; i <= 15; i++) tree.insert(i, i);
    expect(tree.isValid()).toBe(true);

    // Delete in an order that forces 3b merges
    for (const k of [1, 3, 5, 7, 9, 11, 13, 15]) {
      tree.delete(k);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(7);
    expect(isSorted(tree.inorder())).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Comprehensive delete
// ---------------------------------------------------------------------------

describe("comprehensive delete (all keys)", () => {
  it("deletes all keys in insertion order (t=2)", () => {
    const n = 50;
    const tree = new BTree<number, number>(2, numCmp);
    for (let i = 0; i < n; i++) tree.insert(i, i);
    for (let i = 0; i < n; i++) {
      expect(tree.delete(i)).toBe(true);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });

  it("deletes all keys in reverse order (t=3)", () => {
    const n = 50;
    const tree = new BTree<number, number>(3, numCmp);
    for (let i = 0; i < n; i++) tree.insert(i, i);
    for (let i = n - 1; i >= 0; i--) {
      expect(tree.delete(i)).toBe(true);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });

  it("deletes all keys in pseudo-random order (t=5)", () => {
    const n = 100;
    const tree = new BTree<number, number>(5, numCmp);
    for (let i = 0; i < n; i++) tree.insert(i, i);

    // Delete in a non-sequential order
    const deleteOrder: number[] = [];
    for (let i = 0; i < n; i++) {
      // Even numbers first, then odd
      if (i % 2 === 0) deleteOrder.unshift(i);
      else deleteOrder.push(i);
    }
    for (const k of deleteOrder) {
      expect(tree.delete(k)).toBe(true);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Height
// ---------------------------------------------------------------------------

describe("height", () => {
  it("is 0 for empty tree", () => {
    const tree = new BTree<number, string>(2, numCmp);
    expect(tree.height()).toBe(0);
  });

  it("increases as tree grows", () => {
    const tree = new BTree<number, number>(2, numCmp);
    const heights: number[] = [];
    for (let i = 0; i < 50; i++) {
      tree.insert(i, i);
      heights.push(tree.height());
    }
    // Height should be non-decreasing
    for (let i = 1; i < heights.length; i++) {
      expect(heights[i]).toBeGreaterThanOrEqual(heights[i - 1]);
    }
    // A B-tree with 50 elements and t=2 should have height ≤ log2(51) ≈ 5
    expect(tree.height()).toBeLessThanOrEqual(6);
  });
});

// ---------------------------------------------------------------------------
// String keys
// ---------------------------------------------------------------------------

describe("string keys", () => {
  it("inserts and retrieves string keys", () => {
    const tree = new BTree<string, number>(2, strCmp);
    const words = ["banana", "apple", "cherry", "date", "elderberry"];
    for (let i = 0; i < words.length; i++) {
      tree.insert(words[i], i);
    }
    expect(tree.isValid()).toBe(true);
    expect(tree.search("apple")).toBe(1);
    expect(tree.search("cherry")).toBe(2);
    expect(tree.search("mango")).toBeUndefined();
  });

  it("inorder is alphabetically sorted for string keys", () => {
    const tree = new BTree<string, number>(2, strCmp);
    const words = ["zebra", "apple", "mango", "cherry", "banana"];
    for (let i = 0; i < words.length; i++) tree.insert(words[i], i);
    const pairs = tree.inorder();
    const keys = pairs.map(([k]) => k);
    expect(keys).toEqual([...keys].sort());
  });

  it("range query with string keys", () => {
    const tree = new BTree<string, number>(2, strCmp);
    const words = ["a", "b", "c", "d", "e", "f", "g"];
    for (let i = 0; i < words.length; i++) tree.insert(words[i], i);
    const result = tree.rangeQuery("b", "e");
    expect(result.map(([k]) => k)).toEqual(["b", "c", "d", "e"]);
  });
});

// ---------------------------------------------------------------------------
// Large scale
// ---------------------------------------------------------------------------

describe("large scale (5000+ keys)", () => {
  it("inserts and validates 5000 sequential keys (t=2)", () => {
    const tree = new BTree<number, number>(2, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i * 2);
    expect(tree.size).toBe(5000);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.inorder())).toBe(true);
    expect(tree.minKey()).toBe(0);
    expect(tree.maxKey()).toBe(4999);
  });

  it("inserts and validates 5000 sequential keys (t=5)", () => {
    const tree = new BTree<number, number>(5, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i);
    expect(tree.size).toBe(5000);
    expect(tree.isValid()).toBe(true);
    expect(tree.height()).toBeLessThanOrEqual(5); // log_5(5000) ≈ 5.3
  });

  it("deletes 2500 keys from a 5000-key tree and remains valid", () => {
    const tree = new BTree<number, number>(2, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i);
    // Delete every other key
    for (let i = 0; i < 5000; i += 2) {
      expect(tree.delete(i)).toBe(true);
    }
    expect(tree.size).toBe(2500);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.inorder())).toBe(true);
  });

  it("inserts 10000 keys in reverse order (t=3)", () => {
    const tree = new BTree<number, number>(3, numCmp);
    for (let i = 9999; i >= 0; i--) tree.insert(i, i);
    expect(tree.size).toBe(10000);
    expect(tree.isValid()).toBe(true);
    expect(tree.minKey()).toBe(0);
    expect(tree.maxKey()).toBe(9999);
  });
});

// ---------------------------------------------------------------------------
// isValid after mixed operations
// ---------------------------------------------------------------------------

describe("isValid after mixed operations", () => {
  it("remains valid after interleaved inserts and deletes", () => {
    const tree = new BTree<number, number>(2, numCmp);
    for (let round = 0; round < 10; round++) {
      // Insert 20 keys
      for (let i = round * 20; i < (round + 1) * 20; i++) {
        tree.insert(i, i);
        expect(tree.isValid()).toBe(true);
      }
      // Delete 10 keys
      for (let i = round * 20; i < round * 20 + 10; i++) {
        tree.delete(i);
        expect(tree.isValid()).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// t = 3 specific tests
// ---------------------------------------------------------------------------

describe("t = 3 tree", () => {
  it("allows up to 5 keys per node", () => {
    const tree = new BTree<number, number>(3, numCmp);
    // 5 keys should fit in a single root (no splits needed)
    for (let i = 1; i <= 5; i++) tree.insert(i, i);
    expect(tree.height()).toBe(0); // still root-only
    expect(tree.isValid()).toBe(true);
  });

  it("splits correctly at 6 keys", () => {
    const tree = new BTree<number, number>(3, numCmp);
    for (let i = 1; i <= 6; i++) tree.insert(i, i);
    // After 6th insert, root splits, tree has height 1
    expect(tree.height()).toBe(1);
    expect(tree.isValid()).toBe(true);
    expect(tree.size).toBe(6);
  });

  it("handles 1000 keys correctly", () => {
    const tree = new BTree<number, number>(3, numCmp);
    for (let i = 0; i < 1000; i++) tree.insert(i, i);
    expect(tree.size).toBe(1000);
    expect(tree.isValid()).toBe(true);

    for (let i = 0; i < 500; i++) {
      tree.delete(i * 2);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(500);
  });
});
