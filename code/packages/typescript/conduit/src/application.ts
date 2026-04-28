/**
 * application.ts — Application: the route + filter registry.
 *
 * Design intent
 * ─────────────
 * `Application` is a pure TypeScript data store.  It holds:
 *
 *   - Route callbacks keyed by (method, pattern)
 *   - Before-request filters (run on every request, before route dispatch)
 *   - After-response filters (run after every route handler)
 *   - A custom not-found handler (fallback when no route matches)
 *   - A custom error handler (fallback when a handler throws)
 *   - A settings map (arbitrary key→value config)
 *
 * Nothing in this file touches the Rust cdylib.  `Server` is responsible for
 * feeding this data into the native `NativeApp` object.
 *
 * The class design mirrors Sinatra's Application class closely so that readers
 * familiar with the Ruby port can navigate both codebases side-by-side.
 *
 * Handler signature
 * ─────────────────
 * Every handler receives a single `Request` parameter and may return:
 *
 *   undefined                   — pass-through (before/after filters only)
 *   [status, headers, body]     — a concrete response tuple
 *   HaltError thrown            — caught by the framework (halt/redirect)
 *
 * Route patterns
 * ──────────────
 * Pattern syntax is the same as Sinatra (and web-core's Router):
 *   "/hello/:name"        — named segment capture → req.params.name
 *   "/files/*"            — wildcard (not named)
 *   "/health"             — exact match
 *
 * Example
 * ───────
 *   const app = new Application();
 *
 *   app.before((req) => {
 *     if (req.headers["x-shutdown"] === "1") halt(503, "Shutting down");
 *   });
 *
 *   app.get("/hello/:name", (req) => {
 *     return json({ message: `Hello ${req.params.name}` });
 *   });
 *
 *   app.notFound((req) => html(`Not Found: ${req.path}`, 404));
 */

import { type Request } from "./request.js";
import { type ResponseTuple } from "./handler_context.js";

/**
 * The signature every handler (route, filter, not-found, error) must match.
 *
 * - Route handlers: `(req) => ResponseTuple | undefined`
 * - Before/after filters: may return `undefined` to pass through, or a
 *   ResponseTuple to short-circuit (only the before filter short-circuits).
 * - Not-found handler: same as a route handler.
 * - Error handler: receives the request and the error message string.
 */
export type Handler = (req: Request) => ResponseTuple | undefined | void;

/** Handler for the error hook — receives request + error message. */
export type ErrorHandler = (
  req: Request,
  errorMessage: string,
) => ResponseTuple | undefined | void;

/** Internal route record stored per (method, pattern) registration. */
export interface RouteEntry {
  method: string;
  pattern: string;
  handler: Handler;
}

/**
 * Application is the route and filter registry.
 *
 * Instances are created by users and then handed to `Server` for binding.
 */
export class Application {
  /** Ordered list of before-request filters. */
  readonly beforeFilters: Handler[] = [];

  /** Ordered list of after-response filters. */
  readonly afterFilters: Handler[] = [];

  /** Registered routes in insertion order. */
  readonly routes: RouteEntry[] = [];

  /** Optional custom not-found handler. */
  notFoundHandler: Handler | null = null;

  /** Optional custom error handler. */
  errorHandler: ErrorHandler | null = null;

  /** Arbitrary settings map (e.g. { appName: "Conduit Hello", port: 3000 }). */
  readonly settings: Record<string, unknown> = {};

  // ── Route registration ────────────────────────────────────────────────────

  /** Register a GET route. */
  get(pattern: string, handler: Handler): this {
    return this._route("GET", pattern, handler);
  }

  /** Register a POST route. */
  post(pattern: string, handler: Handler): this {
    return this._route("POST", pattern, handler);
  }

  /** Register a PUT route. */
  put(pattern: string, handler: Handler): this {
    return this._route("PUT", pattern, handler);
  }

  /** Register a DELETE route. */
  delete(pattern: string, handler: Handler): this {
    return this._route("DELETE", pattern, handler);
  }

  /** Register a PATCH route. */
  patch(pattern: string, handler: Handler): this {
    return this._route("PATCH", pattern, handler);
  }

  private _route(method: string, pattern: string, handler: Handler): this {
    this.routes.push({ method, pattern, handler });
    return this;
  }

  // ── Filter registration ───────────────────────────────────────────────────

  /**
   * Register a before-request filter.
   *
   * Filters run in insertion order before route dispatch.  If a filter throws
   * HaltError (or calls `halt()`), processing stops and the halt response is
   * sent immediately.  If a filter returns a ResponseTuple, that response is
   * used and further processing is skipped.
   *
   * Use before filters for: auth, maintenance mode, rate limiting, logging.
   */
  before(handler: Handler): this {
    this.beforeFilters.push(handler);
    return this;
  }

  /**
   * Register an after-response filter.
   *
   * After filters run after the route handler but before the response is
   * sent.  They receive the request but cannot modify the response (yet).
   * Use after filters for: metrics, access logging, cleanup.
   */
  after(handler: Handler): this {
    this.afterFilters.push(handler);
    return this;
  }

  // ── Special handlers ──────────────────────────────────────────────────────

  /**
   * Register a custom not-found handler.
   *
   * Called when no route pattern matches the incoming request.
   * If not set, the framework returns a generic 404.
   *
   *   app.notFound((req) => html(`Not Found: ${req.path}`, 404));
   */
  notFound(handler: Handler): this {
    this.notFoundHandler = handler;
    return this;
  }

  /**
   * Register a custom error handler.
   *
   * Called when a route handler throws an unexpected exception (not a
   * HaltError).  Receives the original request and the error message.
   *
   *   app.onError((req, err) => json({ error: err }, 500));
   */
  onError(handler: ErrorHandler): this {
    this.errorHandler = handler;
    return this;
  }

  // ── Settings ──────────────────────────────────────────────────────────────

  /**
   * Store a setting value.
   *
   *   app.set("appName", "My App");
   *   app.set("debug", true);
   */
  set(key: string, value: unknown): this {
    this.settings[key] = value;
    return this;
  }

  /**
   * Read a setting value.
   *
   *   const name = app.get_setting("appName") as string;
   */
  getSetting(key: string): unknown {
    return this.settings[key];
  }
}
