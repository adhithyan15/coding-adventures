export class TreeSet {
  constructor(values?: Iterable<number>);

  add(value: number): this;
  delete(value: number): boolean;
  discard(value: number): boolean;
  has(value: number): boolean;
  contains(value: number): boolean;
  size(): number;
  readonly length: number;
  isEmpty(): boolean;
  min(): number | undefined;
  max(): number | undefined;
  first(): number | undefined;
  last(): number | undefined;
  predecessor(value: number): number | undefined;
  successor(value: number): number | undefined;
  rank(value: number): number;
  byRank(rank: number): number | undefined;
  kthSmallest(k: number): number | undefined;
  toArray(): number[];
  toSortedArray(): number[];
  range(min: number, max: number, inclusive?: boolean): number[];
  union(other: TreeSet): TreeSet;
  intersection(other: TreeSet): TreeSet;
  difference(other: TreeSet): TreeSet;
  symmetricDifference(other: TreeSet): TreeSet;
  isSubset(other: TreeSet): boolean;
  isSuperset(other: TreeSet): boolean;
  isDisjoint(other: TreeSet): boolean;
  equals(other: TreeSet): boolean;
  toString(): string;
  [Symbol.iterator](): Iterator<number>;
}

