# VLT-PM21 — Audit-First Generation Zero

## Status

Normative Phase 1A application and CLI contract for creating every new product
vault with a signed, encrypted operation-audit genesis. Legacy pre-audit
preparation remains an application migration primitive; the CLI does not use
it for new vaults.

## 1. Goal

VLT-PM15 reserved `VaultInitialize` as the only event allowed to begin at
device counter 1 with no previous event and no basis heads. Earlier CLI slices
nevertheless created a pre-audit generation zero and required a later
`audit enable` migration. That left vault initialization outside the trace.

This slice closes that gap. A successful new CLI vault has one repository
commit and one audit event. The event is the durable audit head before `init`
returns, so the next authenticated edit or access must advance from it.

## 2. Separate compatibility boundary

The application exposes two explicit preparation boundaries:

- `prepare_generation_zero` retains the exact legacy pre-audit contract for
  decoding, recovery, migration, and tests of fail-closed epoch activation;
- `prepare_audited_generation_zero` is the required boundary for every new
  product vault.

The audited boundary consumes `AUDITED_GENERATION_ZERO_RANDOM_BYTES` through
`AuditedGenerationZeroRandomness`. The block contains the complete legacy
generation-zero entropy plus a fresh trace ID and an independent encrypted
audit-object randomness block. Both containers are owned, non-cloneable,
redacted in debug output, and wiped on drop.

## 3. Genesis event

Audited preparation constructs exactly one `AuditEventV1` with:

| Field | Required value |
|---|---|
| vault/device | the new generation-zero identities |
| device counter | `1` |
| trace | fresh host-supplied CSPRNG identity |
| action | `VaultInitialize` |
| outcome | `Succeeded` |
| item/revisions | absent |
| previous event | absent |
| basis heads | empty |
| timestamp | the generation-zero advisory creation time |

The new device signs the event. The application canonical-encodes and seals it
under `ObjectKind::AuditEvent` with independent key/nonce randomness.

## 4. Atomic generation-zero publication

The initial commit's sorted `added_objects` contains the device certificate,
empty catalog, and audit-event object. The retry journal contains all three
non-commit frames, binds the encrypted event as `audit_event_head`, and binds
the same head into its intended active owner state.

The existing `PreparedInit -> Active` completion protocol remains unchanged:
the exact prepared bytes are installed before bootstrap or repository effects,
publication is idempotent, exact heads are verified, and active owner state is
installed last. No CLI success is released from a partial publication.

After a crash, `rehydrate_prepared_init` recovers the same signed and randomized
genesis journal. It does not generate a replacement trace or audit object.

## 5. CLI behavior

`vault-pm init` fills the complete audited randomness block and calls only
`prepare_audited_generation_zero` for a new configuration. Its grammar,
passphrase ceremony, storage selection, retry ordering, and success text do
not change.

For a newly initialized vault:

- `audit verify` reports one commit and one verified audit event;
- `audit list` can expose the redacted `vault_initialize` genesis row;
- `audit enable` returns the existing no-write `Audit: already enabled.`
  success; and
- item creation, including prompt failure, is audited immediately rather than
  belonging to an unlogged prefix.

Existing pre-audit owner state remains unlockable. Its explicit `audit enable`
path still installs `AuditEpochStart` and is covered at the application layer.

## 6. Privacy and failure contract

The initialization event contains no vault alias, storage alias, filesystem or
provider location, passphrase, KDF input, key material, credential reference,
host identity, or arbitrary detail. The trace ID is random and cannot be
derived from those values.

Preparation performs no external write. Any signing, encoding, sealing, bound,
or invariant failure returns the closed application taxonomy without exposing
partially prepared bytes. Completion retains the existing exact retry journal
on ambiguous provider or local-state failures.

## 7. Verification requirements

Tests must prove:

1. audited randomness is exact-size, redacted, and wipe-on-drop;
2. the prepared publication contains three unique non-commit objects;
3. journal and intended active state bind the same audit head;
4. the audit head identifies one supplied encrypted object;
5. legacy generation zero remains pre-audit and its migration tests pass;
6. CLI init, restart, audit verification, and audit history observe the
   initialization event;
7. `audit enable` is a no-write idempotent success for new CLI vaults; and
8. every existing active-epoch CLI test advances from the genesis event.

## 8. Follow-up

With new targets audit-enabled from generation zero, the next recovery slice
can add in-root target creation and configuration selection without an
intermediate unaudited target. Automatic import-to-verifier composition remains
after that target lifecycle is durable and retryable.
