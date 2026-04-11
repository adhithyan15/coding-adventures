import { HashSet } from "@coding-adventures/hash-set";
import {
  array,
  bulkString,
  errorValue,
  integer,
  simpleString,
  type RespValue,
} from "@coding-adventures/resp-protocol";
import { HyperLogLog } from "@coding-adventures/hyperloglog";
import { HashMap } from "@coding-adventures/hash-map";
import { currentTimeMs, Database, Store } from "./store.js";
import {
  type Entry,
  SortedSet,
  cloneEntry,
  entryValueType,
  hashEntry,
  hllEntry,
  listEntry,
  setEntry,
  stringEntry,
  zsetEntry,
} from "./types.js";

export type CommandResult = [Store, RespValue];
export type CommandHandler = (store: Store, args: string[]) => CommandResult;

export function installDefaultCommands(register: (name: string, handler: CommandHandler) => void): void {
  register("PING", cmdPing);
  register("ECHO", cmdEcho);
  register("SET", cmdSet);
  register("GET", cmdGet);
  register("DEL", cmdDel);
  register("EXISTS", cmdExists);
  register("TYPE", cmdType);
  register("RENAME", cmdRename);
  register("INCR", cmdIncr);
  register("DECR", cmdDecr);
  register("INCRBY", cmdIncrby);
  register("DECRBY", cmdDecrby);
  register("APPEND", cmdAppend);
  register("HSET", cmdHset);
  register("HGET", cmdHget);
  register("HDEL", cmdHdel);
  register("HGETALL", cmdHgetall);
  register("HLEN", cmdHlen);
  register("HEXISTS", cmdHexists);
  register("HKEYS", cmdHkeys);
  register("HVALS", cmdHvals);
  register("LPUSH", cmdLpush);
  register("RPUSH", cmdRpush);
  register("LPOP", cmdLpop);
  register("RPOP", cmdRpop);
  register("LLEN", cmdLlen);
  register("LRANGE", cmdLrange);
  register("LINDEX", cmdLindex);
  register("SADD", cmdSadd);
  register("SREM", cmdSrem);
  register("SISMEMBER", cmdSismember);
  register("SMEMBERS", cmdSmembers);
  register("SCARD", cmdScard);
  register("SUNION", cmdSunion);
  register("SINTER", cmdSinter);
  register("SDIFF", cmdSdiff);
  register("ZADD", cmdZadd);
  register("ZRANGE", cmdZrange);
  register("ZRANGEBYSCORE", cmdZrangebyscore);
  register("ZRANK", cmdZrank);
  register("ZSCORE", cmdZscore);
  register("ZCARD", cmdZcard);
  register("ZREM", cmdZrem);
  register("PFADD", cmdPfadd);
  register("PFCOUNT", cmdPfcount);
  register("PFMERGE", cmdPfmerge);
  register("EXPIRE", cmdExpire);
  register("EXPIREAT", cmdExpireat);
  register("TTL", cmdTtl);
  register("PTTL", cmdPttl);
  register("PERSIST", cmdPersist);
  register("SELECT", cmdSelect);
  register("FLUSHDB", cmdFlushdb);
  register("FLUSHALL", cmdFlushall);
  register("DBSIZE", cmdDbsize);
  register("INFO", cmdInfo);
  register("KEYS", cmdKeys);
}

export function isMutatingCommand(name: string): boolean {
  switch (name.toUpperCase()) {
    case "SET":
    case "DEL":
    case "RENAME":
    case "INCR":
    case "DECR":
    case "INCRBY":
    case "DECRBY":
    case "APPEND":
    case "HSET":
    case "HDEL":
    case "LPUSH":
    case "RPUSH":
    case "LPOP":
    case "RPOP":
    case "SADD":
    case "SREM":
    case "ZADD":
    case "ZREM":
    case "PFADD":
    case "PFMERGE":
    case "EXPIRE":
    case "EXPIREAT":
    case "PERSIST":
    case "SELECT":
    case "FLUSHDB":
    case "FLUSHALL":
      return true;
    default:
      return false;
  }
}

export function commandName(parts: string[]): string {
  return parts[0]?.trim().toUpperCase() ?? "";
}

function cmdPing(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, simpleString("PONG")];
  }
  if (args.length === 1) {
    return [store, bulkString(args[0])];
  }
  return [store, errorValue("ERR wrong number of arguments for 'PING'")];
}

function cmdEcho(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'ECHO'")];
  }
  return [store, bulkString(args[0])];
}

function cmdSet(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'SET'")];
  }
  const key = args[0];
  const value = args[1];
  let expiresAt: number | null = null;
  let nx = false;
  let xx = false;
  for (let i = 2; i < args.length; ) {
    const option = args[i].toUpperCase();
    if (option === "EX" && i + 1 < args.length) {
      const seconds = parseInteger(args[i + 1]);
      if (seconds === null) {
        return [store, errorValue("ERR value is not an integer or out of range")];
      }
      expiresAt = expirationFromSeconds(seconds);
      i += 2;
    } else if (option === "PX" && i + 1 < args.length) {
      const millis = parseInteger(args[i + 1]);
      if (millis === null) {
        return [store, errorValue("ERR value is not an integer or out of range")];
      }
      expiresAt = expirationFromMillis(millis);
      i += 2;
    } else if (option === "NX") {
      nx = true;
      i += 1;
    } else if (option === "XX") {
      xx = true;
      i += 1;
    } else {
      return [store, errorValue("ERR syntax error")];
    }
  }
  if (nx && xx) {
    return [store, errorValue("ERR syntax error")];
  }
  const exists = store.get(key) !== undefined;
  if (nx && exists) {
    return [store, bulkString(null)];
  }
  if (xx && !exists) {
    return [store, bulkString(null)];
  }
  return [store.set(key, stringEntry(value, expiresAt)), simpleString("OK")];
}

function cmdGet(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'GET'")];
  }
  const entry = store.clone().get(args[0]);
  if (!entry) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "string") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, bulkString(entry.value.value)];
}

function cmdDel(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'DEL'")];
  }
  let removed = 0;
  let next = store;
  for (const key of args) {
    if (next.get(key) !== undefined) {
      removed += 1;
      next = next.delete(key);
    }
  }
  return [next, integer(removed)];
}

function cmdExists(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'EXISTS'")];
  }
  const count = args.reduce((total, key) => total + (store.get(key) !== undefined ? 1 : 0), 0);
  return [store, integer(count)];
}

function cmdType(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'TYPE'")];
  }
  return [store, simpleString(store.typeOf(args[0]) ?? "none")];
}

function cmdRename(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'RENAME'")];
  }
  const [src, dst] = args;
  const entry = store.get(src);
  if (!entry) {
    return [store, errorValue("ERR no such key")];
  }
  return [store.delete(src).set(dst, cloneEntry(entry)), simpleString("OK")];
}

function adjustInteger(store: Store, key: string, delta: number): CommandResult {
  const entry = store.get(key);
  let current = 0;
  let expiresAt: number | null = null;
  if (entry !== undefined) {
    if (entry.entryType !== "string") {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    const parsed = parseInteger(entry.value.value);
    if (parsed === null) {
      return [store, errorValue("ERR value is not an integer or out of range")];
    }
    current = parsed;
    expiresAt = entry.expiresAt;
  }

  const next = current + delta;
  if (!Number.isSafeInteger(next)) {
    return [store, errorValue("ERR increment or decrement would overflow")];
  }
  return [store.set(key, stringEntry(String(next), expiresAt)), integer(next)];
}

function cmdIncr(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'INCR'")];
  }
  return adjustInteger(store, args[0], 1);
}

function cmdDecr(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'DECR'")];
  }
  return adjustInteger(store, args[0], -1);
}

function cmdIncrby(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'INCRBY'")];
  }
  const delta = parseInteger(args[1]);
  if (delta === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  return adjustInteger(store, args[0], delta);
}

function cmdDecrby(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'DECRBY'")];
  }
  const delta = parseInteger(args[1]);
  if (delta === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  return adjustInteger(store, args[0], -delta);
}

function cmdAppend(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'APPEND'")];
  }
  const key = args[0];
  const value = args[1];
  const entry = store.get(key);
  let current = "";
  let expiresAt: number | null = null;
  if (entry !== undefined) {
    if (entry.entryType !== "string") {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    current = entry.value.value;
    expiresAt = entry.expiresAt;
  }
  const next = current + value;
  return [store.set(key, stringEntry(next, expiresAt)), integer(next.length)];
}

function cmdHset(store: Store, args: string[]): CommandResult {
  if (args.length < 3 || args.length % 2 === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'HSET'")];
  }
  const key = args[0];
  const entry = store.get(key);
  let map = entry?.entryType === "hash" ? entry.value.value.clone() : new HashMap<string, string>();
  const expiresAt = entry?.expiresAt ?? null;
  let added = 0;
  for (let i = 1; i < args.length; i += 2) {
    const field = args[i];
    const value = args[i + 1];
    if (!map.has(field)) {
      added += 1;
    }
    map = map.set(field, value);
  }
  return [store.set(key, hashEntry(map, expiresAt)), integer(added)];
}

function cmdHget(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'HGET'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, maybeBulk(entry.value.value.get(args[1]))];
}

function cmdHdel(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'HDEL'")];
  }
  const key = args[0];
  const entry = store.get(key);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  let map = entry.value.value.clone();
  let removed = 0;
  for (const field of args.slice(1)) {
    if (map.has(field)) {
      removed += 1;
      map = map.delete(field);
    }
  }
  const next = map.size === 0 ? store.delete(key) : store.set(key, hashEntry(map, entry.expiresAt));
  return [next, integer(removed)];
}

function cmdHgetall(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'HGETALL'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const entries = entry.value.value.entries().sort(([left], [right]) => left.localeCompare(right));
  return [
    store,
    array(entries.flatMap(([field, value]) => [bulkString(field), bulkString(value)])),
  ];
}

function cmdHlen(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'HLEN'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, integer(entry.value.value.size)];
}

function cmdHexists(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'HEXISTS'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, integer(entry.value.value.has(args[1]) ? 1 : 0)];
}

function cmdHkeys(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'HKEYS'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [
    store,
    array(
      entry.value.value
        .keys()
        .sort()
        .map((field) => bulkString(field)),
    ),
  ];
}

function cmdHvals(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'HVALS'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "hash") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [
    store,
    array(
      entry.value.value
        .entries()
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([, value]) => bulkString(value)),
    ),
  ];
}

function cmdLpush(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'LPUSH'")];
  }
  const key = args[0];
  const entry = store.get(key);
  const expiresAt = entry?.expiresAt ?? null;
  let list = entry?.entryType === "list" ? entry.value.value.slice() : [];
  for (const value of args.slice(1)) {
    list.unshift(value);
  }
  return [store.set(key, listEntry(list, expiresAt)), integer(list.length)];
}

function cmdRpush(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'RPUSH'")];
  }
  const key = args[0];
  const entry = store.get(key);
  const expiresAt = entry?.expiresAt ?? null;
  let list = entry?.entryType === "list" ? entry.value.value.slice() : [];
  for (const value of args.slice(1)) {
    list.push(value);
  }
  return [store.set(key, listEntry(list, expiresAt)), integer(list.length)];
}

function cmdLpop(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'LPOP'")];
  }
  const key = args[0];
  const entry = store.get(key);
  if (entry === undefined) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "list") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const list = entry.value.value.slice();
  const value = list.shift();
  const next = list.length === 0 ? store.delete(key) : store.set(key, listEntry(list, entry.expiresAt));
  return [next, maybeBulk(value)];
}

function cmdRpop(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'RPOP'")];
  }
  const key = args[0];
  const entry = store.get(key);
  if (entry === undefined) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "list") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const list = entry.value.value.slice();
  const value = list.pop();
  const next = list.length === 0 ? store.delete(key) : store.set(key, listEntry(list, entry.expiresAt));
  return [next, maybeBulk(value)];
}

function cmdLlen(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'LLEN'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "list") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, integer(entry.value.value.length)];
}

function cmdLrange(store: Store, args: string[]): CommandResult {
  if (args.length !== 3) {
    return [store, errorValue("ERR wrong number of arguments for 'LRANGE'")];
  }
  const start = parseInteger(args[1]);
  const end = parseInteger(args[2]);
  if (start === null || end === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "list") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const list = entry.value.value;
  const len = list.length;
  const normalizedStart = start < 0 ? len + start : start;
  const normalizedEnd = end < 0 ? len + end : end;
  if (
    normalizedStart < 0 ||
    normalizedEnd < 0 ||
    normalizedStart >= len ||
    normalizedStart > normalizedEnd
  ) {
    return [store, array([])];
  }
  return [
    store,
    array(list.slice(normalizedStart, normalizedEnd + 1).map((value) => bulkString(value))),
  ];
}

function cmdLindex(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'LINDEX'")];
  }
  const index = parseInteger(args[1]);
  if (index === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "list") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const list = entry.value.value;
  const normalized = index < 0 ? list.length + index : index;
  if (normalized < 0 || normalized >= list.length) {
    return [store, bulkString(null)];
  }
  return [store, bulkString(list[normalized])];
}

function cmdSadd(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'SADD'")];
  }
  const key = args[0];
  const entry = store.get(key);
  const expiresAt = entry?.expiresAt ?? null;
  let set = entry?.entryType === "set" ? entry.value.value.clone() : new HashSet<string>();
  let added = 0;
  for (const member of args.slice(1)) {
    if (!set.has(member)) {
      added += 1;
    }
    set = set.add(member);
  }
  return [store.set(key, setEntry(set, expiresAt)), integer(added)];
}

function cmdSrem(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'SREM'")];
  }
  const key = args[0];
  const entry = store.get(key);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "set") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  let set = entry.value.value.clone();
  let removed = 0;
  for (const member of args.slice(1)) {
    if (set.has(member)) {
      removed += 1;
      set = set.remove(member);
    }
  }
  const next = set.isEmpty() ? store.delete(key) : store.set(key, setEntry(set, entry.expiresAt));
  return [next, integer(removed)];
}

function cmdSismember(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'SISMEMBER'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "set") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, integer(entry.value.value.has(args[1]) ? 1 : 0)];
}

function cmdSmembers(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'SMEMBERS'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "set") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, array(entry.value.value.toList().sort().map((member) => bulkString(member)))];
}

function cmdScard(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'SCARD'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "set") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, integer(entry.value.value.size)];
}

function cmdSunion(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'SUNION'")];
  }
  let out = new HashSet<string>();
  for (const key of args) {
    const entry = store.get(key);
    if (entry === undefined) {
      continue;
    }
    if (entry.entryType !== "set") {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    out = out.union(entry.value.value);
  }
  return [store, array(out.toList().sort().map((member) => bulkString(member)))];
}

function cmdSinter(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'SINTER'")];
  }
  let out = new HashSet<string>();
  let first = true;
  for (const key of args) {
    const entry = store.get(key);
    const set = entry === undefined ? new HashSet<string>() : entry.entryType === "set" ? entry.value.value : null;
    if (set === null) {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    out = first ? set.clone() : out.intersection(set);
    first = false;
  }
  return [store, array(out.toList().sort().map((member) => bulkString(member)))];
}

function cmdSdiff(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'SDIFF'")];
  }
  let out = new HashSet<string>();
  let first = true;
  for (const key of args) {
    const entry = store.get(key);
    const set = entry === undefined ? new HashSet<string>() : entry.entryType === "set" ? entry.value.value : null;
    if (set === null) {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    out = first ? set.clone() : out.difference(set);
    first = false;
  }
  return [store, array(out.toList().sort().map((member) => bulkString(member)))];
}

function cmdZadd(store: Store, args: string[]): CommandResult {
  if (args.length < 3 || args.length % 2 === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'ZADD'")];
  }
  const key = args[0];
  const entry = store.get(key);
  const expiresAt = entry?.expiresAt ?? null;
  let zset = entry?.entryType === "zset" ? entry.value.value.clone() : SortedSet.new();
  let added = 0;
  for (let i = 1; i < args.length; i += 2) {
    const score = parseFloatStrict(args[i]);
    if (score === null) {
      return [store, errorValue("ERR value is not a valid float")];
    }
    if (zset.insert(score, args[i + 1])) {
      added += 1;
    }
  }
  return [store.set(key, zsetEntry(zset, expiresAt)), integer(added)];
}

function cmdZrange(store: Store, args: string[]): CommandResult {
  if (args.length < 3 || args.length > 4) {
    return [store, errorValue("ERR wrong number of arguments for 'ZRANGE'")];
  }
  const start = parseInteger(args[1]);
  const end = parseInteger(args[2]);
  if (start === null || end === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  const withScores = args[3]?.toUpperCase() === "WITHSCORES";
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "zset") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const values = entry.value.value.rangeByIndex(start, end);
  return [
    store,
    array(
      withScores
        ? values.flatMap(([member, score]) => [bulkString(member), bulkString(String(score))])
        : values.map(([member]) => bulkString(member)),
    ),
  ];
}

function cmdZrangebyscore(store: Store, args: string[]): CommandResult {
  if (args.length < 3 || args.length > 4) {
    return [store, errorValue("ERR wrong number of arguments for 'ZRANGEBYSCORE'")];
  }
  const min = parseFloatStrict(args[1]);
  const max = parseFloatStrict(args[2]);
  if (min === null || max === null) {
    return [store, errorValue("ERR value is not a valid float")];
  }
  const withScores = args[3]?.toUpperCase() === "WITHSCORES";
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, array([])];
  }
  if (entry.entryType !== "zset") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const values = entry.value.value.rangeByScore(min, max);
  return [
    store,
    array(
      withScores
        ? values.flatMap(([member, score]) => [bulkString(member), bulkString(String(score))])
        : values.map(([member]) => bulkString(member)),
    ),
  ];
}

function cmdZrank(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'ZRANK'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "zset") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const rank = entry.value.value.rank(args[1]);
  return rank === null ? [store, bulkString(null)] : [store, integer(rank)];
}

function cmdZscore(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'ZSCORE'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, bulkString(null)];
  }
  if (entry.entryType !== "zset") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const score = entry.value.value.score(args[1]);
  return score === undefined ? [store, bulkString(null)] : [store, bulkString(String(score))];
}

function cmdZcard(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'ZCARD'")];
  }
  const entry = store.get(args[0]);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "zset") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  return [store, integer(entry.value.value.len())];
}

function cmdZrem(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'ZREM'")];
  }
  const key = args[0];
  const entry = store.get(key);
  if (entry === undefined) {
    return [store, integer(0)];
  }
  if (entry.entryType !== "zset") {
    return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
  }
  const zset = entry.value.value.clone();
  let removed = 0;
  for (const member of args.slice(1)) {
    if (zset.remove(member)) {
      removed += 1;
    }
  }
  const next = zset.isEmpty() ? store.delete(key) : store.set(key, zsetEntry(zset, entry.expiresAt));
  return [next, integer(removed)];
}

function cmdPfadd(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'PFADD'")];
  }
  const key = args[0];
  const entry = store.get(key);
  const expiresAt = entry?.expiresAt ?? null;
  let hll = entry?.entryType === "hll" ? entry.value.value.clone() : new HyperLogLog();
  const before = hll.clone();
  for (const member of args.slice(1)) {
    hll.add(member);
  }
  const changed = before.equals(hll) ? 0 : 1;
  return [store.set(key, hllEntry(hll, expiresAt)), integer(changed)];
}

function cmdPfcount(store: Store, args: string[]): CommandResult {
  if (args.length === 0) {
    return [store, errorValue("ERR wrong number of arguments for 'PFCOUNT'")];
  }
  let hll = new HyperLogLog();
  let first = true;
  for (const key of args) {
    const entry = store.get(key);
    if (entry === undefined) {
      continue;
    }
    if (entry.entryType !== "hll") {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    hll = first ? entry.value.value.clone() : hll.merge(entry.value.value);
    first = false;
  }
  return [store, integer(hll.count())];
}

function cmdPfmerge(store: Store, args: string[]): CommandResult {
  if (args.length < 2) {
    return [store, errorValue("ERR wrong number of arguments for 'PFMERGE'")];
  }
  const dest = args[0];
  let merged = new HyperLogLog();
  let first = true;
  for (const key of args.slice(1)) {
    const entry = store.get(key);
    if (entry === undefined) {
      continue;
    }
    if (entry.entryType !== "hll") {
      return [store, errorValue("WRONGTYPE Operation against a key holding the wrong kind of value")];
    }
    merged = first ? entry.value.value.clone() : merged.merge(entry.value.value);
    first = false;
  }
  const expiresAt = store.get(dest)?.expiresAt ?? null;
  return [store.set(dest, hllEntry(merged, expiresAt)), simpleString("OK")];
}

function cmdExpire(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'EXPIRE'")];
  }
  const seconds = parseInteger(args[1]);
  if (seconds === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  return setExpiration(store, args[0], expirationFromSeconds(seconds));
}

function cmdExpireat(store: Store, args: string[]): CommandResult {
  if (args.length !== 2) {
    return [store, errorValue("ERR wrong number of arguments for 'EXPIREAT'")];
  }
  const timestamp = parseInteger(args[1]);
  if (timestamp === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  return setExpiration(store, args[0], expirationFromSeconds(timestamp - unixNowSeconds()));
}

function cmdTtl(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'TTL'")];
  }
  return ttlLike(store, args[0], false);
}

function cmdPttl(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'PTTL'")];
  }
  return ttlLike(store, args[0], true);
}

function cmdPersist(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'PERSIST'")];
  }
  const key = args[0];
  const entry = store.get(key);
  if (!entry || entry.expiresAt === null) {
    return [store, integer(0)];
  }
  const next = store.set(key, cloneEntry({ ...entry, expiresAt: null }));
  return [next, integer(1)];
}

function cmdSelect(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'SELECT'")];
  }
  const index = parseInteger(args[0]);
  if (index === null) {
    return [store, errorValue("ERR value is not an integer or out of range")];
  }
  if (index < 0 || index >= store.databases.length) {
    return [store, errorValue("ERR DB index out of range")];
  }
  return [store.select(index), simpleString("OK")];
}

function cmdFlushdb(store: Store, args: string[]): CommandResult {
  if (args.length !== 0) {
    return [store, errorValue("ERR wrong number of arguments for 'FLUSHDB'")];
  }
  return [store.flushdb(), simpleString("OK")];
}

function cmdFlushall(store: Store, args: string[]): CommandResult {
  if (args.length !== 0) {
    return [store, errorValue("ERR wrong number of arguments for 'FLUSHALL'")];
  }
  return [store.flushall(), simpleString("OK")];
}

function cmdDbsize(store: Store, args: string[]): CommandResult {
  if (args.length !== 0) {
    return [store, errorValue("ERR wrong number of arguments for 'DBSIZE'")];
  }
  return [store, integer(store.dbsize())];
}

function cmdInfo(store: Store, args: string[]): CommandResult {
  if (args.length !== 0) {
    return [store, errorValue("ERR wrong number of arguments for 'INFO'")];
  }
  const info = `# Server\r\nin_memory_data_store_version:0.1.0\r\nactive_db:${store.activeDb}\r\ndbsize:${store.dbsize()}\r\n`;
  return [store, bulkString(info)];
}

function cmdKeys(store: Store, args: string[]): CommandResult {
  if (args.length !== 1) {
    return [store, errorValue("ERR wrong number of arguments for 'KEYS'")];
  }
  return [store, array(store.keys(args[0]).map((key) => bulkString(key)))];
}

function ttlLike(store: Store, key: string, milliseconds: boolean): CommandResult {
  const entry = store.get(key);
  if (!entry) {
    return [store, integer(-2)];
  }
  if (entry.expiresAt === null) {
    return [store, integer(-1)];
  }
  const now = currentTimeMs();
  if (now >= entry.expiresAt) {
    return [store.delete(key), integer(-2)];
  }
  const remaining = entry.expiresAt - now;
  return [store, integer(milliseconds ? remaining : Math.floor(remaining / 1000))];
}

function setExpiration(store: Store, key: string, expiresAt: number): CommandResult {
  if (!store.exists(key)) {
    return [store, integer(0)];
  }
  if (expiresAt <= currentTimeMs()) {
    return [store.delete(key), integer(1)];
  }
  const entry = store.get(key);
  if (!entry) {
    return [store, integer(0)];
  }
  return [store.set(key, cloneEntry({ ...entry, expiresAt })), integer(1)];
}

function expirationFromSeconds(seconds: number): number {
  return currentTimeMs() + seconds * 1_000;
}

function expirationFromMillis(millis: number): number {
  return currentTimeMs() + millis;
}

function unixNowSeconds(): number {
  return Math.floor(currentTimeMs() / 1_000);
}

function parseInteger(value: string): number | null {
  if (!/^-?\d+$/.test(value)) {
    return null;
  }
  const parsed = Number.parseInt(value, 10);
  return Number.isSafeInteger(parsed) ? parsed : null;
}

function parseFloatStrict(value: string): number | null {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function maybeBulk(value: string | undefined): RespValue {
  return value === undefined ? bulkString(null) : bulkString(value);
}
