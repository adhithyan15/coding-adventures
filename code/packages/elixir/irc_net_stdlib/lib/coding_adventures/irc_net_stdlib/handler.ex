defmodule CodingAdventures.IrcNetStdlib.Handler do
  @moduledoc """
  Behaviour defining the callback interface for IRC connection lifecycle events.

  The `EventLoop` calls these three callbacks as connections open, deliver data,
  and close. Implementations must handle all three.

  ## Thread safety

  All three callbacks are invoked from within the `EventLoop` GenServer via
  `dispatch/2`. Because GenServers process one message at a time, implementations
  are **automatically serialised** -- no additional locking is required.

  ## Type alias

  `Handler.t()` is a module atom -- the behaviour is implemented by modules,
  not instances. This is the standard Elixir behaviour pattern.
  """

  @type conn_id :: non_neg_integer()

  @typedoc "A module that implements the Handler behaviour."
  @type t :: module()

  @doc """
  Called once when a new TCP connection is established.

  ## Parameters

  - `conn_id` -- Unique integer identifying this connection.
  - `host`    -- The peer's IP address as a string (e.g. "127.0.0.1").
  """
  @callback on_connect(conn_id :: conn_id(), host :: String.t()) :: any()

  @doc """
  Called each time raw bytes arrive from *conn_id*.

  The bytes may be a partial IRC message, multiple complete messages, or
  anything in between. The handler is responsible for framing.

  ## Parameters

  - `conn_id` -- Which connection sent the data.
  - `data`    -- Raw bytes (binary), never empty.
  """
  @callback on_data(conn_id :: conn_id(), data :: binary()) :: any()

  @doc """
  Called once when *conn_id* has closed (either end may have initiated).

  After this call, `conn_id` is invalid. Any subsequent `send_to` calls with
  this `conn_id` are silent no-ops.

  ## Parameters

  - `conn_id` -- The connection that closed.
  """
  @callback on_disconnect(conn_id :: conn_id()) :: any()
end
