# Changelog

All notable changes to the `task-app-web` host are documented here.

## [0.1.0] - Unreleased

### Added

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
