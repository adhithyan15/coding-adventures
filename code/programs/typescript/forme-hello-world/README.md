# forme-hello-world

End-to-end Forme demo: read a Markdown post, run it through the full
FM03 orchestrator pipeline, and write static HTML to disk.

This is the smallest thing that proves the Forme stack works
end-to-end. It exercises:

| Stage | I/O | Spec |
|---|---|---|
| `forme-source-fs` | `Void` → `Stream<ContentSource>` | FM00 §5.1 |
| `forme-parse-markdown` | `ContentSource` → `ContentNode` | FM00 §5.2 |
| `forme-router` | `Stream<ContentNode>` → `Stream<ContentNode>` | FM00 §5.4 |
| `forme-render-static` | `Stream<ContentNode>` → `Stream<RenderedPage>` | FM00 §5.5 |
| `forme-emit-fs` | `Stream<RenderedPage>` → `DeployArtifact` | FM00 §5.6 |

The orchestrator (`forme-orchestrator`, FM03 §3–4) walks the explicitly
wired typed DAG. The router is the sole owner of URL policy; the renderer
requires and preserves its canonical route.

## Run it

```bash
npm install
npm start
# → dist/blog/hello.html
```

Open `dist/blog/hello.html` in a browser to see the rendered post with the
reusable `forme-theme-classless` Style IR theme. The renderer records matched
rules and inlines its AOT-sliced light/dark CSS.

## Run the tests

```bash
npm test
```

The end-to-end test copies `content/hello.md` into an OS tempdir, runs
the full pipeline against it, and asserts that the expected HTML
artifact exists and contains the expected substrings.

## What's deliberately NOT here (yet)

- **`forme-collect-chronological`** is omitted from the topology.
  The collector produces a `Collection` (not a sink kind), so this minimal
  linear demo would create an orphan output. The complete blog example fans
  routed nodes into collector and renderer branches and consumes both.

- **Watch mode** (FM03 §7). The orchestrator's v0 release shipped
  `runOnce` only.

- **Cache reuse across runs.** The orchestrator currently constructs a
  fresh in-memory cache on each `createOrchestrator` call. Persistent
  cache wiring waits on incremental rebuild (FM03 §6).

- **A real CLI binary** — see `code/specs/FM03-forme-orchestrator.md`
  §15 for the eventual `forme run` shape. This program is the
  TypeScript-direct-import shape; the CLI will come with FM07.

## Files

- `content/hello.md` — the source post (the only piece of content).
- `src/config.ts` — `makePipelineConfig({ contentRoot, outDir })`,
  the pure value the orchestrator consumes.
- `src/build.ts` — `buildBlog({ contentRoot, outDir, logger? })`,
  the function that drives the orchestrator end-to-end.
- `src/cli.ts` — the executable entry point (`npm start`).
- `tests/e2e.test.ts` — the integration test that pins the demo's
  observable output.
