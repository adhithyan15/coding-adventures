// index.ts — public surface of @coding-adventures/mosaic-flux-react.
//
// Per UI33-rewrite §6.1, this runtime exposes:
//
//   - MosaicAction<State>: the Command Pattern interface (every
//     action implements this).
//   - MosaicStore<State>: the state container and dispatcher.
//   - Middleware<State>: cross-cutting concern hook.
//   - createSelector: memoised derived-state combinator.
//   - devToolsMiddleware: opt-in connection to Mosaic DevTools.
//
// React-specific integration (hooks, provider) lives in the
// `./react` subpath import so non-React consumers don't pay the React
// dependency cost. v0.1.0 keeps that file lightweight — it expects
// React 18+ for `useSyncExternalStore`. See react.ts for details.

export type { MosaicAction } from "./action.js";
export { isMosaicAction } from "./action.js";

export type {
  Equality,
  MosaicStoreOptions,
  Subscriber,
} from "./store.js";
export { MosaicStore } from "./store.js";

export type { Middleware } from "./middleware.js";
export { composeMiddleware, loggerMiddleware } from "./middleware.js";

export { createSelector } from "./selector.js";

export type {
  ActionEvent,
  DevToolsEvent,
  DevToolsOptions,
  SubscriptionEvent,
} from "./devtools.js";
export { devToolsMiddleware } from "./devtools.js";

// React-specific exports are re-exported from a subpath to keep the
// core zero-dep. Consumers do:
//   import { MosaicStoreProvider, useMosaicSelector } from
//     "@coding-adventures/mosaic-flux-react/react";
// See ./react.ts for what's available.
