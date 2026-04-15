/**
 * JSON-RPC 2.0 Message Types
 *
 * JSON-RPC defines four message shapes. All of them carry `"jsonrpc": "2.0"`.
 * The shape is determined by which fields are present:
 *
 *   ┌──────────────┬──────┬──────────┬────────┬───────┐
 *   │ Shape        │  id  │  method  │ result │ error │
 *   ├──────────────┼──────┼──────────┼────────┼───────┤
 *   │ Request      │  yes │   yes    │   —    │   —   │
 *   │ Notification │   —  │   yes    │   —    │   —   │
 *   │ Response OK  │  yes │    —     │  yes   │   —   │
 *   │ Response Err │  yes │    —     │   —    │  yes  │
 *   └──────────────┴──────┴──────────┴────────┴───────┘
 *
 * The discriminated union `Message` covers all four cases. Use `parseMessage`
 * to turn a raw JSON object into one of these typed shapes, and
 * `messageToObject` to turn one back into a plain object suitable for
 * `JSON.stringify`.
 *
 * @example
 *     // Incoming bytes → typed message
 *     const raw = JSON.parse(jsonText) as unknown;
 *     const msg = parseMessage(raw);
 *     if (msg.type === "request") {
 *       console.log(msg.method, msg.params);
 *     }
 *
 *     // Typed message → bytes to send
 *     const obj = messageToObject(msg);
 *     const bytes = JSON.stringify(obj);
 */

import { ErrorCodes } from "./errors.js";

// ---------------------------------------------------------------------------
// ResponseError — the structured error object carried inside a Response
// ---------------------------------------------------------------------------

/**
 * A structured error returned inside a JSON-RPC error Response.
 *
 * @example
 *     { code: -32601, message: "Method not found", data: "unknown/method" }
 */
export interface ResponseError {
  /** Integer error code; see ErrorCodes for standard values. */
  code: number;
  /** Short human-readable description. */
  message: string;
  /**
   * Optional additional information. Can be any JSON value: a string with
   * a stack trace, an object with field-level validation errors, etc.
   */
  data?: unknown;
}

// ---------------------------------------------------------------------------
// Four message shapes — each carries a `type` discriminant
// ---------------------------------------------------------------------------

/**
 * A call from client to server that expects a Response.
 *
 * The `id` field ties the Response back to this Request. It must not be null.
 *
 * Wire format:
 *     { "jsonrpc": "2.0", "id": 1, "method": "textDocument/hover", "params": {...} }
 */
export interface Request {
  type: "request";
  /** Unique per in-flight request; string or integer, never null. */
  id: string | number;
  /** The procedure to invoke, e.g. "textDocument/hover". */
  method: string;
  /** Optional arguments; object or array per spec. */
  params?: unknown;
}

/**
 * A one-way message with no Response.
 *
 * Notifications are used for events: "file opened", "cursor moved", etc.
 * The server must NOT send a response, even if it encounters an error
 * processing the notification.
 *
 * Wire format:
 *     { "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {...} }
 */
export interface Notification {
  type: "notification";
  /** The event name, e.g. "textDocument/didOpen". */
  method: string;
  /** Optional payload. */
  params?: unknown;
}

/**
 * A server's reply to a Request — carries either `result` or `error`.
 *
 * The `id` must match the originating Request's id. It may only be null when
 * the server cannot determine the request id (e.g., the request was
 * unparseable).
 *
 * Wire format (success):
 *     { "jsonrpc": "2.0", "id": 1, "result": {...} }
 *
 * Wire format (error):
 *     { "jsonrpc": "2.0", "id": 1, "error": { "code": -32601, "message": "..." } }
 */
export interface Response {
  type: "response";
  /** Matches the Request's id; null only when the request was unparseable. */
  id: string | number | null;
  /** The return value on success. Exactly one of result/error is present. */
  result?: unknown;
  /** The error on failure. Exactly one of result/error is present. */
  error?: ResponseError;
}

/**
 * Discriminated union covering all three public message shapes.
 * (ResponseError is a struct, not a top-level message.)
 */
export type Message = Request | Notification | Response;

// ---------------------------------------------------------------------------
// parseMessage — raw JSON object → typed Message
// ---------------------------------------------------------------------------

/**
 * Parse a raw JSON object into a typed `Message`.
 *
 * This function does NOT parse JSON text — the caller must already have done
 * `JSON.parse`. It validates the shape and assigns the `type` discriminant.
 *
 * Throws a `JsonRpcError` with code `-32600` (Invalid Request) if the object
 * is not a recognisable JSON-RPC message.
 *
 * Recognition rules (mirror the table at the top of this file):
 *   - Has `id` AND `method`               → Request
 *   - Has `method` but no `id`            → Notification
 *   - Has `id` AND (`result` OR `error`)  → Response
 *   - Anything else                        → throw Invalid Request
 *
 * @example
 *     const raw = JSON.parse('{"jsonrpc":"2.0","id":1,"method":"ping"}');
 *     const msg = parseMessage(raw);  // → { type: "request", id: 1, method: "ping" }
 */
export function parseMessage(data: unknown): Message {
  if (typeof data !== "object" || data === null || Array.isArray(data)) {
    throw new JsonRpcError(
      ErrorCodes.InvalidRequest,
      "Invalid Request: message must be a JSON object",
    );
  }

  const obj = data as Record<string, unknown>;

  // The spec requires "jsonrpc": "2.0", but we are lenient here and only
  // validate the structural fields. Strict mode could add this check.
  const hasId = "id" in obj;
  const hasMethod = "method" in obj && typeof obj["method"] === "string";
  const hasResult = "result" in obj;
  const hasError = "error" in obj;

  if (hasId && hasMethod) {
    // Request: has both id and method
    const id = obj["id"];
    if (typeof id !== "string" && typeof id !== "number") {
      throw new JsonRpcError(
        ErrorCodes.InvalidRequest,
        "Invalid Request: id must be string or number",
      );
    }
    return {
      type: "request",
      id,
      method: obj["method"] as string,
      params: obj["params"],
    };
  }

  if (hasMethod && !hasId) {
    // Notification: has method, no id
    return {
      type: "notification",
      method: obj["method"] as string,
      params: obj["params"],
    };
  }

  if (hasId && (hasResult || hasError)) {
    // Response: has id and result/error
    const id = obj["id"];
    if (
      typeof id !== "string" &&
      typeof id !== "number" &&
      id !== null
    ) {
      throw new JsonRpcError(
        ErrorCodes.InvalidRequest,
        "Invalid Request: response id must be string, number, or null",
      );
    }
    const resp: Response = {
      type: "response",
      id,
    };
    if (hasResult) {
      resp.result = obj["result"];
    }
    if (hasError) {
      const errObj = obj["error"];
      if (
        typeof errObj !== "object" ||
        errObj === null ||
        Array.isArray(errObj)
      ) {
        throw new JsonRpcError(
          ErrorCodes.InvalidRequest,
          "Invalid Request: error must be an object",
        );
      }
      const errRecord = errObj as Record<string, unknown>;
      if (
        typeof errRecord["code"] !== "number" ||
        typeof errRecord["message"] !== "string"
      ) {
        throw new JsonRpcError(
          ErrorCodes.InvalidRequest,
          "Invalid Request: error must have numeric code and string message",
        );
      }
      resp.error = {
        code: errRecord["code"] as number,
        message: errRecord["message"] as string,
        data: errRecord["data"],
      };
    }
    return resp;
  }

  throw new JsonRpcError(
    ErrorCodes.InvalidRequest,
    "Invalid Request: unrecognised message shape",
  );
}

// ---------------------------------------------------------------------------
// messageToObject — typed Message → plain object for JSON.stringify
// ---------------------------------------------------------------------------

/**
 * Convert a typed `Message` back to a plain JavaScript object that can be
 * serialised with `JSON.stringify`.
 *
 * The `type` discriminant is stripped — it is our internal tag, not part of
 * the wire format. The `"jsonrpc": "2.0"` field is added.
 *
 * @example
 *     const req: Request = { type: "request", id: 1, method: "ping" };
 *     JSON.stringify(messageToObject(req));
 *     // → '{"jsonrpc":"2.0","id":1,"method":"ping"}'
 */
export function messageToObject(msg: Message): Record<string, unknown> {
  switch (msg.type) {
    case "request": {
      const obj: Record<string, unknown> = {
        jsonrpc: "2.0",
        id: msg.id,
        method: msg.method,
      };
      if (msg.params !== undefined) {
        obj["params"] = msg.params;
      }
      return obj;
    }
    case "notification": {
      const obj: Record<string, unknown> = {
        jsonrpc: "2.0",
        method: msg.method,
      };
      if (msg.params !== undefined) {
        obj["params"] = msg.params;
      }
      return obj;
    }
    case "response": {
      const obj: Record<string, unknown> = {
        jsonrpc: "2.0",
        id: msg.id,
      };
      if (msg.error !== undefined) {
        const errObj: Record<string, unknown> = {
          code: msg.error.code,
          message: msg.error.message,
        };
        if (msg.error.data !== undefined) {
          errObj["data"] = msg.error.data;
        }
        obj["error"] = errObj;
      } else {
        // result may be undefined (null is a valid result)
        obj["result"] = msg.result !== undefined ? msg.result : null;
      }
      return obj;
    }
  }
}

// ---------------------------------------------------------------------------
// JsonRpcError — thrown when framing or parsing fails
// ---------------------------------------------------------------------------

/**
 * Error thrown by the JSON-RPC layer when a message cannot be read or parsed.
 *
 * The `code` follows the standard error code table in `errors.ts`.
 *
 * @example
 *     throw new JsonRpcError(ErrorCodes.ParseError, "Unexpected token");
 */
export class JsonRpcError extends Error {
  readonly code: number;

  constructor(code: number, message: string) {
    super(message);
    this.name = "JsonRpcError";
    this.code = code;
  }
}
