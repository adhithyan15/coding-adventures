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
  /** Create/read stable adjacent identity sidecars. Defaults to true. */
  readonly persistIdentities?: boolean;
}
```

## v0 simplifications

- **Glob is `**/*.<ext>` only.** No brace expansion, no negation, no character classes. Uses a tiny hand-rolled walker (`src/walker.ts`) instead of pulling in `fast-glob` or `picomatch`. Patterns like `posts/**/*.md` are not supported in v0 — set `root: "posts"` and `glob: "**/*.md"` instead.
- **Reads and creates identity sidecars via `node:fs` directly**, not through `ctx.storage`. See "Capability discipline" below.
- **Each invocation is a one-shot scan.** The product CLI owns `forme watch`
  and reruns the pipeline when project files change.

Before `run`, the orchestrator calls the stage's `externalState` hook. The hook
publishes a canonical manifest sorted by portable `/`-separated locator. Each
entry carries the source's stable `LogicalId` and binary content `RevisionId`;
the manifest revision hashes the complete entry list. The hook and `run` share
one `ctx.cache` snapshot, so files are read once and the observation cannot
describe different bytes from the values emitted in that invocation.

## Capability discipline

Source stages have a chicken-and-egg problem with `ctx.storage`: the storage API is supposed to be the orchestrator-supplied implementation, but **for the source-fs stage to read disk, *something* has to be that implementation**. We are.

The pragmatic v0 resolution: source-fs declares `storage:read` and `storage:write` in its `required_capabilities.json` and accesses the filesystem via `node:fs/promises`. The write capability is limited to exclusive creation of missing `.<basename>.id.json` sidecars; existing metadata is never replaced. When the FM02 plugin host lands and the orchestrator wires real `StorageApi`s around stages, source-fs will be one of the storage *implementations* rather than a consumer.

With `persistIdentities` enabled, first encounter creates a UUIDv7 sidecar atomically. Concurrent builds converge on the winning file. An existing malformed sidecar fails the build so published identity cannot silently change; delete or repair it explicitly. Set `persistIdentities: false` for an ephemeral, write-free source.

Documented as a stage-level exception both in this README and in the source.

## Emitted ContentSource shape

For each matching file:

```typescript
{
  path: <relative to root>,
  bytes: <Uint8Array of file contents>,
  mimeType: "text/markdown",   // for .md/.mdx/.markdown; null for unrecognised exts
  identity: <stable UUIDv7>,   // read or atomically created in .<name>.id.json
  revision: <blake2b:...>,     // binary content hash; unchanged by rename
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
