/**
 * @coding-adventures/json-rpc
 *
 * JSON-RPC 2.0 over stdin/stdout with Content-Length framing.
 *
 * This package implements the transport layer beneath the Language Server
 * Protocol (LSP). Any LSP server in this repository delegates all message
 * framing and dispatch to this package, keeping the LSP layer thin.
 *
 * Architecture
 * ------------
 *
 *   stdin  →  MessageReader  →  Server (dispatch)  →  MessageWriter  →  stdout
 *                                      ↓
 *                              onRequest / onNotification handlers
 *
 * Quick start
 * -----------
 *
 *     import { Server } from "@coding-adventures/json-rpc";
 *
 *     new Server(process.stdin, process.stdout)
 *       .onRequest("initialize", (_id, _params) => ({
 *         capabilities: { hoverProvider: true },
 *       }))
 *       .onNotification("textDocument/didOpen", (params) => {
 *         console.error("opened:", (params as any).textDocument.uri);
 *       })
 *       .serve();
 *
 * Public exports
 * --------------
 *
 *   MessageReader   — reads one framed message from a Readable stream
 *   MessageWriter   — writes one framed message to a Writable stream
 *   Server          — combines reader + writer with a dispatch table
 *   parseMessage    — raw JSON object → typed Message (used by MessageReader)
 *   messageToObject — typed Message → plain object (used by MessageWriter)
 *   JsonRpcError    — error thrown by reader/writer on framing failures
 *   ErrorCodes      — standard error code constants (-32700, -32601, etc.)
 *
 * @module
 */

export { ErrorCodes, type ErrorCode } from "./errors.js";
export {
  type Request,
  type Notification,
  type Response,
  type ResponseError,
  type Message,
  parseMessage,
  messageToObject,
  JsonRpcError,
} from "./message.js";
export { MessageReader } from "./reader.js";
export { MessageWriter } from "./writer.js";
export {
  Server,
  type RequestHandler,
  type NotificationHandler,
} from "./server.js";
