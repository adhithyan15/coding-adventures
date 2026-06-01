# MosaicFlux (mosaic-flux-swiftui)

Strict-Flux runtime for Mosaic UI's SwiftUI emitter — the first non-TypeScript runtime in the family.

Implements the architecture specified in `code/specs/UI33-rewrite-unified-architecture.md`:

- **`MosaicAction`** — the Command Pattern protocol. Each action is a struct (or class) carrying its payload as stored properties + an `apply(to:)` method that expresses the state transform.
- **`MosaicStore<State>`** — state container + dispatcher. Routes by calling `action.apply(to:)` directly.
- **Fine-grained subscription** — selectors fire only when the projected slice changes (by reference identity by default; pass a custom equality function for value-typed slices).
- **`Middleware<State>`** — cross-cutting hook with `composeMiddleware` and `loggerMiddleware`.
- **`createSelector`** — memoised derived-state combinator (1-, 2-, and 3-input variants).
- **`devToolsMiddleware`** — emits UI33-rewrite §8 protocol events; v0.1.0 logs to console, v0.2.0 will add TCP socket transport on port 9229.

## Platforms

| Platform | Minimum version |
|---|---|
| macOS | 14 (Sonoma) |
| iOS / iPadOS | 17 |
| watchOS | 10 |
| tvOS | 17 |
| visionOS | 1 |

These versions ship modern Swift features (primary associated types, Observation framework) that the runtime relies on. Downstream apps targeting older OS versions can lower these bounds and contribute compatibility patches.

## Quick start

```swift
import MosaicFlux

// 1. State + actions (in real Mosaic projects these are auto-generated from .mil)
struct CounterState: Equatable {
    var count: Int
}

struct Increment: MosaicAction {
    typealias State = CounterState
    func apply(to state: CounterState) -> CounterState {
        var s = state
        s.count += 1
        return s
    }
}

// 2. Store
let store = MosaicStore(initialState: CounterState(count: 0))

// 3. Subscribe to a slice
let unsubscribe = store.subscribe(
    selector: { $0.count },
    equality: ==
) { newCount in
    print("count is now \(newCount)")
}

// 4. Dispatch
store.dispatch(Increment())   // prints "count is now 1"
store.dispatch(Increment())   // prints "count is now 2"

// 5. Cleanup when done
unsubscribe()
```

## SwiftUI integration

v0.1.0 ships the imperative `subscribe()` API only. SwiftUI hosts wrap the store in an `@Observable` class (or pre-iOS-17, an `ObservableObject` with `@Published` properties) that calls `store.dispatch(...)` and bridges `subscribe(...)` callbacks to `@State` updates.

A v0.2.0 follow-up will ship a `MosaicObservable` wrapper that provides the `@Bindable`-friendly surface directly, eliminating that boilerplate.

## DevTools

`devToolsMiddleware()` emits the UI33-rewrite §8 cross-backend protocol. v0.1.0 logs each event to stdout in a human-readable form:

```
[mosaic-flux-devtools] 2026-06-01T22:14:15Z default/Increment
```

The same protocol is implemented across all `mosaic-flux-*` runtimes so the future Mosaic DevTools desktop app can attach uniformly. v0.2.0 will add a TCP socket transport on `localhost:9229` (matching Node's `--inspect` convention).

## Status

v0.1.0. Initial release.

- 26 XCTest unit tests, all passing
- Tested on macOS 14+ via Swift 6.3 toolchain

## Test

```bash
swift test
```

## Architecture

See `code/specs/UI33-rewrite-unified-architecture.md`, especially:

- §3 — strict-Flux invariants
- §5 — Command Pattern action classes
- §6 — runtime library API surface
- §8 — DevTools protocol
