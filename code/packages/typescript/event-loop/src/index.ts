/**
 * @coding-adventures/event-loop
 *
 * A pluggable, generic event loop — the heartbeat of any interactive application.
 *
 * ## What is an event loop?
 *
 * An event loop is the outermost structure of any interactive program. It runs
 * forever (until told to stop), repeatedly asking "did anything happen?" and
 * dispatching whatever happened to registered handlers:
 *
 * ```
 * while running:
 *     collect events from all sources
 *     for each event:
 *         dispatch to handlers
 *         if any handler says "exit" → stop
 * ```
 *
 * ## Why generic and pluggable?
 *
 * A naïve loop hardcodes what events look like (KeyPress, MouseMove…). That
 * makes the loop untestable. A generic loop works at any layer — a game loop,
 * a GUI, a network server, and a test harness all share the same shape. The
 * event type `E` is defined by the caller.
 *
 * ## Quick start
 *
 * ```typescript
 * import { EventLoop, ControlFlow } from "@coding-adventures/event-loop";
 *
 * enum AppEvent { Tick, Quit }
 *
 * class TickSource implements EventSource<AppEvent> {
 *   constructor(private n: number) {}
 *   poll(): AppEvent[] {
 *     if (this.n-- > 0) return [AppEvent.Tick];
 *     return [AppEvent.Quit];
 *   }
 * }
 *
 * const loop = new EventLoop<AppEvent>();
 * loop.addSource(new TickSource(3));
 * loop.onEvent((e) => e === AppEvent.Quit ? ControlFlow.Exit : ControlFlow.Continue);
 * loop.run();
 * ```
 */

export const VERSION = "0.1.0";

// ════════════════════════════════════════════════════════════════════════════
// ControlFlow
// ════════════════════════════════════════════════════════════════════════════

/**
 * Signals whether the event loop should continue running or stop.
 *
 * Using an enum instead of `boolean` makes handler return values
 * self-documenting:
 *
 * ```typescript
 * return ControlFlow.Exit;    // intent is clear
 * return true;                // ambiguous — true means what, exactly?
 * ```
 *
 * String values are used so that debugging output is readable.
 */
export enum ControlFlow {
  /** Keep looping — there is more work to do. */
  Continue = "Continue",
  /** Stop the loop immediately after this event. */
  Exit = "Exit",
}

// ════════════════════════════════════════════════════════════════════════════
// EventSource
// ════════════════════════════════════════════════════════════════════════════

/**
 * Anything that can produce events for the loop to dispatch.
 *
 * The critical contract: **`poll()` must return immediately**. Return an
 * empty array if nothing is ready. Never block — blocking is the loop's job.
 *
 * This pull-based design keeps the loop in control of scheduling. Sources
 * that receive events from async operations should buffer into a local array
 * and expose a `poll()` that drains it.
 *
 * @example
 * ```typescript
 * class CountdownSource implements EventSource<number> {
 *   constructor(private count: number) {}
 *   poll(): number[] {
 *     if (this.count > 0) return [this.count--];
 *     return [];
 *   }
 * }
 * ```
 */
export interface EventSource<E> {
  /** Return all currently available events. Must not block. */
  poll(): E[];
}

// ════════════════════════════════════════════════════════════════════════════
// EventLoop
// ════════════════════════════════════════════════════════════════════════════

/**
 * A pluggable, generic event loop.
 *
 * `EventLoop<E>` is generic over the event type `E`. You define what events
 * look like; the loop handles collection and dispatch.
 *
 * Single-threaded by design. JavaScript is inherently single-threaded, so the
 * loop processes events synchronously. This is the same mental model as the
 * browser's event loop and Node.js's event loop — just made explicit.
 *
 * **Note on JavaScript vs native languages:** JavaScript has no true
 * `thread.yield()`. The `run()` method is a synchronous busy-loop and will
 * block the thread until the loop exits. In production JavaScript you would
 * use `setInterval`, Promises, or async generators. This synchronous version
 * exists to make the concept visible.
 */
export class EventLoop<E> {
  private sources: EventSource<E>[] = [];
  private handlers: ((event: E) => ControlFlow)[] = [];
  private stopped = false;

  /**
   * Register an event source. Sources are polled in registration order.
   */
  addSource(source: EventSource<E>): void {
    this.sources.push(source);
  }

  /**
   * Register an event handler.
   *
   * Handlers receive each event in registration order. If any handler
   * returns `ControlFlow.Exit`, the loop stops immediately — subsequent
   * handlers for the same event are not called.
   */
  onEvent(handler: (event: E) => ControlFlow): void {
    this.handlers.push(handler);
  }

  /**
   * Signal the loop to exit on the next iteration.
   */
  stop(): void {
    this.stopped = true;
  }

  /**
   * Start the event loop. Blocks until a handler returns `Exit` or `stop()` is called.
   *
   * Each iteration performs three phases:
   *
   * 1. **Collect** — call `poll()` on every source; append results to a local queue.
   * 2. **Dispatch** — deliver each queued event to every handler in order.
   *    Stop immediately if any handler returns `Exit`.
   * 3. **Idle** — if the queue was empty, the loop continues (no true yield
   *    in synchronous JS). In practice, sources should become exhausted quickly
   *    and a handler should return `Exit` to terminate the loop.
   */
  run(): void {
    this.stopped = false;

    while (!this.stopped) {
      // ── Phase 1: Collect ───────────────────────────────────────────────
      const queue: E[] = [];
      for (const source of this.sources) {
        queue.push(...source.poll());
      }

      // ── Phase 2: Dispatch ──────────────────────────────────────────────
      let shouldExit = false;
      outer: for (const event of queue) {
        for (const handler of this.handlers) {
          if (handler(event) === ControlFlow.Exit) {
            shouldExit = true;
            break outer;
          }
        }
      }
      if (shouldExit) return;

      // ── Phase 3: Idle ──────────────────────────────────────────────────
      // JavaScript has no thread.yield(). When the queue is empty and no
      // handler has exited, we rely on sources eventually becoming exhausted
      // and tests using finite sources that trigger Exit.
      //
      // For production use, replace this synchronous loop with:
      //   setInterval(() => { this.tick(); }, 0);
      // or an async generator pattern.
      //
      // In tests, sources always terminate so this never spins infinitely.
    }
  }
}
