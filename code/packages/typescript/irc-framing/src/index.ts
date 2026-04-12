/**
 * irc-framing — Stateful byte-stream-to-line-frame converter.
 *
 * ## The Problem: TCP delivers a byte stream, not messages
 *
 * When you call `read()` on a TCP socket, the operating system hands you
 * however many bytes happen to be available in the kernel's receive buffer.
 * That may be:
 *
 * - Half a message:  `Buffer("NICK ali")`
 * - Exactly one message: `Buffer("NICK alice\r\n")`
 * - Three messages and the start of a fourth
 *
 * IRC solves this with a simple framing convention: every message ends with
 * `\r\n` (carriage return + line feed, ASCII 13 + 10).  The framer's job is
 * to absorb raw byte chunks and emit complete, `\r\n`-stripped lines to the
 * layer above.
 *
 * This package is **pure**.  It touches no sockets or I/O.
 * It is a single stateful buffer transformer.
 *
 * Layer diagram:
 * ```
 * irc-proto   ← receives complete \r\n-stripped Buffers; calls parse()
 *      ↑
 * irc-framing ← THIS PACKAGE: feed(rawBytes) / frames() → Buffer[]
 *      ↑
 * irc-net-*   ← calls socket.on('data') and feeds raw bytes upward
 * ```
 *
 * ## RFC 1459 Maximum Line Length
 *
 * RFC 1459 §2.3 states that a single IRC message MUST NOT exceed 512 bytes
 * **including** the trailing `\r\n`.  That leaves at most 510 bytes of
 * content.  Lines that exceed this limit are silently discarded to prevent
 * memory exhaustion from malformed or malicious clients.
 *
 * ## Usage
 *
 * ```typescript
 * const framer = new Framer();
 * socket.on('data', (chunk: Buffer) => {
 *   framer.feed(chunk);
 *   for (const line of framer.frames()) {
 *     const msg = parse(line.toString('utf-8'));
 *     handleMessage(msg);
 *   }
 * });
 * socket.on('close', () => framer.reset());
 * ```
 */

// RFC 1459 §2.3: maximum line length is 512 bytes including CRLF.
// Content beyond 510 bytes must be discarded.
const MAX_CONTENT_BYTES = 510;

/**
 * Stateful byte-stream-to-line-frame converter.
 *
 * Call {@link feed} with raw bytes from the socket.
 * Call {@link frames} to get an array of complete CRLF-stripped lines.
 *
 * The Framer is **not thread-safe** (but Node.js is single-threaded, so this
 * is fine as long as each connection owns its own `Framer` instance).
 *
 * ## How it works internally
 *
 * The framer owns a single `Buffer` that accumulates incoming bytes.
 * `Buffer.concat()` allocates a new buffer on each call — this is acceptable
 * for IRC traffic volumes where messages are at most 512 bytes each and
 * arrive at human typing speed.  For ultra-high-throughput scenarios a
 * ring-buffer implementation would avoid copies, but that adds complexity
 * without measurable benefit here.
 *
 * The CRLF scan is O(n) in the frame length, which is bounded by 512 bytes,
 * so it is effectively O(1) per message.
 */
export class Framer {
  // The internal accumulation buffer.  We rebuild it on each feed() call by
  // concatenating the existing buffer with new data.  Node's Buffer.concat()
  // handles the memory allocation for us.
  private buf: Buffer = Buffer.alloc(0);

  // ------------------------------------------------------------------
  // Public API
  // ------------------------------------------------------------------

  /**
   * Append *data* to the internal buffer.
   *
   * This should be called immediately after each socket `data` event.
   * Passing an empty Buffer is a safe no-op.
   *
   * @param data Raw bytes received from the network.  May be any length,
   *             including zero.
   */
  feed(data: Buffer): void {
    if (data.length === 0) return;
    // Buffer.concat allocates a new Buffer containing both halves.
    // For IRC traffic (≤512 bytes per message at human typing speed)
    // this is fast enough.
    this.buf = Buffer.concat([this.buf, data]);
  }

  /**
   * Extract and return all complete lines from the buffer, with `\r\n` stripped.
   *
   * Each call scans from the beginning of the internal buffer and collects
   * every complete line it finds.  Partial data (no `\n` yet) is left in
   * the buffer until the next {@link feed}.
   *
   * Lines exceeding 510 bytes of content are **discarded** silently.
   *
   * @returns Array of complete IRC lines (as `Buffer` values) with terminators
   *          stripped.  Returns an empty array if no complete line is available.
   */
  frames(): Buffer[] {
    const result: Buffer[] = [];

    // We loop as long as there is at least one newline character somewhere
    // in the buffer.  indexOf returns -1 when there is no newline, which
    // terminates the loop — meaning we hold the remaining partial data for
    // the next feed().
    while (true) {
      // Locate the first newline (LF) byte in the buffer.
      // IRC mandates CRLF but many clients only send LF.  We handle both
      // by scanning for LF and then peeking at the byte before it to
      // check for a preceding CR.
      const lfPos = this.buf.indexOf(0x0a); // 0x0A = '\n'

      if (lfPos === -1) {
        // No complete line yet.  Stop — the caller will feed more bytes
        // before calling frames() again.
        break;
      }

      // --- Extract the raw line (without the LF) ---
      // If there is a CR immediately before the LF we want to strip that
      // too.  We check lfPos > 0 to avoid an index error when the very
      // first byte in the buffer is \n.
      let contentEnd: number;
      if (lfPos > 0 && this.buf[lfPos - 1] === 0x0d) {
        // 0x0D = '\r'
        // CRLF terminator: content ends one byte before the CR.
        contentEnd = lfPos - 1;
      } else {
        // LF-only terminator: content ends at lfPos.
        contentEnd = lfPos;
      }

      // The raw frame content (bytes before any CR/LF).
      const line = this.buf.slice(0, contentEnd);

      // --- Advance the buffer past the consumed line + terminator ---
      // We remove everything up to and including the LF byte.
      this.buf = this.buf.slice(lfPos + 1);

      // --- Enforce the RFC 1459 maximum line length ---
      // The RFC allows at most 512 bytes per message including CRLF,
      // leaving 510 bytes of actual content.  Lines longer than this
      // are discarded (not yielded) to prevent a client from growing
      // our buffer without bound.
      if (line.length > MAX_CONTENT_BYTES) {
        // Silently drop the overlong frame and continue scanning for
        // the next line.  A real server would disconnect the offending
        // client; the framer layer is not responsible for that policy.
        continue;
      }

      result.push(line);
    }

    return result;
  }

  /**
   * Discard all buffered data.
   *
   * Call this when a connection is closed or restarted so that stale
   * bytes from the old connection cannot bleed into a new one.
   */
  reset(): void {
    // Replacing buf with a fresh empty Buffer is the clearest way to
    // express "all data is gone".
    this.buf = Buffer.alloc(0);
  }

  /**
   * Number of bytes currently held in the internal buffer.
   *
   * Useful for monitoring buffer growth and writing precise unit tests.
   * A value of 0 means the buffer is empty (no partial data pending).
   */
  get bufferSize(): number {
    return this.buf.length;
  }
}
