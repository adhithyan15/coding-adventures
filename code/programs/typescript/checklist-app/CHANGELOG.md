# Changelog

## [0.7.0] - Unreleased

### Changed

- **BUILD file** — now chains shared package installs (indexeddb → store →
  ui-components → checklist-app) before running Vitest, matching the engram-app
  pattern. Without this, CI fails when shared packages aren't installed first.
- **package.json** — bumped to 0.7.0.
- **InstanceRunner, TemplateEditor** — replaced V0.2/V0.3 historical version
  annotations with clean current-state documentation describing the architecture
  as it stands today.
- **TemplateLibrary, TodoList** — added missing `type="button"` attributes to all
  buttons (Run, Edit, Delete, New Checklist, New Todo). Buttons default to
  `type="submit"` which can trigger form submission unexpectedly; explicit
  `type="button"` is always correct for action buttons outside forms.

## [0.6.0] - Unreleased

### Added

- **Electron desktop app** — wraps the existing React app in an Electron
  shell. Same code, now runs as a native desktop application.
- **electron/main.ts** — main process with literate-programming docs
  explaining Electron's architecture, security model, and process separation.
- **electron-builder.yml** — packaging for macOS (dmg+zip), Windows
  (nsis+portable), and Linux (AppImage).
- **GitHub Actions release workflow** — `release-checklist.yml` triggers on
  `checklist-v*` tags, builds for all 3 platforms in parallel, uploads
  binaries to GitHub Releases.
- **Electron dev mode** — `npm run electron:dev` opens a desktop window
  pointing at the Vite dev server (hot reload works).

### Changed

- **vite.config.ts** — base changed from `/coding-adventures/checklist/` to
  `"./"` for file:// protocol compatibility in Electron.
- **package.json** — bumped to 0.6.0, added `"main"` field, electron +
  electron-builder + cross-env dev dependencies, electron:* scripts.

## [0.5.0] - Unreleased

### Added

- **Due dates** — optional YYYY-MM-DD due date on todo items, stored as
  an ISO 8601 date string (JSON-serializable, timezone-agnostic)
- **DatePicker** — shared component in `@coding-adventures/ui-components`
  wrapping `<input type="date">` with dark theme styling and clear button
- **Overdue highlighting** — todos past their due date shown in red in the list
- **TodoEditor** — date picker field for setting/clearing due dates

## [0.4.0] - Unreleased

### Added

- **Todo list** — basic CRUD for todo items integrated into the app
- **TodoItem data model** — id, title, description, status (todo/in-progress/done),
  createdAt, updatedAt. All JSON-serializable for future DB compatibility.
- **4 new actions** — TODO_CREATE, TODO_UPDATE, TODO_DELETE, TODO_TOGGLE
- **2 new screens** — TodoList (grouped by status) and TodoEditor (create/edit form)
- **Header navigation** — Checklists / Todos tab bar for switching between modes
- **IndexedDB v2** — new "todos" object store, schema upgrade from v1
- **Persistence** — todos saved to IndexedDB via the same middleware pattern

## [0.3.0] - Unreleased

### Added

- **@coding-adventures/store integration** — replaced the mutable AppState
  singleton with an immutable Flux-style store. All state changes now flow
  through dispatch(action) -> reducer -> new state -> listeners.
- **@coding-adventures/indexeddb integration** — persistent storage via
  IndexedDB with automatic MemoryStorage fallback for environments where
  IndexedDB is unavailable (private browsing, Node test runners).
- **src/actions.ts** — action type constants and action creator functions
  following the Flux pattern. Each state transition has a named constant
  and a factory function.
- **src/reducer.ts** — pure reducer function that computes new state from
  (state, action) pairs. All mutation logic moved here from state.ts.
  Returns new objects (never mutates) for React compatibility.
- **src/persistence.ts** — middleware that intercepts dispatched actions
  and writes affected records to IndexedDB. Fire-and-forget for UI speed.

### Changed

- **src/state.ts** — simplified from 350 lines of mutation functions to a
  thin wrapper that creates the Store and re-exports utility functions.
- **src/main.tsx** — now async: opens IndexedDB, loads persisted data,
  attaches persistence middleware, then seeds or hydrates before mounting.
- **src/seed.ts** — accepts a Store and dispatches TEMPLATE_CREATE actions
  instead of calling createTemplate() directly.
- **All components** — switched from `appState` singleton reads and direct
  mutation calls to `useStore(store)` for reactive reads and
  `store.dispatch(action)` for writes. Removed the `setTick` re-render
  hack from InstanceRunner (useStore handles re-renders automatically).
- **Test suite** — rewritten to create a Store, dispatch actions, and
  assert on store.getState(). Same test cases, new API surface.
- **BUILD file** — now installs indexeddb and store package dependencies
  before building the checklist app.

## [0.1.0] - Unreleased

### Added

- **Template CRUD** — create, edit, and delete checklist templates in memory
- **Decision-tree items** — two item types: `check` (a simple step) and
  `decision` (a yes/no question with two branches); nesting is unlimited
- **Instance execution** — each "Run" deep-clones a template into an
  independent instance; two runs of the same template do not share state
- **flattenVisibleItems** — tree-walking algorithm that returns only the
  items on the active decision path; hidden branches are never shown
- **Stats computation** — pure function over the final instance state:
  completion rate, total/checked item counts, decision count, elapsed time
- **Four screens** — Template Library, Template Editor, Instance Runner,
  Stats View; navigation via URL hash router (`#/path`)
- **Seed data** — three pre-loaded example templates demonstrating flat,
  branching, and nested-decision checklists
- **i18n** — all UI strings externalised to `src/i18n/locales/en.json` via
  `@coding-adventures/ui-components` i18n singleton
- **Unit test suite** — 30+ tests for `state.ts` (95%+ coverage); component
  tests with React Testing Library for all four screens
- **Tech stack** — React 19, Vite 6, Vitest 3, TypeScript strict mode,
  `@coding-adventures/ui-components` for shared theme and i18n
