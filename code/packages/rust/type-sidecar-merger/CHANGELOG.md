# Changelog

All notable changes to the `coding-adventures-type-sidecar-merger` crate will be documented in this file.

## [0.1.0] - 2026-05-22

### Added
- New crate per CLOC04 §"Merging policy" — combines multiple sidecars from JSDoc / TypeScript / external producers into one before the typechecker runs.
- `MergePolicy::Default` — conservative default. Matching `ty` survives with merged provenance evidence; conflicting `ty` clears to `None` with a `merge` EvidenceStep noting the conflict; per-attribute "more conservative wins" (`False` > `True` > `Unknown` for `nullable`/`readonly`/`pure`/`no_side_effects`/`idempotent`); `deprecated` messages join with `; `; `extension` keys union with later-wins on collision.
- `MergePolicy::Strict` — errors on any `ty` disagreement.
- `merge(sidecars: Vec<Sidecar>, policy: MergePolicy) -> Result<Sidecar, MergeError>`.
- `MergeError { cv, message }` with `Display` + `std::error::Error` impls.
- 12 tests covering: empty input, single-sidecar pass-through, matching-ty union, differing-ty Default clears + logs evidence, differing-ty Strict errors, attribute conservative merge, attribute Unknown yields to claim, attribute Strict behavior, deprecated message join, deprecated single-side pass-through, extension union with later-wins, records-in-one-sidecar pass-through, `MergeError` Display/Error.

### Notes
- Dependencies: `coding-adventures-type-sidecar` + `serde_json` (for `extension` value comparison in tests). No other deps — the merger is policy-agnostic about producers.
- v1 type-agreement is exact `PartialEq` on `Type`. The richer structural-intersection from CLOC04 (e.g. `string` ∩ `string | undefined` = `string`) ships once the full `Type` lattice lands.
- `MergePolicy::TsWins` and other priority-based policies are deferred.
