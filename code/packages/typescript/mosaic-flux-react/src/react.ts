// react.ts — React-specific hooks and provider for mosaic-flux-react.
//
// This file is the ONLY place that imports React. Non-React consumers
// (e.g., a Node test, an HTML emitter that shares the store
// implementation) can import everything else from the package root
// without paying the React dependency cost.
//
// We target React 18+ for `useSyncExternalStore`, which is React's
// blessed mechanism for subscribing external stores to the render
// cycle. It handles concurrent rendering, tearing prevention, and
// strict mode double-invocation correctly — manually implementing
// these is a tar pit, so we delegate.
//
// API surface:
//
//   - <MosaicStoreProvider store={...}>: provide a store via Context.
//   - useMosaicStore(): get the store from context.
//   - useMosaicSelector(selector): subscribe to a slice; re-render on
//     change.
//   - useMosaicDispatch(): get a dispatch function bound to the store.

import {
  createContext,
  createElement,
  useContext,
  useMemo,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import type { MosaicAction } from "./action.js";
import type { Equality } from "./store.js";
import type { MosaicStore } from "./store.js";

/**
 * Context that carries the store down the component tree. We do NOT
 * inline a default value because using the hook outside a provider
 * is an error worth surfacing loudly — null forces the hooks to
 * throw with a clear message.
 */
const MosaicStoreContext = createContext<MosaicStore<unknown> | null>(null);

export interface MosaicStoreProviderProps<State> {
  store: MosaicStore<State>;
  children: ReactNode;
}

/**
 * Provide a store via Context. Wrap your App in this and child
 * components can use `useMosaicSelector` / `useMosaicDispatch`.
 *
 * One provider per store. Multi-store apps nest providers — children
 * see the nearest one. The selectors and dispatches you get back are
 * typed to whatever the nearest provider holds.
 */
export function MosaicStoreProvider<State>(
  props: MosaicStoreProviderProps<State>,
): ReactNode {
  // Cast to unknown for the Context bridge — the consumer hooks
  // re-narrow via their selector signature. This is the standard
  // pattern when Context is generic at construction but invariant
  // at storage.
  return createElement(
    MosaicStoreContext.Provider,
    { value: props.store as unknown as MosaicStore<unknown> },
    props.children,
  );
}

/**
 * Retrieve the store from context. Throws if no provider is above.
 *
 * Most components should use `useMosaicSelector` (state) or
 * `useMosaicDispatch` (actions); this hook is for the rare case
 * where you need imperative access to the store itself.
 */
export function useMosaicStore<State>(): MosaicStore<State> {
  const store = useContext(MosaicStoreContext) as MosaicStore<State> | null;
  if (store === null) {
    throw new Error(
      "useMosaicStore must be used inside a <MosaicStoreProvider>",
    );
  }
  return store;
}

/**
 * Subscribe to a slice of state. The hook re-renders the component
 * whenever the selected slice changes by `equality` (default
 * `Object.is`, matching React's hook conventions).
 *
 * The selector function should be pure and stable — i.e., the same
 * input should always produce the same output, and ideally the
 * function reference itself shouldn't change between renders.
 * Inline arrow functions are fine; `useSyncExternalStore` handles
 * the re-subscription dance.
 */
export function useMosaicSelector<State, T>(
  selector: (state: State) => T,
  equality?: Equality<T>,
): T {
  const store = useMosaicStore<State>();

  // Wrap subscribe so React's signature `(onChange: () => void) => () => void`
  // works with our `(selector, callback) => unsubscribe` API. We
  // memoise on selector + equality so React doesn't tear down and
  // recreate the subscription each render.
  const subscribe = useMemo(
    () => (onChange: () => void) =>
      store.subscribe(selector, () => onChange(), equality),
    [store, selector, equality],
  );

  const getSnapshot = (): T => store.select(selector);

  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}

/**
 * Get a dispatch function bound to the current store. The returned
 * function is stable across renders (per store) so it's safe to use
 * as a dependency in `useEffect` etc.
 */
export function useMosaicDispatch<State>(): <A extends MosaicAction<State>>(
  action: A,
) => void {
  const store = useMosaicStore<State>();
  return useMemo(
    () =>
      <A extends MosaicAction<State>>(action: A): void =>
        store.dispatch(action),
    [store],
  );
}
