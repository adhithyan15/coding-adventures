defmodule Ls00.ParseCacheTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for ParseCache: hit/miss behavior and eviction.
  """

  alias Ls00.ParseCache

  # A minimal bridge module for testing.
  defmodule TestBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(_source), do: {:ok, []}

    @impl true
    def parse(source) do
      diags =
        if String.contains?(source, "ERROR") do
          [
            %Ls00.Types.Diagnostic{
              range: %Ls00.Types.Range{
                start: %Ls00.Types.Position{line: 0, character: 0},
                end_pos: %Ls00.Types.Position{line: 0, character: 5}
              },
              severity: Ls00.Types.severity_error(),
              message: "syntax error"
            }
          ]
        else
          []
        end

      {:ok, source, diags}
    end
  end

  test "cache hit returns same result for same version" do
    cache = ParseCache.new()

    {r1, cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 1, "hello", TestBridge)
    assert r1 != nil

    # Second call same version -- cache hit
    {r2, _cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 1, "hello", TestBridge)
    # Same struct (identical content)
    assert r1 == r2
  end

  test "cache miss for different version" do
    cache = ParseCache.new()

    {r1, cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 1, "hello", TestBridge)
    {r3, _cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 2, "hello world", TestBridge)

    # Different version -- should be a different result
    assert r3.ast == "hello world"
    assert r1.ast == "hello"
  end

  test "evict removes cached entry" do
    cache = ParseCache.new()

    {r1, cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 1, "hello", TestBridge)
    cache = ParseCache.evict(cache, "file:///a.txt")

    # After eviction, same (uri, version) produces a new parse
    {r2, _cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 1, "hello", TestBridge)
    # Content should be same but it is a new struct instance
    assert r1.ast == r2.ast
  end

  test "diagnostics populated for ERROR source" do
    cache = ParseCache.new()

    {result, _cache} = ParseCache.get_or_parse(cache, "file:///a.txt", 1, "source with ERROR token", TestBridge)
    assert length(result.diagnostics) > 0
  end

  test "no diagnostics for clean source" do
    cache = ParseCache.new()

    {result, _cache} = ParseCache.get_or_parse(cache, "file:///clean.txt", 1, "hello world", TestBridge)
    assert result.diagnostics == []
  end
end
