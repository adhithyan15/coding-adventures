# Changelog — `lang-refined-types`

## 0.1.1 — 2026-05-04

Security hardening — allocation-free canonical sort for `Predicate`.

### Fixed

- **`Predicate::canonicalise()` used `format!("{:?}")` as a sort key**.
  `And` / `Or` operands were sorted by comparing their `Debug` strings:
  `parts.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")))`
  This allocated two `String`s per comparison pair, giving O(n log n ×
  average-predicate-size) allocation cost.  A `Membership { values }` with
  k values, nested inside an `And`, produced a string proportional to k on
  every comparison; a user who crafted an `And` of many such predicates
  could trigger O(n² k) allocation.

  **Fix**: implemented `PartialOrd + Ord` for `Predicate` using a
  variant-tag discriminant (stable fixed order across variants) followed
  by field-level lexicographic comparison — all allocation-free.
  `canonicalise` now calls `parts.sort()` + `parts.dedup()`.

### Added

- `impl PartialOrd for Predicate` — delegates to `Ord`.
- `impl Ord for Predicate` — tag-then-fields total order; uses existing
  `Ord` impls on `VarId`, `CmpOp`, `Option<i128>`, `Vec<i128>`,
  `Vec<Predicate>`, `Box<Predicate>`, `i128`, `String`.
- 3 new unit tests: `canonicalise_and_is_order_independent`,
  `canonicalise_or_is_order_independent`, `canonicalise_deduplicates`.

## 0.1.0 — 2026-05-04

Initial release.  **LANG23 PR 23-A.**

### Added

- `Kind` enum: `Int`, `I8`–`I32`, `U8`–`U64`, `F32`, `Float`, `Bool`, `Nil`, `Str`, `Any`, `ClassId(String)`.
  - `from_type_hint(s)`: parses LANG22 type-hint strings.
  - `as_type_hint()`: inverse mapping (round-trips).
  - `is_solver_supported()`: true for integer and Bool kinds.
  - `integer_bounds()`: returns `(min, max)` for bounded integer kinds.
  - `is_integer()`: true for all integer kinds.
- `Predicate` enum: `Range`, `Membership`, `And`, `Or`, `Not`, `LinearCmp`, `Opaque`.
  - Smart constructors: `and()`, `or()`, `not()` with flattening and double-negation elimination.
  - `simplify()`: range intersection merging + deduplication.
  - `canonicalise()`: sorted + deduped operands for stable `Hash`.
  - `to_constraint_predicate(var_name)`: lowers to `constraint_core::Predicate`.
  - `Display` impl for human-readable output.
- `VarId(String)`: named variable for `LinearCmp`.
- `CmpOp` enum: `Lt`, `Le`, `Eq`, `Ge`, `Gt` with `Display`.
- `RefinedType { kind, predicate }`: unified type representation.
  - `unrefined(kind)`, `refined(kind, predicate)`.
  - `is_unrefined()`, `is_refined()`, `display_str()`.
  - `Display`, `Hash`, `Eq` implementations.
- 38 unit tests covering kind round-trips, predicate algebra, lowering, and display.
