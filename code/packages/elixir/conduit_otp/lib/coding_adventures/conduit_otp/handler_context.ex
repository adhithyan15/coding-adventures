defmodule CodingAdventures.ConduitOtp.HandlerContext do
  @moduledoc """
  Teaching topic: Response constructors and non-local exits.

  ## What this is

  Handlers return a 3-tuple `{status, headers_map, body_binary}`.  Rather
  than ask users to assemble that tuple by hand, this module provides
  Sinatra-style ergonomic helpers.

  ## Usage

  Import this module at the top of any file that defines handlers:

      import CodingAdventures.ConduitOtp.HandlerContext

      Application.get("/", fn _req -> html("<h1>OK</h1>") end)
      Application.get("/api", fn _req -> json(%{ok: true}) end)

  ## Response helpers

      html("<h1>OK</h1>")                       # 200 text/html
      html("<h1>OK</h1>", 201)                  # custom status
      json(%{user: 1})                          # 200 application/json
      json(%{error: "boom"}, 500)               # custom status
      text("plain string")                      # 200 text/plain
      respond(204, "", %{"x-custom" => "v"})    # arbitrary

  ## Halt / redirect (non-local exits using `throw`)

      halt(404, "Not found")
      redirect("/login")

  ## Zero deps JSON encoding

  We use Elixir 1.18+'s built-in `JSON` module when available, and fall
  back to a minimal inline encoder for older Elixir versions. This is
  consistent with WEB06 and avoids pulling in `jason` as a dependency.
  """

  alias CodingAdventures.ConduitOtp.HaltError

  @type response :: {integer, map, binary}

  @doc "200/HTML response (UTF-8). Status overridable."
  @spec html(binary, integer) :: response
  def html(body, status \\ 200) when is_binary(body) and is_integer(status) do
    {status, %{"content-type" => "text/html; charset=utf-8"}, body}
  end

  @doc """
  JSON response — encodes `value` with Elixir 1.18+'s built-in `JSON`.

  Falls back to a hand-rolled minimal encoder for older Elixir versions.
  Handles strings, numbers, booleans, nil, lists, and maps.
  """
  @spec json(term, integer) :: response
  def json(value, status \\ 200) when is_integer(status) do
    body = encode_json(value)
    {status, %{"content-type" => "application/json"}, body}
  end

  @doc "200/plain-text response (UTF-8). Status overridable."
  @spec text(binary, integer) :: response
  def text(body, status \\ 200) when is_binary(body) and is_integer(status) do
    {status, %{"content-type" => "text/plain; charset=utf-8"}, body}
  end

  @doc "Build a custom response with arbitrary headers (map of strings)."
  @spec respond(integer, binary, map) :: response
  def respond(status, body, headers \\ %{})
      when is_integer(status) and is_binary(body) and is_map(headers) do
    {status, headers, body}
  end

  # Delegate throw-based helpers to HaltError so users can import just this
  # module and get everything they need.
  defdelegate halt(status), to: HaltError
  defdelegate halt(status, body), to: HaltError
  defdelegate halt(status, body, headers), to: HaltError
  defdelegate redirect(location), to: HaltError
  defdelegate redirect(location, status), to: HaltError

  # ── JSON encoding ────────────────────────────────────────────────────────────
  #
  # We use a compile-time `if Code.ensure_loaded?(JSON)` to select the
  # implementation. The `JSON` module ships with Elixir 1.18; older versions
  # fall through to the hand-rolled encoder below. This is the same technique
  # used in WEB06, so both packages handle the same Elixir version range.

  if Code.ensure_loaded?(JSON) do
    defp encode_json(value), do: JSON.encode!(value)
  else
    defp encode_json(nil), do: "null"
    defp encode_json(true), do: "true"
    defp encode_json(false), do: "false"
    defp encode_json(n) when is_integer(n) or is_float(n), do: to_string(n)
    defp encode_json(s) when is_binary(s), do: "\"" <> escape_string(s) <> "\""
    defp encode_json(a) when is_atom(a), do: encode_json(Atom.to_string(a))

    defp encode_json(list) when is_list(list) do
      "[" <> Enum.map_join(list, ",", &encode_json/1) <> "]"
    end

    defp encode_json(map) when is_map(map) do
      pairs =
        map
        |> Enum.map(fn {k, v} ->
          k_s = if is_binary(k), do: k, else: to_string(k)
          "\"" <> escape_string(k_s) <> "\":" <> encode_json(v)
        end)
        |> Enum.join(",")

      "{" <> pairs <> "}"
    end

    defp escape_string(s) do
      s
      |> String.replace("\\", "\\\\")
      |> String.replace("\"", "\\\"")
      |> String.replace("\n", "\\n")
      |> String.replace("\r", "\\r")
      |> String.replace("\t", "\\t")
    end
  end
end
