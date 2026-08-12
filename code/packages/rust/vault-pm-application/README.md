# `coding_adventures_vault_pm_application`

The host-neutral VLT-PM05 application core for the local-first password
manager. The current slices define lossless canonical persistence for local
device secrets, item revisions, catalog snapshots, and signed VLT-PM15
operation events, plus domain-separated V1 encrypted object framing and an
authority-anchored repository verifier for the single authorized Phase 1A
device. Audit events use their own authenticated object-kind domain; durable
owner state can now journal a redacted per-device event head and refuses to
activate a later publication that omits or reuses that head. Once such a head
exists, every item mutation and portable import constructs an encrypted signed
event and advances it atomically with the repository commit. Legacy local state
decodes with auditing disabled. Complete audit verification now follows any
durable event head back to its explicit genesis and authenticates every event
against its signed repository commit. A dedicated audit-only journal can now
advance that event head and device counter while reusing only the exact active
catalog root; ambiguous provider success retains and replays the exact pending
bytes. A first session-consuming audited access boundary now publishes an
access success or post-authentication failure before releasing any redacted
list, one-item show, search, bounded history, or conflict-candidate result.
Successful item reads bind the exact selected live revision; missing and
tombstoned items become audited `NotFound` results. All covered methods share
one publish-before-release completion path, and publication failure releases
neither result nor original error while leaving the exact pending journal
recoverable. Complete verification, coarse diagnostics, and encrypted portable
export now use that same boundary, including failed export-input attempts.
Exact current-revision capabilities, whole secret-bearing documents, and
schema-specific secret fields are also held until their succeeded, denied, or
failed event is durable. The production migration boundary can start one
explicit audit epoch. Explicit authenticated audit-history reads now publish
their own successful `AuditRead` event first, re-verify the complete newly
advanced chain, and return a bounded newest-first secret-free projection or an
exact trace lookup. The projection includes stable item and revision selectors
only for this explicit surface; default debug output redacts every stable
identity. An authenticated host-side item-create input failure can now consume
the unlocked session through a dedicated boundary that publishes a failed
`ItemCreate` event against the already-reserved item identity before returning
control to the host. CLI rendering remains a later slice. It also defines the
exact canonical
`PreparedInit -> Active -> PendingPublication -> Active` owner-state machine,
retry-stable publication journals, encrypted local-secret custody, and
byte-oriented bootstrap/local-state store contracts.

An unlocked pre-audit vault can now begin its one explicit audit epoch through
a production session-consuming boundary. The successful `AuditEpochStart`
event has no fabricated predecessor, advances the owner-private event head and
device counter through the exact audit-only pending journal, and returns only
after the next owner state is durable. Repeat activation fails closed. Hosts
must not expose this transition until every authenticated path can continue
the chain or fail closed.

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

Optimistic hosts can separately request the exact sole current live revision
for a later compare-and-swap mutation. Missing/tombstoned items return no
capability and conflicts fail closed. Revision identities remain absent from
ordinary redacted item views and public diagnostics.

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

Mutation entropy blocks reserve one independent operation trace and one audit
object frame in addition to the existing revision, catalog, and commit material.
When an audit epoch is active, create, update, delete, restore, conflict choice,
authored conflict merge, and portable import bind their action, exact basis
heads, device counter, prior event, selected revision where applicable, and
result revision into the event. The event object, logical mutation objects, and
commit share the same write-ahead journal and activation compare-exchange.

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
candidate, rewrites history, or asks the host to round-trip plaintext.

After an explicit host-controlled field-reveal ceremony, a user may instead
author one complete merged document. The application accepts that owned
secret-bearing document only for an actual current conflict, requires at least
one live candidate, preserves every live candidate's immutable schema and
creation time, and publishes the complete conflict set as causal parents. The
document and session are consumed on every return path, while all prior
candidate bytes remain immutable reachable history.

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

Audited variants consume the session before returning an exact current
revision capability, whole secret-bearing document, or selected field. They
publish the item and exact successfully reached revision before release;
unconfirmed disclosure ceremonies publish `Denied` without revision traversal,
while missing revisions and field/schema mismatches publish `Failed`. An audit
publication failure withholds the capability, secret, and original error.

Long-lived hosts can retain an explicit `VaultAccessV1` lifecycle boundary.
It begins with only a redacted `LockedVaultV1` locator handle, authenticates a
complete verified session in place, returns the stable payload-free `Locked`
error when a caller asks for session access too early, and synchronously drops
the live session on `lock()`. A failed unlock leaves the boundary locked;
repeated lock is idempotent. The unlocked variant is boxed so the host state
object stays compact without copying key material.

That boundary now exposes a safe status workflow for CLI and later UI hosts.
While locked it strictly decodes the bounded owner-private record to report
only `Absent`, `Prepared`, `Locked`, or `RecoveryRequired`; it does not access
bootstrap or repository providers. While unlocked it reports `Unlocked` plus
authenticated aggregate item, candidate, and conflicted-item counts. Counts
are omitted in every other state, and status diagnostics contain no vault,
device, item, revision, object, locator, or provider identity.

An unlocked session can now run a complete read-only integrity audit. The
workflow repeats verified repository discovery relative to durable local pins,
requires one exact pinned commit matching the local device counter, catalog,
and certificate, walks complete bounded ancestry from every current head, and
authenticates and decrypts every distinct reachable catalog and catalog-named
item revision. If an audit epoch exists, it also decrypts the complete bounded
per-device event chain, verifies every event with the authority-certified
device key, and cross-checks its counter, basis heads, timestamp, event-object
membership, selected revision, and mutation result against the corresponding
signed commit. It rejects chain cycles, counter gaps, skipped durable heads,
wrong roots, signers, or commit bindings. Success returns only aggregate
announcement, commit, catalog, revision, item, and audit-event counts plus
`integrity_verified = true`; pre-audit vaults report zero events. Any provider,
format, graph, anchor, cryptographic, or domain failure returns the closed
application error taxonomy without a partial report.

The audited verification method consumes the unlocked session and releases
this aggregate report or its closed failure only after publishing a signed
`VaultVerify` event and durable next owner state.

The lifecycle boundary also exposes a read-only `doctor` workflow with one
closed coarse outcome. Locked checks distinguish absent/prepared
initialization, pending-publication recovery, owner-state availability,
bootstrap availability, unsupported persisted versions or suites, integrity
failure, and the authentication required before repository verification.
Unlocked checks first require the exact durable active state and signed
bootstrap retained by the session, then run the complete audit to distinguish
healthy, repository-unavailable, unsupported, and integrity-failure states.
The report carries no counts or vault, device, item, revision, object, locator,
or provider identities, and the workflow never repairs state or accepts pins.
The audited unlocked workflow likewise consumes the session and publishes a
signed `VaultDiagnose` event before releasing this coarse report; an unhealthy
report is still a successful completed diagnosis.

An unlocked session can now produce one canonical authenticated portable
export. The caller supplies the exact active signed bootstrap, a separately
collected owned passphrase, a bounded Argon2id policy, and exactly 40 host-CSPRNG
bytes for the fresh salt and XChaCha20 nonce. The encrypted snapshot retains
every current live, tombstone, and conflict candidate in deterministic
item/revision order and binds its exact bootstrap, candidate count, and
domain-separated snapshot hash. Header-bound AEAD authenticates the version,
protection mode, suite, KDF policy, salt, and nonce. The artifact excludes
owner-private state, private seeds, provider credentials, local pins, journals,
and search data; public diagnostics reveal no bytes, while passphrase, key,
plaintext, encoded revisions, and hash preimages are wiped on drop.

The host receives only exact encrypted bytes and remains responsible for an
explicit destination and safe file persistence. The audited export workflow
holds those bytes until its signed `PortableExport` event and next owner state
are durable, and records invalid export inputs as failed attempts. A separate
session-consuming host-failure boundary lets CLI composition publish a failed
itemless `PortableExport` event when distinct-passphrase collection fails after
an active-epoch unlock, without passing a partial secret or destination into
the event.

Untrusted artifacts can now be opened through a separate no-write boundary.
The caller supplies the owned export passphrase plus an explicit maximum
Argon2id memory/iteration/lane policy, preventing artifact-controlled resource
cost beyond host approval. Opening strictly checks the bounded canonical
header, authenticates header-bound AEAD before plaintext parsing, verifies the
snapshot count/hash and signed source bootstrap, enforces deterministic unique
source item/revision order and per-item bounds, and decodes every candidate with
exact item binding. Wrong credentials and valid-shape tag tampering share the
closed authentication failure.

Success remains inside an opaque non-cloneable application object exposing only
item and candidate counts. Source identities, bootstrap bytes, metadata, and
documents have no public accessor; diagnostics are redacted and all intermediate
secret-bearing buffers and CBOR trees wipe on every path.

An untouched empty generation-zero target can consume that opaque snapshot in
one atomic cross-vault import. The host asks the application for the exact
count-derived CSPRNG byte requirement, then supplies the owned entropy block and
an advisory commit time. Import rejects the source vault, a mutated/non-empty
target, stale pins, identity collisions, and snapshots that exceed one 4,096
object publication. It creates a new target item identity per source item and a
new encrypted target revision per retained live, tombstone, or conflict
candidate, preserves complete validated record/CRDT/deletion state, intentionally
drops non-portable source causal-parent identities, and seals a new target
catalog and signed commit. The ordinary write-ahead pending journal makes the
complete restore crash-resumable without partial logical visibility. The source
snapshot, imported plaintexts, and entropy remain non-printable and wipe on
drop.

An audit-enabled target may retain any number of audit-only attempts while
remaining logically empty. Host/artifact failures publish itemless failed
`PortableImport` events, target-side validation failures publish before their
closed error, and success keeps its `PortableImport` event atomic with every
re-identified candidate and the new catalog.

Before import consumes an authenticated snapshot, the application can derive
an opaque `PortableRestoreExpectationV1`. It normalizes only the item identity
that import replaces, canonical-encodes every complete live or tombstone state,
hashes sorted candidate groups, and retains source identities for disjointness
checks. An independently reopened target can consume that token through
`audited_verify_portable_restore`: source vault/item/revision reuse, retained
causal parents, grouping drift, or any schema, timestamp, deletion, CRDT, or
record-value change yields one closed integrity failure. Match and mismatch
publish a dedicated itemless `PortableRestoreVerify` event before the caller
receives an aggregate count proof or the error.

The crate accepts key and randomness material from its caller. It does not own
a filesystem path, provider SDK, network client, process, environment, clock,
credential store, or entropy source. Host field clipboard implementation,
portable target creation and CLI restore-verification composition, and richer
host-side path, authorization, quota, and cache checks land in later slices.
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
explicit key wiping. Status tests prove every coarse lifecycle state,
unlocked-only counts, diagnostic redaction, strict corrupt-state rejection,
and closed local-store error translation. Open tests additionally prove empty
and conflicted current catalog materialization plus dangling current-revision,
missing-parent, and
cross-item rejection. Repository tests exercise initialization, publication,
verified open/read/history, every injected provider operation, constructor
paths, and complete error translation.
Audit tests cover complete re-discovery and ancestry traversal, aggregate-only
diagnostics, historical catalog/revision counts, and exact local-anchor
rejection. Repository tests also prove the complete security-history seam uses
the same deterministic order as bounded interactive history.
Doctor tests cover absent, prepared, recovery-required, locked-authentication,
healthy, local/bootstrap/repository unavailable, unsupported-version, and
integrity-failure outcomes; exact durable session binding; aggregate-free
diagnostics; and failure without repair.
Search tests cover Unicode normalization, short queries, oversized safe-field
fallback, exact collection filters, deterministic product ordering, every
record schema's allowlist/denylist, secret exclusion, query/result bounds, and
conflict closure. The exact Tarpaulin LLVM result is 3,086 of 3,245 production
lines (95.10%); portable export/opening is 374 of 401, mutation including
portable import is 475 of 504, doctor is 44 of 45, and status remains 46 of 46.
Add-item tests cover exact
entropy partitioning and wiping,
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
tombstone selection, authored merged-secret persistence, complete multi-parent
causality, immutable live-identity preservation, immutable losing-candidate
retention, missing/unconflicted/all-tombstone rejection before
compare-exchange, and entropy redaction/wiping.
Portable-export/open/import tests prove the exact canonical encrypted vector,
separate passphrase authentication, header/ciphertext tamper rejection, exact
active bootstrap binding, plaintext-secret exclusion, complete snapshot
count/hash, signed-bootstrap validation, host KDF-ceiling enforcement,
candidate identity and ordering validation, live-document recovery, bounded
credential rejection, lossless retention of every current conflicting
tombstone, exact import entropy sizing, cross-vault item/revision
re-identification, target-only encryption, atomic publication, and
conflict/deletion preservation. Restore-verifier tests independently reopen the
target, match canonical normalized candidate groups, reject same-count semantic
drift, source identity reuse, and retained parents, and prove failed/succeeded
comparison events are durable before their result.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_application --all-targets -- -D warnings
```
