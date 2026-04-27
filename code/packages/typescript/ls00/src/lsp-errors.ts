/**
 * lsp-errors.ts -- LSP-specific error codes
 *
 * The JSON-RPC 2.0 specification reserves error codes in the range [-32768, -32000].
 * The LSP specification further reserves [-32899, -32800] for LSP protocol-level errors.
 *
 * Standard JSON-RPC error codes (from the json-rpc package):
 *   -32700  ParseError
 *   -32600  InvalidRequest
 *   -32601  MethodNotFound
 *   -32602  InvalidParams
 *   -32603  InternalError
 *
 * LSP-specific error codes are listed below.
 *
 * Reference: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes
 *
 * @module
 */

export const LspErrorCodes = {
  /**
   * The server has received a request before the initialize handshake was
   * completed. The server must reject any request (other than initialize)
   * before it has been initialized.
   */
  ServerNotInitialized: -32002,

  /** A generic error code for unknown errors. */
  UnknownErrorCode: -32001,

  /**
   * A request failed but not due to a protocol problem.
   * For example, the document requested was not found.
   */
  RequestFailed: -32803,

  /** The server cancelled the request. */
  ServerCancelled: -32802,

  /**
   * The document content was modified before the request completed.
   * The client should retry.
   */
  ContentModified: -32801,

  /** The client cancelled the request. */
  RequestCancelled: -32800,
} as const;
