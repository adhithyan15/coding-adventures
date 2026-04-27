defmodule Ls00.LspErrors do
  @moduledoc """
  LSP-specific error codes.

  The JSON-RPC 2.0 specification reserves error codes in the range [-32768, -32000].
  The LSP specification further reserves [-32899, -32800] for LSP protocol-level
  errors.

  Standard JSON-RPC error codes (from the json_rpc package):

  | Code   | Name              |
  |--------|-------------------|
  | -32700 | ParseError        |
  | -32600 | InvalidRequest    |
  | -32601 | MethodNotFound    |
  | -32602 | InvalidParams     |
  | -32603 | InternalError     |

  LSP-specific error codes are listed below.

  Reference:
  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#errorCodes
  """

  @doc """
  The server has received a request before the initialize handshake was
  completed (-32002).
  """
  def server_not_initialized, do: -32002

  @doc "A generic error code for unknown errors (-32001)."
  def unknown_error_code, do: -32001

  @doc """
  A request failed but not due to a protocol problem (-32803). For example,
  the document requested was not found.
  """
  def request_failed, do: -32803

  @doc "The server cancelled the request (-32802)."
  def server_cancelled, do: -32802

  @doc """
  The document content was modified before the request completed (-32801).
  The client should retry.
  """
  def content_modified, do: -32801

  @doc "The client cancelled the request (-32800)."
  def request_cancelled, do: -32800
end
