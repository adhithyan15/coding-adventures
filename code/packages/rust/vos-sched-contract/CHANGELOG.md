# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-012b0, VOS-D061)

- `tier representation` in `contract.vos` and `warrant/obligations-representation.part`: eight
  claims (`rep_init`, `rep_alpha`, `insert_rep`, `insert_rep_post`, `remove_rep`, `remove_rep_post`,
  `dispatch_rep`, `dispatch_rep_post`) over an implementation-supplied `rep(m: Ready, h: U8) -> Impl`
  and `hist(s: Impl) -> U8`, with one packed `k: U8` binder (history in its low three bits, the
  thread argument `shr(k, 3)` above them; 65,536 cases each).  The contract text names no field of
  `Impl`.  `contract.part`, and so every existing document and digest, is unchanged.

### Added (VOS-D081)

- `warrant/obligations-safety.part`: `safety_init` (`forall b: Bool. dec(alpha_w(init())) == empty()`), the
  fifth safety claim. Before it nothing constrained `init` in the safety tier (`canon_init` is
  canonical-only), so an `-any` or round-robin scheduler could start from a non-empty ready set
  and pass every generated test. The safety block's digest and every implementation's embedded
  safety block change; the contract block, the policy, canonical and `any` blocks and every other
  claim's `claim_sha256` are unchanged.

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
  - `warrant/obligations-{safety,policy,canonical,any}.part`: 4 + 1 + 7 + 3 claims at VOS-010 (`safety_alpha_build` is a safety claim; `safety_init` made it 5 + 1 + 7 + 3, VOS-D081) in
    the fixed vocabulary of VOS02 R3, interim projection form (VOS02 section 4.4).
  - `contract.vos`: ceilings (`16 N + 32` steps, heap 0, depth 2; `init` `8 N + 16`)
    and the instance `N 8`.
  - `vos.toml` (class K, kind `contract`), `required_capabilities.json` (empty),
    `tests/trait_usable.rs`.
