// middleware.ts — middleware contract.

import type { MosaicAction } from "./action.js";

export type Middleware<State> = (
  action: MosaicAction<State>,
  prevState: State,
  nextState: State,
) => void;

/**
 * Compose middleware into one function. Errors in any middleware are
 * caught and logged; subsequent middleware still run.
 */
export function composeMiddleware<State>(
  middleware: ReadonlyArray<Middleware<State>>,
): Middleware<State> {
  if (middleware.length === 0) {
    return () => {
      /* no-op */
    };
  }
  if (middleware.length === 1) {
    return middleware[0]!;
  }
  return (action, prevState, nextState) => {
    for (const m of middleware) {
      try {
        m(action, prevState, nextState);
      } catch (err) {
        // eslint-disable-next-line no-console
        console.error("[mosaic-flux] middleware threw:", err);
      }
    }
  };
}

/**
 * Dev logger with shallow-state-diff output.
 */
export function loggerMiddleware<State>(): Middleware<State> {
  return (action, prevState, nextState) => {
    const name = action.constructor.name;
    const changedKeys = shallowChangedKeys(prevState, nextState);
    // eslint-disable-next-line no-console
    console.groupCollapsed(
      `[mosaic-flux] ${name} — changed: ${changedKeys.length > 0 ? changedKeys.join(", ") : "(none)"}`,
    );
    // eslint-disable-next-line no-console
    console.log("action  :", action);
    // eslint-disable-next-line no-console
    console.log("prev    :", prevState);
    // eslint-disable-next-line no-console
    console.log("next    :", nextState);
    // eslint-disable-next-line no-console
    console.groupEnd();
  };
}

function shallowChangedKeys<State>(
  prev: State,
  next: State,
): ReadonlyArray<string> {
  if (
    typeof prev !== "object" ||
    prev === null ||
    typeof next !== "object" ||
    next === null
  ) {
    return prev !== next ? ["<root>"] : [];
  }
  const changed: string[] = [];
  const prevObj = prev as Record<string, unknown>;
  const nextObj = next as Record<string, unknown>;
  const allKeys = new Set([...Object.keys(prevObj), ...Object.keys(nextObj)]);
  for (const k of allKeys) {
    if (prevObj[k] !== nextObj[k]) changed.push(k);
  }
  return changed;
}
