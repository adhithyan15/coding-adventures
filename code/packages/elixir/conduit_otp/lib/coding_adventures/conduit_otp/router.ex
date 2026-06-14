defmodule CodingAdventures.ConduitOtp.Router do
  @moduledoc """
  Teaching topic: Pure functions — no processes, easy to test.

  ## What this is

  The router is a plain module — no gen_server, no Agent, no process at all.
  It takes a list of routes and a `{method, path}` and returns either the
  matching route (with named captures extracted) or `nil`.

  ## Why pure?

  A pure function is:
  - Easy to test: call it directly, no setup/teardown.
  - Easy to reason about: given the same inputs, always the same output.
  - Composable: can be called from Worker, tests, or an interactive session.

  The OTP concurrency primitives (gen_server, Agent) are for *stateful*
  resources that live across calls. Route matching is stateless — the routes
  list never changes mid-call, and the path never changes the routes list.
  Pure function is the right tool here.

  ## Pattern syntax

  Patterns follow the same `:param` named-capture syntax used in WEB05/WEB06
  and the other language ports:

  | Pattern         | Example path   | Captured params          |
  |-----------------|----------------|--------------------------|
  | `/`             | `/`            | `%{}`                    |
  | `/hello/:name`  | `/hello/Alice` | `%{"name" => "Alice"}`   |
  | `/a/:x/b/:y`   | `/a/1/b/2`    | `%{"x" => "1", "y" => "2"}` |
  | `/*`            | (not used)     | n/a                      |

  Segments are split on `/`. A `:param` segment matches any single path
  segment and captures its value. Literal segments must match exactly.
  Trailing slashes are normalised (both `/foo` and `/foo/` match pattern `/foo`).

  ## Example

      iex> Router.match([%{method: "GET", pattern: "/hello/:name", handler_id: 1}], "GET", "/hello/Alice")
      {:ok, 1, %{"name" => "Alice"}}

      iex> Router.match([], "GET", "/missing")
      :not_found
  """

  @doc """
  Find the first matching route for `{method, path}`.

  Returns `{:ok, handler_id, params}` or `:not_found`.
  """
  @spec match([map], String.t(), String.t()) ::
          {:ok, pos_integer, map} | :not_found
  def match(routes, method, path) when is_list(routes) and is_binary(method) and is_binary(path) do
    # Normalise the path: strip trailing slash (except root "/").
    normalized = normalize_path(path)

    Enum.find_value(routes, :not_found, fn %{method: m, pattern: p, handler_id: id} ->
      if m == method do
        case match_pattern(p, normalized) do
          {:ok, params} -> {:ok, id, params}
          :no_match -> nil
        end
      end
    end)
  end

  # ── Private helpers ──────────────────────────────────────────────────────────

  # Normalise path: remove trailing slash unless the path IS just "/".
  # This makes `/foo` and `/foo/` match the same pattern `/foo`.
  defp normalize_path("/"), do: "/"

  defp normalize_path(path) do
    if String.ends_with?(path, "/") do
      String.trim_trailing(path, "/")
    else
      path
    end
  end

  # Match a single pattern string against a normalised path.
  # Returns `{:ok, params_map}` on success, `:no_match` otherwise.
  defp match_pattern(pattern, path) do
    pat_segments = String.split(pattern, "/", trim: true)
    path_segments = String.split(path, "/", trim: true)

    if length(pat_segments) == length(path_segments) do
      match_segments(pat_segments, path_segments, %{})
    else
      :no_match
    end
  end

  # Recursively match segment lists, accumulating named captures.
  defp match_segments([], [], params), do: {:ok, params}

  defp match_segments([":" <> name | pat_rest], [seg | path_rest], params) do
    # Named capture: `:param` matches any non-empty path segment.
    match_segments(pat_rest, path_rest, Map.put(params, name, seg))
  end

  defp match_segments([literal | pat_rest], [literal | path_rest], params) do
    # Literal match: pattern segment equals path segment exactly.
    match_segments(pat_rest, path_rest, params)
  end

  defp match_segments(_, _, _), do: :no_match
end
