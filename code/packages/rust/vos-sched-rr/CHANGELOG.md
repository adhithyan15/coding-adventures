# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

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
