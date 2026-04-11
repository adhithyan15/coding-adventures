defmodule CodingAdventures.JsonRpc.Message do
  @moduledoc """
  JSON-RPC 2.0 message types and parse/encode helpers.

  ## The Four Message Types

  JSON-RPC 2.0 defines four distinct message shapes. All carry `"jsonrpc":"2.0"`.

  ### Request

  A call from client to server that expects a response. The `id` ties the
  response back to this request.

  ```json
  {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "textDocument/hover",
    "params": {"textDocument": {"uri": "file:///main.bf"}}
  }
  ```

  ### Response (success)

  The server's reply to a Request. `id` matches the originating request.

  ```json
  { "jsonrpc": "2.0", "id": 1, "result": {"contents": "**INC**"} }
  ```

  ### Response (error)

  Sent when the server cannot fulfil the request.

  ```json
  { "jsonrpc": "2.0", "id": 1, "error": {"code": -32601, "message": "Method not found"} }
  ```

  ### Notification

  A one-way message. No `id` field. The server must NOT send a response.

  ```json
  { "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {...} }
  ```

  ## Discriminating Requests vs Notifications

  The sole discriminator is the presence of `"id"`:
  - Has `"id"` → it is a Request.
  - No `"id"` → it is a Notification.

  `parse_message/1` applies this rule automatically.
  """

  alias CodingAdventures.JsonRpc.JsonCodec

  # ---------------------------------------------------------------------------
  # Struct definitions
  # ---------------------------------------------------------------------------
  #
  # We use plain Elixir structs. Using structs (instead of raw maps) gives us:
  # 1. Compile-time documentation via @type
  # 2. Pattern matching on the struct tag (%Request{} etc.)
  # 3. @enforce_keys to catch missing required fields early
  #
  # Note: `id` in a Response is allowed to be nil when the server could not
  # determine the originating request id (e.g. the request was unparseable).

  defmodule Request do
    @moduledoc """
    A JSON-RPC Request message. Has `id`, `method`, and optional `params`.

    ## Example

        %Request{id: 1, method: "initialize", params: %{"rootUri" => nil}}
    """
    @enforce_keys [:id, :method]
    defstruct [:id, :method, :params]

    @type t :: %__MODULE__{
            id: String.t() | integer(),
            method: String.t(),
            params: any()
          }
  end

  defmodule Response do
    @moduledoc """
    A JSON-RPC Response message. Has `id` and either `result` or `error`.

    ## Example (success)

        %Response{id: 1, result: %{"capabilities" => %{}}}

    ## Example (error)

        %Response{id: 1, error: %{code: -32601, message: "Method not found"}}
    """
    @enforce_keys [:id]
    defstruct [:id, :result, :error]

    @type t :: %__MODULE__{
            id: String.t() | integer() | nil,
            result: any(),
            error: map() | nil
          }
  end

  defmodule Notification do
    @moduledoc """
    A JSON-RPC Notification message. Has `method` and optional `params`.
    No `id` field — the server must not send a response.

    ## Example

        %Notification{method: "textDocument/didChange", params: %{"changes" => []}}
    """
    @enforce_keys [:method]
    defstruct [:method, :params]

    @type t :: %__MODULE__{
            method: String.t(),
            params: any()
          }
  end

  # The discriminated union over all message types.
  @type message :: Request.t() | Response.t() | Notification.t()

  # ---------------------------------------------------------------------------
  # parse_message/1 — binary JSON → typed message struct
  # ---------------------------------------------------------------------------
  #
  # Algorithm:
  # 1. Decode the JSON binary to a native map.
  # 2. Verify "jsonrpc" == "2.0" (optional but good practice).
  # 3. Determine message type:
  #    - Has "method" AND "id"? → Request
  #    - Has "method" but no "id"? → Notification
  #    - Has "result" or "error"? → Response
  #    - Otherwise? → Invalid Request error

  @doc """
  Parse a JSON binary string into a typed message struct.

  Returns `{:ok, message}` where message is a `%Request{}`, `%Response{}`, or
  `%Notification{}`, or `{:error, error_map}` where error_map has `code`,
  `message`, and optionally `data`.

  ## Examples

      iex> json = ~s({"jsonrpc":"2.0","id":1,"method":"ping"})
      iex> {:ok, %Request{id: 1, method: "ping"}} = parse_message(json)

      iex> json = ~s({"jsonrpc":"2.0","method":"notify","params":{}})
      iex> {:ok, %Notification{method: "notify"}} = parse_message(json)

      iex> json = ~s({"jsonrpc":"2.0","id":1,"result":42})
      iex> {:ok, %Response{id: 1, result: 42}} = parse_message(json)

      iex> {:error, %{code: -32700}} = parse_message("not json")
  """
  @spec parse_message(binary()) ::
          {:ok, message()} | {:error, map()}
  def parse_message(json) when is_binary(json) do
    case JsonCodec.decode(json) do
      {:error, _reason} ->
        {:error, %{code: -32_700, message: "Parse error", data: "invalid JSON"}}

      {:ok, decoded} when not is_map(decoded) ->
        # Top-level value is not a JSON object — not a valid JSON-RPC message.
        {:error, %{code: -32_600, message: "Invalid Request", data: "expected JSON object"}}

      {:ok, decoded} ->
        parse_map(decoded)
    end
  end

  # ---------------------------------------------------------------------------
  # message_to_map/1 — typed message struct → plain map for encoding
  # ---------------------------------------------------------------------------
  #
  # Converts a struct back to a plain map that `JsonCodec.encode/1` can handle.
  # The `"jsonrpc": "2.0"` field is always injected.

  @doc """
  Convert a typed message struct to a plain map ready for JSON encoding.

  ## Example

      msg = %Response{id: 1, result: %{"ok" => true}}
      map = message_to_map(msg)
      # => %{"jsonrpc" => "2.0", "id" => 1, "result" => %{"ok" => true}}
  """
  @spec message_to_map(message()) :: map()
  def message_to_map(%Request{id: id, method: method, params: params}) do
    base = %{"jsonrpc" => "2.0", "id" => id, "method" => method}
    if params != nil, do: Map.put(base, "params", params), else: base
  end

  def message_to_map(%Response{id: id, result: result, error: error}) do
    base = %{"jsonrpc" => "2.0", "id" => id}

    cond do
      error != nil -> Map.put(base, "error", error)
      true -> Map.put(base, "result", result)
    end
  end

  def message_to_map(%Notification{method: method, params: params}) do
    base = %{"jsonrpc" => "2.0", "method" => method}
    if params != nil, do: Map.put(base, "params", params), else: base
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Classify a decoded map as Request, Response, or Notification.
  defp parse_map(map) when is_map(map) do
    has_method = Map.has_key?(map, "method")
    has_id = Map.has_key?(map, "id")
    has_result = Map.has_key?(map, "result")
    has_error = Map.has_key?(map, "error")

    cond do
      has_method and has_id ->
        {:ok,
         %Request{
           id: map["id"],
           method: map["method"],
           params: map["params"]
         }}

      has_method ->
        {:ok,
         %Notification{
           method: map["method"],
           params: map["params"]
         }}

      has_result or has_error ->
        {:ok,
         %Response{
           id: map["id"],
           result: map["result"],
           error: map["error"]
         }}

      true ->
        {:error,
         %{
           code: -32_600,
           message: "Invalid Request",
           data: "missing 'method', 'result', or 'error' field"
         }}
    end
  end
end
