## Unreleased - VOS-039c: the `vos-sched-any` document gains seven claims

### Changed

- `tests/census.rs` moves with the contract's `any` block (VOS-039c adds seven oracle-preservation claims to
  `code/packages/rust/vos-sched-contract/warrant/obligations-any.part`, so `code/specs/vos-sched-any.warrant` is 15
  claims, was 8): `vos-sched-any` is 15 `ir.data`, the total 357 to 364 claims (242 emittable, `W0701` 6), `ir.data`
  100 to 107.  The README census table and the fixture paragraph follow.  No source change.

Specification: VOS10 section 9.3.
