defmodule CodingAdventures.ConduitOtp.Request do
  @moduledoc """
  Teaching topic: Plain immutable struct as request view.

  ## What this is

  `%Request{}` is the read-only view of an HTTP request handed to every
  handler, before_filter, after_filter, not_found handler, and error handler.

  It is a plain struct — not a gen_server, not a process. Elixir structs
  are syntactic sugar over maps with a defined shape and compile-time type
  checking. They are cheap to create and safe to pass between processes
  (they are immutable data).

  ## Building a Request

  The `Worker` module calls `Request.from_parsed/5` after the `HttpParser`
  has decoded the raw TCP bytes. `from_parsed/5` validates types and fills
  in defaults.

  ## Fields

  - `:method`         — `"GET"`, `"POST"`, etc.
  - `:path`           — `"/hello/world"` (no query string)
  - `:query_string`   — `"foo=bar"` (no leading `?`)
  - `:params`         — named route captures from pattern matching, e.g.
                        `%{"name" => "world"}` from `/hello/:name`
  - `:query_params`   — parsed query string, `%{"foo" => "bar"}`
  - `:headers`        — request headers with lower-case keys
  - `:body`           — raw body binary (empty string for bodyless methods)
  - `:content_type`   — value of the `content-type` header, or `""`
  - `:content_length` — integer parsed from `content-length` header, or 0
  - `:env`            — raw map for power users / framework internals

  ## Example

      # In a handler:
      Application.get("/hello/:name", fn req ->
        name = req.params["name"]
        ct   = req.content_type        # "application/json" or ""
        qs   = req.query_string        # "q=hello&page=2"
        json(%{message: "Hello " <> name})
      end)
  """

  defstruct env: %{},
            method: "GET",
            path: "/",
            query_string: "",
            params: %{},
            query_params: %{},
            headers: %{},
            body: "",
            content_type: "",
            content_length: 0

  @type t :: %__MODULE__{
          env: map,
          method: String.t(),
          path: String.t(),
          query_string: String.t(),
          params: %{optional(String.t()) => String.t()},
          query_params: %{optional(String.t()) => String.t()},
          headers: %{optional(String.t()) => String.t()},
          body: binary,
          content_type: String.t(),
          content_length: non_neg_integer()
        }

  @doc """
  Build a `Request` from the decomposed fields produced by `HttpParser`.

  Called by `Worker` after successful HTTP parsing. The `params` field
  starts empty here; `Router` fills it in when it matches the route.
  """
  @spec from_parsed(String.t(), String.t(), map, binary, String.t()) :: t
  def from_parsed(method, path_with_query, headers, body, query_string \\ "") do
    # Split path from query string if the caller did not pre-split.
    {path, qs} =
      case String.split(path_with_query, "?", parts: 2) do
        [p, q] -> {p, q}
        [p] -> {p, query_string}
      end

    ct = Map.get(headers, "content-type", "")
    cl = parse_content_length(Map.get(headers, "content-length", "0"))

    env = %{
      "REQUEST_METHOD" => method,
      "PATH_INFO" => path,
      "QUERY_STRING" => qs,
      "conduit.headers" => headers,
      "conduit.body" => body,
      "conduit.content_type" => ct,
      "conduit.content_length" => to_string(cl),
      "conduit.query_params" => parse_query_string(qs)
    }

    %__MODULE__{
      env: env,
      method: method,
      path: path,
      query_string: qs,
      params: %{},
      query_params: parse_query_string(qs),
      headers: headers,
      body: body,
      content_type: ct,
      content_length: cl
    }
  end

  @doc """
  Build a `Request` from the raw CGI/Rack-style env map.

  This variant is used in tests and by any code that constructs requests
  from an existing map (e.g. the Dispatcher compatibility layer). The
  WEB06 Rust side sends exactly this map shape.
  """
  @spec from_env(map) :: t
  def from_env(env) when is_map(env) do
    cl =
      case env["conduit.content_length"] do
        nil -> 0
        s when is_binary(s) -> parse_content_length(s)
        n when is_integer(n) -> n
        _ -> 0
      end

    %__MODULE__{
      env: env,
      method: Map.get(env, "REQUEST_METHOD", "GET"),
      path: Map.get(env, "PATH_INFO", "/"),
      query_string: Map.get(env, "QUERY_STRING", ""),
      params: Map.get(env, "conduit.route_params", %{}),
      query_params: Map.get(env, "conduit.query_params", %{}),
      headers: Map.get(env, "conduit.headers", %{}),
      body: Map.get(env, "conduit.body", ""),
      content_type: Map.get(env, "conduit.content_type", ""),
      content_length: cl
    }
  end

  @doc """
  Decode the request body as JSON. Returns the parsed value.

  Raises `ArgumentError` if the body is not valid JSON.
  Raises `HaltError` (413) if the body exceeds 10 MiB.
  """
  @spec json_body!(t) :: term
  def json_body!(%__MODULE__{body: body}) do
    if byte_size(body) > 10 * 1024 * 1024 do
      raise CodingAdventures.ConduitOtp.HaltError,
        message: "Payload Too Large",
        status: 413,
        body: "Payload Too Large"
    end

    if Code.ensure_loaded?(JSON) do
      JSON.decode!(body)
    else
      raise ArgumentError,
            "JSON module not available; upgrade to Elixir 1.18+"
    end
  end

  @doc "Tagged-tuple variant of `json_body!/1`."
  @spec json_body(t) :: {:ok, term} | {:error, term}
  def json_body(%__MODULE__{} = req) do
    {:ok, json_body!(req)}
  rescue
    e -> {:error, e}
  catch
    :throw, e -> {:error, e}
  end

  # ── Private helpers ──────────────────────────────────────────────────────────

  defp parse_content_length(s) when is_binary(s) do
    case Integer.parse(s) do
      {n, _} when n >= 0 -> n
      _ -> 0
    end
  end

  defp parse_content_length(_), do: 0

  # Simple query-string parser: "foo=bar&baz=qux" -> %{"foo" => "bar", ...}
  # Does NOT handle array params ("a[]=1&a[]=2") — beyond our scope.
  defp parse_query_string(""), do: %{}

  defp parse_query_string(qs) do
    qs
    |> String.split("&")
    |> Enum.reduce(%{}, fn pair, acc ->
      case String.split(pair, "=", parts: 2) do
        [k, v] -> Map.put(acc, URI.decode_www_form(k), URI.decode_www_form(v))
        [k] -> Map.put(acc, URI.decode_www_form(k), "")
        _ -> acc
      end
    end)
  end
end
