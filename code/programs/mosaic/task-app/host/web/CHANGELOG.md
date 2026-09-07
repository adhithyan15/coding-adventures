# Changelog

All notable changes to the `task-app-web` host are documented here.

## [0.1.0] - Unreleased

### Fixed

- `vitest.config.ts` now resolves aliases from the ESM-native
  `import.meta.dirname`, so Vite's native config loader no longer warns about
  the unsupported CommonJS `__dirname` global.

- Startup no longer fails to a blank page. The host paints a loading state
  before the first `await` and replaces it with a failure state carrying the
  error detail and an in-place **Try again**, instead of leaving `#root` empty
  with the error only in the console. A 404 on the engine now reports its
  status rather than reaching `WebAssembly.compile` as an error page and
  surfacing as a misleading `CompileError`.

  The states live in `src/startup.tsx` — a real seam with tests, rather than
  glue inside the coverage-exempt `main.tsx`. `vitest.config.ts` had to widen
  its test glob to collect `.tsx` at all: it matched `*.test.ts` only, so a
  component test would have been collected by nobody and silently reported as
  a pass.

### Fixed

- Persistence is no longer silent: the host reports IndexedDB versus volatile
  memory in the app, surfaces background-save failures, and turns rejected saved
  snapshots into a recoverable startup warning. The rejected record is copied to
  the fixed `workspace/web-corrupt` recovery key before normal saves can replace
  `workspace/web`.

### Fixed

- List completion buttons now have state-aware ARIA names that include the
  task name while retaining the compact circle/check visual.

- A blank first-run or legacy root project is now presented as **Inbox**, matching
  `task-mosaic-app`, instead of exposing task-core's internal `project` id.

- The generated web host now completes a strict production TypeScript/Vite
  build after emitting both TaskApp themes. The host's `BUILD` contract runs
  that production build after Vitest, closing the CI gap that previously
  allowed invalid generated React styles and transitive type errors to merge.
- The copied `task-engine.mjs` accessor now has an extension-aware `.d.mts`
  declaration that TypeScript's bundler resolution recognizes.

### Added

- The real `task-wasm` module and web controller now consume the shared TaskApp
  presentation fixture, checking engine state and core slot values at every
  lifecycle checkpoint against the native adapter's identical contract.

- **A proper npm package for the web host.** Previously the host was a set of files
  overlaid onto a generated Vite project; it is now a committed package
  (`package.json`, `vite.config.ts`, `tsconfig.json`, `vitest.config.ts`, `index.html`,
  `BUILD`) with its dependencies and tests under version control. `scripts/build-web`
  now emits only the generated `TaskApp.tsx` component and copies the wasm runtime
  *into* `src/`/`public/`, instead of emitting a whole project and overlaying files.
- **Pluggable local persistence (`src/persistence.ts`).** The whole workspace is
  serialized via the engine's `snapshot()` and stored through the repo's canonical
  `KVStorage` contract (`@coding-adventures/indexeddb`'s `IndexedDBStorage`, with an
  in-memory fallback for private-browsing / SSR / tests). One whole-workspace record
  holds the engine snapshot plus the host-owned session state (row order + id
  high-water mark), so a reload restores the exact session. Writes are fire-and-forget
  after each structural mutation, mirroring the checklist-app pattern.
- **Boot-time restore (`src/main.tsx`).** On start the host opens storage, loads the
  saved snapshot into the engine, and seeds the controller's order/counter before the
  first render — no loading spinner, no lost work across reloads. New ids continue past
  the restored high-water mark so they never collide with loaded tasks.
- **Tests (`__tests__/persistence.test.ts`).** Round-trip the record shape, ordering,
  counter, array-copy isolation, and single-record overwrite semantics under jsdom.

### Verified

- End-to-end in a browser: add tasks, reload the page, and the tasks (with their
  computed schedule and completion state) are still there — driven by the Rust engine
  over WASM and persisted through IndexedDB, with no console errors.
