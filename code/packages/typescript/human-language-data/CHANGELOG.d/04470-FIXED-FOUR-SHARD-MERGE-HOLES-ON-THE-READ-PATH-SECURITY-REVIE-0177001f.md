### Fixed - four shard-merge holes on the READ path (security review)

All four sit at a trust boundary the write-side checks do not cover: shard
*names* and `_meta.json` *contents* come off disk, where a pull request put
them.

- **Prototype poisoning of an `"object"` section via a crafted filename.**
  Moving the object-section merge out of `shard-cli` dropped the id check, and
  the key was assigned with plain `[]=`. A branch committing
  `spanish/curriculum.d/spine/0010-__proto__.json` invoked the inherited setter
  instead of creating a key: the node's realization vanished and the assembled
  `spine` object was re-parented to attacker-supplied JSON.
  `rejectDangerousKeys` did not help — it inspects a parsed *value's* own keys,
  and this key came from a *filename*. Reachable through
  `curriculum.ts`'s `curriculum.spine?.[node.id]`, which reads through the
  prototype chain, silencing `missing-curriculum-spine-node` for every
  unrealized node; and invisible to the reverse check, which uses
  `Object.keys`. `--check` could not catch it either — a poisoned node produces
  no own key, so the rebuild omits it and the committed monolith still matched.
  Fixed three ways: `SHARD_ID_PATTERN` is re-applied on read, every
  name-derived key goes through `assertSafeKey`, and both mergers assign with
  `Object.defineProperty`.
- **A shard belonging to no section was read, parsed and silently discarded** —
  the silence that let the above pass CI green. Now refused by name. The
  grouped path gets the mirror-image guard: it consumed *every* non-meta shard
  regardless of directory, so `book-generation.d/sub/spanish.json` would have
  duplicated that language's entries into all six arrays.
- **`_keys` could truncate the rebuilt document.** It decides which properties
  the document has, and the rebuild emitted only the keys it named, so
  `"_keys": ["path","spine","extensions"]` silently dropped `version`,
  `language` and `conceptAliases`. Caught today only by luck — every plan is
  `"generated"`, so `--check` byte-compares against a committed file; a
  `"removed"` ledger has none. `_keys` must now be a **permutation**.
- **`mergeGroupedShards` was missing the two `_meta.json` guards its siblings
  have.** A `_meta.json` holding `["a","b"]` spread to `{0:"a",1:"b"}`; one
  carrying `targets` was silently shadowed by the assembled array.
- Two sections sharing a shard directory are now refused at plan level: each
  would claim the other's files, and the ledger would come back with the same
  elements under two keys.
- Deleted the dead `idFromShardName` in `shard-cli`, whose docstring still
  claimed the result was "re-validated against `SAFE_ID` by the caller" —
  documenting a control that no longer existed anywhere.

