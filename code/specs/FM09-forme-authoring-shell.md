# FM09 — Forme Authoring Shell

> **Status:** Authoring v1 in progress. The durable authoring core, default
> editor, and exact pipeline preview are implemented; publish and packaging follow as
> separately reviewable product boundaries.
> **Scope:** Project state, editing transactions, persistent history, editor
> composition, live preview, publish composition, and the local desktop shell.
> **Packages:** `forme-authoring-core`, the editor packages, and
> `forme-shell-desktop`.

## Implementation status

| Surface | Status | Evidence / next step |
|---|---|---|
| Bounded project codec | Implemented | `forme-authoring-core` validates the closed v1 project and authorable Content IR subset with hard recursive limits. |
| Crash-safe autosave and persistent undo/redo | Implemented | The injected compare-and-swap adapter, immutable transactions, and canonical persisted history pass failure, conflict, cancellation, and restart tests. |
| Accessible block editor and configuration UI | Implemented | `forme-authoring-editor` provides keyboard-complete settings, document, block, history, and declarative plugin-slot controls over the durable core. |
| Pipeline-backed preview | Implemented | `forme-authoring-preview` runs exact persisted revisions through the real FM03/FM07 watch and artifact path with cancellation, last-good retention, and bounded diagnostics. |
| Reviewed publish workflow | Implemented | `forme-authoring-publish` composes exact-revision product builds with FM08 validation, closed target review, cleanup, and durable acknowledgement without exposing host authority. |
| Installable desktop shell | Pending | FM-B066 packages the proven workflow and first-run experience. |

## 1. Purpose and delivery boundary

The authoring shell is the non-developer product surface described by FM00.
It does not create a second build system. Editing produces validated Forme
inputs, preview invokes the real pipeline, and publish invokes the real FM08
deploy runner. The shell owns window and project lifecycle; plugins contribute
editor controls through reviewed slots and never receive ambient host handles.

FM-B016 is a completion milestone rather than one code review. Its original
acceptance gate spans five independently risky boundaries: durable user data,
editor behavior, preview fidelity, deployment authority, and desktop
packaging. FM-B062 through FM-B066 make those boundaries explicit while
preserving one linear path to the original product outcome.

The first slice is intentionally UI-free. A block editor cannot promise
autosave or undo until the state it edits has a bounded codec and an atomic
persistence contract. Conversely, the core must not know about DOM, Tauri,
filesystem paths, network tokens, or pipeline processes.

## 2. Authoring project v1

An `AuthoringProject` is a closed, versioned value:

```ts
interface AuthoringProject {
  readonly schemaVersion: 1;
  readonly projectId: string;       // canonical UUIDv7 text
  readonly title: string;
  readonly site: {
    readonly baseUrl: string | null;
    readonly themeId: string;
  };
  readonly documents: readonly AuthoringDocument[];
  readonly activeDocumentId: string | null;
}

interface AuthoringDocument {
  readonly id: string;              // canonical UUIDv7 text
  readonly slug: string;            // one portable URL segment
  readonly title: string;
  readonly status: "draft" | "published";
  readonly body: DocumentNode;      // Content IR
}
```

The codec is stricter than TypeScript's structural types. It rejects unknown
fields, accessors, prototypes other than plain objects, sparse arrays,
duplicate identifiers or slugs, dangling active-document references,
non-finite numbers, lone surrogates, unsafe URL schemes, raw backend nodes,
and values beyond the hard limits below. Authoring v1 permits paragraph,
heading, blockquote, list, task item, code block, thematic break, table, text,
emphasis, strong, strikethrough, code span, link, image, autolink, and break
nodes. Raw block and raw inline nodes remain import-only until a later trusted
source mode is specified.

The canonical serializer recursively orders object keys, preserves document
and child order, and produces UTF-8 JSON with no insignificant whitespace. It
is the only persisted representation used by the core.

### 2.1 Hard limits

The implementation may expose lower caller-selected limits, but never exceed:

| Resource | Hard maximum |
|---|---:|
| Persisted JSON bytes | 8 MiB |
| Documents per project | 1,000 |
| Document tree depth | 64 |
| Nodes per document | 100,000 |
| Inline or literal string | 1 MiB |
| Project/document title | 512 Unicode scalar values |
| Slug or URL | 2,048 Unicode scalar values |
| Retained history entries | 200 |

Limits are checked before expensive traversal where possible and throughout
recursive validation. Diagnostics contain bounded field paths and never echo
untrusted persisted strings.

## 3. Transactions and persistent history

The public core exposes semantic commands rather than mutable project
references. FM-B062 includes project creation, document creation/removal,
document metadata replacement, body replacement, site configuration, active
document selection, undo, and redo. Every successful command:

1. derives a new immutable project value;
2. validates the complete result;
3. appends one history entry and clears the redo branch;
4. saves the complete bounded session through the injected adapter; and
5. publishes the new in-memory state only after persistence succeeds.

A failed validation, conflict, cancellation, or adapter write leaves both the
observable session and its history unchanged. No debounced window exists in
which an acknowledged edit is not durable. Later UI work may coalesce input
events into one semantic command, but it must not weaken this rule.

Undo and redo are state transitions and are persisted exactly like edits.
History is bounded by dropping the oldest unreachable snapshots. The current
snapshot is always retained. Restarting from a valid stored session restores
the same project, cursor, undo availability, and redo availability.

## 4. Storage adapter and concurrency

`forme-authoring-core` is pure except for an injected asynchronous adapter:

```ts
interface AuthoringStorage {
  load(signal?: AbortSignal): Promise<StoredAuthoringState | null>;
  compareAndSwap(
    expectedRevision: string | null,
    bytes: Uint8Array,
    signal?: AbortSignal,
  ): Promise<{ readonly revision: string }>;
}
```

`load` returns immutable bytes plus an opaque revision. The core treats the
revision only as a comparison token. `compareAndSwap` must atomically reject a
stale expected revision; it must never silently overwrite another session.
Adapters own filesystem, OPFS, or platform capabilities and must publish new
bytes atomically. The core performs no ambient I/O.

Any adapter rejection guarantees that no durable publication occurred.
Cancellation is honored only before the atomic commit point; once bytes are
published the adapter must resolve successfully with the new revision even if
the signal becomes aborted. A malformed success result is indeterminate rather
than a rollback: the shell must reload before it retries.
The core preserves stale-revision conflicts and standardized cancellation, but
maps other adapter exceptions to a bounded storage error that does not expose
host paths or arbitrary exception messages.

Opening a missing store requires an explicit initial project and persists it
through `expectedRevision: null`. Opening malformed or unsupported bytes fails
closed without replacement. There is no "recover by resetting" behavior in
the core; a shell may offer an explicit export-and-reset flow later.

## 5. Editor composition

FM-B063 supplies `forme-authoring-editor`, the accessible React block editor
and project-settings surface. The component is controlled by an
`AuthoringSession`: it reads only the session's frozen project snapshot and
changes state only by dispatching the semantic commands defined in section 3.
Document selection, creation, metadata, removal, site configuration, undo,
redo, and every block edit therefore cross the same validated persistence
boundary as non-UI callers. A failed, cancelled, or conflicting dispatch does
not advance the rendered snapshot and is announced without exposing adapter
error text.

The default editor owns document-order controls and authoring forms for
paragraph, heading, ordered or unordered list, image, code block, blockquote,
table, and link blocks. Insert, replace, move, and remove operations construct
one complete `replace-document-body` command; they never mutate a retained AST.
List and table forms cap rows, columns, and padded cells before allocating AST
nodes, keeping editor-side expansion below the core's authoring node budget.
Hosts provide the reviewed theme choices as identifier/label data, so the
primary settings flow cannot introduce raw CSS or an unreviewed theme
identifier. Document identities are supplied by a host callback because UUID
creation is a host concern, but the callback receives no session or adapter.

### 5.1 Declarative plugin slots

Third-party editor extensions are data across this boundary, not same-realm
React components. A bounded contribution names one of three fixed slots
(`document-toolbar`, `block-toolbar`, or `site-toolbar`) and carries only a
portable plugin/action identity plus a short plain-text label. Contributions
are exact-key, plain-object data; duplicate identities, accessors, prototypes,
symbols, over-count arrays, control or bidi-format characters, and over-limit
strings are rejected before rendering. React escapes every displayed label.

Activating a contribution calls one injected `EditorPluginBridge` with a
deeply frozen request containing the contribution identity, slot target, and a
private validated snapshot of the current project. Each action also shows its
validated plugin identity. The bridge represents the already-sandboxed FM02
plugin boundary. It receives a bounded cancellation signal that expires or
aborts on editor unmount; the host must retire non-cooperative sandbox work. It
returns exactly one `AuthoringCommand`; the editor
dispatches that command through `AuthoringSession` and refreshes only after the
core accepts and persists it. The request contains no storage adapter, session,
DOM object, preview or publish handle, credential, filesystem path, network
client, or host callback. Arbitrary plugin JSX, event handlers, and DOM nodes
are deliberately outside the v1 slot contract.

### 5.2 Accessibility contract

Every operation is reachable through native labelled form controls and
buttons. Block reordering uses explicit Move up / Move down buttons rather than
drag-only behavior. Focus moves to a newly inserted block, to the nearest
surviving block after removal, and to the selected document heading after
document changes. A polite status region announces saves and failures; an
assertive alert reports the current failure. Busy controls expose
`aria-busy`/disabled state, visible `:focus-visible` styling ships with the
component, and no operation requires a pointer, hover, or `contenteditable`.
Browser tests must drive the same labelled controls a keyboard or assistive
technology user reaches; direct calls into component internals do not satisfy
this contract.

Style and interactivity configuration operate on the validated FM04 and FM05
IRs. They do not introduce raw CSS or handwritten JavaScript into the primary
workflow.

## 6. Preview

FM-B064 supplies `forme-authoring-preview`, a capability-free coordinator over
the durable authoring core and the existing FM03/FM07 watch path. A preview
request snapshots both `AuthoringSession.project` and
`AuthoringSession.storageRevision`, validates a private copy of the project,
and gives an injected host materializer only that frozen pair plus a bounded
abort signal. The materializer must create a new isolated project input and
return the real typed `Pipeline` for that input. It may not reuse a mutable
input tree across authoring revisions. Filesystem paths, configuration loading,
plugin discovery, and cache ownership remain host capabilities and never cross
back into the editor or authoring project value.

The coordinator runs every prepared pipeline through `Orchestrator.watch`, not
through a second renderer or a direct call to an emitter. Each persisted edit
requests a new preview. A short bounded debounce window coalesces bursts to the
latest requested revision. When a newer revision supersedes preparation or an
active build, the coordinator aborts preparation or stops the watch session,
waits for cancellation to settle, disposes the isolated materialization, and
then starts only the latest request. Disposal is idempotent and stops pending
work before releasing the host materialization. The host must make preparation
and release safe under cancellation. Release is the final retirement boundary:
it must retire all prepared work even when watch stop reports failure, and the
coordinator reports that cleanup failure rather than treating stop as settled.
The host must retire non-cooperative preparation and release work.

Only a successful, still-current watch result is converted with FM07's
`snapshotFromOutputs` contract and sent to the injected preview publisher. A
failed, cancelled, malformed-output, or superseded result never replaces the
last good output. The publisher receives the exact authoring revision together
with the build ID and static artifact snapshot, or a bounded failure record for
that same revision. This preserves FM07's server-sent reload and last-good-site
behavior while making the authoring revision visible to the shell. A success
from revision N is stale as soon as N+1 is requested, even if N finishes before
its cancellation is observed.

Before conversion, the coordinator descriptor-snapshots at most 256 named
outputs and 10,000 total files. Output names are limited to 256 Unicode
scalars, portable artifact paths to 2,048 characters with 255-byte ASCII
segments, each file to 16 MiB, and the complete snapshot to 128 MiB. It rejects
proxies before incrementally enumerating bounded plain records, copies every byte array using
typed-array intrinsics, rejects portable case-fold and file/ancestor
collisions, and publishes a
non-mutating view so producer mutation cannot change validated output. Revision
and build identities are exact tokens of at most 1,024 Unicode scalars; they
are rejected rather than truncated. The publisher may prepare asynchronously,
but every externally visible mutation must run inside the coordinator's
one-shot synchronous generation guard. A successful publish requires exactly
one guarded commit. As soon as that guarded mutation returns successfully, the
coordinator records its exact revision and build as the externally visible
last-good snapshot, even if the publisher then hangs and is superseded.
Rejection after a commit is reported as indeterminate while retaining that
last-good attribution, and no delayed guard can commit after its publisher call
returns.
Publisher settlement is raced against the same abort signal, so a stale
non-cooperative publisher cannot block a newer revision or disposal; its late
rejection remains observed but its closed guard can no longer commit.
The initial watch result is likewise raced against private cancellation state,
not mutable properties on the host-visible signal, so a stopped session cannot
retain the pump merely by leaving its result stream open or shadowing signal
members. The idle change stream and iterator passed to watch are frozen while
their close closure remains private. Stop and release are invoked through
captured call intrinsics. If either final
retirement step fails, the coordinator enters a failed poisoned state, blocks
all pending and later builds with a bounded retirement diagnostic, and never
runs a newer pipeline alongside work whose retirement is unknown. Poisoning
also settles and invalidates the active task without opening a publisher guard,
so delayed failure publication cannot overwrite a later poisoned request.

Preview diagnostics are closed data: severity, code, stage identity, and a
plain message. The coordinator admits at most 64 diagnostics, limits every
scalar, rejects controls and bidi formatting, removes arbitrary fields and
adapter errors, and emits one generic bounded diagnostic if a result is not
safe to inspect. Public result objects, status snapshots, projects, and
diagnostics are private deeply frozen copies. A caller receives one terminal
attempt result (`ready`, `failed`, `cancelled`, or `superseded`) for every
request; state for the last good revision is reported separately from the
active revision so the UI cannot label stale output as current.

## 7. Publish

FM-B065 first builds the exact persisted authoring revision, then invokes FM08
with one reviewed target configuration. Editor plugins never see deployment
credentials. The shell's host boundary obtains and stores credentials through
platform facilities, renders the complete target and ownership decision, and
requires an explicit publish action. A failed or indeterminate deployment does
not mark documents published. A successful deployment records the manifest
identity and authoring revision in the project workflow metadata.

The durable project contains a closed `workflow` record whose
`lastPublication` value is either `null` or the exact
`authoringRevision`, base64 SHA-256 `manifestSha256`, and reviewed `targetId`
from the most recent acknowledged publication. The authoring core exposes one
semantic `record-publication` command and an exact-revision dispatch operation.
That operation executes in the session's serialized transaction queue and
rejects unless the current storage revision still equals the expected source
revision. Its single compare-and-swap write records the publication and changes
every document in that exact snapshot to `published`; validation, conflict,
cancellation, or storage failure leaves both workflow metadata and document
statuses unchanged. Undo and redo retain their existing durable semantics, so
reverting the local acknowledgement never asserts that an external deployment
was rolled back.

`forme-authoring-publish` is the capability-free coordinator for this
boundary. It snapshots and validates the session, captures one injected build
method and one injected reviewed-target method, and permits only one active
explicit action. The target's public review is closed bounded data: a portable
target identity, a human label, and a destination summary. Credentials,
filesystem paths, environment access, network clients, target configuration
files, and arbitrary callbacks are not fields in the review or the builder
input. The opaque host target owns those capabilities and receives only a
validated FM08 manifest, a manifest-bound content store, the computed manifest
identity, and a cancellation signal.

The build adapter receives a deeply frozen copy of the exact project and its
storage revision. It must run the real product pipeline and returns a separately
releasable manifest/content preparation. Before deployment, the coordinator
parses and freezes the manifest through FM08, computes the identity from its
canonical bytes, preflights every referenced content digest, confirms the
session revision is still exact, and then closes the build boundary. The
coordinator never accepts a target-selected manifest identity or silently
substitutes preview output. The target receives a manifest-restricted store,
not the raw builder store: every target read re-verifies size and digest, and
retirement revokes the wrapper, so mutable or retained builder content cannot
escape the reviewed manifest.

The reviewed target resolves one closed result: `success`, `failed`, or
`indeterminate`, always attributed to the supplied manifest identity. `failed`
is permitted only when the adapter knows no external commit occurred;
malformed success data, loss of acknowledgement after a possible commit, or an
adapter-defined uncertain result is `indeterminate`. Rejection is treated as a
closed failure only because the injected boundary contract guarantees rejection
before its commit point; hosts must convert post-commit rejection to an
indeterminate resolution. On success, preparation retirement completes before
the exact-revision metadata transaction. A retirement or metadata failure after
external success is indeterminate and never marks documents published. An
indeterminate coordinator is poisoned against retry until the shell reloads and
reconciles target state. A failed build or known pre-commit target failure may
be retried. Diagnostics are bounded closed data and never include adapter
exceptions, credentials, paths, or target configuration.

## 8. Desktop shell and first run

FM-B066 packages the proven editor, preview, and publish flow as the default
local desktop product. Tauri remains the target unless the packaging slice
documents and reviews a replacement. First run creates a project, selects the
default theme, opens a first draft, previews it, configures a supported
publication target, and publishes without editing JSON, YAML, TypeScript, git,
or shell commands.

The desktop host exposes only narrow commands for project storage, preview,
and publish. Paths are canonicalized and contained; web content receives no
ambient filesystem, network, environment, shell, or subprocess access.

## 9. Required verification

FM-B062 must prove codec rejection, every hard limit, deterministic bytes,
transaction rollback, write failure, cancellation, stale-revision conflict,
history truncation, persisted undo/redo, and restart recovery. Coverage for the
new core package must exceed 95% statements and lines and 90% branches.

Later slices add browser accessibility tests, exact preview/build parity,
deploy dry-run and failure tests, native capability tests, and a product test
that starts from an empty profile and publishes a first site without editing a
source or configuration file.

FM-B063 browser tests cover every default block form, metadata and site
configuration, host-supplied theme choices, selection, create/remove, undo and
redo, keyboard-only reordering, focus restoration, live announcements,
dispatch rollback, and declarative plugin-slot activation. They also prove the
plugin bridge receives only the frozen request and that hostile contribution
descriptors are rejected. The package must exceed 95% statement and line
coverage and 90% branch coverage.

FM-B064 tests use a real `Orchestrator.watch`-shaped session boundary and prove
initial success, exact persisted-revision attribution, burst coalescing,
superseded preparation and active-build cancellation, last-good retention,
malformed artifact refusal, bounded/redacted diagnostics, hostile host result
handling, generation-guarded asynchronous publication, immutable bounded
artifact snapshots, materialization cleanup, concurrent double disposal, and
rejection after disposal. The package must exceed 95% statement and line coverage and 90%
branch coverage. One composition test must pass real FM03 `RunResult` output
through FM07 `snapshotFromOutputs`; a mock DOM renderer is not an acceptable
preview proof.

## 10. Related specifications

- [FM00](FM00-forme-vision.md) — authoring vision and product success bar
- [FM01](FM01-forme-kernel.md) — Content IR and capability boundaries
- [FM02](FM02-forme-plugin-host.md) — plugin authority and sandboxing
- [FM03](FM03-forme-orchestrator.md) — pipeline execution
- [FM04](FM04-forme-style-ir.md) — style values edited by the shell
- [FM05](FM05-forme-interactivity-ir.md) — declarative interactivity
- [FM07](FM07-forme-cli-dev-server.md) — preview composition
- [FM08](FM08-forme-deploy-runner.md) — publication planning and adapters
