/**
 * halt_error.ts — HaltError: the escape hatch for short-circuiting a request.
 *
 * Design intent
 * ─────────────
 * In Sinatra, `halt` is a top-level helper that throws an exception caught by
 * the framework before the normal response path.  We replicate this with a
 * plain JavaScript Error subclass carrying three extra fields:
 *
 *   status  — HTTP status code (default 200)
 *   body    — response body string (default "")
 *   headers — extra response headers as a flat [name, value][] pair list
 *
 * The `__conduit_halt` flag is what the Rust cdylib looks for to distinguish a
 * halt from an unhandled exception:
 *
 *   // Rust side (conduit_native_node/src/lib.rs)
 *   let is_halt = read_bool_prop(env, exc, "__conduit_halt");
 *
 * Throwing vs returning
 * ─────────────────────
 * Route handlers may *return* a response tuple directly, but halt() and
 * redirect() use `throw` to unwind the call stack immediately — the same
 * mental model as Ruby `raise` / Python `raise`.
 *
 * Example
 * ───────
 *   import { halt } from "coding-adventures-conduit";
 *
 *   app.before((req) => {
 *     if (req.headers["x-maintenance"] === "1") {
 *       halt(503, "Under maintenance");
 *     }
 *   });
 */

/**
 * HaltError is thrown by halt() and redirect() to short-circuit a handler.
 *
 * The Rust side reads `__conduit_halt`, `status`, `body`, and
 * `haltHeaderPairs` off the caught exception object.
 */
export class HaltError extends Error {
  /** Sentinel recognised by the Rust cdylib. */
  readonly __conduit_halt = true;

  /**
   * HTTP status code, e.g. 200, 301, 404.
   *
   * `declare` suppresses the ES2022 class-field `[[Define]]` emission so
   * esbuild/tsc don't emit `status;` (which would set the field to `undefined`
   * before the constructor body runs and then get overwritten).  Using `declare`
   * makes the field type-only — only the `this.status = status` assignment in
   * the constructor body touches the property.
   */
  declare readonly status: number;

  /** Response body string. */
  declare readonly body: string;

  /**
   * Flat pair list of extra response headers.
   *
   * Using a pair list (not an object) preserves duplicate header names, which
   * is legal in HTTP (e.g. multiple Set-Cookie headers).
   *
   *   haltHeaderPairs = [["Location", "https://…"], ["Content-Type", "text/html"]]
   */
  declare readonly haltHeaderPairs: [string, string][];

  constructor(
    status: number,
    body = "",
    headers: Record<string, string> = {},
  ) {
    super(`HaltError(${status})`);
    this.name = "HaltError";
    this.status = status;
    this.body = body;
    // Convert the convenience Record form to a pair list for the Rust side.
    this.haltHeaderPairs = Object.entries(headers) as [string, string][];
  }
}

/**
 * halt — immediately stop processing and send a response.
 *
 * Equivalent to Sinatra's `halt` helper.  Throws a HaltError which the
 * Conduit framework catches before the normal response path.
 *
 *   halt(200)                        // empty 200
 *   halt(404, "Not found")
 *   halt(503, "Maintenance", { "Retry-After": "3600" })
 */
export function halt(
  status: number,
  body = "",
  headers: Record<string, string> = {},
): never {
  throw new HaltError(status, body, headers);
}

/**
 * redirect — send an HTTP redirect and stop processing.
 *
 * Defaults to 302 Found.  Use 301 for permanent redirects.
 *
 *   redirect("https://example.com/new-path")
 *   redirect("/login", 303)
 *
 * ⚠️  Security — open redirect risk:
 * Do NOT pass unvalidated user input as the `location` argument.  If `location`
 * is derived from a query parameter, form field, or any other user-controlled
 * source, an attacker can supply an external URL (e.g. `https://evil.com`) and
 * redirect users off-site (phishing, OAuth token theft).
 *
 * Safe patterns:
 *   redirect("/dashboard")                   // static relative path — safe
 *   redirect(allowlist.get(req.params.dest))  // explicit allowlist — safe
 *
 * Unsafe patterns:
 *   redirect(req.queryParams["next"] ?? "/")  // user-controlled — DANGEROUS
 */
export function redirect(location: string, status = 302): never {
  throw new HaltError(status, "", { Location: location });
}
