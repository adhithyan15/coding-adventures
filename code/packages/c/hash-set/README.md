# hash-set (C)

A pure ISO **C17** hash set for byte-string elements — a faithful port of the
Rust `hash-set` crate (DT19). Exactly like the Rust crate, it is a thin wrapper
over the sibling [`hash-map`](../hash-map/) package (`HashSet<T>` = `HashMap<T,
()>`), so all the hashing and collision handling lives there.

It compiles clean under **GCC, Clang, and MSVC** with
`-std=c17 -pedantic-errors -Wall -Wextra -Werror` (and `/std:c17 /permissive-
/W4 /WX` on MSVC), via the shared [`iso-harness`](../iso-harness/).

## API

```c
#include "hash_set.h"

hashset *s = hashset_new();          /* defaults: cap 16, chaining, SipHash-2-4 */
hashset_add(s, "apple", 5);
if (hashset_contains(s, "apple", 5)) { /* member */ }
hashset_remove(s, "apple", 5);
hashset_free(s);
```

| Group | Functions |
| --- | --- |
| Membership | `hashset_add`, `hashset_remove`, `hashset_contains`, `hashset_size`, `hashset_is_empty` |
| Enumeration | `hashset_for_each` |
| Algebra (return a new set) | `hashset_union`, `hashset_intersection`, `hashset_difference`, `hashset_symmetric_difference` |
| Relations | `hashset_is_subset`, `hashset_is_superset`, `hashset_is_disjoint`, `hashset_equals` |

The set-algebra functions allocate and return a fresh set (NULL on allocation
failure) that the caller frees. The result inherits the collision strategy and
hash function of the first argument, matching the Rust crate.

## Depends on `hash-map`

This is the campaign's first C package with a sibling-package dependency: its
`BUILD` declares `# build-tool: deps=c/hash-map`, and `tools/run.sh` adds
`../hash-map/include` to the include path and compiles `../hash-map/src/hash_map.c`
into the test binary. Set algebra is implemented on the map's `hashmap_for_each`
enumeration.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests mirror the Rust suite: membership, duplicate handling, the four set-algebra
operations, and the subset/superset/disjoint/equals relations.
