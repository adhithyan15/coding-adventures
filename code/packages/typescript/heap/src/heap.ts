export type Comparator<T> = (left: T, right: T) => number;

export function defaultComparator<T>(left: T, right: T): number {
  if (left === right) {
    return 0;
  }
  return (left as any) < (right as any) ? -1 : 1;
}

export abstract class Heap<T> {
  protected readonly comparator: Comparator<T>;
  protected _data: T[] = [];

  constructor(comparator: Comparator<T> = defaultComparator) {
    this.comparator = comparator;
  }

  protected abstract _higherPriority(left: T, right: T): boolean;

  push(value: T): void {
    this._data.push(value);
    this._siftUp(this._data.length - 1);
  }

  pop(): T {
    if (this._data.length === 0) {
      throw new Error("pop from an empty heap");
    }

    const root = this._data[0];
    const last = this._data.pop()!;
    if (this._data.length > 0) {
      this._data[0] = last;
      this._siftDown(0);
    }

    return root;
  }

  peek(): T {
    if (this._data.length === 0) {
      throw new Error("peek at an empty heap");
    }
    return this._data[0];
  }

  isEmpty(): boolean {
    return this._data.length === 0;
  }

  toArray(): T[] {
    return [...this._data];
  }

  get size(): number {
    return this._data.length;
  }

  toString(): string {
    const root = this._data.length === 0 ? "empty" : String(this._data[0]);
    return `${this.constructor.name}(size=${this.size}, root=${root})`;
  }

  protected _buildFromIterable(items: Iterable<T>): void {
    this._data = Array.from(items);
    for (let i = Math.floor((this._data.length - 2) / 2); i >= 0; i--) {
      this._siftDown(i);
    }
  }

  protected _siftUp(index: number): void {
    let currentIndex = index;
    while (currentIndex > 0) {
      const parentIndex = Math.floor((currentIndex - 1) / 2);
      if (
        this._higherPriority(this._data[currentIndex], this._data[parentIndex])
      ) {
        [this._data[currentIndex], this._data[parentIndex]] = [
          this._data[parentIndex],
          this._data[currentIndex],
        ];
        currentIndex = parentIndex;
      } else {
        break;
      }
    }
  }

  protected _siftDown(index: number): void {
    let currentIndex = index;
    const total = this._data.length;

    while (true) {
      let best = currentIndex;
      const left = 2 * currentIndex + 1;
      const right = 2 * currentIndex + 2;

      if (left < total && this._higherPriority(this._data[left], this._data[best])) {
        best = left;
      }
      if (
        right < total &&
        this._higherPriority(this._data[right], this._data[best])
      ) {
        best = right;
      }

      if (best === currentIndex) {
        return;
      }

      [this._data[currentIndex], this._data[best]] = [
        this._data[best],
        this._data[currentIndex],
      ];
      currentIndex = best;
    }
  }
}

export class MinHeap<T> extends Heap<T> {
  constructor(comparator: Comparator<T> = defaultComparator) {
    super(comparator);
  }

  static fromIterable<T>(
    items: Iterable<T>,
    comparator: Comparator<T> = defaultComparator
  ): MinHeap<T> {
    const heap = new MinHeap<T>(comparator);
    heap._buildFromIterable(items);
    return heap;
  }

  protected _higherPriority(left: T, right: T): boolean {
    return this.comparator(left, right) < 0;
  }
}

export class MaxHeap<T> extends Heap<T> {
  constructor(comparator: Comparator<T> = defaultComparator) {
    super(comparator);
  }

  static fromIterable<T>(
    items: Iterable<T>,
    comparator: Comparator<T> = defaultComparator
  ): MaxHeap<T> {
    const heap = new MaxHeap<T>(comparator);
    heap._buildFromIterable(items);
    return heap;
  }

  protected _higherPriority(left: T, right: T): boolean {
    return this.comparator(left, right) > 0;
  }
}
