# FM08 — Forme Deploy Runner

> **Status:** v0 specification.  Implementation in progress.
> **Layer:** FM08 (last layer of the FM00 vision — applies a
> deploy manifest to a real target).
> **Predecessor:** `forme-aot-deploy-manifest-emitter` produces
> the manifest this runner consumes.

## Implementation status

| Surface | Status | Evidence / next step |
|---|---|---|
| Deploy manifest producer | Implemented | `forme-aot-deploy-manifest-emitter` produces the input contract. |
| Core planning and validation | Implemented in FM-B044 | `forme-deploy-runner-core` validates manifests, plans complete output sets, preflights bytes, and emits deterministic dry-run reports without capabilities. |
| Filesystem adapter | Pending | FM-B045 must prove atomic tree replacement, rollback, idempotency, and stale-file pruning. |
| GitHub Pages adapter | Pending | FM-B046 owns the first hosted target integration. |
| `forme deploy` composition | Pending | FM-B047 will add this command to the FM07 CLI surface and dogfood both live sites. |

FM08 was originally checked in as FM05. FM-B011 moved it without changing its
deploy contract so FM05 can retain the Interactivity IR number reserved by
FM01–FM04.

## 1. Purpose

The deploy runner is a **standalone program** (not a pure
transform) that reads a `deploy-manifest.json` produced by
`forme-aot-deploy-manifest-emitter`, resolves each file's
content from a content store, and writes every file to a
deployment target — local filesystem, S3 bucket, Netlify edge,
Cloudflare Pages, etc.

It is the **publication trust boundary** where already-built artifacts become
files or remote objects. Earlier filesystem sources and emitters already use
narrow `storage:*` authority; this runner may additionally require an explicit
`filesystem:user`, `network:<host>`, or `env:<name>` grant for the selected
target. Pure planning stays in the capability-free core package.

## 2. Why a spec before implementation

Three reasons the runner ships as a spec first:

1. **Surface area.**  Deploy targets are diverse (fs / S3 /
   Netlify / Cloudflare / Vercel / arbitrary).  Locking the
   adapter interface before any single adapter exists prevents
   the first adapter from accidentally setting the contract for
   the rest.
2. **Capability budget.** This is the publication boundary and therefore the
   broadest effectful FM00 component. Filesystem sources and emitters already
   hold narrowly scoped storage capabilities; deployment may additionally need
   an explicitly selected external root, credential variables, or network
   hosts. Those scopes must be reviewable in one place.
3. **Atomicity semantics.**  Atomicity is harder than it
   looks — partial deploys, retry storms, and rollback all
   need to be designed-not-discovered.  Putting the
   guarantees on paper first prevents inconsistent behaviour
   across adapters.

## 3. Inputs

### 3.1 Required: `deploy-manifest.json`

The output of `forme-aot-deploy-manifest-emitter`.  Shape:

```json
{
  "version": 1,
  "baseUrl": "https://example.com",
  "fileCount": 7,
  "totalSizeBytes": 12345,
  "files": {
    "<outputPath>": {
      "outputPath": "...",
      "contentType": "...",
      "sizeBytes": <int>,
      "sha256": "<base64>",
      "source": "page-bundle | sitemap | robots | web-app-manifest | extra",
      "route": "...",        // only for page-bundle entries
      "lastmod": "..."       // only when present
    },
    ...
  }
}
```

The runner **MUST** re-validate the manifest before acting on
it (re-run `parsePageBundle`-style shape checks) — it cannot
trust the manifest content even though the upstream emitter
validated it.

#### 3.1.1 Portable output paths and ownership

Every manifest map key MUST equal its entry's `outputPath`. An output path is a
portable relative artifact path made from non-empty `/`-separated segments.
Validation MUST reject an empty path; leading `/` or `\\`; a Windows drive or
UNC prefix; `\\` as a separator; `.` or `..` segments; NUL/control characters;
colon-bearing segments (including NTFS alternate streams); duplicate normalized
paths; and file/directory prefix collisions such as `assets` plus
`assets/app.css`. A target MAY additionally reject collisions in its own
case-folding or normalization model. The complete new manifest and any previous
manifest MUST pass these checks before the runner reads target state or starts
a transaction.

The validated new manifest is the complete owned output set. A previous
manifest grants deletion authority only for its own validated paths beneath the
same target root; it never grants authority over an unlisted sibling. The
filesystem adapter MUST stage the complete new set in an exclusively created
sibling tree, reject symlinks or other linked path components and pre-existing
final targets in that tree, create files exclusively, and verify every
canonical parent remains inside the staging root. Before the final swap it MUST
reject a configured root that is itself a link and verify the canonical parent
contains both the root and staging tree. The old root is moved to a same-parent
backup, the complete staging tree is renamed into place, and failure restores
the backup. No per-file write or cleanup may follow a link or mutate an
external hard-link target.

#### 3.1.2 v0 resource limits

The v0 core fails closed before content access when a manifest exceeds any of
these implementation limits: 16,777,216 JSON string characters, 100,000 files,
100 MiB for one file, or 1 GiB total content. Decoded object inputs receive the same file
and content limits. Output paths remain capped at 2,048 characters and 255
bytes per segment. Entries that share one SHA-256 digest MUST declare the same
byte length. A future large-site profile may make these limits configurable,
but adapters MUST NOT silently raise or bypass the reviewed defaults.

### 3.2 Required: content store

A keyed store from which the runner resolves each file's body.
The store is identified by SHA-256 hash — given a file entry
with `sha256: "abc..."`, the runner looks up `abc...` in the
store and gets the bytes back.

Three content store shapes are supported in v0:

- **`directory` store**: a local directory containing files named from the
  digest's unpadded base64url encoding plus `.bin`. The manifest's canonical
  base64 digest remains an opaque lookup key; the store converts its decoded
  32 bytes to one `[A-Za-z0-9_-]{43}` filename segment, verifies containment,
  and never concatenates raw base64 (which may contain `/`). Lookup is
  `fs.readFile`. Used when
  the runner runs in the same process / box as the emitter
  and the caller wrote the contents to disk.
- **`bundle` store**: a single `.tar` or `.zip` archive containing the same
  base64url digest filenames. Archive entries must be exact single-segment
  names beneath the bundle root; absolute, traversal, link, or raw-base64 path
  entries are rejected. Lookup streams
  the entry out of the archive.  Used for cross-machine
  deploys (the entire bundle ships as one file).
- **`inline` store**: an in-memory `Map<sha256, Uint8Array>`
  populated by the caller.  Used when emitter + runner share
  a process (long-lived dev server, CI worker).

The store interface (TypeScript):

```ts
interface ContentStore {
  /** Resolve a hash to its bytes.  Throws if missing. */
  readonly get: (sha256: string, signal?: AbortSignal) => Promise<Uint8Array>;
  /** Quick "do you have this?" check without reading bytes. */
  readonly has: (sha256: string, signal?: AbortSignal) => Promise<boolean>;
  /** Iterate every hash in the store.  Used for verification. */
  readonly hashes: () => AsyncIterable<string>;
}
```

The core binds one validated manifest and content store into a
`VerifiedContentReader`. The reader parses the manifest once, supports
cancellation even when a store promise does not settle, and returns a trusted
plain `Uint8Array` snapshot only after checking its exact length and SHA-256.
Adapters MUST write that returned snapshot directly; they MUST NOT perform a
second unchecked `ContentStore.get()`. Dry-run preflight resolves each unique
digest once and retains only digest metadata, not the complete site's bytes.

### 3.3 Optional: previous deploy manifest

If supplied via `--previous <path>`, the runner uses it to
compute a **diff plan**: which files are new, changed, or
unchanged compared to the previous deploy.  Unchanged files
are skipped (no write); changed files are atomically replaced;
new files are atomically added; stale files (present in the previous manifest
but absent from the new one) are deleted only as part of successful
complete-set publication and only within the validated ownership and
containment rules in §3.1.1.

## 4. Outputs

The runner produces:

1. **Side effects** on the target (file writes, S3 PUTs, etc.).
2. **A deploy report JSON** on stdout (machine-readable) and
   a human-readable summary on stderr.

### 4.1 Deploy report shape

```json
{
  "version": 1,
  "manifestSha256": "<base64>",
  "target": "fs | s3 | netlify | ...",
  "startedAt": "<ISO-8601>",
  "finishedAt": "<ISO-8601>",
  "status": "success | partial | failed | rolled-back",
  "files": {
    "<outputPath>": {
      "action": "create | update | skip | delete",
      "bytesWritten": <int>,
      "elapsedMs": <int>,
      "error": {                // only when action failed
        "code": "WRITE_FAILED",
        "message": "..."
      }
    },
    ...
  },
  "summary": {
    "created": <int>,
    "updated": <int>,
    "skipped": <int>,
    "deleted": <int>,
    "failed": <int>,
    "totalBytesWritten": <int>,
    "totalElapsedMs": <int>
  }
}
```

The deploy report is **byte-deterministic** for the same
inputs + outcomes: sort `files` by `outputPath`; fixed key
order per entry.

## 5. Program contract

### 5.1 CLI

```
forme-deploy [OPTIONS] --manifest <path>
```

Required:
- `--manifest <path>` — path to `deploy-manifest.json`.

Required (one of):
- `--content-dir <path>` — `directory` content store rooted here.
- `--content-bundle <path>` — `bundle` content store (.tar or .zip).
- `--content-inline-fd <int>` — `inline` store reads JSON
  `{ "<sha>": "<base64>" }` from this file descriptor.

Required:
- `--target <kind>` — `fs` | `github-pages` in headless v0. Later adapters
  extend this enum without weakening the v0 capability boundary.

Required when `--target` is non-`fs`:
- `--target-config <path>` — JSON file with adapter-specific
  config (bucket name, account ID, API token reference, ...).
  Never contains secrets directly; secrets come from env vars
  named in the config.

Optional:
- `--previous <path>` — previous manifest for diff-mode deploy.
- `--dry-run` — validate everything + print the deploy report
  but make zero writes.
- `--concurrency <int>` — max parallel writes (default `4`).
- `--retry <int>` — per-file retry budget on transient errors
  (default `3`, with exponential backoff).
- `--strict` — fail-fast: abort the deploy on the first file
  error (default: continue and report partial).
- `--verify-after` — re-read every written file and SHA-256
  it; fail if any digest differs from the manifest.
- `--report <path>` — write the deploy report JSON to this
  path instead of stdout.

### 5.2 Environment variables

Adapter-specific secrets are passed via env vars **named in the
target config**, not directly on the CLI.  The runner reads only
the env vars listed in the config; unrelated env is ignored.

Standard env (all targets):
- `FORME_DEPLOY_LOG_LEVEL` — `silent` | `error` | `info` (default) | `debug`.
- `FORME_DEPLOY_NO_COLOR=1` — disable ANSI codes on stderr.
- `FORME_DEPLOY_TIMEOUT_MS` — global per-file timeout (default 60s).

### 5.3 Exit codes

- `0` — success.  Every file in the manifest was applied as
  planned (or skipped per diff).
- `1` — partial.  Some files failed; the deploy report has
  details.  No rollback was performed (caller decides).
- `2` — rolled back.  A failure triggered automatic rollback;
  the target is back at its pre-deploy state.
- `3` — manifest invalid.  The deploy never started.
- `4` — content store error (a hash was missing or unreadable).
  The deploy may have made partial writes; consult the report.
- `5` — target adapter error (auth failed, network unreachable,
  permissions denied).  Same caveat as 4.
- `6` — user abort (SIGINT, SIGTERM).  Mid-deploy interrupts
  attempt graceful shutdown; the report indicates which files
  completed.
- `>=100` — unexpected internal error (bug).

## 6. Target adapter interface

```ts
interface DeployTarget {
  readonly name: string;
  /**
   * Per-file write.  Atomicity contract: `writeFile` MUST
   * either succeed (file is fully written and visible at
   * `outputPath`) or throw, never leave a half-written file.
   * Implementations achieve this with a temp-file + rename,
   * S3 multipart + complete, etc.
   */
  readonly writeFile: (
    outputPath: string,
    content: Uint8Array,
    metadata: { readonly contentType: string; readonly sha256: string; readonly lastmod?: string },
  ) => Promise<void>;
  /**
   * Per-file delete (used in diff mode).  MUST be idempotent
   * — deleting an already-missing file is a no-op, not an
   * error.
   */
  readonly deleteFile: (outputPath: string) => Promise<void>;
  /**
   * Read back a file's SHA-256 (for `--verify-after`).
   * Returning `null` means "target doesn't support read-back"
   * — the runner downgrades to "skip verification" with a
   * warning.
   */
  readonly verifyFile: (outputPath: string) => Promise<string | null>;
  /**
   * Begin a transaction (optional).  If the target supports
   * atomic multi-file commit (e.g. Netlify's deploy API),
   * `begin` returns a transaction handle and `commit` /
   * `rollback` complete or abort the batch.  Targets that
   * don't support this (for example S3) return `null` and the runner falls
   * back to per-object atomicity only. The v0 filesystem adapter implements a
   * complete-tree transaction as required by §3.1.1.
   */
  readonly begin: () => Promise<Transaction | null>;
}

interface Transaction {
  readonly commit:   () => Promise<void>;
  readonly rollback: () => Promise<void>;
}
```

### 6.1 v0 adapters

The v0 spec covers two reference adapters:

- **`FsAdapter`** — publishes to a local directory rooted at
  `--target-config { "root": "<path>" }` through the complete-tree staging and
  swap contract in §3.1.1. A project-contained root uses `storage:write`; an
  explicitly selected root outside project storage requires the sensitive
  `filesystem:user` capability. The adapter exposes the tree swap as a
  transaction so rollback can restore the same-parent backup.
- **`GitHubPagesAdapter`** — prepares and publishes the complete validated
  output set through GitHub Pages' artifact/deployment boundary. Capabilities
  are limited to the exact GitHub endpoint plus only the selected token
  variable. FM-B046 defines the final transaction and retry mapping against
  that API before implementation.

Future v1+ adapters: S3, Netlify, Cloudflare Pages, and Vercel. Their
per-object versus transactional guarantees remain governed by the generic
adapter contract below but do not block the repository's headless v0 target.

## 7. Atomicity guarantees

### 7.1 Per object (always)

Every adapter MUST guarantee that for any single `outputPath`,
the file is either fully written or unchanged.  A half-written
file (truncated, partially overwritten, mid-PUT) MUST NOT be
observable.

### 7.2 Transactional publication

When `target.begin()` returns a transaction, the runner uses
it: all `writeFile` calls happen on the transaction; on success
`commit()`. The `FsAdapter` MUST always return a transaction and implement it
with the exclusive complete-tree staging, same-parent swap, backup, and restore
contract in §3.1.1. Netlify's draft deploy supplies the corresponding remote
transaction. Either the complete named-output set is published or the previous
set remains available.

A remote adapter with no bundle transaction, such as S3, guarantees only
per-object atomic PUT/DELETE. It MUST NOT emulate filesystem rename semantics or
claim whole-deploy atomicity. It uploads new/changed objects first, verifies
them, then deletes previous-only owned keys. A mid-operation failure can leave a
mixed remote set; the report marks every completed operation and a retry is the
recovery path. Such an adapter is unsuitable when the caller requires atomic
whole-site publication.

### 7.3 Rollback

After `begin()` succeeds, every write, verification, commit, timeout,
cancellation, or adapter failure MUST attempt `rollback()`. A rollback failure
is appended to the report but never replaces the primary failure or changes a
failed/cancelled result into success. The filesystem adapter restores its
same-parent backup before reporting rollback success.

An adapter that returned `null` from `begin()` has no automatic whole-deploy
rollback. Partial remote state is reported explicitly and the operator retries
or repairs it; the runner must not imply that per-object atomicity provides a
transaction.

## 8. Idempotency

The runner is idempotent in two senses:

1. **Re-running with the same manifest + same content store +
   same target** produces no writes (every file's SHA-256
   matches; everything is skipped).
2. **Re-running after a partial deploy** completes the deploy
   — already-written files are detected via target-side
   `verifyFile` (when supported) or via the previous-manifest
   diff and skipped.

The diff mode (`--previous`) makes (2) cheap: the runner only
considers files that differ between previous and new manifest.

## 9. Dry-run mode

`--dry-run` performs every step EXCEPT the actual `writeFile` /
`deleteFile` calls:

- Parse + validate the manifest.
- Resolve every unique content digest from the store with bounded retention
  (catches missing, size-mismatched, and hash-mismatched content errors).
- Compute the diff plan.
- Produce a deploy report with `action` set as if the writes
  had happened, but `bytesWritten` set to `0` and `elapsedMs`
  set to `0` for the unwritten files.

Dry-run output is byte-identical to a real deploy report
(modulo `bytesWritten` and `elapsedMs`) so CI can diff the
plan.

## 10. Concurrency

Default: 4 files in flight at once.  `--concurrency N` overrides.

The runner uses a bounded worker pool, not unbounded `Promise.all`:

- Worker pool size = `--concurrency`.
- Each worker pulls the next file from a shared queue, fetches
  content, writes, records the result.
- Errors don't block other workers (unless `--strict`).
- Workers exit when the queue is empty AND all workers are
  idle.

`--concurrency 1` is the deterministic / sequential mode.  Used
for tests and debugging.

## 11. Error reporting

### 11.1 stderr (human)

One line per file action:

```
[1/7] write   index.html (10.2KB) ... ok    (123ms)
[2/7] write   about/index.html (8.4KB) ... ok    (98ms)
[3/7] write   feed.xml (28B) ... ok    (45ms)
[4/7] write   favicon.ico (1.1KB) ... FAILED  (S3 403 Forbidden)
...
SUMMARY: 6 ok, 1 failed, 0 skipped — partial deploy.
Report written to: ./deploy-report.json
```

Colour codes (when stderr is a tty AND `FORME_DEPLOY_NO_COLOR`
is unset): green for ok, yellow for skip, red for failed.

### 11.2 stdout (machine)

The deploy report JSON (§4.1).  Pipeable into `jq`, diffable
between runs, attachable to a CI artifact.

### 11.3 Error taxonomy

Every per-file error in the report has a `code` field:

- `CONTENT_MISSING` — content store didn't have the hash.
- `CONTENT_READ_ERROR` — the store failed or returned a non-byte value.
- `CONTENT_SIZE_MISMATCH` — stored bytes did not match the manifest length.
- `CONTENT_HASH_MISMATCH` — stored bytes did not match the manifest SHA-256.
- `CONTENT_ABORTED` — cancellation interrupted content lookup or verification.
- `WRITE_FAILED` — adapter `writeFile` threw.
- `WRITE_TIMEOUT` — per-file timeout exceeded.
- `VERIFY_MISMATCH` — `--verify-after` read back wrong digest.
- `PERMISSION_DENIED` — adapter rejected with 403 / EACCES.
- `RATE_LIMITED` — adapter rejected with 429 and retries
  exhausted.
- `NETWORK_ERROR` — TCP-level / DNS failure.
- `UNKNOWN` — anything not in the above.

Error codes are stable for tooling: a CI job can grep `jq
'.files[] | select(.error.code == "WRITE_FAILED")'` to find
all write failures. `CONTENT_ABORTED` maps to the deploy's cancelled failure
path: after `begin()` it triggers rollback under §7.3; before `begin()` it
terminates without opening a transaction.

## 12. Content addressing

The runner identifies content by SHA-256 (as produced by the
emitters).  Two implications:

1. **Cache friendliness.**  Content-addressed identification
   means the target can sit behind a content-addressed cache
   (S3 If-Match, HTTP ETag) and skip writes when the bytes
   haven't changed.
2. **Optional content-hashed output paths.**  If the manifest
   uses content-hashed paths (e.g. `main.abc123.css`),
   long-lived caching is safe.  This is opt-in per file (the
   manifest emitter doesn't do it by default; v1+ may add as
   an option).  The runner doesn't care either way — it
   writes only a path that passed the portable-path, collision, ownership, and
   adapter-containment contract in §3.1.1.

## 13. Capability requirements

The runner runs at capability level:

- **`storage:read` / `storage:write`** — for content stores and deploy roots
  contained by the project storage boundary.
- **`filesystem:user`** — sensitive authority required only when the user
  explicitly selects a deploy root or content store outside project storage;
  adapters must still enforce their configured-root containment.
- **`network:<host>`** — one entry for each exact remote endpoint selected by
  target configuration; unrestricted `network:*` is not a v0 default.
- **`env:<name>`** — one entry for each credential variable named by the
  selected adapter; bare or wildcard environment access is not permitted.
- **NEVER** `shell`, `subprocess`, or unrelated env reads.

Each adapter declares its own `required_capabilities.json`
shape, scoped to its needs.  The top-level runner program's
manifest aggregates them based on which adapter is selected.

## 14. Determinism and reproducibility

- Given the same `(manifest, content store, target state)`, the
  runner produces the same deploy report (modulo wall-clock
  fields like `startedAt`, `elapsedMs` — those have an
  explicit `--deterministic-timestamps` mode for testing that
  zeros them).
- The diff plan is byte-identical between runs.
- The file write order is stable (sorted by `outputPath`)
  even when concurrent workers race to start — the recorded
  action order in the report uses each file's `outputPath` as
  the sort key, not the timing of its completion.

## 15. Out of scope for v0

- Streaming uploads for very large files (>100MB).  v0 buffers
  the whole file in memory.
- CDN cache purge directives.
- Per-file caching headers (Cache-Control, Expires) beyond
  what `contentType` implies.
- Multi-region atomic deploys.
- Schedule-windowed deploys (deploy only between 2 and 4 AM
  UTC, etc).
- Hot reload / live-update (this is a one-shot deploy).
- Dev-server mode (separate program, separate spec).

## 16. Open questions (for v1)

- **Resumable uploads** for the bundle content store (currently
  the runner streams the whole archive; a crash mid-deploy
  re-streams from the start).
- **Per-target verification** beyond SHA-256 (e.g., HTTP HEAD
  the final URL and assert `200` for HTML pages).
- **Soft-delete** mode where Phase 3 deletions go to a
  trash/archive subtree instead of permanent deletion.
- **Manifest-of-manifests** for multi-site deploys (deploy
  several sites in one transaction).

## 17. Delivery sequence

1. `forme-deploy-runner-core` package — TypeScript, all logic
   except adapter implementations.  Capabilities: `[]` (pure
   transform: takes a manifest + content store handles + an
   adapter handle, returns a deploy plan).
2. `forme-deploy-runner-fs-adapter` package — `FsAdapter`.
   Capabilities: `storage:write` for a project root, or the explicitly approved
   `filesystem:user` boundary for a user-selected external root.
3. `forme-deploy-runner-github-pages-adapter` package — the first hosted
   adapter. Capabilities: exact GitHub endpoint and one explicitly selected
   token variable; no shell or subprocess authority.
4. `forme-deploy` program — CLI binary composing the above.
   Capabilities: union of selected adapter + content store.

Each ships as its own PR, each independently testable in
isolation (the core uses a mock adapter for ~95% of test
coverage).
