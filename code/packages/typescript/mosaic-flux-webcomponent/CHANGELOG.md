# Changelog

## 0.1.0

Initial release. Web Component strict-Flux runtime for Mosaic UI per `code/specs/UI33-rewrite-unified-architecture.md`.

### Added

- All core types from `mosaic-flux-html` (duplicated in this package per UI33-rewrite §6.1 "standalone runtimes" policy):
  - `MosaicAction<State>` + `isMosaicAction` guard
  - `MosaicStore<State>` with `dispatch` / `state` / `subscribe` / `select`
  - `Middleware<State>` + `composeMiddleware` + `loggerMiddleware`
  - `createSelector` memoised derived-state combinator
  - `devToolsMiddleware` (postMessage + ws fallback)
  - DOM binding helpers: `bindText` / `bindAttr` / `bindClass` / `bindStyle` / `bindList`
- **`MosaicHostElement<State>` base class** — custom-element base with managed binding lifecycle:
  - `attachShadowIfNeeded()` — idempotent open-shadow attachment
  - `track(unsubscribe)` — register cleanup; auto-disposed on `disconnectedCallback`
  - `dispatch(action)` shortcut delegating to the bound store
  - `connectedCallback` invokes subclass `bindStore(store)` when a store is set
  - `disconnectedCallback` invokes every tracked unsubscribe
  - Setting `element.store = s` after connection triggers rebind (disposes old bindings, calls `bindStore(s)`)
  - Setting `element.store = null` disposes existing bindings
  - Bad unsubscribes are isolated (one throwing unsubscribe doesn't prevent others from running)
- **`defineMosaicElement(tagName, class, options?)`** — idempotent `customElements.define` wrapper. Second call with same tag is a no-op (first definition wins). Throws if `customElements` is unavailable. Module reload / HMR safe.

### Future direction

- v0.2.0 may extract the shared core into `mosaic-flux-core` once all 7 backends ship and the architecture has settled. Until then, the duplicated core is intentional per UI33-rewrite §6.1.

### Tests

- 75 unit tests via vitest (jsdom env)
- Coverage: 92.39% lines / 95.16% branches / 88.88% funcs / 92.39% statements
- `element.ts` (the unique surface) at 100% on every metric
