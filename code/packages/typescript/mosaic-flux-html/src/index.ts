// index.ts — public surface of @coding-adventures/mosaic-flux-html.
//
// Per UI33-rewrite §6.1, this runtime exposes:
//
//   - MosaicAction<State>: Command Pattern interface
//   - MosaicStore<State>: state container and dispatcher
//   - Middleware<State>: cross-cutting hook
//   - createSelector: memoised derived state
//   - devToolsMiddleware: DevTools protocol integration
//   - bindText / bindAttr / bindClass / bindStyle / bindList:
//     vanilla-DOM bindings replacing React hooks

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

export {
  bindAttr,
  bindClass,
  bindList,
  bindStyle,
  bindText,
} from "./dom.js";
