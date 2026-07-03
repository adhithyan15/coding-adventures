# Changelog

## 0.1.0

Initial release. JVM/Kotlin strict-Flux runtime for Mosaic UI's Jetpack Compose emitter per `code/specs/UI33-rewrite-unified-architecture.md`.

### Added

- `MosaicAction<S>` interface with `apply(state: S): S`.
- `MosaicStore<S>` class:
  - Constructor takes `initialState` and optional `middleware` list.
  - `state: S` — read-only synchronous accessor.
  - `stateFlow: StateFlow<S>` — Compose-aware reactive surface for `collectAsState()`.
  - `dispatch(action)` — synchronous; runs apply, swaps state, notifies subscribers, runs middleware. No-op transforms still run middleware.
  - `subscribe(selector, equality, callback)` — returns unsubscribe closure.
  - `select(selector)` — one-shot read.
- `Middleware<S>` typealias `(MosaicAction<S>, S, S) -> Unit`.
- `composeMiddleware(list)` — combines middleware in registration order; isolates throws.
- `loggerMiddleware()` — prints action class name on dispatch.
- `createSelector` — memoised derived state with 1-, 2-, 3-input variants.
- `devToolsMiddleware(storeName)` — emits UI33-rewrite §8 events as stdout log lines (TCP transport deferred to v0.2.0).

### Build

- Gradle 8.14+ with Kotlin JVM plugin (Kotlin 2.0).
- JVM 17 minimum.
- Gradle output redirected to `.gradle-out/` to avoid name collision with the repo's `BUILD` script on case-insensitive macOS HFS+.

### Tests

- 27 JUnit5 tests, all passing on JDK 21.
- Distributed across 5 test classes (Action, Store, Middleware, Selector, DevTools).

### Deferred to v0.2.0

- TCP socket DevTools transport on `localhost:9229`.
- Compose-specific helpers (state-driven Modifier composition, etc.).
- Time-travel replay support on the runtime side.
