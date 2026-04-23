defmodule Rpc.Errors do
  @moduledoc """
  Standard RPC error codes and error-map constructor helpers.

  ## The Error Code Table

  These codes are codec-agnostic integers. They come from the JSON-RPC 2.0
  specification but apply equally to any codec — MessagePack, Protobuf, XML.
  Think of them like HTTP status codes: they communicate *why* a request failed
  without requiring the client to parse a human-readable message string.

  | Code              | Name              | When to use                                        |
  |-------------------|-------------------|----------------------------------------------------|
  | `-32700`          | Parse error       | The framed bytes could not be decoded by the codec |
  | `-32600`          | Invalid request   | Decoded bytes but not a valid RPC message          |
  | `-32601`          | Method not found  | No handler registered for the requested method     |
  | `-32602`          | Invalid params    | Handler rejected the params as malformed           |
  | `-32603`          | Internal error    | Unexpected exception inside the handler            |
  | `-32000..-32099`  | Server errors     | Implementation-defined server-specific errors      |

  ## LSP Range (do NOT use here)

  The Language Server Protocol reserves `-32899` to `-32800` for its own error
  codes (e.g., `ContentModified = -32801`). This `rpc` layer must not use that
  range — it belongs to the application layer sitting above `rpc`.

  ## Usage

      iex> Rpc.Errors.make_parse_error()
      %{code: -32700, message: "Parse error"}

      iex> Rpc.Errors.make_method_not_found("textDocument/hover")
      %{code: -32601, message: "Method not found", data: "textDocument/hover"}

      iex> Rpc.Errors.make_internal_error("handler crashed")
      %{code: -32603, message: "Internal error", data: "handler crashed"}
  """

  # ---------------------------------------------------------------------------
  # Error code constants (functions, not module attributes, so they appear in
  # generated docs and can be pattern-matched by callers).
  # ---------------------------------------------------------------------------

  @doc "The framed bytes could not be decoded by the codec (-32700)."
  @spec parse_error() :: integer()
  def parse_error(), do: -32_700

  @doc "Decoded successfully but not a valid RPC request/notification/response (-32600)."
  @spec invalid_request() :: integer()
  def invalid_request(), do: -32_600

  @doc "No handler registered for the requested method (-32601)."
  @spec method_not_found() :: integer()
  def method_not_found(), do: -32_601

  @doc "Handler rejected the parameters as malformed (-32602)."
  @spec invalid_params() :: integer()
  def invalid_params(), do: -32_602

  @doc "An unexpected exception occurred inside the handler (-32603)."
  @spec internal_error() :: integer()
  def internal_error(), do: -32_603

  # ---------------------------------------------------------------------------
  # Error map constructors
  # ---------------------------------------------------------------------------
  #
  # Each constructor returns a plain map rather than a struct. This keeps the
  # error representation codec-neutral — the codec layer (JSON, MessagePack,
  # etc.) is responsible for serializing this map into wire bytes.
  #
  # The map always has `:code` (integer) and `:message` (string). The optional
  # `:data` field is only included when the caller provides a non-nil value,
  # keeping the wire format compact for the common case.

  @doc """
  Build a parse-error map. Used when the framed bytes cannot be decoded.

  ## Examples

      Rpc.Errors.make_parse_error()
      #=> %{code: -32700, message: "Parse error"}

      Rpc.Errors.make_parse_error("unexpected byte 0xFF at offset 12")
      #=> %{code: -32700, message: "Parse error", data: "unexpected byte 0xFF at offset 12"}
  """
  @spec make_parse_error(any()) :: map()
  def make_parse_error(data \\ nil) do
    make_error(parse_error(), "Parse error", data)
  end

  @doc """
  Build an invalid-request map. Used when bytes decoded but are not an RPC message.

  ## Examples

      Rpc.Errors.make_invalid_request("missing method field")
      #=> %{code: -32600, message: "Invalid Request", data: "missing method field"}
  """
  @spec make_invalid_request(any()) :: map()
  def make_invalid_request(data \\ nil) do
    make_error(invalid_request(), "Invalid Request", data)
  end

  @doc """
  Build a method-not-found map. Used when no handler is registered for a method.

  ## Examples

      Rpc.Errors.make_method_not_found("tools/call")
      #=> %{code: -32601, message: "Method not found", data: "tools/call"}
  """
  @spec make_method_not_found(any()) :: map()
  def make_method_not_found(data \\ nil) do
    make_error(method_not_found(), "Method not found", data)
  end

  @doc """
  Build an invalid-params map. Used when handler rejects the parameters.

  ## Examples

      Rpc.Errors.make_invalid_params("expected object, got list")
      #=> %{code: -32602, message: "Invalid params", data: "expected object, got list"}
  """
  @spec make_invalid_params(any()) :: map()
  def make_invalid_params(data \\ nil) do
    make_error(invalid_params(), "Invalid params", data)
  end

  @doc """
  Build an internal-error map. Used when a handler raises an unexpected exception.

  ## Examples

      Rpc.Errors.make_internal_error("handler raised RuntimeError: kaboom")
      #=> %{code: -32603, message: "Internal error", data: "handler raised RuntimeError: kaboom"}
  """
  @spec make_internal_error(any()) :: map()
  def make_internal_error(data \\ nil) do
    make_error(internal_error(), "Internal error", data)
  end

  # ---------------------------------------------------------------------------
  # Private: generic error map builder
  # ---------------------------------------------------------------------------
  #
  # Two clauses: with and without `:data`. When `data` is nil we omit the key
  # entirely so the wire encoding stays compact (the caller does not have to
  # strip nil fields after the fact).

  defp make_error(code, message, nil) do
    %{code: code, message: message}
  end

  defp make_error(code, message, data) do
    %{code: code, message: message, data: data}
  end
end
