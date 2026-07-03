defmodule CodingAdventures.IrcServerNative.Server do
  @moduledoc """
  A high-performance IRC server for the BEAM, backed by the all-Rust
  `irc-net-reactor` engine (on the home-grown kqueue/epoll reactor).

  Every line of IRC and TCP logic runs in Rust; this module only launches and
  controls the server.

      {:ok, server} = CodingAdventures.IrcServerNative.Server.new(port: 0)
      :ok = CodingAdventures.IrcServerNative.Server.serve_background(server)
      # ... connect IRC clients to local_host(server):local_port(server) ...
      :ok = CodingAdventures.IrcServerNative.Server.stop(server)

  The underlying resource is reference-counted by the BEAM; when the last
  reference is garbage-collected the Rust destructor stops and joins the server.
  """

  alias CodingAdventures.IrcServerNative.Native

  @enforce_keys [:resource]
  defstruct [:resource]

  @type t :: %__MODULE__{resource: reference()}

  @default_motd ["Welcome."]

  @doc """
  Build and bind a server.

  Options: `:host` (default `"127.0.0.1"`), `:port` (default `6667`; `0` picks an
  ephemeral port), `:server_name` (default `"irc.local"`), `:motd` (list of
  lines, default `["Welcome."]`), `:oper_password` (default `""`),
  `:max_connections` (default `1024`).
  """
  @spec new(keyword()) :: {:ok, t()}
  def new(opts \\ []) do
    host = Keyword.get(opts, :host, "127.0.0.1")
    port = Keyword.get(opts, :port, 6667)
    server_name = Keyword.get(opts, :server_name, "irc.local")
    motd = Keyword.get(opts, :motd, @default_motd)
    motd = if motd == [], do: @default_motd, else: motd
    oper_password = Keyword.get(opts, :oper_password, "")
    max_connections = Keyword.get(opts, :max_connections, 1024)

    resource =
      Native.new_server(
        to_string(host),
        port,
        to_string(server_name),
        Enum.map_join(motd, "\n", &to_string/1),
        to_string(oper_password),
        max_connections
      )

    {:ok, %__MODULE__{resource: resource}}
  end

  @doc "Run the event loop in the calling process, blocking until `stop/1`."
  @spec serve(t()) :: :ok
  def serve(%__MODULE__{resource: r}), do: Native.server_serve(r)

  @doc "Run the event loop on a background Rust thread; returns immediately."
  @spec serve_background(t()) :: :ok
  def serve_background(%__MODULE__{resource: r}), do: Native.server_serve_background(r)

  @doc "Signal the server to stop and join the background thread."
  @spec stop(t()) :: :ok
  def stop(%__MODULE__{resource: r}), do: Native.server_stop(r)

  @doc "Whether the event loop is currently running."
  @spec running?(t()) :: boolean()
  def running?(%__MODULE__{resource: r}), do: Native.server_running(r) == true

  @doc "The bound IP address."
  @spec local_host(t()) :: String.t()
  def local_host(%__MODULE__{resource: r}), do: Native.server_local_host(r)

  @doc "The bound TCP port (the OS-assigned port when constructed with `port: 0`)."
  @spec local_port(t()) :: non_neg_integer()
  def local_port(%__MODULE__{resource: r}), do: Native.server_local_port(r)

  @doc "The bound `host:port` address."
  @spec local_addr(t()) :: String.t()
  def local_addr(%__MODULE__{} = server),
    do: "#{local_host(server)}:#{local_port(server)}"
end
