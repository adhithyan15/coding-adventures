defmodule CodingAdventures.TcpServer.Connection do
  @moduledoc """
  Per-client connection metadata passed to TCP server handlers.
  """

  defstruct [:id, :peer_addr, :local_addr, read_buffer: <<>>, selected_db: 0]

  @type t :: %__MODULE__{
          id: pos_integer(),
          peer_addr: {String.t(), non_neg_integer()},
          local_addr: {String.t(), non_neg_integer()},
          read_buffer: binary(),
          selected_db: non_neg_integer()
        }
end

defmodule CodingAdventures.TcpServer do
  @moduledoc """
  Protocol-agnostic TCP server with pluggable handlers.
  """

  alias CodingAdventures.TcpServer.Connection

  defstruct host: "127.0.0.1",
            port: 6380,
            backlog: 128,
            buffer_size: 4096,
            handler: nil,
            listener: nil,
            running: false,
            next_connection_id: 1

  @type handler_result :: iodata() | nil | {iodata() | nil, Connection.t()}
  @type handler :: (Connection.t(), binary() -> handler_result()) | (binary() -> handler_result())
  @type t :: %__MODULE__{}

  @doc """
  Create a TCP server configuration.
  """
  @spec new(keyword()) :: t()
  def new(opts \\ []) do
    %__MODULE__{
      host: Keyword.get(opts, :host, "127.0.0.1"),
      port: Keyword.get(opts, :port, 6380),
      backlog: max(1, Keyword.get(opts, :backlog, 128)),
      buffer_size: max(1, Keyword.get(opts, :buffer_size, 4096)),
      handler: Keyword.get(opts, :handler, &echo/2)
    }
  end

  @doc """
  Create a server with a specific handler.
  """
  @spec with_handler(handler(), keyword()) :: t()
  def with_handler(handler, opts \\ []) when is_function(handler) do
    opts |> Keyword.put(:handler, handler) |> new()
  end

  @doc """
  Bind and listen without entering the accept loop.
  """
  @spec start(t()) :: {:ok, t()} | {:error, term()}
  def start(%__MODULE__{running: true} = server), do: {:ok, server}

  def start(%__MODULE__{} = server) do
    opts = [
      :binary,
      active: false,
      reuseaddr: true,
      backlog: server.backlog,
      ip: parse_host(server.host)
    ]

    case :gen_tcp.listen(server.port, opts) do
      {:ok, listener} -> {:ok, %{server | listener: listener, running: true}}
      {:error, reason} -> {:error, map_error(reason)}
    end
  end

  @doc """
  Enter the accept loop. Blocks until the listener is closed.
  """
  @spec serve(t()) :: :ok | {:error, term()}
  def serve(%__MODULE__{} = server) do
    case ensure_started(server) do
      {:ok, started} -> accept_loop(started)
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Start and serve in one blocking call.
  """
  @spec serve_forever(t()) :: :ok | {:error, term()}
  def serve_forever(%__MODULE__{} = server), do: serve(server)

  @doc """
  Close the listener and stop future accepts.
  """
  @spec stop(t()) :: t()
  def stop(%__MODULE__{} = server) do
    if server.listener != nil do
      :gen_tcp.close(server.listener)
    end

    %{server | listener: nil, running: false}
  end

  @doc """
  Invoke the configured handler without using sockets.
  """
  @spec handle(t(), Connection.t(), binary()) :: {binary(), Connection.t()}
  def handle(%__MODULE__{} = server, %Connection{} = connection, data) when is_binary(data) do
    case invoke_handler(server.handler, connection, data) do
      {response, %Connection{} = updated_connection} ->
        {normalize_response(response), updated_connection}

      response ->
        {normalize_response(response), connection}
    end
  end

  @doc """
  Return the bound `{host, port}`, or `nil` before start.
  """
  @spec address(t()) :: {String.t(), non_neg_integer()} | nil
  def address(%__MODULE__{listener: nil}), do: nil

  def address(%__MODULE__{listener: listener}) do
    case :inet.sockname(listener) do
      {:ok, {ip, port}} -> {format_ip(ip), port}
      _ -> nil
    end
  end

  @doc """
  Whether the server struct represents an open listener.
  """
  @spec running?(t()) :: boolean()
  def running?(%__MODULE__{running: running}), do: running

  defimpl String.Chars do
    def to_string(server) do
      status = if CodingAdventures.TcpServer.running?(server), do: "running", else: "stopped"
      "TcpServer(host=#{inspect(server.host)}, port=#{server.port}, status=#{status})"
    end
  end

  defp ensure_started(%__MODULE__{running: true} = server), do: {:ok, server}
  defp ensure_started(%__MODULE__{} = server), do: start(server)

  defp accept_loop(%__MODULE__{listener: listener} = server) do
    case :gen_tcp.accept(listener, 50) do
      {:ok, socket} ->
        id = server.next_connection_id
        Task.start(fn -> client_loop(%{server | next_connection_id: id + 1}, socket, id) end)
        accept_loop(%{server | next_connection_id: id + 1})

      {:error, :timeout} ->
        accept_loop(server)

      {:error, :closed} ->
        :ok

      {:error, reason} ->
        {:error, map_error(reason)}
    end
  end

  defp client_loop(server, socket, id) do
    try do
      connection = %Connection{
        id: id,
        peer_addr: socket_address(:inet.peername(socket)),
        local_addr: socket_address(:inet.sockname(socket))
      }

      do_client_loop(server, socket, connection)
    after
      :gen_tcp.close(socket)
    end
  end

  defp do_client_loop(server, socket, connection) do
    case :gen_tcp.recv(socket, 0, 50) do
      {:ok, data} ->
        {response, updated_connection} = handle(server, connection, data)

        if byte_size(response) > 0 do
          :ok = :gen_tcp.send(socket, response)
        end

        do_client_loop(server, socket, updated_connection)

      {:error, :timeout} ->
        do_client_loop(server, socket, connection)

      {:error, :closed} ->
        :ok

      {:error, _reason} ->
        :ok
    end
  end

  defp echo(_connection, data), do: data

  defp invoke_handler(handler, connection, data) when is_function(handler, 2),
    do: handler.(connection, data)

  defp invoke_handler(handler, _connection, data) when is_function(handler, 1), do: handler.(data)

  defp normalize_response(nil), do: <<>>
  defp normalize_response(response), do: IO.iodata_to_binary(response)

  defp parse_host(host) do
    case :inet.parse_address(String.to_charlist(host)) do
      {:ok, ip} -> ip
      {:error, _} -> {127, 0, 0, 1}
    end
  end

  defp socket_address({:ok, {ip, port}}), do: {format_ip(ip), port}
  defp socket_address(_), do: {"", 0}

  defp format_ip(tuple) do
    tuple
    |> Tuple.to_list()
    |> Enum.join(".")
  end

  defp map_error(:eaddrinuse), do: :address_in_use
  defp map_error(:eacces), do: :permission_denied
  defp map_error(:closed), do: :connection_reset
  defp map_error(:timeout), do: :timeout
  defp map_error(reason), do: reason
end
