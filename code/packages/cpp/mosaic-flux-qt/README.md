# mosaic-flux-qt

Strict-Flux runtime for the Mosaic UI Qt emitter (C++17, header-only).

This is the C++ counterpart to `@coding-adventures/mosaic-flux-react`,
`mosaic-flux-swiftui`, `mosaic-flux-compose`, `mosaic-flux-flutter`,
and `Mosaic.Flux`. Every Mosaic backend ships its own runtime so
application code never depends on a third-party Flux library — TCA,
Bloc, Fluxor, etc. are deliberately avoided. The shape of the API
is identical across backends so cross-platform Mosaic projects only
learn it once.

## What's in the box

| Header | Role |
| --- | --- |
| `MosaicFlux/Action.h` | command-pattern action base with virtual `apply(state) → state` |
| `MosaicFlux/Store.h` | dispatcher + state holder, thread-safe via `std::mutex`, RAII subscription handles |
| `MosaicFlux/Middleware.h` | logger / dev-tools hook between every dispatch; `composeMiddleware`, `loggerMiddleware` |
| `MosaicFlux/Selector.h` | memoised projection helpers (1–3 input slices) |
| `MosaicFlux/DevTools.h` | local channel hook for inspecting actions and state diffs (no-op in v0.1.0) |

## Why a separate C++ runtime?

Mosaic UI takes the Redux contract literally:

1. The view layer is **read-only**. Nothing in a `.msl` file mutates
   state directly.
2. Every change goes through `store.dispatch`. Even a single
   keystroke in a VisiCalc cell rounds back through the store before
   the view re-renders.
3. Actions are **classes** (subclasses of `MosaicAction<State>`)
   with an explicit `apply(state)` method. If a backend needs
   special handling for an action — for example, debouncing a Qt
   signal — you edit the action class, not a hidden reducer table.

That last bullet is why this exists rather than reusing
`QStateMachine` or any external Flux library: Mosaic wants the
generated action surface to be exactly the thing the user edits.

## Usage

```cpp
#include "MosaicFlux/Store.h"

struct AppState {
    int count = 0;
    bool operator==(const AppState& o) const { return count == o.count; }
};

struct Increment final : MosaicFlux::MosaicAction<AppState> {
    AppState apply(const AppState& s) const override {
        return AppState{ s.count + 1 };
    }
};

int main() {
    MosaicFlux::MosaicStore<AppState> store(AppState{});

    auto sub = store.subscribe<int>(
        [](const AppState& s) { return s.count; },
        [](const int& c) { std::cout << "count = " << c << '\n'; });

    store.dispatch(Increment{});  // → "count = 1"
}  // sub goes out of scope → automatic unsubscribe
```

## Build + test

```bash
cmake -B build -S .
cmake --build build
ctest --test-dir build --output-on-failure
```

Or run the BUILD script which does all three.

## Q_OBJECT and v0.2.0

Templates and `Q_OBJECT` don't mix cleanly — `moc` can't process
template classes — so v0.1.0 keeps the store as a plain template
without `Q_OBJECT` inheritance. The v0.2.0 release will add a thin
shim that wraps a concrete-state store in a `QObject` so it can be
exposed to QML, but the core stays here, header-only and
dependency-free.

## Status

`v0.1.0` — core dispatcher, subscription, selector, middleware,
and dev-tools surface. 29 zero-dependency unit tests, header-only.
Qt-specific glue (QObject shim, QML registration, dev-tools wire
to `qDebug()`) lands in `v0.2.0`.
