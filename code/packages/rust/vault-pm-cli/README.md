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
`export FILE`, `import portable FILE`, `restore verify FILE`, and the
composed `--vault TARGET restore FILE` add the encrypted recovery-artifact
round trip, retryable independent verification, and a completed-and-verified
ceremony. `import bitwarden FILE` and `import csv FILE`
(`VLT-PM49-cli-external-import.md`) add two more, unencrypted, format
adapters: each decodes a competing product's plaintext export and creates new
items through the unmodified `item add` publication path, once per record,
rather than the disaster-recovery ceremony `import portable` uses.
`import kdbx FILE` parses but always fails closed with the `unsupported`
class before opening its file — KDBX's own encrypted container format is
explicitly deferred. `import otpauth-uri FILE`
(`VLT-PM49-cli-external-import.md` §5.5) adds a third, standalone-URI
adapter: a file holding one `otpauth://totp/...` URI — the shape a QR
code or an issuer's manual-setup page encodes — becomes a new TOTP item
through the same unmodified `item add` publication path. `import
otpauth-qr FILE` parses but always fails closed with `unsupported`
before opening its file — decoding a QR code *image* into that URI text
is explicitly deferred (§11). `storage add|list|check|migrate`
(`VLT-PM50-cli-storage-migration.md`, `VLT-PM00` §23 item 14) add named
storage locations, third-party sync-tool conflict-copy detection, and
byte-for-byte migration with a real independent-unlock verification step. The
executable is a thin caller of this package.

The driver composes the existing storage-neutral application over separately
permission-checked application-state and encrypted-object filesystem roots.
It acquires the persistent cross-process writer lock before loading
configuration or constructing either backend. Configuration is parsed by the
closed VLT-PM07 codec. A vault's storage may be any registered
`filesystem`/`removable` location — `configured_vault`'s check used to accept
only the two paths this composition root itself creates, back when `storage
add` did not exist; that restriction is gone. Every repository this crate
opens is wrapped in `vault-pm-storage::ReplicaSetObjectStore` (transparently,
with zero configured mirrors, unless a vault's `remote_stores` names one or
more `storage add`ed locations), so a vault mirrored with `storage migrate
--mirror` gets ongoing, best-effort mirror-write propagation on every later
mutation rather than a one-time copy.

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

`export FILE [--best-effort]` reserves export and audit entropy before
unlock, collects and constant-time confirms a distinct export passphrase
through two hidden fixed prompts, and calls the application's canonical
portable-export boundary. In an active audit epoch, a prompt failure is
durably recorded as failed `PortableExport`; success publishes its event
before releasing the encrypted artifact. The native host then creates the
explicit destination without following or replacing an existing final path,
requests mode `0600` on Unix, writes and synchronizes the complete artifact,
and returns only a path-free success line. A destination write failure
occurs after artifact release and therefore does not rewrite the truthful
successful access event.

Without `--best-effort`, one item this build cannot re-encode still denies
the whole export exactly as it always has — no caller's behavior changes
unless they ask for the flag by name (VLT-PM05 §13.9 closes the backlog item
that named this the still-open half of "an oversized poisoned record locks
edit/merge/export... delete still works"). With `--best-effort`, such an
item is excluded from the artifact instead, and a successful export that
excluded at least one item appends its count and every excluded item's
canonical id to standard output:

```text
Portable export written.
Excluded (too large to include): 2
<item id>
<item id>
```

Every printed id is already visible through this same vault's own
`item list`; an operator can `item delete ITEM` any of them and re-export
for a subsequently complete backup. The flag is recognized only in this
fixed position (`export FILE --best-effort`); `export --best-effort FILE`
does not parse, and a bare `export --best-effort` treats `--best-effort` as
the destination, per this command's pre-existing rule that a path
beginning with `-` is a path value whenever it is the sole positional
argument. See `VLT-PM17-cli-portable-export.md`'s amendment and VLT-PM05
§13.9 for the complete design.

`import portable FILE` requires the configured target to be independently
initialized, logically empty, and audit-enabled. After target unlock it reads
one bounded regular artifact, collects its passphrase through the fixed
hidden `Import passphrase:` prompt, authenticates and validates the complete
snapshot without writes, and obtains the exact count-derived CSPRNG block.
Host, artifact, and target failures publish a failed itemless
`PortableImport` before their error; success re-identifies every
item/candidate and publishes its event atomically with the new catalog.
Output contains aggregate item and candidate counts only. A later list/show
reopens the target through the ordinary audited redacted-read boundary.

`import bitwarden FILE` and `import csv FILE`
(`VLT-PM49-cli-external-import.md`) take a different shape entirely, because
the source is a plaintext export from a different product rather than
another vault-pm vault: no source passphrase, no empty-target requirement,
and no shared item-identity space to merge or restore against. Each reads
one bounded plaintext source through a new `Zeroizing`-returning host method
(`read_external_import_source`), decodes it with a small adapter crate
(`vault-import-bitwarden`, `vault-import-csv`) that has no vault-pm
dependency at all, and maps each decoded record onto vault-pm's own typed
records. Every mapped record is then created through the unmodified `item
add` publication path, once per record — no new mutation or audit primitive
was introduced — so it carries the exact same `ItemCreate` audit event and
crash-resumable publication a manually typed item does. A record whose kind
has no vault-pm equivalent (an SSH key, a Bitwarden identity) is *skipped*,
counted, and never silently dropped. Output is
`Import complete: created=C skipped=S failed=F` — aggregate counts only,
never a title, username, URL, or secret. `import kdbx FILE` parses the same
shape but always answers the closed `unsupported` exit class before opening
its file: KDBX's own Argon2d/AES-or-ChaCha20 encrypted container is
explicitly deferred (VLT-PM49 §8), not silently missing from the grammar.

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

## Foreground interactive shell

`vault-pm [--vault NAME] shell` is a second *host shape* for the commands above,
not a second set of commands. It exists because the one-shot model asks for a
passphrase — and pays a full Argon2id derivation — once per operation, which is
unusable for a working session.

A session binds exactly one vault when it starts: the named selector, or the
configured default as it stood at that moment. Every delegated command carries
that name explicitly, so a retained authenticator can never be presented to a
target the user did not authenticate against. `init`, `vault`, a nested
`shell`, and a leading `--vault` are refused inside a session; `lock`, `help`,
`exit`, and `quit` are the only verbs the shell handles itself.

What is retained between commands is one wipe-on-drop passphrase buffer, and
nothing else. Derived keys, the decrypted catalog, the search projection, the
repository verifier, pinned heads, and the cross-process writer lock all live
and die inside a single command, exactly as they do one-shot — because every
VLT-PM05 access and mutation boundary consumes its session by value so a stale
pinned head cannot be reused. The authenticator is collected lazily on the first
command that needs to unlock, and is wiped by `lock`, by any `locked` exit
class, by the configured `auto_lock_seconds` bound — measured when a command is
submitted, not when the prompt was printed, so an unattended session cannot hand
a stale value to whoever types next — by an unreadable clock, and by session
end. `status` therefore still reports `locked`
inside a session, which is accurate: between commands the vault really is
locked.

Command lines are read from the controlling terminal with the fixed
`vault-pm> ` prompt, never from process standard input, so a redirected stdin
supplies neither a secret nor a command. Hidden ceremonies are untouched: `item
add login` inside a session collects its password through the same echo-disabled
terminal path, and `item reveal` still writes only to the terminal. Ordinary
output goes to stdout and stderr byte-identically to the one-shot invocation, so
a command's failure prints the same fixed line and does not end the session. The
process exit class of `vault-pm shell` reports how the *session* ended, not how
the last command fared; a caller that needs a command's exit class runs that
command one-shot.

The pre-emptive idle timer that locks a session while nobody is typing remains
Phase 1B work; this slice ships only the command-boundary bound.

## Local agent, IPC, and auto-lock

`vault-pm agent start` answers the pre-emptive-timer gap the shell above
leaves open, by moving passphrase retention into its own long-lived,
background process instead of one foreground session. `agent start`
re-executes this same binary, detached, as the hidden `agent run-foreground`
verb, which binds a permission-checked Unix domain socket
(`coding_adventures_vault_pm_agent_host`) at a short, deterministic,
owner-private path and retains one passphrase per vault name in memory.

`agent unlock` is the only way that store is populated. It authenticates
through the exact same `open_authenticated_access` unlock step every other
command uses — locking the session again immediately afterward — and hands
the agent a passphrase only once that open has already succeeded against the
real vault; the agent crate itself has no dependency on
`vault-pm-application` and cannot verify a passphrase even in principle.
Every other authenticated command then opportunistically asks a running
agent for its vault's passphrase before ever reaching the terminal prompt,
through one seam (`agent::passphrase_for`) every one of them shares. Every
branch that is not "a running agent already holds an unexpired passphrase for
exactly this vault" falls back to the unmodified one-shot prompt
unconditionally — one-shot operation remains correct with no agent running
at all.

`passphrase rotate` is the one exception: it always prompts for the current
passphrase fresh, never consulting the agent, for the same reason the
interactive shell above refuses to delegate `passphrase` at all. A successful
rotation also forgets that vault's cached passphrase immediately, and any
command that comes back with the `locked` exit class tells the agent to
forget that vault too — a best-effort self-heal against a cache made stale by
an out-of-band rotation on another device, mirroring the shell's own
in-process `lock`-on-rejection.

Two permission layers gate every connection: the socket's parent directory
and the socket file are owner-only (`0700`/`0600`), and — the requirement
that actually matters — every accepted connection's peer is verified against
the kernel's own record of who opened it (`SO_PEERCRED` on Linux,
`getpeereid` on macOS and the BSD family) *before* a single request byte is
read. A mismatched peer gets no response at all.

Auto-lock is real and pre-emptive here, unlike the shell's command-boundary
bound: a background sweep thread wipes each vault's retained passphrase once
its own `auto_lock_seconds` elapses, whether or not any command asks about it
in the meantime, because a background process — unlike a shell blocked on a
terminal read — has somewhere for that timer to run.

`vault-pm agent stop`, `agent lock [--vault NAME]`, and `agent status
[--json] [--vault NAME]` round out the surface; all three are idempotent, and
`agent start` on an already-running agent reports success rather than
failing. The interactive shell refuses the whole `agent` noun, because
`agent run-foreground` run inline would block the session's own prompt
forever — the same mistake a nested `shell` already is.

Windows named-pipe support is an explicit, documented deferral
(`VLT-PM48-local-agent-ipc.md` §9); every agent verb reports the closed
`unsupported` exit class there instead.

## Finishing what a crash interrupted

A process killed inside a mutation leaves a durable `PendingPublication`
journal. The bytes in it are already signed; the only open question is whether
the provider has them yet, and that question has one correct answer the machine
can check. So every command that opens the vault finishes it, using the
passphrase it has already collected, through the application's
`unlock_recovering_pending_publication`.

There is no recovery verb, and that is a design decision rather than an
omission. The person who needs the repair is by definition someone whose
ordinary command just failed; a verb they would have to know about would not
reach them. The repair therefore lives on the path they are already walking:

| Path | On `PendingPublication` |
|---|---|
| `authenticated_access` — item CRUD/list/show/reveal, `search`, `history`, `conflict`, `audit enable`/`list`/`show`, `import`, `restore` | recovers, then opens |
| `export`, `audit verify` | recovers, then opens |
| `init` and `vault create` resume | recovers, reports `Vault recovered.` |
| `status`, `doctor`, `doctor --unlock` | **reports, never repairs** |

The last row is the interesting one. `doctor` is a diagnostic, and `--unlock`
does not turn it into a repair: a wedged vault short-circuits its authenticated
half entirely — no passphrase is collected, nothing is published — and it
answers `recovery_required` with exit class 5. Keeping both diagnostics
read-only is what lets a person look at an interrupted vault, and restore a
pre-mutation file-level backup instead, without racing an eager repair.

When a command does repair the vault, one fixed payload-free line goes to
standard error:

```text
vault-pm: recovered an interrupted write
```

`execute` decides that by reading the durable lifecycle state immediately
before the command and — only if that reading found `recovery_required` —
again immediately after, both inside the cross-process writer lock the command
already holds. That lock is what makes the inference sound: no other local
writer can move the state between the two reads.

The second reading is conditional for a reason worth knowing. Reading owner
state initializes its storage backend, and a backend initialization is itself a
durable step that VLT-PM41's drill names and kills processes at. Reading after
*every* command would append durable writes past each ceremony's own last one,
so "the portable-export artifact is the last thing this command makes durable"
would stop being true. An observation about a command must not move the
command.

`observed_a_repair` states the whole rule, and the row worth knowing is the
quiet one: if the after-state cannot be *observed*, no notice is emitted. "Not
observed" satisfies "not `recovery_required`" while proving nothing, and
announcing a repair on a vault that is still wedged would be worse than saying
nothing at all. Both ends fail toward silence. Standard output and every exit
class are unchanged.

A `PendingRotation` — the second journal, added by VLT-PM43 — takes the same
door, with one difference that matters: **the roll-forward consumes no
passphrase.** Everything left to do after that journal is durable is a pure
function of the journal, and asking for a secret would create a worse problem
than it solved, because at that point *which* passphrase is correct depends on
how far the interrupted process got — precisely the ambiguity the journal
exists to remove. So a person who types the passphrase they had before the
crash still gets their vault repaired, and then an honest
`authentication required` from the open that follows.

See `code/specs/VLT-PM42-cli-pending-publication-recovery.md` and
`code/specs/VLT-PM43-cli-passphrase-rotation.md`.

## Changing the master passphrase

```text
vault-pm [--vault NAME] passphrase rotate
```

The verb takes no arguments. §14.5 forbids a passphrase reaching this process
through argv, an environment variable, command history, a URL, or config, and a
flag naming a file or a policy would be the first step toward one that named a
secret.

Two prompts, in an order that is the whole safety argument:

```text
Vault passphrase:          the current one -- this is the authentication
New vault passphrase:      collected against an already unlocked vault
Confirm vault passphrase:  constant-time compared, same boundary `init` uses
```

Current first, because someone who cannot produce it must be told so before
being asked to invent a replacement. New second, so a typo is caught while the
old passphrase is still the only one that means anything. Nothing durable
happens until both are in hand and the next bootstrap is built and signed.

What the rotation costs is fixed, and that is the point of it. The passphrase
protects exactly one thing on disk — the 32 bytes of
`BootstrapV1.passphrase_root_wrap` that hold the vault root key — so changing
it is one Argon2id derivation, one AEAD open, one AEAD seal, and a re-signature
by the unchanged vault authority. No item body, DEK, catalog, commit, or
certificate is read or rewritten, whether the vault holds three logins or
thirty thousand.

The retired generation is **deleted**, not merely unpointed-at: it wraps the
same unchanged root key under the old passphrase-derived key, so leaving it on
disk would make the rotation ceremonial against exactly the adversary a person
rotates because of. Two limits are worth stating rather than arguing away — a
backup taken before the rotation still contains the old wrap, and the delete
unlinks a file rather than overwriting media.

`shell` refuses the verb. A session's entire premise is that the authenticator
it collected once still opens the vault, and a rotation is the event that makes
that false.

## Generating a password

```text
vault-pm password generate [--length N] [--no-lowercase] [--no-uppercase]
                           [--no-digits] [--no-symbols] [--exclude-ambiguous]
                           (--reveal|--copy)
```

This is the one command in the grammar that opens no vault. It unlocks nothing,
reads no item, publishes no audit event, and takes no `--vault` selector,
because a selector names a target and this command has none. It works on a
machine where `init` has never run, which is the most common moment to want a
generated password.

`VLT-PM44-cli-password-generate.md` §1 records why that scoping is deliberate
rather than incomplete. Briefly: `VLT-PM15` §2 already exempts operations that
reveal no vault content; a vault-scoped event would be a *new* disclosure,
correlating an instant with whichever item is created next; and requiring the
master passphrase to perform an operation that never opens the vault would
train a person to type it at prompts that do not need it.

The strength of what it produces is fixed by two rules, both in the pure
`vault-pm-password-policy` crate:

- **The randomness is the operating-system CSPRNG**, reached through
  `CliHost::fill_entropy` → `OsEntropy::fill` → `csprng::fill_random` →
  `getrandom`/`getentropy`/`BCryptGenRandom`. That path is fail-closed: an
  unavailable source is a provider failure, never a weaker draw.
- **The floor is 80 bits**, checked as the exact integer comparison
  `alphabet^length >= 2^80`. The default — 24 characters over all four classes
  — is 155 bits, so the floor only ever bites on a policy someone deliberately
  narrowed. A 12-character all-class password is 77.7 bits and is refused, one
  character short, with its own message rather than a generic
  "invalid command".

Exactly one output mode is required. There is no plain-stdout mode: this
command has nothing but a secret to say, and a default stdout mode would put a
live credential into shell history, scrollback, `tee` pipelines, and CI logs
the first time anyone redirected it. `--reveal` confirms on the controlling
terminal and writes there and nowhere else, reusing the `item reveal` prompt
and adapter unchanged. `--copy` confirms with its own prompt and writes to the
system clipboard, scheduling a verified clear — see *Copying to the clipboard*
below. Because this command may not read config, its clear delay is the product
default of 30 seconds rather than a configured value.

Confirmation happens *before* generation, which buys a property `item reveal`
cannot have: on refusal no password is ever created, so there is no secret to
wipe.

The interactive shell can run this verb, and is the one place it is delegated
*without* the session's bound-vault prefix — see `takes_no_vault_selector`.

## Copying to the clipboard

`--copy` on `password generate` and `totp code` delivers the secret to the
system clipboard instead of the controlling terminal, and schedules its clear.
`VLT-PM46-cli-clipboard.md` is the contract; the adapter itself lives in
`vault-pm-cli-host`.

**It is not a new disclosure path.** Every step of both ceremonies — grammar,
audit reservation, unlock, confirmation, durable `ItemRead` publication before
release, the non-secret standard-output line — happens in the same order with
the same consequences. Only the last step differs. That is the whole claim, and
it is what makes `--copy` a change of channel rather than a second, quieter way
to get a secret out of a vault.

The confirmation prompt is the one visible difference:

```text
Copy secret to this system's clipboard? Type yes to continue:
```

Reusing the reveal prompt would be a false statement to the person being asked
to consent — the value is not going to their terminal, it is going somewhere
every process in their session can read. A consent ceremony that misdescribes
what it consents to manufactures a record of an agreement nobody made.

**Availability is checked first**, before any prompt, unlock, clock reading,
entropy reservation, or audit event — exactly where the old blanket `--copy`
refusal sat. Only the condition narrowed, from "always" to "when this host has
no clipboard", so a headless runner still gets `unsupported` (exit 8) without
being asked for a passphrase first.

**Which timeout is used is forced by each command, not chosen.** `totp code`
opens a vault, so the selected vault's `clipboard_clear_seconds` is already in
hand and is used. `password generate` may not read config at all — VLT-PM44 §1
requires it to resolve no platform layout and to work where `init` has never
run — so it uses the product default. VLT-PM46 §6 states that consequence
rather than hiding it.

### `vault-pm clipboard clear`

```text
vault-pm clipboard clear
```

The detached half of `--copy`, and not a command for people: it is what
`vault-pm` re-executes *itself* as, so that a clear scheduled thirty seconds
out survives the exit of a one-shot process. It is listed in the usage text
anyway, because a closed grammar with a secret verb in it is not a closed
grammar.

It opens no vault, resolves no platform layout, takes no writer lock — a
background process that slept for the timeout while holding the cross-process
writer lock would block every other `vault-pm` invocation for that whole window
— prompts for nothing, and publishes no audit event. Its parameters (a delay, a
salt, and a commitment to the copied value) arrive on **standard input**,
because argv is readable by every account on the host through `ps`. Typed by
hand with nothing on standard input it reads zero bytes and exits 2. An
interactive session refuses the verb outright: a shell's standard input is a
person's terminal, not the pipe a parent wrote.

## Showing a TOTP code

```text
vault-pm [--vault NAME] totp code ITEM (--reveal|--copy)
```

`item add totp` stores a seed and `item reveal ITEM totp-secret` hands the seed
back for re-provisioning another device. Neither is the reason anyone puts a
TOTP seed in a password manager. This command is: it computes the six digits
that are valid right now.

It is the *opposite* of `password generate` in almost every respect, and
deliberately so. It opens a vault, requires the passphrase, resolves an item,
and publishes an audit event, because `VLT-PM15` §2 already names "TOTP
display" in its list of accesses. `VLT-PM45-cli-totp-code.md` §3 writes down the
argument for treating it more lightly — a six-digit code lives about thirty
seconds and, unlike the seed, does not let its holder produce the next one —
and rejects it: that is an argument about the consequence of a disclosure,
while the audit trail records the fact of one, and an access log whose
completeness depends on how long the disclosed value stays useful is not an
access log.

The ceremony is therefore `item reveal`'s, unchanged: the same exact-`yes`
terminal prompt, the same `Denied`/`Failed`/`Succeeded` outcomes on the same
`ItemRead` action, the same publish-before-release ordering. The event records
that a code was viewed and never the code, the algorithm, the period, or the
window.

**The clock is read twice, and that is the load-bearing detail.** The audit
timestamp is reserved before authentication, as every audited access reserves
it. The *code* time is read again after unlock and after the confirmation
answer, immediately before the computation — because an Argon2id derivation and
a human reading a prompt sit between the two readings, so several seconds is
ordinary and a whole thirty-second period is entirely reachable. A command that
reused the reserved reading would routinely return the previous code: six
digits, correct-looking, and rejected by the site. There is no NTP query and no
drift correction, so TOTP correctness depends on the host clock being roughly
right, exactly as it does for every other TOTP client.

Output is split by sensitivity, which is why this command's standard output is
not empty the way `item reveal`'s is:

- the **code** goes only to the controlling terminal, through the §14.6 reveal
  adapter — or, with `--copy`, only to the clipboard;
- the **window** — one line, `Code valid for N more seconds` — goes to ordinary
  standard output, because it is a function of the clock and the stored period
  and anyone with a watch can reproduce it. Hiding a public number on the
  private channel would make the command's non-secret output invisible to a
  script and buy nothing.

`N` exists because a person handed `123456` with two seconds left will type it
into a form that rejects it and blame the vault. When `N` is small the command
reports the small number and returns; it does not sleep until the next step,
which would hold an unlocked vault open with decrypted seed material live for a
duration nobody chose, and it does not hand back the next step's code, which is
not valid yet.

The computation happens inside `vault-pm-application`, not here. Building this
as "reveal the seed, then compute" would have worked and would have
materialized the seed in this outermost layer, next to the argument parser and
the terminal. What crosses the boundary is the finished code and the countdown.

`--copy` is recognized and refused with the unsupported class before any
prompt, unlock, clock reading, or entropy reservation — identical to the
generator's refusal, so both stop refusing on the day a clipboard adapter
lands. A live refreshing display is deferred by `VLT-PM45` §8, which records
what it would first have to decide about the idle-lock bound, per-redraw audit
events, and terminal raw-mode handling.

## Attaching a file

```text
vault-pm [--vault NAME] attachment add ITEM FILE
vault-pm [--vault NAME] attachment list ITEM
vault-pm [--vault NAME] attachment export ITEM ATTACHMENT FILE
```

`add` reads the source and validates its base name **before** the passphrase
prompt, so a missing file, a directory, or one over `MAX_ATTACHMENT_BYTES`
costs no terminal interaction — the same position the clipboard-availability
and policy checks sit in, for the same reason. The directory the file came from
is never stored: only the base name is, and a name that is empty, over 255
bytes, `.`, `..`, or contains a control character or a path separator is
rejected rather than repaired.

`list` prints identity, byte length, content hash, and name on ordinary
standard output. The hash is what makes the byte-identical round trip checkable
by hand, and it is a hash of plaintext the operator can already obtain.

`export` is a disclosure, and it runs the same ceremony as `item reveal`: audit
clock and randomness reserved before unlock, an interactive confirmation, a
durable `ItemRead` published *before* the plaintext is released, and `Denied`
recorded on refusal. Only the last step differs, and so does the sentence:

```text
Write this attachment's contents to a plaintext file? Type yes to continue:
```

A third prompt rather than a reused one, because neither existing sentence
describes writing vault content into an ordinary unencrypted file this product
will not track, clear, or know about again.

**The destination is required and never defaulted from the stored name.** In a
synced vault that name is authored by whoever attached the file, which need not
be this person, so no code path here turns a stored name into a filesystem
path. The write refuses to replace an existing destination, creates owner-only,
`fsync`s, and removes the incomplete file if anything fails.

The name is printed **quoted and escaped**, through the same helper every other
stored string here passes through. The application layer already rejects the
characters that make a name render as a different name — Unicode Cc, Cf, and
the line and paragraph separators — on decode as well as on ingest, so this is
the second of two gates. It is here because a validator is a statement about
what was stored and an escape is a statement about what reaches a terminal, and
the operator is reading the terminal when they choose which attachment to
export.

`attachment remove` is not implemented. Removing the reference while every byte
stays in the store until a garbage collection this product has not built would
say something false; it lands with `gc run`.

**Export, import, and restore announce what did not travel.** A snapshot
carries records and not blobs, so an attachment stays in the source vault. All
three ceremonies that can observe that write a fixed sentence to standard
error — `vault-pm: portable export does not carry attachments` — leaving
standard output and the exit class untouched, the same shape as the recovery
notice above.

`restore` is the one that matters most, because its own success line says
*verified*. That word is true about what the ceremony compared — attachments
are normalised out of both sides, so the comparison is sound — but a person
restoring a vault reads it as "everything came back". A backup somebody
believes carries their recovery codes and does not is worse than no backup.

## The durable-write seam

This package is the layer that knows what "durable" means. `vault-pm-application`
is deliberately storage-agnostic and owns no filesystem authority, so it cannot
be. Everything this crate makes durable passes one of three gates: the
`storage-core` backend it composes over the application-state and object roots,
the configuration writer, and the portable-export destination.

`src/crash.rs` names those gates for
`code/specs/VLT-PM41-cli-crash-fault-matrix.md`, so a drill can kill the real
process at a chosen durable write and then check what the *next* real process
can see and repair. The module has two bodies:

- **`crash-injection` off** — the default, and the only configuration the
  product executable is ever built in. `LocalBackend` is exactly
  `FsStorageBackend`, each combinator is an `#[inline]` function whose whole
  body is `action()`, and `coding_adventures_vault_pm_crash_injection` is an
  optional dependency that is not compiled at all.
- **`crash-injection` on** — enabled by exactly one crate,
  `code/programs/rust/vault-pm-cli-drill`, through its ordinary
  `[dependencies]`.

Neither configuration changes behavior, output, exit classes, files, or on-disk
formats. Nothing here is reachable from an argument vector or from
configuration.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_cli --all-targets -- -D warnings
cargo clippy -p coding_adventures_vault_pm_cli --features crash-injection \
  --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_cli --no-deps
```

The real-process crash drill lives in
`code/programs/rust/vault-pm-cli-drill`, because only there is there a process
to remove - and because keeping it out of the product crate is what makes the
"never in a released binary" claim structural rather than conventional. Cargo
resolves features per package, so a product crate that named
`crash-injection` even in `dev-dependencies` would hand
`cargo build --all-targets` an instrumented `target/release/vault-pm`.

Naming no feature is necessary and not sufficient, since
`--features <dep>/<feature>` reaches a direct dependency's features regardless.
This crate therefore also exports `CRASH_INJECTION_COMPILED`, and the product
executable asserts on it in a `const` block, so a `vault-pm` with the
instrumentation in it does not compile.
