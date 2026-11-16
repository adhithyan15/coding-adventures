# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added

- VOS-010: the scheduler interface contract (`code/specs/VOS10-scheduler-contract.md`).
  - `trait Sched` (`State`, `N`, `init`, `insert`, `remove`, `dispatch`), `Upd` and
    `Pick`; `no_std`, `forbid(unsafe_code)`, no dependency.
  - `warrant/contract.part`: the abstract state (ready set over 8 thread ids,
    priority = id), total abstract operations, `inv`, the safety and policy
    relations, `pre_` models and nine model-level lemmas.
  - `warrant/obligations-{safety,policy,canonical,any}.part`: 4 + 1 + 7 + 3 claims (`safety_alpha_build` is a safety claim) in
    the fixed vocabulary of VOS02 R3, interim projection form (VOS02 section 4.4).
  - `contract.vos`: ceilings (`16 N + 32` steps, heap 0, depth 2; `init` `8 N + 16`)
    and the instance `N 8`.
  - `vos.toml` (class K, kind `contract`), `required_capabilities.json` (empty),
    `tests/trait_usable.rs`.
