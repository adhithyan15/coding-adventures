import { describe, expect, it } from "vitest";
import {
  heapify,
  heapSort,
  MaxHeap,
  MinHeap,
  nlargest,
  nsmallest,
} from "../src/index.js";

function isValidMinHeap(arr: number[]): boolean {
  for (let i = 0; i < arr.length; i++) {
    const left = 2 * i + 1;
    const right = 2 * i + 2;
    if (left < arr.length && arr[i] > arr[left]) {
      return false;
    }
    if (right < arr.length && arr[i] > arr[right]) {
      return false;
    }
  }
  return true;
}

function isValidMaxHeap(arr: number[]): boolean {
  for (let i = 0; i < arr.length; i++) {
    const left = 2 * i + 1;
    const right = 2 * i + 2;
    if (left < arr.length && arr[i] < arr[left]) {
      return false;
    }
    if (right < arr.length && arr[i] < arr[right]) {
      return false;
    }
  }
  return true;
}

describe("MinHeap", () => {
  it("pushes and pops in ascending order", () => {
    const heap = new MinHeap<number>();
    [5, 3, 8, 1, 4].forEach((value) => heap.push(value));
    expect(heap.peek()).toBe(1);
    expect(heap.pop()).toBe(1);
    expect(heap.pop()).toBe(3);
    expect(heap.pop()).toBe(4);
    expect(heap.pop()).toBe(5);
    expect(heap.pop()).toBe(8);
  });

  it("keeps the heap property after each push and pop", () => {
    const heap = new MinHeap<number>();
    [5, 3, 8, 1, 4, 2, 7].forEach((value) => {
      heap.push(value);
      expect(isValidMinHeap(heap.toArray())).toBe(true);
    });
    while (!heap.isEmpty()) {
      heap.pop();
      expect(isValidMinHeap(heap.toArray())).toBe(true);
    }
  });

  it("throws on empty pop and peek", () => {
    const heap = new MinHeap<number>();
    expect(() => heap.pop()).toThrow(/empty heap/);
    expect(() => heap.peek()).toThrow(/empty heap/);
  });
});

describe("MaxHeap", () => {
  it("pushes and pops in descending order", () => {
    const heap = new MaxHeap<number>();
    [5, 3, 8, 1, 4].forEach((value) => heap.push(value));
    expect(heap.peek()).toBe(8);
    expect(heap.pop()).toBe(8);
    expect(heap.pop()).toBe(5);
    expect(heap.pop()).toBe(4);
    expect(heap.pop()).toBe(3);
    expect(heap.pop()).toBe(1);
  });

  it("keeps the heap property after each push and pop", () => {
    const heap = new MaxHeap<number>();
    [5, 3, 8, 1, 4, 2, 7].forEach((value) => {
      heap.push(value);
      expect(isValidMaxHeap(heap.toArray())).toBe(true);
    });
    while (!heap.isEmpty()) {
      heap.pop();
      expect(isValidMaxHeap(heap.toArray())).toBe(true);
    }
  });
});

describe("fromIterable", () => {
  it("builds min and max heaps with Floyd heapify", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    const minHeap = MinHeap.fromIterable(values);
    const maxHeap = MaxHeap.fromIterable(values);
    expect(isValidMinHeap(minHeap.toArray())).toBe(true);
    expect(isValidMaxHeap(maxHeap.toArray())).toBe(true);
    expect(minHeap.peek()).toBe(1);
    expect(maxHeap.peek()).toBe(9);
  });

  it("preserves all elements", () => {
    const values = [9, 2, 7, 1, 5];
    const heap = MinHeap.fromIterable(values);
    const popped: number[] = [];
    while (!heap.isEmpty()) {
      popped.push(heap.pop());
    }
    expect(popped).toEqual([...values].sort((a, b) => a - b));
  });
});

describe("pure functions", () => {
  it("heapify returns a valid min-heap without mutating input", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    const original = [...values];
    const heapArray = heapify(values);
    expect(isValidMinHeap(heapArray)).toBe(true);
    expect(values).toEqual(original);
  });

  it("heapSort returns sorted output without mutating input", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    const original = [...values];
    expect(heapSort(values)).toEqual([1, 1, 2, 3, 4, 5, 6, 9]);
    expect(values).toEqual(original);
  });

  it("nlargest and nsmallest handle edge cases", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    expect(nlargest(values, 3)).toEqual([9, 6, 5]);
    expect(nsmallest(values, 3)).toEqual([1, 1, 2]);
    expect(nlargest(values, 0)).toEqual([]);
    expect(nsmallest(values, 0)).toEqual([]);
    expect(nlargest(values, 50)).toEqual([...values].sort((a, b) => b - a));
    expect(nsmallest(values, 50)).toEqual([...values].sort((a, b) => a - b));
  });
});

describe("stress and generic usage", () => {
  it("matches Array.sort for random arrays", () => {
    const rng = 42;
    let seed = rng;
    const nextInt = (): number => {
      seed = (seed * 1664525 + 1013904223) >>> 0;
      return seed % 2001 - 1000;
    };

    for (let run = 0; run < 100; run++) {
      const length = Math.abs(nextInt()) % 100;
      const values = Array.from({ length }, () => nextInt());
      expect(heapSort(values)).toEqual([...values].sort((a, b) => a - b));
    }
  });

  it("supports custom comparators", () => {
    const values = ["aaaa", "bb", "c", "ddd"];
    const comparator = (left: string, right: string): number =>
      left.length - right.length;
    const heap = MinHeap.fromIterable(values, comparator);
    expect(heap.pop()).toBe("c");
    expect(heap.pop()).toBe("bb");
    expect(heap.pop()).toBe("ddd");
    expect(heap.pop()).toBe("aaaa");
  });
});
