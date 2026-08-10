# `coding_adventures_vault_pm_repository`

The VLT-PM04 storage-agnostic immutable repository for the local-first password
manager. It publishes opaque object frames in dependency order, verifies every
read and signed commit/announcement through a mandatory injected verifier,
reconstructs the commit DAG, enforces local head pins, provides deterministic
bounded interactive history plus complete graph-bounded audit history, and
produces conservative plan-only garbage-collection reports.

The crate owns no filesystem path, provider client, key, entropy source, clock,
or plaintext codec. Those authorities stay in the host and unlocked
application composition.

## Verification

The package has 15 tests covering address vectors, ordered publication,
ambiguous-write retry, restart/open, branches and merges, pinned-head
withholding, direct and interleaved counter equivocation, iterative cycle
detection, malformed provider responses, explicit verified reads,
deterministic bounded/complete history, and plan-only GC. Tarpaulin's LLVM
engine measures 487 of 507 production lines covered (96.05%).

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_repository --all-targets -- -D warnings
```
