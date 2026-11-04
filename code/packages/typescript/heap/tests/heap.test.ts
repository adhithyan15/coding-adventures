import assert from "node:assert/strict";
import { describe, it } from "node:test";
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
    assert.equal(heap.peek(), 1);
    assert.equal(heap.pop(), 1);
    assert.equal(heap.pop(), 3);
    assert.equal(heap.pop(), 4);
    assert.equal(heap.pop(), 5);
    assert.equal(heap.pop(), 8);
  });

  it("keeps the heap property after each push and pop", () => {
    const heap = new MinHeap<number>();
    [5, 3, 8, 1, 4, 2, 7].forEach((value) => {
      heap.push(value);
      assert.equal(isValidMinHeap(heap.toArray()), true);
    });
    while (!heap.isEmpty()) {
      heap.pop();
      assert.equal(isValidMinHeap(heap.toArray()), true);
    }
  });

  it("throws on empty pop and peek", () => {
    const heap = new MinHeap<number>();
    assert.throws(() => heap.pop(), /empty heap/);
    assert.throws(() => heap.peek(), /empty heap/);
  });
});

describe("MaxHeap", () => {
  it("pushes and pops in descending order", () => {
    const heap = new MaxHeap<number>();
    [5, 3, 8, 1, 4].forEach((value) => heap.push(value));
    assert.equal(heap.peek(), 8);
    assert.equal(heap.pop(), 8);
    assert.equal(heap.pop(), 5);
    assert.equal(heap.pop(), 4);
    assert.equal(heap.pop(), 3);
    assert.equal(heap.pop(), 1);
  });

  it("keeps the heap property after each push and pop", () => {
    const heap = new MaxHeap<number>();
    [5, 3, 8, 1, 4, 2, 7].forEach((value) => {
      heap.push(value);
      assert.equal(isValidMaxHeap(heap.toArray()), true);
    });
    while (!heap.isEmpty()) {
      heap.pop();
      assert.equal(isValidMaxHeap(heap.toArray()), true);
    }
  });
});

describe("fromIterable", () => {
  it("builds min and max heaps with Floyd heapify", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    const minHeap = MinHeap.fromIterable(values);
    const maxHeap = MaxHeap.fromIterable(values);
    assert.equal(isValidMinHeap(minHeap.toArray()), true);
    assert.equal(isValidMaxHeap(maxHeap.toArray()), true);
    assert.equal(minHeap.peek(), 1);
    assert.equal(maxHeap.peek(), 9);
  });

  it("preserves all elements", () => {
    const values = [9, 2, 7, 1, 5];
    const heap = MinHeap.fromIterable(values);
    const popped: number[] = [];
    while (!heap.isEmpty()) {
      popped.push(heap.pop());
    }
    assert.deepEqual(popped, [...values].sort((a, b) => a - b));
  });
});

describe("pure functions", () => {
  it("heapify returns a valid min-heap without mutating input", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    const original = [...values];
    const heapArray = heapify(values);
    assert.equal(isValidMinHeap(heapArray), true);
    assert.deepEqual(values, original);
  });

  it("heapSort returns sorted output without mutating input", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    const original = [...values];
    assert.deepEqual(heapSort(values), [1, 1, 2, 3, 4, 5, 6, 9]);
    assert.deepEqual(values, original);
  });

  it("nlargest and nsmallest handle edge cases", () => {
    const values = [3, 1, 4, 1, 5, 9, 2, 6];
    assert.deepEqual(nlargest(values, 3), [9, 6, 5]);
    assert.deepEqual(nsmallest(values, 3), [1, 1, 2]);
    assert.deepEqual(nlargest(values, 0), []);
    assert.deepEqual(nsmallest(values, 0), []);
    assert.deepEqual(nlargest(values, 50), [...values].sort((a, b) => b - a));
    assert.deepEqual(nsmallest(values, 50), [...values].sort((a, b) => a - b));
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
      assert.deepEqual(heapSort(values), [...values].sort((a, b) => a - b));
    }
  });

  it("supports custom comparators", () => {
    const values = ["aaaa", "bb", "c", "ddd"];
    const comparator = (left: string, right: string): number =>
      left.length - right.length;
    const heap = MinHeap.fromIterable(values, comparator);
    assert.equal(heap.pop(), "c");
    assert.equal(heap.pop(), "bb");
    assert.equal(heap.pop(), "ddd");
    assert.equal(heap.pop(), "aaaa");
  });
});
