# Changelog

## Unreleased — WX-F driver record schema (slice 1)

- Parse and schema-validate binding-file `driver` records (VER04k 3.2/3.4
  rules 1-3, 7): `state`/`ctx`/`new`/`drive`/`step`/`finish`/`result`/
  `errors`/`lengths` with closed vocabularies, capped identifiers and
  paths, and strictly increasing bounded lengths. Every violation is
  W0501; op lines take exactly one of `export=` and bare `driver`.
- Generation refuses driven operations explicitly (W0704) until a later
  slice consumes the records. Typing against the document (W0711),
  obligations, evaluation and sabotage follow.

## Unreleased — opt-in Boolean record-result fault (WX-FAULT2)

- Add the `bool-field-flip` evidence fault for a declared record result with a
  direct `Bool` field, optionally behind one `Result` or `Option`. It flips only
  the first direct Boolean field and preserves the rest of the record; unsupported
  shapes fail closed. The opt-in helper and targeted-attack checks are emitted
  only for adopters that bind the fault. The WX-FAULT2 fixture compiles and runs
  direct, `Result`, `Option`, combined-fault and non-opted-operation controls.

## Unreleased — `evidence::type_mentions` (VOS-039c, round 2)

- New `pub fn evidence::type_mentions(file, type, target) -> bool`: does a declared type, resolved through the document's aliases, records and variants by the same table the generator's `ty_of_type` uses, mention the record named `target`?  It lets a gate (`vos-manifest`, for `vos-link` CC11) agree with the generator by construction instead of re-reading type text.  Fails closed: an unresolvable name, an alias cycle or a chain deeper than 64 counts as a mention.  A shared sub-type is expanded once.  Tests (`type_mentions_tests`) pin it against `ty_of_type` over alias, alias-of-alias, `Option`, `Result`, tuple, list and record-field shapes.  No generator output changes.

## Unreleased — the `field-flip-last` fault (VOS-039c)

- New record fault `field-flip-last` (VER04j 4.8): the lowest bit of the LAST numeric field of a record result is flipped (`fault_field_flip_last`, the mirror of `fault_field_flip` searching from the back; never inert).  Applicable only to a single-constructor record with at least two numeric fields, else W0701 (with one numeric field it would only duplicate `field-flip`; a variant, or a record with none, is refused as for the other record faults).  No `WARRANT_EVIDENCE_ATTACK` semantics.  Gated by its own prelude feature (`lastflip`): `fault_field_flip_last` and `Fault::FieldFlipLast` are emitted only for a binding that lists it, so every harness that does not is byte for byte what it was (the other twelve `gen_evidence` adopters and the fp-bitmap and round-robin crates did not regenerate).  Why: the record faults touch the first numeric field, which for an `-any` image `Impl(mask, choice)` is the bitmap, so nothing could corrupt the oracle register and a claim that an operation keeps it had no control that could fail it (VOS-D080).
- Tests: `tests/wxa_generator.rs` (accepted on a record with two numeric fields, alone and inside a `Result`; W0701 on one numeric field, none and a variant; emitted only for a binding that names it, with no attack switch, and independently of `field-flip`), `tests/wxa_evidence.rs` (the function flips the last field and leaves the first, twice restores, a single numeric field is flipped where it is, a non-record is unchanged; the full generated suite catches the new fault on `normalize`), the WX-A fixture binds it and `tests/generated/wxa_evidence.rs` is regenerated.
## Unreleased — observer counts the export only; `fixed<N>` in models (VOS-038c)

- VOS-D063 (VER04j 6.3, 14.3 item 54): the allocation observer's window is closed when a job starts and is
  opened by `obs_scope` around the call of the export (the generator wraps each call of the binding's
  `export=` in the emitted adapter; no call found is W0701). The harness's own marshalling of a record or a
  payload-carrying variant (64 bytes for `Run(u8)`) is no longer charged, so `heap <= 0` holds on every
  operation of a source with no allocator. The `limit-after-alloc` hog is allocated inside a scope, so the
  observer still flags it. Prelude `obs` region only: harnesses without a `heap` or `output` row are byte for
  byte unchanged; `hash-functions`, `lzw`, `wasm-leb128` and the three `vos-sched-*` harnesses were regenerated.
  Controls: `tests/wxo_observer.rs` (a record and a variant result pass `heap <= 0`; an export that allocates
  a kilobyte fails it with `bound-heap-exceeded`), the existing `wxh_*` observer tests (now run their jobs
  inside `obs_scope`) and unit tests of `scope_export_calls`.
- Review of VOS-038c: every export call is wrapped as `obs_call!(..)` and generation fails if ANY occurrence of
  the export name (method call, turbofish, bare reference) cannot be wrapped. `obs_call!` marshals a clone and
  drops the original result inside a window, so an allocating `Drop` is charged to the subject; a result with drop
  glue that is not `Clone` fails the instance loudly. wxo gains `held` (allocating Drop, caught) and `raw`
  (uncloneable, refused). The observer's thread and lazy-static limits are stated in VER04j 14.3 item 54 (VOS-D102).
- VOS-D025 first half (VER04j 14.3 item 55): `fixed<N>(bytes)` with a literal `N` is generated (`V::fixed_n`,
  a new `fixedn` prelude region emitted only when a body uses it) instead of stopping at W0701. Fixture
  `tests/fixtures/wxf.warrant` builds a `Bytes<8>` state with `fixed<8>(concat(..))`; its 256-state exhaustive,
  property and fuzz suites and its negative controls pass (`tests/wxf_array.rs`). The second half of D025, a
  claim that calls no operation, is unchanged (VOS-D005).

## Unreleased — warrant_negctl.py bounds a hung command (VOS-038b Windows `rust/lzw` hang)

- `code/scripts/warrant_negctl.py` kills the sabotaged command's whole process tree after
  `WARRANT_NEGCTL_TIMEOUT_SECS` (default 1200 s, clamped to 86400 s; nan, inf, zero, negative and
  non-numeric fall back to the default) and fails the control with the output captured so far,
  which appears once. The same kill runs on SIGTERM or any abnormal exit of the checker, and the
  Windows `taskkill` call has its own 30 s bound. Before this a hang printed nothing and cost the
  build tool's 90-minute limit. TF01 section 6 updated; tests added to `test_warrant_negctl.py`.

## Unreleased — CV2-S10c-2: a parenthesised shift count selects `NatCount`

- `count_binders` (the `NatCount` refinement of VER04j 3.6) looks through `Expression::Grouped`:
  `shl(x, (n))` selects `n` exactly as `shl(x, n)` does, as VER04r 5.1 and 16.10 R10 say
  (parentheses do not matter: the corpus IR has no groups, and the oracle decides R10 on it). An
  expression of the binder (`shl(x, (n + 1))`) still does not select. No adopter writes a
  parenthesised count, so no checked-in evidence file changes.

## Unreleased — bounded VER04p domain witness checks

- `BUILD` and `BUILD_windows` run `code/scripts/tests/test_ver04p_domain_witnesses.py`.
  The exact-rational diagnostic checks the documented integer ties, adjacent F32/F64
  encodings, F64-to-F32 midpoint, post-rounding de-duplication, largest-finite ranges,
  and subnormal one-tap ratio. It uses no host float and is not a Warrant native-float
  or source/compiler proof. The build tool selects `rust/warrant-grammar` when the
  VER04p document or diagnostic changes.

## Unreleased — refinement sampler and observer timeout clamp (VOS-038b)

- VOS-D079 (VER04h 4.1): a refined binder of low acceptance no longer reports `property-exhausted`.
  The rejection draw is unchanged; when a case's 100 retries are all rejected and every binder is
  exactly enumerable, the case is drawn uniformly from the admissible set (built once). Otherwise an
  acceptance floor (1/8 over 2,048 probe draws, derived from C(1-a)^100 < 0.01 at C = 4,096 from a derived stream) refuses the claim on its first
  draw with a clear message. Emitted from a new `refine` prelude region (a harness without a refined
  binder is byte for byte unchanged); passing runs draw identical cases, so no manifest changes.
  The floor runs in the harness, not the generator (VOS-D100).
- VOS-D085 (VER04j): in the `obs` region, `WARRANT_OBS_TIMEOUT_SECS` is clamped to `OBS_MAX_RAISE_SECS`
  (24 h) by `obs_raise_ms`, and the deadline is `obs_deadline` (`checked_add`, fail closed to an
  already-past deadline), so a huge value neither panics nor disables the bound.
- Tests: rescue, empty set, high acceptance, floor refusal, floor boundary and `exact_support` (`wx`),
  the composite branch (`wxa`, whose fixture gains a vacuous refinement so the branch is compiled), the
  clamp table, `Duration::MAX` and a child process with the maximal variable (`wxh_supervisor`).
- Regenerated: adopters arithmetic, ct-compare, gf256, logic-gates, lzw, wasm-leb128 (refine);
  hash-functions, lzw, wasm-leb128, vos-sched-any, -fp-bitmap, -rr (obs); fixtures `wx`, `wxa`, `wxh`.
  vos-probe-ready, x25519 and zeroize are unchanged.

## Unreleased — WX-SV2: the `field-flip` fault (VOS-038a)

### Changed

- **WD-HARN1c: negative controls run through the shared checker.** The sole sabotage line in
  `BUILD` and `BUILD_windows` (the `--sabotage always-pass` self-corpus control) now call `code/scripts/warrant_negctl.py -- cargo test ...` instead of a
  `grep`/`findstr` pipe. Every fault, filter and cargo argument is unchanged and in the same
  order. The checker also requires a non-zero exit, exactly one TAP plan, test points numbered
  `1..N` and no `Bail out!`, and prints the captured output when a control fails (the Windows
  lines printed nothing). Each BUILD file declares `# needs-toolchain: python`. No source,
  test or evidence change.

- New composite-operation fault `field-flip` (VER04j 4.8): the lowest bit of the first
  numeric field of a single-constructor record result is flipped (`Int` and fixed-width
  alike). Applicable exactly where `field-zero` is (W0701 otherwise) and, unlike it, never
  inert on a record, so a by-value `init() -> Impl` over a zero state has a negative control.
- Emitted from a prelude region (`//@if fieldflip`) enabled only when some binding lists
  `field-flip`; the `Fault` enum is already restricted to used faults. Every harness that
  does not name it is byte for byte unchanged (checked with `gen_evidence --check` over all
  13 Rust packages with an `evidence.bindings`).
- The same region adds `WARRANT_EVIDENCE_ATTACK=<op>`: a sabotage run faults only that
  operation, because flipping an `init` and the `alpha` that reads it cancels.
- Tests: the applicability table, the gating (no `fault_field_flip`/`FieldFlip`/
  `WARRANT_EVIDENCE_ATTACK` text without the binding), and, on the regenerated `wxa` fixture
  (`normalize` now lists `field-flip`), the fault's behaviour and its negative control.
- Regenerated: `tests/generated/wxa_evidence.rs` (opt-in), `vos-sched-fp-bitmap`, `-any`, `-rr`.

## Unreleased — shared negative-control checker tests (WD-HARN1c)

- BUILD and BUILD_windows run `code/scripts/tests/test_warrant_negctl.py`, the tests of the shared
  sabotage checker `code/scripts/warrant_negctl.py` (stdlib Python; `python3` on Unix, `python` on
  Windows). Both declare `# needs-toolchain: python`. The build tool lists the script and its tests
  as repository inputs of this package, so editing either reruns them. No source, prelude or generated
  file changes; the package's own sabotage line is unchanged (adopters migrate in later PRs).

## Unreleased — WD-HARN1b harness follow-ups (LOW findings of the WD-HARN1 review)

- Prelude, `obs` region only: `obs_bail_out` writes its stderr line with
  `let _ = writeln!(std::io::stderr(), ..)` (`eprintln!` panics on a closed stderr
  pipe, which would have unwound into the test instead of exiting 70); the
  `OBS_CALL_TIMEOUT_MS` doc comment now says a timeout bails out the whole process
  (it said "fails the instance"); the dead `latched = true` before `obs_bail_out`
  in the timeout arm is gone (the process exits at once; the `Tripped` and `Refused`
  arms keep theirs, which later instances read).
- Regenerated: the five obs adopters' harnesses (hash-functions, lzw, wasm-leb128,
  vos-sched-any, vos-sched-fp-bitmap) and the `wxh` fixture. No other adopter's
  harness changes.
- Sabotage lines of the obs adopters (BUILD, BUILD_linux, BUILD_windows) now pass
  only if the run printed a `not ok ` line and no `Bail out!` line, so a hang after
  an intended failure is no longer hidden by the pipe's last-command status.
  Documented in VER04j section 14.2 item 53.

## 0.8.0 — CV2-S2 public domain API, WD-HARN1 harness hardening, WX-FAULT1

Three changes ship together in this release; the first is new public API, the other two
were previously listed as Unreleased and are folded in unchanged.

### VER04r CV2-S2 public resolved-domain enumerator and post-slot resolver

Published API for the v2 corpus emitter (slice S3).  **No behaviour change**: every
adopter's generated `warrant_evidence.rs` (arithmetic, ct-compare, gf256,
hash-functions, logic-gates, lzw, vos-sched-any, vos-sched-fp-bitmap, wasm-leb128,
x25519, zeroize) is byte-identical to the one the previous generator emits, and
`regenerate --check` and `gen_corpus --check` are unchanged.

- New `pub mod domain`: `Domain` (the generator's binder type `evidence::Ty`, now
  `pub` and `#[non_exhaustive]`, with `domain_size`, `domain_bytes` and the new `nth(i)` / `iter()` that
  unrank any VER04r 5.1 descriptor without materialising it); `unrank_product`,
  `rank_product` and `ProductIter` (row-major, last binder fastest, start at any
  product ordinal); `DomainEnv` and `resolve_binders` (aliases, records, variants,
  `NatCount`, the VER04j 2.6 demotion, `W0706`/`W0707`); `resolve_post_slot_of` and
  `op_post_slot` (the VER04k 4.2 slot over domains); re-exports of the checker's
  `resolve_post_slot`/`slot_shape`/`PostSlot`/`SlotShape`, `corpus::inout_descriptor`
  and the budget constants.
- Internal, same code: the inline per-claim resolution in
  `generate_evidence_harness_with_tables` is now `resolve_claim_tys` (shared with
  `resolve_binders`), and `inout::slot_of` delegates to `resolve_post_slot_of`.
  `evidence::Ty`, `TyEnv`, `product_size`, `claim_domain` and the `Ty` size methods
  are `pub` or `pub(crate)`; `corpus.rs` and `gen_corpus` are untouched (frozen until
  S13).
- Documented divergences, not changed: the harness prelude enumerates byte strings
  first-byte-fastest while VER04i 2.6 and the v1 corpus are lexicographic (the API
  follows the spec); the generator's `NatCount` is 73 values where VER04r 5.1 says
  `0..=K`; the generator's signed edge sets are sorted and include `-1..=-255` where
  VER04r 5.1 says "same order".  S3 decides how the corpus names them.
- `Ty::Res` now carries the error type name (`ErrName`, ignored by equality) and
  `Ty::Data` the declaration's `record` flag, so `nth` produces evaluator-compatible
  values (`x == E.CASE` and `match ... E.CASE` work on them). Neither changes the
  generated output or any type comparison. `ResolvedBinders::sizes()` returns an
  `Option`. `FxEdge` is documented for widths of 16 and up.
- One deliberate exception to "no behaviour change": generator diagnostics that print a
  composite `Ty` with `{:?}` now also show the new `record: ..` field and `ErrName(..)`.
  No generated output and no test pins that text.
- Correction to the first draft of these notes: the byte-string order difference is a
  set difference for `List` of byte strings (the 16/4-value prefix differs); see the
  module docs and VER04r 16.2.
- Tests: `tests/domain_api.rs` (27, from outside the crate, against an independent
  eager oracle, an evaluator round trip, and a recomputation of the checked-in v1
  ct-compare `inputs_sha256` digests from `Domain` enumeration) and 8 doc examples.
- Docs: README section, VER-BACKLOG CV2 row, VER04r section 16.2.

### WD-HARN1 harness supervisor hardening (from the 2026-11-14 Windows `rust/lzw` hang)

- Prelude, observer supervisor: every wait on the worker is a `Condvar::wait_timeout`
  against one deadline of `OBS_CALL_TIMEOUT_MS` (120 s; `WARRANT_OBS_TIMEOUT_SECS` can
  only raise it). An overrun or a lost worker is fatal for the process: TAP
  `Bail out! observer-timeout: ...` on stdout, the same on stderr, exit status 70
  (`OBS_TIMEOUT_EXIT`), no later TAP line, no manifest. It is deliberately not a failed
  instance: a negative control counts any failure as a caught fault and a sabotage line
  greps `^not ok `, so either would pass on a stall (review finding H1). `is_detected`
  also refuses `observer-timeout` verdicts. A job that cannot be sent (worker gone) is
  reported as lost, not as a heap trip.
- Prelude, exit: abandoned workers (tripped or timed out) are never joined; the
  invariant (a worker takes only its own reply slot, no lock the supervisor, TAP printer
  or manifest writer needs) is written into the prelude comment and tested.
- Prelude, poison: new `lock_or_recover`; no `lock().unwrap()` remains (META, STATS,
  OBS_STATS, TABLE_STATS, SOURCE_SHA256, the reply slot). The allocator-side trip no
  longer skips its write on a poisoned slot (which would have left the supervisor
  waiting).
- Regenerated: every adopter's `tests/generated/warrant_evidence.rs` (arithmetic,
  ct-compare, gf256, hash-functions, logic-gates, lzw, wasm-leb128, x25519, zeroize) and
  the six `warrant-grammar` fixtures. Only lzw, wasm-leb128 and the `wxh` fixture carry
  the observer; the rest change only by `lock_or_recover`.
- Tests: `tests/wxh_supervisor.rs` (new; runs the generated WX-H harness against a
  subject that stalls and panics, with a 300 ms bound: in child processes it checks a
  stall inside a negative control and inside the whole suite each bail out with status
  70 and no `not ok`; in-process it checks the call times out, the shared locks stay
  usable, and a poisoned lock and reply slot recover), two generator tests in `wxh_generator`, hook updates in `wxh_evidence`
  and `wxh_caps`. New support subject `tests/support/wxh_subject_stalling.rs`.
- Specification: VER04j 14.2 item 53 (and a pointer from item 33), VER-BACKLOG row.

### WX-FAULT1 `flip-result-bit` for a fixed-width integer result

- Generator: the composite adapter accepts `flip-result-bit` for a `Bytes -> U32/U64`
  (any `Fx`/`FxEdge`, inside an `Ok` or `Some`) result, so a checksum or hash operation
  has a negative control. `drop-last-result-byte` is still bytes only (W0701 otherwise).
- Prelude: `fault_flip_result_bit` gains `V::Fixed(f, x) => V::Fixed(f, x ^ 1)` (width
  kept, in range for signed and unsigned) in a new `fixedfault` region, emitted only when
  such an operation is bound. Every existing adopter and the WX-N/WX-V fixtures regenerate
  byte-identically (`gen_evidence --check` for all eight, `regenerate --check`).
- Tests: three in `wxe_edges` (the control for `U32` and `U64`, the arm absent for a byte
  result, the refusals that remain).
- Specification: VER04j 14.1 item 46 and the VER-BACKLOG row.

## 0.7.0 — WX-G `inout` parameters and `ensures` postconditions (VER04k section 4)

- Language: `name: inout T` (contextual, op parameters only; `x: inout` alone is still
  an alias; W0101 in a v0 document) and `ensures op:` blocks (W0101 in v0). New
  `Parameter::inout` and `Declaration::Ensures`; `Parameter`'s `Debug` is unchanged
  unless `inout` is set, so the pinned v0 hashes do not move.
- Checker: W0713 for an inadmissible `inout` type, more than four, `fresh`, no
  post-state slot (S1 to S4) or an `ensures` row that calls an operation or quantifies;
  `ensures` of an unknown operation (W0401), a duplicate block (W0201), more than 32
  rows (W0602), a parameter named `result` (W0203), a non-Bool row (W0301). New public
  `resolve_post_slot`, `SlotShape`, `PostSlot`, `slot_shape`; no field is added to
  `CheckedWarrantModule`.
- Bindings: `inout=<param>:<form>` (`slice`, `vec`, `array`, `int:<ht>`, `bool`,
  `struct:<Name>`; the VER04m forms parse and are refused W0704), and an `export=` Rust
  path of at most 8 segments for an operation with `inout=`.
- Generator: `Shape::Inout`; pre-state isolation; fully qualified call; read-back from the
  same buffer; S1 to S4 result assembly (a panic is `Err` with no post-state); the faults
  `stale-post`, `skip-last-write`, `drop-last-push`, `clobber-prefix`,
  `cursor-off-by-one` (W0701 where they do not apply); `ensures` monitors on every call,
  `<op>/post` suites, `monitored_calls`/`postcondition_rows` in the manifest, W0601 for an
  example that contradicts its rows. Gated: no other harness changes by a byte (all seven
  existing adopters regenerate identically).
- `corpus::inout_descriptor` gives the `inout` and `post_slot` fields VER04k 4.9 asks the
  v1 corpus to carry; the corpus compiler itself is still the v0 `ct_compare` profile.
- New diagnostic W0713 (checker) and W0501 (generator, malformed or stray `inout=` entry).
- Tests: `wxg_syntax`, `wxg_checker`, `wxg_slots`, `wxg_generator`, `wxg_evidence`,
  `wxg_broken`, `wxg_regen`.
- Specification: VER04k section 4.13 records the implementation notes.

## 0.6.0 — WX-N fixed-length byte arrays across the host boundary (VER04j 14.4)

- The language already had `Bytes<N>`; the gap was the binding and generator.
  New host type `bytes<N>` (the Rust `[u8; N]`, `&[u8; N]` as a top-level
  parameter), accepted only against a Warrant `Bytes<N>` of the same literal
  width (W0701 otherwise), width 0 to 4096.
- The composite adapter accepts `Bytes<N>` parameters (it refused them) and
  narrows with `try_from`; a wrong length is the harness's `Unreachable` signal,
  never a truncation.  A `Bytes<N>` result is length-checked whatever the host
  type, outside the panic containment, so a wrong-length export fails the harness.
- New op option `err_any=CASE` for a host `Result<T, E>` with an opaque `E`
  (`&'static str`): every `Err(_)` is the one Warrant case.  Sound only for an
  error type with exactly one case, so it is refused (W0704) otherwise: with
  two or more cases a wrong error would still map to `CASE` and `f(..) ==
  Err(CASE)` would pass vacuously.  Use `errors=` for a multi-case type.
- Known gap (LP-012): suite-level "exhaustive" totals include sampled
  `Bytes<N>` cases; the per-claim `domain` field is the authoritative label.
- Honest domain labels: a v1 `Bytes<N>` binder is `full` for N <= 1 and `sample`
  from N = 2 (it was `set`), through a `bytesn` prelude region that is emitted
  only for a document with a `Bytes<N>` binder, so no existing harness changes
  by a byte (`gen_evidence --check` on all adopters is the proof).
- WD-013 re-checked: closed for v1, pinned by a new test; the v0 evaluator is
  left frozen (backlog row).  Fixed arrays of integers are backlog row WD-015.
- Tests: U8 multiply and `shl` wrapping pinned alongside addition.  `wxn_generator` (host type, refusals, emitted conversions),
  `wxn_evidence` + `wxn_regen` (a compiled, running fixture: suite and negative
  controls, `sample`/`full` labels, both length checks, wrapping `U8`).

## Unreleased — VER04i WD-003 portable corpus

- Add a closed ct-compare Warrant v0 corpus profile generated from the checked
  Warrant source, with distinct subject-operation and pure-model IR, canonical
  admitted-case enumeration, independent input/result SHA-256 streams, generated
  examples and sabotage-control descriptors, and byte-stable `--check` support.
- Add a stdlib-only Python oracle that independently validates source-bound
  declarations, suite semantics, corpus digests and TFV examples, with mutation
  tests for incomplete or misbound evidence. The first profile rejects sharding
  and unsupported Warrant constructs; it does not claim to execute a lane.
- Route the corpus generator, oracle and their tests through package BUILD and
  repository-input selection.

- **`gen_evidence` passes the bindings' host lines through (fix).** The example
  that every package `BUILD` runs (`--example gen_evidence -- [--check] <pkg>`)
  built its `EvidenceOptions` with `hosts: Default::default()`, so the WX-A host
  lines of `evidence.bindings` (`struct`, `enum`, and `args=`/`ret=`/`errors=` on
  an `op` line) were parsed and then dropped. Only the in-test drivers passed
  them, so no real package could bind a host composite: an operation returning
  `Result<(u64, usize), Leb128Error>` failed with W0701 ("an args= entry is not a
  host type") or W0704 ("neither error normalization nor errors="). The driver
  now passes `b.hosts.clone()`. New `tests/gen_evidence_driver.rs` compiles the
  example into the test crate (`#[path]`) and drives `generate_one` on a scratch
  repository built from the WX-A fixture (`struct`/`enum` lines, composite
  `args=`, tuple and record `ret=`, `errors=`): generation succeeds, equals the
  in-test driver's output byte for byte, `--check` passes, and a hand edit is
  reported as drift. Before the fix the first two tests failed with the W0701
  above. A bindings file without host lines parses to the default, so the
  checked-in ct-compare and gf256 harnesses regenerate byte-identically
  (`gen_evidence --check gf256 ct-compare`, `v0_compat`, and the wave-3 regen
  tests). `generate_one` became `pub(crate)` for the test; nothing else changed.
- **`tests/v0_compat.rs` admits the first v1 adopter without relaxing v0.** It
  asserted that every `code/specs/*.warrant` and every bound package's document
  is v0, which `wasm-leb128.warrant` (header `warrant wasm_leb128 v1`) broke.
  The pre-WX-B documents (`ct-compare`, `gf256`) are now pinned by name and must
  still be v0 and present; every document must type-check; a v1 adopter's
  harness must regenerate byte for byte with its bindings' host lines, exactly as
  `gen_evidence --check` does, and record `warrant_version`; and for each v0
  package the harness generated with the host lines passed (the driver's path
  since the fix above) must equal the one generated without them.

## 0.5.0 - Warrant v1: WX-D1 slice S1 (checker and ground evaluator)

Implements the checker and ground-evaluator half of VER04k WX-D1 (slice S1 of
section 8.1), following the S0 errata of section 9.1. All of it is under the `v1`
header: a v0 document means exactly what it did, the checker's `Debug` output for
`ct-compare`/`gf256` is unchanged (`v0_compat`), and every generated harness
(`wx_regen`, `wxa_regen`, `wxh_regen`, `wxv_regen`) is byte-identical. Generation
still refuses the new forms and list literals with W0701 (slice S2a translates
them). `CheckedWarrantModule`, `CodegenLimits` and `EvalValue` keep their shape.

- **Checker.** `V1_FUNCTIONS` grows from 33 to 45 names: the seven D1 forms
  `take drop slice replicate range to_list to_bytes` and the five D2 names
  `fold map count all any`, all reserved in v1 (a v1 declaration so named is
  W0201) and still free in v0 (a call is W0202, a v0 model of that name keeps its
  meaning). The D2 names are refused in v1 with W0202 "reserved built-in `<name>`
  is not yet implemented in this version" (`CheckerErrorKind::NotYetImplemented`,
  S0 E7). The section 2.4 typing of the seven forms, with the expected type
  threaded into `typecheck_v1_builtin`, so `to_bytes([])` and `replicate([], n)`
  type-check; counts are `Int` (`Nat` widens). A v1 list literal of more than
  4,096 elements is W0602 at the literal's span, before its elements are typed
  (`MAX_LIST_LITERAL`, `CheckerErrorKind::ListLiteralTooLong`, S0 E8).
- **Ground evaluator.** The seven forms, with negative counts clamped to zero on
  the exact `i128` count and the check-before-allocate order of section 2.7:
  element cap (`MAX_SEQ_ELEMENTS` = 65,536), then data reservation, then work,
  then allocation. A cumulative per-instance data counter (`DATA_BUDGET_BYTES` =
  64 MiB, reset by `reset_work`) charges, in every v1 evaluation, the `dsize` of
  each node's value except the pass-through nodes (S0 E4), with deep-copy charges
  for `take`/`drop`/`slice`/`replicate` (S0 E5). New `CodegenErrorKind::
  {SequenceTooLong, DataBudgetExceeded}` (both W0703) and the accessors
  `Evaluator::{work_used, data_used}`. `and`, `or` and `=>` short-circuit in v1;
  v0 stays eager (S0 E2).
- **Operation discovery.** New `codegen::find_operation_for_version`: in a v1
  document the argument search also enters list-literal elements (S0 E13); claim
  extraction and the table vacuity check use it. `find_operation_in_expr` is
  unchanged (the evidence generator keeps it until S2a).
- **Tests.** `wxd_syntax` (no grammar change: forms parse as calls; token and
  grammar files pinned by SHA-256), `wxd_checker` (typing, W0301/W0201/W0202/
  W0602 negatives, discovery, generation still W0701, a mechanical scan of the
  in-repository v1 documents), `wxd_semantics` (every form over every list of
  length 0..=4 on a 3-symbol alphabet and as `Bytes` against native iterators;
  counts -2..2, len +/- 1 and 2^70; 1,000/65,536/65,537 elements; pinned work and
  data; both caps; depth 64/65; short-circuit; two mutant oracles killed) and
  `wxd_alloc_probe` (a counting global allocator: refused requests allocate
  nothing before W0703). `wxh_front` now expects 45 v1 functions.
- **Security review fixes** (VER04k section 9.1, E20 and E21).
  - `dsize` counts the bytes of the names a value stores: `len(T) + len(C)` for
    a constructor (nullary ones included) and `len(E) + len(CASE)` for an error
    value, which also charges its names when it is built. Before,
    `replicate(<10,000-byte constructor>, 65536)` was admitted with 2 MiB
    charged and requested 661,727,185 bytes; it is now refused with W0703
    after requesting 10,193. `wxd_alloc_probe` holds real allocation within 4x
    the charge for replicate, list literals, `concat`, `Err(E.C)` and long type
    names (it was 210x to 316x), and pins the budget boundary.
  - A v1 identifier longer than 128 bytes (`MAX_IDENTIFIER_BYTES`) is W0602,
    `CheckerErrorKind::IdentifierTooLong { len, limit }`, checked in a pass
    over every identifier before any other check. v0 is unchanged.
  - A v1 parameter or quantifier binder named like any reserved built-in is
    W0203 (`ShadowedBinding`), as pattern binders already were; before,
    `model exp(take: List<U8>) -> List<U8> = take(take, 0)` type-checked as
    the built-in and evaluated to the parameter. The evaluator now reads a
    name from the environment only when it is not applied, as the checker
    does, in v0 too (a v0 `len(len)` with a `len: Bytes` parameter is now the
    `Nat` the checker typed instead of the parameter).
  - A v0 evaluation no longer computes data charges it discards.
  - A dotted name is an error case or a projection, nothing else (VER04k
    section 9.1, E21). A reference of more than one segment that is neither
    `E.CASE` (a declared error and case) nor a v1 projection from a binder is
    W0202 `UnboundName` of the whole dotted name, in v0 and v1. Before, the
    checker typed it by its first segment (v1 `g.x(a)`, `g` a model, as `U8`;
    `Some.x(a)`, `None.x`, `len.x(a)`; v0 `a.foo` as the binder `a`) while the
    evaluator made it the error value `Err(("g", "x"))`, so an example
    `select_bytes(..) == m.FORGED(..)` checked and `compile_warrant_to_tfv`
    emitted an error id `FORGED` no declaration has. This is a v0 deviation,
    but only for documents that never meant what they checked as: no
    in-repository document uses such a name, and `v0_compat` and the four
    regeneration tests are byte-identical. The evidence generator already
    refused these names; the lanes now agree.
  - Known open hazard: reads deep-copy and charge no data, so the 64 MiB
    counter bounds each value, not an evaluation's memory (VER04k note on E4,
    backlog WX-RD1).

## 0.4.0 - Warrant v1: WX-H (output bounds) and WX-E (pinned external tables)

Implements VER04j sections 5 and 6 (sections 14.2 and 14.3 list every deviation).
All of it is under the `v1` header; a v0 document means exactly what it did
(`output` in a v0 `bound` and `vectors` in a v0 document are `W0101`), and every
generated harness of `ct-compare`, `gf256` and the earlier v1 fixtures is
byte-identical (the new prelude parts are gated by features a document must use).

- **Parser.** `output` as a bound metric; `vectors NAME:` with the contextual row
  words `source`, `sha256`, `rows`, `column`, `holds` (path alphabet and shape,
  64-hex pin, plain decimal row count, at most 32 unique columns); new
  `Declaration::Vectors`, `VectorsDecl`, `ColumnDecl`, `valid_table_path`,
  `Expression::mentioned_names`, `ParserErrorKind::TableIntegrity` (W0708).
- **Checker.** `output` needs a `Bytes`/`Bytes<N>`/`List<T>` result or a `Result`
  of one (W0301); `imin`/`imax` built-ins (`V1_FUNCTIONS` is now 33 names); tables
  share the test-name namespace, are capped at 100,000 rows each and 250,000 per
  document with saturating arithmetic (W0602), empty or vacuous tables are W0601,
  unused or undecodable columns W0710, shadowing W0203. New `DiagnosticCode::{W0601,
  W0710}`, `CheckerErrorKind::{TableRows, VacuousTable, UnusedColumn,
  UndecodableColumn}`, `MAX_TABLE_ROWS`, `MAX_DOCUMENT_ROWS`.
- **Evaluator / model code.** `imin`, `imax`, and byte-string `concat` in the
  generated model translation.
- **Evidence generator (WX-H).** The allocation observer (counting global
  allocator, per-call window, containment by a parked worker, per-test latch, 64
  workers / 1 GiB process cap), bounds as functions of the call arguments (a
  negative or unevaluable bound fails the instance), the synthesized
  `<op>/output-bound` suites, manifest `instances_observed` / `max_observed_heap` /
  `bound_at_max`, relation bias for a limit parameter, the faults `ignore-limit`,
  `off-by-one-limit`, `limit-after-alloc`, `truncate-silently`, and the composite
  adapter as the fallback shape for other v1 signatures (`Bytes` results,
  `Bytes<N>` results, `fresh`).
- **Evidence generator (WX-E).** `vectors::{load_table, read_fixture, Shape,
  check_value, expected_column, is_constant_output}`, `evidence::{TableSet,
  load_document_tables, compile_rust_evidence_harness_with_tables,
  generate_evidence_harness_with_tables}`, the generated table runner and manifest
  entry, the `table` negative-control kind, the discrimination check, the faults
  `flip-result-bit` and `drop-last-result-byte`, and `gen_evidence` materialising
  and drift-checking `tests/generated/tables/<name>.tfv`.
- **Dependencies.** `coding_adventures_test_bootstrap` and `coding_adventures_sha256`
  are now regular dependencies (the loader hashes and parses fixtures).
  `required_capabilities.json` now declares `fs:read` (it was already read by
  `codegen::verify_regeneration`).
- **Fixtures.** `code/specs/fixtures/warrant-vectors-synthetic-v1/` (synthetic
  messages, hashlib digests, stdlib-only generator with a drift check). The build
  tool maps that directory to this package.
- **Tests.** `wxh_front`, `wxh_generator`, `wxh_evidence` (generated pilot harness,
  observer, every fault, ground-evaluator versus generated model differential),
  `wxh_vulnerable`, `wxh_wasteful`, `wxh_caps`, `wxh_caps_count`, `wxh_regen`,
  `wxe_front`, `wxe_loader`, `wxe_evidence` (including a differential run against
  Python `hashlib`), `wxe_edges`, `wxv_regen`.

## 0.3.0 - Warrant v1: WX-A (tuples, records and non-recursive sum types)

Implements VER04j section 4 (its section 14.1 lists every deviation). All of it
is under the `v1` header; a v0 document means exactly what it did (WX-A syntax in
a v0 document is `W0101`), the checker output and evidence of `ct-compare` and
`gf256` are byte-identical, and a v1 document that uses no composite generates
the WX-B/WX-C harness byte for byte.

- **Parser.** Tuple types and literals (arity 2 to 8), `record` and `variant`
  declarations (32 fields, 256 constructors, duplicates are `W0201`), postfix
  projection `t.0` / `f(x).field` (a `.` with no whitespace around it), recursive
  patterns (`_`, `true`/`false`, tuples, constructors, `E.CASE`, `Some(p)`,
  `Ok(p)`, `Err(p)`, depth 8). `Pattern` is now recursive; new AST nodes
  `Declaration::{Record, Variant}`, `TypeExpr::Tuple`,
  `Expression::{Tuple, Project}`, `ProjKey`.
- **Checker.** `Type::{Tuple, Adt}`, a constructor namespace shared with every
  other module symbol (`W0201`), an acyclic type graph through tuples, records,
  variants, `List`, `Option`, `Result` and aliases (`W0203`), projection (including
  `x.f` with a binder head), constructor typing with the v0 per-argument
  subtyping, invariant composites, pattern typing, and a pattern-matrix
  exhaustiveness and redundancy check (`patterns.rs`, `W0301`, `W0602` on 256
  arms, depth or work exhaustion). `labels op: r.field public` accepts a record
  field path. The v0 `match` check is kept for v0 documents.
- **Evaluator.** `EvalValue::{Tuple, Con}`, construction, projection, general
  matching, structural equality, a composite size bound (`W0703`),
  `encode_portable` (VER04j 4.9, `Result` in the VER04i `M1` form), `encode_checked`
  and `W0709` past 64 containers.
- **Evidence generator.** Composite domains (product, constructors in order,
  `Option`, `Result`, `List`), the byte/case budgets and leftmost-first demotion
  inside composites with component paths, `match`/tuple/constructor/projection
  translation, composite-aware generators, relation bias and shrinkers, composite
  adapters bound through `struct`/`enum` lines and `ret=`/`errors=`, the six
  faults, the unreachable-by-type count, manifest `domain` text. New
  `binding::{HostTy, HostSpec, ...}`, `evidence::EvidenceOptions::hosts`.
- **Tests.** `wxa_syntax`, `wxa_checker`, `wxa_patterns` (matrix versus brute
  force), `wxa_semantics`, `wxa_generator`, `wxa_regen` and `wxa_evidence`
  (generated pilot harness, domains, generators, shrinkers, every fault, and the
  ground-evaluator versus generated-model differential on every composite domain
  within budget). `wx_generator` no longer expects a `match` body to be refused.
- Version 0.3.0.

## 0.2.0 — Warrant v1: WX-B (fixed-width integers) and WX-C (bitwise operations)

Implements VER04j sections 1, 2 and 3 (see its section 14 for every deviation).
A document whose header is `warrant name v1` enables the extensions; a v0
document means exactly what it did before. The layout tokens, parsed
declarations and checker output of `ct-compare.warrant` and `gf256.warrant`
hash to the values the previous version produced (pinned in `tests/v0_compat.rs`),
and `ct-compare`'s and `gf256`'s generated evidence regenerates byte-identically.

- **Grammar, lexer, parser.** `warrant.tokens` gains `U16_LITERAL U32_LITERAL
  I8_LITERAL I16_LITERAL I32_LITERAL I64_LITERAL HEX_LITERAL` and
  `warrant.grammar` the optional second header word and the new literal
  alternatives (additive; `warrant-syntax-v1` untouched and passing). The layout
  lexer reads every suffix and `0x...` as one `Number` token; the parser gates
  them on the header: `W0101` for a v1 construct in a v0 document
  (`ParserErrorKind::V1ConstructInV0`), for a header word other than `v1`
  (`InvalidVersionHeader`) and for an out-of-range suffixed literal
  (`LiteralOutOfRange`). `-128i8` is folded into one literal. New AST pieces:
  `Version`, `WarrantFile::version`, `Literal::Fixed`.
- **`fixed` module (new).** `FixedTy` and the single host-independent definition
  of wrapping arithmetic, checked arithmetic, truncating and floor division,
  `wrap`/`try`/`widen` ranges, every bitwise function, endian conversion and the
  evidence edge domains. Tested against native Rust on every pair of `U8`/`I8`
  and large seeded sweeps of the other six types.
- **Checker.** `Type::{U16,U32,I8,I16,I32,I64}`, version-scoped type and
  built-in names (`widen wrap try to_int to_nat max_value min_value checked_*
  rem div_trunc rem_trunc unwrap_or bit_* shl shr sar rotl rotr popcount clz ctz
  bit be_bytes le_bytes from_be from_le`), no implicit conversion in either
  direction, signed-only unary minus, `div`/`rem` unsigned-or-Int only.
- **Evaluator (`codegen::Evaluator`).** `EvalValue::Fixed`, wrapping `+ - *`,
  every built-in with the specification's total semantics, numeric equality by
  value, bare `None`, `W0703` on `Int` overflow, shifts/popcount charge
  `ceil(w/8)` work. `find_operation_in_expr` now also searches call arguments when
  the shallow search finds nothing.
- **Evidence generator.** v1 documents get a typed `V::Fixed` model in which `U8`
  and `U64` wrap (WD-013 for v1; the v0 harness is unchanged), the domain
  descriptors of VER04j 2.6 (`full` for `U8 I8 U16 I16`, `edge` sets of 281, 313,
  377, 527, 559 and 623 values), the leftmost-first `U16`/`I16` demotion rule with
  `W0706`, the count-aware `Nat` domain, a v1 scalar adapter shape (fixed-width
  parameters; fixed-width, `Bool`, `Nat`, `Option` and `Result` results), fifteen
  new faults (`saturate-*`, `trap-as-zero-*`, `narrow-width`, `sign-confuse`,
  `off-by-one-range`, `shift-mask`, `shift-ub`, `rotate-as-shift-*`, `sar-as-shr`,
  `popcount-off`) and manifest fields `warrant_version` and `demoted`. The
  prelude's `//@if` regions now nest and select `v0` or `v1`.
- **Tests.** `v0_compat`, `wx_syntax`, `wx_checker`, `wx_semantics`, `wx_generator`,
  `wx_evidence` (compiles and runs a checked-in generated v1 harness, with every
  negative control) and `wx_regen` (drift check for it).
- Not delivered: the `endian-swap` fault and `Bytes` adapters, a
  `warrant-syntax-v2` fixture, the independent Python reference evaluator.

## Unreleased

- Security fixes (review of the gf256 / package-agnostic branch):
  - L3: `exhaustive_budget` is capped at 2^26 (`MAX_EXHAUSTIVE_BUDGET`) by the `evidence.bindings` parser, with a clear error above it; gf256's 2^24 is unchanged.
  - L4: `package` and every `export` must be Rust identifiers (also re-checked by the generator, together with error cases and the embedded spec path); `spec` / `output` must be plain relative paths (no absolute path, `..`, backslash, drive letter) and `EvidenceBindingFile::validate_paths` pins them to `code/specs/<pkg>.warrant` and a `.rs` file directly under `code/packages/rust/<pkg>/tests/generated/`; `gen_evidence` validates its `<package>` argument (`[A-Za-z0-9][A-Za-z0-9_-]*`) and calls `validate_paths`.
  - L1 (support): a new per-op option `proofs_not_live=<reason>` (`EvidenceBindingFile::proofs_not_live`) declares that an operation's proof obligations cannot run; `ca-assurance-report` accepts a `not-live` proof entry only for such an operation.
  - Generated harnesses (ct-compare, gf256) are byte-identical.
- Round-2: `resolve_sabotage` rejects a filter unless the fault is a real (non-`none`) one; `run_main` takes the crate's `src/` directory and records the framed `src/**/*.rs` digest as `source_sha256`. Generated harness regenerated.
- Security review fixes (VER04h section 3 and 6):
  - A1: exhaustive-domain size uses checked arithmetic in the generator and in
    the generated prelude; an overflowing product (eight `U8` binders is
    `2^64`, which wrapped to `0`) is over budget.
  - A2: the budget also bounds bytes (`DOMAIN_BYTES_BUDGET`), and fixed widths
    above `MAX_FIXED_WIDTH` (4096), as binders or call widths, fail with the new
    diagnostic `W0707`. The prelude checks the budget before allocating domains.
  - A3: the generated target exits `2` when `WARRANT_EVIDENCE_FILTER` is set
    without `WARRANT_EVIDENCE_FAULT`, or when the filter matches no test
    (`resolve_sabotage`, `require_tests`).

## 0.1.0 — Unreleased

- Evidence generator, second package (gf256, VER04h): the per-package binding
  table is now data. `examples/gen_evidence [--check] [<package> ...]` reads
  `code/packages/rust/<package>/evidence.bindings` (parsed by the new
  `binding::parse_evidence_bindings`, which rejects unknown keys and options so a
  typo cannot weaken evidence) instead of a hardcoded op-to-export table.
  ct-compare's generated harness is byte-identical through its new bindings file.
  - `EvidenceOptions` gains `exhaustive_budget` (default 400,000; gf256 uses 2^24
    for the `U8^3` field-law claims, measured at about 9 s per claim in a debug
    build) and `arg_types` (Rust parameter types of an export, e.g. `power`'s
    `u32` exponent against the Warrant `U64`).
  - New scalar adapter shape for operations over `U8`/`U64`/`Nat` returning `U8` or
    `Result<U8, E>` (a panic is normalised to `E` by the binding, as for
    `ct_select_bytes`), with a closed registry of field-arithmetic faults
    (`or-for-xor`, `wrapping-sub`, `mul-as-add`, `alt-poly`, `generator-shift`,
    `exp-mod-256`, `missing-error`). The generated `Fault` enum lists only the
    faults a package uses.
  - A negative control on a scalar operation attacks that operation alone
    (`attacking`/`fault_for`) and stops at the first claim that catches the fault;
    a fault applied to every operation of a field was invisible to every
    relational claim (lesson recorded).
  - Binder refinements may use models only; an operation inside one is refused
    (W0701). `U8` binders stay inside 0..=255 under relation-biased generation,
    `U64` binders draw small values often, and the fuzz coverage map sees `U8`
    values; these prelude regions are feature-gated so other packages' output
    does not change.
  - Tests: `tests/evidence_bindings.rs` (bindings reader, budget, fault and
    refinement rules, byte-for-byte regeneration of both checked-in harnesses).

- Add `evidence` (VER04h / VER-P030): compile a checked Warrant document into a
  Rust evidence harness (examples, bounded-exhaustive, property, fuzz and
  sabotage suites) whose oracle is the *translated claim body*, not a hand-copied
  `a == b`; supports any number of binders and `Bytes<N>` widths. New diagnostic
  `W0706` (exhaustive domain over budget). `examples/gen_evidence` regenerates or
  `--check`s `ct-compare/tests/generated/warrant_evidence.rs`.
- Stop emitting claim helpers from `compile_rust_l0_harness`: the old helper
  hard-coded `a == b`, accepted at most two binders and was never compiled.
  Removed the tests that string-matched it and the `a == b` vs `a == b`
  "sabotage" emulation; `tests/evidence.rs` compiles and runs the generated
  harness against the real `ct-compare` crate instead.
- Add `ct-compare/warrant` claims for `eq_fixed` (widths 0/1/2/4/16),
  `select_bytes` (pick + length mismatch) and `eq_u64`.

- Add VER04f/P011 Warrant Universal Claims to Property Tests Compiler:
  - Extract and validate universally quantified `claim` declarations (`ClaimCase`) calling bound host operations.
  - Compile claims into Level 2 property-based tests executed on `coding_adventures_test_bootstrap::pbt`.
  - Type-to-generator mappings for `Bytes`, `U64`, `U8`, `Bool` with deterministic PRNG replay and isolated sub-seeds.
  - Integrated shrinking minimizing counterexamples across component values.
  - Public host API call binding dispatch with panic isolation and fault injection.
  - Harness emission of claim sabotage negative controls (`claim_sabotage/{id}_broken_impl`) to detect broken implementations.
  - Privacy-preserving failure output in CI mode redacting raw inputs with FNV-1a reproducer digest.
  - Exceed strict ≥90% line and branch/region test coverage (93.19% lines, 94.51% regions) with zero third-party dependencies.
- Add VER04e/P008 Warrant code generation and L0 test vector compiler:
  - Bounded pure compile-time ground model evaluator with strict recursion depth (<= 64)
    and work quotas (<= 1,000,000 steps).
  - Canonical Portable Test Vector (TFV) compiler transforming Warrant `example` and `claim`
    declarations into structured portable records with logical work accounting.
  - Standalone, deterministic Rust L0 test harness generator executing on `test-bootstrap`,
    calling real public exports, verifying fresh-output non-aliasing, containing and
    normalizing declared panics via `catch_unwind`, and incorporating adapter negative controls.
  - Closed diagnostic error codes (`W0701`..`W0705`) and checked regeneration drift detection.
  - Pilot `code/specs/ct-compare.warrant` canonical specification.
  - Exceed strict ≥90% line and branch/region test coverage with zero third-party dependencies.
- Add VER04d/P007 Warrant type checker and Rust binding schema: static type checking,
  scope/symbol resolution, cycle detection on type aliases and pure model functions,
  minimal subtyping (`Nat <: Int`, `Bytes<N> <: Bytes`), exhaustive match verification,
  and contract validation (`labels`, `bound`, `input`, `assume`).
- Enforce closed diagnostic codes (`W0201`, `W0202`, `W0203`, `W0301`, `W0401`, `W0501`, `W0502`, `W0602`)
  and opaque error messages with zero input secret leakage.
- Add closed host binding schema (`BindingManifest`, `OpBinding`) validating complete operation
  coverage, fresh requirements, negative controls, and error normalizations.
- Achieve ≥90% line and branch/region test coverage with zero third-party dependencies.
- Add VER04c/P006b3 guarded Warrant source parser: typed AST construction conforming
  to `warrant.grammar`, strict recursion depth guard (`max_depth: 64`) preventing
  call-stack exhaustion (CWE-674), AST node quota (`max_nodes: 65,536`), and work limits.
- Enforce closed `W0001` diagnostics and verify zero source leakage in error Display/Debug.
- Validate all 57 canonical syntax cases from `cases.tfv` through admitted document,
  layout capture, and guarded AST parser.
- Add VER04b/P006b2 bounded significant-layout capture: space-only indentation,
  indentation stack tracking with configurable limits, tabs-in-indentation rejection,
  and exact zero-width synthetic token origins (INDENT/DEDENT/NEWLINE) without
  fictitious positive-width byte spans.
- Add VER04a/P006b1 source admission: bounded UTF-8/BOM/NUL/CR validation,
  borrowed LF views, fallible CRLF normalization and original-byte/one-based
  line/byte-column mapping through a source-bound line-start index.
- Enforce lowerable byte/capacity/work caps before scans/reservations; use
  closed W0001 errors and suppress source in error/document Debug output.
- Test authored Unicode/CRLF ranges, exhaustive small scalar ranges, malformed
  encodings, source/buffer/work boundaries and opaque diagnostics. Route the
  source contract to the Warrant consumer in the build tool.
- Add canonical significant-indentation Warrant token/EBNF grammars and
  deterministically generated Rust TokenGrammar/ParserGrammar constructors.
- Exercise 57 authored syntax cases through TFV/L0, exact subtree witnesses and
  dynamic/embedded grammar agreement. Add three adapter controls and a
  separately falsified precedence witness, source-shape guards and opaque errors.
- Check exact generated bytes and wire grammar/corpus/reference changes into
  repository input selection plus platform BUILD commands.
- Keep significant-layout origin capture, the guarded parser and its remaining
  quotas, typing/binding validation, proof and publishing work deferred.
