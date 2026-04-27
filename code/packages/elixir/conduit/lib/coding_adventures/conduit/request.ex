defmodule CodingAdventures.Conduit.Request do
  @moduledoc """
  Read-only view of an incoming HTTP request, derived from the Rust env map.

  ## Wire format

  The Rust dispatcher sends a single message per request:

      {:conduit_request, slot_id, handler_id, env_map}

  where `env_map` is an Elixir `%{}` with the CGI/Rack-style keys
  documented in WEB06. `Request.from_env/1` projects that map onto the
  fields below.

  ## Fields

  - `:env`             — raw env map for power users
  - `:method`          — `"GET"` etc.
  - `:path`            — `"/hello/world"` (no query string)
  - `:query_string`    — `"foo=bar"` (no leading `?`)
  - `:params`          — named route captures, `%{"name" => "world"}`
  - `:query_params`    — parsed query string, `%{"foo" => "bar"}`
  - `:headers`         — request headers with lower-case keys
  - `:body`            — raw body binary
  - `:content_type`    — value of `content-type` header, or `""`
  - `:content_length`  — integer (0 if absent / non-numeric)
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

  @doc "Build a `Request` from the env map sent by the Rust dispatcher."
  @spec from_env(map) :: t
  def from_env(env) when is_map(env) do
    cl =
      case env["conduit.content_length"] do
        nil -> 0
        s when is_binary(s) ->
          case Integer.parse(s) do
            {n, _} -> n
            _ -> 0
          end
        _ -> 0
      end

    %__MODULE__{
      env: env,
      method:        Map.get(env, "REQUEST_METHOD", "GET"),
      path:          Map.get(env, "PATH_INFO", "/"),
      query_string:  Map.get(env, "QUERY_STRING", ""),
      params:        Map.get(env, "conduit.route_params", %{}),
      query_params:  Map.get(env, "conduit.query_params", %{}),
      headers:       Map.get(env, "conduit.headers", %{}),
      body:          Map.get(env, "conduit.body", ""),
      content_type:  Map.get(env, "conduit.content_type", ""),
      content_length: cl
    }
  end

  @doc """
  Decode the request body as JSON. Returns the parsed value.

  ## Errors

  - Raises `ArgumentError` if the body is not valid JSON.
  - Raises `Conduit.HaltError` (413) if the body exceeds 10 MiB —
    a safety guard against algorithmic-complexity DoS attacks.

  Use `json_body/1` (no `!`) for a tagged-tuple variant if you'd rather
  pattern-match on `{:ok, value}` / `{:error, reason}`.
  """
  @spec json_body!(t) :: term
  def json_body!(%__MODULE__{body: body}) do
    if byte_size(body) > 10 * 1024 * 1024 do
      raise CodingAdventures.Conduit.HaltError,
        message: "Payload Too Large",
        status: 413,
        body: "Payload Too Large"
    end

    if Code.ensure_loaded?(JSON) do
      JSON.decode!(body)
    else
      raise ArgumentError,
            "JSON module not available; depend on jason or upgrade to Elixir 1.18+"
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
end
