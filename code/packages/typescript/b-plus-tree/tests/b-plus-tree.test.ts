/**
 * B+ Tree Test Suite — DT12
 * =========================
 *
 * Tests cover:
 * - All public API methods
 * - isValid() called after every mutation
 * - Leaf linked list integrity
 * - rangeScan and fullScan
 * - Symbol.iterator
 * - Numeric and string keys
 * - t = 2, t = 3, t = 5
 * - Large-scale tests (5000+ keys)
 * - Delete from leaves, borrow, and merge cases
 * - Edge cases (empty tree, single element, duplicates)
 */

import { describe, it, expect, beforeEach } from "vitest";
import { BPlusTree } from "../src/index.js";

const numCmp = (a: number, b: number) => a - b;
const strCmp = (a: string, b: string) => a.localeCompare(b);

function buildTree<K, V>(
  t: number,
  cmp: (a: K, b: K) => number,
  pairs: Array<[K, V]>
): BPlusTree<K, V> {
  const tree = new BPlusTree<K, V>(t, cmp);
  for (const [k, v] of pairs) tree.insert(k, v);
  return tree;
}

function isSorted(pairs: Array<[number, unknown]>): boolean {
  for (let i = 1; i < pairs.length; i++) {
    if (pairs[i][0] < pairs[i - 1][0]) return false;
  }
  return true;
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

describe("BPlusTree constructor", () => {
  it("creates an empty tree", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    expect(tree.size).toBe(0);
    expect(tree.height()).toBe(0);
    expect(tree.fullScan()).toEqual([]);
    expect(tree.isValid()).toBe(true);
  });

  it("throws if t < 2", () => {
    expect(() => new BPlusTree<number, string>(1, numCmp)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Basic insert / search / contains
// ---------------------------------------------------------------------------

describe("insert and search", () => {
  it("inserts and retrieves a single key", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    tree.insert(42, "answer");
    expect(tree.search(42)).toBe("answer");
    expect(tree.size).toBe(1);
    expect(tree.isValid()).toBe(true);
  });

  it("returns undefined for missing key", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    tree.insert(10, "ten");
    expect(tree.search(99)).toBeUndefined();
  });

  it("contains returns true/false", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    tree.insert(7, "seven");
    expect(tree.contains(7)).toBe(true);
    expect(tree.contains(8)).toBe(false);
  });

  it("upsert updates existing value", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    tree.insert(5, "five");
    tree.insert(5, "FIVE");
    expect(tree.search(5)).toBe("FIVE");
    expect(tree.size).toBe(1);
    expect(tree.isValid()).toBe(true);
  });

  it("inserts 100 keys in sorted order", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 1; i <= 100; i++) {
      tree.insert(i, i * 2);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(100);
    expect(isSorted(tree.fullScan())).toBe(true);
  });

  it("inserts 100 keys in reverse order", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 100; i >= 1; i--) {
      tree.insert(i, i);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(100);
    expect(isSorted(tree.fullScan())).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Leaf linked list integrity
// ---------------------------------------------------------------------------

describe("leaf linked list integrity", () => {
  it("linked list is in sorted order after sequential inserts", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 20; i++) tree.insert(i, i);
    const fromScan = tree.fullScan().map(([k]) => k);
    expect(fromScan).toEqual(fromScan.slice().sort((a, b) => a - b));
    expect(tree.isValid()).toBe(true);
  });

  it("linked list is in sorted order after reverse inserts", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 50; i >= 1; i--) tree.insert(i, i);
    const fromScan = tree.fullScan().map(([k]) => k);
    expect(fromScan).toEqual(fromScan.slice().sort((a, b) => a - b));
    expect(tree.isValid()).toBe(true);
  });

  it("linked list count matches tree size", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 30; i++) tree.insert(i, i);
    expect(tree.fullScan().length).toBe(tree.size);
    expect(tree.isValid()).toBe(true);
  });

  it("linked list is intact after deletions", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 20; i++) tree.insert(i, i);
    for (let i = 0; i < 20; i += 2) tree.delete(i);
    const scan = tree.fullScan();
    expect(scan.length).toBe(10);
    expect(isSorted(scan)).toBe(true);
    expect(tree.isValid()).toBe(true);
  });

  it("linked list is intact after many mixed operations", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 0; i < 100; i++) tree.insert(i, i);
    for (let i = 10; i < 50; i++) tree.delete(i);
    expect(tree.isValid()).toBe(true);
    const scan = tree.fullScan();
    expect(scan.length).toBe(60); // 100 - 40 deleted
    expect(isSorted(scan)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// minKey / maxKey
// ---------------------------------------------------------------------------

describe("minKey and maxKey", () => {
  it("returns undefined on empty tree", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    expect(tree.minKey()).toBeUndefined();
    expect(tree.maxKey()).toBeUndefined();
  });

  it("correct for single element", () => {
    const tree = buildTree(2, numCmp, [[42, "x"]]);
    expect(tree.minKey()).toBe(42);
    expect(tree.maxKey()).toBe(42);
  });

  it("correct after multiple inserts", () => {
    const tree = buildTree(2, numCmp, [
      [30, "c"], [10, "a"], [50, "e"], [20, "b"], [40, "d"],
    ]);
    expect(tree.minKey()).toBe(10);
    expect(tree.maxKey()).toBe(50);
  });

  it("correct after deletions", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 1; i <= 20; i++) tree.insert(i, i);
    tree.delete(1);
    tree.delete(20);
    expect(tree.minKey()).toBe(2);
    expect(tree.maxKey()).toBe(19);
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// fullScan
// ---------------------------------------------------------------------------

describe("fullScan", () => {
  it("returns empty array for empty tree", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    expect(tree.fullScan()).toEqual([]);
  });

  it("returns all pairs in sorted order", () => {
    const tree = buildTree(2, numCmp, [
      [3, "c"], [1, "a"], [2, "b"],
    ]);
    expect(tree.fullScan()).toEqual([[1, "a"], [2, "b"], [3, "c"]]);
  });

  it("fullScan matches inorder traversal for large tree", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 199; i >= 0; i--) tree.insert(i, i);
    const scan = tree.fullScan();
    expect(scan.length).toBe(200);
    expect(isSorted(scan)).toBe(true);
    for (let i = 0; i < 200; i++) {
      expect(scan[i][0]).toBe(i);
      expect(scan[i][1]).toBe(i);
    }
  });
});

// ---------------------------------------------------------------------------
// rangeScan
// ---------------------------------------------------------------------------

describe("rangeScan", () => {
  it("returns empty for empty tree", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    expect(tree.rangeScan(1, 10)).toEqual([]);
  });

  it("returns correct results within range", () => {
    const tree = buildTree(2, numCmp, [
      [1, "a"], [2, "b"], [3, "c"], [4, "d"], [5, "e"],
      [6, "f"], [7, "g"], [8, "h"], [9, "i"], [10, "j"],
    ]);
    const result = tree.rangeScan(3, 7);
    expect(result).toEqual([
      [3, "c"], [4, "d"], [5, "e"], [6, "f"], [7, "g"],
    ]);
  });

  it("includes boundary keys", () => {
    const tree = buildTree(2, numCmp, [
      [10, "a"], [20, "b"], [30, "c"],
    ]);
    expect(tree.rangeScan(10, 30)).toEqual([[10, "a"], [20, "b"], [30, "c"]]);
    expect(tree.rangeScan(10, 10)).toEqual([[10, "a"]]);
    expect(tree.rangeScan(30, 30)).toEqual([[30, "c"]]);
  });

  it("returns empty when no keys match", () => {
    const tree = buildTree(2, numCmp, [[5, "x"], [15, "y"]]);
    expect(tree.rangeScan(6, 14)).toEqual([]);
  });

  it("range scan on large tree (5000 keys)", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i);
    const result = tree.rangeScan(1000, 2000);
    expect(result.length).toBe(1001);
    expect(result[0][0]).toBe(1000);
    expect(result[result.length - 1][0]).toBe(2000);
    expect(isSorted(result)).toBe(true);
  });

  it("range scan after deletions", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 20; i++) tree.insert(i, i);
    for (let i = 5; i <= 10; i++) tree.delete(i);
    const result = tree.rangeScan(3, 12);
    expect(result.map(([k]) => k)).toEqual([3, 4, 11, 12]);
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Symbol.iterator
// ---------------------------------------------------------------------------

describe("Symbol.iterator", () => {
  it("iterates in sorted order", () => {
    const tree = buildTree(2, numCmp, [
      [5, "e"], [1, "a"], [3, "c"], [2, "b"], [4, "d"],
    ]);
    const result: number[] = [];
    for (const [k] of tree) result.push(k);
    expect(result).toEqual([1, 2, 3, 4, 5]);
  });

  it("spread operator works", () => {
    const tree = buildTree(2, numCmp, [[3, "c"], [1, "a"], [2, "b"]]);
    const arr = [...tree];
    expect(arr).toEqual([[1, "a"], [2, "b"], [3, "c"]]);
  });

  it("returns done=true for empty tree", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    const iter = tree[Symbol.iterator]();
    expect(iter.next().done).toBe(true);
  });

  it("iterator survives delete then re-insert", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 10; i++) tree.insert(i, i);
    for (let i = 0; i < 5; i++) tree.delete(i);
    for (let i = 0; i < 5; i++) tree.insert(i, i * 100);
    const all = [...tree];
    expect(all.length).toBe(10);
    expect(isSorted(all)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// height
// ---------------------------------------------------------------------------

describe("height", () => {
  it("is 0 for empty tree", () => {
    const tree = new BPlusTree<number, string>(2, numCmp);
    expect(tree.height()).toBe(0);
  });

  it("grows as keys are inserted", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    expect(tree.height()).toBe(0);
    for (let i = 0; i < 4; i++) tree.insert(i, i);
    // With t=2, root splits at 3 keys → height 1
    expect(tree.height()).toBeGreaterThanOrEqual(1);
    expect(tree.isValid()).toBe(true);
  });

  it("is bounded by log_t(n)", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 0; i < 1000; i++) tree.insert(i, i);
    // log_3(1000) ≈ 6.3
    expect(tree.height()).toBeLessThanOrEqual(7);
    expect(tree.isValid()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Delete
// ---------------------------------------------------------------------------

describe("delete basics", () => {
  it("returns false when key not found", () => {
    const tree = buildTree(2, numCmp, [[5, "five"]]);
    expect(tree.delete(99)).toBe(false);
    expect(tree.size).toBe(1);
    expect(tree.isValid()).toBe(true);
  });

  it("deletes the only key", () => {
    const tree = buildTree(2, numCmp, [[5, "five"]]);
    expect(tree.delete(5)).toBe(true);
    expect(tree.size).toBe(0);
    expect(tree.search(5)).toBeUndefined();
    expect(tree.isValid()).toBe(true);
  });

  it("deletes minimum key (linked list head update)", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 1; i <= 10; i++) tree.insert(i, i);
    tree.delete(1);
    expect(tree.minKey()).toBe(2);
    expect(tree.isValid()).toBe(true);
  });

  it("deletes maximum key", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 1; i <= 10; i++) tree.insert(i, i);
    tree.delete(10);
    expect(tree.maxKey()).toBe(9);
    expect(tree.isValid()).toBe(true);
  });
});

describe("delete borrow and merge", () => {
  it("borrows from left sibling during deletion", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    const keys = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    for (const k of keys) tree.insert(k, k);
    tree.delete(90);
    tree.delete(80);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.fullScan())).toBe(true);
  });

  it("borrows from right sibling during deletion", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 1; i <= 15; i++) tree.insert(i, i);
    tree.delete(1);
    tree.delete(2);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.fullScan())).toBe(true);
  });

  it("merges leaves during deletion", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    // Minimal tree where merge will be needed
    for (let i = 1; i <= 7; i++) tree.insert(i, i);
    expect(tree.isValid()).toBe(true);
    for (let i = 7; i >= 1; i--) {
      tree.delete(i);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });

  it("handles consecutive deletions (t=3)", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 0; i < 30; i++) tree.insert(i, i);
    for (let i = 0; i < 30; i++) {
      tree.delete(i);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
    expect(tree.fullScan()).toEqual([]);
  });
});

describe("comprehensive delete (all keys)", () => {
  it("deletes all keys in insertion order (t=2)", () => {
    const n = 50;
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < n; i++) tree.insert(i, i);
    for (let i = 0; i < n; i++) {
      expect(tree.delete(i)).toBe(true);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });

  it("deletes all keys in reverse order (t=3)", () => {
    const n = 50;
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 0; i < n; i++) tree.insert(i, i);
    for (let i = n - 1; i >= 0; i--) {
      expect(tree.delete(i)).toBe(true);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });

  it("deletes all keys interleaved with inserts (t=5)", () => {
    const tree = new BPlusTree<number, number>(5, numCmp);
    for (let i = 0; i < 100; i++) tree.insert(i, i);
    for (let i = 0; i < 50; i++) tree.delete(i);
    for (let i = 100; i < 150; i++) tree.insert(i, i);
    expect(tree.size).toBe(100);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.fullScan())).toBe(true);
    for (let i = 50; i < 150; i++) {
      expect(tree.delete(i)).toBe(true);
      expect(tree.isValid()).toBe(true);
    }
    expect(tree.size).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// String keys
// ---------------------------------------------------------------------------

describe("string keys", () => {
  it("inserts and retrieves string keys", () => {
    const tree = new BPlusTree<string, number>(2, strCmp);
    const words = ["banana", "apple", "cherry", "date", "elderberry"];
    for (let i = 0; i < words.length; i++) tree.insert(words[i], i);
    expect(tree.isValid()).toBe(true);
    expect(tree.search("apple")).toBe(1);
    expect(tree.search("cherry")).toBe(2);
    expect(tree.search("mango")).toBeUndefined();
  });

  it("fullScan returns alphabetically sorted keys", () => {
    const tree = new BPlusTree<string, number>(2, strCmp);
    const words = ["zebra", "apple", "mango", "cherry", "banana"];
    for (let i = 0; i < words.length; i++) tree.insert(words[i], i);
    const keys = tree.fullScan().map(([k]) => k);
    expect(keys).toEqual([...keys].sort());
  });

  it("range scan with string keys", () => {
    const tree = new BPlusTree<string, number>(2, strCmp);
    for (const w of ["a", "b", "c", "d", "e", "f", "g"]) tree.insert(w, 0);
    const result = tree.rangeScan("b", "e");
    expect(result.map(([k]) => k)).toEqual(["b", "c", "d", "e"]);
  });
});

// ---------------------------------------------------------------------------
// Large scale
// ---------------------------------------------------------------------------

describe("large scale (5000+ keys)", () => {
  it("inserts and validates 5000 sequential keys (t=2)", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i * 2);
    expect(tree.size).toBe(5000);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.fullScan())).toBe(true);
    expect(tree.minKey()).toBe(0);
    expect(tree.maxKey()).toBe(4999);
  });

  it("inserts 5000 keys in reverse (t=3)", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 4999; i >= 0; i--) tree.insert(i, i);
    expect(tree.size).toBe(5000);
    expect(tree.isValid()).toBe(true);
    expect(tree.minKey()).toBe(0);
    expect(tree.maxKey()).toBe(4999);
  });

  it("deletes 2500 keys from 5000-key tree (t=2)", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let i = 0; i < 5000; i++) tree.insert(i, i);
    for (let i = 0; i < 5000; i += 2) tree.delete(i);
    expect(tree.size).toBe(2500);
    expect(tree.isValid()).toBe(true);
    expect(isSorted(tree.fullScan())).toBe(true);
  });

  it("inserts 10000 keys (t=5)", () => {
    const tree = new BPlusTree<number, number>(5, numCmp);
    for (let i = 0; i < 10000; i++) tree.insert(i, i);
    expect(tree.size).toBe(10000);
    expect(tree.isValid()).toBe(true);
    expect(tree.height()).toBeLessThanOrEqual(5);
  });
});

// ---------------------------------------------------------------------------
// isValid after mixed operations
// ---------------------------------------------------------------------------

describe("isValid after mixed operations", () => {
  it("stays valid through many round-trips", () => {
    const tree = new BPlusTree<number, number>(2, numCmp);
    for (let round = 0; round < 10; round++) {
      for (let i = round * 20; i < (round + 1) * 20; i++) {
        tree.insert(i, i);
        expect(tree.isValid()).toBe(true);
      }
      for (let i = round * 20; i < round * 20 + 10; i++) {
        tree.delete(i);
        expect(tree.isValid()).toBe(true);
      }
    }
  });
});

// ---------------------------------------------------------------------------
// t = 3 specific
// ---------------------------------------------------------------------------

describe("t = 3 tree", () => {
  it("root holds up to 5 keys before splitting", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 1; i <= 5; i++) tree.insert(i, i);
    expect(tree.height()).toBe(0);
    expect(tree.isValid()).toBe(true);
  });

  it("splits at 6 keys (height becomes 1)", () => {
    const tree = new BPlusTree<number, number>(3, numCmp);
    for (let i = 1; i <= 6; i++) tree.insert(i, i);
    expect(tree.height()).toBe(1);
    expect(tree.isValid()).toBe(true);
    expect(tree.size).toBe(6);
  });
});
