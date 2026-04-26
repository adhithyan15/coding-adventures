defmodule CodingAdventures.Conduit.Native do
  @moduledoc """
  NIF stubs for the Rust `conduit_native` cdylib.

  Every function in this module has a `_a, _b -> :erlang.nif_error(:not_loaded)`
  body that BEAM replaces at module-load time with the real Rust function via
  `@on_load`. If the NIF library fails to load (e.g. wrong path, ABI mismatch)
  the original `:not_loaded` body runs and you'll get a clear error.

  ## Module name and NIF lookup

  The Rust cdylib's `nif_init/0` declares its Elixir module name as
  `Elixir.CodingAdventures.Conduit.Native` (the BEAM atom form of this
  Elixir module name). The two MUST match exactly — BEAM will refuse to
  load the NIF if they don't.

  ## Why `@on_load`

  Without `@on_load`, you'd have to remember to call `:erlang.load_nif/2`
  in every module that uses the NIF. The `@on_load` callback fires once,
  the first time this module is touched, regardless of which entry point
  triggered the load.
  """

  @on_load :load_nif

  @doc false
  def load_nif do
    priv_dir = :code.priv_dir(:coding_adventures_conduit)
    nif_path = Path.join(priv_dir, "conduit_native")
    :erlang.load_nif(to_charlist(nif_path), 0)
  end

  # ── App construction NIFs ──────────────────────────────────────────────────

  @doc "Allocate a fresh empty native app resource."
  def new_app, do: :erlang.nif_error(:not_loaded)

  @doc "Register a route: method/pattern keyed by `handler_id`."
  def app_add_route(_app, _method, _pattern, _handler_id), do: :erlang.nif_error(:not_loaded)

  @doc "Append a `before` filter handler id."
  def app_add_before(_app, _handler_id), do: :erlang.nif_error(:not_loaded)

  @doc "Append an `after` filter handler id."
  def app_add_after(_app, _handler_id), do: :erlang.nif_error(:not_loaded)

  @doc "Set the not-found handler id (overwrites any previous)."
  def app_set_not_found(_app, _handler_id), do: :erlang.nif_error(:not_loaded)

  @doc "Set the error handler id (overwrites any previous)."
  def app_set_error_handler(_app, _handler_id), do: :erlang.nif_error(:not_loaded)

  @doc "Set a string-valued setting on the app."
  def app_set_setting(_app, _key, _value), do: :erlang.nif_error(:not_loaded)

  @doc "Read a string-valued setting; returns `nil` if absent."
  def app_get_setting(_app, _key), do: :erlang.nif_error(:not_loaded)

  # ── Server lifecycle NIFs ─────────────────────────────────────────────────

  @doc """
  Build a server resource bound to `host:port`.

  `dispatcher_pid` is the gen_server that will receive `{:conduit_request,
  slot_id, handler_id, env_map}` messages from the Rust I/O threads.
  """
  def new_server(_app, _host, _port, _max_conn, _dispatcher_pid),
    do: :erlang.nif_error(:not_loaded)

  @doc """
  Run the server in the calling process, blocking until `server_stop/1`.

  This is a dirty I/O NIF — BEAM moves it onto the dirty I/O thread pool
  so it doesn't starve normal schedulers.
  """
  def server_serve(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Spawn a Rust thread to run the server in the background; returns immediately."
  def server_serve_background(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Signal the server to stop."
  def server_stop(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Return the bound port (useful when port=0 was passed)."
  def server_local_port(_server), do: :erlang.nif_error(:not_loaded)

  @doc "Return a boolean indicating whether the server thread is active."
  def server_running(_server), do: :erlang.nif_error(:not_loaded)

  # ── Request/response slot completion ──────────────────────────────────────

  @doc """
  Signal the Rust side that the Elixir handler for `slot_id` has finished.

  `response` is a 3-tuple `{status, headers_map, body_binary}`, or any
  other term to mean "no override" (e.g. `nil`, `:ok`).
  """
  def respond(_slot_id, _response), do: :erlang.nif_error(:not_loaded)
end
