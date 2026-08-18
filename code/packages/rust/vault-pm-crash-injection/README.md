# `coding_adventures_vault_pm_crash_injection`

Deterministic real-process crash injection for local `vault-pm` hosts,
specified by `code/specs/VLT-PM41-cli-crash-fault-matrix.md`.

This package is **test scaffolding**. It ships no product behavior, and a
released `vault-pm` executable contains none of its code.

## The problem it solves

`vault-pm` claims that a power cut can land anywhere inside a write and still
leave a vault a person can open. VLT-PM05 §7 states the claim as a state
machine: journal first, publish second, advance local state third, and any
interruption leaves either the old state or a resumable journal.

Until this package, that claim was checked *in process*. A unit test built the
application over an in-memory store, made a store call fail, and then called the
recovery function itself. That proves the recovery *logic*. It does not prove
that a real operating-system process, killed by a signal it cannot catch,
leaves a real directory tree that the next real process can open. Only the
second statement is what a user experiences when a laptop battery dies.

## The model

Everything a local host makes durable passes through a small number of gates:

```text
  application owner state  ─┐
  bootstrap generations    ─┤
  immutable object frames  ─┼─→ storage_core::StorageBackend
  signed announcements     ─┤
  audit-chain event frames ─┘

  client configuration     ───→ LocalWriterGuard create / compare-exchange
  portable export artifact ───→ CliHost::write_portable_export
```

Each of those is an atomic `write → fsync → rename`. `rename(2)` has no middle,
so the on-disk state after a crash is entirely determined by *how many* durable
writes completed. That converts an uncountable set of crash instants into a
finite, ordered set of landing points:

```text
  step 1   before durable write #1
  step 2   after  durable write #1
  step 3   before durable write #2
  …
```

An operation performing `n` durable writes has exactly `2n` landing points, and
every possible crash of it is equivalent to landing on one of them. Enumerating
them is a complete case analysis, not a sample.

## Surface

- `DurableStep` — the closed ledger vocabulary: `storage.initialize`,
  `storage.put`, `storage.delete`, `storage.lease`, `config.create`,
  `config.replace`, `export.artifact`.
- `Phase` — `before` / `after`.
- `record(step, phase) -> u64` — consume the next process-global ordinal,
  append it to the ledger if one is configured, and remove this process if it
  is the chosen one.
- `around(step, action)` — bracket one durable write in its two landing points.
- `ledger_line(ordinal, step, phase)` — the exact wire format a drill parses.
- `CrashInjectingStorageBackend<B>` — a `StorageBackend` decorator that brackets
  `initialize`, `put`, `delete`, and `acquire_lease`. Reads pass through
  untouched and consume no ordinal: a crash during a read changes nothing on
  disk, so it collapses into the "before" point of the next write.

Two environment variables drive one process:

| Variable | Meaning |
|---|---|
| `VAULT_PM_CRASH_TRACE` | append the durable-step ledger to this path |
| `VAULT_PM_CRASH_AT` | remove this process when that ordinal is reached |

A `VAULT_PM_CRASH_AT` that is not a positive decimal integer is a hard error. A
typo must not silently turn a crash drill into an ordinary successful run.

## Why `SIGKILL`

`std::process::exit` runs `atexit` handlers and flushes standard output.
`std::process::abort` raises `SIGABRT`, which a process can install a handler
for and which asks macOS to write a crash report. Neither models a power cut.
`SIGKILL` cannot be caught, blocked, or handled: the kernel removes the process
immediately, closing its descriptors and dropping its advisory locks without
running one instruction of cleanup.

It is also *provable*. A killed child reports a termination signal instead of
an exit code, so a drill that expected a crash and got a clean exit fails
loudly rather than quietly measuring nothing.

## What the ledger may contain

An ordinal, a phase, and a name from the closed vocabulary. Never a key,
namespace, object identifier, path, item title, ciphertext, or length, so no
vault *content* can reach it.

Be precise about what that does and does not hide. Each object write emits one
`storage.put` pair, so the ledger's *length* correlates with how much a
ceremony wrote — it is a shape and activity oracle, not a content oracle. Two
vaults running the same ceremony over the same number of objects produce
identical ledgers; a larger vault produces a longer one. That is acceptable
because the ledger exists only when a drill names it, and counting those writes
is the drill's whole purpose.

The path is treated as untrusted even though only something that already
controls this process's environment can set it. It must be absolute. It is
opened with `O_NOFOLLOW | O_NONBLOCK`: the first refuses a symlink at the final
component, the second refuses a reader-less FIFO, which `O_NOFOLLOW` says
nothing about and which would otherwise block the open forever. It is created
`0600`, and is refused outright if it already exists as anything other than a
regular file this user owns privately — a creation mode says nothing about a
file that is already there. Ownership is checked by `fstat` on the open
descriptor rather than by a second path lookup, so nothing can be swapped in
between the check and the write. The executable never reads the ledger back.

What none of that does is confine *where* the ledger may be. Any absolute path
naming an existing private regular file this user owns is accepted, including
one inside their own vault state. That authority is declared in the drill
crate's capability manifest rather than argued away.

## How it stays out of a release

The package is an *optional* dependency of `coding_adventures_vault_pm_cli`
behind that crate's non-default `crash-injection` feature.

The tempting way to reach it is to enable the feature through the product
executable's own `dev-dependencies`: `cargo build` and `cargo install` never
resolve dev-dependencies, so they would produce an uninstrumented binary. That
is a trap. Cargo resolves features per *package* across a build graph, and
`cargo build --all-targets` does pull dev-dependencies in; cargo then uplifts
the feature-unified binary to `target/release/vault-pm`, the exact path a
packaging step copies from. Whether a password manager shipped with a kill
switch would depend on which cargo command ran last.

So the product crate `code/programs/rust/vault-pm-cli` names the feature in no
section at all, and the instrumented twin lives in its own crate and its own
workspace — `code/programs/rust/vault-pm-cli-drill`, producing a binary called
`vault-pm-drill`.

Naming no feature is necessary and not sufficient. Cargo's
`--features <dep>/<feature>` syntax reaches a direct dependency's features even
when the root package declares none of its own, so the product's `main.rs` also
carries `const _: () = assert!(!CRASH_INJECTION_COMPILED);` and a build with
the feature active is a *compile error* rather than a binary somebody has to
remember to inspect. `the_shipped_executable_contains_no_crash_injection` in
the product crate's own suite then reads the binary that crate produced and
fails if either variable name appears in it — running, deliberately, in a build
that *does* have dev-dependencies resolved.

## Relationship to `FaultInjectingObjectStore`

`vault-pm-storage`'s `FaultInjectingObjectStore` (VLT-PM02 §9) is the tool for
hostile *storage*: typed provider errors, corrupted bodies, stale list pages,
duplicated entries, ambiguous commit-then-fail responses. It lives inside the
process and makes the store fail.

This package makes the *process* stop. A store error is a value the application
handles; a power cut is not. The two are complementary and neither replaces the
other.

## Verification

Twelve unit tests cover the step vocabulary and its distinctness, the ledger
line format, owner-only append behavior, refusal of a relative path, of a
symlinked path, of a reader-less FIFO without blocking on it, and of an
existing world-readable file, ordinal allocation and monotonicity, the before/action/after bracketing order, write
wrapping and read pass-through over the in-memory backend, wrapper accessors,
and the production-shaped path where no policy is configured. The kill itself
is exercised by
`code/programs/rust/vault-pm-cli-drill/tests/crash_fault_matrix.rs`, which is
where a real process is available to remove.
