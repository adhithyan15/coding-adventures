/**
 * request.ts — Request: the read-only view of an incoming HTTP request.
 *
 * Design intent
 * ─────────────
 * The Rust cdylib (conduit_native_node) passes each incoming HTTP request to
 * a JavaScript handler as a plain object called the "env map" — a flat
 * string-to-string dictionary mirroring the CGI / Rack / PSGI environment
 * convention:
 *
 *   {
 *     "REQUEST_METHOD":        "GET",
 *     "PATH_INFO":             "/hello/world",
 *     "QUERY_STRING":          "foo=bar",
 *     "conduit.route_params":  '{"name":"world"}',   // JSON
 *     "conduit.query_params":  '{"foo":"bar"}',       // JSON
 *     "conduit.headers":       '{"content-type":"application/json"}', // JSON
 *     "conduit.body":          '{"ping":"pong"}',
 *     "conduit.content_type":  "application/json",
 *     "conduit.content_length":"14",
 *   }
 *
 * The Request class wraps this map and provides ergonomic accessors.
 *
 * Parsing is lazy — `json()` and `form()` only parse when called, and the
 * parsed result is cached so repeated calls are cheap.
 *
 * Example
 * ───────
 *   app.get("/search", (req) => {
 *     const q = req.queryParams["q"] ?? "";
 *     return json({ results: search(q) });
 *   });
 *
 *   app.post("/users", (req) => {
 *     const body = req.json<{ name: string }>();
 *     return json({ id: createUser(body.name) }, 201);
 *   });
 */

import { HaltError } from "./halt_error.js";

/** The raw string-to-string env map produced by the Rust cdylib. */
export type EnvMap = Record<string, string>;

/**
 * Request wraps the CGI-style env map that the Rust cdylib passes to every
 * handler.  All properties are read-only.
 */
export class Request {
  /** The raw env map — useful for debugging or accessing custom headers. */
  readonly env: EnvMap;

  // ── Core fields ──────────────────────────────────────────────────────────

  /** HTTP method, always upper-case: "GET", "POST", "PUT", "DELETE", "PATCH". */
  readonly method: string;

  /**
   * Request path without query string, e.g. "/hello/world".
   *
   * Named params are NOT substituted here — the raw pattern path is preserved
   * so the route pattern is still visible.  Use `params` to read named
   * captures.
   */
  readonly path: string;

  /** Raw query string without the leading "?", or "" if absent. */
  readonly queryString: string;

  // ── Parameter maps ────────────────────────────────────────────────────────

  /**
   * Named route captures, e.g. "/hello/:name" → `{ name: "world" }`.
   * Empty object when no named captures exist.
   */
  readonly params: Record<string, string>;

  /**
   * Parsed query-string parameters, e.g. "?q=hello&n=5" → `{ q: "hello", n: "5" }`.
   * All values remain strings — use `parseInt` / `parseFloat` as needed.
   */
  readonly queryParams: Record<string, string>;

  // ── Headers ───────────────────────────────────────────────────────────────

  /**
   * Request headers as a lowercased-name-keyed object.
   *   `{ "content-type": "application/json", "authorization": "Bearer ..." }`
   *
   * Header names are always lower-case to simplify look-ups:
   *   `req.headers["content-type"]` — always works regardless of case on wire.
   */
  readonly headers: Record<string, string>;

  // ── Body ──────────────────────────────────────────────────────────────────

  /** Raw request body as a string, or "" if absent. */
  readonly bodyText: string;

  /** Content-Type header value, or "" if absent. */
  readonly contentType: string;

  /** Content-Length as a number, or 0 if absent / non-numeric. */
  readonly contentLength: number;

  // ── Lazy-parsed caches ────────────────────────────────────────────────────
  private _jsonCache: unknown = undefined;
  private _jsonParsed = false;
  private _formCache: Record<string, string> | undefined = undefined;

  constructor(env: EnvMap) {
    this.env = env;

    // ── Core fields ──────────────────────────────────────────────────────
    this.method = env["REQUEST_METHOD"] ?? "GET";
    this.path = env["PATH_INFO"] ?? "/";
    this.queryString = env["QUERY_STRING"] ?? "";

    // ── Named route params ────────────────────────────────────────────────
    // Encoded as JSON by the Rust side, e.g. '{"name":"world"}'.
    const routeParamsRaw = env["conduit.route_params"];
    this.params =
      routeParamsRaw != null ? safeParseObject(routeParamsRaw) : {};

    // ── Query params ──────────────────────────────────────────────────────
    const queryParamsRaw = env["conduit.query_params"];
    this.queryParams =
      queryParamsRaw != null
        ? safeParseObject(queryParamsRaw)
        : parseQueryString(this.queryString);

    // ── Headers ───────────────────────────────────────────────────────────
    const headersRaw = env["conduit.headers"];
    this.headers =
      headersRaw != null ? safeParseObject(headersRaw) : {};

    // ── Body ──────────────────────────────────────────────────────────────
    this.bodyText = env["conduit.body"] ?? "";
    this.contentType = env["conduit.content_type"] ?? "";
    this.contentLength = parseInt(env["conduit.content_length"] ?? "0", 10) || 0;
  }

  // ── Convenience methods ───────────────────────────────────────────────────

  /**
   * Parse the request body as JSON.  Returns the parsed value, caching the
   * result so subsequent calls are cheap.
   *
   * Throws a SyntaxError if the body is not valid JSON.
   * Throws a HaltError(413) if the body exceeds MAX_JSON_BODY_BYTES (10 MiB).
   *
   * Security note: `JSON.parse` is safe from prototype pollution in Node.js
   * (a key named `"__proto__"` creates an own property, it does NOT modify
   * `Object.prototype`).  However, callers should always validate the shape of
   * the parsed value before using it — the `T` type parameter is erased at
   * runtime and does not guarantee structure.
   *
   * Type parameter `T` lets callers narrow the return type:
   *   const body = req.json<{ name: string }>();
   */
  json<T = unknown>(): T {
    if (!this._jsonParsed) {
      // Guard against CPU-exhaustion DoS from huge deeply-nested JSON payloads.
      if (this.bodyText.length > Request.MAX_JSON_BODY_BYTES) {
        throw new HaltError(413, "Payload Too Large");
      }
      this._jsonCache = JSON.parse(this.bodyText);
      this._jsonParsed = true;
    }
    return this._jsonCache as T;
  }

  /**
   * Maximum number of bytes accepted by `json()`.
   *
   * Set to 10 MiB by default.  Override at the application level by
   * reassigning: `Request.MAX_JSON_BODY_BYTES = 1 * 1024 * 1024;` (1 MiB).
   */
  static MAX_JSON_BODY_BYTES = 10 * 1024 * 1024; // 10 MiB

  /**
   * Parse the request body as `application/x-www-form-urlencoded`.
   *
   * Returns a string-to-string record.  Values are URL-decoded.
   *
   *   // POST body: "name=Alice&role=admin"
   *   req.form()  // → { name: "Alice", role: "admin" }
   */
  form(): Record<string, string> {
    if (this._formCache == null) {
      this._formCache = parseQueryString(this.bodyText);
    }
    return this._formCache;
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/**
 * safeParseObject — JSON.parse that returns an empty object on error.
 *
 * The Rust side always encodes these maps as valid JSON, but we guard
 * defensively so a malformed value doesn't crash the process.
 */
function safeParseObject(raw: string): Record<string, string> {
  try {
    const val = JSON.parse(raw);
    if (val != null && typeof val === "object" && !Array.isArray(val)) {
      return val as Record<string, string>;
    }
  } catch {
    // fall through to default
  }
  return {};
}

/**
 * parseQueryString — parse a URL-encoded query string into a record.
 *
 *   parseQueryString("q=hello+world&n=5")
 *   // → { q: "hello world", n: "5" }
 *
 * Both `+` (form-encoding) and `%20` are decoded as spaces.
 */
function parseQueryString(qs: string): Record<string, string> {
  const out: Record<string, string> = {};
  if (!qs) return out;

  for (const pair of qs.split("&")) {
    const eqIdx = pair.indexOf("=");
    if (eqIdx === -1) {
      // Key with no value — treat as empty string.
      const key = decodeURIComponent(pair.replace(/\+/g, " "));
      if (key) out[key] = "";
    } else {
      const key = decodeURIComponent(pair.slice(0, eqIdx).replace(/\+/g, " "));
      const val = decodeURIComponent(pair.slice(eqIdx + 1).replace(/\+/g, " "));
      if (key) out[key] = val;
    }
  }

  return out;
}
