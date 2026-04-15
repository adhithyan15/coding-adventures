/**
 * RpcFramer — the byte-boundary interface
 *
 * A framer answers one question: "where does one message end and the next
 * begin?" On a continuous byte stream (like a TCP connection or a Unix pipe),
 * bytes flow without inherent boundaries. The framer imposes those boundaries
 * so the codec can receive exactly one payload at a time.
 *
 * Analogy: imagine text messages sent over a telegraph wire. The raw wire
 * carries a continuous stream of dots and dashes. A framer is the convention
 * that says "STOP" marks the end of one sentence. Without that convention,
 * you cannot tell where one message ends and the next begins.
 *
 * Where does the framer sit in the stack?
 * ----------------------------------------
 *
 *   RpcCodec    — "what does this payload mean?"
 *       ↕  Uint8Array   ← framer provides/consumes exactly this
 *   RpcFramer   — "where does one payload end and the next begin?"
 *       ↕  raw byte stream
 *   Transport   — stdin/stdout, TCP socket, Unix socket, pipe …
 *
 * Framing schemes
 * ---------------
 *
 *   ContentLengthFramer — `Content-Length: N\r\n\r\n` + N payload bytes.
 *                          Used by LSP (Language Server Protocol).
 *
 *   LengthPrefixFramer  — 4-byte big-endian N + N payload bytes.
 *                          Compact; used in many binary RPC systems.
 *
 *   NewlineFramer       — payload bytes + `\n`.
 *                          Used by NDJSON (Newline-Delimited JSON) streams.
 *
 *   WebSocketFramer     — wraps payload in a WebSocket data frame.
 *                          Used when the transport is a WebSocket connection.
 *
 *   PassthroughFramer   — reads the entire stream as one frame; no envelope.
 *                          Used when HTTP or another outer protocol handles
 *                          framing externally (e.g., HTTP request body).
 *
 * Error handling
 * --------------
 * `readFrame` returns `null` on clean EOF (the remote closed the connection
 * gracefully). On a framing error (e.g., a malformed Content-Length header),
 * it should throw an `RpcError` with code `-32700` (ParseError).
 *
 * The framer does NOT throw on EOF — returning `null` is the clean signal
 * that there is no more data.
 */

/**
 * Reads and writes discrete byte chunks from a raw byte stream.
 *
 * The framer knows nothing about the content of the chunks — it only concerns
 * itself with boundaries. A single framer instance holds a reference to the
 * underlying transport (stream) and is NOT safe for concurrent use.
 *
 * @example
 *     // A simple newline framer (pseudocode):
 *     class NewlineFramer implements RpcFramer {
 *       readFrame(): Uint8Array | null {
 *         const line = this.stream.readLine(); // returns null on EOF
 *         return line === null ? null : Buffer.from(line, "utf8");
 *       }
 *       writeFrame(data: Uint8Array): void {
 *         this.stream.write(data);
 *         this.stream.write("\n");
 *       }
 *     }
 */
export interface RpcFramer {
  /**
   * Read the next frame payload from the stream.
   *
   * A "frame" is a single discrete byte chunk — exactly the bytes that the
   * codec will receive. The framer has already stripped any envelope (length
   * prefix, delimiter, header, etc.) before returning.
   *
   * @returns The payload bytes of the next frame, or `null` on clean EOF.
   *
   * @throws {RpcError} with code `-32700` on a malformed frame envelope.
   *
   * @example
   *     while (true) {
   *       const frame = framer.readFrame();
   *       if (frame === null) break;  // EOF — remote closed connection
   *       const msg = codec.decode(frame);
   *       // handle msg …
   *     }
   */
  readFrame(): Uint8Array | null;

  /**
   * Write one frame to the stream.
   *
   * The framer wraps `data` in whatever envelope its framing scheme requires
   * (e.g., prepends a Content-Length header, appends a newline) and then
   * writes the complete envelope + payload to the underlying stream.
   *
   * @param data - The raw payload bytes to frame and write.
   *
   * @example
   *     const bytes = codec.encode(responseMsg);
   *     framer.writeFrame(bytes);
   *     // The remote side will receive exactly `bytes` from its readFrame()
   */
  writeFrame(data: Uint8Array): void;
}
