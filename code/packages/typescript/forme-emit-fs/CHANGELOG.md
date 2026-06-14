# Changelog — @coding-adventures/forme-emit-fs

## 0.1.0 — 2026-05-15

Initial release. Fifth (and final) Forme stage of the blog v0 effort.

### Added

- `emitFs` default-exported stage:
  - `consumes: streamOf(Kinds.RenderedPage)`
  - `produces: Kinds.DeployArtifact`
  - `capabilities: ["filesystem:write"]`
  - `configSchema: { outDir: string }` (outDir required)
- Per-page pipeline: `routeToOutPath(outDir, page.route)` → `mkdir -p`
  parent → `writeFile(absPath, utf8Bytes)`.
- Final emit: `DeployArtifact` with `variant: { kind: "dist-tree" }`,
  `files: Record<routePath, Uint8Array>`, `manifest.routes` per page,
  `manifest.assets: []`, `manifest.buildTime` from `ctx.time.nowIso()`,
  `manifest.buildId` blake2b over the route → sha256 map.
- `routeToOutPath` — traversal-guarded route mapping; refuses empty
  routes, bare `/`, multiple leading slashes, and any route whose
  resolved path falls outside `resolve(outDir) + sep`.
- `sha256Hex` helper exported for downstream stages that need to
  pre-compute file hashes against the same algorithm.
- Cancellation honoured between input pages.

### Spec adherence

No deliberate divergences from FM00 / FM01.

### v0 simplifications (documented)

- **`manifest.assets` is always `[]`** — no asset-emission stages
  yet.  Assets will get their own pipeline that merges into this
  manifest.
- **`DeployRoute.islands` / `DeployRoute.css` are always `[]`** — the
  renderer doesn't track them (theme CSS is inlined in `<style>`).
  When FM04 (Style IR) / FM05 (Interactivity IR) land, the renderer
  will populate them and this stage will copy them through verbatim.
- **`DeployRoute.target` is always `{ kind: "file", path }`** — no
  handler-typed routes for the static blog.
- **`filesystem:write` capability declared without a scoped path
  detail.**  A scoped form like `filesystem:write:<outDir>` would be
  more precise but requires runtime substitution from config; v0
  declares the coarse form.

### Capability discipline (v0 pragmatic exception)

Emit stages have the same chicken-and-egg problem as
`forme-source-fs`: `ctx.filesystem` is the orchestrator-provided
`FilesystemApi`, but for this stage to *write* disk *something* has
to be the implementation.  Resolution mirrors source-fs: declare the
capability in `required_capabilities.json` + the stage's `capabilities`
array, and use `node:fs/promises` directly at runtime.  When FM02
lands and the orchestrator wires real `FilesystemApi`s around stages,
this stage becomes one of the implementations rather than a consumer.

### Notes

- `buildId` deliberately hashes only the file → sha256 map, NOT
  `outDir`.  The same site deployed to two different directories is
  the same build.  Canonical-JSON key sorting in the hash input makes
  the id insensitive to stream-arrival order too.
- `buildTime` comes from `ctx.time.nowIso()` so a pipeline run under
  `frozenClock` produces a deterministic timestamp (FM03 §8).
- File map keys are POSIX-style (`/` separator) regardless of host
  platform — manifests stay reproducible across Linux/macOS/Windows.
