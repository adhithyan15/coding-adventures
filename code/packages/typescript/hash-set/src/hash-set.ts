import { HashMap } from "@coding-adventures/hash-map";

export type HashSetEntries<T> = Iterable<T>;

export class HashSet<T> {
  private readonly map: HashMap<T, true>;

  constructor(entries?: HashSetEntries<T>) {
    let map = new HashMap<T, true>();
    if (entries) {
      for (const entry of entries) {
        map = map.set(entry, true);
      }
    }
    this.map = map;
  }

  static fromList<T>(entries: Iterable<T>): HashSet<T> {
    return new HashSet(entries);
  }

  static fromListWithOptions<T>(
    entries: Iterable<T>,
    _capacity: number,
    _strategy: string,
    _hashFn: string,
  ): HashSet<T> {
    return new HashSet(entries);
  }

  static withOptions<T>(
    _capacity: number,
    _strategy: string,
    _hashFn: string,
  ): HashSet<T> {
    return new HashSet<T>();
  }

  clone(): HashSet<T> {
    return new HashSet(this.map.keys());
  }

  add(value: T): HashSet<T> {
    return new HashSet(this.map.set(value, true).keys());
  }

  remove(value: T): HashSet<T> {
    return new HashSet(this.map.delete(value).keys());
  }

  discard(value: T): HashSet<T> {
    return this.remove(value);
  }

  has(value: T): boolean {
    return this.map.has(value);
  }

  contains(value: T): boolean {
    return this.has(value);
  }

  get size(): number {
    return this.map.size;
  }

  len(): number {
    return this.size;
  }

  isEmpty(): boolean {
    return this.size === 0;
  }

  toList(): T[] {
    return this.map.keys();
  }

  union(other: HashSet<T>): HashSet<T> {
    return HashSet.fromList([...this.toList(), ...other.toList()]);
  }

  intersection(other: HashSet<T>): HashSet<T> {
    return HashSet.fromList(this.toList().filter((value) => other.has(value)));
  }

  difference(other: HashSet<T>): HashSet<T> {
    return HashSet.fromList(this.toList().filter((value) => !other.has(value)));
  }

  symmetricDifference(other: HashSet<T>): HashSet<T> {
    const left = this.toList().filter((value) => !other.has(value));
    const right = other.toList().filter((value) => !this.has(value));
    return HashSet.fromList([...left, ...right]);
  }

  isSubset(other: HashSet<T>): boolean {
    return this.toList().every((value) => other.has(value));
  }

  isSuperset(other: HashSet<T>): boolean {
    return other.isSubset(this);
  }

  isDisjoint(other: HashSet<T>): boolean {
    return this.toList().every((value) => !other.has(value));
  }

  equals(other: HashSet<T>): boolean {
    return this.size === other.size && this.isSubset(other);
  }
}
