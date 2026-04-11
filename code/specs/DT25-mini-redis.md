# DT25 — In-Memory Data Store

## Overview

The in-memory data store is the culmination of the entire DT series. Every
data structure from DT00 through DT24 comes together in one working system: a
Redis-compatible in-memory database server that speaks the real Redis wire
protocol.

The first target is a **single-node** Redis baseline. That means the core
priority is to get the protocol, command semantics, and in-memory data
structures right using our own packages where possible. We can layer on the
distributed Redis features later once the single-node behavior is stable.

When you point `redis-cli` at the in-memory data store and type `SET foo bar`,
then `GET foo`, and get back `"bar"` - every layer we have built is doing its
job:

```
redis-cli ──► TCP connection (DT24) ──► RESP parser (DT23)
                                             │
                               ┌─────────────┴──────────────────────┐
                               │           Key Space                 │
                               │  RadixTree[key → Entry]  (DT14)    │
                               │  String  → bytes                   │
                               │  Hash    → HashMap       (DT18)    │
                               │  List    → linked list             │
                               │  Set     → HashSet       (DT19)    │
                               │  ZSet    → SkipList      (DT20)    │
                               │  HLL     → HyperLogLog   (DT21)    │
                               │  TTL     → MinHeap       (DT04)    │
                               └─────────────────────────────────────┘
                                             │
                               RESP encoder (DT23) ──► TCP (DT24) ──► redis-cli
```

This is not full Redis parity yet. The goal for this phase is a single-node
server that behaves like Redis for the common in-memory command surface and
RESP2 client/server flow. You should be able to connect normal Redis clients
and exercise the supported commands, but cluster, replication, Lua, and the
rest of Redis's distributed surface are still future work.

## Layer Position

```
DT00: graph           ─┐
DT01: directed-graph  ─┤
DT02: tree            ─┤
DT03: binary-tree     ─┤
DT04: heap            ─┼── core data structure primitives
DT05: segment-tree    ─┤   used by mini-redis for TTL (DT04),
DT06: fenwick-tree    ─┤   sorted sets (DT20), approximate
DT07: binary-search-tree│  counting (DT21), and membership
DT08: avl-tree        ─┤   filtering (DT22)
DT09: red-black-tree  ─┤
DT10: treap           ─┤
DT11: b-tree          ─┤
DT12: b-plus-tree     ─┤
DT13: trie            ─┤
DT14: radix-tree      ─┼── key space indexing (KEYS pattern)
DT15: suffix-tree     ─┤
DT16: rope            ─┘
DT17: hash-functions  ─┐
DT18: hash-map        ─┼── per-key value storage (Hash type)
DT19: hash-set        ─┼── Set type
DT20: skip-list       ─┼── ZSet (sorted set) type
DT21: hyperloglog     ─┼── HLL type (PFADD/PFCOUNT)
DT22: bloom-filter    ─┘   (optional: pre-filter key existence)
DT23: resp-protocol   ─── wire format encode/decode
DT24: tcp-server      ─── network I/O layer

DT25: in-memory data store ← [YOU ARE HERE]
                         Single-node Redis baseline built from the DT stack
```

**Depends on:** Everything. DT25 is the integration point for the entire series.

## Concepts

### Why Redis Matters

Redis is arguably the most widely deployed data structure server in the world.
Instagram, Twitter, GitHub, Stack Overflow, and millions of other services
use it as a cache, a message broker, a leaderboard engine, and a session store.

What makes Redis fast is not magic. It is fast because:
1. All data lives in RAM (no disk seeks during reads)
2. All commands execute in a single thread (no lock contention)
3. The event loop (DT24) handles thousands of connections simultaneously
4. RESP (DT23) is trivially fast to parse and encode

Mini-redis gives you the same foundation in a codebase you wrote yourself.

### Single-Node First

The implementation strategy is intentionally conservative:

- Keep the server single-node and in-memory first.
- Prefer our own packages for storage, protocol, and support layers.
- Make the supported command surface behave correctly before adding
  distributed or operational complexity.
- Avoid introducing a generic lexer/parser stack for the wire path unless a
  command family truly needs it.

That approach keeps the 10+ language ports tractable and makes the shared
architecture easier to reason about.

### What We Are Still Missing From Real Redis

This is the gap list we should keep visible while the single-node baseline is
being completed:

- **Transactions and optimistic coordination:** `MULTI`, `EXEC`, `WATCH`,
  `UNWATCH`, and the queuing semantics around them.
- **Blocking list operations:** `BLPOP`, `BRPOP`, `BLMOVE`, `BRPOPLPUSH`, and
  the related timeout/unblock behavior.
- **Pub/Sub:** `SUBSCRIBE`, `PSUBSCRIBE`, `PUBLISH`, and the push-message
  protocol flow.
- **Scripting:** `EVAL`, `EVALSHA`, script caching, and atomic script
  execution semantics.
- **Streams:** `XADD`, `XREAD`, consumer groups, pending-entry tracking, and
  stream trimming.
- **Key scanning and cursor iteration:** `SCAN`, `HSCAN`, `SSCAN`, `ZSCAN`
  and their incremental iteration rules.
- **Replication and failover:** primary/replica sync, `PSYNC`, Sentinel-style
  promotion, and reconnect logic.
- **Cluster mode:** hash-slot routing, `MOVED`/`ASK` redirects, resharding,
  and topology awareness.
- **Persistence beyond AOF replay:** RDB snapshots, AOF rewrite, crash-safe
  durability guarantees, and recovery semantics.
- **Memory management and eviction:** `maxmemory`, LRU/LFU/random eviction,
  and accurate memory accounting.
- **Protocol expansion:** RESP3, `HELLO`, client tracking, push replies,
  maps/sets/attributes, and related compatibility details.
- **Operational surface:** ACLs, modules, `CLIENT` administration, `CONFIG`,
  `INFO` parity, and keyspace notifications.
- **Command parity and edge cases:** all the option variants, subcommands,
  error messages, and ordering semantics that real Redis clients expect.

### Command Flow: Tracing a Request End-to-End

Let's trace `SET counter 0` followed by `INCR counter`:

```
Client sends:           *3\r\n$3\r\nSET\r\n$7\r\ncounter\r\n$1\r\n0\r\n
TCP server (DT24):      chunk of bytes arrives → append to conn.read_buffer
RESP decoder (DT23):    decode([b"SET", b"counter", b"0"]) — complete message!
Command dispatcher:     dispatch("SET", ["counter", "0"])
String handler:         store["counter"] = b"0"
RESP encoder (DT23):    encode(SimpleString("OK")) → b"+OK\r\n"
TCP server (DT24):      write(conn.fd, b"+OK\r\n")
Client receives:        +OK

──────────────────────────────────────────────────

Client sends:           *2\r\n$4\r\nINCR\r\n$7\r\ncounter\r\n
TCP server (DT24):      bytes arrive
RESP decoder (DT23):    decode([b"INCR", b"counter"])
Command dispatcher:     dispatch("INCR", ["counter"])
Integer handler:        val = get(store, "counter")   → b"0"
                        n = int(b"0")                 → 0
                        n += 1                        → 1
                        store["counter"] = b"1"
RESP encoder (DT23):    encode(Integer(1))           → b":1\r\n"
TCP server (DT24):      write(conn.fd, b":1\r\n")
Client receives:        :1
```

### The Key Space

The central data structure of mini-redis is the key space: a mapping from
string keys to typed entries. Every Redis key can hold one of six types:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Key Space                                   │
│                                                                     │
│  "counter"  → Entry { type: String,  value: b"42"              }   │
│  "users"    → Entry { type: Hash,    value: HashMap{...}       }   │
│  "queue"    → Entry { type: List,    value: LinkedList[...]    }   │
│  "online"   → Entry { type: Set,     value: HashSet{...}       }   │
│  "scores"   → Entry { type: ZSet,    value: SkipList[...]      }   │
│  "visitors" → Entry { type: HLL,     value: HyperLogLog{...}   }   │
│                                                                     │
│  Every entry also has:                                              │
│    expires_at: int | None   (Unix timestamp ms, or None = no TTL)  │
└─────────────────────────────────────────────────────────────────────┘
```

The key space itself is implemented with `HashMap[str, Entry]` (DT18) for
O(1) get/set/delete. A `RadixTree[str]` (DT14) is maintained in parallel to
support `KEYS pattern` prefix matching in O(p + k) where p is the pattern
prefix length and k is the number of matching keys.

### Command-to-Data-Structure Mapping

Every Redis command is simply a method call on one of the DT structures:

```
Command          Structure Used       Operation
─────────────────────────────────────────────────────────────────────
SET key val      HashMap (DT18)       store[key] = Entry(String, val)
GET key          HashMap (DT18)       store.get(key)?.value
DEL key [key…]   HashMap (DT18)       delete each key
EXISTS key       HashMap (DT18)       key in store
KEYS pattern     RadixTree (DT14)     prefix_match(pattern)
TYPE key         HashMap (DT18)       store.get(key)?.type
RENAME src dst   HashMap (DT18)       get + delete + set

INCR key         HashMap (DT18)       parse int, increment, set
DECR key         HashMap (DT18)       parse int, decrement, set
INCRBY key n     HashMap (DT18)       parse int, add n, set
DECRBY key n     HashMap (DT18)       parse int, subtract n, set
APPEND key s     HashMap (DT18)       get bytes, concatenate, set

HSET key f v     HashMap of HashMaps  store[key].hash[field] = value
HGET key f       HashMap of HashMaps  store[key].hash[field]
HDEL key f [f…]  HashMap of HashMaps  delete field(s) from hash
HGETALL key      HashMap of HashMaps  all (field, value) pairs
HLEN key         HashMap of HashMaps  len(store[key].hash)
HEXISTS key f    HashMap of HashMaps  field in store[key].hash

LPUSH key v[…]   LinkedList           prepend to list at key
RPUSH key v[…]   LinkedList           append to list at key
LPOP key         LinkedList           remove + return head
RPOP key         LinkedList           remove + return tail
LLEN key         LinkedList           len(list)
LRANGE key s e   LinkedList           slice [start, end]

SADD key m[…]    HashSet (DT19)       add members
SREM key m[…]    HashSet (DT19)       remove members
SISMEMBER key m  HashSet (DT19)       m in set
SMEMBERS key     HashSet (DT19)       all members
SCARD key        HashSet (DT19)       cardinality

ZADD key s m     SkipList (DT20)      insert (score, member)
ZRANGE key s e   SkipList (DT20)      members in index range [s, e]
ZRANGEBYSCORE    SkipList (DT20)      members with score in [min, max]
ZRANK key m      SkipList (DT20)      0-based rank of member
ZSCORE key m     SkipList (DT20)      score of member
ZCARD key        SkipList (DT20)      cardinality
ZREM key m[…]    SkipList (DT20)      remove members

PFADD key e[…]   HyperLogLog (DT21)   add elements
PFCOUNT key[…]   HyperLogLog (DT21)   estimate cardinality

EXPIRE key secs  MinHeap (DT04)       set expiry = now + secs
TTL key          MinHeap (DT04)       time until expiry
PERSIST key      MinHeap (DT04)       remove expiry
EXPIREAT key ts  MinHeap (DT04)       set absolute expiry timestamp

PING             —                    return PONG
SELECT db        —                    switch active database (0-15)
FLUSHDB          —                    delete all keys in current db
DBSIZE           —                    number of keys in current db
INFO             —                    server statistics string
─────────────────────────────────────────────────────────────────────
```

### TTL Implementation

Key expiration is one of Redis's most important features. The design uses
three mechanisms working together:

```
1. Passive expiry (on every read):
   GET key → check if key has expired → if yes, delete and return nil

2. Active expiry (background sweep):
   Every 100ms, pop the top of the TTL min-heap.
   If expiry_time <= now: delete the key, pop next.
   If expiry_time > now: push back, stop (heap is sorted).

3. Lazy expiry (belt and suspenders):
   Before any command touches a key, always check is_expired().
   This catches keys that the background sweep has not yet removed.
```

The min-heap stores `(expiry_timestamp_ms, key)` pairs. The heap property
guarantees that the key expiring soonest is always at the top — the
background sweep only needs to inspect the top element.

```
TTL heap example:
  [(1700000050000, "session:abc"),
   (1700000060000, "cache:home"),
   (1700000070000, "rate_limit:user_1")]

  Now = 1700000055000 ms (Unix ms timestamp):
  - Pop (1700000050000, "session:abc") → expired! Delete from store.
  - Pop (1700000060000, "cache:home")  → not yet. Push back. Stop.

EXPIRE key implementation:
  expiry_ms = current_time_ms() + seconds * 1000
  store[key].expires_at = expiry_ms
  heap.push((expiry_ms, key))

is_expired(key):
  entry = store.get(key)
  if entry is None: return True
  if entry.expires_at is None: return False
  return current_time_ms() >= entry.expires_at
```

### AOF Persistence

The Append-Only File (AOF) is a durability mechanism: every mutating command
is written to a log file before being applied. On startup, replay the log to
reconstruct state. If the server crashes mid-operation, at most the in-flight
command is lost.

```
AOF file contents (plain RESP encoding of each command):
  *3\r\n$3\r\nSET\r\n$4\r\nname\r\n$5\r\nalice\r\n
  *3\r\n$4\r\nHSET\r\n$4\r\nuser\r\n$4\r\nname\r\n
  *2\r\n$4\r\nINCR\r\n$7\r\ncounter\r\n
  *3\r\n$6\r\nEXPIRE\r\n$7\r\ncounter\r\n$2\r\n60\r\n

AOF write path:
  1. Receive and parse command
  2. Apply command to in-memory store
  3. If command mutated state: encode command as RESP, append to AOF file
  4. Send response to client

AOF replay (startup):
  for each RESP-encoded command in aof_file:
      decode command
      apply to empty store (skip TTL restoration — re-EXPIRE if needed)

Fsync policies (tradeoff: durability vs performance):
  always:    fsync() after every write — safest, slowest
  everysec:  fsync() once per second — good balance (default)
  no:        let OS decide — fastest, least durable
```

Note: EXPIRE timestamps stored in AOF use absolute epoch time so that TTLs
are correct after a replay even if time has passed.

### Compatibility: Testing with redis-cli

The mini-redis server binds to port 6380 (not 6380 to avoid conflicting with
a real Redis server running on 6379). Any standard Redis client connects:

```
# Connect to mini-redis on port 6380
redis-cli -p 6380

# Basic string commands
127.0.0.1:6380> SET name alice
OK
127.0.0.1:6380> GET name
"alice"
127.0.0.1:6380> INCR counter
(integer) 1
127.0.0.1:6380> INCR counter
(integer) 2

# Hash commands
127.0.0.1:6380> HSET user name bob age 30
(integer) 2
127.0.0.1:6380> HGETALL user
1) "name"
2) "bob"
3) "age"
4) "30"

# Sorted set
127.0.0.1:6380> ZADD scores 100 alice 200 bob 150 carol
(integer) 3
127.0.0.1:6380> ZRANGE scores 0 -1 WITHSCORES
1) "alice"
2) "100"
3) "carol"
4) "150"
5) "bob"
6) "200"

# HyperLogLog
127.0.0.1:6380> PFADD pageviews user1 user2 user3
(integer) 1
127.0.0.1:6380> PFCOUNT pageviews
(integer) 3

# TTL
127.0.0.1:6380> SET temp "ephemeral"
OK
127.0.0.1:6380> EXPIRE temp 10
(integer) 1
127.0.0.1:6380> TTL temp
(integer) 9
```

## Representation

```
Store (the in-memory database):
  keyspace: HashMap[str, Entry]      # O(1) access by key
  key_index: RadixTree[str, None]    # O(prefix) for KEYS pattern
  ttl_heap: MinHeap[(int, str)]      # (expiry_ms, key), for active expiry
  databases: list[Store]             # SELECT 0-15; this is dbs[active_db]
  active_db: int                     # current database index

Entry (one value stored at a key):
  type: EntryType                    # String | Hash | List | Set | ZSet | HLL
  value: bytes | HashMap | LinkedList | HashSet | SkipList | HyperLogLog
  expires_at: int | None             # Unix milliseconds, None = no TTL

EntryType (enum):
  String = "string"
  Hash   = "hash"
  List   = "list"
  Set    = "set"
  ZSet   = "zset"
  HLL    = "hll"

MiniRedisServer:
  store: Store                       # the live database
  tcp_server: TcpServer              # DT24 — handles I/O
  aof_file: FileHandle | None        # append-only log (None = no persistence)
  aof_fsync: str                     # "always" | "everysec" | "no"
```

## Algorithms (Pure Functions)

Every command handler is a pure function: `command(store, args) → (new_store, response)`.
The server is just a loop that applies these functions.

### dispatch(store, command_parts) → (Store, RespValue)

```
dispatch(store, parts):
    # parts is a list of bytes decoded from RESP, e.g. [b"SET", b"foo", b"bar"]
    if not parts:
        return store, RespError("ERR empty command")
    cmd = parts[0].upper().decode("ascii")
    args = parts[1:]

    handlers = {
        "PING":     cmd_ping,
        "SET":      cmd_set,
        "GET":      cmd_get,
        "DEL":      cmd_del,
        "EXISTS":   cmd_exists,
        "INCR":     cmd_incr,
        ...
    }
    handler = handlers.get(cmd)
    if handler is None:
        return store, RespError(f"ERR unknown command '{cmd}'")
    # Always check TTL before handling — lazy expiry
    store = expire_lazy(store, args[0] if args else None)
    return handler(store, args)
```

### cmd_set(store, args) → (Store, RespValue)

```
cmd_set(store, args):
    # SET key value [EX seconds] [PX milliseconds] [NX] [XX]
    if len(args) < 2:
        return store, RespError("ERR wrong number of arguments for 'SET'")
    key = args[0].decode("utf-8")
    value = args[1]   # keep as bytes

    # Parse options: EX, PX, NX, XX
    expires_at = None
    i = 2
    nx, xx = False, False
    while i < len(args):
        opt = args[i].upper()
        if opt == b"EX" and i + 1 < len(args):
            expires_at = current_time_ms() + int(args[i+1]) * 1000
            i += 2
        elif opt == b"PX" and i + 1 < len(args):
            expires_at = current_time_ms() + int(args[i+1])
            i += 2
        elif opt == b"NX":
            nx = True; i += 1
        elif opt == b"XX":
            xx = True; i += 1
        else:
            return store, RespError("ERR syntax error")

    existing = store.keyspace.get(key)
    if nx and existing is not None: return store, None  # NX: only if not exists
    if xx and existing is None:     return store, None  # XX: only if exists

    entry = Entry(EntryType.String, value, expires_at)
    new_keyspace = store.keyspace.set(key, entry)      # DT18 put
    new_index = store.key_index.insert(key)            # DT14 insert
    if expires_at:
        new_heap = store.ttl_heap.push((expires_at, key))  # DT04 push
    else:
        new_heap = store.ttl_heap
    new_store = Store(new_keyspace, new_index, new_heap, ...)
    return new_store, SimpleString("OK")
```

### cmd_get(store, args) → (Store, RespValue)

```
cmd_get(store, args):
    if len(args) != 1:
        return store, RespError("ERR wrong number of arguments for 'GET'")
    key = args[0].decode("utf-8")
    entry = store.keyspace.get(key)
    if entry is None or is_expired(store, key):
        return expire_key(store, key), None  # return null bulk string
    if entry.type != EntryType.String:
        return store, RespError("WRONGTYPE Operation against a key holding the wrong kind of value")
    return store, entry.value   # bytes → encodes as BulkString
```

### cmd_incr(store, args) → (Store, RespValue)

```
cmd_incr(store, args):
    key = args[0].decode("utf-8")
    entry = store.keyspace.get(key)
    if entry is None:
        current = 0           # INCR on missing key treats it as 0
    elif entry.type != EntryType.String:
        return store, RespError("WRONGTYPE ...")
    else:
        try:
            current = int(entry.value)
        except ValueError:
            return store, RespError("ERR value is not an integer")
    new_val = current + 1
    entry = Entry(EntryType.String, str(new_val).encode(), entry.expires_at if entry else None)
    new_store = store.set_key(key, entry)
    return new_store, new_val   # Integer reply
```

### cmd_zadd(store, args) → (Store, RespValue)

```
cmd_zadd(store, args):
    # ZADD key [NX|XX] [GT|LT] [CH] [INCR] score member [score member …]
    key = args[0].decode("utf-8")
    entry = store.keyspace.get(key)
    if entry is not None and entry.type != EntryType.ZSet:
        return store, RespError("WRONGTYPE ...")
    skiplist = entry.value if entry else SkipList()    # DT20
    added = 0
    i = 1
    while i + 1 < len(args):
        score = float(args[i])
        member = args[i+1].decode("utf-8")
        if not skiplist.contains(member):
            added += 1
        skiplist = skiplist.insert(score, member)     # DT20 insert
        i += 2
    entry = Entry(EntryType.ZSet, skiplist, entry.expires_at if entry else None)
    new_store = store.set_key(key, entry)
    return new_store, added    # Integer: number of new members added
```

### expire_lazy(store, key) → Store

```
expire_lazy(store, key):
    if key is None: return store
    entry = store.keyspace.get(key)
    if entry is None: return store
    if entry.expires_at and current_time_ms() >= entry.expires_at:
        return expire_key(store, key)
    return store

expire_key(store, key):
    new_keyspace = store.keyspace.delete(key)
    new_index = store.key_index.delete(key)
    return Store(new_keyspace, new_index, store.ttl_heap, ...)
```

### active_expire(store) → Store

Run periodically (e.g., every 100ms in a background goroutine/task):

```
active_expire(store):
    now = current_time_ms()
    while store.ttl_heap.size() > 0:
        (expiry_ms, key) = store.ttl_heap.peek()   # DT04 peek min
        if expiry_ms > now:
            break                                  # nothing more to expire
        store.ttl_heap = store.ttl_heap.pop()      # DT04 pop min
        # Verify: key might have been re-SET with a new TTL since we pushed
        entry = store.keyspace.get(key)
        if entry and entry.expires_at == expiry_ms:
            store = expire_key(store, key)
    return store
```

## Public API

```python
from dataclasses import dataclass, field
from typing import Any
from enum import Enum


class EntryType(Enum):
    String = "string"
    Hash   = "hash"
    List   = "list"
    Set    = "set"
    ZSet   = "zset"
    HLL    = "hll"


@dataclass(frozen=True)
class Entry:
    type: EntryType
    value: Any           # bytes | HashMap | list | HashSet | SkipList | HyperLogLog
    expires_at: int | None = None   # Unix milliseconds, or None for no TTL


@dataclass(frozen=True)
class Store:
    """
    Immutable snapshot of the database state.
    Command handlers return a new Store — never mutate in place.
    (In a production system you'd use mutable data for performance,
    but immutable makes testing and reasoning far easier.)
    """
    keyspace: "HashMap"          # DT18: str → Entry
    key_index: "RadixTree"       # DT14: for KEYS pattern
    ttl_heap: "MinHeap"          # DT04: (expiry_ms, key)
    active_db: int = 0

    @staticmethod
    def empty() -> "Store":
        """Create an empty database."""

    def get(self, key: str) -> Entry | None:
        """Look up a key. Returns None if missing or expired."""

    def set(self, key: str, entry: Entry) -> "Store":
        """Set a key to an entry. Returns a new Store."""

    def delete(self, key: str) -> "Store":
        """Remove a key. Returns a new Store."""

    def exists(self, key: str) -> bool:
        """True if key exists and is not expired."""

    def keys(self, pattern: str) -> list[str]:
        """
        Return all keys matching the glob pattern.
        Supports: * (any chars), ? (one char), [abc] (char class).
        Uses DT14 RadixTree for prefix optimisation where possible.
        """

    def type_of(self, key: str) -> str | None:
        """Return the type string ("string", "hash", etc.) or None."""


@dataclass
class MiniRedis:
    """
    A Redis-compatible in-memory server.

    Usage:
        server = MiniRedis(port=6380)
        server.start()   # blocks — run with Ctrl+C to stop
    """
    port: int = 6380
    host: str = "127.0.0.1"
    aof_path: str | None = None   # None = no persistence
    aof_fsync: str = "everysec"   # "always" | "everysec" | "no"

    def start(self) -> None:
        """Bind, listen, run the event loop. Blocks."""

    def stop(self) -> None:
        """Graceful shutdown: finish in-flight requests, then close."""

    def execute(self, command: list[bytes]) -> Any:
        """
        Execute one command against the store.
        Useful for testing without a network connection.
        Returns the response value (not encoded as RESP).
        """

    @property
    def store(self) -> Store:
        """Direct access to current state. For testing and inspection."""
```

## Composition Model

Mini-redis composes all previous layers. The key architectural decision:
**the command layer is pure functions, the I/O layer is effectful**.

```
Pure core (testable in isolation):
  dispatch(store, cmd)     → (store, response)
  cmd_set(store, args)     → (store, response)
  cmd_get(store, args)     → (store, response)
  ...
  active_expire(store)     → store
  encode(response)         → bytes          [DT23]
  decode(buffer)           → (cmd, n)       [DT23]

Effectful shell (the I/O layer):
  TcpServer.start()        → bind/listen/epoll loop  [DT24]
  handler(conn, data)      → calls pure core, writes result
  aof_append(cmd)          → file write
  background_expire()      → periodic active_expire()
```

### Python

```python
class MiniRedis:
    def __init__(self, port=6380, aof_path=None):
        self.store = Store.empty()
        self.aof_path = aof_path
        self._server = TcpServer("127.0.0.1", port, self._handle)

    def _handle(self, conn: Connection, data: bytes) -> None:
        # This is the bridge between DT24 (raw bytes) and DT23+DT25 (commands)
        conn.read_buffer = getattr(conn, "read_buffer", b"") + data
        while True:
            cmd_parts, consumed = decode(conn.read_buffer)   # DT23
            if consumed == 0:
                break
            conn.read_buffer = conn.read_buffer[consumed:]
            self.store, response = dispatch(self.store, cmd_parts)
            if self.aof_path and is_mutating(cmd_parts):
                aof_append(self.aof_path, cmd_parts)   # DT23 encode
            conn.send(encode(response))                # DT23 encode

    def start(self):
        start_background_expire(self)   # runs active_expire() every 100ms
        self._server.start()
```

### Go

```go
type MiniRedis struct {
    mu     sync.RWMutex
    store  *Store
    server *TcpServer
    aof    *os.File
}

func (m *MiniRedis) handle(conn *Connection, data []byte) {
    conn.buf = append(conn.buf, data...)
    for {
        parts, consumed := resp.Decode(conn.buf)  // DT23
        if consumed == 0 { break }
        conn.buf = conn.buf[consumed:]

        m.mu.Lock()
        newStore, response := dispatch(m.store, parts)  // pure function
        m.store = newStore
        m.mu.Unlock()

        if m.aof != nil && isMutating(parts) {
            m.aof.Write(resp.EncodeArray(parts))  // DT23
        }
        conn.Send(resp.Encode(response))           // DT23
    }
}
```

### Rust

```rust
pub struct MiniRedis {
    store: Arc<Mutex<Store>>,
    server: TcpServer,
}

impl MiniRedis {
    pub async fn handle(&self, conn: Arc<Connection>, data: Vec<u8>) {
        let mut buf = conn.read_buffer.lock().await;
        buf.extend_from_slice(&data);

        loop {
            let (parts, consumed) = resp::decode(&buf);  // DT23
            if consumed == 0 { break; }
            *buf = buf[consumed..].to_vec();

            let response = {
                let mut store = self.store.lock().await;
                let (new_store, resp) = dispatch(&store, &parts);
                *store = new_store;
                resp
            };
            conn.send(resp::encode(&response)).await;  // DT23
        }
    }
}
```

### Elixir

```elixir
defmodule MiniRedis do
  use GenServer

  def start_link(port: port) do
    GenServer.start_link(__MODULE__, %{store: Store.empty()}, name: __MODULE__)
  end

  def init(state) do
    # Start TCP acceptor — each connection gets its own process (DT24)
    Task.start(fn -> TcpServer.start(6380, &handle_connection/2) end)
    {:ok, state}
  end

  defp handle_connection(socket, _addr) do
    Stream.repeatedly(fn -> :gen_tcp.recv(socket, 0) end)
    |> Enum.each(fn {:ok, data} ->
        {parts, _} = Resp.decode(data)   # DT23
        response = GenServer.call(__MODULE__, {:command, parts})
        :gen_tcp.send(socket, Resp.encode(response))   # DT23
    end)
  end

  def handle_call({:command, parts}, _from, %{store: store} = state) do
    {new_store, response} = Commands.dispatch(store, parts)
    {:reply, response, %{state | store: new_store}}
  end
end
```

## Test Strategy

### Unit Tests: Each Command in Isolation

```python
def test_set_get():
    """SET then GET returns the value."""
    store = Store.empty()
    store, resp = cmd_set(store, [b"key", b"value"])
    assert resp == "OK"
    store, resp = cmd_get(store, [b"key"])
    assert resp == b"value"

def test_get_missing():
    """GET of missing key returns None (null bulk string)."""
    store = Store.empty()
    _, resp = cmd_get(store, [b"missing"])
    assert resp is None

def test_incr_starts_at_zero():
    """INCR on a missing key treats it as 0."""
    store = Store.empty()
    store, resp = cmd_incr(store, [b"counter"])
    assert resp == 1
    store, resp = cmd_incr(store, [b"counter"])
    assert resp == 2

def test_incr_on_non_integer():
    """INCR on a non-integer value returns an error."""
    store = Store.empty()
    store, _ = cmd_set(store, [b"k", b"not_a_number"])
    _, resp = cmd_incr(store, [b"k"])
    assert isinstance(resp, RespError)
    assert "not an integer" in resp.message

def test_wrongtype_error():
    """Using a string command on a list key returns WRONGTYPE."""
    store = Store.empty()
    store, _ = cmd_lpush(store, [b"mylist", b"val"])
    _, resp = cmd_get(store, [b"mylist"])
    assert isinstance(resp, RespError)
    assert resp.error_type == "WRONGTYPE"
```

### TTL Tests

```python
def test_expire_and_ttl():
    """EXPIRE sets TTL, TTL returns remaining time."""
    store = Store.empty()
    store, _ = cmd_set(store, [b"k", b"v"])
    store, resp = cmd_expire(store, [b"k", b"60"])
    assert resp == 1    # 1 = success
    _, ttl = cmd_ttl(store, [b"k"])
    assert 58 <= ttl <= 60   # allow for a second of test runtime

def test_expired_key_returns_nil():
    """A key past its TTL returns nil, not stale data."""
    store = Store.empty()
    store, _ = cmd_set(store, [b"k", b"v"])
    # Manually inject an already-expired TTL
    entry = store.keyspace.get("k")
    expired_entry = Entry(entry.type, entry.value, expires_at=1)  # epoch 1 ms
    store = store.set("k", expired_entry)
    _, resp = cmd_get(store, [b"k"])
    assert resp is None

def test_persist_removes_ttl():
    store = Store.empty()
    store, _ = cmd_set(store, [b"k", b"v"])
    store, _ = cmd_expire(store, [b"k", b"60"])
    store, resp = cmd_persist(store, [b"k"])
    assert resp == 1
    _, ttl = cmd_ttl(store, [b"k"])
    assert ttl == -1   # -1 = no TTL
```

### Integration Tests: Full Stack (TCP + RESP + Commands)

```python
def test_redis_cli_compatible(mini_redis_server):
    """Test with a real socket connection, just like redis-cli."""
    import socket
    from resp import encode, decode

    with socket.create_connection(("127.0.0.1", 6380)) as s:
        # SET
        s.sendall(encode(["SET", "foo", "bar"]))
        resp_bytes = recv_all(s)
        value, _ = decode(resp_bytes)
        assert value == "OK"

        # GET
        s.sendall(encode(["GET", "foo"]))
        resp_bytes = recv_all(s)
        value, _ = decode(resp_bytes)
        assert value == b"bar"

        # DEL
        s.sendall(encode(["DEL", "foo"]))
        resp_bytes = recv_all(s)
        value, _ = decode(resp_bytes)
        assert value == 1   # 1 key deleted

        # GET after DEL
        s.sendall(encode(["GET", "foo"]))
        resp_bytes = recv_all(s)
        value, _ = decode(resp_bytes)
        assert value is None
```

### Data Type Correctness

```python
def test_sorted_set_ordering():
    """ZRANGE returns members sorted by score, lowest to highest."""
    store = Store.empty()
    store, _ = cmd_zadd(store, [b"scores",
        b"300", b"charlie",
        b"100", b"alice",
        b"200", b"bob"])
    _, resp = cmd_zrange(store, [b"scores", b"0", b"-1"])
    assert resp == [b"alice", b"bob", b"charlie"]

def test_hyperloglog_approximate_count():
    """PFCOUNT returns approximately correct cardinality."""
    store = Store.empty()
    elements = [str(i).encode() for i in range(10000)]
    store, _ = cmd_pfadd(store, [b"hll"] + elements)
    _, count = cmd_pfcount(store, [b"hll"])
    # HyperLogLog is approximate (DT21) — accept ±2% error
    assert 9800 <= count <= 10200

def test_set_operations():
    """SADD/SMEMBERS/SISMEMBER work correctly."""
    store = Store.empty()
    store, added = cmd_sadd(store, [b"s", b"a", b"b", b"c"])
    assert added == 3
    store, added2 = cmd_sadd(store, [b"s", b"b", b"d"])
    assert added2 == 1   # only "d" is new
    _, members = cmd_smembers(store, [b"s"])
    assert set(members) == {b"a", b"b", b"c", b"d"}
    _, is_member = cmd_sismember(store, [b"s", b"a"])
    assert is_member == 1
    _, is_member = cmd_sismember(store, [b"s", b"z"])
    assert is_member == 0
```

### AOF Persistence

```python
def test_aof_replay(tmp_path):
    """Commands survive a server restart via AOF replay."""
    aof_file = str(tmp_path / "appendonly.aof")

    # First server: write data
    server1 = MiniRedis(port=6381, aof_path=aof_file)
    server1.execute([b"SET", b"persistent", b"yes"])
    server1.execute([b"INCR", b"count"])
    server1.execute([b"INCR", b"count"])
    server1.stop()

    # Second server: replay AOF, check data
    server2 = MiniRedis(port=6382, aof_path=aof_file)
    assert server2.execute([b"GET", b"persistent"]) == b"yes"
    assert server2.execute([b"GET", b"count"]) == b"2"
```

## Implementation Phases

Build mini-redis in this order. Each phase is a working, testable milestone.

```
Phase 1: Skeleton — TCP + RESP + PING
  ✓ TcpServer (DT24) accepting connections
  ✓ RESP decoder/encoder (DT23) parsing commands
  ✓ Dispatch table with just PING → "+PONG\r\n"
  Test: redis-cli -p 6380 PING  →  PONG

Phase 2: String Commands
  ✓ Store: HashMap (DT18) of string entries
  ✓ RadixTree (DT14) for key index
  ✓ SET, GET, DEL, EXISTS, KEYS, TYPE
  Test: SET/GET round trip, DEL removes, KEYS returns matching

Phase 3: Integer Commands
  ✓ INCR, DECR, INCRBY, DECRBY, APPEND
  ✓ Integer overflow detection
  Test: INCR on missing key, INCR/DECR, error on non-integer value

Phase 4: Hash Commands
  ✓ HashMap of HashMap (nested DT18)
  ✓ HSET, HGET, HDEL, HGETALL, HLEN, HEXISTS, HKEYS, HVALS
  Test: HSET multiple fields, HGETALL returns all pairs

Phase 5: List Commands
  ✓ Doubly-linked list per key
  ✓ LPUSH, RPUSH, LPOP, RPOP, LLEN, LRANGE, LINDEX
  Test: queue/stack semantics, LRANGE slicing

Phase 6: Set Commands
  ✓ HashSet (DT19) per key
  ✓ SADD, SREM, SISMEMBER, SMEMBERS, SCARD
  ✓ Bonus: SUNION, SINTER, SDIFF (set algebra)
  Test: membership, cardinality, set operations

Phase 7: Sorted Set Commands
  ✓ SkipList (DT20) per key: stores (score, member) pairs
  ✓ ZADD, ZRANGE, ZRANGEBYSCORE, ZRANK, ZSCORE, ZCARD, ZREM
  Test: ordering by score, rank, range queries

Phase 8: HyperLogLog
  ✓ HyperLogLog (DT21) per key
  ✓ PFADD, PFCOUNT, PFMERGE
  Test: approximate count within 2%, PFMERGE combines estimates

Phase 9: TTL
  ✓ MinHeap (DT04) for active expiry
  ✓ Lazy expiry on every read
  ✓ EXPIRE, EXPIREAT, TTL, PTTL, PERSIST
  Test: key disappears after TTL, PERSIST re-enables indefinite life

Phase 10: AOF Persistence
  ✓ Append mutating commands to log file (RESP encoded)
  ✓ Replay on startup
  ✓ Configurable fsync policy
  Test: data survives simulated restart, AOF is valid RESP

Phase 11: Single-Node Admin Commands
  ✓ SELECT (switch databases 0-15)
  ✓ FLUSHDB, FLUSHALL
  ✓ DBSIZE
  ✓ INFO (server stats string)
  Future work: MULTI/EXEC/WATCH and the rest of the distributed surface
```

## Future Extensions

**RDB Snapshots:** A periodic full serialisation of the store to a binary
file. Faster to load than replaying a long AOF. Redis uses both: RDB for
recovery and AOF for durability.

**Transactions:** `MULTI`, `EXEC`, `WATCH`, and `UNWATCH`. Queue commands
inside a transaction, abort when watches are invalidated, and preserve Redis'
single-threaded atomic execution model.

**Pub/Sub:** `SUBSCRIBE channel` and `PUBLISH channel msg`. The server
maintains a map of `channel → set of subscribers`. On PUBLISH, iterate the
subscriber set and send a Push message (RESP3) or a 3-element Array (RESP2).

**Lua Scripting:** `EVAL script numkeys key [key…] arg [arg…]`. Embed a
Lua interpreter (LuaJIT or WASM-compiled Lua). Scripts run atomically —
the event loop is blocked for their duration, just like any other command.
DT16 (rope-like string operations) become useful for the scripting layer.

**Cluster Mode:** Shard keyspace across N nodes using consistent hashing.
Each node owns a subset of hash slots (0-16383). MOVED and ASK redirects
steer clients to the correct node. Requires gossip protocol for topology.

**Replica Replication:** One primary, N replicas. Primary propagates AOF
commands to replicas. Replicas apply commands in order. On primary failure,
replicas elect a new primary (Raft or Sentinel).

**Memory Limits and Eviction Policies:** When memory usage exceeds a
`maxmemory` limit, evict keys using LRU, LFU, or random sampling. The
Bloom filter (DT22) helps here: track access frequency to implement LFU
without per-key counters.

**RESP3 Upgrade:** Add `HELLO 3` negotiation. Return Maps (instead of flat
Arrays) for HGETALL, CONFIG GET, etc. Return proper null (instead of null
bulk string) for missing keys. This is backward compatible — clients that
don't send HELLO continue to receive RESP2.

**Keyspace Notifications:** Publish events to `__keyevent@<db>__:<event>`
channels when keys are modified, deleted, or expired. Useful for cache
invalidation and event-driven architectures.
