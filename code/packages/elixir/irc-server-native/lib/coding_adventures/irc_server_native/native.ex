defmodule CodingAdventures.IrcServerNative.Native do
  @moduledoc """
  NIF stubs for the Rust `irc_server_native` cdylib, which embeds the all-Rust
  `irc-net-reactor` IRC engine.

  Each function body (`:erlang.nif_error(:not_loaded)`) is replaced at module
  load by the real Rust function via `@on_load`. The cdylib's `nif_init/0`
  declares its module as `Elixir.CodingAdventures.IrcServerNative.Native`, which
  MUST match this module's name exactly or the BEAM refuses to load the NIF.

  All IRC and TCP logic runs in Rust; Elixir only launches and controls the
  server. There is no callback into Elixir.
  """

  @on_load :load_nif

  @doc false
  def load_nif do
    priv_dir = :code.priv_dir(:coding_adventures_irc_server_native)
    nif_path = Path.join(priv_dir, "irc_server_native")
    :erlang.load_nif(to_charlist(nif_path), 0)
  end

  @doc """
  Build a server resource bound to `host:port`.

  `motd` is a single newline-joined binary (the Rust side splits it into lines).
  Returns an opaque resource, or raises `ArgumentError` on bad arguments.
  """
  def new_server(_host, _port, _server_name, _motd, _oper_password, _max_connections),
    do: :erlang.nif_error(:not_loaded)

  @doc "Run the event loop in the calling process, blocking until stopped (dirty I/O)."
  def server_serve(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Spawn a Rust thread to run the event loop; returns immediately."
  def server_serve_background(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Signal the loop to stop and join the background thread."
  def server_stop(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Whether the loop is currently running (boolean)."
  def server_running(_server), do: :erlang.nif_error(:not_loaded)

  @doc "The bound IP address (binary)."
  def server_local_host(_server), do: :erlang.nif_error(:not_loaded)

  @doc "The bound TCP port (integer)."
  def server_local_port(_server), do: :erlang.nif_error(:not_loaded)
end
