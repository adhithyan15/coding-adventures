defmodule CodingAdventures.JsonRpc.Errors do
  @moduledoc """
  Standard JSON-RPC 2.0 error codes and constructor helpers.

  ## The Standard Error Code Table

  JSON-RPC 2.0 reserves a specific set of integer error codes. These are
  modelled after HTTP status codes — they tell the client *why* the request
  failed without requiring the client to parse the error message string.

  | Code    | Name              | When to use                                           |
  |---------|-------------------|-------------------------------------------------------|
  | -32700  | Parse error       | The framed bytes are not valid JSON                   |
  | -32600  | Invalid Request   | Valid JSON but not a valid JSON-RPC Request object    |
  | -32601  | Method not found  | No handler registered for the requested method        |
  | -32602  | Invalid params    | The handler rejected the params as malformed          |
  | -32603  | Internal error    | An unexpected error occurred inside the handler       |
  | -32000 to -32099 | Server errors | Reserved for implementation-specific server errors  |

  ## LSP-Reserved Range

  The Language Server Protocol reserves `-32899` to `-32800` for its own error
  codes (e.g., `ContentModified = -32801`). The JSON-RPC layer defined here
  does NOT use that range — it belongs to the LSP layer that sits on top.

  ## Usage

      error = Errors.method_not_found("textDocument/hover")
      # => %{code: -32601, message: "Method not found", data: "textDocument/hover"}

      error = Errors.internal_error("unexpected nil from handler")
      # => %{code: -32603, message: "Internal error", data: "unexpected nil from handler"}
  """

  # ---------------------------------------------------------------------------
  # Error code constants
  # ---------------------------------------------------------------------------
  #
  # We use module attributes so the constants are inlined at compile time and
  # appear in documentation.

  @doc "JSON body could not be parsed (-32700)."
  @spec parse_error() :: integer()
  def parse_error(), do: -32_700

  @doc "Valid JSON but not a valid JSON-RPC Request (-32600)."
  @spec invalid_request() :: integer()
  def invalid_request(), do: -32_600

  @doc "No handler registered for the method (-32601)."
  @spec method_not_found() :: integer()
  def method_not_found(), do: -32_601

  @doc "The handler rejected the method parameters as invalid (-32602)."
  @spec invalid_params() :: integer()
  def invalid_params(), do: -32_602

  @doc "An unexpected error occurred inside the handler (-32603)."
  @spec internal_error() :: integer()
  def internal_error(), do: -32_603

  # ---------------------------------------------------------------------------
  # ResponseError constructors
  # ---------------------------------------------------------------------------
  #
  # Each constructor returns a plain map (not a struct) so it can be directly
  # JSON-encoded. The `data` field is optional — if nil, it is omitted from
  # the encoded output.

  @doc """
  Build a parse-error ResponseError map.

  ## Example

      Errors.make_parse_error(nil)
      # => %{code: -32700, message: "Parse error"}

      Errors.make_parse_error("unexpected token at byte 42")
      # => %{code: -32700, message: "Parse error", data: "unexpected token at byte 42"}
  """
  @spec make_parse_error(any()) :: map()
  def make_parse_error(data \\ nil) do
    make_error(parse_error(), "Parse error", data)
  end

  @doc """
  Build an invalid-request ResponseError map.

  ## Example

      Errors.make_invalid_request("missing 'jsonrpc' field")
      # => %{code: -32600, message: "Invalid Request", data: "missing 'jsonrpc' field"}
  """
  @spec make_invalid_request(any()) :: map()
  def make_invalid_request(data \\ nil) do
    make_error(invalid_request(), "Invalid Request", data)
  end

  @doc """
  Build a method-not-found ResponseError map.

  ## Example

      Errors.make_method_not_found("textDocument/hover")
      # => %{code: -32601, message: "Method not found", data: "textDocument/hover"}
  """
  @spec make_method_not_found(any()) :: map()
  def make_method_not_found(data \\ nil) do
    make_error(method_not_found(), "Method not found", data)
  end

  @doc """
  Build an invalid-params ResponseError map.

  ## Example

      Errors.make_invalid_params("expected object, got null")
  """
  @spec make_invalid_params(any()) :: map()
  def make_invalid_params(data \\ nil) do
    make_error(invalid_params(), "Invalid params", data)
  end

  @doc """
  Build an internal-error ResponseError map.

  ## Example

      Errors.make_internal_error("handler crashed")
  """
  @spec make_internal_error(any()) :: map()
  def make_internal_error(data \\ nil) do
    make_error(internal_error(), "Internal error", data)
  end

  # ---------------------------------------------------------------------------
  # Private: generic ResponseError builder
  # ---------------------------------------------------------------------------
  #
  # A ResponseError always has `code` and `message`. The `data` field is only
  # included when the caller provides it (non-nil), keeping the wire format
  # lean for the common case.

  defp make_error(code, message, nil) do
    %{code: code, message: message}
  end

  defp make_error(code, message, data) do
    %{code: code, message: message, data: data}
  end
end
