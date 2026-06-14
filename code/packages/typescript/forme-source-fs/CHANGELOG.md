# Changelog — @coding-adventures/forme-source-fs

## 0.2.0 — 2026-05-16

### Added — identity persistence (read side)

Resolves the TODO from v0.1.0's CHANGELOG: "LogicalId is freshly
generated on every read (TODO: persist + read from id.json adjacent
to the source file when the identity-tracking story lands)."

When a hidden sidecar file `<dirname>/.<basename>.id.json` exists
next to a source file, source-fs now reads the persisted
`LogicalId` from it instead of generating a fresh one.  Sidecar
shape:

```json
{
  "logicalId": "01952c0d-7e63-7000-8000-...",
  "createdAt": "2026-05-16T00:00:00Z",   // optional
  "note": "optional human-readable note"   // optional
}
```

Only `logicalId` is required.  Unknown fields are ignored (draft-07-
style forward-compat).  Malformed JSON, missing/wrong-typed
`logicalId`, or wrong-version UUID values log a `warn` and fall
back to a fresh LogicalId — never fail the pipeline.

A new config field `persistIdentities: boolean` controls behaviour;
default `true`.  Set to `false` to opt out (matches v0.1.0
generate-every-read semantics; useful for ephemeral test fixtures
where stable identities don't matter).

The write side — automatically creating sidecars on first read —
is intentionally NOT in this release.  A separate stage (or CLI
command) will own that responsibility so source-fs can keep its
`storage:read`-only capability declaration; adding `filesystem:write`
to a read-shaped source would change the capability profile and
trigger user-facing install prompts.

### Fixed

- `walker.test.ts` was using POSIX `/` separators in assertions
  that ran against `path.join` output containing native
  separators.  Passed on Linux/macOS CI; would have failed on
  Windows CI as soon as a forme-source-fs change triggered a
  Windows-side BUILD (pre-existing bug from 0.1.0).  Now
  normalised to forward slashes in the assertion.

### Tests

13 new tests in `tests/identity-sidecar.test.ts` covering:
- Valid sidecar → persisted id used.
- Same id across runs when sidecar present.
- Different ids across runs when sidecar absent.
- Per-file sidecar resolution (no cross-file leakage).
- Malformed-JSON sidecar → warn + fallback.
- Wrong-shape sidecar (string / number / missing field) → fallback.
- UUIDv4 in sidecar → rejected, fallback.
- Forward-compat: unknown sidecar fields ignored.
- `persistIdentities: false` opts out.
- Sidecar files are not themselves emitted as content sources.
- Empty sidecar file → fresh id.

Total: 37 tests pass (up from 23).

## 0.1.0 — 2026-05-15

Initial release.  First actual pipeline stage in the Forme effort —
walks a filesystem directory and emits one `ContentSource` per
matching file (FM00 §5.1).

### Added

- `sourceFs` default-exported stage with `consumes: Kinds.Void`,
  `produces: streamOf(Kinds.ContentSource)`.
- `SourceFsConfig`: `{ glob: string; root?: string }`.
- Tiny hand-rolled directory walker (`src/walker.ts`) with
  `parseGlob` and `walkFiles`.
- ContentSource emission populates: `path` (relative to root),
  `bytes`, `mimeType` (text/markdown for .md/.mdx/.markdown;
  text/html for .html; text/plain for .txt; application/json for
  .json; null otherwise), `identity` (UUIDv7 from
  generateLogicalId), `revision` (computeRevisionId of
  `{path, bytes}`), `providerMeta: { mtimeMs, sizeBytes }`.

### v0 simplifications

- Glob is **`**/*.<ext>`** only — no brace expansion, no negation,
  no character classes.  Pulls no glob-library dependencies.
- Reads via `node:fs/promises` directly, not through `ctx.storage`.
  Documented as a "source stages ARE the storage adapter"
  pragmatic exception.
- No watching (FM03 §7).
- LogicalId is freshly generated on every read (TODO: persist +
  read from `id.json` adjacent to the source file when the
  identity-tracking story lands).

### Walker behaviour

- Sorted output within each directory (deterministic across FS).
- Skips dotfiles, symlinks, non-regular files.
- Recurses into subdirectories.
- Missing root ⇒ silent empty.
- Case-insensitive extension matching.

### Dependencies

- `@coding-adventures/forme-types` — Kinds, streamOf, ContentSource
- `@coding-adventures/forme-stage` — defineStage
- `@coding-adventures/forme-identity` — generateLogicalId,
  computeRevisionId
- `node:fs/promises`, `node:path` for the actual filesystem
  operations
