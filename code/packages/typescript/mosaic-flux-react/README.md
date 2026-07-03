# @coding-adventures/mosaic-flux-react

Strict-Flux runtime for Mosaic UI's React emitter.

Implements the architecture specified in `code/specs/UI33-rewrite-unified-architecture.md`:

- **`MosaicAction<State>` interface** — the Command Pattern. Each action is a class with a constructor (payload) and an `apply(state) → state` method that expresses the state transform.
- **`MosaicStore<State>` class** — state container + dispatcher. Routes by calling `action.apply(state)` directly; no central reducer registry.
- **Fine-grained subscription** — selectors fire only when the selected slice changes (by `Object.is`, unless a custom equality is supplied). This is what makes per-keystroke dispatch cheap.
- **Middleware** — cross-cutting hook for logging, persistence, analytics.
- **`createSelector`** — memoised derived-state combinator (Reselect-shaped, zero-dep).
- **React integration** — `MosaicStoreProvider`, `useMosaicSelector`, `useMosaicDispatch`, `useMosaicStore` hooks via the `./react` subpath.
- **DevTools integration** — `devToolsMiddleware` publishes the UI33-rewrite §8 uniform event stream via `window.postMessage` (with `ws://localhost:9229` fallback).

## Why a Mosaic-owned runtime

Per UI33-rewrite §6.1: every Mosaic backend ships its own `mosaic-flux-*` library. This gives us:

1. Same architecture everywhere. A React dev moving to a Mosaic SwiftUI codebase sees the same Store / Action / Reducer shape.
2. Unified DevTools across all 7 backends because the runtime is uniform.
3. No external dependency risk — we don't break when Redux Toolkit / Zustand / TCA ships a breaking API change.
4. We can ship features no third-party would prioritise (e.g., cross-backend action replay).

This package is the React-specific runtime; sibling packages cover SwiftUI, Jetpack Compose, Flutter, WinUI XAML, Qt, HTML, and Web Components.

## Quick start

```typescript
import { MosaicStore, type MosaicAction } from "@coding-adventures/mosaic-flux-react";
import { MosaicStoreProvider, useMosaicSelector, useMosaicDispatch } from "@coding-adventures/mosaic-flux-react/react";

// 1. Define your state shape
interface GridState {
  count: number;
}

// 2. Define your actions as classes implementing MosaicAction<State>.
// (In a real Mosaic project these are auto-generated from .mil files;
// authors edit the apply() body between <mosaic:custom> markers.)
class Increment implements MosaicAction<GridState> {
  apply(state: GridState): GridState {
    return { ...state, count: state.count + 1 };
  }
}

// 3. Create a store
const store = new MosaicStore<GridState>({ initialState: { count: 0 } });

// 4. Provide it to your React tree
function App() {
  return (
    <MosaicStoreProvider store={store}>
      <Counter />
    </MosaicStoreProvider>
  );
}

// 5. Read state via selector hook + dispatch via dispatch hook
function Counter() {
  const count = useMosaicSelector((s: GridState) => s.count);
  const dispatch = useMosaicDispatch<GridState>();
  return (
    <button onClick={() => dispatch(new Increment())}>
      Count: {count}
    </button>
  );
}
```

## Architecture

See `code/specs/UI33-rewrite-unified-architecture.md` for the canonical specification, especially:

- §3 — strict-Flux invariants
- §5 — Command Pattern action classes
- §6 — runtime library API surface
- §8 — DevTools protocol

## Status

v0.1.0. Initial implementation of:

- MosaicAction interface + isMosaicAction type guard
- MosaicStore with fine-grained subscription + middleware
- composeMiddleware + loggerMiddleware
- createSelector
- devToolsMiddleware (postMessage + WebSocket transports)
- React integration: Provider, hooks for selector and dispatch

Not yet:

- Time-travel replay support on the runtime side (the DevTools desktop app drives this externally for v0.1.0; native replay support is a v0.2.0 follow-up).
- Multi-store coordination primitives (each store is standalone in v0.1.0).

## Testing

```bash
npm test
```

Coverage thresholds: 80% lines / branches / functions / statements (vitest config).
