# hash-map (C)

A pure ISO **C17** hash map built from scratch — a faithful port of the Rust
`hash-map` crate (DT18). Keys and values are arbitrary byte strings.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/). No compiler
extensions, no third-party dependencies.

## What's inside

Two collision-resolution strategies and four hash functions, exactly as in the
Rust crate:

| | |
| --- | --- |
| **Chaining** | each bucket is a linked list; resizes when load factor > 1.0 |
| **Open addressing** | one slot array, linear probing, tombstones on delete; resizes above 0.75 |
| **Hashes** | SipHash-2-4 (default), FNV-1a-32, MurmurHash3-32, djb2 |

```c
hashmap *m = hashmap_new(16, HASHMAP_CHAINING, HASHMAP_SIPHASH24);
hashmap_set(m, "user", 4, "alice", 5);      /* key bytes, value bytes */
const void *val; size_t len;
if (hashmap_get(m, "user", 4, &val, &len)) { /* val/len borrow from the map */ }
hashmap_delete(m, "user", 4);
hashmap_free(m);
```

The map copies and owns the key/value bytes you pass in; returned value pointers
are borrowed and valid until the next mutation.

## Implementation notes

- **Four hash functions, self-contained.** SipHash-2-4, MurmurHash3-32,
  FNV-1a-32, and djb2 are all reproduced inline (the same constants and round
  functions as the `coding-adventures` hash-functions crate).
- **Byte keys.** Rust hashes each key's `Debug` string; this port hashes the raw
  key bytes. The map is self-consistent (set and get hash identically), so
  behaviour is faithful — only which bucket a key lands in differs, which a hash
  map never exposes.
- **Allocation-free resize.** Growing the table relinks the existing chaining
  nodes / moves the open-addressing slots into the new array; it never
  re-duplicates keys or values, and it degrades gracefully (stays at the old
  size) if the one new-array allocation fails.
- **Overflow-safe.** The doubling step is skipped rather than wrapped when
  `capacity > SIZE_MAX/2`.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

The tests run the full behavioural suite against **all eight** (strategy × hash)
combinations, plus resize stress (500 chaining / 300 open-addressing inserts) and
empty-key/empty-value edge cases.

## Where it fits

Part of the `code/packages/c` pure-ISO C set — the foundational associative
container. The `hash-set` port builds directly on it.
