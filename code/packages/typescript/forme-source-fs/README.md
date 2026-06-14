# @coding-adventures/forme-source-fs

First actual pipeline stage in the Forme effort — walks a filesystem directory and emits one `ContentSource` per matching file (FM00 §5.1).

## Usage

```typescript
import sourceFs from "@coding-adventures/forme-source-fs";

// In your forme.config.ts:
{
  stage: sourceFs,
  config: { glob: "**/*.md", root: "./data" },
}
```

## Config

```typescript
interface SourceFsConfig {
  /** Glob pattern. v0 supports only "** /*.<ext>". */
  readonly glob: string;
  /** Directory to search; defaults to process.cwd(). */
  readonly root?: string;
}
```

## v0 simplifications

- **Glob is `**/*.<ext>` only.** No brace expansion, no negation, no character classes. Uses a tiny hand-rolled walker (`src/walker.ts`) instead of pulling in `fast-glob` or `picomatch`. Patterns like `posts/**/*.md` are not supported in v0 — set `root: "posts"` and `glob: "**/*.md"` instead.
- **Reads via `node:fs` directly**, not through `ctx.storage`. See "Capability discipline" below.
- **No watching.** `forme watch` mode (FM03 §7) lives in a future package.

## Capability discipline

Source stages have a chicken-and-egg problem with `ctx.storage`: the storage API is supposed to be the orchestrator-supplied implementation, but **for the source-fs stage to read disk, *something* has to be that implementation**. We are.

The pragmatic v0 resolution: source-fs declares `storage:read` in its `required_capabilities.json` (so the manifest layer audits it correctly) and reads via `node:fs/promises` directly. When the FM02 plugin host lands and the orchestrator wires real `StorageApi`s around stages, source-fs will be one of the storage *implementations* rather than a consumer.

Documented as a stage-level exception both in this README and in the source.

## Emitted ContentSource shape

For each matching file:

```typescript
{
  path: <relative to root>,
  bytes: <Uint8Array of file contents>,
  mimeType: "text/markdown",   // for .md/.mdx/.markdown; null for unrecognised exts
  identity: <fresh UUIDv7>,    // generateLogicalId() — TODO: persist + read from id.json
  revision: <blake2b:...>,     // computeRevisionId({ path, bytes })
  providerMeta: {
    mtimeMs: <file mtime>,
    sizeBytes: <file size>,
  },
}
```

## Walker behaviour

- Output is **sorted lexicographically** within each directory (deterministic across filesystems with different inode-order guarantees).
- **Skips:** dotfiles (`.git`, `.DS_Store`, etc.), symlinks (cycle hazard), non-regular files (sockets, devices), files whose extension doesn't match.
- **Recurses** into subdirectories with the same skip rules.
- **Missing root** returns no files (silent), not an error.

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets ≥90% line coverage. Walker + stage tests both run against real `os.tmpdir()` directories with cleanup.
