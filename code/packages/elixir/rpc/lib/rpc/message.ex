defmodule Rpc.Message do
  @moduledoc """
  Codec-agnostic RPC message types.

  ## The Four Message Types

  An RPC conversation consists of exactly four kinds of messages. Think of a
  telephone call between a client (the caller) and a server (the callee):

  ```
  Client                            Server
    │                                 │
    │── Request(id=1, "ping", …) ──►  │   "please answer question #1"
    │                                 │
    │◄── Response(id=1, result=…) ──  │   "here is the answer to question #1"
    │                                 │
    │── Notification("log", …) ────►  │   "FYI, no reply needed"
    │                                 │
    │◄── ErrorResponse(id=2, …) ────  │   "I couldn't answer question #2"
    │                                 │
  ```

  ### `Rpc.Request`
  The client sends a request when it wants a response. It carries an `id` so
  the matching response can be correlated later. The `id` is either an integer
  or a string (never `nil` for requests). The `method` is a string naming the
  procedure to call. The `params` field carries the arguments (any value, or
  `nil` if there are no arguments).

  ### `Rpc.Response`
  The server sends a response after successfully processing a request. It
  carries the same `id` as the request and a `result` field with the return
  value.

  ### `Rpc.ErrorResponse`
  The server sends an error response when it cannot fulfil a request. It
  carries the same `id` as the request (or `nil` if the request was so
  malformed that its id could not be extracted), plus an integer `code`, a
  human-readable `message`, and an optional `data` field with debug details.

  ### `Rpc.Notification`
  Fire-and-forget. The client (or server) sends a notification when it does not
  need a reply. Notifications have no `id`. The server must not respond to a
  notification — even to report an error.

  ## The Union Type

  The `Rpc.Message.t()` type alias represents any of the four message types:

      @type t ::
        %Rpc.Request{}
        | %Rpc.Response{}
        | %Rpc.ErrorResponse{}
        | %Rpc.Notification{}

  In dynamically-typed Elixir, `params` and `result` and `data` can hold any
  term — a map, a list, a string, an integer, `nil`, etc. The codec (not the
  `rpc` layer) is responsible for mapping between Elixir terms and wire bytes.
  """

  # ---------------------------------------------------------------------------
  # Rpc.Request
  # ---------------------------------------------------------------------------
  #
  # Sent by the client to call a named procedure on the server.
  # - `id`:     Integer or string. Used to correlate the response.
  # - `method`: String naming the procedure (e.g., "initialize", "tools/call").
  # - `params`: Optional arguments. Any Elixir term. `nil` means no params.

  defmodule Request do
    @moduledoc """
    A request from client to server that expects a response.

    Fields:
    - `:id`     — Correlation id. Integer or string. Required.
    - `:method` — Name of the procedure to invoke.
    - `:params` — Arguments for the procedure. Any term, or `nil`.
    """
    @enforce_keys [:id, :method]
    defstruct [:id, :method, :params]

    @typedoc "A request expecting a response."
    @type t :: %__MODULE__{
            id: integer() | String.t(),
            method: String.t(),
            params: term()
          }
  end

  # ---------------------------------------------------------------------------
  # Rpc.Response
  # ---------------------------------------------------------------------------
  #
  # Sent by the server after successfully processing a Request.
  # - `id`:     Must match the `id` of the corresponding Request.
  # - `result`: The return value. Any Elixir term.

  defmodule Response do
    @moduledoc """
    A successful response from server to client.

    Fields:
    - `:id`     — Must match the `:id` of the originating request.
    - `:result` — The procedure's return value. Any term.
    """
    @enforce_keys [:id]
    defstruct [:id, :result]

    @typedoc "A successful response."
    @type t :: %__MODULE__{
            id: integer() | String.t() | nil,
            result: term()
          }
  end

  # ---------------------------------------------------------------------------
  # Rpc.ErrorResponse
  # ---------------------------------------------------------------------------
  #
  # Sent by the server when it cannot fulfil a Request.
  # - `id`:      Matches the originating Request id, or `nil` if the request
  #              was so malformed that the id could not be extracted.
  # - `code`:    Standard integer error code (see `Rpc.Errors`).
  # - `message`: Human-readable explanation.
  # - `data`:    Optional structured debug detail. Any term, or `nil`.

  defmodule ErrorResponse do
    @moduledoc """
    An error response from server to client.

    Fields:
    - `:id`      — Matches the originating request id, or `nil`.
    - `:code`    — Integer error code (see `Rpc.Errors` for standard codes).
    - `:message` — Human-readable error description.
    - `:data`    — Optional debug detail. Any term or `nil`.
    """
    @enforce_keys [:id, :code, :message]
    defstruct [:id, :code, :message, :data]

    @typedoc "An error response."
    @type t :: %__MODULE__{
            id: integer() | String.t() | nil,
            code: integer(),
            message: String.t(),
            data: term()
          }
  end

  # ---------------------------------------------------------------------------
  # Rpc.Notification
  # ---------------------------------------------------------------------------
  #
  # Fire-and-forget. No id, no response.
  # - `method`: String naming the procedure.
  # - `params`: Optional arguments. Any Elixir term, or `nil`.

  defmodule Notification do
    @moduledoc """
    A fire-and-forget notification that does not expect a response.

    Notifications have no `:id`. The receiver must not send any response —
    even if no handler is registered for the method.

    Fields:
    - `:method` — Name of the notification event.
    - `:params` — Arguments for the handler. Any term, or `nil`.
    """
    @enforce_keys [:method]
    defstruct [:method, :params]

    @typedoc "A notification (no response expected)."
    @type t :: %__MODULE__{
            method: String.t(),
            params: term()
          }
  end

  # ---------------------------------------------------------------------------
  # Union type alias
  # ---------------------------------------------------------------------------
  #
  # `Rpc.Message.t()` is a union of all four message structs. This is what
  # `Rpc.Codec` encodes and decodes.

  @typedoc """
  Any RPC message: request, response, error response, or notification.
  """
  @type t ::
          Request.t()
          | Response.t()
          | ErrorResponse.t()
          | Notification.t()
end
