# Changelog

All notable changes to this package are documented here.

## Unreleased — VOS-D098 closed: every control passes

- An operation's negative control is now attacked by every claim that calls it, not only those
  whose subject it is, so `safety_init` counts for `init` and the three `init`/`field-flip`
  controls pass. The `excused-controls` in `vos.toml` and the generated
  `warrant/check_evidence.sh` are deleted; BUILD runs the suite plainly (86 ok, exit 0).

## Unreleased — the component check and the coverage gate (VOS-039a)

- `BUILD` and `BUILD_windows` are now **generated** by `vos-link gen-build` (VOS04 5.6) from `vos.toml`, `evidence.bindings`, the document and the new `warrant/sabotage.plan`, and `vos-link check-component` requires them byte for byte (a text lint over BUILD was bypassable by single-line tricks). `vos.toml` declares `coverage-floor = 95`, enforced by a Linux `cargo tarpaulin ... --fail-under 95` line (VOS-D070). Every sabotage line now runs through the shared checker `code/scripts/warrant_negctl.py` (VOS-D099, closing VOS-D088); no claim or fault lost its line (`sabotage.plan` is the old lines, one entry each). Measured line coverage of `src/lib.rs`: 100%. The three controls rr cannot pass (`init`/`field-flip`, VOS-D098) are declared as `excused-controls` in `vos.toml`; `warrant/check_evidence.sh` is generated from them.

## Heap rows restored (VOS-038c)

- `heap <= 0` is declared on every bound operation again (VOS-D063): the harness's heap observer now
  counts the export call only, not its own result marshalling. `warrant/impl.part`, the generated
  `code/specs/vos-sched-rr.warrant`, `tests/generated/warrant_evidence.rs` and `evidence/test-manifest.json`
  regenerated; every operation records `max_observed_heap 0`.

## Unreleased — regenerated evidence harness (VOS-038b)

- `tests/generated/warrant_evidence.rs` regenerated with `gen_evidence` for the observer timeout clamp (`WARRANT_OBS_TIMEOUT_SECS` at most 24 h, `checked_add` deadline; VER04j). No test, case count or recorded evidence manifest changes.

## [0.1.0] - Unreleased

### Changed (VOS-038a, WX-SV2)

- `init` is bound with `field-flip`, `alpha_w` with `field-zero,field-flip`; harness regenerated.
  `warrant/check_evidence.sh` is **kept, narrowed**: exactly the three `init`/`field-flip` controls
  still cannot pass (their only member `rep_init` cannot see a flip that stays in the image of
  `rep`), 78 ok (VOS-D087, VOS-D098). New sabotage lines: `safety_init` (flip of `init`, of
  `alpha_w`), `rep_init` and `dispatch_rep_post` (flip of `alpha_w`), which had no line before.

### Added

- VOS-012b: the round-robin scheduler (`code/specs/VOS10-scheduler-contract.md` section 11.2), the
  first swap of the verified-OS program. `impl Sched for RoundRobin` at `N = 8`: state `Rr { mask,
  ptr }`, `insert` and `remove` as a checked-shift set and clear that never move `ptr`, `dispatch` as
  a loop-free rotate-and-lowest-bit scan from `ptr` that clears the chosen bit and sets
  `ptr = (t + 1) & 7`. `no_std`, `forbid(unsafe_code)`, no loop, no division, no panic path.
- Class K, kind `production`, `obligations = ["representation", "safety"]`: the contract's
  representation tier (VOS02 section 2.7) because `alpha` forgets `ptr`; no `canonical` and no
  `policy` tier (round-robin is not highest-priority-first).
- `warrant/impl.part`: `rep`, `hist`, the op-free `rr_pick`/`rr_next`, and four ungated impl-own
  claims: `dispatch_det_out`, `dispatch_det_img` (dispatch against the op-free model, split per
  projection so each projection has a non-inert negative control) and `pointer_kept` (insert and
  remove never move the pointer, which the representation claims cannot see); four worked
  examples over non-zero states.
- The generated document `code/specs/vos-sched-rr.warrant` and the VER04h harness: 16 exhaustive
  claims (the 13 contract claims of the two tiers plus the 3 own ones), 74 ok, 3 known-inert
  `init`/`field-zero` controls (VOS-D065) named by `warrant/check_evidence.sh`.
- 12 behaviour tests (14 with `ghost`) against a plain scan reference over every reachable state,
  including fairness, wraparound, out-of-range ids, idle and a mutation sanity check.
- BUILD and BUILD_windows: tests with and without `ghost`, shape check, capability analyzer,
  `check-doc`, the generated-harness drift check, the evidence run and 25 sabotage lines.

### Fixed

- VOS-D090: regenerated `tests/generated/warrant_evidence.rs` for the WD-HARN1b observer prelude (the
  `gen_evidence --check` drift introduced when #19550 and this crate merged in parallel) and moved all
  25 BUILD and BUILD_windows sabotage lines to the bail-out-aware tail (`not ok ` seen AND no
  `Bail out!`), the one `vos-sched-fp-bitmap` uses.
