/**
 * RPC Message Types
 *
 * This module defines the four message shapes that flow through any RPC
 * system. The shapes are codec-agnostic: a JSON codec, a MessagePack codec,
 * and a Protobuf codec all produce and consume these same TypeScript types.
 *
 * The `V` type parameter ("Value") represents the codec's native dynamic
 * value type:
 *   - For a JSON codec, `V` might be `unknown` (any JSON-compatible value).
 *   - For a MessagePack codec, `V` might be a `MsgpackValue` union type.
 *   - For a Protobuf codec, `V` might be `proto.Message`.
 *
 * The RPC layer never inspects or transforms `V` — it just passes it through
 * as params or result data. This is the "black box" principle: the codec
 * owns `V`; the RPC layer doesn't need to know what's inside.
 *
 * Message shape decision tree
 * ---------------------------
 *
 *   Does the message expect a reply?
 *   ├── YES → It has an `id`
 *   │   ├── Is it going from client to server? → RpcRequest  { id, method, params? }
 *   │   └── Is it going from server to client? → RpcResponse { id, result }
 *   │                                          or RpcErrorResponse { id, code, message, data? }
 *   └── NO  → It has no id → RpcNotification { method, params? }
 *
 * The `kind` discriminant lets TypeScript narrow the union safely:
 *
 *     function handleMsg<V>(msg: RpcMessage<V>) {
 *       switch (msg.kind) {
 *         case 'request':      // TypeScript knows msg is RpcRequest<V>
 *         case 'response':     // TypeScript knows msg is RpcResponse<V>
 *         case 'error':        // TypeScript knows msg is RpcErrorResponse<V>
 *         case 'notification': // TypeScript knows msg is RpcNotification<V>
 *       }
 *     }
 */

// ---------------------------------------------------------------------------
// RpcId — the correlation key that ties requests to responses
// ---------------------------------------------------------------------------

/**
 * An RPC message identifier.
 *
 * Clients assign an id to every Request. The server echoes the same id in
 * its Response, allowing the client to correlate which response belongs to
 * which request — especially important when multiple requests are in-flight
 * simultaneously.
 *
 * The spec allows either a string (e.g., "req-001") or a number (e.g., 42).
 * A monotonically-increasing integer counter is the most common choice for
 * clients that manage concurrency.
 *
 * `null` is only valid in an `RpcErrorResponse` when the server could not
 * even extract the id from the malformed incoming request.
 */
export type RpcId = string | number;

// ---------------------------------------------------------------------------
// Four message shapes
// ---------------------------------------------------------------------------

/**
 * A call from client to server that expects a response.
 *
 * The `id` links this request to its eventual `RpcResponse` or
 * `RpcErrorResponse`. The `method` names the procedure to invoke on the
 * server. `params` carries the arguments (optional).
 *
 * @example
 *     const req: RpcRequest<unknown> = {
 *       kind: 'request',
 *       id: 1,
 *       method: 'textDocument/hover',
 *       params: { position: { line: 10, character: 5 } },
 *     };
 */
export interface RpcRequest<V> {
  /** Discriminant — always `'request'`. */
  kind: "request";
  /** Unique id for this in-flight call. Must not be null. */
  id: RpcId;
  /** The name of the procedure to invoke. */
  method: string;
  /** Optional arguments to pass to the handler. */
  params?: V;
}

/**
 * A server's successful reply to an `RpcRequest`.
 *
 * The `id` must match the originating request's `id`. The `result` carries
 * the return value from the handler.
 *
 * @example
 *     const resp: RpcResponse<unknown> = {
 *       kind: 'response',
 *       id: 1,
 *       result: { contents: 'function add(a, b) returns a + b' },
 *     };
 */
export interface RpcResponse<V> {
  /** Discriminant — always `'response'`. */
  kind: "response";
  /** Matches the originating request's id. */
  id: RpcId;
  /** The return value produced by the handler. */
  result: V;
}

/**
 * A server's error reply to an `RpcRequest`.
 *
 * Sent when the server cannot fulfil a request: the method is unknown, the
 * params are wrong, or the handler threw an exception. The `id` matches the
 * originating request; when the request was so malformed that the id could
 * not be extracted, `id` is `null`.
 *
 * @example
 *     const errResp: RpcErrorResponse<unknown> = {
 *       kind: 'error',
 *       id: 1,
 *       code: -32601,
 *       message: 'Method not found: textDocument/missing',
 *     };
 */
export interface RpcErrorResponse<V> {
  /** Discriminant — always `'error'`. */
  kind: "error";
  /** Matches the originating request's id; null if id was not recoverable. */
  id: RpcId | null;
  /** Standard error code; see `RpcErrorCodes`. */
  code: number;
  /** Human-readable description of the error. */
  message: string;
  /**
   * Optional extra information — a stack trace, field-level validation
   * errors, etc. The codec decides how to serialise this.
   */
  data?: V;
}

/**
 * A one-way message with no response.
 *
 * Notifications are fire-and-forget: the sender never waits for a reply and
 * the receiver must never send one, even if it encounters an error processing
 * the notification.
 *
 * Analogy: you send a text message saying "I'm on my way". You don't wait for
 * a confirmation; the recipient just reads it and acts accordingly.
 *
 * Common uses: event streams ("file saved"), progress updates, log sinks.
 *
 * @example
 *     const notif: RpcNotification<unknown> = {
 *       kind: 'notification',
 *       method: 'textDocument/didSave',
 *       params: { uri: 'file:///src/main.ts' },
 *     };
 */
export interface RpcNotification<V> {
  /** Discriminant — always `'notification'`. */
  kind: "notification";
  /** The event name. */
  method: string;
  /** Optional payload. */
  params?: V;
}

/**
 * Discriminated union of all four RPC message shapes.
 *
 * Use `switch (msg.kind)` to narrow to a specific shape. TypeScript will
 * exhaustively check that you've handled all four cases.
 *
 * @example
 *     function process<V>(msg: RpcMessage<V>) {
 *       switch (msg.kind) {
 *         case 'request':      return handleRequest(msg);
 *         case 'response':     return handleResponse(msg);
 *         case 'error':        return handleError(msg);
 *         case 'notification': return handleNotification(msg);
 *       }
 *     }
 */
export type RpcMessage<V> =
  | RpcRequest<V>
  | RpcResponse<V>
  | RpcErrorResponse<V>
  | RpcNotification<V>;
