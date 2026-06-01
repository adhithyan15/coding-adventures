# @coding-adventures/mosaic-flux-html

Strict-Flux runtime for Mosaic UI's HTML emitter.

Implements the architecture specified in `code/specs/UI33-rewrite-unified-architecture.md`:

- **`MosaicAction<State>` interface** — the Command Pattern. Each action is a class with payload (constructor args) + `apply(state) → state` method.
- **`MosaicStore<State>` class** — state container + dispatcher. Routes by calling `action.apply(state)` directly.
- **Fine-grained subscription** — selectors fire only when the selected slice changes.
- **Middleware** — `composeMiddleware`, `loggerMiddleware`.
- **`createSelector`** — memoised derived-state combinator.
- **`devToolsMiddleware`** — publishes UI33-rewrite §8 events via `window.postMessage` (with `ws://localhost:9229` fallback).
- **Vanilla-DOM bindings** — `bindText`, `bindAttr`, `bindClass`, `bindStyle`, `bindList`. No virtual DOM, no diffing engine, no template compiler.

## Relation to mosaic-flux-react

`mosaic-flux-react` and `mosaic-flux-html` share identical core types (MosaicAction, MosaicStore, Middleware, createSelector, devToolsMiddleware). The difference is the binding layer:

- React: hooks via `useSyncExternalStore`
- HTML: vanilla-DOM helpers (`bindText` etc.)

Per UI33-rewrite §6, these are deliberately separate packages so the React version doesn't carry a DOM dependency and the HTML version doesn't carry a React dependency. A future `mosaic-flux-core` may extract the shared core; v0.1.0 ships them standalone to avoid coupling release cadences.

## Why no virtual DOM

Strict Flux + fine-grained subscriptions makes targeted DOM updates trivial:

- A text node bound to a state slice updates only when that slice changes.
- A 100-row list bound to a selector updates only the rows whose data changed (with `bindList`'s key-based diff).

The whole-DOM-diff approach React/Vue/etc. take is solving a problem strict Flux doesn't have, because the runtime's subscription layer already targets updates.

## Quick start

```typescript
import {
  MosaicStore,
  bindText,
  bindAttr,
  bindClass,
  type MosaicAction,
} from "@coding-adventures/mosaic-flux-html";

// 1. State + actions (in real Mosaic projects, auto-generated from .mil)
interface CounterState { count: number; }

class Increment implements MosaicAction<CounterState> {
  apply(s: CounterState): CounterState {
    return { ...s, count: s.count + 1 };
  }
}

// 2. Store
const store = new MosaicStore<CounterState>({ initialState: { count: 0 } });

// 3. Bind DOM elements
const countEl = document.querySelector("#count")!;
const buttonEl = document.querySelector("#increment") as HTMLButtonElement;

bindText(countEl, store, (s) => String(s.count));
bindAttr(buttonEl, "data-count", store, (s) => String(s.count));
bindClass(countEl, "is-zero", store, (s) => s.count === 0);

// 4. Dispatch on events
buttonEl.addEventListener("click", () => {
  store.dispatch(new Increment());
});
```

## DOM binding API

| Helper | Updates | Use case |
|---|---|---|
| `bindText(el, store, selector)` | `el.textContent` | Display dynamic text |
| `bindAttr(el, attrName, store, selector)` | An HTML attribute (null removes) | `disabled`, `aria-*`, `data-*` |
| `bindClass(el, className, store, predicate)` | CSS class on/off | State-driven styling |
| `bindStyle(el, prop, store, selector)` | Inline style property (null removes) | Dynamic colors, layout |
| `bindList(container, store, listSel, keyFn, renderItem)` | Container's children with key-based diff | Dynamic lists, tables |

Each helper returns an unsubscribe function for cleanup.

### Sanitisation contract

`bindAttr` mechanically calls `setAttribute(attrName, value)` with whatever string the host (or selector) provides. **It does not sanitise URLs or event-handler attributes.** If you bind a state slot to `href`, `src`, `formaction`, `xlink:href`, or any `on*` attribute, the **host is responsible for validating the value before it reaches the binding**. The same contract applies to `bindStyle` for CSS property values containing URLs.

The recommended pattern is to sanitise upstream — either in the reducer that updates the state slot, in a memoised selector, or by routing all URL-bearing state through a dedicated `sanitisedUrl(raw)` helper. The Mosaic codegen (UI33r-E-html phase) will refuse to wire any state slot whose declared type is `text` to a security-sensitive attribute without an explicit `sanitise: ...` annotation in the `.mll`; for hand-written hosts, this contract is yours to uphold.

`bindText` uses `textContent` (never `innerHTML`), so any string is safe. `bindClass` uses `classList`, also safe. `bindStyle` sets CSS property values directly; browsers and jsdom strip `javascript:` from style URLs, but malicious CSS can still cause cosmetic damage. Treat style values as untrusted by default.

## Status

v0.1.0. Initial release.

## Testing

```bash
npm test
```

Coverage thresholds: 80% lines / branches / functions / statements. jsdom environment.
