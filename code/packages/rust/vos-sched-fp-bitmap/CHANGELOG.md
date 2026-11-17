# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

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

### Fixed

- `tests/generated/warrant_evidence.rs` regenerated with `gen_evidence` for the WD-HARN1
  evidence prelude (bounded observer wait, `lock_or_recover`, `Bail out!` exit 70). It was
  generated while that prelude change was in CI, so `gen_evidence --check` failed on main.
  No hand edit; `evidence/` is unchanged and `warrant/check_evidence.sh` reports the same
  result (cargo status 1, 61 ok, the three known `init`/`field-zero` controls).
