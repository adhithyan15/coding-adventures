/**
 * server.ts — Server: binds an Application to a TCP port and drives web-core.
 *
 * Design intent
 * ─────────────
 * `Server` is the bridge between the pure-TypeScript `Application` and the
 * Rust cdylib (`conduit_native_node`).  On construction it:
 *
 *   1. Loads the compiled `.node` addon with `require()`.
 *   2. Calls `native.newApp()` to create a Rust `NativeApp` object.
 *   3. Registers every route, filter, and handler from the `Application`.
 *   4. Calls `native.newServer(nativeApp, host, port)` to create a
 *      `NativeServer` object bound to the TCP socket.
 *
 * `serve()` blocks the current thread by calling
 * `nativeServer.serve()` which internally spawns a background Rust thread,
 * then keeps the Node.js event loop alive via `napi_ref_threadsafe_function`.
 * When the server stops (via `stop()` or a signal), the event loop drains
 * and the process exits normally.
 *
 * Threading model
 * ───────────────
 *
 *   ┌──────────────────────────────────────────────────────────────────┐
 *   │  Node.js V8 main thread                                          │
 *   │   - receives JS calls via TSFN queue                             │
 *   │   - executes route handlers, filters, not-found, error handler   │
 *   └─────────────────────────────┬────────────────────────────────────┘
 *                                 │  napi_threadsafe_function
 *                                 │  (one per registered callback)
 *   ┌─────────────────────────────▼────────────────────────────────────┐
 *   │  Rust background thread (spawned by serve())                     │
 *   │   - runs web-core WebServer (kqueue / epoll / IOCP)              │
 *   │   - for each request: posts to TSFN queue, blocks on condvar     │
 *   │   - when JS resolves: wakes up, sends HTTP response              │
 *   └──────────────────────────────────────────────────────────────────┘
 *
 * The N-API threadsafe function (TSFN) is what makes this thread-safe:
 * `napi_call_threadsafe_function` enqueues the call data; Node.js dequeues
 * and executes it on the main thread.
 *
 * The `Condvar` blocks the Rust background thread until JS resolves the
 * request, preventing the Rust side from reusing the request slot.
 *
 * Stopping
 * ────────
 * `server.stop()` calls `nativeServer.stop()` which signals the StopHandle
 * in the Rust background thread.  The background thread unwinds, releases
 * all TsFns (dropping their ref counts), and the event loop can exit.
 *
 * Example
 * ───────
 *   const app = new Application();
 *   app.get("/", () => html("<h1>Hello</h1>"));
 *
 *   const server = new Server(app, { host: "127.0.0.1", port: 3000 });
 *   server.serve();      // blocks until stop() or Ctrl-C
 */

import { createRequire } from "node:module";
import { Application } from "./application.js";
import { Request } from "./request.js";
import { HaltError } from "./halt_error.js";
import { type ResponseTuple } from "./handler_context.js";

// ── Native module loading ─────────────────────────────────────────────────────

/**
 * The native N-API interface exposed by the Rust cdylib.
 *
 * Types here mirror the N-API wrapper structs in conduit_native_node/src/lib.rs.
 * They are opaque handles — you cannot inspect their internals from JS.
 */
interface NativeApp {
  addRoute(method: string, pattern: string, handler: JsHandler): void;
  addBefore(handler: JsHandler): void;
  addAfter(handler: JsHandler): void;
  setNotFound(handler: JsHandler): void;
  setErrorHandler(handler: JsHandler): void;
  setSetting(key: string, value: string): void;
  getSetting(key: string): string | null;
}

interface NativeServer {
  serve(): void;
  serveBackground(): void;
  stop(): void;
  localPort(): number;
  running(): boolean;
}

interface NativeModule {
  newApp(): NativeApp;
  newServer(app: NativeApp, host: string, port: number, maxConn: number): NativeServer;
}

/**
 * JS-side handler type as seen by the Rust cdylib.
 *
 * The cdylib calls handlers with a flat env-map object and expects back:
 *   - undefined / null  → no override
 *   - [status, {headers}, body]  → concrete response
 *
 * Error handlers receive the same flat env map — the Rust side encodes the
 * error message as `env["conduit.error"]` so both handler types share the
 * same 1-arg TSFN callback signature.
 */
type JsHandler = (env: Record<string, string>) => ResponseTuple | null | undefined;

/**
 * loadNative — find and load the compiled .node addon.
 *
 * Build scripts copy the cdylib to the package root as
 * `conduit_native_node.node`.  We use `createRequire(import.meta.url)` so
 * this works in both CJS and ESM contexts.
 */
function loadNative(): NativeModule {
  // __dirname equivalent for ESM.
  const req = createRequire(import.meta.url);
  // The BUILD script copies the compiled .node file next to index.js.
  return req("../conduit_native_node.node") as NativeModule;
}

// ── Server options ────────────────────────────────────────────────────────────

export interface ServerOptions {
  /** TCP bind address (default: "127.0.0.1"). */
  host?: string;
  /** TCP port (default: 3000). Use 0 to let the OS pick a free port. */
  port?: number;
  /**
   * Maximum number of concurrent connections (default: 128).
   * Passed to the web-core WebApp as the connection pool limit.
   */
  maxConnections?: number;
}

// ── Server ────────────────────────────────────────────────────────────────────

/**
 * Server binds an Application to a TCP port using the Rust web-core engine.
 *
 * Usage:
 *   const server = new Server(app, { host: "127.0.0.1", port: 3000 });
 *   server.serve();
 */
export class Server {
  private readonly _native: NativeServer;

  constructor(app: Application, options: ServerOptions = {}) {
    const host = options.host ?? "127.0.0.1";
    const port = options.port ?? 3000;
    const maxConnections = options.maxConnections ?? 128;

    const native = loadNative();
    const nativeApp = native.newApp();

    // ── Register routes ───────────────────────────────────────────────────
    for (const route of app.routes) {
      const h = route.handler;
      nativeApp.addRoute(route.method, route.pattern, (env) =>
        callHandler(h, env),
      );
    }

    // ── Register before filters ───────────────────────────────────────────
    for (const f of app.beforeFilters) {
      nativeApp.addBefore((env) => callHandler(f, env));
    }

    // ── Register after filters ────────────────────────────────────────────
    for (const f of app.afterFilters) {
      nativeApp.addAfter((env) => callHandler(f, env));
    }

    // ── Not-found handler ─────────────────────────────────────────────────
    if (app.notFoundHandler != null) {
      const h = app.notFoundHandler;
      nativeApp.setNotFound((env) => callHandler(h, env));
    }

    // ── Error handler ─────────────────────────────────────────────────────
    //
    // The Rust side encodes the error message in `env["conduit.error"]` so the
    // TSFN callback has the same 1-arg signature as every other handler type.
    if (app.errorHandler != null) {
      const h = app.errorHandler;
      nativeApp.setErrorHandler((env) =>
        callErrorHandler(h, env, env["conduit.error"] ?? ""),
      );
    }

    // ── Settings ──────────────────────────────────────────────────────────
    for (const [key, value] of Object.entries(app.settings)) {
      nativeApp.setSetting(key, String(value));
    }

    this._native = native.newServer(nativeApp, host, port, maxConnections);
  }

  /**
   * serve — start the server and block until it stops.
   *
   * The Rust background thread runs web-core.  The Node.js event loop stays
   * alive because the TsFns are ref'd.  Returns only after stop() is called
   * or the process receives SIGINT/SIGTERM.
   */
  serve(): void {
    this._native.serve();
  }

  /**
   * serveBackground — start the server without blocking.
   *
   * Useful for tests: start in background, run test code, then call stop().
   * The event loop is kept alive by the ref'd TsFns — call stop() to allow
   * the process to exit.
   */
  serveBackground(): void {
    this._native.serveBackground();
  }

  /**
   * stop — signal the server to shut down.
   *
   * Safe to call from any thread.  The background serve thread will unwind,
   * all TsFns will be released, and the event loop can drain and exit.
   */
  stop(): void {
    this._native.stop();
  }

  /**
   * localPort — the port the server is listening on.
   *
   * Useful when port 0 was requested (OS picks a free port).
   */
  get localPort(): number {
    return this._native.localPort();
  }

  /**
   * running — true if the server background thread is active.
   */
  get running(): boolean {
    return this._native.running();
  }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/**
 * callHandler — invoke a user-provided handler and translate its output into
 * the `[status, headers, body]` tuple that the Rust cdylib expects.
 *
 * The function catches HaltError and converts it to a response tuple.
 * Other errors are re-thrown so the Rust side can detect them via
 * `napi_is_exception_pending` and route to the error handler.
 */
function callHandler(
  handler: (req: Request) => ResponseTuple | undefined | void,
  env: Record<string, string>,
): ResponseTuple | null {
  const req = new Request(env);
  try {
    const result = handler(req);
    // undefined / void → pass-through (before/after filters)
    if (result == null) return null;
    return result;
  } catch (err) {
    if (err instanceof HaltError) {
      return [err.status, Object.fromEntries(err.haltHeaderPairs), err.body];
    }
    // Re-throw so N-API exception machinery picks it up.
    throw err;
  }
}

/**
 * callErrorHandler — invoke the error handler with request + message.
 *
 * The error handler receives the Request and the stringified error message.
 * HaltError from within the error handler is also supported.
 */
function callErrorHandler(
  handler: (req: Request, msg: string) => ResponseTuple | undefined | void,
  env: Record<string, string>,
  errorMessage: string,
): ResponseTuple | null {
  const req = new Request(env);
  try {
    const result = handler(req, errorMessage);
    if (result == null) return null;
    return result;
  } catch (err) {
    if (err instanceof HaltError) {
      return [err.status, Object.fromEntries(err.haltHeaderPairs), err.body];
    }
    throw err;
  }
}
