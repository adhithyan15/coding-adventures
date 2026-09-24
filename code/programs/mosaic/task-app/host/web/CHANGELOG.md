# Changelog

All notable changes to the `task-app-web` host are documented here.

## [0.1.0] - Unreleased

### Added — writing questions into a checklist template (C3c, #14018)

The web host serves the same authoring as `task-mosaic-app`: selecting outline
items, question/step toggling, Add to Yes/No and subtree delete, with the same
rows and props. "Add to Yes/No" deletes the item it created if extending the
branch fails, because this controller has no engine-level rollback. Tested in
`__tests__/checklists.test.ts`.

### Added — the Checklists view (C3b, #14018)

The web host now serves Checklists to the same contract as `task-mosaic-app`
(spec `task-app-checklists-view-v1.md`):

- It appears in the switcher before Timeline, at both tiers.
- **Library:** templates by name, then runs newest first, with progress
  subtitles and ✓/✗ badges.
- **A selected template:** its outline, with *Add item* (numbered with the new
  `setOrder`), Start run and Delete.
- **A selected run:** tick, answer Yes/No (answering again clears it),
  Complete (offered only once complete) and Abandon.
- **Bounds:** 512-character composers, the engine's item cap, and indents
  capped at 16 levels.
- **Guards and runs:** switching project clears the selection, and a calendar
  drop now refuses a key the views do not show. Runs are stamped with an
  injectable `now` (`ControllerInit.now`).
- `__tests__/checklists.test.ts` walks the same scenarios as the Rust tests.
- **The id counter is checked everywhere** (tasks, notes, labels, checklists,
  runs). A stored counter that isn't a safe non-negative integer (NaN, or at
  2^53, where `++` stops advancing) is recovered from the highest id already
  in use. It is never reset to 0, which would reuse ids. Minting fails
  rather than looping once the counter can't advance.

### Added — inert Checklists slots (C3a, #14018)

`TaskApp.mil` gained the Checklists view's slots, and the generated component
types them as required, so the host passes inert values. The view is not
offered on the web until C3b (web parity); its events fall through `apply`
unhandled. The presentation-contract test mirrors the Rust contract's
`checklists` view mapping.

### Changed — `navOptions` is a list of labels (UI86, #15420)

`switcherViews().map(([, label]) => label)`: the toolkit's SegmentedControl
reports the selected view through the kernel's selected state, so the host no
longer appends ", selected" to the showing view's accessible name.

### Changed

- The view switcher's props (#14016). `navOptions` holds
  `[label, accessible-name]` rows in the same order as `task-mosaic-app`,
  with Timeline only for a Full project; `navSelectedIndex` is the showing
  view. The controller handles `showView` by index into the views the
  project offers, and ignores an index it does not offer.

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
