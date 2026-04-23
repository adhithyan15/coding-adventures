/**
 * RpcClient — codec-agnostic request sender
 *
 * The `RpcClient` sends requests to a remote RPC server and waits (blocking)
 * for matching responses. It also sends fire-and-forget notifications.
 *
 * Where the server's loop is "receive, dispatch, respond", the client's flow
 * is "send, wait, return":
 *
 *   client.request("add", { a: 1, b: 2 })
 *     → encodes and writes a Request frame
 *     → reads frames until it finds the Response with a matching id
 *     → returns the result (or throws on error)
 *
 * Analogy: a synchronous phone call. You call a number (send a request),
 * wait on the line (block reading frames), and listen for the answer (the
 * matching response). While waiting, you might receive automated messages
 * (server-push notifications) which you handle via registered handlers before
 * going back to listening for your answer.
 *
 * Id management
 * -------------
 * The client maintains a monotonically increasing integer counter starting at
 * 1. Each call to `request()` atomically increments the counter and uses the
 * new value as the request id. This guarantees uniqueness within a single
 * client instance's lifetime.
 *
 *   first request  → id = 1
 *   second request → id = 2
 *   third request  → id = 3
 *   …
 *
 * Blocking request flow
 * ---------------------
 * This client is deliberately synchronous, matching the single-threaded,
 * stdio-based transport that most RPC-over-pipe systems use:
 *
 *   1. Write Request frame.
 *   2. Loop reading frames:
 *        - If frame id matches our request id → return result (or error).
 *        - If it's a server-push Notification → call handler if registered.
 *        - If it's EOF → throw "connection closed".
 *        - Otherwise → discard and continue.
 *
 * Server-push notifications
 * -------------------------
 * The server may send unsolicited notifications while the client is blocked
 * waiting for a response (e.g., diagnostics in LSP). Register handlers with
 * `onNotification()` to receive these. Unregistered notification methods are
 * silently dropped.
 *
 * @example
 *     const client = new RpcClient(new JsonCodec(), new ContentLengthFramer(inStream, outStream));
 *
 *     // Register for server-push notifications:
 *     client.onNotification("$/progress", (params) => {
 *       console.log("Progress:", params);
 *     });
 *
 *     // Send a request and block until the response arrives:
 *     const result = client.request("add", { a: 3, b: 4 });
 *     console.log(result); // 7
 *
 *     // Fire-and-forget:
 *     client.notify("$/cancelRequest", { id: 1 });
 */

import type { RpcCodec } from "./codec.js";
import type { RpcFramer } from "./framer.js";
import type { RpcId, RpcMessage } from "./message.js";
import { RpcErrorCodes, RpcError } from "./errors.js";

// ---------------------------------------------------------------------------
// RpcClientError — thrown when a request fails
// ---------------------------------------------------------------------------

/**
 * Thrown by `RpcClient.request()` when the server responds with an error,
 * or when the connection is closed before a response arrives.
 *
 * @example
 *     try {
 *       const result = client.request("divide", { a: 1, b: 0 });
 *     } catch (err) {
 *       if (err instanceof RpcClientError) {
 *         console.error(`RPC error ${err.code}: ${err.message}`);
 *       }
 *     }
 */
export class RpcClientError extends Error {
  /** The error code from the server's RpcErrorResponse. */
  readonly code: number;
  /** Optional extra data attached to the server's error response. */
  readonly data?: unknown;

  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = "RpcClientError";
    this.code = code;
    this.data = data;
  }
}

// ---------------------------------------------------------------------------
// RpcClient
// ---------------------------------------------------------------------------

/**
 * A codec-agnostic RPC client.
 *
 * Sends requests to a remote server and waits synchronously for matching
 * responses. Handles server-push notifications via registered handlers.
 *
 * @typeParam V - The codec's native dynamic value type (e.g., `unknown` for
 *               a JSON codec). Request params and response results are of
 *               this type.
 *
 * @example
 *     const client = new RpcClient(codec, framer);
 *
 *     // Register a handler for server-push notifications:
 *     client.onNotification("heartbeat", (params) => {
 *       console.log("Server heartbeat received", params);
 *     });
 *
 *     // Make a synchronous request:
 *     const result = client.request("ping");
 *     console.log(result); // whatever the server returned
 */
export class RpcClient<V> {
  private readonly codec: RpcCodec<V>;
  private readonly framer: RpcFramer;

  /**
   * Monotonically increasing request id counter.
   * Starts at 1; incremented before each use.
   */
  private nextId: number = 0;

  /** Map from notification method name to handler. */
  private readonly notificationHandlers: Map<
    string,
    (params: V | undefined) => void
  >;

  /**
   * Construct a new `RpcClient`.
   *
   * @param codec  - Translates between `RpcMessage<V>` and `Uint8Array`.
   * @param framer - Reads and writes discrete byte frames from/to the stream.
   */
  constructor(codec: RpcCodec<V>, framer: RpcFramer) {
    this.codec = codec;
    this.framer = framer;
    this.notificationHandlers = new Map();
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Register a handler for server-push notifications.
   *
   * While `request()` is blocked waiting for a response, the server may send
   * unsolicited notifications (e.g., log events, diagnostics). Register
   * handlers here to receive them. Unregistered methods are silently dropped.
   *
   * Registering the same method name twice replaces the earlier handler.
   * Returns `this` for chaining.
   *
   * @param method  - The notification method name to listen for.
   * @param handler - Called with the notification params.
   *
   * @example
   *     client
   *       .onNotification("$/logMessage", (params) => {
   *         const p = params as { message: string };
   *         console.log("[server]", p.message);
   *       })
   *       .onNotification("$/progress", (params) => updateProgress(params));
   */
  onNotification(
    method: string,
    handler: (params: V | undefined) => void,
  ): this {
    this.notificationHandlers.set(method, handler);
    return this;
  }

  /**
   * Send a request to the server and wait for the matching response.
   *
   * Generates a new unique id, encodes the request, writes it through the
   * framer, then reads frames until a response with a matching id arrives.
   * While waiting, any server-push notifications are dispatched to their
   * registered handlers.
   *
   * @param method - The procedure to call.
   * @param params - Optional arguments to pass to the procedure.
   * @returns The result value from the server's response.
   *
   * @throws {RpcClientError} If the server responds with an error.
   * @throws {RpcClientError} If the connection is closed before a response arrives.
   *
   * @example
   *     const sum = client.request("add", { a: 5, b: 3 } as unknown as V);
   *     console.log(sum); // 8 (as V)
   */
  request(method: string, params?: V): V {
    // Allocate a new unique id for this in-flight request.
    const id: RpcId = ++this.nextId;

    // Encode and write the request frame.
    const reqMsg: RpcMessage<V> = { kind: "request", id, method, params };
    const reqBytes = this.codec.encode(reqMsg);
    this.framer.writeFrame(reqBytes);

    // Read frames until we find the response that matches our id.
    while (true) {
      const frameBytes = this.framer.readFrame();

      if (frameBytes === null) {
        // Clean EOF before we got our response — the server hung up.
        throw new RpcClientError(
          RpcErrorCodes.InternalError,
          "Connection closed before response received",
        );
      }

      // Decode the frame. If the codec fails, skip this frame and keep waiting.
      let msg: RpcMessage<V>;
      try {
        msg = this.codec.decode(frameBytes);
      } catch {
        // Undecodable frame — not our response. Keep waiting.
        continue;
      }

      switch (msg.kind) {
        case "response":
          if (msg.id === id) {
            // This is our response. Return the result.
            return msg.result;
          }
          // Response for a different in-flight request (in concurrent use).
          // In synchronous single-request-at-a-time mode this shouldn't happen,
          // but we skip it gracefully rather than crashing.
          break;

        case "error":
          if (msg.id === id) {
            // The server sent an error for our request. Throw it.
            throw new RpcClientError(msg.code, msg.message, msg.data);
          }
          // Error for a different request — skip.
          break;

        case "notification":
          // Server-push notification received while we were waiting.
          // Dispatch to the registered handler (if any) then continue waiting.
          this.dispatchNotification(msg.method, msg.params);
          break;

        case "request":
          // Server sent us a request (bidirectional RPC). In client-only mode,
          // we don't have handlers for server-initiated requests — skip.
          break;
      }
    }
  }

  /**
   * Send a fire-and-forget notification to the server.
   *
   * No response is expected or waited for. The notification is encoded and
   * written to the framer immediately. This method returns as soon as the
   * frame is written.
   *
   * @param method - The notification method name.
   * @param params - Optional payload.
   *
   * @example
   *     client.notify("textDocument/didSave", { uri: "file:///src/main.ts" } as unknown as V);
   */
  notify(method: string, params?: V): void {
    const notifMsg: RpcMessage<V> = { kind: "notification", method, params };
    const bytes = this.codec.encode(notifMsg);
    this.framer.writeFrame(bytes);
  }

  // -------------------------------------------------------------------------
  // Private helpers
  // -------------------------------------------------------------------------

  /**
   * Dispatch a server-push notification to its registered handler.
   *
   * Unknown methods are silently dropped. Handler errors are silently
   * swallowed — a notification handler must not disrupt the request/response
   * correlation loop.
   */
  private dispatchNotification(method: string, params: V | undefined): void {
    const handler = this.notificationHandlers.get(method);
    if (!handler) return;
    try {
      handler(params);
    } catch {
      // Swallow — notification handler errors must not interrupt request flow.
    }
  }
}
