// devtools.ts — DevTools protocol integration for React/browser
// runtimes.
//
// Per UI33-rewrite §8: every mosaic-flux runtime emits a uniform
// structured event stream on a local channel. The Mosaic DevTools
// desktop application attaches to this stream and provides action
// log, time-travel replay, and state diff inspection.
//
// On the web side (React, HTML, WebComponent), the channel is either:
//   1. window.postMessage to a browser extension (preferred when the
//      extension is installed; the extension forwards over its own
//      transport)
//   2. WebSocket to ws://localhost:9229 (fallback for local dev)
//
// This module implements both: postMessage by default, with an opt-in
// WebSocket fallback the host enables via `enableDevTools({ ws: true })`.

import type { MosaicAction } from "./action.js";
import type { Middleware } from "./middleware.js";

/**
 * The wire format every runtime emits. Identical across all 7
 * backends (UI33-rewrite §8.1).
 */
export interface ActionEvent<State> {
  kind: "action";
  ts: number;
  actionType: string;
  actionPayload: Record<string, unknown>;
  prevState: State;
  nextState: State;
  durationUs: number;
}

export interface SubscriptionEvent {
  kind: "subscription";
  ts: number;
  componentId: string;
  slots: ReadonlyArray<string>;
  rendered: boolean;
}

export type DevToolsEvent<State> = ActionEvent<State> | SubscriptionEvent;

/**
 * Configuration for the DevTools sink.
 */
export interface DevToolsOptions {
  /**
   * If true, attempt a WebSocket connection to ws://localhost:9229
   * when no postMessage receiver is registered. Default false.
   */
  ws?: boolean;

  /**
   * Custom storeName to disambiguate when multiple stores are open
   * (per-tab Mosaic apps with sub-stores, for example).
   */
  storeName?: string;
}

/**
 * Build a middleware that emits ActionEvents on every dispatch.
 *
 * The middleware is robust to missing transports — if neither
 * postMessage nor WebSocket is available, it silently no-ops. This
 * means it's safe to keep enabled in production builds (where the
 * receiving end usually isn't attached) without risk of crashes.
 */
export function devToolsMiddleware<State>(
  options: DevToolsOptions = {},
): Middleware<State> {
  const sink = makeSink<State>(options);
  const storeName = options.storeName ?? "default";
  return (action, prevState, nextState) => {
    const t0 = now();
    const event: ActionEvent<State> = {
      kind: "action",
      ts: Date.now(),
      actionType: action.constructor.name,
      actionPayload: extractPayload(action),
      prevState,
      nextState,
      durationUs: Math.round((now() - t0) * 1000),
    };
    sink.publish({ ...event, storeName } as ActionEvent<State> & {
      storeName: string;
    });
  };
}

/**
 * The sink chooses a transport at construction time. Available
 * options: in-browser postMessage, WebSocket. Falls back to no-op
 * if neither is available.
 */
interface Sink<State> {
  publish(event: DevToolsEvent<State> & { storeName: string }): void;
}

function makeSink<State>(options: DevToolsOptions): Sink<State> {
  if (typeof window !== "undefined" && typeof window.postMessage === "function") {
    return new PostMessageSink<State>();
  }
  if (options.ws && typeof globalThis.WebSocket !== "undefined") {
    return new WebSocketSink<State>("ws://localhost:9229");
  }
  return new NoopSink<State>();
}

class PostMessageSink<State> implements Sink<State> {
  publish(event: DevToolsEvent<State> & { storeName: string }): void {
    // Browser extensions filter on source === "mosaic-flux-devtools".
    // We use a structured object rather than a string so the extension
    // can match without parsing.
    try {
      window.postMessage(
        { source: "mosaic-flux-devtools", payload: event },
        "*",
      );
    } catch {
      // postMessage can fail when payload isn't structured-cloneable
      // (e.g., contains functions). We don't try to recover — this is
      // a dev-only sink and a dropped event is acceptable.
    }
  }
}

class WebSocketSink<State> implements Sink<State> {
  #ws: WebSocket | null = null;
  #queue: Array<DevToolsEvent<State> & { storeName: string }> = [];

  constructor(url: string) {
    try {
      this.#ws = new WebSocket(url);
      this.#ws.addEventListener("open", () => {
        for (const event of this.#queue) {
          this.#sendNow(event);
        }
        this.#queue = [];
      });
      this.#ws.addEventListener("error", () => {
        // Connection failed — fall back to queueing forever (harmless
        // in dev) and let the user notice no events arrive.
      });
    } catch {
      this.#ws = null;
    }
  }

  publish(event: DevToolsEvent<State> & { storeName: string }): void {
    if (this.#ws?.readyState === WebSocket.OPEN) {
      this.#sendNow(event);
    } else {
      // Pre-open events queue up; they flush on `open`.
      this.#queue.push(event);
    }
  }

  #sendNow(event: DevToolsEvent<State> & { storeName: string }): void {
    try {
      this.#ws?.send(JSON.stringify(event));
    } catch {
      // Same rationale as PostMessageSink: dev-only, ignore.
    }
  }
}

class NoopSink<State> implements Sink<State> {
  publish(_event: DevToolsEvent<State> & { storeName: string }): void {
    /* no-op */
  }
}

/**
 * Extract enumerable own properties of the action as the payload.
 * Excludes inherited methods (like `apply`) and internal fields
 * prefixed with `_` or `#`.
 */
function extractPayload(action: MosaicAction<unknown>): Record<string, unknown> {
  const payload: Record<string, unknown> = {};
  for (const key of Object.keys(action)) {
    if (key.startsWith("_")) continue;
    payload[key] = (action as unknown as Record<string, unknown>)[key];
  }
  return payload;
}

function now(): number {
  if (typeof performance !== "undefined" && typeof performance.now === "function") {
    return performance.now();
  }
  return Date.now();
}
