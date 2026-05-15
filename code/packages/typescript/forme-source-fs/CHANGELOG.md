# Changelog — @coding-adventures/forme-source-fs

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
