# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Changed (VOS-012a PR, VOS-D054, VOS-D079)

- `warrant/obligations-any.part`: `dispatch_covers_run` lost its `| t < 8u8` refinement. `mem` is
  false outside `0..7`, so the claim means the same, and the generated property suite can now run
  (the generator's rejection sampler gave up at 3 percent acceptance). Its enumerated cases are
  65,536 (were 2,048). Only the `any` tier's digests change; the safety, policy and canonical
  claims, the contract block and `vos-sched-fp-bitmap`'s document are unchanged.

### Changed (VOS-011 PR, VOS-D064)

- The nine model-level lemmas moved from `warrant/contract.part` to the new
  `warrant/lemmas.part`, which no implementation document embeds: the evidence generator
  refuses a claim that calls no operation (W0701). The contract-block digest changed
  (`1cafa96a...` to `fd4b88aa...`); every obligation's `claim_sha256` and every obligations
  block digest is unchanged. `vos-sched-contract-check` evaluates the lemmas from the new part.

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
