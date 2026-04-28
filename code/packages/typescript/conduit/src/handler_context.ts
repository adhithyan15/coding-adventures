/**
 * handler_context.ts — Response builder helpers available to every handler.
 *
 * Design intent
 * ─────────────
 * Handlers return a *response tuple* back to the Conduit framework:
 *
 *   [status: number, headers: Record<string,string>, body: string]
 *
 * Writing `[200, { "Content-Type": "text/html" }, "<h1>Hi</h1>"]` every
 * time is verbose and error-prone.  The helper functions below make the
 * common cases a one-liner:
 *
 *   return html("<h1>Hello</h1>");
 *   return json({ ok: true });
 *   return text("pong");
 *   return respond(201, "Created", { "Location": "/items/42" });
 *
 * These helpers are also exported from the package top-level so handlers can
 * import them directly without knowing where they come from:
 *
 *   import { json, html, text, respond } from "coding-adventures-conduit";
 *
 * Response tuple
 * ──────────────
 * The Rust cdylib (conduit_native_node/src/lib.rs, function `parse_response`)
 * expects JS handlers to return:
 *
 *   undefined  → no override (before/after filters use this to pass through)
 *   [number, Record<string,string>, string]
 *
 * The pair list the Rust side actually reads comes from the flat headers
 * object here — the cdylib reads `headers` as a JS object and iterates its
 * own property keys.
 *
 * HandlerContext vs standalone helpers
 * ─────────────────────────────────────
 * Ruby Sinatra puts these as instance methods on the handler class.  Python
 * Conduit passes a `HandlerContext` object.  For TypeScript, we take the
 * simpler approach: export them as standalone functions.  Handlers do not
 * receive a context parameter — they just call the helpers by name.
 *
 * The `halt()` and `redirect()` helpers live in halt_error.ts and are
 * re-exported from here for convenience.
 */

export { halt, redirect, HaltError } from "./halt_error.js";

/** A response tuple as returned by a Conduit handler. */
export type ResponseTuple = [
  status: number,
  headers: Record<string, string>,
  body: string,
];

/**
 * html — return an HTML response.
 *
 *   return html("<h1>Hello</h1>");           // 200 text/html
 *   return html("<h1>Not Found</h1>", 404);  // 404 text/html
 */
export function html(body: string, status = 200): ResponseTuple {
  return [status, { "Content-Type": "text/html; charset=utf-8" }, body];
}

/**
 * json — serialise a value to JSON and return an application/json response.
 *
 *   return json({ ok: true });              // 200 application/json
 *   return json({ id: 42 }, 201);           // 201 application/json
 */
export function json(value: unknown, status = 200): ResponseTuple {
  return [
    status,
    { "Content-Type": "application/json" },
    JSON.stringify(value),
  ];
}

/**
 * text — return a plain-text response.
 *
 *   return text("pong");
 */
export function text(body: string, status = 200): ResponseTuple {
  return [status, { "Content-Type": "text/plain; charset=utf-8" }, body];
}

/**
 * respond — full control over status, headers, and body.
 *
 *   return respond(204, {}, "");
 *   return respond(201, { "Location": "/items/1" }, "");
 */
export function respond(
  status: number,
  body: string,
  headers: Record<string, string> = {},
): ResponseTuple {
  return [status, headers, body];
}
