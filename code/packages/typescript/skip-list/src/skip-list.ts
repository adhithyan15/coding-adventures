export type Comparator<T> = (left: T, right: T) => number;

export function defaultComparator<T>(left: T, right: T): number {
  if (left === right) {
    return 0;
  }
  return (left as any) < (right as any) ? -1 : 1;
}

interface Entry<K, V> {
  key: K;
  value: V;
}

export class SkipList<K, V> {
  private readonly comparator: Comparator<K>;
  private readonly maxLevelValue: number;
  private readonly probabilityValue: number;
  private items: Array<Entry<K, V>> = [];

  constructor(
    comparator: Comparator<K> = defaultComparator as Comparator<K>,
    maxLevel = 32,
    probability = 0.5,
  ) {
    this.comparator = comparator;
    this.maxLevelValue = Math.max(1, Number.isFinite(maxLevel) ? Math.floor(maxLevel) : 32);
    this.probabilityValue =
      Number.isFinite(probability) && probability > 0 && probability < 1
        ? probability
        : 0.5;
  }

  static withParams<K, V>(
    maxLevel = 32,
    probability = 0.5,
    comparator: Comparator<K> = defaultComparator as Comparator<K>,
  ): SkipList<K, V> {
    return new SkipList(comparator, maxLevel, probability);
  }

  static newWithParams<K, V>(
    maxLevel = 32,
    probability = 0.5,
    comparator: Comparator<K> = defaultComparator as Comparator<K>,
  ): SkipList<K, V> {
    return SkipList.withParams(maxLevel, probability, comparator);
  }

  insert(key: K, value: V): void {
    const index = this.findInsertIndex(key);
    if (index < this.items.length && this.comparator(this.items[index].key, key) === 0) {
      this.items[index] = { key, value };
      return;
    }
    this.items.splice(index, 0, { key, value });
  }

  delete(key: K): boolean {
    const index = this.findIndex(key);
    if (index < 0) {
      return false;
    }
    this.items.splice(index, 1);
    return true;
  }

  search(key: K): V | undefined {
    const index = this.findIndex(key);
    return index < 0 ? undefined : this.items[index].value;
  }

  contains(key: K): boolean {
    return this.findIndex(key) >= 0;
  }

  containsKey(key: K): boolean {
    return this.contains(key);
  }

  rank(key: K): number | undefined {
    const index = this.findIndex(key);
    return index < 0 ? undefined : index;
  }

  byRank(rank: number): K | undefined {
    if (rank < 0 || rank >= this.items.length) {
      return undefined;
    }
    return this.items[rank]?.key;
  }

  rangeQuery(lo: K, hi: K, inclusive: boolean): Array<[K, V]> {
    return this.range(lo, hi, inclusive);
  }

  range(lo: K, hi: K, inclusive: boolean): Array<[K, V]> {
    if (this.comparator(lo, hi) > 0) {
      return [];
    }

    const lower = inclusive
      ? (value: K) => this.comparator(value, lo) >= 0
      : (value: K) => this.comparator(value, lo) > 0;
    const upper = inclusive
      ? (value: K) => this.comparator(value, hi) <= 0
      : (value: K) => this.comparator(value, hi) < 0;

    return this.items
      .filter((entry) => lower(entry.key) && upper(entry.key))
      .map((entry) => [entry.key, entry.value] as const);
  }

  toList(): K[] {
    return this.items.map((entry) => entry.key);
  }

  entriesList(): Array<[K, V]> {
    return this.items.map((entry) => [entry.key, entry.value] as const);
  }

  entries(): Array<[K, V]> {
    return this.entriesList();
  }

  min(): K | undefined {
    return this.items[0]?.key;
  }

  max(): K | undefined {
    return this.items[this.items.length - 1]?.key;
  }

  len(): number {
    return this.items.length;
  }

  size(): number {
    return this.len();
  }

  isEmpty(): boolean {
    return this.items.length === 0;
  }

  maxLevel(): number {
    return this.maxLevelValue;
  }

  probability(): number {
    return this.probabilityValue;
  }

  currentMax(): number {
    return this.estimatedCurrentMax();
  }

  iter(): IterableIterator<K> {
    return this.items.map((entry) => entry.key)[Symbol.iterator]();
  }

  private findIndex(key: K): number {
    let low = 0;
    let high = this.items.length - 1;
    while (low <= high) {
      const mid = Math.floor((low + high) / 2);
      const cmp = this.comparator(this.items[mid].key, key);
      if (cmp === 0) {
        return mid;
      }
      if (cmp < 0) {
        low = mid + 1;
      } else {
        high = mid - 1;
      }
    }
    return -1;
  }

  private findInsertIndex(key: K): number {
    let low = 0;
    let high = this.items.length;
    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (this.comparator(this.items[mid].key, key) < 0) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    return low;
  }

  private estimatedCurrentMax(): number {
    if (this.items.length === 0) {
      return 1;
    }
    const probability = this.probabilityValue;
    const levels = Math.ceil(Math.log(this.items.length) / Math.log(1 / probability));
    return Math.min(this.maxLevelValue, Math.max(1, levels));
  }
}
