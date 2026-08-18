# `vault-pm-drill`

The instrumented twin of `vault-pm`, and the VLT-PM41 crash/fault drill that
uses it. Specified by
`code/specs/VLT-PM41-cli-crash-fault-matrix.md`.

This is **not a product executable**. It is never installed, and its binary is
deliberately named `vault-pm-drill` so it cannot be mistaken for one.

## Why it is a separate crate

`code/programs/rust/vault-pm-cli/tests/local_cli_e2e.rs` proves what the real
executable does when it is allowed to finish. Proving what happens when it is
*not* needs a binary that can be killed at a chosen durable write — which means
a binary built with `coding_adventures_vault_pm_cli`'s `crash-injection`
feature.

The obvious way to get one is to enable that feature through the product
crate's own `dev-dependencies`, and it *almost* works: `cargo build` and
`cargo install` never resolve dev-dependencies, so they produce an
uninstrumented binary. But Cargo resolves features per package across a build
graph, and `cargo build --all-targets` does pull dev-dependencies in. Cargo
then uplifts the feature-unified binary to `target/release/vault-pm` — the
exact path a packaging step copies from. A password manager would ship an
environment-variable kill switch that fires between durable writes, and whether
it did would depend on which cargo command ran last.

So the product crate never names the feature in any section, and this crate
exists to hold it. The two are separate cargo workspaces, so their feature
resolution never meets.

Naming no feature is necessary and not sufficient, because
`--features <dep>/<feature>` reaches a direct dependency's features regardless
of what the root package declares. The product's `main.rs` therefore also
carries a `const` assertion on `CRASH_INJECTION_COMPILED`, so a `vault-pm` with
the instrumentation in it does not compile; and `local_cli_e2e.rs` carries a
guard rail, `the_shipped_executable_contains_no_crash_injection`, that reads
the product binary and fails if either injection variable name appears in it.

The twin also declares a larger capability profile than the product: it adds
`env: read` for the two injection variables, `proc: signal` for removing its
own process, and `fs: create` plus `fs: write` for the durable-step ledger.
That last pair matters — the ledger path is not confined to the vault roots, so
any absolute path naming an existing private regular file this user owns is
accepted. The extra authority is visible in a manifest rather than implied.

`src/main.rs` is twelve lines copied from the product's. The duplication is the
price of the guarantee.

## The drill

`tests/crash_fault_matrix.rs` kills a real process with `SIGKILL` at a
deterministically chosen durable write and then asks the *next* real process
what it can see and what it can repair. Nothing calls a recovery function
directly; the only interface used is the one a person has — an argument vector,
a controlling terminal, and a directory tree.

Landing points come from `coding_adventures_vault_pm_crash_injection`, which
brackets every durable write in a "before" and an "after" ordinal. Because each
durable write is an atomic `write → fsync → rename`, those ordinals are not a
sample of where a crash can land: they are the complete case analysis. A drill
therefore runs a ceremony once with only a ledger, counts the lines to learn how
many landing points it has, and replays it once per point from a byte-identical
tree — so the matrix is derived from the code under test rather than from a list
somebody has to remember to update.

Each cell must land in one of two acceptable classes — **clean rollback** or
**crash-resumable** — and never in the forbidden third, **torn**. Every cell
also asserts that no drill secret is readable anywhere under the platform home
and that the advisory writer lock was released by the kernel.

| Coverage | Ceremony |
|---|---|
| every landing point | generation zero (34), the shared mutation publication path (20), portable export |
| first / middle / last | item create, item edit, item delete, history restore |
| first / last | a fail-closed conflict merge |
| every stage | the read-only diagnostics drill, including file-level backup restore |

## What the drill found

Two results are worth reading before trusting the product with anything.

An interrupted `init` is always repairable by running `init` again, and the
resumed vault passes authenticated `doctor --unlock`.

An interrupted **mutation** is not repairable from the command surface. The
tree is never torn and the durable `PendingPublication` journal is exact — but
no verb replays it, so every later command fails, and it fails as exit 2
`vault-pm: invalid command`, telling a person their command is wrong about a
vault that is intact and one journal replay from healthy. See VLT-PM41 section
8 and VLT-PM00 §23 item 10a. The drill pins that behavior with a comment
directing the fixing slice to rewrite those assertions rather than delete them.

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```

Each cell pays a production Argon2id derivation per unlock, so the
generation-zero sweep runs its cells across worker threads. The whole file is
roughly two minutes.
