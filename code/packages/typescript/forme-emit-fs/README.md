# @coding-adventures/forme-emit-fs

Forme emit stage: `Stream<RenderedPage>` → `DeployArtifact`. Writes
each rendered page to disk under a configured `outDir` and emits a
single `DeployArtifact` summarising the result.

Fifth (final) Forme stage of the blog v0 effort. This is the bottom
of the pipeline — the bytes it writes are the bytes the deploy job
ships.

## Stage shape

```ts
import emit from "@coding-adventures/forme-emit-fs";

emit.consumes      // streamOf(Kinds.RenderedPage)
emit.produces      // Kinds.DeployArtifact
emit.capabilities  // ["filesystem:write"]
emit.configSchema  // { type: "object", required: ["outDir"], properties: { outDir: { type: "string" } } }
```

## What it does

For each `RenderedPage` arriving on the stream:

1. Map `route` → on-disk path under `outDir` (see *Safety* below).
2. `mkdir -p` the parent directory.
3. Write `page.html` as UTF-8 bytes via `node:fs/promises.writeFile`.
4. Stash `{ path, bytes }` for the manifest.

After the stream finishes:

5. Emit a single `DeployArtifact`:
   - `variant: { kind: "dist-tree" }` (a static-files tree)
   - `files: Record<routePath, Uint8Array>` (POSIX-separator keys)
   - `manifest.routes: DeployRoute[]` (one per emitted page,
     `target: { kind: "file", path: <route-path> }`, `islands: []`,
     `css: []`)
   - `manifest.assets: []`
   - `manifest.buildTime: ISO timestamp from ctx.time.nowIso()`
   - `manifest.buildId: blake2b over { route → sha256 }`

## Safety

`routeToOutPath` (in `path-utils.ts`) refuses to write outside
`outDir`:

- Empty routes throw.
- Bare `"/"` throws (no filename component).
- Routes with multiple leading slashes (`"//etc/passwd"`) throw.
- Routes whose resolved path falls outside `resolve(outDir) + sep`
  throw (catches `..` traversal, even when interleaved with normal
  segments — `/blog/../../escape.html`).

These checks are pure path math — they run **before** any file open,
so a malicious route never reaches the kernel.

## Capability discipline (v0 pragmatic exception)

Emit stages have the same chicken-and-egg problem as
`forme-source-fs`: `ctx.filesystem` is the orchestrator-provided
`FilesystemApi`, but for `forme-emit-fs` to *write* disk *something*
has to be the implementation.

v0 resolves it the same way the source stage does:

- Declare `"filesystem:write"` in `required_capabilities.json` and
  in the stage's `capabilities` array so the manifest audit layer
  sees correct intent.
- Read via `node:fs/promises` directly (the runtime path).

When FM02 lands and the orchestrator wires real `FilesystemApi`s
around stages, this stage will become one of the implementations
rather than a consumer. Documented as a stage-level pragmatic
exception in `required_capabilities.json` and the source header.

## Reproducible builds

`buildTime` comes from `ctx.time.nowIso()` rather than
`new Date()`, so a pipeline run under `frozenClock` produces a
deterministic timestamp (FM03 §8 reproducible-build mode).

`buildId` deliberately hashes only the file → sha256 map, NOT
`outDir`. The same site deployed to two different directories is the
same build — the build identity is what's in the artifact, not where
it lives. The canonical-JSON serialiser sorts keys, so stream-arrival
order doesn't affect the hash either.

## v0 simplifications (documented)

- **`manifest.assets` is always `[]`** — no asset-emission stages
  yet. Assets will get their own emit pipeline that this stage's
  manifest merges with.
- **`DeployRoute.islands` / `DeployRoute.css` are always `[]`** —
  the renderer doesn't track them (theme CSS is inlined in `<style>`
  per page). When FM04 (Style IR) + FM05 (Interactivity IR) land,
  the renderer will populate them and this stage will copy them
  through verbatim.
- **`DeployRoute.target` is always `{ kind: "file", path }`** — no
  handler-typed routes for the static blog.

## Config

```ts
interface EmitFsConfig {
  outDir: string;   // REQUIRED — no default; an unset outDir is a bug
}
```

## Dependencies

- `@coding-adventures/forme-types` — `Kinds`, `streamOf`,
  `RenderedPage`, `DeployArtifact`, `DeployRoute`, `DeployManifest`.
- `@coding-adventures/forme-stage` — `defineStage`, `StageContext`.
- `@coding-adventures/forme-identity` — `computeRevisionId` for the
  blake2b build-id.
- `node:fs/promises`, `node:path`, `node:crypto` — the runtime path
  (see *Capability discipline* above).

## Tests

```
npx vitest run --coverage
```

Coverage target 90%+ line. Tests run against real `os.tmpdir()`
with cleanup; the build-id reproducibility test exercises the
across-runs invariant.
