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
namespace, object identifier, path, item title, ciphertext, or length. The
ledger of a vault holding ten thousand secrets is indistinguishable from the
ledger of an empty vault running the same ceremony. The file is created
owner-only and is never read back by the executable.

## How it stays out of a release

The package is an *optional* dependency of `coding_adventures_vault_pm_cli`
behind that crate's non-default `crash-injection` feature. The executable
`code/programs/rust/vault-pm-cli` enables the feature through its
`dev-dependencies`, and Cargo unifies features across normal and dev
dependencies only when dev-dependencies are in the graph. So `cargo test` and
`cargo clippy --all-targets` build the binary with injection compiled in, while
`cargo build` and `cargo install` build it without — the strings
`VAULT_PM_CRASH_AT` and `VAULT_PM_CRASH_TRACE` do not appear in a released
executable at all.

## Relationship to `FaultInjectingObjectStore`

`vault-pm-storage`'s `FaultInjectingObjectStore` (VLT-PM02 §9) is the tool for
hostile *storage*: typed provider errors, corrupted bodies, stale list pages,
duplicated entries, ambiguous commit-then-fail responses. It lives inside the
process and makes the store fail.

This package makes the *process* stop. A store error is a value the application
handles; a power cut is not. The two are complementary and neither replaces the
other.

## Verification

Eight unit tests cover the step vocabulary and its distinctness, the ledger
line format, owner-only append behavior, ordinal allocation and monotonicity,
the before/action/after bracketing order, write wrapping and read pass-through
over the in-memory backend, wrapper accessors, and the production-shaped path
where no policy is configured. The kill itself is exercised by
`code/programs/rust/vault-pm-cli/tests/crash_fault_matrix.rs`, which is where a
real process is available to remove.
