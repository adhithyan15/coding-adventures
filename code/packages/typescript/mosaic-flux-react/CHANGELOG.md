# Changelog

## 0.1.0

Initial release. Implements the strict-Flux runtime for Mosaic UI's React emitter per `code/specs/UI33-rewrite-unified-architecture.md`.

### Added

- `MosaicAction<State>` interface — the Command Pattern. Each action is a class with payload + `apply(state) → state` method.
- `isMosaicAction(value)` — structural type guard.
- `MosaicStore<State>` — state container + dispatcher.
  - `dispatch(action)` — runs `action.apply(state)`, swaps state, notifies subscribers, runs middleware.
  - `state` — current state (read-only).
  - `subscribe(selector, callback, equality?)` — fine-grained subscription; fires when the selected slice changes (default `Object.is`, custom equality supported).
  - `select(selector)` — one-shot read without subscription.
- `Middleware<State>` — cross-cutting hook signature.
- `composeMiddleware(arr)` — combines middleware; isolates throws so one bad middleware can't take down peers.
- `loggerMiddleware()` — dev logger with shallow-state-diff output.
- `createSelector(...inputs, combiner)` — memoised derived-state combinator. Reselect-shaped, zero-dep.
- `devToolsMiddleware(options?)` — publishes uniform UI33-rewrite §8 event stream via `window.postMessage` (with `ws://localhost:9229` fallback).
- React integration via the `./react` subpath:
  - `<MosaicStoreProvider store={...}>` — Context provider.
  - `useMosaicStore<State>()` — get the store imperatively.
  - `useMosaicSelector(selector, equality?)` — subscribe to a slice; re-renders on change. Uses React 18+ `useSyncExternalStore` for tearing prevention.
  - `useMosaicDispatch<State>()` — get a dispatch function bound to the store.

### Notes

- Zero runtime dependencies. React is a peer dependency for the `./react` entry only.
- Designed for use with the upcoming Mosaic UI codegen (UI33r-E-react phase) which generates action classes from `.mil` files. This runtime is the library those classes target.
- Synchronous dispatch only. Async effects flow through middleware that schedules subsequent dispatches.
