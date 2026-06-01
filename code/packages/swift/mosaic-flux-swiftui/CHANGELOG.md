# Changelog

## 0.1.0

Initial release. Apple-platform strict-Flux runtime for Mosaic UI per `code/specs/UI33-rewrite-unified-architecture.md`.

### Added

- `MosaicAction` protocol with `associatedtype State` and `func apply(to:) -> State`.
- `MosaicStore<State>` final class:
  - `init(initialState:middleware:)` — store construction with optional middleware.
  - `state: State` — read-only computed property.
  - `dispatch<A: MosaicAction>(_:) where A.State == State` — synchronous dispatch.
  - `subscribe(selector:equality:callback:)` — fine-grained subscription with custom equality (defaults to reference identity for class types, `false` for value types so explicit `==` is required for Equatable value selectors).
  - `select(_:)` — one-shot read without subscription.
- `Middleware<State>` typealias `(any MosaicAction<State>, State, State) -> Void`.
- `composeMiddleware(_:)` — combines middleware in registration order; no-op for empty array.
- `loggerMiddleware()` — prints action class name on each dispatch.
- `createSelector` — memoised derived-state combinator with 1-, 2-, and 3-input arities.
- `devToolsMiddleware(storeName:)` — emits UI33-rewrite §8 events. v0.1.0 logs to stdout in human-readable form; v0.2.0 will add TCP socket transport on port 9229.

### Platform requirements

- macOS 14+, iOS/iPadOS 17+, watchOS 10+, tvOS 17+, visionOS 1+
- Swift 5.9+ toolchain

### Deferred to v0.2.0

- **`MosaicObservable` SwiftUI wrapper** — provides `@Bindable`-friendly surface so SwiftUI views can read store state directly without manual `subscribe()` plumbing.
- **TCP socket DevTools transport** — connects to local Mosaic DevTools desktop app on `localhost:9229`.
- **Time-travel replay support** on the runtime side (v0.1.0 logs events; replay is driven externally by the DevTools client).

### Tests

- 26 XCTest unit tests, all passing
- Coverage: action, store (dispatch + subscribe + select + middleware integration), selector (1/2/3-input), middleware (compose + logger), devtools (callable + custom store name + store integration)
