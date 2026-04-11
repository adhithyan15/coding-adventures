/**
 * MessageWriter — writes Content-Length-framed JSON-RPC messages to a stream
 *
 * Framing format:
 *
 *     Content-Length: <n>\r\n
 *     \r\n
 *     <UTF-8 JSON payload, exactly n bytes>
 *
 * The `Content-Length` value is the BYTE length of the UTF-8-encoded payload,
 * not the character count. For ASCII-only JSON these are always the same, but
 * multi-byte Unicode characters (e.g. in string values) can make them differ.
 *
 * @example
 *     const writer = new MessageWriter(process.stdout);
 *     writer.writeMessage({ type: "response", id: 1, result: { ok: true } });
 *
 * Why no buffering?
 * -----------------
 * Each call to `writeMessage` writes a complete, self-contained framed message.
 * There is no need to buffer across calls because the Content-Length header
 * already tells the reader where each message ends.
 */

import { Writable } from "node:stream";
import { messageToObject, type Message } from "./message.js";

/**
 * Writes one JSON-RPC message at a time to a `Writable` stream, applying
 * Content-Length framing.
 */
export class MessageWriter {
  private readonly stream: Writable;

  constructor(stream: Writable) {
    this.stream = stream;
  }

  // -------------------------------------------------------------------------
  // Public API
  // -------------------------------------------------------------------------

  /**
   * Serialize `msg` to JSON and write it with Content-Length framing.
   *
   * @example
   *     writer.writeMessage({
   *       type: "response",
   *       id: 1,
   *       result: { contents: { kind: "plaintext", value: "INC" } },
   *     });
   */
  writeMessage(msg: Message): void {
    const obj = messageToObject(msg);
    const json = JSON.stringify(obj);
    this.writeRaw(json);
  }

  /**
   * Write a pre-serialized JSON string with Content-Length framing.
   *
   * Use this when you already have the JSON text and don't need message
   * parsing — for example, in tests or proxies.
   *
   * @example
   *     writer.writeRaw('{"jsonrpc":"2.0","id":1,"result":null}');
   */
  writeRaw(json: string): void {
    // Encode to UTF-8 bytes first so we count bytes, not characters.
    // A multi-byte Unicode character like "€" (U+20AC) is 3 bytes in UTF-8
    // but only 1 character — Content-Length must reflect the byte count.
    const payload = Buffer.from(json, "utf8");
    const header = `Content-Length: ${payload.length}\r\n\r\n`;
    const headerBytes = Buffer.from(header, "ascii");

    // Write header and payload as a single buffer to prevent interleaving if
    // multiple writers ever coexist (even though serve() is single-threaded).
    this.stream.write(Buffer.concat([headerBytes, payload]));
  }
}
