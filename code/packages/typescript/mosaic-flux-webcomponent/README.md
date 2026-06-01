# @coding-adventures/mosaic-flux-webcomponent

Strict-Flux runtime for Mosaic UI's Web Component emitter.

Same core types as `mosaic-flux-html` (`MosaicAction`, `MosaicStore`, middleware, `createSelector`, `devToolsMiddleware`, DOM bindings) plus a `MosaicHostElement` base class and `defineMosaicElement` helper for lifecycle-managed custom elements.

Implements the architecture specified in `code/specs/UI33-rewrite-unified-architecture.md`.

## Why a separate package from mosaic-flux-html

Per UI33-rewrite §6.1, each runtime ships standalone to avoid coupling release cadences. v0.1.0 duplicates the shared core; a future `mosaic-flux-core` may extract it once the design has settled and all backends are shipping.

## The Web-Component-specific surface

### `MosaicHostElement<State>` base class

Subclass it to write custom elements that bind to a `MosaicStore`. The base class handles the lifecycle so you don't have to remember binding cleanup:

```typescript
import {
  MosaicHostElement,
  defineMosaicElement,
  bindText,
  type MosaicStore,
  type MosaicAction,
} from "@coding-adventures/mosaic-flux-webcomponent";

interface CounterState { count: number; }

class Increment implements MosaicAction<CounterState> {
  apply(s: CounterState): CounterState {
    return { ...s, count: s.count + 1 };
  }
}

class CounterDisplay extends MosaicHostElement<CounterState> {
  private span: HTMLSpanElement;

  constructor() {
    super();
    const shadow = this.attachShadowIfNeeded();
    this.span = document.createElement("span");
    shadow.appendChild(this.span);
  }

  protected bindStore(store: MosaicStore<CounterState>): void {
    // track() registers the unsubscribe for automatic cleanup on
    // disconnectedCallback.
    this.track(bindText(this.span, store, (s) => String(s.count)));
  }
}

defineMosaicElement("counter-display", CounterDisplay);

// Usage:
const counter = document.createElement("counter-display") as CounterDisplay;
counter.store = myStore;
document.body.appendChild(counter);

counter.dispatch(new Increment()); // shortcut, throws if no store bound
```

Lifecycle:

| Event | What happens |
|---|---|
| `constructor` | Subclass typically attaches shadow DOM and assembles child nodes |
| Setting `element.store = s` | Stores the reference; if already connected, disposes old bindings and calls `bindStore(s)` |
| `connectedCallback` | If a store is set, calls `bindStore(store)` |
| `bindStore(store)` | Subclass override; register bindings via `this.track(unsubscribe)` |
| `disconnectedCallback` | Calls every tracked unsubscribe; you can move the element and reconnect to re-bind |

The `track()` mechanism eliminates a common memory-leak class: bindings registered with the store would otherwise outlive the element after it's removed from the DOM. With `MosaicHostElement`, every binding is automatically disposed on disconnect.

### `defineMosaicElement(tagName, elementClass, options?)`

Thin wrapper around `customElements.define`. Idempotent — calling twice with the same tag is a no-op (the first definition wins) instead of throwing. This makes module reloads / HMR safe.

```typescript
defineMosaicElement("my-counter", MyCounter);
```

## Core surface (same as mosaic-flux-html)

| Export | Purpose |
|---|---|
| `MosaicAction<State>` | Command Pattern interface |
| `MosaicStore<State>` | State container + dispatcher |
| `composeMiddleware` / `loggerMiddleware` | Middleware utilities |
| `createSelector` | Memoised derived state |
| `devToolsMiddleware` | DevTools protocol (postMessage + ws fallback) |
| `bindText` / `bindAttr` / `bindClass` / `bindStyle` / `bindList` | Vanilla-DOM binding helpers (work inside shadow roots) |

See `mosaic-flux-html`'s README for the core API details. The DOM binding helpers work identically inside an open shadow root.

### Sanitisation contract

Same as `mosaic-flux-html`: `bindAttr` and `bindStyle` do not sanitise URLs or event-handler attributes. The host (or the Mosaic codegen) is responsible for validating these before they reach the binding. `bindText` (`textContent`) and `bindClass` (`classList`) are always safe.

## Status

v0.1.0. Initial release.

## Testing

```bash
npm test
```

Coverage thresholds: 80% lines/branches/functions/statements. jsdom environment provides `customElements`, shadow DOM, and HTMLElement.
