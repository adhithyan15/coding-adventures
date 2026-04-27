/**
 * irc-net-stdlib — Node.js TCP event loop for the IRC stack.
 *
 * ## Overview
 *
 * This package provides the **concrete TCP networking layer** for the IRC stack.
 * It is the first of potentially several `irc-net-*` packages, each of which
 * implements the same stable Handler interface using a different I/O strategy:
 *
 * - `irc-net-stdlib` (this package): Node.js `net` module, event-driven I/O,
 *   single-threaded async.  Works well for hundreds of concurrent clients.
 * - `irc-net-cluster` (future): Node.js cluster module for multi-core.
 * - `irc-net-worker` (future): worker_threads for CPU-bound processing.
 *
 * ## Event-driven model
 *
 * Node.js is single-threaded.  Rather than spawning one OS thread per
 * connection (the Python `irc-net-stdlib` approach), we use the Node.js `net`
 * module's event loop:
 *
 * ```
 * server.on('connection', socket => {
 *   socket.on('data', data => { ... });
 *   socket.on('close', () => { ... });
 * });
 * ```
 *
 * This means all callbacks execute on the same thread, sequentially.  The
 * IRC server state machine is therefore naturally safe without any locking.
 *
 * ## Dependency Inversion
 *
 * The `EventLoop` class accepts a `Handler` interface, not a specific IRC
 * implementation.  This means:
 * - `EventLoop` knows nothing about IRC protocol
 * - Tests can inject mock handlers
 * - The actual IRC driver (`ircd`) wires everything together
 *
 * ## Usage
 *
 * ```typescript
 * const loop = new EventLoop();
 * await loop.run("0.0.0.0", 6667, {
 *   onConnect(connId, host) { ... },
 *   onData(connId, data) { ... },
 *   onDisconnect(connId) { ... },
 * });
 * ```
 */

import * as net from "node:net";

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/**
 * Opaque connection identifier.  Assigned by the `EventLoop` when a new
 * TCP connection arrives.  The `Handler` uses this to identify which
 * connection's data/disconnect is being reported.
 *
 * Branded so TypeScript catches accidental mix-ups with arbitrary numbers.
 */
export type ConnId = number & { __connId?: true };

// ---------------------------------------------------------------------------
// Handler interface
// ---------------------------------------------------------------------------

/**
 * Callback interface that the event loop drives.
 *
 * The event loop calls these methods as connection lifecycle events occur.
 * In Node.js, all callbacks execute on the single event-loop thread, so no
 * locking is needed — but callers must not perform blocking operations inside
 * these callbacks (use async/await or callbacks for any I/O).
 *
 * The interface deliberately passes **raw bytes** to `onData`, not parsed
 * `Message` objects.  Framing and parsing happen in the driver layer above
 * this one, keeping `irc-net-stdlib` free of any IRC-specific knowledge.
 */
export interface Handler {
  /**
   * Called once when a new client connects.
   *
   * @param connId  Unique identifier for this connection.
   * @param host    The peer's hostname or IP address string.
   */
  onConnect(connId: ConnId, host: string): void;

  /**
   * Called each time new bytes arrive from `connId`.
   *
   * The bytes may contain a partial IRC message, multiple complete messages,
   * or anything in between — it is the handler's responsibility to buffer and
   * frame them.
   *
   * @param connId  Which connection sent the data.
   * @param data    The raw bytes chunk, never empty.
   */
  onData(connId: ConnId, data: Buffer): void;

  /**
   * Called once when `connId` has closed (either end initiated).
   *
   * After this call the connId is invalid; `sendTo()` with it is a no-op.
   *
   * @param connId  The connection that closed.
   */
  onDisconnect(connId: ConnId): void;
}

// ---------------------------------------------------------------------------
// EventLoop
// ---------------------------------------------------------------------------

/**
 * Node.js `net`-based TCP event loop.
 *
 * Creates a TCP server, accepts connections, and drives a `Handler` with
 * lifecycle callbacks.  Uses Node.js's built-in event-driven I/O — no threads,
 * no explicit locking.
 *
 * ## Lifecycle
 *
 * 1. Caller calls `loop.run(host, port, handler)` — this returns a `Promise`
 *    that resolves when `loop.stop()` is called.
 * 2. Meanwhile, `sendTo(connId, data)` can be called to push data to clients.
 * 3. When the caller wants to shut down: `loop.stop()` closes the server and
 *    all active sockets, causing the `run()` Promise to resolve.
 *
 * ## Connection ID allocation
 *
 * Each accepted connection gets a monotonically increasing `ConnId` starting
 * at 1.  IDs are never reused within a process lifetime, so a stale reference
 * to a closed connection's ID will simply be ignored by `sendTo()`.
 */
export class EventLoop {
  // TCP server socket.  Set during run(), cleared after stop().
  private server: net.Server | null = null;

  // Map from ConnId → socket for all currently-open connections.
  private sockets: Map<ConnId, net.Socket> = new Map();

  // Monotonically increasing connection ID counter.
  private nextConnId: number = 0;

  // Promise resolve function — called by stop() to resolve the run() promise.
  private resolveStop: (() => void) | null = null;

  // ---------------------------------------------------------------------------
  // Public API
  // ---------------------------------------------------------------------------

  /**
   * Start listening on `host:port` and drive `handler` with lifecycle events.
   *
   * Returns a `Promise<void>` that resolves when `stop()` is called.  The
   * caller should `await` this to keep the process running.
   *
   * @param host     IP address to bind (e.g. `"0.0.0.0"` for all interfaces).
   * @param port     TCP port (e.g. `6667`).  Port `0` lets the OS pick a free
   *                 ephemeral port — useful in tests.
   * @param handler  Callback receiver for connection lifecycle events.
   */
  run(host: string, port: number, handler: Handler): Promise<void> {
    return new Promise((resolve, reject) => {
      // Store the resolve function so stop() can trigger it.
      this.resolveStop = resolve;

      const server = net.createServer((socket) => {
        this.handleConnection(socket, handler);
      });

      server.on("error", (err) => {
        // If the server errors before we've stored it, reject the promise.
        // After it's stored, errors are typically non-fatal (e.g., a single
        // bad connection).
        if (!this.server) {
          reject(err);
        }
      });

      server.listen(port, host, () => {
        // Server is listening.  Store the reference so stop() can close it.
        this.server = server;
      });

      this.server = server;
    });
  }

  /**
   * Stop the event loop.
   *
   * Closes the listening socket (no new connections) and destroys all active
   * client sockets.  This causes the `run()` Promise to resolve.
   *
   * Safe to call from within a handler callback or from a signal handler.
   */
  stop(): void {
    // Close the server socket.  This prevents new connections.
    if (this.server) {
      this.server.close();
      this.server = null;
    }

    // Destroy all active client sockets.  Each socket's 'close' event will
    // fire asynchronously, which will call handler.onDisconnect via the
    // 'close' listener we attached in handleConnection().
    for (const socket of this.sockets.values()) {
      socket.destroy();
    }

    // Resolve the run() promise immediately.  We don't wait for all sockets
    // to close — this matches the "stop accepting, return quickly" semantics.
    if (this.resolveStop) {
      this.resolveStop();
      this.resolveStop = null;
    }
  }

  /**
   * Write `data` to connection `connId`.
   *
   * If `connId` no longer exists (connection closed), this is a silent no-op.
   * The caller should not treat absence as an error — it is a normal race
   * condition where the client disconnected between the handler deciding to
   * write and actually calling `sendTo`.
   *
   * @param connId  Which connection to write to.
   * @param data    Raw bytes to send.
   */
  sendTo(connId: ConnId, data: Buffer): void {
    const socket = this.sockets.get(connId);
    if (socket && !socket.destroyed) {
      socket.write(data);
    }
  }

  /**
   * Return the TCP port number this server is currently listening on.
   *
   * Useful in tests when you passed `port=0` to `run()` and need to know
   * which ephemeral port the OS actually assigned.
   *
   * Returns `null` if the server is not currently listening.
   */
  get listenPort(): number | null {
    if (!this.server) return null;
    const addr = this.server.address();
    if (addr && typeof addr === "object") {
      return addr.port;
    }
    return null;
  }

  // ---------------------------------------------------------------------------
  // Internal: connection management
  // ---------------------------------------------------------------------------

  /**
   * Wire up lifecycle callbacks for a newly accepted socket.
   *
   * This method is called once per accepted connection.  It:
   * 1. Assigns a unique `ConnId`.
   * 2. Calls `handler.onConnect(connId, host)`.
   * 3. Sets up `data`, `close`, and `error` event listeners.
   * 4. When the socket closes, calls `handler.onDisconnect(connId)` and
   *    removes the socket from the active map.
   */
  private handleConnection(socket: net.Socket, handler: Handler): void {
    // Allocate a monotonically increasing ConnId for this connection.
    this.nextConnId++;
    const connId = this.nextConnId as ConnId;

    // Cache the peer address before anything can go wrong.
    const host = socket.remoteAddress ?? "unknown";

    // Register the socket before calling onConnect so that any sendTo()
    // calls made from within onConnect have a socket to write to.
    this.sockets.set(connId, socket);

    // Notify the handler of the new connection.
    handler.onConnect(connId, host);

    // ── Data received ──────────────────────────────────────────────────────
    // Node.js calls this handler each time bytes arrive.  The data is
    // passed to the handler for framing and parsing.
    socket.on("data", (data: Buffer) => {
      handler.onData(connId, data);
    });

    // ── Connection closed ──────────────────────────────────────────────────
    // 'close' fires after both 'end' (graceful FIN) and 'error' (abrupt reset).
    // We use 'close' rather than 'end' to handle both cases in one place.
    socket.on("close", () => {
      this.sockets.delete(connId);
      handler.onDisconnect(connId);
    });

    // ── Socket error ───────────────────────────────────────────────────────
    // We must attach an 'error' listener or Node.js will throw unhandled
    // exceptions for connection resets, broken pipes, etc.  The 'close'
    // event fires after 'error', so cleanup happens in the 'close' handler.
    socket.on("error", (_err: Error) => {
      // The error is expected (connection reset, broken pipe, etc.).
      // The 'close' event will fire next and trigger onDisconnect.
      // We don't call handler.onDisconnect here to avoid calling it twice.
    });
  }
}
