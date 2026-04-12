/**
 * JSON-RPC 2.0 Standard Error Codes
 *
 * The JSON-RPC 2.0 specification reserves a set of integer error codes that
 * carry well-known meanings. Whenever the server cannot handle a request, it
 * sends a Response whose `error` field contains one of these codes plus a
 * human-readable message.
 *
 * Error code ranges
 * -----------------
 *
 *   -32700              Parse error   — the bytes were not valid JSON at all
 *   -32600              Invalid Request — valid JSON, but not a JSON-RPC message
 *   -32601              Method not found — the method name is not registered
 *   -32602              Invalid params — the handler received bad arguments
 *   -32603              Internal error — an unexpected exception inside a handler
 *   -32099 .. -32000    Server errors — free for implementations to use
 *
 * LSP additionally reserves -32899 .. -32800 for LSP-specific conditions; we
 * intentionally leave that range alone here.
 *
 * Why constants instead of an enum?
 * ----------------------------------
 * TypeScript `const enum` values are inlined by the compiler and produce no
 * runtime object, which is ideal for a hot path. A plain object with `as const`
 * achieves the same result while also being importable at runtime if needed.
 *
 * @example
 *     import { ErrorCodes } from "./errors.js";
 *     const err = { code: ErrorCodes.MethodNotFound, message: "Method not found" };
 */

export const ErrorCodes = {
  /**
   * The message body is not valid JSON.
   *
   * Example: the bytes `{broken json` arrive — they cannot be parsed at all.
   */
  ParseError: -32700,

  /**
   * The JSON was parsed but is not a valid JSON-RPC 2.0 Request object.
   *
   * Example: `{"jsonrpc":"2.0"}` is valid JSON but has neither `id+method`
   * (Request) nor `method` alone (Notification) nor `id+result/error`
   * (Response).
   */
  InvalidRequest: -32600,

  /**
   * The requested method name is not registered on the server.
   *
   * The server must send this error — it must NOT silently drop the request.
   * (Silently dropping is only allowed for Notifications.)
   */
  MethodNotFound: -32601,

  /**
   * The method was found but the supplied params are wrong.
   *
   * Handlers may return this when required fields are missing or types do not
   * match what the method expects.
   */
  InvalidParams: -32602,

  /**
   * An unexpected error occurred inside the handler.
   *
   * This is the catch-all for server-side bugs. Distinguish it from
   * InvalidParams (-32602) by asking: "was the request itself well-formed?"
   * If yes, use InternalError; if no, use InvalidParams.
   */
  InternalError: -32603,
} as const;

/** Union of all valid error code values. */
export type ErrorCode = (typeof ErrorCodes)[keyof typeof ErrorCodes];
