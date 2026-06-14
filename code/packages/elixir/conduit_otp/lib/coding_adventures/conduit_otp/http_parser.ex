defmodule CodingAdventures.ConduitOtp.HttpParser do
  @moduledoc """
  Teaching topic: `:erlang.decode_packet/3` and Erlang's built-in HTTP framing.

  ## What this is

  This module wraps Erlang's built-in HTTP/1.1 packet decoder,
  `:erlang.decode_packet/3`. Rather than writing a hand-rolled parser, we
  leverage a decoder that ships with every Erlang/Elixir install — the same
  one used by Erlang's own `:httpc` and `:httpd` since R13.

  ## How `:erlang.decode_packet/3` works

  The function signature:

      :erlang.decode_packet(Type, Binary, Options) ->
          {:ok, Packet, Rest} | {:more, Length} | {:error, Reason}

  | `Type`     | Meaning |
  |------------|---------|
  | `:http_bin`| Parse one HTTP/1.1 line. Returns structured tuples. |
  | `:raw`     | Return raw bytes (used for body). |

  ### What the structured tuples look like

  After setting `Type = :http_bin`, each call returns one of:

  | Returned form | Meaning |
  |---------------|---------|
  | `{:http_request, method, {:abs_path, path}, {1,1}}` | Request line (GET /... HTTP/1.1) |
  | `{:http_header, _, name, _, value}`                 | One header field |
  | `:http_eoh`                                          | End of headers |

  Where `method` is an atom like `:GET` or a binary for custom methods,
  and `path` is the raw URI path binary (e.g. `"/hello?q=x"`).

  ## Socket-level usage

  When you open a `:gen_tcp` socket with `{:packet, :http_bin}`, the BEAM
  socket driver performs the framing *before* your `recv` call returns.
  Each `recv` call delivers exactly one decoded HTTP packet.

  This module provides `read_request/1` which:
  1. Reads the request line.
  2. Loops reading headers until `:http_eoh`.
  3. Switches the socket to `{:packet, :raw}` mode.
  4. Reads the body if `Content-Length > 0`.

  It also exposes `decode_packet/1` for use in tests where you have a raw
  binary (not a live socket) — calls `:erlang.decode_packet(:http_bin, binary, [])`.
  """

  require Logger

  @doc """
  Read a full HTTP/1.1 request from a passive TCP socket in `:http_bin` mode.

  Returns `{:ok, {method, path, headers, body}}` on success.
  Returns `{:error, reason}` on socket errors or malformed input.

  ## Reading model

  1. `recv(socket, 0)` — returns the request line as a structured tuple.
  2. Loop `recv(socket, 0)` — returns header tuples until `:http_eoh`.
  3. Set `{:packet, :raw}` and `recv(socket, content_length)` for body.

  The `0` byte-count argument to `recv` means "read as many bytes as the
  packet framing says is one unit" — with `:http_bin` that is one HTTP line.
  """
  @spec read_request(:gen_tcp.socket()) ::
          {:ok, {String.t(), String.t(), map, binary}} | {:error, term}
  def read_request(socket) do
    with {:ok, method, path} <- read_request_line(socket),
         {:ok, headers} <- read_headers(socket),
         {:ok, body} <- read_body(socket, headers) do
      {:ok, {method, path, headers, body}}
    end
  end

  @doc """
  Decode a single HTTP packet from a raw binary using `:erlang.decode_packet/3`.

  Useful for unit testing the parser logic without a live socket.

  Returns the raw Erlang term (e.g. `{:http_request, :GET, {:abs_path, "/"}, {1, 1}}`).
  """
  @spec decode_packet(binary) ::
          {:ok, term, binary} | {:more, non_neg_integer | :undefined} | {:error, term}
  def decode_packet(binary) when is_binary(binary) do
    :erlang.decode_packet(:http_bin, binary, [])
  end

  # ── Private: request line ────────────────────────────────────────────────────

  defp read_request_line(socket) do
    case :gen_tcp.recv(socket, 0, 15_000) do
      {:ok, {:http_request, method_atom, {:abs_path, path_bin}, _version}} ->
        method = normalise_method(method_atom)
        {:ok, method, to_string(path_bin)}

      {:ok, {:http_request, method_atom, path_term, _version}} ->
        # Handle :* (OPTIONS *) or other forms
        method = normalise_method(method_atom)
        path = case path_term do
          {:abs_path, p} -> to_string(p)
          _ -> "/"
        end
        {:ok, method, path}

      {:ok, other} ->
        Logger.debug("Unexpected HTTP packet in request line: #{inspect(other)}")
        {:error, :bad_request}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ── Private: headers ─────────────────────────────────────────────────────────

  defp read_headers(socket), do: read_headers(socket, %{})

  defp read_headers(socket, acc) do
    case :gen_tcp.recv(socket, 0, 15_000) do
      {:ok, :http_eoh} ->
        {:ok, acc}

      {:ok, {:http_header, _reserved, name, _res, value}} ->
        # The BEAM HTTP parser returns header names as atoms for known headers
        # (e.g. `:host`) and as binaries for unknown headers. Normalise to
        # lower-case strings — HTTP headers are case-insensitive per RFC 7230.
        key = name |> normalise_header_name()
        val = to_string(value)
        read_headers(socket, Map.put(acc, key, val))

      {:ok, other} ->
        Logger.debug("Unexpected HTTP packet in header: #{inspect(other)}")
        {:error, :bad_request}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ── Private: body ────────────────────────────────────────────────────────────

  defp read_body(socket, headers) do
    cl = headers |> Map.get("content-length", "0") |> parse_int(0)

    if cl > 0 do
      # Switch to raw mode so we can read exactly `cl` bytes.
      :ok = :inet.setopts(socket, [{:packet, :raw}])

      case :gen_tcp.recv(socket, cl, 30_000) do
        {:ok, body} ->
          # Restore :http_bin mode for keep-alive (next request on same socket).
          :ok = :inet.setopts(socket, [{:packet, :http_bin}])
          {:ok, body}

        {:error, reason} ->
          {:error, reason}
      end
    else
      {:ok, ""}
    end
  end

  # ── Private utilities ────────────────────────────────────────────────────────

  # HTTP methods come back as atoms from the BEAM parser for common methods,
  # and as binaries for custom methods. Normalise to upper-case strings.
  defp normalise_method(method) when is_atom(method), do: Atom.to_string(method)
  defp normalise_method(method) when is_binary(method), do: String.upcase(method)

  # Header names come back as atoms (`:host`, `:content_type`) for standard
  # headers, or as binaries for custom/unknown headers. Map known atoms to
  # their canonical lower-case string. Unknown binaries get lower-cased.
  #
  # The BEAM maps `:content_type` → `"content-type"`, `:transfer_encoding`
  # → `"transfer-encoding"`, etc. We replicate that mapping to avoid
  # callers needing to know both forms.
  defp normalise_header_name(name) when is_atom(name) do
    name
    |> Atom.to_string()
    |> String.replace("_", "-")
    |> String.downcase()
  end

  defp normalise_header_name(name) when is_binary(name) do
    String.downcase(name)
  end

  defp parse_int(s, default) when is_binary(s) do
    case Integer.parse(s) do
      {n, _} when n >= 0 -> n
      _ -> default
    end
  end

  defp parse_int(_, default), do: default
end
