# Changelog

## 0.1.0

Initial release. Pure-Dart strict-Flux runtime for Mosaic UI's Flutter emitter per `code/specs/UI33-rewrite-unified-architecture.md`.

### Added

- `MosaicAction<S>` abstract class with `apply(S state) → S`.
- `MosaicStore<S>` class:
  - Constructor takes `initialState` and optional `middleware` list.
  - `state` — read-only synchronous accessor.
  - `dispatch(action)` — synchronous; runs apply, swaps state, notifies subscribers, runs middleware. No-op transforms still run middleware.
  - `subscribe(selector, callback, {equality})` — fine-grained; returns unsubscribe closure.
  - `select(selector)` — one-shot read.
  - `addListener(listener)` — bulk change notification for Flutter ChangeNotifier-style consumers; returns unsubscribe closure.
- `Middleware<S>` typedef `void Function(MosaicAction<S>, S, S)`.
- `composeMiddleware(list)` — combines middleware in registration order; isolates throws.
- `loggerMiddleware()` — prints action runtime type on dispatch.
- `createSelector1` / `createSelector2` / `createSelector3` — memoised derived state.
- `devToolsMiddleware({storeName})` — emits UI33-rewrite §8 events as stdout log lines (TCP transport deferred to v0.2.0).

### Why pure Dart instead of Flutter dependency

v0.1.0 keeps the package usable by non-Flutter consumers (CLI tools, server-side Dart, Dart Frog). The Flutter SDK is heavyweight and brings in many transitive dependencies; binding the runtime to it would prevent non-Flutter Dart projects from using it.

### Tests

- 31 tests passing via `dart test`. Distributed across action, store (12 tests), middleware, selector, devtools.

### Deferred to v0.2.0

- `MosaicBuilder` widget — `StatefulWidget` that rebuilds on store changes (declarative Flutter integration).
- TCP socket DevTools transport on `localhost:9229`.
- Time-travel replay support on the runtime side.
- Optional ChangeNotifier mixin for drop-in Provider compatibility.
