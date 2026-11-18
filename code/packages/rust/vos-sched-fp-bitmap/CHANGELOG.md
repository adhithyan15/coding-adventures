# Changelog

All notable changes to this package are documented here.

## Unreleased — regenerated evidence harness (VOS-038b)

- `tests/generated/warrant_evidence.rs` regenerated with `gen_evidence` for the observer timeout clamp (`WARRANT_OBS_TIMEOUT_SECS` at most 24 h, `checked_add` deadline; VER04j). No test, case count or recorded evidence manifest changes.

## [0.1.0] - Unreleased

### Changed (VOS-038a, WX-SV2)

- `init` is bound with `field-flip` (never inert on the zero state) and `alpha_w` with
  `field-zero,field-flip`; the harness is regenerated. The three `init` controls now pass
  (`canon_init` catches a mask of 1), so `warrant/check_evidence.sh` is **deleted** and BUILD runs
  the suite plainly: 71 ok, exit 0. New sabotage lines: `canon_init`, `safety_init` (flip of `init`
  and of `alpha_w`), `canon_{insert,remove,dispatch}_post` (flip of `alpha_w`).

### Changed (VOS-D081)

- Regenerated for the contract's new `safety_init` claim: `code/specs/vos-sched-fp-bitmap.warrant`, the
  generated harness and the manifest. 13 exhaustive claims, 394,500 cases; `warrant/check_evidence.sh`
  now pins 64 ok (was 61) next to the same three known-inert `init` controls. A mask-8 `init` fails
  `canon_init` and `safety_init` (checked in a scratch copy). No sabotage line for `safety_init`: its
  only fault, `field-zero` on `alpha_w`, is inert on the empty state.

### Added

- VOS-011: the fixed-priority bitmap scheduler (`code/specs/VOS10-scheduler-contract.md`
  section 11.1). `impl Sched for FpBitmap` at `N = 8`: a one-byte ready set, `insert` and
  `remove` as a checked-shift set and clear, `dispatch` as the highest set bit found by a
  three-stage branch-free `clz8`, cleared. `no_std`, `forbid(unsafe_code)`, no loop, no
  division, no panic path, `clippy::arithmetic_side_effects` denied.
- The ghost face (VOS02 section 4): feature `ghost`, a public-field `Image`, `AbsW`, `CfgN`,
  and `vos_ghost_{init,insert_out,insert_img,remove_out,remove_img,dispatch_out,dispatch_img,alpha_w,cfg_n}`
  re-exported at the crate root.
- `vos.toml` (class K, kind `production`, contract `vos-sched-contract`, obligations
  canonical, policy and safety), `warrant/impl.part`, `required_capabilities.json` (empty).
- `code/specs/vos-sched-fp-bitmap.warrant`: the first document produced by `vos-link gen-doc`
  on a real implementation; `vos-link check-doc` is in BUILD.
- `tests/behaviour.rs`: every state and every id against a plain reference model, with and
  without the ghost face; 100% line coverage measured once by hand (not part of BUILD).

- The VER04h evidence: `evidence.bindings` (every operation bound to a `vos_ghost_` wrapper),
  `tests/generated/warrant_evidence.rs` and its shim, `evidence/test-manifest.json`, the
  regeneration check and sabotage lines in BUILD. 12 exhaustive claims, 394,498 cases, all
  at or below 65,536; the `init` / `field-zero` controls are known-inert (VOS-D065) and named
  in `warrant/check_evidence.sh`.
- `heap <= 0` rows only on `insert_out` and `remove_out` (VOS-D063).

### Changed

- WD-HARN1b: `tests/generated/warrant_evidence.rs` regenerated with `gen_evidence` for the WD-HARN1b
  changes to the obs region of the Warrant prelude (`Bail out!` is written with `writeln!` to
  stderr, the dead latch is gone). No hand edit; `evidence/` is unchanged and
  `warrant/check_evidence.sh` reports the same result as on main.
- The sabotage lines in `BUILD` and `BUILD_windows` now fail when the sabotaged run prints
  `Bail out!`, not only when it lacks a `not ok ` line (a hang after an intended failure no longer
  passes silently).
