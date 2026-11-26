# 00020 - LGE-S3-FIX review round

* **CWE-1321 subset renamed** `no_computed_key_writes_to_shared_objects` and restated: no write to any
  class, module, global, prototype or metatable object; no write under a non-compile-time-constant
  key; no recursive merge. The old wording missed `setattr(OwnClass, k)`, `globals()[k]`, `_G[k]`,
  `Object.prototype[k]` (the key is caller-chosen, the object is not reachable from the caller's value).
  Registry JSON rationale, VER15a sections 1.3 and 5.3 updated.
* **CWE-94 `no_dynamic_eval_reachable`** keeps the `proved` floor only under fail-closed semantics:
  over-approximating rule (unresolved dynamic dispatch is a finding), mandatory seeded negative-control
  `lane-corpus` in `exclude_requires`, closed versioned per-lane primitive list with a mandatory
  minimum, transitive and runtime-loaded coverage, non-literal load/require counts as reaching;
  build/install hooks are out of scope and belong to the supply-chain row.
* **Whitelist rationales (VER15a 5.5):** JVM/.NET/Go entries exclude the prototype-chain mechanism only;
  Swift conditional on no NSObject, `@objc dynamic` or KVC; Dart mirrors note; Haskell/OCaml/Elixir
  unsafe-state note; whitelist is sole authority for haskell/swift/dart/elixir; a whitelisted lane
  uses `excluded-by-language`, never `not-applicable`.
* **`source-ref`:** id prefix must equal kind; route membership checked on resolved kind.
* **Schema:** `becomes_when` backlog id requires a hyphen (`^[A-Z][A-Z0-9]*(-[A-Za-z0-9<>]+)+$`).
* **Rule 13:** exact-case match (no case folding); templates other than `LGE-L-<lane>` match only
  lanes named in a `(lanes: ...)` marker on the backlog row.
* **Rule 1:** v1 equality defined testably (classes only; `version`/`last_updated` excluded;
  appended-only deltas, rationale `startswith` v1 text).
