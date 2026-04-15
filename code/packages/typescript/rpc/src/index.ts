/**
 * @coding-adventures/rpc
 *
 * Codec-agnostic RPC primitive — the abstract layer that concrete RPC
 * packages (`json-rpc`, `msgpack-rpc`, `protobuf-rpc`, etc.) build on top of.
 *
 * Architecture
 * ------------
 *
 *   Application  (your server or client logic)
 *       ↕  RpcMessage<V>
 *   RpcCodec     — translate between RpcMessage<V> and bytes
 *       ↕  Uint8Array
 *   RpcFramer    — split the byte stream into discrete chunks
 *       ↕  raw byte stream
 *   Transport    — stdin/stdout, TCP, Unix socket, pipe, …
 *
 * Quick start
 * -----------
 *
 *   // 1. Bring a codec and framer (from a concrete rpc package, or your own):
 *   import { JsonCodec, ContentLengthFramer } from "@coding-adventures/json-rpc";
 *
 *   // 2. Create a server:
 *   import { RpcServer } from "@coding-adventures/rpc";
 *   const server = new RpcServer(new JsonCodec(), new ContentLengthFramer(process.stdin, process.stdout));
 *   server
 *     .onRequest("add", (_id, params) => {
 *       const { a, b } = params as { a: number; b: number };
 *       return a + b;
 *     })
 *     .serve();
 *
 *   // 3. Create a client:
 *   import { RpcClient } from "@coding-adventures/rpc";
 *   const client = new RpcClient(new JsonCodec(), new ContentLengthFramer(inStream, outStream));
 *   const result = client.request("add", { a: 3, b: 4 });
 *
 * @module
 */

// Message types — the four shapes that flow through any RPC system.
export type {
  RpcId,
  RpcRequest,
  RpcResponse,
  RpcErrorResponse,
  RpcNotification,
  RpcMessage,
} from "./message.js";

// Codec interface — translate RpcMessage<V> ↔ Uint8Array.
export type { RpcCodec } from "./codec.js";

// Framer interface — read/write discrete byte frames from a stream.
export type { RpcFramer } from "./framer.js";

// Error codes and error class — standard integer codes + throwable wrapper.
export { RpcErrorCodes, RpcError } from "./errors.js";
export type { RpcErrorCode } from "./errors.js";

// Server — codec-agnostic request dispatcher.
export { RpcServer } from "./server.js";
export type { RpcRequestHandler, RpcNotificationHandler } from "./server.js";

// Client — codec-agnostic request sender with blocking response correlation.
export { RpcClient, RpcClientError } from "./client.js";
