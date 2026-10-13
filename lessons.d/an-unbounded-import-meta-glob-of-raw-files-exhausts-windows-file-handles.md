---
category: TypeScript / JavaScript
---

# An unbounded import.meta.glob of raw files exhausts Windows file handles as the corpus grows

**What happened:** on PR #17185, `build (windows-2025)` failed in
`typescript/programs/language-ladder` with 15,738 `EMFILE: too many open
files` errors from `[plugin vite:asset]`. The same package had built on Windows
one head earlier; only the lesson count had grown. `src/lesson-sources.ts`
globs every lesson Markdown file with `?raw`. Vite 8's asset loader answers
each load with `await fsp.readFile(file)`, and rolldown calls every load hook
at once, so ~29,600 files are opened in parallel. The same build fails on
Linux under `ulimit -n 1024` (21,899 errors). Linux CI passes only because its
ulimit is far higher.

**Fix:** `bounded-raw-markdown-plugin.ts` is an `enforce: "pre"` plugin that
answers only `?raw` Markdown loads inside the curriculum root. It returns the
same `export default <text>` module Vite would, after waiting for one of 64
read slots. Its `dist/` is byte-identical to the unbounded build, and it passes
under `ulimit -n 1024`.

**It came back in the tests:** the next Windows run built fine, then
`npx vitest run` failed with the same `EMFILE` in three test files. Vitest
reads `vitest.config.ts`, which keeps its own `plugins` list and does not
inherit `vite.config.ts`. The plugin went into the build config only. Adding it
to `vitest.config.ts` fixed it, and that also reproduces under `ulimit -n 1024`.

**Do differently:** a glob that grows with content will eventually outgrow the
OS file-handle limit. Reproduce on Linux with a low `ulimit -n` before blaming
the runner, and cap concurrency in a plugin rather than raising limits.
Rolldown 1.1.5 has no `maxParallelFileOps` option to do this for you.
When a fix lives in a Vite plugin, register it in every config that loads the
same modules: `vite.config.ts`, `vitest.config.ts`, and any other entry point.
