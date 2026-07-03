# Changelog

## 0.1.0

Initial release. Vanilla-DOM strict-Flux runtime for Mosaic UI's HTML emitter, per `code/specs/UI33-rewrite-unified-architecture.md`.

### Added

- `MosaicAction<State>` interface + `isMosaicAction(value)` type guard.
- `MosaicStore<State>` with `dispatch`, `state`, `subscribe(selector, callback, equality?)`, `select(selector)`.
- `Middleware<State>`, `composeMiddleware(arr)` (with throw isolation), `loggerMiddleware()`.
- `createSelector(...inputs, combiner)` — memoised derived-state combinator.
- `devToolsMiddleware(options?)` — publishes UI33-rewrite §8 events via `window.postMessage` (with `ws://localhost:9229` fallback).
- Vanilla-DOM binding helpers:
  - `bindText(el, store, selector)` — `textContent`
  - `bindAttr(el, attrName, store, selector)` — HTML attribute (null removes)
  - `bindClass(el, className, store, predicate)` — CSS class toggle (multi-class via space-separated names)
  - `bindStyle(el, prop, store, selector)` — inline style property (null removes)
  - `bindList(container, store, listSelector, keyFn, renderItem)` — key-based child reconciliation preserving DOM identity for stable keys

### Notes

- Same core types as `mosaic-flux-react`. The packages are deliberately standalone to avoid coupling release cadences while the design is settling. A future `mosaic-flux-core` may extract the shared code.
- Zero runtime dependencies.
- Tested in jsdom via vitest.
- Designed as the target for the upcoming Mosaic UI HTML codegen (UI33r-E-html phase).
