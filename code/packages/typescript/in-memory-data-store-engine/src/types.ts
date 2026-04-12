import { HashMap } from "@coding-adventures/hash-map";
import { HashSet } from "@coding-adventures/hash-set";
import { MinHeap } from "@coding-adventures/heap";
import { HyperLogLog } from "@coding-adventures/hyperloglog";
import { SkipList, type Comparator } from "@coding-adventures/skip-list";

export type EntryType = "string" | "hash" | "list" | "set" | "zset" | "hll";

export interface StringEntryValue {
  readonly kind: "string";
  readonly value: string;
}

export interface HashEntryValue {
  readonly kind: "hash";
  readonly value: HashMap<string, string>;
}

export interface ListEntryValue {
  readonly kind: "list";
  readonly value: string[];
}

export interface SetEntryValue {
  readonly kind: "set";
  readonly value: HashSet<string>;
}

export interface ZSetEntryValue {
  readonly kind: "zset";
  readonly value: SortedSet;
}

export interface HllEntryValue {
  readonly kind: "hll";
  readonly value: HyperLogLog;
}

export type EntryValue =
  | StringEntryValue
  | HashEntryValue
  | ListEntryValue
  | SetEntryValue
  | ZSetEntryValue
  | HllEntryValue;

export interface Entry {
  readonly entryType: EntryType;
  readonly value: EntryValue;
  readonly expiresAt: number | null;
}

export interface SortedEntry {
  readonly score: number;
  readonly member: string;
}

const sortedEntryComparator: Comparator<SortedEntry> = (left, right) => {
  if (left.score !== right.score) {
    return left.score < right.score ? -1 : 1;
  }
  return left.member.localeCompare(right.member);
};

export class SortedSet {
  private members: HashMap<string, number>;
  private ordering: SkipList<SortedEntry, null>;

  constructor() {
    this.members = new HashMap<string, number>();
    this.ordering = new SkipList(sortedEntryComparator);
  }

  static new(): SortedSet {
    return new SortedSet();
  }

  clone(): SortedSet {
    const cloned = new SortedSet();
    for (const [member, score] of this.orderedEntries()) {
      cloned.insert(score, member);
    }
    return cloned;
  }

  len(): number {
    return this.members.size;
  }

  isEmpty(): boolean {
    return this.len() === 0;
  }

  contains(member: string): boolean {
    return this.members.has(member);
  }

  score(member: string): number | undefined {
    return this.members.get(member);
  }

  insert(score: number, member: string): boolean {
    if (Number.isNaN(score)) {
      throw new Error("sorted set score cannot be NaN");
    }
    const isNew = !this.members.has(member);
    const existing = this.members.get(member);
    if (existing !== undefined) {
      this.ordering.delete({ score: existing, member });
    }
    this.members = this.members.set(member, score);
    this.ordering.insert({ score, member }, null);
    return isNew;
  }

  remove(member: string): boolean {
    const oldScore = this.members.get(member);
    if (oldScore === undefined) {
      return false;
    }
    this.ordering.delete({ score: oldScore, member });
    this.members = this.members.delete(member);
    return true;
  }

  rank(member: string): number | null {
    let index = 0;
    for (const entry of this.ordering.iter()) {
      if (entry.member === member) {
        return index;
      }
      index += 1;
    }
    return null;
  }

  orderedEntries(): Array<[string, number]> {
    return Array.from(this.ordering.iter(), (entry) => [entry.member, entry.score]);
  }

  rangeByIndex(start: number, end: number): Array<[string, number]> {
    const entries = this.orderedEntries();
    if (entries.length === 0) {
      return [];
    }
    const len = entries.length;
    const normalizedStart = start < 0 ? len + start : start;
    const normalizedEnd = end < 0 ? len + end : end;
    if (
      normalizedStart < 0 ||
      normalizedEnd < 0 ||
      normalizedStart >= len ||
      normalizedStart > normalizedEnd
    ) {
      return [];
    }
    return entries.slice(normalizedStart, normalizedEnd + 1);
  }

  rangeByScore(min: number, max: number): Array<[string, number]> {
    if (Number.isNaN(min) || Number.isNaN(max)) {
      throw new Error("sorted set score cannot be NaN");
    }
    return this.orderedEntries().filter(([, score]) => score >= min && score <= max);
  }
}

export function stringEntry(value: string, expiresAt: number | null = null): Entry {
  return {
    entryType: "string",
    value: { kind: "string", value },
    expiresAt,
  };
}

export function hashEntry(value: HashMap<string, string>, expiresAt: number | null = null): Entry {
  return {
    entryType: "hash",
    value: { kind: "hash", value },
    expiresAt,
  };
}

export function listEntry(value: string[], expiresAt: number | null = null): Entry {
  return {
    entryType: "list",
    value: { kind: "list", value },
    expiresAt,
  };
}

export function setEntry(value: HashSet<string>, expiresAt: number | null = null): Entry {
  return {
    entryType: "set",
    value: { kind: "set", value },
    expiresAt,
  };
}

export function zsetEntry(value: SortedSet, expiresAt: number | null = null): Entry {
  return {
    entryType: "zset",
    value: { kind: "zset", value },
    expiresAt,
  };
}

export function hllEntry(value: HyperLogLog, expiresAt: number | null = null): Entry {
  return {
    entryType: "hll",
    value: { kind: "hll", value },
    expiresAt,
  };
}

export function cloneEntryValue(value: EntryValue): EntryValue {
  switch (value.kind) {
    case "string":
      return { kind: "string", value: value.value };
    case "hash":
      return { kind: "hash", value: value.value.clone() };
    case "list":
      return { kind: "list", value: value.value.slice() };
    case "set":
      return { kind: "set", value: value.value.clone() };
    case "zset":
      return { kind: "zset", value: value.value.clone() };
    case "hll":
      return { kind: "hll", value: value.value.clone() };
  }
}

export function cloneEntry(entry: Entry): Entry {
  return {
    entryType: entry.entryType,
    value: cloneEntryValue(entry.value),
    expiresAt: entry.expiresAt,
  };
}

export function entryValueType(value: EntryValue): EntryType {
  return value.kind;
}

export const EXPIRY_COMPARATOR: Comparator<[number, string]> = (left, right) => {
  if (left[0] !== right[0]) {
    return left[0] < right[0] ? -1 : 1;
  }
  return left[1].localeCompare(right[1]);
};

export function createExpiryHeap(entries?: Iterable<[number, string]>): MinHeap<[number, string]> {
  return MinHeap.fromIterable(entries ?? [], EXPIRY_COMPARATOR);
}
