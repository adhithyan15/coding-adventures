# FM09 — Forme Authoring Shell

> **Status:** Authoring v1 in progress. The durable authoring core is the first
> delivery slice; editor, preview, publish, and desktop packaging follow as
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
| Accessible block editor and configuration UI | Active | FM-B063 composes the core through editor slots. |
| Pipeline-backed preview | Pending | FM-B064 uses the real FM07 watch path and last-good output. |
| Reviewed publish workflow | Pending | FM-B065 composes FM08 without exposing tokens or target files to editor plugins. |
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

Opening a missing store requires an explicit initial project and persists it
through `expectedRevision: null`. Opening malformed or unsupported bytes fails
closed without replacement. There is no "recover by resetting" behavior in
the core; a shell may offer an explicit export-and-reset flow later.

## 5. Editor composition

FM-B063 supplies the accessible default block editor and project settings UI.
Editor plugins receive frozen snapshots and dispatch semantic core commands.
They cannot mutate storage, invoke preview or publish, read tokens, or obtain a
DOM node outside their registered slot. The default editor supports keyboard
operation, visible focus, labelled controls, status announcements, and a
complete no-pointer workflow. It includes at least paragraph, heading, list,
image, code block, blockquote, table, and link authoring.

Style and interactivity configuration operate on the validated FM04 and FM05
IRs. They do not introduce raw CSS or handwritten JavaScript into the primary
workflow.

## 6. Preview

FM-B064 materializes a validated draft snapshot into an isolated project input
and runs the same FM03/FM07 pipeline used by product builds. Preview inherits
FM07's coalescing, cancellation, server-sent reload, and last-good-output
semantics. It may cache aggressively, but it may not substitute a separate DOM
renderer for pipeline output. Preview artifacts and diagnostics are bound to
the exact authoring revision that produced them so stale success cannot be
mistaken for the active draft.

## 7. Publish

FM-B065 first builds the exact persisted authoring revision, then invokes FM08
with one reviewed target configuration. Editor plugins never see deployment
credentials. The shell's host boundary obtains and stores credentials through
platform facilities, renders the complete target and ownership decision, and
requires an explicit publish action. A failed or indeterminate deployment does
not mark documents published. A successful deployment records the manifest
identity and authoring revision in the project workflow metadata.

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

## 10. Related specifications

- [FM00](FM00-forme-vision.md) — authoring vision and product success bar
- [FM01](FM01-forme-kernel.md) — Content IR and capability boundaries
- [FM02](FM02-forme-plugin-host.md) — plugin authority and sandboxing
- [FM03](FM03-forme-orchestrator.md) — pipeline execution
- [FM04](FM04-forme-style-ir.md) — style values edited by the shell
- [FM05](FM05-forme-interactivity-ir.md) — declarative interactivity
- [FM07](FM07-forme-cli-dev-server.md) — preview composition
- [FM08](FM08-forme-deploy-runner.md) — publication planning and adapters
