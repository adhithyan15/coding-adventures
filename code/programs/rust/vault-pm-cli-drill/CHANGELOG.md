# Changelog

All notable changes to this package are documented here.

## [0.1.0] - 2026-08-18

### Added

- `vault-pm-drill`, the instrumented twin of the `vault-pm` executable. It is
  the same composition over the same library, differing only in that it enables
  `coding_adventures_vault_pm_cli`'s `crash-injection` feature as an ordinary
  dependency feature. It is not a product executable and is never installed.
- The separation itself, which is the point of the crate. Cargo resolves
  features per package across a build graph, so enabling the feature through
  the product crate's `dev-dependencies` would let `cargo build --all-targets`
  uplift an instrumented binary to `target/release/vault-pm` — the path a
  packaging step copies from. The product crate therefore names the feature in
  no section, its `main.rs` fails to compile when the feature is active (naming
  no feature is not enough on its own, since `--features <dep>/<feature>`
  reaches a direct dependency's features regardless), and its own suite carries
  a guard rail that reads the binary and fails if either injection variable
  name appears in it.
- A capability manifest declaring the authorities the twin has and the product
  does not: `env: read` for `VAULT_PM_CRASH_AT` and `VAULT_PM_CRASH_TRACE`,
  `proc: signal` for removing its own process, and `fs: create` plus
  `fs: write` for the metadata-only durable-step ledger. The ledger path is not
  confined to the vault roots, so the write authority is declared rather than
  argued away.
- `tests/crash_fault_matrix.rs`, the VLT-PM41 crash/fault matrix and local
  restore drill: twelve tests that kill a real process with `SIGKILL` at a
  deterministically chosen durable write and then ask the next real process
  what it can see and what it can repair, through nothing but an argument
  vector, a controlling terminal, and a directory tree. Generation zero (34
  landing points) and the shared mutation publication path (20) are swept
  exhaustively; item create, edit, delete, history restore, a fail-closed
  conflict merge, and portable export are probed at their characteristic
  points; and a separate drill walks one vault through every stage of
  interruption, pinning what `status` and `doctor` report at each and proving
  that a pre-mutation tree restored from an ordinary file-level backup opens
  and verifies. Each ceremony's landing-point count is measured from a ledger
  the run itself emits, so the matrix is derived from the code under test
  rather than from a list somebody has to remember to update.
- A bounded prompt wait. Every read is `poll`ed with a deadline, and a wait
  that expires kills the child and fails the test, so a hang in the product can
  never present as a hung suite.
