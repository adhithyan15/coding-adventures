// devtools.ts — DevTools protocol middleware for browser hosts.

import type { MosaicAction } from "./action.js";
import type { Middleware } from "./middleware.js";

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

export interface DevToolsOptions {
  ws?: boolean;
  storeName?: string;
}

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
    try {
      window.postMessage(
        { source: "mosaic-flux-devtools", payload: event },
        "*",
      );
    } catch {
      /* dev-only sink; ignore non-cloneable payloads */
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
        /* fail silently in dev */
      });
    } catch {
      this.#ws = null;
    }
  }

  publish(event: DevToolsEvent<State> & { storeName: string }): void {
    if (this.#ws?.readyState === WebSocket.OPEN) {
      this.#sendNow(event);
    } else {
      this.#queue.push(event);
    }
  }

  #sendNow(event: DevToolsEvent<State> & { storeName: string }): void {
    try {
      this.#ws?.send(JSON.stringify(event));
    } catch {
      /* ignore */
    }
  }
}

class NoopSink<State> implements Sink<State> {
  publish(_event: DevToolsEvent<State> & { storeName: string }): void {
    /* no-op */
  }
}

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
