export type HashMapEntries<K, V> = Iterable<readonly [K, V]>;

export class HashMap<K, V> {
  private readonly map: Map<K, V>;

  constructor(entries?: HashMapEntries<K, V>) {
    this.map = new Map(entries);
  }

  static fromEntries<K, V>(entries: HashMapEntries<K, V>): HashMap<K, V> {
    return new HashMap(entries);
  }

  clone(): HashMap<K, V> {
    return new HashMap(this.map);
  }

  get(key: K): V | undefined {
    return this.map.get(key);
  }

  has(key: K): boolean {
    return this.map.has(key);
  }

  set(key: K, value: V): HashMap<K, V> {
    const next = this.clone();
    next.map.set(key, value);
    return next;
  }

  delete(key: K): HashMap<K, V> {
    if (!this.map.has(key)) {
      return this.clone();
    }
    const next = this.clone();
    next.map.delete(key);
    return next;
  }

  clear(): HashMap<K, V> {
    return new HashMap();
  }

  keys(): K[] {
    return [...this.map.keys()];
  }

  values(): V[] {
    return [...this.map.values()];
  }

  entries(): Array<[K, V]> {
    return [...this.map.entries()];
  }

  get size(): number {
    return this.map.size;
  }

  toMap(): Map<K, V> {
    return new Map(this.map);
  }
}
