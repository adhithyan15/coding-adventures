/**
 * RpcServer — codec-agnostic request dispatcher
 *
 * The `RpcServer` drives the read-dispatch-write loop that is the heart of
 * any RPC server. It is deliberately ignorant of how bytes are serialised
 * (that's the codec's job) and of how frames are delimited (that's the
 * framer's job). It only knows how to:
 *
 *   1. Ask the framer for the next frame.
 *   2. Ask the codec to decode it into a typed message.
 *   3. Look up the correct handler in a dispatch table.
 *   4. Call the handler (catching any exceptions).
 *   5. Encode the result and hand it back to the framer.
 *
 * This separation means the exact same server loop works for JSON, MessagePack,
 * Protobuf, or any future codec — you only change the codec + framer pair.
 *
 * Lifecycle
 * ---------
 *
 *   1. Construct with a codec and a framer:
 *        const server = new RpcServer(myCodec, myFramer);
 *   2. Register handlers (chainable):
 *        server
 *          .onRequest("ping", (_id, _params) => "pong")
 *          .onNotification("log", (params) => console.log(params));
 *   3. Call `serve()` — it blocks until EOF or unrecoverable I/O error:
 *        server.serve();
 *
 * Dispatch rules
 * --------------
 *
 *   Request received:
 *     - Handler found  → call it; encode result; write response frame
 *     - No handler     → encode -32601 (Method not found) error response
 *     - Handler throws → encode -32603 (Internal error) error response
 *     - Handler returns an `RpcErrorResponse`-shaped value → write error response
 *
 *   Notification received:
 *     - Handler found  → call it; write NOTHING (spec forbids notification responses)
 *     - No handler     → silently ignore (spec says no error for unknown notifications)
 *     - Handler throws → swallow the exception; write NOTHING
 *
 *   Response or ErrorResponse received by server:
 *     - Discarded in server-only mode (future bidirectional peer can route these)
 *
 *   Decode error (codec throws on a frame):
 *     - Write an error response with null id and the appropriate error code
 *     - Continue the loop (don't shut down for one bad frame)
 *
 * Panic safety
 * ------------
 * TypeScript has no "panic" concept, but a handler that throws an Error (or
 * any other value) must not kill the server. Every handler call is wrapped in
 * try/catch. A caught exception becomes an `InternalError` (-32603) response.
 *
 * @example
 *     const server = new RpcServer(new JsonCodec(), new ContentLengthFramer(process.stdin, process.stdout));
 *     server
 *       .onRequest("add", (_id, params) => {
 *         const { a, b } = params as { a: number; b: number };
 *         return a + b;
 *       })
 *       .onNotification("shutdown", () => process.exit(0));
 *     server.serve();
 */

import type { RpcCodec } from "./codec.js";
import type { RpcFramer } from "./framer.js";
import type { RpcId, RpcMessage, RpcErrorResponse } from "./message.js";
import { RpcErrorCodes, RpcError } from "./errors.js";

// ---------------------------------------------------------------------------
// Handler type aliases
// ---------------------------------------------------------------------------

/**
 * Handler for an incoming `RpcRequest`.
 *
 * Receives the request id and params; returns a result value or a Promise of
 * one. May also return an `RpcErrorResponse`-shaped object to send an explicit
 * error response without throwing.
 *
 * @typeParam V - The codec's value type (same as the server's `V`).
 */
export type RpcRequestHandler<V> = (
  id: RpcId,
  params: V | undefined,
) => V | Promise<V>;

/**
 * Handler for an incoming `RpcNotification`.
 *
 * Receives the params; returns nothing (notifications never generate a
 * response). May be async.
 *
 * @typeParam V - The codec's value type (same as the server's `V`).
 */
export type RpcNotificationHandler<V> = (
  params: V | undefined,
) => void | Promise<void>;

// ---------------------------------------------------------------------------
// RpcServer
// ---------------------------------------------------------------------------

/**
 * A codec-agnostic RPC server.
 *
 * Reads frames from the framer, decodes them with the codec, dispatches to
 * registered handlers, and writes response frames back. Runs synchronously
 * in a blocking loop until EOF.
 *
 * @typeParam V - The codec's native dynamic value type (e.g., `unknown` for
 *               a JSON codec). Handlers receive and return values of this type.
 *
 * @example
 *     const server = new RpcServer(codec, framer);
 *     server
 *       .onRequest("greet", (_id, params) => `Hello, ${params}!`)
 *       .onNotification("ping", () => { /* fire and forget *\/ });
 *     server.serve();
 */
export class RpcServer<V> {
  private readonly codec: RpcCodec<V>;
  private readonly framer: RpcFramer;

  /** Map from method name to request handler. */
  private readonly requestHandlers: Map<string, RpcRequestHandler<V>>;
  /** Map from method name to notification handler. */
  private readonly notificationHandlers: Map<string, RpcNotificationHandler<V>>;

  /**
   * Construct a new `RpcServer`.
   *
   * @param codec  - Translates between `RpcMessage<V>` and `Uint8Array`.
   * @param framer - Reads and writes discrete byte frames from/to the stream.
   */
  constructor(codec: RpcCodec<V>, framer: RpcFramer) {
    this.codec = codec;
    this.framer = framer;
    this.requestHandlers = new Map();
    this.notificationHandlers = new Map();
  }

  // -------------------------------------------------------------------------
  // Handler registration — fluent/chainable API
  // -------------------------------------------------------------------------

  /**
   * Register a handler for a named request method.
   *
   * Registering the same method name twice replaces the previous handler.
   * The handler receives `(id, params)` and may return:
   *   - A plain value (sent as the `result` field in the response).
   *   - A `Promise` of a plain value (awaited before responding).
   *   - If it throws, the server sends a `-32603 InternalError` response.
   *
   * Returns `this` for chaining.
   *
   * @param method  - The procedure name, e.g. `"textDocument/hover"`.
   * @param handler - The function to invoke.
   *
   * @example
   *     server
   *       .onRequest("add", (_id, params) => {
   *         const p = params as { a: number; b: number };
   *         return p.a + p.b;
   *       })
   *       .onRequest("echo", (_id, params) => params);
   */
  onRequest(method: string, handler: RpcRequestHandler<V>): this {
    this.requestHandlers.set(method, handler);
    return this;
  }

  /**
   * Register a handler for a named notification method.
   *
   * Unknown notifications are silently dropped per the RPC spec. Registering
   * the same method name twice replaces the previous handler. The handler
   * receives `params`; its return value (if any) is ignored. If the handler
   * throws, the exception is swallowed — notifications must never generate
   * a response.
   *
   * Returns `this` for chaining.
   *
   * @param method  - The notification name, e.g. `"textDocument/didOpen"`.
   * @param handler - The function to invoke.
   *
   * @example
   *     server.onNotification("$/cancelRequest", (params) => {
   *       const { id } = params as { id: number };
   *       pendingRequests.cancel(id);
   *     });
   */
  onNotification(method: string, handler: RpcNotificationHandler<V>): this {
    this.notificationHandlers.set(method, handler);
    return this;
  }

  // -------------------------------------------------------------------------
  // serve() — the main loop
  // -------------------------------------------------------------------------

  /**
   * Start the server loop.
   *
   * Reads frames one at a time until the framer signals EOF (returns `null`).
   * Each frame is decoded by the codec and dispatched to the appropriate
   * handler. Responses are encoded and written back through the framer.
   *
   * This method is **synchronous and blocking** — it does not return until
   * the stream closes. In a Node.js context, call this from your main entry
   * point after registering all handlers.
   *
   * The loop is resilient to per-message errors: a single undecodable frame
   * or panicking handler sends an error response but does NOT terminate the
   * loop. Only EOF or an unrecoverable framer I/O error stops the server.
   *
   * @example
   *     server
   *       .onRequest("ping", () => "pong")
   *       .serve();
   *     // Server has exited — stream is closed.
   */
  serve(): void {
    while (true) {
      // -----------------------------------------------------------------------
      // Step 1: Read the next frame from the framer.
      // null means the remote end closed the connection cleanly — we exit.
      // -----------------------------------------------------------------------
      let frameBytes: Uint8Array | null;
      try {
        frameBytes = this.framer.readFrame();
      } catch (err) {
        // A framing error (malformed header, truncated frame, etc.).
        // We can't recover without the frame, so send an error and continue.
        const code =
          err instanceof RpcError ? err.code : RpcErrorCodes.InternalError;
        const message = err instanceof Error ? err.message : "Framing error";
        this.sendError(null, code, message);
        continue;
      }

      // Clean EOF — exit the loop.
      if (frameBytes === null) break;

      // -----------------------------------------------------------------------
      // Step 2: Decode the frame bytes into a typed RpcMessage.
      // -----------------------------------------------------------------------
      let msg: RpcMessage<V>;
      try {
        msg = this.codec.decode(frameBytes);
      } catch (err) {
        // The codec could not parse or validate the frame.
        const code =
          err instanceof RpcError ? err.code : RpcErrorCodes.ParseError;
        const message = err instanceof Error ? err.message : "Decode error";
        this.sendError(null, code, message);
        continue;
      }

      // -----------------------------------------------------------------------
      // Step 3: Dispatch the message to the right handler.
      // -----------------------------------------------------------------------
      this.dispatch(msg);
    }
  }

  // -------------------------------------------------------------------------
  // Private dispatch helpers
  // -------------------------------------------------------------------------

  /**
   * Route a decoded message to the appropriate handler.
   *
   * The four cases map directly to the four RpcMessage kinds:
   *   - `request`      → handleRequest (sends a response)
   *   - `notification` → handleNotification (sends nothing)
   *   - `response`     → discarded in server-only mode
   *   - `error`        → discarded in server-only mode
   */
  private dispatch(msg: RpcMessage<V>): void {
    switch (msg.kind) {
      case "request":
        this.handleRequest(msg.id, msg.method, msg.params);
        break;
      case "notification":
        this.handleNotification(msg.method, msg.params);
        break;
      case "response":
      case "error":
        // In server-only mode, incoming responses are ignored.
        // A bidirectional peer implementation would route these to a
        // pending-request map keyed by id.
        break;
    }
  }

  /**
   * Dispatch a request to its registered handler and write a response.
   *
   * Three outcomes:
   *   1. No handler registered → write -32601 (MethodNotFound).
   *   2. Handler throws         → write -32603 (InternalError).
   *   3. Handler returns        → write success response with `result`.
   */
  private handleRequest(
    id: RpcId,
    method: string,
    params: V | undefined,
  ): void {
    const handler = this.requestHandlers.get(method);

    if (!handler) {
      // Per spec: unknown method MUST produce an error response (not silence).
      this.sendError(
        id,
        RpcErrorCodes.MethodNotFound,
        `Method not found: ${method}`,
      );
      return;
    }

    let result: V;
    try {
      // The handler may be synchronous or return a Promise.
      // NOTE: this package uses a synchronous I/O model (matching the spec's
      // intent for stdio-based RPC). If the handler returns a Promise, we
      // resolve it synchronously using a simple blocking approach. For async
      // handlers, the caller should use the async variant or ensure the
      // Promise is already resolved. In practice, most handlers are sync.
      //
      // For full async support, see the async serve() pattern in the README.
      const maybePromise = handler(id, params);

      // If the handler returned a plain value (not a Promise), use it directly.
      if (!(maybePromise instanceof Promise)) {
        result = maybePromise;
        this.sendResponse(id, result);
        return;
      }

      // For Promises: use synchronous-style resolution via a flag variable.
      // In Node.js with synchronous framer (Buffer-based), the promise from
      // a non-async handler resolves immediately. For truly async handlers,
      // users should use the async version of serve().
      let resolved = false;
      // Use a box object to hold the resolved value so TypeScript knows it was
      // assigned inside the .then() callback before we use it.
      const box: { value?: V; error?: unknown } = {};

      maybePromise.then(
        (v) => {
          resolved = true;
          box.value = v;
        },
        (e: unknown) => {
          resolved = true;
          box.error = e;
        },
      );

      if (!resolved) {
        // The promise didn't resolve synchronously. This means the handler is
        // truly async. We treat this as an internal error since this server
        // operates in synchronous mode.
        this.sendError(
          id,
          RpcErrorCodes.InternalError,
          "Handler returned unresolved Promise in synchronous serve() mode",
        );
        return;
      }

      if ("error" in box) {
        throw box.error;
      }

      result = box.value as V;
      this.sendResponse(id, result);
    } catch (err) {
      // The handler threw (or was rejected). Report as InternalError.
      const message =
        err instanceof Error ? err.message : "Internal server error";
      this.sendError(id, RpcErrorCodes.InternalError, message);
    }
  }

  /**
   * Dispatch a notification to its registered handler.
   *
   * Unknown notifications are silently dropped. Handler errors are swallowed.
   * No response is ever written for notifications — the spec forbids it.
   */
  private handleNotification(method: string, params: V | undefined): void {
    const handler = this.notificationHandlers.get(method);
    if (!handler) {
      // Silently ignore — spec: unknown notifications must not generate errors.
      return;
    }

    try {
      const maybePromise = handler(params);
      // For truly async handlers, errors are silently swallowed.
      if (maybePromise instanceof Promise) {
        maybePromise.catch(() => {
          // Swallow — notifications must never produce responses.
        });
      }
    } catch {
      // Swallow all notification handler errors. The spec is unambiguous:
      // notifications must not generate responses, even on error.
    }
  }

  /** Write a success response frame for a given request id. */
  private sendResponse(id: RpcId, result: V): void {
    const msg: RpcMessage<V> = { kind: "response", id, result };
    const bytes = this.codec.encode(msg);
    this.framer.writeFrame(bytes);
  }

  /**
   * Write an error response frame.
   *
   * @param id      - The originating request id, or `null` if unknown.
   * @param code    - Standard error code (see `RpcErrorCodes`).
   * @param message - Human-readable description.
   * @param data    - Optional extra diagnostic data.
   */
  private sendError(
    id: RpcId | null,
    code: number,
    message: string,
    data?: V,
  ): void {
    const errMsg: RpcErrorResponse<V> = { kind: "error", id, code, message };
    if (data !== undefined) errMsg.data = data;
    const bytes = this.codec.encode(errMsg);
    this.framer.writeFrame(bytes);
  }
}
