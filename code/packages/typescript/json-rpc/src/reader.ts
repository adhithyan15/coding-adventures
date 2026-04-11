/**
 * MessageReader — reads Content-Length-framed JSON-RPC messages from a stream
 *
 * The LSP/JSON-RPC wire format is HTTP-inspired:
 *
 *     Content-Length: 97\r\n
 *     \r\n
 *     {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
 *
 * Why Content-Length framing?
 * ----------------------------
 * JSON has no self-delimiting structure. You cannot tell where one JSON object
 * ends without parsing the whole thing. The Content-Length header solves this:
 * the reader reads headers until it sees the blank line, then reads exactly
 * `Content-Length` bytes — no more, no less — and parses them as JSON. This
 * makes it safe to concatenate messages in a stream without any separator.
 *
 * Implementation notes
 * --------------------
 *
 * Node.js streams can be in two modes:
 *
 *   1. **Flowing mode** (events) — data arrives via `"data"` events
 *   2. **Paused mode** (pull) — you call `read(n)` and get a chunk
 *
 * We use a Promise-based approach that reads in chunks. Because `Readable`
 * does not expose a simple `read(n)` that blocks, we implement `readBytes(n)`
 * ourselves: it drains from an internal buffer and waits for more `"data"`
 * events if the buffer is too small.
 *
 * This produces a clean async/await interface while remaining non-blocking.
 */

import { Readable } from "node:stream";
import { ErrorCodes } from "./errors.js";
import { parseMessage, JsonRpcError, type Message } from "./message.js";

/**
 * Reads one Content-Length-framed JSON-RPC message at a time from a
 * `Readable` stream.
 *
 * @example
 *     const reader = new MessageReader(process.stdin);
 *     let msg: Message | null;
 *     while ((msg = await reader.readMessage()) !== null) {
 *       console.log(msg);
 *     }
 */
export class MessageReader {
  private readonly stream: Readable;
  /** Internal byte buffer accumulated from "data" events. */
  private buffer: Buffer;
  /** True once the stream has emitted "end" or "error". */
  private ended: boolean;
  /** Callbacks waiting for more data in the buffer. */
  private waiters: Array<() => void>;

  constructor(stream: Readable) {
    this.stream = stream;
    this.buffer = Buffer.alloc(0);
    this.ended = false;
    this.waiters = [];

    // Put the stream in flowing mode so data accumulates in our buffer.
    this.stream.on("data", (chunk: Buffer | string) => {
      const bytes =
        typeof chunk === "string" ? Buffer.from(chunk, "utf8") : chunk;
      this.buffer = Buffer.concat([this.buffer, bytes]);
      // Wake up any pending readBytes() calls.
      const pending = this.waiters.splice(0);
      for (const cb of pending) cb();
    });

    this.stream.on("end", () => {
      this.ended = true;
      // Wake up waiters so they can detect EOF.
      const pending = this.waiters.splice(0);
      for (const cb of pending) cb();
    });

    this.stream.on("error", () => {
      this.ended = true;
      const pending = this.waiters.splice(0);
      for (const cb of pending) cb();
    });
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Read the next framed message and return it as a typed `Message`.
   *
   * Returns `null` on clean EOF (stream ended with no partial message).
   * Throws `JsonRpcError(-32700)` on malformed JSON.
   * Throws `JsonRpcError(-32600)` if the JSON is valid but not a message.
   */
  async readMessage(): Promise<Message | null> {
    const raw = await this.readRaw();
    if (raw === null) return null;

    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      throw new JsonRpcError(
        ErrorCodes.ParseError,
        `Parse error: invalid JSON — ${raw.slice(0, 80)}`,
      );
    }

    return parseMessage(parsed);
  }

  /**
   * Read the next framed message and return it as a raw UTF-8 JSON string,
   * without parsing.
   *
   * Useful for testing or for scenarios where the caller wants to control
   * JSON parsing itself.
   *
   * Returns `null` on EOF.
   */
  async readRaw(): Promise<string | null> {
    // Step 1: Read headers until we see the blank line (\r\n\r\n).
    const headerBytes = await this.readUntilBlankLine();
    if (headerBytes === null) return null; // clean EOF before any data

    // Step 2: Parse Content-Length from the header block.
    const headerText = headerBytes.toString("utf8");
    const contentLength = this.parseContentLength(headerText);

    // Step 3: Read exactly contentLength bytes — the JSON payload.
    const payloadBytes = await this.readBytes(contentLength);
    if (payloadBytes === null) {
      throw new JsonRpcError(
        ErrorCodes.ParseError,
        "Parse error: stream ended before payload was complete",
      );
    }

    return payloadBytes.toString("utf8");
  }

  // -------------------------------------------------------------------------
  // Private helpers
  // -------------------------------------------------------------------------

  /**
   * Read bytes until the sequence `\r\n\r\n` (blank line) is found.
   *
   * Returns the bytes BEFORE the blank line (the header block).
   * Returns `null` if the stream ends before any data arrives.
   */
  private async readUntilBlankLine(): Promise<Buffer | null> {
    const sentinel = Buffer.from("\r\n\r\n", "ascii");

    while (true) {
      // Search our buffer for the blank-line sentinel.
      const idx = this.indexOf(this.buffer, sentinel);
      if (idx !== -1) {
        // Found it — extract header bytes and consume through the sentinel.
        const header = this.buffer.slice(0, idx);
        this.buffer = this.buffer.slice(idx + sentinel.length);
        return header;
      }

      // Not found yet. If stream ended with nothing, return null (EOF).
      if (this.ended && this.buffer.length === 0) {
        return null;
      }

      // If stream ended but we have partial data, that is an error — but we
      // surface it in readBytes; here we just return null for clean EOF.
      if (this.ended) {
        return null;
      }

      // Wait for more data.
      await this.waitForData();
    }
  }

  /**
   * Read exactly `n` bytes from the stream.
   *
   * Returns the bytes if available. Returns `null` if the stream ends before
   * `n` bytes are available.
   */
  private async readBytes(n: number): Promise<Buffer | null> {
    while (true) {
      if (this.buffer.length >= n) {
        const chunk = this.buffer.slice(0, n);
        this.buffer = this.buffer.slice(n);
        return chunk;
      }

      if (this.ended) {
        // Not enough bytes left.
        return null;
      }

      await this.waitForData();
    }
  }

  /**
   * Return a promise that resolves the next time data arrives or the stream
   * ends — whichever comes first.
   */
  private waitForData(): Promise<void> {
    return new Promise<void>((resolve) => {
      this.waiters.push(resolve);
    });
  }

  /**
   * Find the first occurrence of `needle` inside `haystack`.
   *
   * Returns the byte offset, or `-1` if not found.
   * We implement this ourselves because `Buffer.indexOf` with a Buffer needle
   * is available but we want the logic explicit for clarity.
   */
  private indexOf(haystack: Buffer, needle: Buffer): number {
    if (needle.length === 0) return 0;
    if (haystack.length < needle.length) return -1;

    outer: for (let i = 0; i <= haystack.length - needle.length; i++) {
      for (let j = 0; j < needle.length; j++) {
        if (haystack[i + j] !== needle[j]) continue outer;
      }
      return i;
    }
    return -1;
  }

  /**
   * Extract the `Content-Length` value from a header block string.
   *
   * The header block looks like:
   *     Content-Length: 97\r\n
   *     Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n
   *
   * We only need the Content-Length line; the rest are ignored.
   */
  private parseContentLength(headers: string): number {
    for (const line of headers.split("\r\n")) {
      const lower = line.toLowerCase();
      if (lower.startsWith("content-length:")) {
        const value = line.slice("content-length:".length).trim();
        const n = parseInt(value, 10);
        if (isNaN(n) || n < 0) {
          throw new JsonRpcError(
            ErrorCodes.ParseError,
            `Parse error: invalid Content-Length value: "${value}"`,
          );
        }
        return n;
      }
    }
    throw new JsonRpcError(
      ErrorCodes.ParseError,
      "Parse error: missing Content-Length header",
    );
  }
}
