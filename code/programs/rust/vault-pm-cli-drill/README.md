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
| every landing point | generation zero (34), the shared mutation publication path (20), portable export, passphrase rotation |
| first / middle / last | item create, item edit, item delete, history restore |
| first / last | a fail-closed conflict merge |
| every stage | the read-only diagnostics drill, including file-level backup restore |

## What the drill found

Two results are worth reading before trusting the product with anything.

An interrupted `init` is always repairable by running `init` again, and the
resumed vault passes authenticated `doctor --unlock`.

An interrupted **mutation** used to be unrepairable from the command surface,
and that finding is why this crate exists. The tree was never torn and the
durable `PendingPublication` journal was exact — but nothing replayed it, so
every later command failed, as exit 2 `vault-pm: invalid command`, telling a
person their command was wrong about a vault that was intact and one journal
replay from healthy. See VLT-PM41 section 8 and VLT-PM00 §23 item 10a.

`VLT-PM42-cli-pending-publication-recovery.md` repaired it, and the assertions
this drill had pinned were rewritten to require the opposite rather than
deleted. Section 3 of the matrix now proves that every landing point of the
publication path is finished by the next ordinary command, and section 6 proves
what a landing-point count cannot: that the write which was interrupted is the
write that comes back. An `item add` killed after its journal lands is a listed
item afterwards, with its title and username readable, and it is the only one.

The read-only diagnostics are still not repairs. `status` and `doctor` — with
or without `--unlock` — report a wedged vault and leave it wedged, however many
times they are run, which is what keeps restoring a pre-mutation file-level
backup a real option rather than a race.

## The attachment drills

Attaching a file is the ceremony that publishes the most objects in this
product — a three-chunk attachment adds four content objects on top of the
revision, catalog, and audit event — which makes it the one where "a crash
leaves the vault either untouched or one command from healthy" is least
obviously true. `an_interrupted_attachment_add_is_clean_or_resumable` kills a
real process at its characteristic landing points and then proves the vault
still attaches and still returns the bytes identically, which a torn write
would have destroyed silently rather than loudly.

`an_interrupted_attachment_export_never_leaves_a_partial_plaintext` covers the
one durable write that leaves the storage backend. It locates the two
`attachment.artifact` landing points **by name** in the ledger rather than by
ordinal, because that ordinal depends on how many objects the preceding audit
publication wrote, and pinning it as a number would make the test a statement
about arithmetic. Afterwards the destination must be either absent or the
complete plaintext; a file that exists and is neither is the torn class this
matrix forbids.

## The asymmetric ceremony

Passphrase rotation gets a stronger property than "clean or resumable", because
it is the one ceremony whose two failure modes are *different kinds of wrong*.
A rotation moves a single durable fact — which signed bootstrap the owner state
accepts — across two independent stores, and the pin is checked absolutely on
every open. So
`every_passphrase_rotation_landing_point_leaves_exactly_one_working_passphrase`
requires precisely that at every landing point:

- **both passphrases working** would mean the retired wrap survived the
  rotation that existed to retire it — a security failure that a "the vault
  still opens" assertion would happily call success; and
- **neither working** would mean the vault was bricked — an availability
  failure of the exact shape VLT-PM41 section 8 already found once.

Both are explicit panics naming which of the two happened, rather than one
generic assertion, so a regression says what it broke. Each cell then confirms
that the vault behind the surviving passphrase is the whole vault, by requiring
the fixture's item still to be listed. See
`code/specs/VLT-PM43-cli-passphrase-rotation.md` §7 gate 5.

That sweep builds a private vault per cell and runs under `sweep`, unlike the
snapshot-restoring sweeps around it. The reason is cost. A rotation has 48
landing points and each cell pays up to five production Argon2id derivations in
a debug build, so serially it is ~240 KDF runs — enough to make this package the
slowest unit in the repository's CI. A snapshot cannot be restored into a
*different* path (the client configuration records the resolved object root),
so the only way to parallelize is to let each cell build its own fixture. That
costs a little more total work and buys back the worker count in wall clock:
508s to 117s locally, coverage unchanged.

## The KDF cost is overhead, not coverage, and now runs at test cost

Parallelizing bought back the worker count, but the KDF itself — 64 MiB, 3
iterations, in a debug build — was still real work every worker paid, on every
cell, of every sweep in this file, not just the rotation. None of that cost is
what a crash-injection cell proves: a landing point's clean-or-resumable
classification is a fact about `write -> fsync -> rename` ordering, never
about how expensive the KDF that ran earlier in the same process happened to
be. Backlog item #20 had once declined the obvious next lever — cutting the
rotation sweep's 48 landing points to a representative subset by equivalence
class — because that *is* a real coverage reduction, and it still is. It was
not needed here: the actual bottleneck was the KDF cost, not the landing-point
count, and that cost was separable from what the sweep proves.

Every process this file drives now carries
`VAULT_PM_DRILL_KDF_{MEMORY_KIB,ITERATIONS,LANES}` (set once, in
`TestHome::configure`), which `coding_adventures_vault_pm_cli`'s
`crash-injection` build reads in place of its production Argon2id policy — see
`crash.rs`'s `kdf_policy_override`. The values (`8 * 1024` KiB, 1 iteration, 1
lane) are not a weaker policy invented for this purpose; they are the same
minimal, still bound-valid Argon2id parameters this repository's own
`vault-pm-cli` unit tests already use for KDF-adjacent assertions that do not
care about strength. Reading the override only when `crash-injection` is
compiled in keeps it out of the shipped `vault-pm` the same way the
landing-point instrumentation itself is kept out — by construction, not by
convention. See `code/specs/VLT-PM41-cli-crash-fault-matrix.md` §8.1 for the
full argument.

Every landing point of every sweep is still swept — the rotation's 48, the
generation-zero sweep's 34, the shared mutation-publication path's 20, every
other ceremony — through a real `SIGKILL`ed process, with every assertion
unchanged. Measured on one development machine, this file's whole `cargo test`
run: **266.92s to 38.92s**. (Local timing is a proxy, not the ground truth —
see this campaign's standing lesson on CI-vs-local timing drift — so the PR
that lands this also records the real CI run's number.)

## Verification

```bash
bash BUILD
cargo clippy --manifest-path Cargo.toml --all-targets -- -D warnings
```
