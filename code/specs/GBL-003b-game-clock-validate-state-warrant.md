# GBL-003b — Exact GAME10 `validate_state` Warrant Claim

## Status and selection

This is the selected next GAME10 assurance slice after the merged Rust and TypeScript public
clock implementations and the merged GBL-050a input-frame vertical slice. It targets a concrete
assurance gap: no exact `validate_state` claim exists, the current Warrant `state_inv` omits the
elapsed-simulation/tick product relation, and existing invariant claims tolerate errors. This
item adds the missing exact validator claim and corrects its invariant. It is limited to exact
`validate_state` behavior; it does not complete GAME10 source assurance, `advance_frame`,
constructors, or whole-package proof.

The authoritative behavior is `code/packages/rust/game-clock/src/lib.rs::validate_state`,
with the public types, constants, error enum, and field order specified by
[`GAME10-fixed-step-clock-contract.md`](GAME10-fixed-step-clock-contract.md). The checked-in
Warrant document is `code/specs/game-clock.warrant`; this work must add the exact validator model
and claim and correct its success invariant without silently changing that public Rust behavior.

## Exact subject behavior

For a `GameClockState` with ten fields, evaluate these guards in this exact order and return the
first matching result:

1. `fixed_dt_ns == 0` → `Err(ZeroFixedTimestep)`.
2. `fixed_dt_ns > 10_000_000_000` → `Err(CorruptedStateInvariant)`.
3. `max_frame_steps == 0 || max_frame_steps > 1024` → `Err(CorruptedStateInvariant)`.
4. `max_frame_delta_ns < fixed_dt_ns || max_frame_delta_ns > 10_000_000_000` →
   `Err(CorruptedStateInvariant)`.
5. `accumulator_ns >= fixed_dt_ns` → `Err(CorruptedStateInvariant)`.
6. The mathematical product `tick_count * fixed_dt_ns` is greater than `u64::MAX` →
   `Err(TimestampOverflow)`.
7. `elapsed_sim_ns` differs from the exact product → `Err(CorruptedStateInvariant)`.
8. Otherwise → `Ok(())`.

The overflow case must not use wrapping multiplication or compare against a wrapped product.
Prefer Warrant's checked multiplication result (`checked_mul(tick_count, fixed_dt_ns)`) and map
`None` to `TimestampOverflow`; confirm this exact expression is accepted by the native
parser/typechecker/evaluator and has the same concrete semantics as Rust. Preserve the distinction
between overflow at guard 6 and an ordinary elapsed mismatch at guard 7. For SAT, construct and
validate an independent overflow-safe circuit for each declared fixed `fixed_dt_ns`, with full
64-bit `tick_count` and `elapsed_sim_ns` inputs. A universal symbolic proof of multiplication over
arbitrary pairs of 64-bit operands is a separate open assurance task; it is not required or implied
by this bounded fixed-parameter slice.

Exactly four state fields (`frame_count`, `elapsed_wall_ns`, `total_dropped_ns`,
`spiral_clamped_frames`) are ignored and do not affect acceptance, while
`tick_count`, `elapsed_sim_ns`, `accumulator_ns`, `fixed_dt_ns`, `max_frame_steps`, and
`max_frame_delta_ns` do. In particular, do not invent a relation for wall time, frame count, drop
counters, or clamp count.

## Warrant model and claim requirements

1. Write a total independent validator model over the actual `GameClockState` record and the
   existing `ClockError` variants. It must return `Result<Unit, ClockError>`. This Warrant
   document declares `model Unit = Bool`; normalize public Rust `Ok(())` to Warrant `Ok(true)` at
   the operation boundary. Do not compare a host unit value with Bool.
2. Express all eight ordered branches above, whose observable results are exactly four values:
   `Ok(true)`, `ZeroFixedTimestep`, `CorruptedStateInvariant`, and `TimestampOverflow`. Guards 2–5
   share one error discriminant, so their order is preserved in the model but not presented as
   externally distinguishable return precedence. The overflow branch must have the same precedence
   relative to earlier invariant failures and the elapsed mismatch as the Rust implementation.
3. Add the exact result-equality assertion between the public `validate_state` operation and this
   model for every representable state input. The assertion states the full-domain contract; this
   item's finite evaluator runs and fixed-parameter SAT results do not by themselves certify that
   universal assertion. Do not retain a wildcard `Err(_) -> true` escape or a claim that checks
   only success invariants, and do not upgrade a universal assurance-report slot without a sound
   full-domain proof route.
4. Replace or rename the current weaker `state_inv` helper so no downstream claim can mistake it
   for validator acceptance. The validator-success predicate must include configuration limits,
   accumulator bound, checked product existence, and exact elapsed/product equality. Keep a weaker
   transition invariant only if it has a distinct name and claims use it intentionally.
5. Keep the existing constructor proof separate. This claim is not a proof of `new_clock`,
   `advance_frame`, or the whole source module.
6. The model must run through the native Warrant parse/typecheck/evaluator route used by repository
   assurance tooling. Execute both the table-driven branch corpus and seeded generated cases
   through this model, comparing each result with both the public Rust API and the independent
   oracle. Any needed representation or evaluator support must be implemented and independently
   tested as part of this item; do not substitute a hand-coded test model for Warrant execution
   or cite a build-only `BUILT` row as evaluator evidence. Native evaluation over generated
   full-width inputs is finite evidence, not exhaustive evaluation.
7. Add a claim-AST shape check requiring exactly one unrefined bare `state` binder and direct
   equality of `validate_state(state)` with `validate_state_model(state)`. Negative controls must
   reject a missing claim, extra/guarded claim, reversed operands, a wrong binder, a renamed subject,
   or an extra source-level bypass. The gate must inspect the parsed Warrant AST, not only search
   for matching text.

## Independent evidence

### Public API and oracle

- Add table-driven cases for all four result variants and every branch boundary. Include states
  with multiple invalid fields so the first error is pinned where distinct discriminants expose
  precedence (zero vs corrupted, corrupted vs overflow, and overflow vs elapsed mismatch). Guards
  2–5 all return `CorruptedStateInvariant`; demonstrate their isolated reachability without
  claiming their relative order is observable from the result alone.
- Build an independent test oracle using checked/widened `u128` arithmetic, not the Warrant
  expression or production helper. Compare the complete public result (exact `ClockError` or
  `Ok(())`) from the real Rust `validate_state` API. The oracle must not mutate its input.
- Cover the product boundary at `u64::MAX`, first overflowing products, exact product matches,
  and mismatches on both sides of the boundary. Include zero and hard-limit configurations and
  `max_frame_steps` endpoints.
- Add four ignored-field metamorphic controls, one for each ignored counter
  (`frame_count`, `elapsed_wall_ns`, `total_dropped_ns`, `spiral_clamped_frames`), using zero,
  ordinary, and `u64::MAX` representatives while holding the result fixed. Also mutate each of
  the six semantically read fields (`tick_count`, `elapsed_sim_ns`, `accumulator_ns`,
  `fixed_dt_ns`, `max_frame_steps`, `max_frame_delta_ns`) in valid or deliberately isolated
  boundary cases to demonstrate its result/precedence effect. The adapter must round-trip all ten
  fields in their declared order and reject missing, extra, reordered, or wrongly typed fields;
  if serialized as bytes, one all-`u64`/`u32` state occupies exactly 76 payload bytes. State
  clearly that generated totals are finite evidence, not exhaustive runtime coverage.
- Ensure failed validation leaves all ten original public state field values unchanged; do not
  claim a meaningful raw-struct padding-byte guarantee.

### Property and fuzz evidence

- Add deterministic seeded properties over full-width `u64` fields and `u32` step limits,
  stratified toward valid states, all four observable result variants, every guard, multiplication
  overflow, and near-boundary products. Compare public API and native Warrant results against the
  independent `u128` oracle for each generated input.
- Use the repository's existing Rust property and fuzz tooling; record the exact engine,
  generator, seed, case/callback counts, shrink behavior, and resource limits. Keep valid-state
  typed Rust generation separate from malformed/hostile serialized-adapter input campaigns; only
  the adapter boundary has malformed record shapes or types. Unexpected panics, oracle
  disagreement, or a valid typed state refused by the adapter is a failure, not a swallowed fuzz
  result.
- Report per-stratum executions so all four outcomes (`Ok`, `ZeroFixedTimestep`,
  `CorruptedStateInvariant`, `TimestampOverflow`) and precedence combinations are shown reached.
  Include a known-bad oracle/predicate control that shrinks to a smaller counterexample while the
  production oracle remains unchanged.
- Preserve minimized failures as focused regression cases. Mutation/refusal counts are not
  native branch coverage.

### Bounded SAT and replay

- Add a clearly named bounded bit-vector check for the ordered validator and exact result.
  Use independent Warrant/model and subject circuits and the repository's checked solver/certificate
  path (including DRAT verification where supported). Report the exact domain and every fixed
  parameter; never label a finite miter universal. The circuit must use full-width 64-bit
  `tick_count` and `elapsed_sim_ns` inputs for each fixed-`fixed_dt_ns` query; do not shrink these
  domains to small integers.
- The required fixed `fixed_dt_ns` probes are `1`, `2`, `3`, and `10_000_000_000`. Include
  explicit `fixed_dt_ns == 0` and `fixed_dt_ns == 10_000_000_001` controls with actual query
  constraints and unchanged public replay; `fixed_dt_ns == 1` cannot overflow a `u64` tick product
  and must not be counted as overflow coverage.
- Describe whether each circuit is hand-encoded or derived from a checked AST/model. If
  hand-encoded, compare its truth table on concrete branch vectors against both Warrant evaluation
  and the independent oracle before relying on its SAT result. Keep native Warrant evaluation on
  generated full-width cases distinct from the bounded fixed-`fixed_dt_ns` SAT theorems.
- SAT witnesses for a distinct-result precedence, overflow, mismatch, or accepted-state mutant
  must be decoded without changing any input and replayed through the public Rust API and independent `u128`
  oracle. Require the expected exact result on both. An invalid binding, parser refusal, timeout,
  unknown verdict, or failed replay is not a killed mutant. Negative controls must show that a
  deliberately broken predicate admits a satisfiable counterexample; evaluate that broken
  predicate on the unchanged decoded witness and require the expected changed result. Also require
  that the unmutated comparison has its claimed verdict.
- Keep these fixed-parameter or otherwise bounded checks distinct from finite native Warrant
  evaluation of generated full-width inputs and from source-to-Warrant equivalence. No
  whole-`validate_state` source proof is
  claimed unless a later item binds the actual Rust source and proves its lowering.

## Inputs, build, and evidence boundaries

- Track every exact source, Warrant document, test, and fixture file read by the evidence command
  through the repository's existing source-selection and cache mechanisms. Build-tool dependency
  metadata alone is not proof that the provider or transitive inputs are selected or hashed.
- The native Rust API and Warrant tooling must be run by the normal package `BUILD` recipes on
  supported platforms, with effective strict preflights recorded. Ensure test-only Warrant
  evaluator/proof targets are actually executed by both normal platform recipes; compile-only
  targets and aggregate `BUILT` summaries are insufficient.
- Reuse the established assurance report, fixture, and build-tool formats. Do not create a second
  package matrix or evidence registry. Refresh canonical package presence separately from
  behavioral evidence.

## Explicit non-claims and follow-ups

- This item does not prove `advance_frame`, constructor APIs, the complete GAME10 module, a Rust
  source-to-Warrant translation, zero allocation, timing, or cross-language parity. Do not claim
  that fixed-parameter SAT checks prove universal correspondence of Rust source to Warrant.
- `GBL-020a` fixed-point game primitives need a separate contract and remains deferred.
- Remaining cache-wide external-input audits, implicit CI preflight receipts, stale superseded
  workflow-run cleanup safety, TypeScript parser ASI behavior, and macOS temporary-path
  canonicalization are separately tracked follow-ups. They must not be folded into this validator
  proof without a demonstrated dependency.
