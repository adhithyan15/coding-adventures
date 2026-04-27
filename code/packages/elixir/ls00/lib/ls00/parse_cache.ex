defmodule Ls00.ParseCache do
  @moduledoc """
  Avoid re-parsing unchanged documents.

  ## Why Cache Parse Results?

  Parsing is the most expensive operation in a language server. For a large
  file, parsing on every keystroke would lag the editor noticeably.

  The LSP protocol helps by sending a version number with every change. If the
  document hasn't changed (same URI, same version), the parse result from the
  previous keystroke is still valid.

  ## Cache Key Design

  The cache key is `{uri, version}`. Version is a monotonically increasing
  integer that the editor increments with each change. Using version in the key
  means:

    - Same `{uri, version}` -> cache hit -> return cached result
    - Different version -> cache miss -> re-parse and cache new result

  The old entry is evicted when a new version is cached for the same URI.
  This keeps memory bounded at O(open_documents) entries.

  ## Implementation

  The cache is a plain map (not a GenServer). The LspServer owns it and
  passes it through function calls. This keeps the design simple and testable.
  """

  alias Ls00.Types.ParseResult

  @type cache_key :: {String.t(), integer()}
  @type t :: %{cache_key() => ParseResult.t()}

  @doc "Create an empty ParseCache."
  @spec new() :: t()
  def new, do: %{}

  @doc """
  Return the parse result for `{uri, version}`.

  If the result is already cached, it is returned immediately without calling
  the bridge again. Otherwise, `bridge_module.parse(source)` is called, the
  result is stored, and the previous cache entry for this URI (if any) is
  evicted to prevent unbounded growth.

  Returns `{parse_result, updated_cache}`.
  """
  @spec get_or_parse(t(), String.t(), integer(), String.t(), module()) ::
          {ParseResult.t(), t()}
  def get_or_parse(cache, uri, version, source, bridge_module) do
    key = {uri, version}

    case Map.fetch(cache, key) do
      {:ok, result} ->
        # Cache hit: the document hasn't changed since last parse.
        {result, cache}

      :error ->
        # Cache miss: parse and store. Evict any stale entry for this URI first.
        cache = evict(cache, uri)

        result =
          case bridge_module.parse(source) do
            {:ok, ast, diags} ->
              # Normalize nil diagnostics list to empty list for JSON encoding.
              diags = diags || []
              %ParseResult{ast: ast, diagnostics: diags}

            {:error, reason} ->
              %ParseResult{ast: nil, diagnostics: [], err: reason}
          end

        {result, Map.put(cache, key, result)}
    end
  end

  @doc """
  Remove all cached entries for a given URI.

  Called when a document is closed (didClose) so the cache entry is cleaned up.
  """
  @spec evict(t(), String.t()) :: t()
  def evict(cache, uri) do
    cache
    |> Enum.reject(fn {{cached_uri, _version}, _result} -> cached_uri == uri end)
    |> Map.new()
  end
end
