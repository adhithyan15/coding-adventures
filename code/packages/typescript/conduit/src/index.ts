/**
 * index.ts — Public API surface for coding-adventures-conduit.
 *
 * Everything a user needs is re-exported from this single entry point:
 *
 *   import {
 *     Application,            // route + filter registry
 *     Server,                 // TCP bind + web-core engine
 *     HaltError,              // thrown by halt() / redirect()
 *     halt, redirect,         // short-circuit helpers
 *     html, json, text,       // response builders
 *     respond,
 *   } from "coding-adventures-conduit";
 *
 * The exports are split across four files:
 *
 *   halt_error.ts       — HaltError, halt(), redirect()
 *   handler_context.ts  — html(), json(), text(), respond(), ResponseTuple
 *   request.ts          — Request, EnvMap
 *   application.ts      — Application, Handler, ErrorHandler, RouteEntry
 *   server.ts           — Server, ServerOptions
 *
 * Internal implementation detail: the Rust cdylib (conduit_native_node.node)
 * is loaded lazily by Server's constructor.  It is never exposed in this
 * public API.
 */

export { HaltError, halt, redirect } from "./halt_error.js";
export { html, json, text, respond, type ResponseTuple } from "./handler_context.js";
export { Request, type EnvMap } from "./request.js";
export {
  Application,
  type Handler,
  type ErrorHandler,
  type RouteEntry,
} from "./application.js";
export { Server, type ServerOptions } from "./server.js";
