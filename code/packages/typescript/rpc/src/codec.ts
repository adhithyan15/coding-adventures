/**
 * RpcCodec — the serialisation interface
 *
 * A codec translates between `RpcMessage<V>` (the typed in-memory
 * representation) and `Uint8Array` (raw bytes ready to hand to a framer).
 *
 * Where does the codec sit in the stack?
 * ---------------------------------------
 *
 *   Application
 *       ↕  RpcMessage<V>          ← codec operates here
 *   RpcCodec  (encode / decode)
 *       ↕  Uint8Array             ← framer operates on these bytes
 *   RpcFramer (readFrame / writeFrame)
 *       ↕  raw byte stream
 *   Transport (stdin/stdout, TCP, Unix socket …)
 *
 * The codec does NOT know about framing (how frames are delimited), and
 * the framer does NOT know about message structure (what the bytes mean).
 * This strict layering means you can swap either independently:
 *
 *   json-rpc       = JsonCodec       + ContentLengthFramer
 *   msgpack-rpc    = MsgpackCodec    + LengthPrefixFramer
 *   json-ws-rpc    = JsonCodec       + WebSocketFramer
 *
 * Statefulness
 * ------------
 * A codec SHOULD be stateless — a single instance can encode and decode
 * concurrently without locks. The `encode` and `decode` methods do not hold
 * any per-connection state. If a codec implementation is stateful for
 * performance reasons (e.g., it pools byte buffers), it must document that
 * it is not safe for concurrent use.
 *
 * Error handling
 * --------------
 * `decode` throws `RpcError` (from `./errors.ts`) on failure:
 *   - Code `-32700` (ParseError) if the bytes are not parseable at all.
 *   - Code `-32600` (InvalidRequest) if the bytes parsed but do not represent
 *     a valid RpcMessage shape.
 *
 * The server's `serve()` loop catches these errors and sends an
 * `RpcErrorResponse` with a null id back to the client.
 */

import type { RpcMessage } from "./message.js";

/**
 * Translates between `RpcMessage<V>` and raw bytes.
 *
 * Implementors only need to provide two methods: `encode` and `decode`.
 * The `RpcServer` and `RpcClient` call these methods; they never touch bytes
 * directly.
 *
 * @typeParam V - The codec's native dynamic value type.
 *               For JSON, use `unknown`. For MessagePack, use a `MsgpackValue`
 *               union. The RPC layer never inspects V — it passes it through.
 *
 * @example
 *     // A minimal JSON codec (simplified):
 *     class JsonCodec implements RpcCodec<unknown> {
 *       encode(msg: RpcMessage<unknown>): Uint8Array {
 *         return Buffer.from(JSON.stringify(toWireObject(msg)), "utf8");
 *       }
 *       decode(data: Uint8Array): RpcMessage<unknown> {
 *         const raw = JSON.parse(Buffer.from(data).toString("utf8"));
 *         return parseWireObject(raw);  // may throw RpcError
 *       }
 *     }
 */
export interface RpcCodec<V> {
  /**
   * Encode an `RpcMessage<V>` into raw bytes.
   *
   * The bytes produced here are passed directly to `RpcFramer.writeFrame`.
   * The codec must not add framing markers (e.g., no Content-Length header,
   * no length prefix) — that is the framer's responsibility.
   *
   * @param msg - The message to encode.
   * @returns A `Uint8Array` containing the serialised payload.
   *
   * @example
   *     const bytes = codec.encode({ kind: 'request', id: 1, method: 'ping' });
   *     // bytes is now the serialised representation, e.g. JSON bytes
   */
  encode(msg: RpcMessage<V>): Uint8Array;

  /**
   * Decode raw bytes (produced by `RpcFramer.readFrame`) into an `RpcMessage<V>`.
   *
   * The bytes here are a single frame payload — framing markers have already
   * been stripped by the framer. The codec should not need to parse any
   * envelope beyond the payload format it owns.
   *
   * @param data - Raw bytes from the framer.
   * @returns The decoded message.
   *
   * @throws {RpcError} with code `-32700` if `data` is not parseable by this codec.
   * @throws {RpcError} with code `-32600` if `data` parsed but is not a valid RpcMessage.
   *
   * @example
   *     const msg = codec.decode(bytes);
   *     // msg.kind is 'request' | 'response' | 'error' | 'notification'
   */
  decode(data: Uint8Array): RpcMessage<V>;
}
