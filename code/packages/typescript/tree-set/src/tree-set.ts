export type Comparator<T> = (left: T, right: T) => number;

function defaultCompare(left: any, right: any): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

function lowerBound<T>(items: readonly T[], value: T, compare: Comparator<T>): number {
  let low = 0;
  let high = items.length;
  while (low < high) {
    const mid = (low + high) >>> 1;
    if (compare(items[mid]!, value) < 0) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return low;
}

function upperBound<T>(items: readonly T[], value: T, compare: Comparator<T>): number {
  let low = 0;
  let high = items.length;
  while (low < high) {
    const mid = (low + high) >>> 1;
    if (compare(items[mid]!, value) <= 0) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }
  return low;
}

function mergeUnique<T>(left: readonly T[], right: readonly T[], compare: Comparator<T>): T[] {
  const result: T[] = [];
  let li = 0;
  let ri = 0;
  while (li < left.length && ri < right.length) {
    const order = compare(left[li]!, right[ri]!);
    if (order < 0) {
      result.push(left[li++]!);
    } else if (order > 0) {
      result.push(right[ri++]!);
    } else {
      result.push(left[li++]!);
      ri += 1;
    }
  }
  while (li < left.length) {
    result.push(left[li++]!);
  }
  while (ri < right.length) {
    result.push(right[ri++]!);
  }
  return result;
}

function intersectionSorted<T>(left: readonly T[], right: readonly T[], compare: Comparator<T>): T[] {
  const result: T[] = [];
  let li = 0;
  let ri = 0;
  while (li < left.length && ri < right.length) {
    const order = compare(left[li]!, right[ri]!);
    if (order < 0) {
      li += 1;
    } else if (order > 0) {
      ri += 1;
    } else {
      result.push(left[li++]!);
      ri += 1;
    }
  }
  return result;
}

function differenceSorted<T>(left: readonly T[], right: readonly T[], compare: Comparator<T>): T[] {
  const result: T[] = [];
  let li = 0;
  let ri = 0;
  while (li < left.length && ri < right.length) {
    const order = compare(left[li]!, right[ri]!);
    if (order < 0) {
      result.push(left[li++]!);
    } else if (order > 0) {
      ri += 1;
    } else {
      li += 1;
      ri += 1;
    }
  }
  while (li < left.length) {
    result.push(left[li++]!);
  }
  return result;
}

function symmetricDifferenceSorted<T>(
  left: readonly T[],
  right: readonly T[],
  compare: Comparator<T>,
): T[] {
  const result: T[] = [];
  let li = 0;
  let ri = 0;
  while (li < left.length && ri < right.length) {
    const order = compare(left[li]!, right[ri]!);
    if (order < 0) {
      result.push(left[li++]!);
    } else if (order > 0) {
      result.push(right[ri++]!);
    } else {
      li += 1;
      ri += 1;
    }
  }
  while (li < left.length) {
    result.push(left[li++]!);
  }
  while (ri < right.length) {
    result.push(right[ri++]!);
  }
  return result;
}

function isSubsetSorted<T>(left: readonly T[], right: readonly T[], compare: Comparator<T>): boolean {
  let li = 0;
  let ri = 0;
  while (li < left.length && ri < right.length) {
    const order = compare(left[li]!, right[ri]!);
    if (order < 0) {
      return false;
    }
    if (order > 0) {
      ri += 1;
    } else {
      li += 1;
      ri += 1;
    }
  }
  return li === left.length;
}

function isDisjointSorted<T>(left: readonly T[], right: readonly T[], compare: Comparator<T>): boolean {
  let li = 0;
  let ri = 0;
  while (li < left.length && ri < right.length) {
    const order = compare(left[li]!, right[ri]!);
    if (order < 0) {
      li += 1;
    } else if (order > 0) {
      ri += 1;
    } else {
      return false;
    }
  }
  return true;
}

export class TreeSet<T> implements Iterable<T> {
  #values: T[];
  #compare: Comparator<T>;

  constructor(values: Iterable<T> = [], compare: Comparator<T> = defaultCompare) {
    this.#values = [];
    this.#compare = compare;
    for (const value of values) {
      this.add(value);
    }
  }

  add(value: T): this {
    const index = lowerBound(this.#values, value, this.#compare);
    if (index < this.#values.length && this.#compare(this.#values[index]!, value) === 0) {
      return this;
    }
    this.#values.splice(index, 0, value);
    return this;
  }

  delete(value: T): boolean {
    const index = lowerBound(this.#values, value, this.#compare);
    if (index >= this.#values.length || this.#compare(this.#values[index]!, value) !== 0) {
      return false;
    }
    this.#values.splice(index, 1);
    return true;
  }

  discard(value: T): boolean {
    return this.delete(value);
  }

  has(value: T): boolean {
    const index = lowerBound(this.#values, value, this.#compare);
    return index < this.#values.length && this.#compare(this.#values[index]!, value) === 0;
  }

  contains(value: T): boolean {
    return this.has(value);
  }

  size(): number {
    return this.#values.length;
  }

  get length(): number {
    return this.size();
  }

  isEmpty(): boolean {
    return this.#values.length === 0;
  }

  min(): T | undefined {
    return this.#values[0];
  }

  max(): T | undefined {
    return this.#values[this.#values.length - 1];
  }

  first(): T | undefined {
    return this.min();
  }

  last(): T | undefined {
    return this.max();
  }

  predecessor(value: T): T | undefined {
    const index = lowerBound(this.#values, value, this.#compare);
    return index > 0 ? this.#values[index - 1] : undefined;
  }

  successor(value: T): T | undefined {
    const index = upperBound(this.#values, value, this.#compare);
    return index < this.#values.length ? this.#values[index] : undefined;
  }

  rank(value: T): number {
    return lowerBound(this.#values, value, this.#compare);
  }

  byRank(rank: number): T | undefined {
    return rank >= 0 && rank < this.#values.length ? this.#values[rank] : undefined;
  }

  kthSmallest(k: number): T | undefined {
    return k <= 0 ? undefined : this.byRank(k - 1);
  }

  toArray(): T[] {
    return this.#values.slice();
  }

  toSortedArray(): T[] {
    return this.toArray();
  }

  range(min: T, max: T, inclusive = true): T[] {
    if (this.#compare(min, max) > 0) {
      return [];
    }
    const start = inclusive
      ? lowerBound(this.#values, min, this.#compare)
      : upperBound(this.#values, min, this.#compare);
    const end = inclusive
      ? upperBound(this.#values, max, this.#compare)
      : lowerBound(this.#values, max, this.#compare);
    return this.#values.slice(start, end);
  }

  union(other: TreeSet<T>): TreeSet<T> {
    return new TreeSet(mergeUnique(this.#values, other.#values, this.#compare), this.#compare);
  }

  intersection(other: TreeSet<T>): TreeSet<T> {
    return new TreeSet(
      intersectionSorted(this.#values, other.#values, this.#compare),
      this.#compare,
    );
  }

  difference(other: TreeSet<T>): TreeSet<T> {
    return new TreeSet(differenceSorted(this.#values, other.#values, this.#compare), this.#compare);
  }

  symmetricDifference(other: TreeSet<T>): TreeSet<T> {
    return new TreeSet(
      symmetricDifferenceSorted(this.#values, other.#values, this.#compare),
      this.#compare,
    );
  }

  isSubset(other: TreeSet<T>): boolean {
    return isSubsetSorted(this.#values, other.#values, this.#compare);
  }

  isSuperset(other: TreeSet<T>): boolean {
    return other.isSubset(this);
  }

  isDisjoint(other: TreeSet<T>): boolean {
    return isDisjointSorted(this.#values, other.#values, this.#compare);
  }

  equals(other: TreeSet<T>): boolean {
    if (this.#values.length !== other.#values.length) {
      return false;
    }
    for (let index = 0; index < this.#values.length; index += 1) {
      if (this.#compare(this.#values[index]!, other.#values[index]!) !== 0) {
        return false;
      }
    }
    return true;
  }

  [Symbol.iterator](): Iterator<T> {
    return this.#values[Symbol.iterator]();
  }

  toString(): string {
    return `TreeSet(${JSON.stringify(this.#values)})`;
  }
}

export function fromValues<T>(values: Iterable<T>, compare?: Comparator<T>): TreeSet<T> {
  return new TreeSet(values, compare);
}

