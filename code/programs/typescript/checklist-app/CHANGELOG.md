# Changelog

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
