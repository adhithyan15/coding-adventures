## Unreleased — vite build reads at most 64 raw lesson files at a time

- **Fix:** on the Windows CI runner, `vite build` failed with tens of thousands
  of `[plugin vite:asset] Error: EMFILE: too many open files` errors, on both
  vite 8.1.5 (#17522) and 8.3.4 (#17972). `src/lesson-sources.ts` globs every
  lesson with `?raw`, and Vite's `vite:asset` plugin answers each of those
  ~39,000 loads with an unbounded `readFile`. The runtime loader got the same
  cure in the previous entry; this applies it to the bundler.
- **New `raw-load-throttle-plugin.ts`:** a build-only, `enforce: "pre"` plugin
  that claims `.md?raw` ids under the curriculum root, reads them with at most
  `RAW_LOAD_CONCURRENCY` (64) in flight, and returns the same
  `export default "<contents>"` module `vite:asset` would. Every other id falls
  through to Vite. The dev server is untouched.
- **Verified:** the built `dist/` is byte-for-byte identical with and without
  the plugin (13,778 files), `check:bundle` passes, and an instrumented build
  showed the plugin serving all 40,000+ lesson loads. macOS does not reproduce
  the EMFILE even at `ulimit -n 512`, so the Windows runner is the real test.
- **Tests:** `tests/raw-load-throttle-plugin.test.ts` checks the module shape,
  that non-lesson ids are left alone, that in-flight reads never exceed the
  limit, that a failed read frees its slot, and that bad limits are rejected.
