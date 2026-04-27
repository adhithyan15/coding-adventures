defmodule CodingAdventures.TcpClient do
  @moduledoc """
  TCP client with buffered I/O and configurable timeouts.

  This module is part of the coding-adventures monorepo, a ground-up
  implementation of the computing stack from transistors to operating systems.

  ## Analogy: A telephone call

      Making a TCP connection is like making a phone call:

      1. DIAL (DNS + connect)
         Look up "Grandma" -> 555-0123     (DNS resolution)
         Dial and wait for ring            (TCP three-way handshake)
         If nobody picks up -> hang up     (connect timeout)

      2. TALK (read/write)
         Say "Hello, Grandma!"             (write_all + flush)
         Listen for response               (read_line)
         If silence for 30s -> "Still there?" (read timeout)

      3. HANG UP (shutdown/close)
         Say "Goodbye" and hang up         (shutdown_write + close)

  ## Where it fits

      url-parser (NET00) -> tcp-client (NET01, THIS) -> frame-extractor (NET02)
                               |
                          raw byte stream

  ## Architecture

  Elixir's `:gen_tcp` operates in "passive" mode (active: false), which means
  we must explicitly call `:gen_tcp.recv/3` to read data. This mirrors the Rust
  implementation's synchronous BufReader approach.

  The struct holds:
  - `socket`       - the raw `:gen_tcp` socket handle
  - `buffer`       - a binary accumulator for partial reads (our "BufReader")
  - `read_timeout` - milliseconds to wait on recv calls
  - `write_timeout`- milliseconds to wait on send calls (unused by gen_tcp.send
                     on most platforms, but kept for API symmetry)
  - `buffer_size`  - how many bytes to request per recv call

  ## Why a manual buffer?

  `:gen_tcp.recv(socket, 0)` returns whatever the OS has available, which could
  be a partial line like `"HT"` or `"TP/1.0 2"`. To implement `read_line`, we
  need to accumulate chunks until we find `\\n`. The `buffer` field stores
  leftover bytes from previous reads that haven't been consumed yet.

      recv returns: "Hello\\nWor"
                     ^^^^^^^^^
                     read_line consumes "Hello\\n"
                     buffer keeps "Wor" for the next call

  ## Error mapping

  Erlang/OTP returns POSIX-style error atoms. We map them to our domain:

      :nxdomain       -> :dns_resolution_failed   (hostname not found)
      :econnrefused   -> :connection_refused       (nobody listening)
      :timeout        -> :timeout                  (operation took too long)
      :closed         -> :connection_reset         (remote hung up)
      :enotconn       -> :connection_reset         (socket not connected)
      :epipe          -> :broken_pipe              (write after remote close)

  ## Example

      {:ok, conn} = CodingAdventures.TcpClient.connect("info.cern.ch", 80)
      :ok = CodingAdventures.TcpClient.write_all(conn, "GET / HTTP/1.0\\r\\nHost: info.cern.ch\\r\\n\\r\\n")
      :ok = CodingAdventures.TcpClient.flush(conn)
      {:ok, status_line} = CodingAdventures.TcpClient.read_line(conn)
      IO.puts(status_line)
  """

  # ---------------------------------------------------------------------------
  # Struct definition
  # ---------------------------------------------------------------------------
  #
  # The struct is our "TcpConnection" equivalent. It bundles the socket with
  # its configuration and any buffered data that has been received from the
  # network but not yet consumed by the caller.

  defstruct [:socket, :buffer, :read_timeout, :write_timeout, :buffer_size]

  @type t :: %__MODULE__{
          socket: :gen_tcp.socket(),
          buffer: binary(),
          read_timeout: non_neg_integer(),
          write_timeout: non_neg_integer(),
          buffer_size: pos_integer()
        }

  # ---------------------------------------------------------------------------
  # Default configuration
  # ---------------------------------------------------------------------------
  #
  # These mirror the Rust ConnectOptions defaults:
  #   connect_timeout: 30s
  #   read_timeout:    30s
  #   write_timeout:   30s
  #   buffer_size:     8192 bytes (8 KiB)
  #
  # The buffer size balances memory usage against syscall frequency. With 8 KiB,
  # most HTTP headers fit in a single recv call.

  @default_connect_timeout 30_000
  @default_read_timeout 30_000
  @default_write_timeout 30_000
  @default_buffer_size 8192

  # ---------------------------------------------------------------------------
  # connect/3 — establish a TCP connection
  # ---------------------------------------------------------------------------
  #
  # Algorithm:
  #
  #   1. Parse options (timeouts, buffer size)
  #   2. Convert hostname to charlist (Erlang requirement)
  #   3. Call :gen_tcp.connect with passive mode + binary packets
  #   4. Wrap the socket in our struct with an empty buffer
  #
  # :gen_tcp.connect handles DNS resolution internally. If the hostname does
  # not resolve, it returns {:error, :nxdomain}. If the port is unreachable,
  # it returns {:error, :econnrefused}.

  @doc """
  Connect to a TCP server at the given host and port.

  ## Options

  - `:connect_timeout` - milliseconds to wait for handshake (default: 30000)
  - `:read_timeout` - milliseconds to wait on reads (default: 30000)
  - `:write_timeout` - milliseconds to wait on writes (default: 30000)
  - `:buffer_size` - internal read buffer size in bytes (default: 8192)

  ## Examples

      {:ok, conn} = CodingAdventures.TcpClient.connect("127.0.0.1", 8080)
      {:ok, conn} = CodingAdventures.TcpClient.connect("example.com", 80, connect_timeout: 5000)
      {:error, :dns_resolution_failed} = CodingAdventures.TcpClient.connect("nonexistent.invalid", 80)
  """
  @spec connect(String.t(), non_neg_integer(), keyword()) ::
          {:ok, t()} | {:error, atom() | {atom(), term()}}
  def connect(host, port, opts \\ []) do
    # Step 1: Extract configuration from the options keyword list.
    # Keyword.get/3 provides defaults so callers only need to specify
    # the values they want to override.
    connect_timeout = Keyword.get(opts, :connect_timeout, @default_connect_timeout)
    read_timeout = Keyword.get(opts, :read_timeout, @default_read_timeout)
    write_timeout = Keyword.get(opts, :write_timeout, @default_write_timeout)
    buf_size = Keyword.get(opts, :buffer_size, @default_buffer_size)

    # Step 2: Convert the host string to a charlist.
    #
    # Erlang's networking functions expect charlists (single-quoted strings),
    # not Elixir binaries. "example.com" becomes 'example.com'.
    host_charlist = String.to_charlist(host)

    # Step 3: Attempt the TCP connection.
    #
    # Socket options explained:
    #   :binary      - receive data as Elixir binaries (not charlists)
    #   active: false - passive mode; we call recv explicitly
    #   buffer: size - kernel-level receive buffer hint
    #
    # The fourth argument is the connect timeout in milliseconds.
    tcp_opts = [:binary, active: false, buffer: buf_size]

    case :gen_tcp.connect(host_charlist, port, tcp_opts, connect_timeout) do
      {:ok, socket} ->
        # Step 4: Wrap in our struct with an empty read buffer.
        # The buffer starts as an empty binary <<>> because no data has
        # been received yet.
        conn = %__MODULE__{
          socket: socket,
          buffer: <<>>,
          read_timeout: read_timeout,
          write_timeout: write_timeout,
          buffer_size: buf_size
        }

        {:ok, conn}

      {:error, reason} ->
        {:error, map_error(reason)}
    end
  end

  # ---------------------------------------------------------------------------
  # read_line/1 — read bytes until a newline is found
  # ---------------------------------------------------------------------------
  #
  # This is the workhorse for line-oriented protocols (HTTP/1.0, SMTP, RESP).
  #
  # Algorithm:
  #   1. Check if the buffer already contains a '\n'
  #   2. If yes: split at the newline, return the line, keep the remainder
  #   3. If no: recv more data, append to buffer, go to step 1
  #   4. If recv returns 0 bytes (EOF): return whatever is in the buffer
  #
  # The line is returned INCLUDING the trailing '\n' (and '\r\n' if present),
  # matching the Rust implementation's behavior.

  @doc """
  Read a single line from the connection (up to and including `\\n`).

  Returns `{:ok, line}` where `line` includes the trailing newline characters.
  Returns `{:ok, ""}` at EOF (remote closed the connection cleanly).

  ## Examples

      {:ok, "HTTP/1.0 200 OK\\r\\n"} = CodingAdventures.TcpClient.read_line(conn)
  """
  @spec read_line(t()) :: {:ok, String.t()} | {:error, term()}
  def read_line(%__MODULE__{} = conn) do
    do_read_line(conn)
  end

  # Private recursive helper for read_line.
  #
  # Pattern: check buffer for newline -> if found, split and return;
  # if not, recv more data and recurse.
  defp do_read_line(%__MODULE__{buffer: buf, socket: socket, read_timeout: timeout} = conn) do
    case :binary.match(buf, <<?\n>>) do
      {pos, 1} ->
        # Found a newline! Split the buffer at position pos+1.
        #
        # Example: buffer is "Hello\nWorld"
        #   line = "Hello\n"  (bytes 0..pos inclusive, plus the \n itself)
        #   leftover = "World" (everything after)
        line_length = pos + 1
        <<line::binary-size(line_length), leftover::binary>> = buf
        {:ok, {line, %{conn | buffer: leftover}}}

      :nomatch ->
        # No newline yet — need more data from the network.
        case :gen_tcp.recv(socket, 0, timeout) do
          {:ok, data} ->
            # Append newly received data to our buffer and try again.
            # This is O(n) for each append, but in practice lines are short
            # (HTTP headers are typically < 8 KiB total).
            updated_conn = %{conn | buffer: buf <> data}
            do_read_line(updated_conn)

          {:error, :closed} ->
            # Remote closed the connection. Return whatever we have buffered.
            # If the buffer is empty, return "" to signal EOF (matching Rust).
            if byte_size(buf) > 0 do
              {:ok, {buf, %{conn | buffer: <<>>}}}
            else
              {:ok, {"", conn}}
            end

          {:error, reason} ->
            {:error, map_error(reason)}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # read_exact/2 — read exactly n bytes
  # ---------------------------------------------------------------------------
  #
  # Blocks until all n bytes have been received. Useful for protocols that
  # specify an exact content length (e.g., HTTP Content-Length header).
  #
  # Algorithm:
  #   1. If buffer has >= n bytes: split and return
  #   2. If buffer has < n bytes: recv(socket, remaining, timeout)
  #      :gen_tcp.recv(socket, n) blocks until exactly n bytes arrive
  #   3. On EOF before n bytes: return :unexpected_eof

  @doc """
  Read exactly `count` bytes from the connection.

  Returns `{:ok, data}` where `data` is a binary of exactly `count` bytes.
  Returns `{:error, :unexpected_eof}` if the connection closes before
  enough bytes arrive.

  ## Examples

      {:ok, <<0, 1, 2, 3, 4>>} = CodingAdventures.TcpClient.read_exact(conn, 5)
  """
  @spec read_exact(t(), non_neg_integer()) :: {:ok, {binary(), t()}} | {:error, term()}
  def read_exact(%__MODULE__{} = conn, count) do
    do_read_exact(conn, count)
  end

  defp do_read_exact(%__MODULE__{buffer: buf, socket: socket, read_timeout: timeout} = conn, count) do
    buffered = byte_size(buf)

    cond do
      # Case 1: We already have enough data in the buffer.
      # Split off exactly `count` bytes and keep the rest.
      buffered >= count ->
        <<result::binary-size(count), leftover::binary>> = buf
        {:ok, {result, %{conn | buffer: leftover}}}

      # Case 2: Need more data from the network.
      # Ask for exactly the remaining bytes. :gen_tcp.recv(socket, n)
      # with n > 0 blocks until exactly n bytes are available.
      true ->
        remaining = count - buffered

        case :gen_tcp.recv(socket, remaining, timeout) do
          {:ok, data} ->
            # Combine buffer + new data, then split off what we need.
            combined = buf <> data
            <<result::binary-size(count), leftover::binary>> = combined
            {:ok, {result, %{conn | buffer: leftover}}}

          {:error, :closed} ->
            {:error, :unexpected_eof}

          {:error, reason} ->
            {:error, map_error(reason)}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # read_until/2 — read until a delimiter byte is found
  # ---------------------------------------------------------------------------
  #
  # Similar to read_line but with an arbitrary delimiter byte instead of '\n'.
  # Useful for protocols with custom delimiters:
  #   - RESP (Redis) uses \r\n
  #   - Null-terminated strings use \0
  #   - Netstrings use ':'
  #
  # Returns all bytes up to AND including the delimiter.

  @doc """
  Read bytes until the given delimiter byte is found.

  Returns `{:ok, data}` where `data` includes the delimiter byte.
  Returns `{:ok, data}` with whatever is buffered if the connection
  closes before the delimiter is found.

  ## Examples

      {:ok, "key:value\\0"} = CodingAdventures.TcpClient.read_until(conn, 0)
  """
  @spec read_until(t(), byte()) :: {:ok, {binary(), t()}} | {:error, term()}
  def read_until(%__MODULE__{} = conn, delimiter) when is_integer(delimiter) do
    do_read_until(conn, <<delimiter>>)
  end

  defp do_read_until(%__MODULE__{buffer: buf, socket: socket, read_timeout: timeout} = conn, delimiter_bin) do
    case :binary.match(buf, delimiter_bin) do
      {pos, 1} ->
        # Found the delimiter. Return everything up to and including it.
        split_at = pos + 1
        <<result::binary-size(split_at), leftover::binary>> = buf
        {:ok, {result, %{conn | buffer: leftover}}}

      :nomatch ->
        # Delimiter not in buffer yet — recv more data and try again.
        case :gen_tcp.recv(socket, 0, timeout) do
          {:ok, data} ->
            updated_conn = %{conn | buffer: buf <> data}
            do_read_until(updated_conn, delimiter_bin)

          {:error, :closed} ->
            # Return whatever we have, even without the delimiter.
            if byte_size(buf) > 0 do
              {:ok, {buf, %{conn | buffer: <<>>}}}
            else
              {:ok, {<<>>, conn}}
            end

          {:error, reason} ->
            {:error, map_error(reason)}
        end
    end
  end

  # ---------------------------------------------------------------------------
  # write_all/2 — write data to the connection
  # ---------------------------------------------------------------------------
  #
  # :gen_tcp.send/2 is already "write all" — it blocks until all bytes are
  # sent or an error occurs. Unlike Rust's BufWriter, there's no separate
  # flush step needed because :gen_tcp.send pushes data directly to the OS
  # send buffer.
  #
  # We keep flush/1 as a no-op for API compatibility with the Rust version.

  @doc """
  Write all bytes to the connection.

  Returns `:ok` on success, `{:error, reason}` on failure.

  ## Examples

      :ok = CodingAdventures.TcpClient.write_all(conn, "GET / HTTP/1.0\\r\\n")
  """
  @spec write_all(t(), iodata()) :: :ok | {:error, term()}
  def write_all(%__MODULE__{socket: socket}, data) do
    case :gen_tcp.send(socket, data) do
      :ok -> :ok
      {:error, reason} -> {:error, map_error(reason)}
    end
  end

  # ---------------------------------------------------------------------------
  # flush/1 — flush write buffer (no-op in Elixir)
  # ---------------------------------------------------------------------------
  #
  # :gen_tcp.send already pushes data to the OS. There is no userspace write
  # buffer to flush. This function exists for API parity with the Rust version,
  # where BufWriter requires an explicit flush.

  @doc """
  Flush the write buffer.

  This is a no-op in the Elixir implementation because `:gen_tcp.send/2`
  writes directly to the OS send buffer. It exists for API compatibility
  with the Rust version.
  """
  @spec flush(t()) :: :ok
  def flush(%__MODULE__{}), do: :ok

  # ---------------------------------------------------------------------------
  # shutdown_write/1 — half-close the connection
  # ---------------------------------------------------------------------------
  #
  # Signals to the remote side that we are done writing. The read half stays
  # open, so we can still receive data. This is used in protocols where the
  # client signals "I'm done sending" and then waits for a final response.
  #
  #   Before shutdown_write():
  #     Client <-> Server  (full-duplex, both directions open)
  #
  #   After shutdown_write():
  #     Client <- Server   (client can still READ)
  #     Client X  Server   (client can no longer WRITE)

  @doc """
  Shut down the write half of the connection (half-close).

  The read half remains open. Returns `:ok` on success.
  """
  @spec shutdown_write(t()) :: :ok | {:error, term()}
  def shutdown_write(%__MODULE__{socket: socket}) do
    case :gen_tcp.shutdown(socket, :write) do
      :ok -> :ok
      {:error, reason} -> {:error, map_error(reason)}
    end
  end

  # ---------------------------------------------------------------------------
  # peer_addr/1 — remote address of the connection
  # ---------------------------------------------------------------------------

  @doc """
  Returns the remote address and port of the connection.

  ## Examples

      {:ok, {{127, 0, 0, 1}, 8080}} = CodingAdventures.TcpClient.peer_addr(conn)
  """
  @spec peer_addr(t()) :: {:ok, {tuple(), non_neg_integer()}} | {:error, term()}
  def peer_addr(%__MODULE__{socket: socket}) do
    case :inet.peername(socket) do
      {:ok, {addr, port_num}} -> {:ok, {addr, port_num}}
      {:error, reason} -> {:error, map_error(reason)}
    end
  end

  # ---------------------------------------------------------------------------
  # local_addr/1 — local address of the connection
  # ---------------------------------------------------------------------------

  @doc """
  Returns the local address and port of the connection.

  ## Examples

      {:ok, {{127, 0, 0, 1}, 54321}} = CodingAdventures.TcpClient.local_addr(conn)
  """
  @spec local_addr(t()) :: {:ok, {tuple(), non_neg_integer()}} | {:error, term()}
  def local_addr(%__MODULE__{socket: socket}) do
    case :inet.sockname(socket) do
      {:ok, {addr, port_num}} -> {:ok, {addr, port_num}}
      {:error, reason} -> {:error, map_error(reason)}
    end
  end

  # ---------------------------------------------------------------------------
  # close/1 — close the connection
  # ---------------------------------------------------------------------------
  #
  # Closes both the read and write halves. After this call, the socket handle
  # is invalid. Any further operations will return {:error, :closed}.
  #
  # In Rust, connections are closed automatically when dropped (RAII). In
  # Elixir, we must call close explicitly — or rely on the process exiting,
  # which also closes all owned sockets.

  @doc """
  Close the connection.

  Returns `:ok`. The socket is invalid after this call.
  """
  @spec close(t()) :: :ok
  def close(%__MODULE__{socket: socket}) do
    :gen_tcp.close(socket)
  end

  # ---------------------------------------------------------------------------
  # Error mapping — translate Erlang/OTP errors to our domain atoms
  # ---------------------------------------------------------------------------
  #
  # Erlang's :gen_tcp and :inet modules return POSIX-inspired error atoms.
  # We translate these into our standardized error atoms for a consistent
  # API across all language implementations.
  #
  # Truth table:
  #
  #   Erlang atom       | Our atom                  | Meaning
  #   ------------------|---------------------------|---------------------------
  #   :nxdomain         | :dns_resolution_failed    | Hostname not found in DNS
  #   :econnrefused     | :connection_refused       | TCP RST during handshake
  #   :timeout          | :timeout                  | Operation took too long
  #   :closed           | :connection_reset         | Remote closed connection
  #   :enotconn         | :connection_reset         | Socket disconnected
  #   :epipe            | :broken_pipe              | Write after remote close
  #   (other)           | {:unknown, original}      | Pass-through for debugging

  @spec map_error(atom() | term()) :: atom() | {atom(), term()}
  defp map_error(:nxdomain), do: :dns_resolution_failed
  defp map_error(:econnrefused), do: :connection_refused
  defp map_error(:timeout), do: :timeout
  defp map_error(:closed), do: :connection_reset
  defp map_error(:enotconn), do: :connection_reset
  defp map_error(:epipe), do: :broken_pipe
  defp map_error(other), do: {:unknown, other}
end
