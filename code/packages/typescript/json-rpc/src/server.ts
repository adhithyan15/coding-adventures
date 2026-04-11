/**
 * Server — JSON-RPC 2.0 request dispatcher
 *
 * The Server combines a MessageReader and MessageWriter with a method dispatch
 * table. It drives the read-dispatch-write loop, isolating the application
 * from all framing, parsing, and error-handling details.
 *
 * Lifecycle
 * ---------
 *
 *   1. Create a Server with input and output streams.
 *   2. Register handlers with `onRequest` and `onNotification`.
 *   3. Call `serve()` — it blocks (awaits) until the stream closes.
 *
 * Dispatch rules
 * --------------
 *
 *   Request received:
 *     - Handler found  → call it; send result or ResponseError as Response
 *     - No handler     → send -32601 (Method not found) Response
 *     - Handler throws → send -32603 (Internal error) Response
 *
 *   Notification received:
 *     - Handler found  → call it; send NOTHING (spec forbids notification responses)
 *     - No handler     → silently ignore (spec says notifications must not generate errors)
 *
 *   Response received:
 *     - Forwarded to the pending-response table (future client-side use)
 *     - In server-only mode, responses from the remote are logged and discarded
 *
 * Concurrency
 * -----------
 *
 * The serve() loop is single-threaded — it processes one message at a time.
 * This is correct for LSP, where editors send one request and wait before
 * sending the next (notifications can arrive at any time but require no reply).
 *
 * @example
 *     const server = new Server(process.stdin, process.stdout);
 *     server
 *       .onRequest("initialize", (_id, _params) => ({ capabilities: {} }))
 *       .onNotification("textDocument/didOpen", (params) => { ... })
 *       .serve();
 */

import { Readable, Writable } from "node:stream";
import { MessageReader } from "./reader.js";
import { MessageWriter } from "./writer.js";
import { ErrorCodes } from "./errors.js";
import {
  type Message,
  type Request,
  type Notification,
  type ResponseError,
  JsonRpcError,
} from "./message.js";

/** Handler for an incoming Request. Returns a result value or a ResponseError. */
export type RequestHandler = (
  id: string | number,
  params: unknown,
) => unknown | ResponseError | Promise<unknown | ResponseError>;

/** Handler for an incoming Notification. Returns nothing. */
export type NotificationHandler = (params: unknown) => void | Promise<void>;

/**
 * JSON-RPC 2.0 server.
 *
 * Reads messages from `inStream`, dispatches them to registered handlers,
 * and writes responses to `outStream`.
 */
export class Server {
  private readonly reader: MessageReader;
  private readonly writer: MessageWriter;

  /** Map from method name to request handler. */
  private readonly requestHandlers: Map<string, RequestHandler>;
  /** Map from method name to notification handler. */
  private readonly notificationHandlers: Map<string, NotificationHandler>;

  constructor(inStream: Readable, outStream: Writable) {
    this.reader = new MessageReader(inStream);
    this.writer = new MessageWriter(outStream);
    this.requestHandlers = new Map();
    this.notificationHandlers = new Map();
  }

  // -------------------------------------------------------------------------
  // Handler registration — chainable for fluent configuration
  // -------------------------------------------------------------------------

  /**
   * Register a handler for a Request method.
   *
   * The handler receives `(id, params)` and must return either:
   *   - A plain value (serialised as the `result` field)
   *   - A `ResponseError` object (serialised as the `error` field)
   *   - A Promise of either of the above
   *
   * @example
   *     server.onRequest("initialize", (_id, _params) => ({
   *       capabilities: { hoverProvider: true }
   *     }));
   */
  onRequest(method: string, handler: RequestHandler): this {
    this.requestHandlers.set(method, handler);
    return this;
  }

  /**
   * Register a handler for a Notification method.
   *
   * The handler receives `params` and returns nothing. Even if it throws,
   * no response is sent — notifications must not generate responses.
   *
   * @example
   *     server.onNotification("textDocument/didOpen", (params) => {
   *       const p = params as { textDocument: { uri: string } };
   *       console.error("opened:", p.textDocument.uri);
   *     });
   */
  onNotification(method: string, handler: NotificationHandler): this {
    this.notificationHandlers.set(method, handler);
    return this;
  }

  // -------------------------------------------------------------------------
  // serve() — the main loop
  // -------------------------------------------------------------------------

  /**
   * Start the server loop.
   *
   * Reads messages one at a time until the stream closes (EOF) or an
   * unrecoverable error occurs. This method returns a Promise that resolves
   * when the loop exits.
   *
   * @example
   *     await server.serve();
   *     // Stream is now closed; process can exit.
   */
  async serve(): Promise<void> {
    while (true) {
      let msg: Message | null;

      try {
        msg = await this.reader.readMessage();
      } catch (err) {
        // Framing or parse error — send an error response and continue.
        // We cannot know the id, so we use null.
        if (err instanceof JsonRpcError) {
          this.sendError(null, err.code, err.message);
        } else {
          this.sendError(null, ErrorCodes.InternalError, "Internal error");
        }
        continue;
      }

      // null means clean EOF — stream closed, exit the loop.
      if (msg === null) break;

      await this.dispatch(msg);
    }
  }

  // -------------------------------------------------------------------------
  // Private dispatch helpers
  // -------------------------------------------------------------------------

  /** Route a parsed message to the appropriate handler. */
  private async dispatch(msg: Message): Promise<void> {
    switch (msg.type) {
      case "request":
        await this.handleRequest(msg);
        break;
      case "notification":
        await this.handleNotification(msg);
        break;
      case "response":
        // Server-only mode: discard incoming responses.
        // A future client implementation would look these up in a pending table.
        break;
    }
  }

  /** Dispatch a Request and write a Response. */
  private async handleRequest(req: Request): Promise<void> {
    const handler = this.requestHandlers.get(req.method);

    if (!handler) {
      // Per spec: unknown method → -32601 Method not found.
      this.sendError(
        req.id,
        ErrorCodes.MethodNotFound,
        `Method not found: ${req.method}`,
      );
      return;
    }

    let result: unknown;
    try {
      result = await handler(req.id, req.params);
    } catch (err) {
      // Handler threw an exception — report as Internal error.
      const message =
        err instanceof Error ? err.message : "Internal server error";
      this.sendError(req.id, ErrorCodes.InternalError, message);
      return;
    }

    // If the handler returned a ResponseError, send an error response.
    if (isResponseError(result)) {
      this.writer.writeMessage({
        type: "response",
        id: req.id,
        error: result,
      });
      return;
    }

    // Otherwise send a success response.
    this.writer.writeMessage({
      type: "response",
      id: req.id,
      result,
    });
  }

  /** Dispatch a Notification. No response is sent regardless of outcome. */
  private async handleNotification(notif: Notification): Promise<void> {
    const handler = this.notificationHandlers.get(notif.method);
    if (!handler) {
      // Silently ignore — spec says no error response for unknown notifications.
      return;
    }

    try {
      await handler(notif.params);
    } catch {
      // Swallow errors — notifications must never produce responses.
    }
  }

  /** Send a JSON-RPC error Response. */
  private sendError(
    id: string | number | null,
    code: number,
    message: string,
    data?: unknown,
  ): void {
    const error: ResponseError = { code, message };
    if (data !== undefined) {
      error.data = data;
    }
    this.writer.writeMessage({
      type: "response",
      id,
      error,
    });
  }
}

// ---------------------------------------------------------------------------
// Type guard
// ---------------------------------------------------------------------------

/**
 * Returns true if `value` looks like a ResponseError.
 *
 * We check for `code` (number) and `message` (string) — the two required
 * fields. This distinguishes a ResponseError from an arbitrary plain object
 * the handler might return.
 */
function isResponseError(value: unknown): value is ResponseError {
  if (typeof value !== "object" || value === null) return false;
  const obj = value as Record<string, unknown>;
  return typeof obj["code"] === "number" && typeof obj["message"] === "string";
}
