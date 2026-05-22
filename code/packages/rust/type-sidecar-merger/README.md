# coding-adventures-type-sidecar-merger

Fuses multiple [`Sidecar`]s from different producers — JSDoc, TypeScript,
hand-written external `.d.ts`-style annotations — into one canonical sidecar
the Closure typechecker consumes. Per
[CLOC04 §"Merging policy"](../../../specs/CLOC04-type-sidecar-format.md).

## What's here (v1)

- `merge(sidecars, policy) -> Result<Sidecar, MergeError>`.
- `MergePolicy::Default` — the conservative default. Matching `ty` survives
  with merged provenance; conflicting `ty` clears to `None` with an
  `EvidenceStep`; per-attribute "more conservative wins" (False > True >
  Unknown); `deprecated` joins with `; `; `extension` keys union with later-
  wins on collision.
- `MergePolicy::Strict` — errors on differing `ty`.
- `MergeError { cv, message }` with `Display` + `Error`.

## Dependency whitelist

- `coding-adventures-type-sidecar` — the format we operate on.
- `serde_json` — for `extension` value comparison in tests.

Nothing else. The merger is policy-agnostic about producers; it only knows
the sidecar shape.

## What's deferred

Per CLOC04 §"Merging policy":

- Structural intersection of `Type` (e.g. `string` ∩ `string | undefined` =
  `string`) — needs the full `Type` lattice; v1 uses exact `PartialEq`.
- `MergePolicy::TsWins` and other priority-based policies.
- Streaming API for very large sidecars.
