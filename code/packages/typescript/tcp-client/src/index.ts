/**
 * # tcp-client
 *
 * A TCP client with buffered I/O and configurable timeouts for Node.js.
 *
 * This package wraps Node.js `net.Socket` with ergonomic defaults for
 * building network clients. It is **protocol-agnostic** -- it knows nothing
 * about HTTP, SMTP, or Redis. It just moves bytes reliably between two
 * machines. Higher-level packages build application protocols on top.
 *
 * ## Analogy: A telephone call
 *
 * ```text
 * Making a TCP connection is like making a phone call:
 *
 * 1. DIAL (DNS + connect)
 *    Look up "Grandma" -> 555-0123     (DNS resolution)
 *    Dial and wait for ring            (TCP three-way handshake)
 *    If nobody picks up -> hang up      (connect timeout)
 *
 * 2. TALK (read/write)
 *    Say "Hello, Grandma!"             (writeAll + flush)
 *    Listen for response               (readLine)
 *    If silence for 30s -> "Still there?" (read timeout)
 *
 * 3. HANG UP (shutdown/close)
 *    Say "Goodbye" and hang up         (shutdownWrite + close)
 * ```
 *
 * ## Where it fits
 *
 * ```text
 * url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
 *                         |
 *                    raw byte stream
 * ```
 *
 * ## Key difference from the Rust version
 *
 * Node.js TCP is event-based and asynchronous, so all I/O methods return
 * Promises. The Rust version uses blocking I/O with OS-level timeouts; here
 * we use event listeners and `setTimeout` to achieve the same semantics.
 *
 * ## Example
 *
 * ```typescript
 * import { connect } from "@coding-adventures/tcp-client";
 *
 * const conn = await connect("info.cern.ch", 80);
 * await conn.writeAll("GET / HTTP/1.0\r\nHost: info.cern.ch\r\n\r\n");
 * await conn.flush();
 * const statusLine = await conn.readLine();
 * console.log(statusLine);
 * conn.close();
 * ```
 *
 * @module
 */

import * as net from "net";

// ============================================================================
// Version
// ============================================================================

export const VERSION = "0.1.0";

// ============================================================================
// Error hierarchy
// ============================================================================
//
// Each error class corresponds to a specific TCP failure mode. This lets
// callers use `instanceof` to decide how to recover:
//
// ```text
// TcpError (base)
//   +-- DnsResolutionFailed   hostname could not be resolved
//   +-- ConnectionRefusedError server up, nothing listening on that port
//   +-- TimeoutError          connect/read/write took too long
//   +-- ConnectionResetError  remote side crashed (TCP RST)
//   +-- BrokenPipeError       tried to write after remote closed
//   +-- UnexpectedEofError    connection closed before expected data arrived
// ```

/**
 * Base class for all TCP-related errors.
 *
 * All specific error types extend this, so you can catch `TcpError` to
 * handle any TCP failure generically, or catch a specific subclass for
 * fine-grained control.
 */
export class TcpError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TcpError";
  }
}

/**
 * DNS lookup failed -- the hostname could not be resolved to an IP address.
 *
 * Common causes:
 * - Typo in the hostname ("exmaple.com")
 * - No internet connection
 * - DNS server is down
 */
export class DnsResolutionFailed extends TcpError {
  constructor(
    public host: string,
    message: string,
  ) {
    super(`DNS resolution failed for '${host}': ${message}`);
    this.name = "DnsResolutionFailed";
  }
}

/**
 * The server is reachable but nothing is listening on the requested port.
 *
 * This means the TCP handshake received a RST (reset) packet from the
 * server. The machine is up, but no process has bound to that port.
 */
export class ConnectionRefusedError extends TcpError {
  constructor(public addr: string) {
    super(`connection refused by ${addr}`);
    this.name = "ConnectionRefusedError";
  }
}

/**
 * An operation took longer than the configured timeout.
 *
 * The `phase` field tells you which operation timed out:
 * - "connect" -- the TCP handshake
 * - "read" -- waiting for data from the server
 * - "write" -- waiting for the OS send buffer to drain
 *
 * The `duration` field is the timeout value in milliseconds.
 */
export class TimeoutError extends TcpError {
  constructor(
    public phase: string,
    public duration: number,
  ) {
    super(`${phase} timed out after ${duration}ms`);
    this.name = "TimeoutError";
  }
}

/**
 * The remote side reset the connection unexpectedly (TCP RST during transfer).
 *
 * Unlike ConnectionRefused (RST during handshake), this happens mid-conversation.
 * The server process likely crashed or was killed.
 */
export class ConnectionResetError extends TcpError {
  constructor() {
    super("connection reset by peer");
    this.name = "ConnectionResetError";
  }
}

/**
 * Tried to write to a connection the remote side already closed.
 *
 * This is the TCP equivalent of "talking into a dead phone line."
 * The remote end sent a FIN (or RST) and is no longer reading.
 */
export class BrokenPipeError extends TcpError {
  constructor() {
    super("broken pipe (remote closed)");
    this.name = "BrokenPipeError";
  }
}

/**
 * The connection closed before the expected number of bytes arrived.
 *
 * For example, if you call `readExact(100)` but the server sends 50 bytes
 * and then closes the connection, you get this error with
 * `expected = 100, received = 50`.
 */
export class UnexpectedEofError extends TcpError {
  constructor(
    public expected: number,
    public received: number,
  ) {
    super(`unexpected EOF: expected ${expected} bytes, got ${received}`);
    this.name = "UnexpectedEofError";
  }
}

// ============================================================================
// ConnectOptions -- configuration for establishing a connection
// ============================================================================

/**
 * Configuration for establishing a TCP connection.
 *
 * All timeouts default to 30000 milliseconds (30 seconds). The buffer size
 * defaults to 8192 bytes (8 KiB).
 *
 * ## Why separate timeouts?
 *
 * ```text
 * connectTimeout (30s) -- how long to wait for the TCP handshake
 *   If a server is down or firewalled, the OS might wait minutes.
 *
 * readTimeout (30s) -- how long to wait for data after calling read
 *   Without this, a stalled server hangs your program forever.
 *
 * writeTimeout (30s) -- how long to wait for the OS send buffer
 *   Usually instant, but blocks if the remote side isn't reading.
 * ```
 */
export interface ConnectOptions {
  /** Maximum time in ms to wait for the TCP handshake. Default: 30000. */
  connectTimeout?: number;
  /** Maximum time in ms to wait for data on read. Default: 30000. */
  readTimeout?: number;
  /** Maximum time in ms to wait on write. Default: 30000. */
  writeTimeout?: number;
  /** Size of internal read buffer in bytes. Default: 8192. */
  bufferSize?: number;
}

/** Default values for ConnectOptions, extracted for reuse. */
const DEFAULT_CONNECT_TIMEOUT = 30_000;
const DEFAULT_READ_TIMEOUT = 30_000;
const DEFAULT_WRITE_TIMEOUT = 30_000;
const DEFAULT_BUFFER_SIZE = 8192;

// ============================================================================
// mapSocketError -- translate Node.js error codes to typed errors
// ============================================================================

/**
 * Map a Node.js socket error to the most specific TcpError variant.
 *
 * Node.js socket errors have a `.code` property set by the OS:
 *
 * ```text
 * ECONNREFUSED  -> ConnectionRefusedError  (nothing listening)
 * ETIMEDOUT     -> TimeoutError            (OS-level timeout)
 * ECONNRESET    -> ConnectionResetError    (remote crashed)
 * EPIPE         -> BrokenPipeError         (write after close)
 * ENOTFOUND     -> DnsResolutionFailed     (hostname not found)
 * ECONNABORTED  -> ConnectionResetError    (connection aborted)
 * ```
 */
function mapSocketError(err: NodeJS.ErrnoException, host?: string): TcpError {
  switch (err.code) {
    case "ECONNREFUSED":
      return new ConnectionRefusedError(err.address ?? host ?? "unknown");
    case "ETIMEDOUT":
      return new TimeoutError("connect", 0);
    case "ECONNRESET":
    case "ECONNABORTED":
      return new ConnectionResetError();
    case "EPIPE":
      return new BrokenPipeError();
    case "ENOTFOUND":
    case "EAI_AGAIN":
      return new DnsResolutionFailed(host ?? "unknown", err.message);
    default:
      return new TcpError(err.message);
  }
}

// ============================================================================
// connect() -- establish a TCP connection
// ============================================================================

/**
 * Establish a TCP connection to the given host and port.
 *
 * ## Algorithm
 *
 * ```text
 * 1. Create a net.Socket and call connect({host, port})
 * 2. Set a connect timeout -- if the handshake doesn't complete in time,
 *    destroy the socket and reject with TimeoutError
 * 3. On 'connect' event, clear the timeout and resolve with TcpConnection
 * 4. On 'error' event, map the error code to a typed TcpError
 * ```
 *
 * Node.js handles DNS resolution internally (via libuv/c-ares), so we
 * don't need to resolve addresses manually like the Rust version.
 *
 * ## Example
 *
 * ```typescript
 * const conn = await connect("example.com", 80, { connectTimeout: 5000 });
 * ```
 */
export async function connect(
  host: string,
  port: number,
  options?: ConnectOptions,
): Promise<TcpConnection> {
  const connectTimeout = options?.connectTimeout ?? DEFAULT_CONNECT_TIMEOUT;
  const readTimeout = options?.readTimeout ?? DEFAULT_READ_TIMEOUT;
  const writeTimeout = options?.writeTimeout ?? DEFAULT_WRITE_TIMEOUT;
  const bufferSize = options?.bufferSize ?? DEFAULT_BUFFER_SIZE;

  return new Promise<TcpConnection>((resolve, reject) => {
    const socket = net.createConnection({ host, port });

    // --- Connect timeout ---
    // If the handshake doesn't complete within the timeout, we destroy
    // the socket and reject. This mirrors Rust's connect_timeout behavior.
    const timer = setTimeout(() => {
      socket.destroy();
      reject(new TimeoutError("connect", connectTimeout));
    }, connectTimeout);

    // --- Connection established ---
    socket.once("connect", () => {
      clearTimeout(timer);
      resolve(
        new TcpConnection(socket, {
          readTimeout,
          writeTimeout,
          bufferSize,
        }),
      );
    });

    // --- Connection error ---
    socket.once("error", (err: NodeJS.ErrnoException) => {
      clearTimeout(timer);
      reject(mapSocketError(err, host));
    });
  });
}

// ============================================================================
// TcpConnection -- buffered I/O over a TCP stream
// ============================================================================

/** Internal options passed from connect() to TcpConnection. */
interface ConnectionInternalOptions {
  readTimeout: number;
  writeTimeout: number;
  bufferSize: number;
}

/**
 * A TCP connection with buffered I/O and configured timeouts.
 *
 * Wraps a `net.Socket` with an internal read buffer for efficient
 * line-oriented or chunk-oriented communication.
 *
 * ## Why buffered I/O?
 *
 * ```text
 * Without buffering:
 *   socket 'data' events arrive in arbitrary chunks: "HT", "TP/", "1.0 2"
 *   You'd need to concatenate them manually for every read operation.
 *
 * With an internal buffer:
 *   All incoming data is pushed to a Buffer.
 *   readLine() scans the buffer for \n.
 *   readExact(n) waits until n bytes are buffered.
 *   This decouples "when data arrives" from "when the caller needs it."
 * ```
 *
 * ## How the buffer works
 *
 * ```text
 * Socket 'data' events:
 *   [chunk1] -> buffer: [chunk1]
 *   [chunk2] -> buffer: [chunk1 + chunk2]
 *
 * readLine() call:
 *   Scan buffer for \n
 *   Found at position 15?
 *     -> Return buffer[0..16], shift buffer to buffer[16..]
 *     -> Return the string
 *   Not found?
 *     -> Wait for more 'data' events, then scan again
 * ```
 *
 * The connection is closed when you call `close()`.
 */
export class TcpConnection {
  /** Internal buffer holding data received from the socket but not yet consumed. */
  private buffer: Buffer;

  /** The underlying Node.js TCP socket. */
  private socket: net.Socket;

  /** Read timeout in milliseconds. */
  private readTimeout: number;

  /** Write timeout in milliseconds. */
  private writeTimeout: number;

  /**
   * Whether the socket has been closed or the remote end has signaled EOF.
   * Once true, no more data will arrive from the network.
   */
  private ended = false;

  /**
   * Stores the first socket error encountered, so we can report it
   * on subsequent operations instead of a generic "closed" error.
   */
  private socketError: TcpError | null = null;

  /**
   * Callback invoked when new data arrives. Set by read operations
   * that are waiting for more data.
   *
   * Only one read operation can be pending at a time (TCP is ordered),
   * so a single callback suffices.
   */
  private dataCallback: (() => void) | null = null;

  /**
   * Callback invoked when the socket emits 'end' (remote closed write half).
   * Read operations use this to detect EOF.
   */
  private endCallback: (() => void) | null = null;

  /** @internal -- created by connect(), not directly by users. */
  constructor(socket: net.Socket, opts: ConnectionInternalOptions) {
    this.socket = socket;
    this.readTimeout = opts.readTimeout;
    this.writeTimeout = opts.writeTimeout;
    this.buffer = Buffer.alloc(0);

    // Disable Nagle's algorithm for lower latency. Without this, Node
    // may buffer small writes and wait for more data before sending,
    // which adds up to 40ms latency per write on some platforms.
    socket.setNoDelay(true);

    // --- Event: 'data' ---
    // Every chunk of data from the network is appended to our buffer.
    // If a read operation is waiting (dataCallback is set), we notify it.
    socket.on("data", (chunk: Buffer) => {
      this.buffer = Buffer.concat([this.buffer, chunk]);
      if (this.dataCallback) {
        const cb = this.dataCallback;
        this.dataCallback = null;
        cb();
      }
    });

    // --- Event: 'end' ---
    // The remote side has closed its write half (sent FIN). No more data
    // will arrive. Any pending read should be notified so it can check
    // the buffer and return what it has (or error).
    socket.on("end", () => {
      this.ended = true;
      if (this.endCallback) {
        const cb = this.endCallback;
        this.endCallback = null;
        cb();
      }
      if (this.dataCallback) {
        const cb = this.dataCallback;
        this.dataCallback = null;
        cb();
      }
    });

    // --- Event: 'error' ---
    // A socket error occurred. Store it and notify any pending operation.
    socket.on("error", (err: NodeJS.ErrnoException) => {
      this.socketError = mapSocketError(err);
      this.ended = true;
      if (this.dataCallback) {
        const cb = this.dataCallback;
        this.dataCallback = null;
        cb();
      }
      if (this.endCallback) {
        const cb = this.endCallback;
        this.endCallback = null;
        cb();
      }
    });

    // --- Event: 'close' ---
    // The socket has been fully closed (both read and write).
    socket.on("close", () => {
      this.ended = true;
    });
  }

  // --------------------------------------------------------------------------
  // Private helper: wait for data or EOF
  // --------------------------------------------------------------------------

  /**
   * Wait until more data arrives or the socket closes/errors.
   *
   * This is the core primitive that all read methods build on. It returns
   * a Promise that resolves when the buffer has new data, or rejects if
   * a timeout or error occurs.
   *
   * ```text
   * waitForData() flow:
   *   1. If already errored -> reject immediately
   *   2. If already ended -> resolve (caller checks buffer)
   *   3. Set a read timeout timer
   *   4. Register dataCallback to resolve when data arrives
   *   5. Register endCallback to resolve on EOF
   * ```
   */
  private waitForData(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      // If there's already a socket error, fail immediately
      if (this.socketError) {
        reject(this.socketError);
        return;
      }

      // If the socket is ended (EOF), resolve so the caller can
      // check the buffer and decide whether that's an error or not.
      if (this.ended) {
        resolve();
        return;
      }

      const timer = setTimeout(() => {
        this.dataCallback = null;
        this.endCallback = null;
        reject(new TimeoutError("read", this.readTimeout));
      }, this.readTimeout);

      // When data arrives, clear the timeout and resolve.
      this.dataCallback = () => {
        clearTimeout(timer);
        this.endCallback = null;
        resolve();
      };

      // When the socket ends, clear the timeout and resolve.
      // The caller will see there's no more data and handle accordingly.
      this.endCallback = () => {
        clearTimeout(timer);
        this.dataCallback = null;
        resolve();
      };
    });
  }

  // --------------------------------------------------------------------------
  // readLine() -- read until \n
  // --------------------------------------------------------------------------

  /**
   * Read bytes until a newline (`\n`) is found.
   *
   * Returns the line as a string, **including** the trailing `\n` (and
   * `\r\n` if present). Returns an empty string at EOF (remote closed).
   *
   * This is the workhorse for line-oriented protocols like HTTP/1.0,
   * SMTP, and RESP (Redis protocol).
   *
   * ## Algorithm
   *
   * ```text
   * loop:
   *   Scan buffer for \n
   *   Found at index i?
   *     -> Extract buffer[0..i+1] as string
   *     -> Shift buffer to buffer[i+1..]
   *     -> Return the string
   *   Not found?
   *     -> Wait for more data (or EOF)
   *     -> If EOF and buffer empty -> return ""
   *     -> If EOF and buffer not empty -> return remaining as string
   * ```
   */
  async readLine(): Promise<string> {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      // Scan the buffer for a newline character (0x0a = '\n')
      const newlineIndex = this.buffer.indexOf(0x0a);

      if (newlineIndex !== -1) {
        // Found a newline: extract everything up to and including it
        const line = this.buffer.subarray(0, newlineIndex + 1).toString("utf-8");
        this.buffer = this.buffer.subarray(newlineIndex + 1);
        return line;
      }

      // No newline yet. If the socket has ended, return what we have.
      if (this.ended) {
        if (this.socketError) {
          throw this.socketError;
        }
        // EOF: return remaining buffer content (may be empty string = EOF)
        const remaining = this.buffer.toString("utf-8");
        this.buffer = Buffer.alloc(0);
        return remaining;
      }

      // Wait for more data from the socket
      await this.waitForData();
    }
  }

  // --------------------------------------------------------------------------
  // readExact(n) -- read exactly n bytes
  // --------------------------------------------------------------------------

  /**
   * Read exactly `n` bytes from the connection.
   *
   * Blocks (asynchronously) until all `n` bytes have been received.
   * Useful for protocols that specify an exact content length
   * (e.g., HTTP Content-Length header).
   *
   * ## Errors
   *
   * - `UnexpectedEofError` if the connection closes before `n` bytes arrive
   * - `TimeoutError` if no data arrives within the read timeout
   */
  async readExact(n: number): Promise<Buffer> {
    while (this.buffer.length < n) {
      // Not enough data yet

      if (this.ended) {
        if (this.socketError) {
          throw this.socketError;
        }
        // EOF before we got enough data
        throw new UnexpectedEofError(n, this.buffer.length);
      }

      await this.waitForData();
    }

    // We have at least n bytes. Extract exactly n.
    const result = Buffer.from(this.buffer.subarray(0, n));
    this.buffer = this.buffer.subarray(n);
    return result;
  }

  // --------------------------------------------------------------------------
  // readUntil(delimiter) -- read until a specific byte
  // --------------------------------------------------------------------------

  /**
   * Read bytes until the given delimiter byte is found.
   *
   * Returns all bytes up to **and including** the delimiter. Useful for
   * protocols with custom delimiters (RESP uses `\r\n`, null-terminated
   * strings use `\0`).
   *
   * ## Algorithm
   *
   * Same as readLine(), but scans for an arbitrary byte instead of `\n`.
   */
  async readUntil(delimiter: number): Promise<Buffer> {
    // eslint-disable-next-line no-constant-condition
    while (true) {
      const delimIndex = this.buffer.indexOf(delimiter);

      if (delimIndex !== -1) {
        // Found the delimiter: extract everything up to and including it
        const result = Buffer.from(this.buffer.subarray(0, delimIndex + 1));
        this.buffer = this.buffer.subarray(delimIndex + 1);
        return result;
      }

      // No delimiter yet. If ended, return what we have.
      if (this.ended) {
        if (this.socketError) {
          throw this.socketError;
        }
        // Return remaining buffer (without delimiter -- it was never found)
        const remaining = Buffer.from(this.buffer);
        this.buffer = Buffer.alloc(0);
        return remaining;
      }

      await this.waitForData();
    }
  }

  // --------------------------------------------------------------------------
  // writeAll(data) -- write all bytes
  // --------------------------------------------------------------------------

  /**
   * Write all bytes to the connection.
   *
   * Wraps `socket.write(data)` in a Promise. The Promise resolves when
   * the data has been flushed to the OS send buffer (not necessarily
   * received by the remote side).
   *
   * ## Errors
   *
   * - `BrokenPipeError` if the remote side has closed the connection
   * - `TimeoutError` if the write takes too long
   */
  async writeAll(data: Buffer | string): Promise<void> {
    if (this.socketError) {
      throw this.socketError;
    }

    return new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        reject(new TimeoutError("write", this.writeTimeout));
      }, this.writeTimeout);

      // Attach a one-time error handler for this specific write.
      // This catches errors that happen during the write (e.g., EPIPE).
      const errorHandler = (err: NodeJS.ErrnoException) => {
        clearTimeout(timer);
        reject(mapSocketError(err));
      };
      this.socket.once("error", errorHandler);

      this.socket.write(data, (err) => {
        clearTimeout(timer);
        this.socket.removeListener("error", errorHandler);
        if (err) {
          reject(mapSocketError(err as NodeJS.ErrnoException));
        } else {
          resolve();
        }
      });
    });
  }

  // --------------------------------------------------------------------------
  // flush() -- ensure all buffered data is sent
  // --------------------------------------------------------------------------

  /**
   * Flush the write buffer, sending all buffered data to the network.
   *
   * Node.js auto-flushes on `socket.write()`, so this is a no-op in most
   * cases. However, if the internal Node.js write buffer is backed up
   * (i.e., `socket.write()` returned `false` due to backpressure), this
   * waits for the 'drain' event before resolving.
   *
   * ## Why have flush() if it's usually a no-op?
   *
   * API compatibility with the Rust version. In Rust, the `BufWriter`
   * holds data in userspace until you explicitly flush. In Node.js,
   * data is handed to the OS immediately, so flush() is essentially free.
   * But callers should still call it to be protocol-correct.
   */
  async flush(): Promise<void> {
    if (this.socket.writableNeedDrain) {
      return new Promise<void>((resolve) => {
        this.socket.once("drain", resolve);
      });
    }
    // Already flushed -- nothing to do
  }

  // --------------------------------------------------------------------------
  // shutdownWrite() -- half-close the connection
  // --------------------------------------------------------------------------

  /**
   * Shut down the write half of the connection (half-close).
   *
   * Signals to the remote side that no more data will be sent. The
   * read half remains open -- you can still receive data.
   *
   * ```text
   * Before shutdownWrite():
   *   Client <-> Server  (full-duplex, both directions open)
   *
   * After shutdownWrite():
   *   Client <- Server   (client can still READ)
   *   Client X Server    (client can no longer WRITE)
   * ```
   *
   * Internally calls `socket.end()`, which sends a TCP FIN packet.
   */
  async shutdownWrite(): Promise<void> {
    return new Promise<void>((resolve) => {
      this.socket.end(() => {
        resolve();
      });
    });
  }

  // --------------------------------------------------------------------------
  // Address methods
  // --------------------------------------------------------------------------

  /**
   * Returns the remote address (host and port) of this connection.
   *
   * Useful for logging, diagnostics, and protocol implementations that
   * need to know the peer's address.
   */
  peerAddr(): { host: string; port: number } {
    return {
      host: this.socket.remoteAddress ?? "unknown",
      port: this.socket.remotePort ?? 0,
    };
  }

  /**
   * Returns the local address (host and port) of this connection.
   *
   * The local port is ephemeral -- assigned by the OS when the
   * connection is established. It's different every time.
   */
  localAddr(): { host: string; port: number } {
    const addr = this.socket.address() as net.AddressInfo;
    return {
      host: addr.address ?? "unknown",
      port: addr.port ?? 0,
    };
  }

  // --------------------------------------------------------------------------
  // close() -- tear down the connection
  // --------------------------------------------------------------------------

  /**
   * Close the connection, releasing all resources.
   *
   * This is a synchronous call that destroys the underlying socket
   * immediately. After calling close(), all subsequent read/write
   * calls will fail.
   */
  close(): void {
    this.ended = true;
    this.socket.destroy();
  }
}
