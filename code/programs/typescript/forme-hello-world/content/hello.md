---
title: Hello, Forme
date: 2026-05-16
slug: hello
---

# Hello, Forme

This is the first post built end-to-end by the **Forme** authoring
pipeline. Four stages, one typed DAG, one HTML file on disk:

1. `forme-source-fs` walked the content directory and emitted a
   `ContentSource` for this file — bytes plus a stable `LogicalId`
   (UUIDv7) and a content-hash `RevisionId` (BLAKE2b).
2. `forme-parse-markdown` decoded the bytes, split the YAML-subset
   frontmatter, and handed the body to `gfm-parser`. The result is a
   `ContentNode` carrying a `DocumentNode` AST plus the parsed
   frontmatter.
3. `forme-render-static` walked the AST via `document-ast-to-html`,
   wrapped the body in a classless HTML5 theme, derived the title from
   `frontmatter.title`, and emitted a `RenderedPage`.
4. `forme-emit-fs` mapped the page's `route` (`/blog/hello.html`) to
   a path under `outDir`, wrote the UTF-8 bytes, and assembled the
   final `DeployArtifact` with a manifest naming this one route.

> The orchestrator (`forme-orchestrator`, FM03 §3–4) walks the typed
> DAG that connects all four stages. Stream-iteration promotion (the
> v0.1.1 fix) is what lets the per-item parser slot between the
> streaming source and the streaming renderer without an explicit
> adapter.

## What this proves

- Kind compatibility checking works on a real four-stage wire.
- The streaming scheduler delivers items lazily through fan-in /
  fan-out.
- Capability declarations round-trip — `forme-source-fs` declares
  `storage:read`, `forme-emit-fs` declares `filesystem:write`, the
  parser and renderer declare nothing, and the orchestrator's
  capability check accepts all of them.
- Identity is stable: re-running the pipeline against an unchanged
  source produces the same `buildId` (BLAKE2b over the route → sha256
  map). Once incremental rebuild (FM03 §6) lands, this becomes the
  hook for cache reuse across runs.

## What this does not prove yet

- **No collection.** `forme-collect-chronological` is wired up as a
  package but omitted from this demo until the router stage lands.
  A multi-post index page is the natural next demo.
- **No Style IR.** The theme is a hard-coded inline `<style>` block
  in `forme-render-static`. FM04 will replace that with a proper
  `StyleDocument` flowing through its own stages.
- **No interactivity.** No JavaScript is emitted. That's the point —
  static content ships zero bytes of JS, which is the FM00 thesis.
- **No incremental rebuild.** Every run re-executes every stage.
  FM03 §6 is the next chunk of orchestrator work.

Welcome to Forme.
