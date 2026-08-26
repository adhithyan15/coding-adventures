# VLT-PM41 — CLI Crash/Fault Matrix and Local Restore Drill

## Status

Normative Phase 1A **test** contract. This document specifies no product
feature. It adds no command, no flag, no file format, no on-disk artifact, and
no capability a user can invoke. Everything it describes is verification
scaffolding plus the enumeration and pass criteria that scaffolding checks.

It is the last item of VLT-PM00 §23 Phase 1A ("Crash/fault matrix and local
restore drill"). Section 8 records what the drill found, which is the reason
Phase 1A does **not** close with this slice.

## 1. Purpose

`vault-pm` has claimed crash safety in almost every slice it shipped. The
phrases recur: *crash-resumable*, *write-ahead owner-state transitions*,
*publish-before-release*, *ambiguous-provider recovery*, *real-process restart
proof*. VLT-PM05 §7 states the claim as a state machine, and VLT-PM00 §14.8
turns it into an acceptance criterion:

> A simulated crash at every publication step either exposes the old commit or
> a valid new commit; never a partial logical state.

Two things were missing.

**The claim was never enumerated.** Each slice asserted crash safety for its
own ceremony against its own hand-picked interruption. Nobody had written down
the whole set: every operation with a write path, crossed with every point at
which a crash can land in it. Without the enumeration there is no way to say
whether a case is untested or merely unlisted.

**The claim was never tested against a real killed process.** Existing
crash tests build the application over an in-memory store, arrange for a store
call to fail, and then call the recovery function themselves. That exercises
recovery *logic*. It does not exercise recovery *from a process the operating
system removed*. Those differ in everything a user actually experiences: the
process gets no chance to unwind, no destructor runs, no buffer flushes, the
advisory writer lock is released by the kernel rather than by a guard, and the
next process starts from nothing but a directory tree.

This contract closes both gaps.

## 2. What "a crash" means here

The drill injects `SIGKILL` into a real `vault-pm` process — specifically
`vault-pm-drill`, the instrumented twin described in section 4.6. `SIGKILL`
cannot be caught, blocked, or handled. The kernel removes the process
immediately: no `atexit`, no `Drop`, no flush, no cleanup. This is deliberately
stronger than `exit()` (which runs handlers and flushes) and than `abort()`
(which is a catchable signal and asks the platform for a crash report).

A killed child is also *provable*: the parent observes a termination signal
rather than an exit code, so a drill cell that expected a crash and got a clean
exit fails loudly instead of silently measuring nothing.

The drill does not model media corruption, partially-written sectors, or a
lying filesystem. Those are VLT-PM02's `FaultInjectingObjectStore` (§3 below)
and remain in scope for the storage conformance suite, not for this one.

## 3. Why the existing fault-injection backend is not the mechanism

`VLT-PM02-storage.md` §9 specifies `FaultInjectingObjectStore<S>`, and
`vault-pm-storage` implements it. It is the right tool for hostile *storage*:
typed provider errors, corrupted bodies, stale list pages, duplicated entries,
and ambiguous "committed then reported a network failure" responses. It is
used throughout `vault-pm-repository` and `vault-pm-application`.

It cannot be the mechanism here, for one structural reason: it lives inside the
process it is testing, and it makes the *store* fail rather than making the
*process* stop. A store error is an error value the application handles. A
power cut is not. Reusing it would have re-tested the recovery logic that is
already tested, in the same way it is already tested.

So this contract adds a second, complementary mechanism, at a different layer
and with a different job.

## 4. The mechanism: durable steps

### 4.1 Every local durable write passes one of three gates

```text
  application owner state  ─┐
  bootstrap generations    ─┤
  immutable object frames  ─┼─→ storage_core::StorageBackend
  signed announcements     ─┤       (initialize / put / delete / acquire_lease)
  audit-chain event frames ─┘

  client configuration     ───→ LocalWriterGuard create / compare-exchange
  portable export artifact ───→ CliHost::write_portable_export
```

`vault-pm-application` is storage-agnostic and owns no filesystem authority, so
it is not the layer that knows what "durable" means. The composition root —
`code/packages/rust/vault-pm-cli` — is. The instrumentation therefore lives
there and nowhere else.

### 4.2 A durable write has no middle

`storage-fs` writes a temporary file, `fsync`s it, `rename`s it into place, and
best-effort `fsync`s the parent directory. `vault-pm-local-host` does the same
for `vault-pm.toml`, using `linkat` for creation so an existing file is never
clobbered. `rename(2)` is atomic.

It follows that "crashing partway through a write" is not a distinct outcome.
The complete on-disk state after any crash is determined by *how many* durable
writes completed. That turns an uncountable set of crash instants into a
finite, totally ordered set of landing points:

```text
  step 1   before durable write #1
  step 2   after  durable write #1
  step 3   before durable write #2
  step 4   after  durable write #2
  …
```

An operation performing `n` durable writes has exactly `2n` landing points, and
every possible crash of that operation is equivalent to landing on one of them.
Enumerating them is a complete case analysis, not a sampling strategy.

### 4.3 The package

`code/packages/rust/vault-pm-crash-injection` provides:

- `DurableStep` — the closed vocabulary `storage.initialize`, `storage.put`,
  `storage.delete`, `storage.lease`, `config.create`, `config.replace`,
  `export.artifact`;
- `Phase` — `before` / `after`;
- `record(step, phase) -> u64` — consume the next process-global ordinal,
  optionally append it to a ledger, and `SIGKILL` this process if it is the
  chosen one;
- `around(step, action)` — bracket one durable write in its two landing points;
- `CrashInjectingStorageBackend<B>` — a `StorageBackend` decorator that
  brackets `initialize`, `put`, `delete`, and `acquire_lease`, and passes reads
  through untouched. A crash during a read changes nothing on disk, so it
  collapses into the "before" landing point of the next write.

Two environment variables drive one process:

| Variable | Meaning |
|---|---|
| `VAULT_PM_CRASH_TRACE` | append the durable-step ledger to this path |
| `VAULT_PM_CRASH_AT` | `SIGKILL` this process when that ordinal is reached |

A `VAULT_PM_CRASH_AT` that is not a positive decimal integer is a hard error. A
typo must not silently turn a crash drill into an ordinary successful run.

### 4.4 The ledger is metadata about shape only

A ledger line is `ordinal \t phase \t step`. Nothing else may ever appear in
it: no key, namespace, object identifier, path, title, ciphertext, or length.
No vault *content* can reach the file.

Be precise about what that does and does not hide. Each object write emits one
`storage.put` pair, so the ledger's *length* correlates with how much a
ceremony wrote — it is a shape and activity oracle, not a content oracle. Two
vaults running the same ceremony over the same number of objects produce
identical ledgers; a larger vault produces a longer one. That is acceptable
because the ledger exists only when a drill names it, and counting those writes
is the drill's whole purpose.

The path is treated as untrusted even though only something that already
controls the process environment can set it. It must be absolute, so a working
directory cannot redirect it. It is opened with `O_NOFOLLOW | O_NONBLOCK`: the
first refuses a symlink at the final component, the second refuses a
reader-less FIFO, which `O_NOFOLLOW` says nothing about and which would
otherwise block the open forever — making the "not a regular file" check
unreachable, since the open would never return to run it. It is created `0600`,
and an *existing* file is refused unless it is a regular file, owned by this
user, and already owner-only, because a creation mode says nothing about a file
that already exists. Ownership is checked by `fstat` on the open descriptor,
not by a second path lookup, so there is no window between the check and the
write. The executable never reads the ledger back.

What none of that does is confine *where* the ledger may be: any absolute path
naming an existing private regular file this user owns is accepted, including
one inside their own vault state. That authority is declared in the drill
crate's capability manifest rather than argued away.

### 4.5 The drill derives the matrix from the code

A drill runs a ceremony once with only `VAULT_PM_CRASH_TRACE` set, counts the
ledger lines to learn `2n`, and then replays that ceremony `2n` times — once
per landing point, from a byte-identical starting tree.

This matters more than it looks. The matrix is not a list a person maintains
alongside the code; it is *computed from the code under test*. A slice that
adds a durable write to a ceremony grows that ceremony's sweep automatically,
and cannot quietly escape it.

### 4.6 The shipped binary cannot contain any of this

The instrumentation is an **optional** dependency of `vault-pm-cli` behind a
non-default `crash-injection` feature. With the feature off, `LocalBackend` is
exactly `FsStorageBackend` and each `around_*` combinator is an `#[inline]`
function whose whole body is `action()`. There is no counter, no environment
read, and no kill switch.

The question is which builds get the feature, and the obvious answer is wrong.
Enabling it through the executable's own `dev-dependencies` *appears* to work —
`cargo build` and `cargo install` do not resolve dev-dependencies, so they
build an uninstrumented binary. But Cargo resolves features per *package*
across a build graph, and `cargo build --all-targets` (or `--tests`) does pull
dev-dependencies in. Cargo then uplifts the feature-unified binary to the
conventional `target/release/vault-pm` path — the exact path a packaging step
copies from. Last command wins, so the artifact's safety would depend on
invocation order. A password manager would ship an environment-variable kill
switch that fires between durable writes.

So the drill does not share a crate with the product. The shipped executable at
`code/programs/rust/vault-pm-cli` **never names the feature in any section**.

That is necessary and *not sufficient*, and the difference is worth stating
because it is easy to stop one step early. Cargo's `--features <dep>/<feature>`
syntax reaches a direct dependency's features even when the root package
declares none of its own, so `cargo build --release --features
coding_adventures_vault_pm_cli/crash-injection` would still produce an
instrumented `target/release/vault-pm`. The product's `main.rs` therefore also
carries

```rust
const _: () = assert!(!coding_adventures_vault_pm_cli::CRASH_INJECTION_COMPILED);
```

so such a build is a **compile error**, not a binary somebody has to remember
to inspect. The instrumented twin lives at
`code/programs/rust/vault-pm-cli-drill`, is byte-identical in composition,
enables the feature as an ordinary dependency feature, and produces a binary
called `vault-pm-drill` that cannot be mistaken for the product. The two are
separate cargo workspaces, so their feature resolution never meets.

The drill twin also carries a larger declared capability profile than the
product — it adds `env: read` for the two injection variables, `proc: signal`
for removing its own process, and `fs: create` for the ledger — so the extra
authority is visible in a manifest rather than implied.

Four checks hold this down:

- The `const` assertion above: a product build with the feature active does not
  compile, under any invocation, with no test needing to have run.
- `the_shipped_executable_contains_no_crash_injection`, in the *product*
  crate's own suite, reads the binary that crate produced and fails if either
  variable name appears in it. It runs in a build that has dev-dependencies
  resolved, which is one of the configurations the mistake shows up in.
- `the_released_binary_shape_is_the_only_thing_the_drill_changes`, in the
  drill, asserts that an uninstrumented process writes no ledger and behaves
  as the end-to-end suite observes it.
- Manual verification across `cargo build`, `cargo build --release`,
  `cargo install --path .`, `--all-features`, `--release --all-targets`, and
  `--features coding_adventures_vault_pm_cli/crash-injection`: the shipped
  binary contains neither string, and the last one fails to build.

## 5. Pass criteria

Each cell of the matrix must land in exactly one of two acceptable classes.

**Clean rollback.** The tree is indistinguishable from before the operation
started. The read-only diagnostics report the pre-operation state, and the
operation can simply be run again.

**Crash-resumable.** The tree carries an exact journal. The read-only
diagnostics say so, in the closed vocabulary VLT-PM05 defines. Finishing or
replaying the operation reaches the same end state the uninterrupted run would
have reached, from the identical bytes — never newly generated ones.

The forbidden class is **torn**: a tree that decodes to something that is
neither the old nor the new state, that no longer opens, or that has lost a
committed effect.

Three further invariants hold in every cell regardless of class:

1. **No plaintext.** No passphrase, item secret, note, or export passphrase is
   readable anywhere under the platform home after the kill.
2. **No held lock.** The writer lock is advisory and process-scoped; the kernel
   released it when it removed the process, so the next command runs rather
   than reporting a concurrent writer.
3. **No partial artifact.** A file the user chose the name of — today only the
   portable export — either does not exist or is complete.

## 6. The matrix

`Ops` are the operations with a write path. `Points` is `2n` where `n` is the
number of durable writes the ceremony performs, measured by the ledger rather
than asserted by hand.

### 6.1 Swept exhaustively

| Operation | Spec | Points | Coverage |
|---|---|---:|---|
| `init` — generation zero | VLT-PM09, VLT-PM21 | 34 | every point |
| shared publication path, via `audit verify` | VLT-PM05 §7.2, VLT-PM15 | 20 | every point |
| `export FILE` | VLT-PM17 | measured | every point |

The generation-zero row and the publication row are the two that matter most,
and between them they cover the machinery every other row reuses:

- **Generation zero** is the only ceremony that writes the configuration file,
  the bootstrap generation record, the latest-bootstrap pointer, and the
  `PreparedInit` journal. Nothing else creates a vault.
- **The publication path** is *one function*. `add_item`, `replace_item`,
  `delete_item`, `restore_item`, `resolve_item_conflict`,
  `merge_item_conflict`, portable import, and every audit-only commit all reach
  the disk through the same `publish_mutation`: reserve a counter, build the
  complete signed frames, compare-exchange `Active → PendingPublication`,
  publish objects then commit then announcement, check the receipt pins,
  compare-exchange `PendingPublication → Active`. `audit verify` is simply the
  smallest command that drives it.

`audit verify` is used as the vehicle deliberately: it is a command a person
thinks of as read-only, which makes it the least expected place to lose a
vault, and it needs one prompt rather than seven.

### 6.2 Probed at characteristic points

Sweeping one ceremony exhaustively proves the publication state machine. What
every *other* mutating ceremony still has to prove is narrower: that it reaches
that machine, and that its own preparation phase — prompts, entropy, encoding,
validation — makes nothing durable it could tear. Three landing points settle
that: the first of all, the middle (where the write-ahead record has landed),
and the last (the release).

| Operation | Spec | Probe |
|---|---|---|
| `item add login` | VLT-PM11 | first / middle / last |
| `item edit ITEM` | VLT-PM12, VLT-PM30 | first / middle / last |
| `item delete ITEM` | VLT-PM14 | first / middle / last |
| `history restore ITEM REV` | VLT-PM14 | first / middle / last |
| `conflict list` / `conflict merge login` | VLT-PM24, VLT-PM33 | first / last |

The conflict row drills a ceremony that *fails closed*: a single-device vault
has no unresolved conflict, so the merge is refused. That is worth drilling
rather than skipping, because with auditing enabled the durable record of the
refusal is itself a publication that can be interrupted.

### 6.3 Deliberately not covered by this slice

Logged as follow-up rather than silently omitted:

| Not covered | Why | Where it goes |
|---|---|---|
| The other six record types' create ceremonies (VLT-PM16, VLT-PM26–29 and the opaque type) | Same `publish_mutation`, same shape as the login row; only the prompt script differs | Follow-up A |
| The other six authored merges (VLT-PM34–39) | Same | Follow-up A |
| `import FILE`, `restore FILE`, `restore verify FILE` (VLT-PM18–20, VLT-PM23) | Multi-cycle ceremonies with a second unlock; each cycle is the swept publication path, but the *composition* has crash windows of its own — notably "imported but not yet verified" | Follow-up B |
| `vault create NAME` (VLT-PM22) | A second generation zero plus a configuration compare-exchange; the compare-exchange landing points are unswept | Follow-up B |
| `audit enable`, `item reveal`, `conflict reveal`, `search` (VLT-PM15, VLT-PM25, VLT-PM31, VLT-PM32) | Each publishes one audit-only commit — the swept path — before releasing its result | Follow-up A |
| `shell` (VLT-PM40) | A session is a sequence of the same one-shot commands; a kill mid-session is a kill mid-command | Follow-up A |
| Torn media, lost `fsync`, lying filesystem | A different fault class; belongs to VLT-PM02 conformance | Out of scope |

The exhaustive cross-product is roughly 25 ceremonies × 20–40 points ≈ 700
real-process kills, each paying a production Argon2id derivation. That is a
nightly job, not a per-PR check. This slice deliberately buys the two rows that
carry the shared machinery, plus one worked example per ceremony family, and
says so rather than presenting a subset as the whole.

## 7. The local restore drill

The second half of the deliverable answers the question a person actually has
after their machine died: *what can I find out without typing my passphrase,
and what do I do next?*

`the_read_only_diagnostics_describe_every_stage_of_an_interrupted_vault` walks
one vault through every stage and pins both answers at each:

| Stage | `status` | `doctor` | exit | What to do |
|---|---|---|---:|---|
| nothing exists | `uninitialized` | `initialization_required` | 2 | run `init` |
| generation zero interrupted | `initializing` | `initialization_required` | 2 | run `init` — it resumes the exact journal |
| ordinary vault, locked | `locked` | `authentication_required` | 3 | unlock |
| mutation interrupted | `recovery_required` | `recovery_required` | 5 | **see §8** |
| pre-mutation tree restored from a file-level backup | `locked` | `authentication_required` | 3 | nothing |

Two properties of this table are load-bearing.

`status` distinguishes "there is no vault here" from "there is a half-built
one", because those need different reassurance. `doctor` collapses both into
the single instruction a person can act on. Neither reads the repository, opens
a key, or requires a passphrase.

The last row is the file-level backup guarantee: restoring the platform home
from an ordinary filesystem backup taken before a mutation yields a vault that
opens and verifies. The drill proves that by capturing the tree, crashing a
mutation into it, restoring the capture, and running `audit verify` to
completion.

## 8. What the drill found

> This section is the most important output of this slice.
>
> **Resolved.** The defect below was repaired by
> `VLT-PM42-cli-pending-publication-recovery.md` and VLT-PM00 §23 item 10a is
> closed. The finding is kept in the past tense it was written in, because the
> value of a drill is the defect it finds and deleting the record would delete
> the evidence that this one works. What changed: the vault-open boundary now
> replays the exact journal with the passphrase it already collects, so every
> landing point in section 6.1's sweep is either a clean rollback or a state
> the next ordinary command finishes. The assertions section 8's last paragraph
> left pinned have been rewritten to require that, and the drill additionally
> proves the *content* of the repair — an interrupted `item add` is a listed,
> readable item afterwards. The read-only diagnostics still repair nothing.

**A kill anywhere inside the shared publication path leaves a vault that no
`vault-pm` command can repair.**

The invariant VLT-PM00 §14.8 states is *not* violated in the repository sense:
the tree is never torn. The durable `PendingPublication` journal is exact, it
contains the complete already-signed bytes, and
`vault-pm-application`'s `recover_pending_publication` replays them
idempotently — same counter, same object identifiers, same commit — and
advances the owner state. Both read-only diagnostics correctly report
`recovery_required`. The application layer's contract is intact and well
tested.

**No CLI code path calls `recover_pending_publication`.** The function is
exported from `vault-pm-application` and referenced only by that crate's own
tests. There is no `recover` verb, and `init`'s resume path explicitly refuses
a `PendingPublication` state. `open_active_vault` rejects any non-`Active`
owner state, so from the moment of the crash every command that opens the
vault — `item list`, `item show`, `search`, `audit verify`, `export`,
`doctor --unlock` — fails.

It fails as **exit 2, `vault-pm: invalid command`**, because the application
returns `InvalidInput` for a non-`Active` state and the CLI maps that to the
invalid-input class. So a person whose laptop died mid-write is told their
*command* is wrong, over and over, for a vault that is intact and one journal
replay away from healthy.

Severity: **availability, high**. No secret is exposed, no data is lost, no
integrity claim is broken. The vault is recoverable in principle and
unrecoverable in practice.

This is a defect in shipped code, not in this slice, and repairing it is a
product change — a new verb, its prompt and audit policy, its own acceptance
gates — so it is not folded into a test slice. It is filed as **VLT-PM00 §23
item 10a** and Phase 1A cannot be declared complete until it lands.

`every_publication_landing_point_leaves_an_exact_resumable_journal` pins the
observed behavior, including the misleading exit class, with a comment
directing the fixing slice to rewrite those assertions rather than delete them.

**What the repair turned out to be.** Not a verb. VLT-PM42 found that
`VLT-PM05-application.md` §8 step 2 had specified the fix all along — an open
must "resume a prepared initialization or pending publication when present" —
and that only the first half had ever been wired. So the repair added no
command, flag, or artifact: the vault-open boundary gained a second, explicitly
named door that replays the journal with the passphrase the open already
collects and then performs the ordinary strict open of the repaired bytes. The
prediction above that this needed "a new verb" was wrong in a useful direction:
a verb a wedged person does not know about would not have reached the person
whose command was failing.

### 8.1 Secondary observations

Neither of these is a Phase 1A blocker; both are recorded so they are not
rediscovered.

**The owner-state file had weaker rename durability than the configuration
file. Resolved.** `storage-fs` used to treat the parent-directory `fsync`
after `rename` as best-effort and discard its result, while
`vault-pm-local-host` hard-fails on the same operation for `vault-pm.toml`.
Under a true power cut — which this drill does not model, since `SIGKILL`
does not lose the page cache — a completed owner-state rename could have been
lost while the configuration rename survived. VLT-PM00 §11.2 permits
best-effort directory fsync, so this was a documented weakness rather than a
contract violation, but the asymmetry deserved a decision.

The decision: `storage-fs` now hard-fails the same way `vault-pm-local-host`
does, on every platform where the failure is even observable. Both files
carry the identical requirement — the owner-state record and `vault-pm.toml`
sit under the same crash-safety journal, and recovery on both sides assumes a
durable `Ok`, not an ambiguous one — so there was no argument for the
asymmetry to resolve in the *other* direction (loosening `vault-pm.toml` to
match `storage-fs`'s old behavior). The one remaining split is Windows, where
`std::fs::File::open` cannot open a directory handle at all inside a
`#![forbid(unsafe_code)]` crate; there the fsync stays a no-op because there
is nothing to attempt, not because the guarantee is considered unnecessary.
See STR01-storage-fs-backend.md, "Directory-fsync durability," for the full
argument, and `storage-fs`'s `fsync_parent_directory` for the implementation.
`delete` gained the identical parent-directory fsync in the same change
(previously it had none at all — see backlog item #19), closing a
defense-in-depth durability gap that was independent of, and not load-bearing
for, the confidentiality fix in
`vault-pm-application-storage-core::supersede_generation` (VLT-PM43), which
already overwrites sensitive bodies through the fsync-durable `put` path
before ever calling `delete`.

**An interrupted generation zero can strand unreachable bytes. Resolved for
the owner-state record; the object-frame case remains open.** A crash between
the `PreparedInit` write and the configuration write leaves an owner-state
record under a random locator that no configuration references. It is opaque,
unreachable, and was never collected.

Backlog item #16. The finding is kept in the past tense it was written in, for
the same reason §8's main finding is: deleting the record would delete the
evidence that the fix works.

**The decision.** Physical garbage collection of the *immutable repository
object store* — VLT-PM00 §19.4 — reconciles verified heads, retained
conflicts, history windows, and multi-device grace periods, and correctly
remains Phase 2 work; it was never the right tool for this narrower case, and
still is not. The owner-state record's leak has a much smaller cause: a
`PreparedInit` can only exist without a configuration reference because of
this exact crash window, since every later transition in VLT-PM05 §7's state
machine happens strictly after the configuration write that names the
locator. The state alone is therefore sufficient evidence that a given record
is this leak's orphan, with no reachability graph, retention policy, or
multi-device consensus required. VLT-PM05 §7.3 specifies the resulting
`StorageCoreApplicationStore::reclaim_orphaned_preparations`, run by `init`
and `vault create`'s fresh-locator paths immediately before they install a new
locator's own journal, under the same writer lock that already serializes
both commands. It never inspects `Active`, `PendingPublication`, or
`PendingRotation` records against configuration at all — the decoded state is
the entire safety argument, not the caller-supplied set of live locators.

**What remains open.** "The same is true of object frames written before an
abandoned mutation's announcement" is a different mechanism — an interrupted
`publish_mutation` can leave immutable object frames in the repository store
that the abandoned commit never references — and this change does not touch
it. That case sits squarely inside VLT-PM00 §19.4's mark-and-sweep, which
needs the reachability graph this narrower fix deliberately avoids building,
and remains Phase 2 work.

**The rotation sweep (section 6.1) was the single slowest unit of this
repository's CI, even after being parallelized per-cell (508s to 117s
locally). Resolved without reducing coverage.** Backlog item #20 had already
declined the obvious next lever once — cutting section 6.1's 48 landing
points down to a representative subset by equivalence class — because that
*is* a real coverage reduction: every landing point not chosen stops being
proven "exactly one passphrase works," which is precisely the property VLT-PM43
§7 gate 5 exists to state without exception.

Revisiting it, the actual bottleneck was never the landing-point count or the
real `SIGKILL`. It was that every cell pays up to five *production* Argon2id
derivations — 64 MiB, 3 iterations — in a debug build, and that cost has
nothing to do with what a crash-injection cell proves: a durable write's
before/after landing point is a fact about `write -> fsync -> rename`
ordering, never about how expensive the KDF that ran earlier in the same
process happened to be. Two things made that separation exploitable without
weakening anything:

- `Argon2idParametersV1::validate` already accepts a much cheaper policy
  (`8 * 1024` KiB, 1 iteration, 1 lane — this crate's own `vault-pm-cli` unit
  tests already use exactly that combination for KDF-adjacent assertions that
  do not care about strength), so a "fast but still bound-valid" policy was
  not invented for this purpose; and
- the composition root already had the exact seam this needed —
  `crash.rs`'s `#[cfg(feature = "crash-injection")]` split, the same one that
  keeps `around_config_create` and friends out of the shipped binary. A third
  function in that split, `kdf_policy_override`, reads
  `VAULT_PM_DRILL_KDF_{MEMORY_KIB,ITERATIONS,LANES}` only when this build has
  the feature compiled in, so the override is dead code in `vault-pm` by
  construction, exactly like the landing-point instrumentation itself.

`code/programs/rust/vault-pm-cli-drill/tests/crash_fault_matrix.rs` now sets
those three variables on every drilled process (`TestHome::configure`, one
choke point, every `Command` in the file goes through it). Every landing
point of every sweep in this file — generation zero's 34, the shared
mutation-publication path's 20, the rotation's 48, every other ceremony —
is still driven through a real `SIGKILL`ed process at every landing point,
with every assertion unchanged; only the wall-clock cost of the KDF calls
those real processes make dropped. Measured on one development machine,
`cargo test` for the whole file: **266.92s to 38.92s**, all 18 tests still
passing, `took_new` still observed both above zero and below the sweep total
in the rotation sweep specifically. Per this campaign's standing lesson that
local timing is a proxy and CI's own run is the ground truth (backlog item
#26), the PR that lands this records the real GitHub Actions timing rather
than stopping at the local number.

The equivalence-class cut considered by backlog item #20 was not needed and
was not taken. The declined shortcut stays declined; nothing here reduces the
48 landing points or narrows what any of them proves.

## 9. Acceptance gates

1. The instrumentation package builds, is `deny(missing_docs)` clean, and its
   own tests cover the step vocabulary, the ledger format, ordinal allocation,
   the before/after bracketing, and read pass-through.
2. A product build with the feature active does not compile, and a released
   binary contains neither environment-variable name — the first held by a
   `const` assertion in the product's `main.rs`, the second asserted by the
   product crate's own suite in a build with dev-dependencies resolved. The
   instrumented twin is a separate crate and workspace producing a differently
   named binary, and the product crate names the feature in no section.
3. An uninstrumented process writes no ledger and behaves exactly as
   `local_cli_e2e.rs` observes it.
4. Every ledger is dense from ordinal one, alternates `before`/`after`, pairs
   each phase with the same step name, and draws every name from the closed
   vocabulary.
5. Generation zero installs its `PreparedInit` journal strictly before the
   configuration write that makes its locator discoverable.
6. Every one of generation zero's landing points is clean or resumable, and the
   resumed vault passes authenticated `doctor --unlock`.
7. Every one of the publication path's landing points is clean or leaves an
   exact journal that both diagnostics report; both classes occur.
8. Each probed ceremony's characteristic points leave a state the diagnostics
   describe in the closed vocabulary, and the ceremony still works afterwards.
9. A portable export publishes no artifact before its single artifact write
   completes.
10. No cell leaves any drill secret readable under the platform home.
11. The drill never hangs. Every prompt wait is bounded and a wait that
    expires kills the child and fails the test. A `kill(2)` a sandbox refuses
    falls back to `abort`, as does a `kill(2)` that succeeds but whose signal
    the kernel discards — which is what happens to a process that is PID 1 in
    its own namespace. A reader-less FIFO named as the ledger is refused
    rather than blocked on.
12. The whole file runs in the same process-per-command, pseudo-terminal style
    as `local_cli_e2e.rs`, with the same stdin-injection negative control.

## 10. Deliberate exclusions

This contract does not add a recovery verb, does not change any on-disk
format, does not weaken the production Argon2id policy for tests, does not add
a test-only CLI flag, and does not introduce any environment variable the
released executable reads. It does not model concurrent writers — the writer
lock excludes them — and it does not model provider faults, which remain
VLT-PM02's.

## 11. References

### Internal

- `VLT-PM00-local-first-password-manager.md` §14.8, §22.1, §23
- `VLT-PM02-storage.md` §9 — the complementary fault-injection backend
- `VLT-PM05-application.md` §7 — the crash-resumable state machine
- `VLT-PM04-repository.md` — publication and verification
- `VLT-PM07-config.md`, `VLT-PM08-cli-host.md` — the two non-backend writes
- `VLT-PM09-cli-bootstrap.md`, `VLT-PM21-audit-first-generation-zero.md`
- `VLT-PM11-cli-login-create-read.md`, `VLT-PM12-cli-login-replace.md`,
  `VLT-PM30-cli-rich-login-edit.md`
- `VLT-PM14-cli-delete-restore.md`, `VLT-PM13-cli-history-list.md`
- `VLT-PM15-operation-audit.md`
- `VLT-PM16-cli-secure-note-create.md`, `VLT-PM26-cli-card-create.md`,
  `VLT-PM27-cli-api-key-create.md`,
  `VLT-PM28-cli-database-credential-create.md`,
  `VLT-PM29-cli-totp-create.md`
- `VLT-PM17-cli-portable-export.md`, `VLT-PM18-cli-portable-import.md`,
  `VLT-PM19-portable-restore-verification.md`,
  `VLT-PM20-cli-portable-restore-verify.md`,
  `VLT-PM23-cli-verified-restore.md`
- `VLT-PM22-cli-named-targets.md`
- `VLT-PM24-cli-conflict-resolution.md`,
  `VLT-PM32-cli-conflict-candidate-reveal.md`,
  `VLT-PM33-cli-authored-login-conflict-merge.md` through
  `VLT-PM39-cli-authored-opaque-record-conflict-merge.md`
- `VLT-PM25-cli-secret-reveal.md`, `VLT-PM31-cli-audited-search.md`
- `VLT-PM40-cli-interactive-shell.md`

### Code

- `code/packages/rust/vault-pm-crash-injection/` — the mechanism
- `code/packages/rust/vault-pm-cli/src/crash.rs` — the seam
- `code/programs/rust/vault-pm-cli-drill/` — the instrumented twin and the
  drill itself, in `tests/crash_fault_matrix.rs`
- `code/programs/rust/vault-pm-cli/` — the product executable, whose
  `tests/local_cli_e2e.rs` is both the finish-the-command counterpart to the
  drill and the guard rail proving the product binary is uninstrumented

---

*End of VLT-PM41.*
