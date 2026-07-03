// index.ts — public surface of @coding-adventures/mosaic-flux-webcomponent.
//
// Re-exports the same core types as mosaic-flux-html (so consumers
// see one consistent API surface) plus the WebComponent-specific
// MosaicHostElement base class and defineMosaicElement helper for
// custom-element lifecycle integration.

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

// WebComponent-specific surface
export type { Unsubscribe } from "./element.js";
export { MosaicHostElement, defineMosaicElement } from "./element.js";
