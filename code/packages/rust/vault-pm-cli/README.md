# `coding_adventures_vault_pm_cli`

This crate is the first real composition layer for the local-first password
manager. It owns strict parsing, stable exit classes, redacted rendering, and
the bounded `init`, locked `status`/`doctor`, authenticated `audit enable` and
`audit verify`, explicit `audit list`/`audit show TRACE`, and opt-in full
`doctor --unlock` workflows. It now also owns the first usable item vertical:
authenticated login, secure-note, payment-card, API-key, static
database-credential, and TOTP creation plus durable redacted list/show. The same one-shot
boundary now supports revision-safe login
replacement. The
newest-first `history list ITEM` projection exposes canonical revision
selectors plus safe causal metadata without opening historical secrets. The
same selectors now support reversible `item delete ITEM` and `history restore
ITEM REVISION`. `conflict list ITEM`,
`conflict reveal ITEM REVISION FIELD`, `conflict choose ITEM REVISION`, and
`conflict merge login ITEM BASE_REVISION`, and
`conflict merge secure-note ITEM BASE_REVISION` expose redacted current
candidates, explicit audited field inspection, choose-existing resolution, and
complete user-authored login/secure-note merges.
`item reveal ITEM FIELD` adds explicitly confirmed, publish-before-release
interactive access to one schema-specific current secret without returning the
revision capability or complete document to CLI orchestration.
`export FILE`, `import FILE`, `restore verify FILE`, and the
composed `--vault TARGET restore FILE` add the encrypted recovery-artifact
round trip, retryable independent verification, and a completed-and-verified
ceremony. The executable is a thin caller of this package.

The driver composes the existing storage-neutral application over separately
permission-checked application-state and encrypted-object filesystem roots.
It acquires the persistent cross-process writer lock before loading
configuration or constructing either backend. Configuration is parsed by the
closed VLT-PM07 codec. The first vault's filesystem location must exactly match
the prepared platform object root; named targets use only their
locator-derived child roots before storage is touched.

Initialization collects a new passphrase only through the injected fixed
secret-input boundary, fills the complete generation-zero randomness block
from the injected OS entropy boundary, and uses a fixed production Argon2id
floor. New vaults use the audit-first application boundary: the initial commit,
prepared journal, and intended active state all bind one signed encrypted
`VaultInitialize` genesis event. It durably installs the exact `PreparedInit`
owner journal before publishing the configuration that makes the random
locator discoverable. A restart with a prepared journal collects the existing
passphrase and resumes the exact journal without generating new identities,
trace IDs, audit events, or ciphertext.

`vault create NAME` repeats that audit-first ceremony for a separately keyed
empty target. It durably installs the exact encrypted creation trace before a
configuration compare-exchange makes the locator discoverable, allocates a
distinct filesystem-adapter namespace, preserves the existing default, and
resumes an exact prepared target after a crash without new entropy. A leading
`--vault NAME` selects any configured target for one command without mutating
configuration. Authenticated operations therefore advance only the selected
vault's independent audit chain, while portable import and restore verification
can target a new vault without overwriting or switching the source.

Status and plain doctor do not prompt or unlock. Authenticated audit and doctor
commands collect the existing passphrase through the controlling terminal,
open the storage-neutral repository for one action, and synchronously drop the
live session before rendering. Verification and diagnostic projections contain
only closed labels or aggregate counts, including the number of fully
authenticated encrypted operation events (zero for pre-audit vaults), with no
paths, locators, providers, identities, or cryptographic details. No command
accepts a passphrase through argv, stdin, environment, configuration, or URL.

New CLI vaults are already audit-enabled at generation zero, so `audit enable`
returns an idempotent no-write `already enabled` success for them. For legacy
pre-audit owner state, `audit enable` remains the explicit one-time boundary
between the declared unlogged historical prefix and its signed operation-event
epoch. It consumes
the unlocked session through the existing crash-resumable audit-only journal
and returns only after the successful `AuditEpochStart` event and next owner
state are durable. Repeating the command is a no-write success. Once enabled,
the epoch cannot be disabled or silently bypassed by an authenticated item or
verification command.

`audit list` is the explicit authenticated exception to default identity
redaction. It publishes its own successful `AuditRead`, fully verifies the
newly advanced signed chain, and renders at most 100 newest-first rows. Each row
contains the canonical trace, device-local counter, closed action/outcome,
advisory time, and only the item/revision selectors structurally present in the
event. `audit show TRACE` strictly parses one canonical trace, records another
`AuditRead`, and returns exactly the matching row or a durable `not found`.
Neither command emits vault/device identities, heads, signatures, storage
details, record metadata, or secret data.

When an audit epoch already exists, `audit verify`, `doctor --unlock`, `item
list`, `item show`, and `history list` consume the unlocked session through the
application's signed publish-before-release boundary. Output is constructed
only after the event and next owner state are durable. Pre-audit vaults retain
their backward-compatible ordinary read behavior until explicit `audit enable`;
the audit-history surface itself requires that epoch.

`item add login` unlocks once, collects bounded fields from the controlling
terminal, obtains fresh mutation and metadata identities from OS entropy, and
consumes the session through the crash-resumable application mutation.
Time, the item identity, mutation entropy, and audit-failure entropy are
reserved before authentication; after an active-epoch unlock, any item-form
prompt failure publishes a failed traceable `ItemCreate` event before its
closed CLI error becomes observable.
`item add secure-note` reuses that create boundary, collects a required title
and a required single-line body through the fixed hidden `Note:` prompt, and
publishes the same atomic `ItemCreate` event on success. List output contains
only the escaped title; show output contains `Body: <redacted>`. The body never
enters normal CLI rendering or audit projections.
`item add card` again reuses the boundary after reserving every identity and
audit input before unlock. It collects bounded title, holder, expiry, and
optional postal metadata plus hidden wipe-on-drop PAN and CVV. Post-unlock host
or validation failures publish `ItemCreate Failed`; success atomically
publishes the typed encrypted card and `ItemCreate Succeeded`. Show renders
only holder, last four, expiry, postal-presence, and explicit PAN/CVV redaction;
the complete PAN and CVV require the separate audited `item reveal` ceremony.
`item add api-key` composes the existing typed record through the same
audit-first boundary. It collects bounded label, service, scope, and optional
expiry metadata plus one hidden wipe-on-drop token. Scope and expiry
validation failures become durable failed `ItemCreate` events; success
atomically publishes the encrypted record and succeeded event. Show renders
only label, service, scopes, expiry, and explicit token redaction. Complete
token access remains a separate audited `item reveal ITEM api-key-token`.
`item add database-credential` uses that boundary for static connection
metadata and one hidden password. It admits only a canonical local engine
identifier and nonzero decimal TCP port, assigns no dynamic lease metadata,
and publishes validation failures before returning. Show receives only the
redacted domain projection: label, engine, host, port, optional database,
username, absent lease/expiry, and a password omission marker. Password access
requires `item reveal ITEM database-password`.
`item add totp` completes the first-party record set through the same audited
create boundary. It accepts one hidden canonical unpadded Base32 seed and
closed SHA1/SHA256/SHA512, 6-or-8 digit, and 1–3600 second metadata. Show
receives only label, optional issuer, algorithm, digits, period, and a secret
omission marker. `item reveal ITEM totp-secret` publishes the access event
before encoding the selected raw bytes into a wipe-on-drop canonical Base32
buffer for direct terminal delivery.
`item list` and `item show ITEM` reopen in separate one-shot sessions and
render only escaped redacted projections; login passwords, note bodies, PANs,
CVVs, postal values, API-key tokens, database passwords, and TOTP seeds are
never available to the renderer. In an active
epoch, both commands first make their exact signed access outcome durable.

`search QUERY` rebuilds the storage-neutral in-memory search projection during
one authenticated session and drops it on lock. The query is moved into a
wipe-on-drop, debug-redacted owner and is never echoed. Search uses only the
application allowlist of redacted metadata, aborts on any current conflict,
returns at most 100 deterministic item-list rows, and never writes index or
query bytes to a provider. In an active epoch, valid zero/nonzero results and
invalid semantic queries publish `ItemSearch` success/failure before output or
the closed error is released.

`item edit ITEM` asks the application for an opaque edit preparation that owns
the authenticated current revision and wipe-on-drop secret document without
returning either to CLI orchestration. It preserves immutable identity and
unedited metadata, collects the complete bounded login form again—including a
canonical zero-to-sixteen URL count, ordered URLs, and optional hidden
notes—and consumes the preparation through the crash-resumable replacement
compare-and-swap. Existing multi-URL records are accepted and the entire URL
list and notes are replaced without implicit preservation. Show exposes only
notes presence; `item reveal ITEM login-notes` uses the separate audited direct
terminal ceremony. In an active audit epoch, missing/conflicted/unsupported
targets and prompt, count, entropy, or document-validation failures publish a
failed `ItemUpdate` event before the CLI exposes their error; success publishes
its event atomically with the new revision.

`history list ITEM` reopens the authenticated repository, traverses at most the
application history bound, synchronously locks, and then renders each unique
revision as a canonical selector, live/deleted state, direct-parent count,
advisory timestamp, and—only for live revisions—the schema and escaped title.
Passwords and notes bodies never enter the redacted projection; usernames,
URLs, paths, object frames, and cryptographic details are never emitted by the
history renderer.

`item delete ITEM` passes only the stable item identity into an application
boundary that selects the sole current live revision and consumes the session
through an immutable causal-tombstone mutation. In an active audit epoch, a
successful delete publishes its signed event atomically with that tombstone;
missing, already-deleted, and conflicted attempts publish a failed delete event
before the CLI exposes the closed error. The exact optimistic revision
capability never crosses into CLI orchestration.
`history restore ITEM REVISION` passes both user selectors into one bounded
application boundary, which proves the revision belongs to that item's history
and copies it into a new current revision without returning the history
projection to CLI orchestration. In an active audit epoch, the successful
restore event and new revision publish atomically. Missing, cross-item,
tombstone, same-revision, and conflicted attempts publish a failed restore event
before their closed error is exposed. Neither operation erases history,
rewinds repository heads, or emits secret-bearing metadata.

`conflict list ITEM` requires an active audit epoch and publishes its
item-scoped history-read outcome before rendering the current candidates in the
existing secret-free history row format. `conflict choose ITEM REVISION`
reserves mutation and failure-audit entropy before unlock, validates both
selectors inside the application, and publishes either a failed attempt or an
atomic all-current-parent resolution event before its closed outcome. It emits
only the resolved item selector and never deletes losing immutable history.

`conflict reveal ITEM REVISION FIELD` requires the named revision to belong to
the named item's current conflict set before it may disclose one closed
schema-specific field. It reuses the exact-`yes` controlling-terminal ceremony
and direct escaped delivery from `item reveal`: refusal publishes `Denied`
without candidate traversal; unconflicted, missing, noncandidate, tombstone,
and wrong-field attempts publish `Failed`; success binds item and exact
revision before the secret is released. No outcome selects a candidate,
changes the conflict, enters ordinary process output, or persists plaintext.

`conflict merge login ITEM BASE_REVISION` retains one exact current live login
as an opaque metadata base and collects the complete login payload through the
same bounded controlling-terminal form as create/edit. Existing candidate
secrets are never prefilled or returned to the CLI. Invalid selectors, prompt
or entropy failures, and invalid forms publish failed item-scoped
`ItemConflictMerge` events before their closed outcome; success atomically
publishes one all-current-parent revision and a succeeded merge event. The only
output is the merged item selector. Other record schemas remain later slices.

`conflict merge secure-note ITEM BASE_REVISION` applies the same opaque-base,
all-current-parent, item-scoped audit ordering to a complete title and hidden
body form. It never prefills or exposes a candidate body. Success emits only
the merged item selector; failures emit no partial output.

`conflict merge card ITEM BASE_REVISION` extends that ceremony to a complete
payment-card form. PAN and CVV use hidden prompts; month, year, and digit-shape
validation is repeated inside the application-owned preparation so invalid
forms are audited before their closed error. The base and every former
candidate value remain opaque, success names all current candidates as parents,
and ordinary output contains only the merged item selector.

`conflict merge api-key ITEM BASE_REVISION` extends that ceremony to a complete
API-key form. The token uses a hidden prompt, and the scope and expiry lines are
forwarded verbatim so their closed shape is validated inside the
application-owned preparation and invalid forms are audited before their closed
error. The base and every former candidate value remain opaque, success names
all current candidates as parents, and ordinary output contains only the merged
item selector.

`conflict merge database-credential ITEM BASE_REVISION` extends that ceremony
to a complete database-credential form. The password uses a hidden prompt, and
the engine and port lines are forwarded verbatim so their closed shape is
validated inside the application-owned preparation and invalid forms are
audited before their closed error. The merged record is always a static
credential with no lease. The base and every former candidate value remain
opaque, success names all current candidates as parents, and ordinary output
contains only the merged item selector.

`conflict merge totp ITEM BASE_REVISION` extends that ceremony to a complete
TOTP form. The seed uses a hidden prompt, and the Base32 line and every
parameter are forwarded verbatim so their closed shape is decoded and validated
inside the application-owned preparation and invalid forms are audited before
their closed error. Every `TOTP_SEED_V1` field is authored, so nothing carries
over from the base candidate. The base and every former candidate value remain
opaque, success names all current candidates as parents, and ordinary output
contains only the merged item selector.

`conflict merge opaque ITEM BASE_REVISION` closes the family with the one record
type that has no schema. There is no field list to fill in, so the form is a
single hidden prompt for the complete canonical-CBOR payload as lowercase
hexadecimal; it is hidden because an unknown schema offers no way to show that
any part of the payload is not a secret. The line is forwarded verbatim so its
closed shape is decoded and validated inside the application-owned preparation
and invalid payloads are audited before their closed error. The content type is
inherited from the base rather than authored, since an item's schema is
immutable across its history. Choosing one existing candidate unchanged remains
`conflict choose ITEM REVISION`, which already works for any schema. The base
and every former candidate value remain opaque, success names all current
candidates as parents, and ordinary output contains only the merged item
selector.

`item reveal ITEM FIELD` requires an active audit epoch and accepts only closed
schema-specific selectors. It reserves time and audit entropy before
unlock, then requires exact `yes` through a fixed controlling-terminal prompt.
Refusal and prompt failure publish `Denied`; missing, conflicted, or
wrong-schema selections publish `Failed`; success binds the exact current
revision before returning a non-printable wipe-on-drop secret. The native host
writes a quoted, control-escaped `Secret: "..."` line directly to `/dev/tty` or
the attached console. TOTP seed bytes are first rendered as canonical Base32 in
a wipe-on-drop buffer. The secret never enters cloneable/debuggable `CliOutput`,
process stdout/stderr, arguments, stdin, configuration, or audit metadata.

`export FILE` reserves export and audit entropy before unlock, collects and
constant-time confirms a distinct export passphrase through two hidden fixed
prompts, and calls the application's canonical portable-export boundary. In an
active audit epoch, a prompt failure is durably recorded as failed
`PortableExport`; success publishes its event before releasing the encrypted
artifact. The native host then creates the explicit destination without
following or replacing an existing final path, requests mode `0600` on Unix,
writes and synchronizes the complete artifact, and returns only a path-free
success line. A destination write failure occurs after artifact release and
therefore does not rewrite the truthful successful access event.

`import FILE` requires the configured target to be independently initialized,
logically empty, and audit-enabled. After target unlock it reads one bounded
regular artifact, collects its passphrase through the fixed hidden
`Import passphrase:` prompt, authenticates and validates the complete snapshot
without writes, and obtains the exact count-derived CSPRNG block. Host,
artifact, and target failures publish a failed itemless `PortableImport`
before their error; success re-identifies every item/candidate and publishes
its event atomically with the new catalog. Output contains aggregate item and
candidate counts only. A later list/show reopens the target through the
ordinary audited redacted-read boundary.

`restore verify FILE` is a separately retryable, no-mutation ceremony against
the currently configured target. It reserves its trace/time/randomness before
target authentication, requires an active audit epoch, reads the artifact
under the same hard ceiling, collects the fixed hidden `Import passphrase:`,
and prepares the opaque application expectation. Source-read, prompt,
authentication, and preparation failures publish a failed itemless
`PortableRestoreVerify`; semantic match or mismatch publishes its own event
before aggregate proof or integrity error. Success prints item, candidate, and
conflict counts only. No path, source/target identity, semantic root, mismatch
location, record metadata, or field value reaches output or the audit event.

`--vault TARGET restore FILE` requires an explicit non-default named target and
composes those two application boundaries. It reserves independent import and
verification audit inputs before mutation, opens the artifact once, prepares
the opaque expectation before consuming the snapshot, and publishes the import
without releasing intermediate success. It then prompts for the target
passphrase again, independently reopens the durable target, and publishes the
verification result before emitting aggregate completed-and-verified output. A
post-import interruption leaves `restore verify FILE` as the safe retry path;
the command never repeats import against the now non-empty target.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli --no-deps
```
