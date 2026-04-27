/**
 * server.ts -- LspServer: the main coordinator
 *
 * LspServer wires together:
 *   - The LanguageBridge (language-specific logic)
 *   - The DocumentManager (tracks open file contents)
 *   - The ParseCache (avoids redundant parses)
 *   - The JSON-RPC Server (protocol layer)
 *
 * It registers all LSP request and notification handlers with the JSON-RPC
 * server, then calls serve() to start the blocking read-dispatch-write loop.
 *
 * # Server Lifecycle
 *
 *   Client (editor)              Server (us)
 *     |                               |
 *     |--initialize-------------->    |  store clientInfo, return capabilities
 *     | <-----------------result-     |
 *     |                               |
 *     |--initialized (notif)----->    |  no-op (handshake complete)
 *     |                               |
 *     |--textDocument/didOpen---->    |  open doc, parse, push diagnostics
 *     |--textDocument/didChange-->    |  apply change, re-parse, push diagnostics
 *     |--textDocument/hover------>    |  get parse result, call bridge.hover
 *     | <-----------------result-     |
 *     |                               |
 *     |--shutdown---------------->    |  set shutdown flag, return null
 *     |--exit (notif)------------>    |  process.exit(0) or process.exit(1)
 *
 * # Sending Notifications to the Editor
 *
 * The JSON-RPC Server handles request/response pairs. But the LSP server also
 * needs to PUSH notifications to the editor (e.g., textDocument/publishDiagnostics).
 * We do this by holding a reference to the JSON-RPC MessageWriter and calling
 * writeMessage directly.
 *
 * @module
 */

import { Readable, Writable } from "node:stream";
import { Server, MessageWriter } from "@coding-adventures/json-rpc";
import type { LanguageBridge } from "./language-bridge.js";
import { DocumentManager } from "./document-manager.js";
import { ParseCache } from "./parse-cache.js";
import type { HandlerContext } from "./handlers.js";
import {
  handleInitialize,
  handleInitialized,
  handleShutdown,
  handleExit,
  handleDidOpen,
  handleDidChange,
  handleDidClose,
  handleDidSave,
  handleHover,
  handleDefinition,
  handleReferences,
  handleCompletion,
  handleRename,
  handleDocumentSymbol,
  handleSemanticTokensFull,
  handleFoldingRange,
  handleSignatureHelp,
  handleFormatting,
} from "./handlers.js";

// ---------------------------------------------------------------------------
// LspServer
// ---------------------------------------------------------------------------

/**
 * LspServer is the main LSP server.
 *
 * Create it with `new LspServer(bridge, inStream, outStream)`, then call
 * `serve()` to start serving. It is designed to be used once per process --
 * start it, it blocks, it exits.
 *
 * @example
 *     const server = new LspServer(myBridge, process.stdin, process.stdout);
 *     await server.serve();
 */
export class LspServer {
  private readonly rpcServer: Server;
  private readonly writer: MessageWriter;
  private readonly ctx: HandlerContext;

  /** Whether the editor has sent "shutdown". */
  private shutdown = false;
  /** Whether the initialize handshake is complete. */
  private initialized = false;

  constructor(bridge: LanguageBridge, inStream: Readable, outStream: Writable) {
    this.rpcServer = new Server(inStream, outStream);
    this.writer = new MessageWriter(outStream);

    // Build the handler context -- a bag of shared state that every handler
    // can access without needing a reference to the LspServer itself.
    this.ctx = {
      bridge,
      docManager: new DocumentManager(),
      parseCache: new ParseCache(),
      sendNotification: (method: string, params: unknown) => {
        this.writer.writeMessage({
          type: "notification",
          method,
          params,
        });
      },
      isInitialized: () => this.initialized,
      setInitialized: (v: boolean) => { this.initialized = v; },
      isShutdown: () => this.shutdown,
      setShutdown: (v: boolean) => { this.shutdown = v; },
    };

    this.registerHandlers();
  }

  /**
   * Start the blocking JSON-RPC read-dispatch-write loop.
   *
   * This call blocks (awaits) until the editor closes the connection (EOF on stdin).
   * All LSP messages are handled synchronously in this loop.
   */
  async serve(): Promise<void> {
    await this.rpcServer.serve();
  }

  /**
   * Wire all LSP method names to their handler functions.
   *
   * Requests (have an id, get a response):
   *   initialize, shutdown, textDocument/hover, textDocument/definition,
   *   textDocument/references, textDocument/completion, textDocument/rename,
   *   textDocument/documentSymbol, textDocument/semanticTokens/full,
   *   textDocument/foldingRange, textDocument/signatureHelp,
   *   textDocument/formatting
   *
   * Notifications (no id, no response):
   *   initialized, textDocument/didOpen, textDocument/didChange,
   *   textDocument/didClose, textDocument/didSave
   */
  private registerHandlers(): void {
    const ctx = this.ctx;

    // -- Lifecycle --
    this.rpcServer.onRequest("initialize", (id, params) =>
      handleInitialize(ctx, id, params));
    this.rpcServer.onNotification("initialized", (params) =>
      handleInitialized(ctx, params));
    this.rpcServer.onRequest("shutdown", (id, params) =>
      handleShutdown(ctx, id, params));
    this.rpcServer.onNotification("exit", (params) =>
      handleExit(ctx, params));

    // -- Text document synchronization --
    this.rpcServer.onNotification("textDocument/didOpen", (params) =>
      handleDidOpen(ctx, params));
    this.rpcServer.onNotification("textDocument/didChange", (params) =>
      handleDidChange(ctx, params));
    this.rpcServer.onNotification("textDocument/didClose", (params) =>
      handleDidClose(ctx, params));
    this.rpcServer.onNotification("textDocument/didSave", (params) =>
      handleDidSave(ctx, params));

    // -- Feature requests --
    this.rpcServer.onRequest("textDocument/hover", (id, params) =>
      handleHover(ctx, id, params));
    this.rpcServer.onRequest("textDocument/definition", (id, params) =>
      handleDefinition(ctx, id, params));
    this.rpcServer.onRequest("textDocument/references", (id, params) =>
      handleReferences(ctx, id, params));
    this.rpcServer.onRequest("textDocument/completion", (id, params) =>
      handleCompletion(ctx, id, params));
    this.rpcServer.onRequest("textDocument/rename", (id, params) =>
      handleRename(ctx, id, params));
    this.rpcServer.onRequest("textDocument/documentSymbol", (id, params) =>
      handleDocumentSymbol(ctx, id, params));
    this.rpcServer.onRequest("textDocument/semanticTokens/full", (id, params) =>
      handleSemanticTokensFull(ctx, id, params));
    this.rpcServer.onRequest("textDocument/foldingRange", (id, params) =>
      handleFoldingRange(ctx, id, params));
    this.rpcServer.onRequest("textDocument/signatureHelp", (id, params) =>
      handleSignatureHelp(ctx, id, params));
    this.rpcServer.onRequest("textDocument/formatting", (id, params) =>
      handleFormatting(ctx, id, params));
  }
}
