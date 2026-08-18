# VLT-PM42 — Pending-Publication Recovery at Vault Open

## Status

Normative Phase 1A contract for finishing an interrupted mutation publication.
Closes VLT-PM00 §23 item 10a, the defect
`VLT-PM41-cli-crash-fault-matrix.md` §8 found and pinned, and with it the
crash clause of VLT-PM00 §14.8.

It does **not** close Phase 1A. Verifying §14.8 for this contract turned up one
acceptance criterion — passphrase rotation rewrapping the VRK — that nothing
implements and no item tracked, plus a contradiction between §14.4 and §23
about which phase owns the password generator. Both are now VLT-PM00 §23 items
10b and 10c. They are unrelated to crash recovery and out of scope here; they
are named so that "the last open item is closed" is not mistaken for "the phase
is done".

## 1. The defect this contract repairs

VLT-PM41 killed a real `vault-pm` process with `SIGKILL` at every landing point
of the shared mutation publication path and asked the next real process what it
could see. The answer was good news and bad news.

The good news, and it is the larger half: **the tree is never torn.** A kill
inside `publish_mutation` leaves a durable `PendingPublication` owner state
whose journal is exact. It holds the complete already-signed bytes, the base
heads they were computed against, and the heads the provider must return. Both
read-only diagnostics recognise it: `status` prints `recovery_required` and
`doctor` reports `recovery_required` with exit class 5, neither of them asking
for a passphrase. `vault-pm-application::recover_pending_publication` replays
that journal idempotently — same counter, same object identifiers, same
commit — and advances the owner state to `Active` only after the provider
returns exactly the expected heads.

The bad news is one sentence long: **nothing ever called it.**

`recover_pending_publication` was exported from `vault-pm-application` and
referenced only by that crate's own tests. There was no verb that reached it,
`init`'s resume path refused a `PendingPublication` outright, and
`open_active_vault` accepted only an `Active` owner state. So from the instant
of the crash, every command that opens the vault failed — `item list`,
`item show`, `search`, `audit verify`, `export`, `doctor --unlock` — and failed
as exit 2, `vault-pm: invalid command`.

That last detail is what turns a recoverable state into an unrecoverable
product. A person whose laptop lost power mid-write was told, over and over,
that their *command* was wrong, about a vault that was intact and one journal
replay away from healthy. Severity: availability, high. No secret exposed, no
integrity claim broken, no data lost — and no way to get at any of it.

Surviving exactly this is the reason a local-first password manager keeps a
write-ahead journal at all. A journal nobody replays is a receipt for a repair
that never happens.

### 1.1 The contract already said so

This is not a change of mind. `VLT-PM05-application.md` §8 enumerates what an
open performs, and step 2 of that list reads:

> resume a prepared initialization or pending publication when present

Both halves of that sentence were specified from the beginning. The
`PreparedInit` half shipped — `init`'s resume path rehydrates it and completes
generation zero. The `PendingPublication` half was implemented as a function
and then never connected to an open. So the repair below is not a new
capability being argued for; it is an existing normative requirement finally
being met, and VLT-PM05 §8 is amended to say where the resume is composed
rather than to say something new.

## 2. What this contract adds, and what it deliberately does not

It adds **no verb, no flag, no file format, no on-disk artifact, and no
environment variable**. VLT-PM00 §14.4 lists the Phase 1A command surface and
this contract does not extend it.

What it adds is a rule at the vault-open boundary:

> When a command that must open the vault finds a `PendingPublication` owner
> state, it finishes that publication with the passphrase it has already
> collected, and then opens the vault. When it finds anything else, it behaves
> exactly as before.

This is deliberately *not* a `vault-pm recover` verb. Three reasons, in order
of weight:

1. **A verb the wedged person does not know about does not help them.** The
   failure they see is a command that does not work. The repair has to live on
   the path they are already walking.
2. **Recovery needs precisely the authority an open needs, and no more.** It
   authenticates the same passphrase against the same bootstrap root wrap. A
   separate verb would collect the same secret at the same prompt to do a
   strictly smaller thing.
3. **Replaying a write-ahead journal is not a decision a user is qualified to
   make.** The bytes were signed before the crash; the only question is whether
   the provider has them yet. That question has one correct answer, and the
   machine can check it.

The corresponding non-goals: this contract does not garbage-collect the
unreachable frames an abandoned mutation can strand (VLT-PM00 §19.4, Phase 2),
does not change the durability asymmetry VLT-PM41 §8.1 recorded, does not touch
the `PreparedInit` resume path's existing behavior beyond the case below, and
does not make the crash-injection instrumentation reachable from the shipped
binary in any build.

## 3. Placement

```text
vault-pm-application   VaultAccessV1::unlock_recovering_pending_publication
                       UnlockRecoveryV1
vault-pm-cli           authenticated_access, portable_export, audit_verify
                       resume_init, resume_vault_create
                       doctor  (read-only; reclassified, never repaired)
                       execute (observes the transition, renders the notice)
```

The composition happens in `vault-pm-application`'s lifecycle boundary because
that is the layer that already owns "a host decides when to unlock". It does
*not* happen inside `open_active_vault`, and `VaultAccessV1::unlock` keeps its
strict contract unchanged: a host that wants a plain authenticated open of an
`Active` vault, with a `PendingPublication` refused, still has one. The
recovering entry point is a second, explicitly named door, and every caller
that walks through it says so at the call site.

## 4. The recovering unlock

```text
unlock_recovering_pending_publication(passphrase, local, bootstrap, factory)
    -> UnlockRecoveryV1
```

```text
                    ┌─────────────────────────────┐
   locked access ──▶│ read durable owner state    │
                    └──────────────┬──────────────┘
                                   │
              PendingPublication   │   anything else
                 ┌─────────────────┴──────────────────┐
                 ▼                                    │
    ┌────────────────────────────┐                    │
    │ recover_pending_publication│                    │
    │  · republish exact journal │                    │
    │  · require expected heads  │                    │
    │  · compare-exchange Active │                    │
    └──────────────┬─────────────┘                    │
                   │                                  │
                   └──────────────┬───────────────────┘
                                  ▼
                    ┌─────────────────────────────┐
                    │ open_active_vault           │  ← unchanged, strict
                    └──────────────┬──────────────┘
                                   ▼
              RecoveredPendingPublication | AlreadyActive
```

Five properties this shape is chosen for:

**The reopen is a full independent open, not a continuation.** Recovery returns
an `ActiveStateV1`, and this function then throws it away and opens the vault
from durable state exactly as the *next* process would. Every verification an
ordinary unlock performs — bootstrap signature, root-wrap authentication, seed
to pinned-public-identity reproduction, repository open against non-empty local
pins — runs after the repair, on the repaired bytes. A recovery that produced a
vault only the recovering process could open would be worth less than no
recovery at all.

**It costs one extra Argon2id derivation, and only on the crash path.** The
recovery and the reopen each consume a passphrase by value, so the crash branch
derives the vault root key twice. That is the price of the previous paragraph
and it is paid only by a process that found a wedged vault. The `AlreadyActive`
path derives exactly once, as before.

**The second copy of the passphrase is explicit.** `Zeroizing` deliberately
implements neither `Clone` nor `Debug`, so duplicating a secret cannot happen by
accident. The recovering branch constructs its duplicate by name, and both
copies are wiped on drop, including on the unwind path.

**A wrong passphrase fails closed before anything is published.** Recovery
authenticates the root wrap before it touches the provider, so an attempt with
the wrong secret returns the same closed authentication class an ordinary
unlock returns and leaves the exact journal untouched for a later, correct
attempt.

**Failure is idempotent-safe.** `recover_pending_publication` leaves the journal
in place on provider ambiguity and accepts a concurrent local winner only when
that winner installed the identical intended `Active` bytes. Repeating the whole
ceremony after any failure is therefore always sound.

`UnlockRecoveryV1` is a closed two-variant enum — `AlreadyActive` and
`RecoveredPendingPublication`. It carries no identity, no count, and no byte;
it exists so a host can tell a person that something was repaired.

## 5. Which CLI paths recover, and which must not

| Path | On `PendingPublication` | Why |
|---|---|---|
| `authenticated_access` (item CRUD/list/show/reveal, search, history, conflict, audit enable/list/show, import, restore) | recovers, then opens | the path every wedged person is already on |
| `portable_export` | recovers, then opens | an export must describe a settled vault |
| `audit_verify` | recovers, then opens | it publishes an audit-only commit itself |
| `init` resume | recovers, reports repair | it is the verb a stuck person retries |
| `vault create` resume | recovers, reports repair | same shape, same reasoning |
| `status` | **reports, never repairs** | it must answer without a passphrase |
| `doctor` (locked) | **reports, never repairs** | same |
| `doctor --unlock` | **reports, never repairs** | doctor is a diagnostic |

The two read-only diagnostics keep their VLT-PM41-pinned behavior exactly:
`status` prints `recovery_required`, `doctor` reports `recovery_required` with
exit class 5, and neither writes anything. A person who wants to look before
they leap can still look, and a person restoring a pre-mutation tree from an
ordinary file-level backup still sees that tree accepted.

`doctor --unlock` changes in one respect and it is a reclassification, not a
repair. Previously it inherited the misleading exit 2 `invalid command` from
the refused open. It now short-circuits on the durable state it can already
read and emits the same `recovery_required`/5 the locked diagnostic emits. It
still repairs nothing, and it still never publishes.

`init`'s resume path previously refused a `PendingPublication` with the
conflict class. It now finishes the publication and reports success, because
"finish what was interrupted" is precisely what `init`'s resume path already
means for a `PreparedInit` journal; a `PendingPublication` is the same promise
one generation later. `vault create`'s resume path is the same code shape and
gets the same treatment.

## 6. Telling the person

A repair that happens silently is indistinguishable from no repair, and this is
a password manager: a write completing later than the user watched it complete
is exactly the kind of thing they are entitled to know.

The composition root observes the durable owner state immediately before the
command runs and, **only if that reading found `RecoveryRequired`**, again
immediately after. Both reads happen under the same cross-process writer lock
the command already holds. When the state was `RecoveryRequired` before and is
observably something else after, one fixed payload-free line is added to
standard error:

```text
vault-pm: recovered an interrupted write
```

Standard output is untouched, so nothing that parses a command's output has to
change, and the exit class is unchanged.

The complete rule, because the interesting rows are the silent ones:

| before | after | notice | why |
|---|---|---|---|
| `RecoveryRequired` | `Locked` | **yes** | owner state is `Active`, and only this command held the writer lock |
| `RecoveryRequired` | `RecoveryRequired` | no | still wedged; nothing was finished |
| `RecoveryRequired` | unobservable | no | an observation that could not be taken is not evidence |
| `RecoveryRequired` | `Absent` or `Prepared` | no | not a repair — owner state went missing or backwards |
| anything else | anything | no | there was nothing to repair |

The affirmative row names one state rather than "anything else" on purpose.
`Absent` and `Prepared` are unreachable after a repair today — the owner-state
store exposes no delete and both `PreparedInit` writers demand absence — but if
either ever appeared it would mean owner state was lost or rolled back, and
calling that "recovered an interrupted write" would be a worse lie than
silence.

Three notes on why this is trustworthy:

- **It cannot produce a false positive.** The writer lock excludes every other
  local writer for the whole command, so nothing but this command can move the
  state out of `RecoveryRequired` between the two reads.
- **Both ends fail toward silence, and that is the safe direction.** If the
  configuration cannot be read, or the selector names a vault the observation
  cannot resolve, or the owner state becomes unreadable while the command runs,
  the notice is withheld and the repair — if it happened — happens quietly. The
  third row above is the trap this rule exists to avoid: "not observed" is not
  the same as "no longer wedged", and reading it as such would announce a repair
  on a vault that is still broken. A missing notice costs a courtesy; a wrong
  one is a false claim about a person's vault.
- **It says "recovered", not "your command did something extra".** The
  observation is of the vault, not of the verb, so the same sentence is correct
  for `item list`, for `init`, and for a shell command.

The conditional second reading is a requirement, not an optimization. Reading
owner state initializes its storage backend, and a backend initialization is a
*durable step* — one VLT-PM41's ledger names and kills real processes at. An
unconditional second reading therefore appends durable writes after every
ceremony's own last one, which makes "the portable-export artifact is the last
thing this command makes durable" false and invalidates the landing-point
sweeps. An observation about a command must not move the command.
VLT-PM41's `an_interrupted_portable_export_never_publishes_a_partial_artifact`
is what enforces this, and it is the test that caught the mistake.

## 7. Security analysis

**Does the newly reachable path need a secret it did not need before?** No. It
needs exactly the passphrase the open already collected, at the same prompt,
under the same no-echo terminal policy. No new prompt, no new secret, no secret
in argv, an environment variable, or a file. The duplicate passphrase copy
lives strictly inside one function call and is wiped on drop.

**Does it weaken the authentication boundary?** No. The unlock that follows a
repair is the ordinary strict unlock, so a caller cannot reach an unlocked
session by any route an `Active` vault does not already offer. A wrong
passphrase is rejected by the recovery before publication and by the open after
it.

**Does it let an attacker with write access to the vault directory cause a
publication?** Only of bytes that were already signed by the device key before
the crash. An attacker who can forge a `PendingPublication` journal would have
to forge the signed commit inside it, which is the same forgery the ordinary
verified open already refuses.

**Does it make crash injection reachable from the shipped binary?** No, and
this contract touches none of the isolation VLT-PM41 built. The
`crash-injection` feature remains an optional dependency of the
`vault-pm-cli` library, enabled by exactly one crate — the separate
`vault-pm-cli-drill` program — through its ordinary `[dependencies]`. The
product executable still asserts `!CRASH_INJECTION_COMPILED` at compile time
and still contains neither `VAULT_PM_CRASH_AT` nor `VAULT_PM_CRASH_TRACE`.

**Does the notice leak anything?** It is one fixed string chosen at compile
time. It names no vault, item, revision, object, provider, path, or count.

## 8. Acceptance gates

1. `unlock_recovering_pending_publication` returns `AlreadyActive` on an
   `Active` vault and produces a session byte-identical in observable content
   to the one `unlock` produces.
2. It returns `RecoveredPendingPublication` on a `PendingPublication` vault,
   the resulting durable state is exactly the `Active` state
   `recover_pending_publication` produces on its own, and the opened session
   sees the recovered mutation.
3. A wrong passphrase against a `PendingPublication` vault fails with the
   authentication class and leaves the exact pending bytes unchanged.
4. A `PreparedInit` owner state is still refused by the recovering unlock; it
   belongs to `init`.
5. Recovering twice is a no-op: the second call observes `AlreadyActive`.
6. `VaultAccessV1::unlock` still refuses a `PendingPublication`.
7. Real-process, pseudo-terminal proof: a `vault-pm-drill` process killed at a
   landing point that leaves a journal is followed by an ordinary `item list`
   that **succeeds**, and by a `status` that reports `locked`, and a `doctor`
   that reports `authentication_required`/3.
8. Real-process proof that the recovered write actually landed: an `item add`
   killed after its journal is durable is visible in a later `item list`, with
   the identifier the interrupted process announced.
9. Real-process proof that the vault is ordinary afterwards: a new `item add`
   following a recovery succeeds and both items are listed.
10. Real-process proof that `init` on a wedged vault repairs it.
11. Real-process proof that the read-only diagnostics still refuse to repair:
    `status` and `doctor` run against a wedged vault leave it wedged.
12. The recovery notice appears on standard error exactly when a repair
    happened, and never on a command that only reported the wedged state. The
    complete truth table of §6 is pinned directly, including the row where the
    after-state cannot be observed and the notice must be withheld.
13. Every landing point of the shared publication path is, after this contract,
    either a clean rollback or a state the very next ordinary command repairs.
    No landing point leaves a vault any command refuses.
14. The shipped binary still fails to compile with the crash-injection feature
    and still contains neither environment-variable name.

## 9. References

### Internal

- `VLT-PM00-local-first-password-manager.md` §14.4 command surface, §14.7 exit
  classes, §14.8 Phase 1A acceptance criteria, §23 items 10 and 10a.
- `VLT-PM41-cli-crash-fault-matrix.md` §8 (the finding), §8.1 (what is
  deliberately left open), §4.6 (the isolation this contract preserves).
- `VLT-PM05-application.md` §6b pending-publication recovery.
- `VLT-PM09-cli-bootstrap.md` init and resume.
- `VLT-PM40-cli-interactive-shell.md` per-command verified open.

### Code

- `code/packages/rust/vault-pm-application/src/lifecycle.rs`
- `code/packages/rust/vault-pm-application/src/open.rs`
- `code/packages/rust/vault-pm-cli/src/lib.rs`
- `code/programs/rust/vault-pm-cli-drill/tests/crash_fault_matrix.rs` sections
  3 and 6
