import {
  defaultComparator,
  MaxHeap,
  MinHeap,
  type Comparator,
} from "./heap.js";

export function heapify<T>(
  array: readonly T[],
  comparator: Comparator<T> = defaultComparator
): T[] {
  return MinHeap.fromIterable(array, comparator).toArray();
}

export function heapSort<T>(
  array: readonly T[],
  comparator: Comparator<T> = defaultComparator
): T[] {
  const heap = MinHeap.fromIterable(array, comparator);
  const result: T[] = [];
  while (!heap.isEmpty()) {
    result.push(heap.pop());
  }
  return result;
}

export function nlargest<T>(
  iterable: Iterable<T>,
  n: number,
  comparator: Comparator<T> = defaultComparator
): T[] {
  if (n <= 0) {
    return [];
  }

  const items = Array.from(iterable);
  if (n >= items.length) {
    return [...items].sort((left, right) => comparator(right, left));
  }

  const heap = MinHeap.fromIterable(items.slice(0, n), comparator);
  for (const value of items.slice(n)) {
    if (comparator(value, heap.peek()) > 0) {
      heap.pop();
      heap.push(value);
    }
  }

  const result: T[] = [];
  while (!heap.isEmpty()) {
    result.push(heap.pop());
  }
  return result.sort((left, right) => comparator(right, left));
}

export function nsmallest<T>(
  iterable: Iterable<T>,
  n: number,
  comparator: Comparator<T> = defaultComparator
): T[] {
  if (n <= 0) {
    return [];
  }

  const items = Array.from(iterable);
  if (n >= items.length) {
    return [...items].sort(comparator);
  }

  const heap = MaxHeap.fromIterable(items.slice(0, n), comparator);
  for (const value of items.slice(n)) {
    if (comparator(value, heap.peek()) < 0) {
      heap.pop();
      heap.push(value);
    }
  }

  const result: T[] = [];
  while (!heap.isEmpty()) {
    result.push(heap.pop());
  }
  return result.sort(comparator);
}
