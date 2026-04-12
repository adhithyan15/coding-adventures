/**
 * RPC Standard Error Codes
 *
 * These error codes are codec-agnostic integers. The same table applies
 * regardless of whether the wire format is JSON, MessagePack, Protobuf, or
 * any other encoding. They originate from the JSON-RPC 2.0 specification but
 * belong conceptually to the RPC layer, not to JSON.
 *
 * Think of them like HTTP status codes — they are a shared vocabulary that
 * everyone in the ecosystem understands, independent of whether you are using
 * HTTP/1.1 or HTTP/2 or HTTP/3 underneath.
 *
 * Error code ranges
 * -----------------
 *
 *   -32700              Parse error   — the framed bytes could not be decoded
 *                                       by the codec at all
 *   -32600              Invalid Request — decoded successfully but not a valid
 *                                         RPC message structure
 *   -32601              Method not found — no handler registered for the method
 *   -32602              Invalid params — the handler rejected the params
 *   -32603              Internal error — an unexpected exception inside a handler
 *   -32099 .. -32000    Server errors — free for server implementations to use
 *
 * Application-specific codes should be outside the reserved range (i.e., not
 * between -32768 and -32000). LSP additionally reserves -32899 .. -32800 for
 * LSP-specific conditions; we leave that range untouched here.
 *
 * Why `as const` instead of an enum?
 * ------------------------------------
 * TypeScript enums generate a runtime object AND a reverse-mapping object.
 * A plain object with `as const` produces zero runtime overhead: the compiler
 * inlines the integer literals wherever they are used. It also makes the
 * values importable as runtime data if needed (e.g., a lookup table in tests).
 *
 * @example
 *     import { RpcErrorCodes } from "./errors.js";
 *     // Send a -32601 response when no handler matches:
 *     const code = RpcErrorCodes.MethodNotFound;  // -32601
 */

export const RpcErrorCodes = {
  /**
   * The framed bytes could not be decoded by the codec.
   *
   * Analogy: you received a letter but it's written in an alphabet you
   * cannot read at all — not even wrong, just unrecognisable bytes.
   *
   * Example: the codec is JSON and the bytes are `{broken json` — they
   * cannot be parsed as JSON at all.
   */
  ParseError: -32700,

  /**
   * The bytes decoded successfully, but the result is not a valid RPC message.
   *
   * Analogy: the letter is legible, but it doesn't follow the expected
   * structure — it's missing the "To:", "From:", and "Subject:" fields.
   *
   * Example: `{"foo": "bar"}` is valid JSON, but has neither `method` (which
   * would make it a Request or Notification) nor `result`/`error` with `id`
   * (which would make it a Response).
   */
  InvalidRequest: -32600,

  /**
   * The method name in the incoming request is not registered on the server.
   *
   * Analogy: the caller asked for "the plumber", but this office only has
   * an electrician and a painter.
   *
   * Per spec, the server MUST send this error — it must NOT silently drop
   * the request. Silent dropping is only correct for Notifications.
   */
  MethodNotFound: -32601,

  /**
   * The method exists but the supplied params are wrong.
   *
   * Analogy: you called the right department, but gave them an account
   * number in the wrong format.
   *
   * Handler implementations should return this error when required fields are
   * missing, types are wrong, or values are out of range.
   */
  InvalidParams: -32602,

  /**
   * An unexpected error occurred inside the handler.
   *
   * Analogy: the plumber arrived but discovered a burst pipe that wasn't
   * on the work order — something unexpected went wrong on the server's
   * side, not because of bad input from the client.
   *
   * This is the catch-all for server-side bugs or panics. Distinguish it from
   * `InvalidParams` (-32602) by asking: "was the request itself well-formed?"
   * If yes and the error is internal, use `InternalError`.
   */
  InternalError: -32603,
} as const;

/** Union type of all valid standard error code values. */
export type RpcErrorCode = (typeof RpcErrorCodes)[keyof typeof RpcErrorCodes];

// ---------------------------------------------------------------------------
// RpcError — thrown when the RPC layer encounters a fatal message problem
// ---------------------------------------------------------------------------

/**
 * Error class thrown by the RPC layer when a frame cannot be decoded or the
 * resulting value is not a valid RPC message.
 *
 * This is NOT the same as a business-logic error returned by a handler.
 * `RpcError` represents a transport-level failure: the message itself is
 * malformed.
 *
 * The `code` field holds one of the `RpcErrorCodes` constants so the server
 * loop can map it to the correct error response.
 *
 * @example
 *     throw new RpcError(RpcErrorCodes.ParseError, "Unexpected byte 0x42");
 */
export class RpcError extends Error {
  /** Standard RPC error code; see `RpcErrorCodes`. */
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = "RpcError";
    this.code = code;
  }
}
