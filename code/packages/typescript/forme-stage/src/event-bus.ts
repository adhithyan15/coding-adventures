/**
 * Event bus — typed pub/sub between stages (FM01 §4.7).
 *
 * Intended for *coordination*, not data flow.  Data flows along the
 * pipeline's typed edges (Stage<In, Out>); events are for things like
 * "incremental rebuild invalidated ID X" or "preview server wants a
 * flush."  Using events to smuggle unstructured data between stages
 * is a smell and is caught by code review, not the type system.
 *
 * === In-memory implementation ===
 *
 * `inMemoryEventBus()` is a single-process bus suitable for tests and
 * for the v0 orchestrator (which runs everything in one process).
 * Subscribers receive events synchronously in registration order; if
 * a handler throws, the error is swallowed and other subscribers still
 * see the event — a bus-wide error would break the coordination
 * guarantees consumers rely on.
 *
 * The `unsubscribe` function returned by `on` is idempotent: calling
 * it twice is a silent no-op.
 */

import type { JsonValue } from "@coding-adventures/forme-types";

/** Event bus contract. */
export interface EventBus {
  emit(event: string, payload: JsonValue): void;
  on(event: string, handler: (payload: JsonValue) => void): () => void;
}

class InMemoryEventBusImpl implements EventBus {
  private readonly handlers = new Map<string, Array<(payload: JsonValue) => void>>();

  emit(event: string, payload: JsonValue): void {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) return;
    // Iterate over a snapshot so a handler that unsubscribes itself
    // doesn't shift indices mid-loop.
    for (const handler of list.slice()) {
      try { handler(payload); } catch { /* swallow per module header */ }
    }
  }

  on(event: string, handler: (payload: JsonValue) => void): () => void {
    let list = this.handlers.get(event);
    if (!list) {
      list = [];
      this.handlers.set(event, list);
    }
    list.push(handler);
    let active = true;
    return () => {
      if (!active) return;
      active = false;
      const current = this.handlers.get(event);
      if (!current) return;
      const idx = current.indexOf(handler);
      if (idx !== -1) current.splice(idx, 1);
      if (current.length === 0) this.handlers.delete(event);
    };
  }
}

/** Build a fresh in-memory event bus. */
export function inMemoryEventBus(): EventBus {
  return new InMemoryEventBusImpl();
}
