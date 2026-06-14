# Changelog — forme-hello-world

## 0.1.0 — 2026-05-16

Initial release. First end-to-end Forme pipeline demo — proves the
five packages of the FM03 orchestrator stack (orchestrator,
pipeline-config, source-fs, parse-markdown, render-static, emit-fs)
hang together against real Markdown content.

### Added

- `content/hello.md` — single Markdown post with frontmatter
  (`title`, `date`, `slug`).
- `src/config.ts` — `makePipelineConfig({ contentRoot, outDir })`
  factory that returns a `PipelineConfig` wiring the four stages
  in linear order. Topology rationale and the deliberate omission
  of `forme-collect-chronological` for v0 are documented inline.
- `src/build.ts` — `buildBlog(opts)` function that instantiates the
  orchestrator, validates the config, builds the typed DAG, runs it
  once, and returns the orchestrator's `RunResult` for the caller
  to inspect.
- `src/cli.ts` — runnable entry point. Defaults `contentRoot` to
  `./content` and `outDir` to `./dist` relative to the package root.
  Streams structured logs via `consoleLogger()`. Exits with code 1
  on a non-success outcome.
- `tests/e2e.test.ts` — fixture-copy → run → assert dist/. Uses
  `os.tmpdir()` for isolation so the test can run in parallel and
  doesn't pollute the source tree.

### Topology decision (v0)

`source-fs → parse-markdown → render-static → emit-fs`.

`forme-collect-chronological` is intentionally omitted. The collector
produces a `Collection` kind that no v0 sink consumes, and the
renderer derives routes locally from `sourcePath` (per its own
v0 README). Wiring the collector in would create an orphan terminal
output that the orchestrator either errors on or silently drops.
The v0.2 plan is to introduce a router stage that folds
`Collection.entries[i].route` back onto `ContentNode.route`, at which
point this demo grows to include the collector and the renderer
becomes a single-stream consumer of routed nodes.
