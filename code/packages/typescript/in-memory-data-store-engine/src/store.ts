import { HashMap } from "@coding-adventures/hash-map";
import { type Entry, type EntryType, type SortedSet, cloneEntry, createExpiryHeap, stringEntry } from "./types.js";

export const DEFAULT_DB_COUNT = 16;

export function currentTimeMs(): number {
  return Date.now();
}

export class Database {
  entries: HashMap<string, Entry>;
  ttlHeap: ReturnType<typeof createExpiryHeap>;

  constructor(
    entries: HashMap<string, Entry> = new HashMap<string, Entry>(),
    ttlHeap = createExpiryHeap(),
  ) {
    this.entries = entries;
    this.ttlHeap = ttlHeap;
  }

  static empty(): Database {
    return new Database();
  }

  clone(): Database {
    return new Database(this.entries.clone(), createExpiryHeap(this.ttlHeap.toArray()));
  }

  get(key: string): Entry | undefined {
    const entry = this.entries.get(key);
    if (!entry) {
      return undefined;
    }
    if (entry.expiresAt !== null && currentTimeMs() >= entry.expiresAt) {
      return undefined;
    }
    return entry;
  }

  set(key: string, entry: Entry): Database {
    const next = this.clone();
    next.entries = next.entries.set(key, cloneEntry(entry));
    if (entry.expiresAt !== null) {
      next.ttlHeap.push([entry.expiresAt, key]);
    }
    return next;
  }

  delete(key: string): Database {
    const next = this.clone();
    next.entries = next.entries.delete(key);
    return next;
  }

  exists(key: string): boolean {
    return this.get(key) !== undefined;
  }

  typeOf(key: string): EntryType | null {
    return this.get(key)?.entryType ?? null;
  }

  keys(pattern: string): string[] {
    return this.entries
      .keys()
      .filter((key) => this.get(key) !== undefined && globMatch(pattern, key))
      .sort();
  }

  dbsize(): number {
    return this.entries.keys().filter((key) => this.get(key) !== undefined).length;
  }

  expireLazy(key?: string): Database {
    if (key === undefined) {
      return this.clone();
    }
    const entry = this.entries.get(key);
    if (!entry || entry.expiresAt === null || currentTimeMs() < entry.expiresAt) {
      return this.clone();
    }
    return this.delete(key);
  }

  activeExpire(): Database {
    const next = this.clone();
    const now = currentTimeMs();
    while (!next.ttlHeap.isEmpty()) {
      const [expiresAt, key] = next.ttlHeap.peek();
      if (expiresAt > now) {
        break;
      }
      next.ttlHeap.pop();
      const current = next.entries.get(key);
      if (current && current.expiresAt === expiresAt) {
        next.entries = next.entries.delete(key);
      }
    }
    return next;
  }

  clear(): Database {
    return Database.empty();
  }
}

export class Store {
  databases: Database[];
  activeDb: number;

  constructor(databases: Database[] = createDatabases(DEFAULT_DB_COUNT), activeDb = 0) {
    this.databases = databases;
    this.activeDb = activeDb;
  }

  static empty(dbCount = DEFAULT_DB_COUNT): Store {
    return new Store(createDatabases(dbCount), 0);
  }

  clone(): Store {
    return new Store(this.databases.map((database) => database.clone()), this.activeDb);
  }

  withActiveDb(activeDb: number): Store {
    return new Store(this.databases.map((database) => database.clone()), clampDb(activeDb, this.databases.length));
  }

  select(activeDb: number): Store {
    return this.withActiveDb(activeDb);
  }

  currentDb(): Database {
    return this.databases[this.activeDb];
  }

  get(key: string): Entry | undefined {
    return this.currentDb().get(key);
  }

  set(key: string, entry: Entry): Store {
    const next = this.clone();
    next.databases[next.activeDb] = next.currentDb().set(key, entry);
    return next;
  }

  delete(key: string): Store {
    const next = this.clone();
    next.databases[next.activeDb] = next.currentDb().delete(key);
    return next;
  }

  exists(key: string): boolean {
    return this.get(key) !== undefined;
  }

  keys(pattern: string): string[] {
    return this.currentDb().keys(pattern);
  }

  typeOf(key: string): EntryType | null {
    return this.currentDb().typeOf(key);
  }

  dbsize(): number {
    return this.currentDb().dbsize();
  }

  expireLazy(key?: string): Store {
    const next = this.clone();
    next.databases[next.activeDb] = next.currentDb().expireLazy(key);
    return next;
  }

  activeExpire(): Store {
    const next = this.clone();
    next.databases[next.activeDb] = next.currentDb().activeExpire();
    return next;
  }

  activeExpireAll(): Store {
    const next = this.clone();
    next.databases = next.databases.map((database) => database.activeExpire());
    return next;
  }

  flushdb(): Store {
    const next = this.clone();
    next.databases[next.activeDb] = Database.empty();
    return next;
  }

  flushall(): Store {
    return new Store(createDatabases(this.databases.length), this.activeDb);
  }
}

function createDatabases(count: number): Database[] {
  return Array.from({ length: count }, () => Database.empty());
}

function clampDb(index: number, length: number): number {
  if (length <= 0) {
    return 0;
  }
  return Math.min(Math.max(0, index), length - 1);
}

function globMatch(pattern: string, text: string): boolean {
  const p = pattern;
  const t = text;
  let pi = 0;
  let ti = 0;
  let star = -1;
  let match = 0;
  while (ti < t.length) {
    if (pi < p.length && (p[pi] === "?" || p[pi] === t[ti])) {
      pi += 1;
      ti += 1;
    } else if (pi < p.length && p[pi] === "*") {
      star = pi;
      match = ti;
      pi += 1;
    } else if (star !== -1) {
      pi = star + 1;
      match += 1;
      ti = match;
    } else {
      return false;
    }
  }
  while (pi < p.length && p[pi] === "*") {
    pi += 1;
  }
  return pi === p.length;
}
