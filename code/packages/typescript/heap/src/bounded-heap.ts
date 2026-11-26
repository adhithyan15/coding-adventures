import type { Comparator } from "./heap.js";

export type PushResult<T> =
  | { status: "accepted" }
  | { status: "full"; rejected: T };

export type ValueResult<T> =
  | { status: "ok"; value: T }
  | { status: "empty" };

export type BuildResult<H, T> =
  | { status: "built"; heap: H }
  | { status: "capacity-exceeded"; heap: H; rejected: T };

export interface BoundedPriorityQueue<T> {
  readonly capacity: number;
  readonly size: number;
  isEmpty(): boolean;
  isFull(): boolean;
  tryPush(value: T): PushResult<T>;
  tryPeek(): ValueResult<T>;
  tryPop(): ValueResult<T>;
}

function validateCapacity(capacity: number): void {
  if (
    typeof capacity !== "number" ||
    !Number.isSafeInteger(capacity) ||
    capacity < 0 ||
    capacity > 0xffff_ffff
  ) {
    throw new RangeError("capacity must be an integer from 0 through 2^32 - 1");
  }
}

function validateComparator<T>(comparator: Comparator<T>): void {
  if (typeof comparator !== "function") {
    throw new TypeError("comparator must be callable");
  }
}

function buildFromIterable<H extends BoundedHeapCore<T>, T>(
  heap: H,
  items: Iterable<T>
): BuildResult<H, T> {
  for (const item of items) {
    const result = heap.tryPush(item);
    if (result.status === "full") {
      return { status: "capacity-exceeded", heap, rejected: item };
    }
  }
  return { status: "built", heap };
}

abstract class BoundedHeapCore<T> implements BoundedPriorityQueue<T> {
  private readonly data: T[] = [];

  protected constructor(
    readonly capacity: number,
    private readonly comparator: Comparator<T>,
    private readonly min: boolean
  ) {
    validateCapacity(capacity);
    validateComparator(comparator);
  }

  private comparePriority(left: T, right: T): boolean {
    const result = this.comparator(left, right);
    if (typeof result !== "number" || Number.isNaN(result)) {
      throw new TypeError("comparator must return a number other than NaN");
    }
    return this.min ? result < 0 : result > 0;
  }

  get size(): number {
    return this.data.length;
  }

  isEmpty(): boolean {
    return this.data.length === 0;
  }

  isFull(): boolean {
    return this.data.length === this.capacity;
  }

  tryPeek(): ValueResult<T> {
    if (this.data.length === 0) {
      return { status: "empty" };
    }
    return { status: "ok", value: this.data[0]! };
  }

  tryPush(value: T): PushResult<T> {
    if (this.data.length === this.capacity) {
      return { status: "full", rejected: value };
    }

    let current = this.data.length;
    const parents: number[] = [];
    while (current > 0) {
      const parent = Math.floor((current - 1) / 2);
      if (!this.comparePriority(value, this.data[parent]!)) {
        break;
      }
      parents.push(parent);
      current = parent;
    }

    // Array growth may fail before the queue has been logically changed.
    this.data.push(value);
    current = this.data.length - 1;
    for (const parent of parents) {
      this.data[current] = this.data[parent]!;
      current = parent;
    }
    this.data[current] = value;
    return { status: "accepted" };
  }

  tryPop(): ValueResult<T> {
    const length = this.data.length;
    if (length === 0) {
      return { status: "empty" };
    }

    const root = this.data[0]!;
    if (length === 1) {
      this.data.pop();
      return { status: "ok", value: root };
    }

    const newLength = length - 1;
    const candidate = this.data[newLength]!;
    const path: number[] = [];
    let current = 0;

    while (true) {
      const left = 2 * current + 1;
      if (left >= newLength) {
        break;
      }

      const right = left + 1;
      let best = left;
      if (
        right < newLength &&
        this.comparePriority(this.data[right]!, this.data[left]!)
      ) {
        best = right;
      }
      if (!this.comparePriority(this.data[best]!, candidate)) {
        break;
      }
      path.push(best);
      current = best;
    }

    // Comparisons are complete. Only now remove and shift the saved path.
    this.data.pop();
    current = 0;
    for (const child of path) {
      this.data[current] = this.data[child]!;
      current = child;
    }
    this.data[current] = candidate;
    return { status: "ok", value: root };
  }

}

export class BoundedMinHeap<T> extends BoundedHeapCore<T> {
  constructor(capacity: number, comparator: Comparator<T>) {
    super(capacity, comparator, true);
  }

  static fromIterable<T>(
    capacity: number,
    items: Iterable<T>,
    comparator: Comparator<T>
  ): BuildResult<BoundedMinHeap<T>, T> {
    const heap = new BoundedMinHeap<T>(capacity, comparator);
    return buildFromIterable(heap, items);
  }
}

export class BoundedMaxHeap<T> extends BoundedHeapCore<T> {
  constructor(capacity: number, comparator: Comparator<T>) {
    super(capacity, comparator, false);
  }

  static fromIterable<T>(
    capacity: number,
    items: Iterable<T>,
    comparator: Comparator<T>
  ): BuildResult<BoundedMaxHeap<T>, T> {
    const heap = new BoundedMaxHeap<T>(capacity, comparator);
    return buildFromIterable(heap, items);
  }
}
