import { describe, expect, it } from "vitest";
import {
  EmptyTreeError,
  FenwickError,
  FenwickTree,
  IndexOutOfRangeError,
} from "../src/index.js";

function brutePrefix(values: readonly number[], index: number): number {
  return values.slice(0, index).reduce((sum, value) => sum + value, 0);
}

function bruteRange(values: readonly number[], left: number, right: number): number {
  return values.slice(left - 1, right).reduce((sum, value) => sum + value, 0);
}

describe("construction", () => {
  it("constructs empty and sized trees", () => {
    expect(new FenwickTree(0).length).toBe(0);
    expect(new FenwickTree(5).length).toBe(5);
  });

  it("rejects invalid sizes", () => {
    expect(() => new FenwickTree(-1)).toThrow(FenwickError);
    expect(() => new FenwickTree(2.5)).toThrow(FenwickError);
  });

  it("builds from list in O(n) and preserves prefix sums", () => {
    const values = [3, 2, 1, 7, 4];
    const tree = FenwickTree.fromList(values);
    expect(tree.prefixSum(1)).toBe(3);
    expect(tree.prefixSum(2)).toBe(5);
    expect(tree.prefixSum(3)).toBe(6);
    expect(tree.prefixSum(4)).toBe(13);
    expect(tree.prefixSum(5)).toBe(17);
  });
});

describe("prefix and range queries", () => {
  it("handles zero prefix and full ranges", () => {
    const values = [3, 2, 1, 7, 4];
    const tree = FenwickTree.fromList(values);
    expect(tree.prefixSum(0)).toBe(0);
    expect(tree.rangeSum(1, 5)).toBe(17);
    expect(tree.rangeSum(2, 4)).toBe(10);
  });

  it("supports pointQuery as rangeSum(i, i)", () => {
    const values = [3, 2, 1, 7, 4];
    const tree = FenwickTree.fromList(values);
    values.forEach((value, idx) => {
      expect(tree.pointQuery(idx + 1)).toBe(value);
    });
  });

  it("validates query bounds", () => {
    const tree = FenwickTree.fromList([1, 2, 3]);
    expect(() => tree.prefixSum(-1)).toThrow(IndexOutOfRangeError);
    expect(() => tree.prefixSum(4)).toThrow(IndexOutOfRangeError);
    expect(() => tree.rangeSum(3, 1)).toThrow(FenwickError);
    expect(() => tree.rangeSum(0, 2)).toThrow(IndexOutOfRangeError);
  });
});

describe("updates", () => {
  it("applies positive and negative point updates", () => {
    const tree = FenwickTree.fromList([3, 2, 1, 7, 4]);
    tree.update(3, 5);
    expect(tree.pointQuery(3)).toBe(6);
    expect(tree.prefixSum(3)).toBe(11);
    tree.update(2, -1);
    expect(tree.pointQuery(2)).toBe(1);
    expect(tree.prefixSum(3)).toBe(10);
  });

  it("validates update bounds", () => {
    const tree = FenwickTree.fromList([1, 2, 3]);
    expect(() => tree.update(0, 1)).toThrow(IndexOutOfRangeError);
    expect(() => tree.update(4, 1)).toThrow(IndexOutOfRangeError);
  });

  it("handles propagation from index 1 across powers of two", () => {
    const tree = FenwickTree.fromList([0, 0, 0, 0, 0, 0, 0, 0]);
    tree.update(1, 10);
    for (let i = 1; i <= 8; i++) {
      expect(tree.prefixSum(i)).toBe(10);
    }
  });
});

describe("findKth", () => {
  it("matches the documented examples", () => {
    const tree = FenwickTree.fromList([1, 2, 3, 4, 5]); // 1,3,6,10,15
    expect(tree.findKth(1)).toBe(1);
    expect(tree.findKth(2)).toBe(2);
    expect(tree.findKth(3)).toBe(2);
    expect(tree.findKth(4)).toBe(3);
    expect(tree.findKth(10)).toBe(4);
    expect(tree.findKth(11)).toBe(5);
  });

  it("validates invalid k values", () => {
    const tree = FenwickTree.fromList([1, 2, 3]);
    expect(() => tree.findKth(0)).toThrow(FenwickError);
    expect(() => tree.findKth(-2)).toThrow(FenwickError);
    expect(() => tree.findKth(100)).toThrow(FenwickError);
    expect(() => new FenwickTree(0).findKth(1)).toThrow(EmptyTreeError);
  });
});

describe("correctness against brute force", () => {
  it("matches all prefix and range sums on random arrays", () => {
    let seed = 1337;
    const next = (): number => {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      return seed;
    };

    for (let run = 0; run < 120; run++) {
      const n = (next() % 30) + 1;
      const values = Array.from({ length: n }, () => (next() % 101) - 50);
      const tree = FenwickTree.fromList(values);

      for (let i = 1; i <= n; i++) {
        expect(tree.prefixSum(i)).toBe(brutePrefix(values, i));
      }
      for (let left = 1; left <= n; left++) {
        for (let right = left; right <= n; right++) {
          expect(tree.rangeSum(left, right)).toBe(bruteRange(values, left, right));
        }
      }
    }
  });

  it("stays consistent under interleaved updates and queries", () => {
    let seed = 99;
    const next = (): number => {
      seed = (seed * 1664525 + 1013904223) >>> 0;
      return seed;
    };

    const n = 60;
    const values = Array.from({ length: n }, () => (next() % 20) + 1);
    const tree = FenwickTree.fromList(values);

    for (let i = 0; i < 1200; i++) {
      if (next() % 10 < 4) {
        const left = (next() % n) + 1;
        const right = left + (next() % (n - left + 1));
        expect(tree.rangeSum(left, right)).toBe(bruteRange(values, left, right));
      } else {
        const idx = (next() % n) + 1;
        const delta = (next() % 41) - 20;
        values[idx - 1] += delta;
        tree.update(idx, delta);
      }
    }
  });
});
