# FM05 — Forme Deploy Runner

> **Status:** v0 specification.  Implementation pending.
> **Layer:** FM05 (last layer of the FM00 vision — applies a
> deploy manifest to a real target).
> **Predecessor:** `forme-aot-deploy-manifest-emitter` produces
> the manifest this runner consumes.

## 1. Purpose

The deploy runner is a **standalone program** (not a pure
transform) that reads a `deploy-manifest.json` produced by
`forme-aot-deploy-manifest-emitter`, resolves each file's
content from a content store, and writes every file to a
deployment target — local filesystem, S3 bucket, Netlify edge,
Cloudflare Pages, etc.

It is the **first FM00-cluster component with non-`[]`
capabilities** (it needs `fs` and/or `net`).  Every package
upstream of it stays a pure transform; the runner is the
trust-boundary where in-memory bytes become files on disk or
objects in a remote store.

## 2. Why a spec before implementation

Three reasons the runner ships as a spec first:

1. **Surface area.**  Deploy targets are diverse (fs / S3 /
   Netlify / Cloudflare / Vercel / arbitrary).  Locking the
   adapter interface before any single adapter exists prevents
   the first adapter from accidentally setting the contract for
   the rest.
2. **Capability budget.**  This is the only FM00 component
   that touches the outside world.  We want the capability
   shape (which env vars, which fs paths, which network hosts)
   nailed down in one place that reviewers can audit.
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

### 3.2 Required: content store

A keyed store from which the runner resolves each file's body.
The store is identified by SHA-256 hash — given a file entry
with `sha256: "abc..."`, the runner looks up `abc...` in the
store and gets the bytes back.

Three content store shapes are supported in v0:

- **`directory` store**: a local directory containing files
  named `<sha256>.bin`.  Lookup is `fs.readFile`.  Used when
  the runner runs in the same process / box as the emitter
  and the caller wrote the contents to disk.
- **`bundle` store**: a single `.tar` or `.zip` archive
  containing the same `<sha256>.bin` files.  Lookup streams
  the entry out of the archive.  Used for cross-machine
  deploys (the entire bundle ships as one file).
- **`inline` store**: an in-memory `Map<sha256, Uint8Array>`
  populated by the caller.  Used when emitter + runner share
  a process (long-lived dev server, CI worker).

The store interface (TypeScript):

```ts
interface ContentStore {
  /** Resolve a hash to its bytes.  Throws if missing. */
  readonly get: (sha256: string) => Promise<Uint8Array>;
  /** Quick "do you have this?" check without reading bytes. */
  readonly has: (sha256: string) => Promise<boolean>;
  /** Iterate every hash in the store.  Used for verification. */
  readonly hashes: () => AsyncIterable<string>;
}
```

### 3.3 Optional: previous deploy manifest

If supplied via `--previous <path>`, the runner uses it to
compute a **diff plan**: which files are new, changed, or
unchanged compared to the previous deploy.  Unchanged files
are skipped (no write); changed files are atomically replaced;
new files are atomically added; missing files (in the new
manifest but absent from the previous) are deleted at the end
of a successful deploy.

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
      "error": "..."           // only when action failed
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
- `--target <kind>` — `fs` | `s3` | `netlify` | `cloudflare` | ...

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
   * don't support this (fs, S3) return `null` and the runner
   * falls back to per-file atomicity only.
   */
  readonly begin: () => Promise<Transaction | null>;
}

interface Transaction {
  readonly commit:   () => Promise<void>;
  readonly rollback: () => Promise<void>;
}
```

### 6.1 v0 adapters

The v0 spec covers three reference adapters:

- **`FsAdapter`** — writes to a local directory rooted at
  `--target-config { "root": "<path>" }`.  Per-file atomicity
  via temp-file (`<outputPath>.tmp.<random>`) + `rename` (POSIX
  rename is atomic within a filesystem).  No transaction
  support.  Capabilities: `fs:write`.
- **`S3Adapter`** — `s3:PutObject` per file.  Per-file
  atomicity is S3's native model (PUTs are atomic on the
  object).  No transaction support across objects.  Auth via
  env-named `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` (or
  IAM instance profile).  Capabilities: `net:s3:*`.
- **`NetlifyAdapter`** — uses Netlify Deploy API.  Creates a
  deploy, uploads required files, finalizes.  **Supports
  transactions** — `begin` creates a draft deploy, `commit`
  publishes, `rollback` discards.  Capabilities:
  `net:netlify:*`, `env:NETLIFY_AUTH_TOKEN`.

Future v1+ adapters: Cloudflare Pages, Vercel, GitHub Pages.

## 7. Atomicity guarantees

### 7.1 Per-file (always)

Every adapter MUST guarantee that for any single `outputPath`,
the file is either fully written or unchanged.  A half-written
file (truncated, partially overwritten, mid-PUT) MUST NOT be
observable.

### 7.2 Per-bundle (when supported)

When `target.begin()` returns a transaction, the runner uses
it: all `writeFile` calls happen on the transaction; on success
`commit()`; on any failure `rollback()`.  Either all files are
applied or none are.

When transactions aren't supported (fs, S3), the runner does
per-file atomic writes in dependency-aware order:

1. **Phase 1**: write all files with their final paths suffixed
   `.deploying-<random>`.  This populates the target without
   touching anything callers might be reading.
2. **Phase 2**: atomically rename every `.deploying-<random>`
   to the final path (in dependency order: HTML last so
   internal links never point at a missing asset).
3. **Phase 3**: delete files that were in the previous deploy
   but are missing from the new one.

If Phase 1 fails: clean up `.deploying-*` and exit non-zero.
If Phase 2 fails partway: the deploy is in a mixed state; exit
code `1` (partial), report indicates which files made it.  We
do NOT auto-roll-back partial Phase 2 — re-running the deploy
is the recovery path.

### 7.3 Rollback

Automatic rollback only fires when:

- The target supports transactions and `commit` failed → call
  `rollback`.
- The user passed `--rollback-on-error` AND we have a previous
  manifest AND we haven't yet completed Phase 2 of the
  non-transactional flow.

Without an explicit opt-in or transaction support, the runner
**does not roll back** — partial state is reported and the
operator decides.  Hidden rollback is worse than visible
partial state.

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
- Resolve every file's content from the store (catches missing
  content errors).
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
all write failures.

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
   writes whatever path the manifest says.

## 13. Capability requirements

The runner runs at capability level:

- **`fs:write`** — for `FsAdapter` (and content `directory` /
  `bundle` stores).
- **`net:*`** — for remote adapters, scoped to the host(s)
  named in the target config.
- **`env`** — read-only, scoped to the env var names the
  config references.
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

## 17. Implementation plan (when this spec lands)

1. `forme-deploy-runner-core` package — TypeScript, all logic
   except adapter implementations.  Capabilities: `[]` (pure
   transform: takes a manifest + content store handles + an
   adapter handle, returns a deploy plan).
2. `forme-deploy-runner-fs-adapter` package — `FsAdapter`.
   Capabilities: `fs:write`.
3. `forme-deploy-runner-s3-adapter` package — `S3Adapter`.
   Capabilities: `net:s3:*`, scoped env reads.
4. `forme-deploy-runner-netlify-adapter` package —
   `NetlifyAdapter`.  Capabilities: `net:netlify:*`,
   `env:NETLIFY_AUTH_TOKEN`.
5. `forme-deploy` program — CLI binary composing the above.
   Capabilities: union of selected adapter + content store.

Each ships as its own PR, each independently testable in
isolation (the core uses a mock adapter for ~95% of test
coverage).
