# `coding_adventures_vault_pm_application`

The host-neutral VLT-PM05 application core for the local-first password
manager. The current slices define lossless canonical persistence for local
device secrets, item revisions, and catalog snapshots, plus domain-separated
V1 encrypted object framing and an authority-anchored repository verifier for
the single authorized Phase 1A device. It also defines the exact canonical
`PreparedInit -> Active -> PendingPublication -> Active` owner-state machine,
retry-stable publication journals, encrypted local-secret custody, and
byte-oriented bootstrap/local-state store contracts.

The crate also supplies the production object-safe application repository
factory over an injected VLT-PM02 store. It derives no address itself, requires
an unlocked authority-anchored verifier at connection time, consumes exact
publication batches by value, and translates storage and integrity failures to
a closed payload-free application error surface.

Generation-zero preparation is also complete as a pure no-write boundary. It
consumes an owned zeroizing passphrase, bounded KDF policy, advisory time, and
one caller-filled CSPRNG block, then returns the exact `PreparedInit` state,
bootstrap locator, repository address, and mandatory verifier. All root,
signing, device, passphrase, and randomness material is held in wipe-on-drop
containers.

Durable `PreparedInit` journals can be passphrase-rehydrated after process
loss without any external write. Rehydration authenticates the root wrap,
re-derives the repository address, decrypts local custody, and proves that all
three private seeds reproduce the authority and device public identities
pinned in the bootstrap and certificate before rebuilding the verifier.

Generation-zero side effects are crash-resumable over injected stores. The
completion workflow atomically installs the exact `PreparedInit` bytes before
the first external write, performs idempotent bootstrap and repository work,
verifies the exact resulting pins, and compare-exchanges to `Active`. A retry
after any failure rehydrates and reuses the same signed and randomized journal.

Stable active vaults can also be reopened from only their durable locator and
injected stores. Reopen strictly decodes local state, requires the exact latest
authority-signed bootstrap pinned locally, authenticates the passphrase root
wrap, reproduces every persisted public identity from encrypted local seeds,
derives the opaque repository address, and performs a complete repository open
relative to non-empty local head pins. The returned session owns wipe-on-drop
keys and exposes only identities, pins, and the payload-free verified report.

Interrupted item publications are recoverable from their exact durable
`PendingPublication` journal. Recovery authenticates the same bootstrap and
local custody boundary, republishes the already-randomized and already-signed
batch by value, requires the exact expected receipt heads, and only then
compare-exchanges the complete intended `Active` state. Ambiguous provider
failure retains byte-for-byte retry state, and an identical concurrent local
winner is the only compare-exchange conflict accepted as success.

Authenticated reopen now materializes the complete current application view.
It reads every verified maximal head, decrypts each distinct catalog exactly
once, unions identical and concurrent candidate references without loss, and
decrypts every distinct current revision. The open fails closed on dangling,
wrong-kind, cross-item, missing direct-parent, or amplified references. The
unlocked session retains the resulting wipe-on-drop domain candidates while
exposing only payload-free item, candidate, and conflicted-item counts.

Unlocked sessions now provide ordinary redacted current-item reads. A single
live candidate is projected through the typed domain `RedactedItemView`, while
a current tombstone is absent. Any multi-candidate item fails the complete read
with `ConflictRequired`; the application never selects an arbitrary winner or
returns a partial list, and every candidate remains available for later
resolution. Returned lists are deterministic by exact item-ID bytes and their
display metadata is wiped on drop by the domain view types.

Authenticated reopen also builds a session-owned, rebuildable search
projection. The application admits only the VLT-PM05 allowlist of redacted
titles/labels, usernames, URLs, services, database hosts, and present tags;
secret and non-allowlisted fields never enter it. Owned queries are limited to
1–256 UTF-8 bytes, reject control characters, and are normalized with Unicode
lowercase plus NFC. The trigram primitive accelerates eligible searches, while
exact post-filtering and bounded fallbacks preserve correct substring behavior
for each whitespace-delimited token, including short queries and unusually
large safe metadata. Results can apply one
explicit collection filter and are always re-ordered by normalized title,
schema, and item-ID bytes. Conflicts fail the complete search closed, ordinary
diagnostics expose only an indexed-item count, and dropping the unlocked
session clears the accelerator and wipes normalized metadata.

Unlocked sessions can add one new item through the same durable publication
state machine. The host supplies one exact 256-byte CSPRNG block for a fresh
item ID and the three encrypted frames. The application creates a parentless
item revision, rewrites the complete bounded catalog without discarding any
candidate, makes every current head a parent of the signed commit, and moves
the local owner state through an exact compare-exchanged pending journal before
publishing. Ambiguous writes accept only the byte-identical intended state,
and a provider or final-local-write interruption remains recoverable by the
existing pending-publication workflow. The mutation consumes its unlocked
session so callers must reopen before observing or mutating the new pins.

Compare-and-replace is available through the same session-consuming boundary.
It requires the requested item to have exactly one current live candidate equal
to the caller's expected revision, then writes a new revision whose sole direct
causal parent is that expected revision. Item identity, content schema, and
creation time are immutable; every unrelated catalog candidate is preserved.
Absent items return the payload-free `NotFound` error, while stale, tombstoned,
or conflicted candidates return `ConflictRequired` before any local write.
Replacement uses an owned wipe-on-drop 240-byte entropy block and the same exact
pending journal, all-head commit parenting, ambiguous-success rules, and
recovery path as add-item.

Deletion likewise consumes the unlocked session and requires the caller's
expected revision to be the sole current live candidate. It publishes a new
tombstone revision whose sole causal parent is that live revision, preserves
all unrelated catalog candidates, and keeps deletion time separate from the
commit's advisory wall time. Missing revisions return `NotFound`; conflicts and
repeat deletion return `ConflictRequired` before any local write. Authenticated
reopen retains the tombstone for history and restore while ordinary get, list,
and search views omit the deleted item. The three deletion frames use a
caller-owned wipe-on-drop 240-byte entropy block and the shared exact pending
journal/recovery state machine.

Item history walks verified commit ancestry from every current head and
decrypts each distinct catalog needed to collect revisions for one requested
item. Results are bounded to 1–4,096 entries (100 by default), deduplicated
across heads, and ordered newest ancestry depth first with exact commit and
revision object IDs as deterministic tie-breakers. Each result exposes only a
redacted live view or a tombstone marker, direct-parent count, and the
document-update or deletion advisory time; revision IDs require an explicit
accessor and ordinary diagnostics redact them. Historical candidates remain
inside the unlocked session so restore-by-revision can consume authenticated
content without adding provider-specific reads or asking a host for plaintext.

Restore-by-revision consumes the unlocked session and proves the requested
revision appears in a catalog reachable within the 4,096-entry ancestry bound
from a current head. The selected revision must be live, differ from the sole
current candidate, and belong to the same current item; unresolved current
conflicts fail closed. Restoration copies the authenticated historical document
into a newly randomized revision whose sole direct causal parent is the
selected revision, then uses the same all-head commit, exact pending journal,
ambiguous-success handling, and recovery path as other mutations. It never
rewinds heads, mutates historical bytes, or asks the host to resupply plaintext.

Current conflicts can now be inspected without revealing secret fields. The
session returns every retained candidate in exact revision-ID order using the
same typed redacted live view or tombstone marker as history. Choosing one
authenticated candidate consumes the session and republishes its complete live
document or tombstone as a new revision whose direct parents are the entire
current conflict set. The resolution never selects implicitly, drops a losing
candidate, rewrites history, or asks the host to round-trip plaintext. A later
slice will add user-authored merged-document resolution after explicit field
reveal.

An unlocked session can explicitly reveal one exact reachable live revision.
The application proves reachability through the same bounded verified history
walk, rejects tombstones and missing revisions, and moves the authenticated
document into an owned `Zeroizing<ItemDocument>`. That wrapper is neither
printable nor cloneable and wipes the secret-bearing payload and tags on drop;
the application never guesses which current, conflicted, or historical
candidate a host intended.

Hosts can now narrow that exact revision to one schema-specific first-party
secret field. The selector covers login passwords, secure-note bodies, card
numbers and CVVs, raw TOTP seeds, API tokens, and database passwords; a selector
for the wrong schema or an opaque record fails closed. The resulting
`RevealedSecretV1` distinguishes UTF-8 from binary bytes but implements no
printing or cloning and wipes its full allocation on drop.

Every field disclosure also declares one policy path: clipboard with no secret
stdout, reveal after a confirmed controlling-TTY ceremony, or explicitly
unsafe non-interactive output after both flag opt-in and a host-emitted stderr
warning. The application validates those facts before repository traversal.
Actual TTY inspection, warning rendering, clipboard writes, ownership checks,
and timed clear remain host responsibilities.

The crate accepts key and randomness material from its caller. It does not own
a filesystem path, provider SDK, network client, process, environment, clock,
credential store, or entropy source. User-authored conflict merging, host field
clipboard implementation, export, audit, and doctor workflows land in the next
slices.
There is no unchecked
repository verification path: construction decrypts and
authority-verifies the exact locally pinned certificate frame and object ID,
and commits and announcements must match that vault, device, certificate ID,
and Ed25519 key.

## Verification

The package tests cover exact canonical and cryptographic vectors,
lossless removed-value persistence, live and tombstone revisions, catalog
bounds and ordering, cross-kind and cross-vault rejection, AEAD tampering,
authority/device/certificate binding, Ed25519 signature rejection, closed
parser failures, crash-state relationship checks, diagnostic redaction, and
explicit key wiping. Open tests additionally prove empty and conflicted current
catalog materialization plus dangling current-revision, missing-parent, and
cross-item rejection. Repository tests exercise initialization, publication,
verified open/read/history, every injected provider operation, constructor
paths, and complete error translation.
Search tests cover Unicode normalization, short queries, oversized safe-field
fallback, exact collection filters, deterministic product ordering, every
record schema's allowlist/denylist, secret exclusion, query/result bounds, and
conflict closure. The exact Tarpaulin LLVM result is 2,250 of 2,361 production
lines (95.30%). Add-item tests cover exact entropy partitioning and wiping,
identity validation before local writes, parentless revision and complete
catalog construction, ambiguous successful compare-exchanges, write-ahead
publication ordering, ambiguous provider commits, failed final activation,
and recovery through the exact persisted journal. Replacement tests prove
one-parent causality, immutable identity fields, complete catalog preservation,
stale/missing rejection before compare-exchange, and secret/entropy wiping.
Deletion tests prove one-parent tombstone causality, advisory timestamp
separation, ordinary-view omission, repeat/missing rejection before
compare-exchange, and entropy redaction/wiping.
Conflict-resolution tests prove deterministic redacted inspection, live and
tombstone selection, complete multi-parent causality, immutable losing-candidate
retention, missing/unconflicted rejection before compare-exchange, and entropy
redaction/wiping.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application --all-targets -- -D warnings
```
