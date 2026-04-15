import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(join(__dirname, "package.json"));
const native = require("./tree_set_native_node.node");

export class TreeSet {
  #native;

  constructor(values = []) {
    this.#native = new native.NativeTreeSet();
    for (const value of values) {
      this.add(value);
    }
  }

  add(value) {
    this.#native.add(value);
    return this;
  }

  delete(value) {
    return this.#native.delete(value);
  }

  discard(value) {
    return this.delete(value);
  }

  has(value) {
    return this.#native.contains(value);
  }

  contains(value) {
    return this.has(value);
  }

  size() {
    return this.#native.size();
  }

  get length() {
    return this.size();
  }

  isEmpty() {
    return this.#native.isEmpty();
  }

  min() {
    return this.#native.minValue();
  }

  max() {
    return this.#native.maxValue();
  }

  first() {
    return this.min();
  }

  last() {
    return this.max();
  }

  predecessor(value) {
    return this.#native.predecessor(value);
  }

  successor(value) {
    return this.#native.successor(value);
  }

  rank(value) {
    return this.#native.rank(value);
  }

  byRank(rank) {
    return this.#native.byRank(rank);
  }

  kthSmallest(k) {
    return this.#native.kthSmallest(k);
  }

  toArray() {
    return this.#native.toSortedArray();
  }

  toSortedArray() {
    return this.toArray();
  }

  range(min, max, inclusive = true) {
    return this.#native.range(min, max, inclusive);
  }

  union(other) {
    return new TreeSet(this.#native.unionValues(other.#native));
  }

  intersection(other) {
    return new TreeSet(this.#native.intersectionValues(other.#native));
  }

  difference(other) {
    return new TreeSet(this.#native.differenceValues(other.#native));
  }

  symmetricDifference(other) {
    return new TreeSet(this.#native.symmetricDifferenceValues(other.#native));
  }

  isSubset(other) {
    return this.#native.isSubset(other.#native);
  }

  isSuperset(other) {
    return this.#native.isSuperset(other.#native);
  }

  isDisjoint(other) {
    return this.#native.isDisjoint(other.#native);
  }

  equals(other) {
    return this.#native.equals(other.#native);
  }

  [Symbol.iterator]() {
    return this.toArray()[Symbol.iterator]();
  }

  toString() {
    return this.#native.toString();
  }
}

export function fromValues(values) {
  return new TreeSet(values);
}
