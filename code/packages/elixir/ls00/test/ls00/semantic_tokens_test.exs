defmodule Ls00.SemanticTokensTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for semantic token encoding.

  The LSP semantic token encoding is a compact flat integer array. These tests
  verify the delta encoding, modifier bitmasks, sorting, and unknown type handling.
  """

  alias Ls00.Capabilities
  alias Ls00.Types.SemanticToken

  test "empty tokens produce empty data" do
    assert Capabilities.encode_semantic_tokens([]) == []
    assert Capabilities.encode_semantic_tokens(nil) == []
  end

  test "single keyword token" do
    tokens = [
      %SemanticToken{line: 0, character: 0, length: 5, token_type: "keyword", modifiers: []}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)

    # Expected: [deltaLine=0, deltaChar=0, length=5, typeIndex=15 (keyword), modifiers=0]
    assert length(data) == 5
    assert Enum.at(data, 0) == 0   # deltaLine
    assert Enum.at(data, 1) == 0   # deltaChar
    assert Enum.at(data, 2) == 5   # length
    assert Enum.at(data, 3) == 15  # keyword index
    assert Enum.at(data, 4) == 0   # no modifiers
  end

  test "two tokens on the same line" do
    tokens = [
      %SemanticToken{line: 0, character: 0, length: 3, token_type: "keyword", modifiers: []},
      %SemanticToken{line: 0, character: 4, length: 4, token_type: "function", modifiers: ["declaration"]}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)
    assert length(data) == 10

    # Token A: deltaLine=0, deltaChar=0, length=3, keyword(15), mods=0
    assert Enum.slice(data, 0, 5) == [0, 0, 3, 15, 0]

    # Token B: deltaLine=0, deltaChar=4 (relative to A), length=4, function(12), mods=1 (declaration=bit0)
    assert Enum.slice(data, 5, 5) == [0, 4, 4, 12, 1]
  end

  test "tokens on different lines" do
    tokens = [
      %SemanticToken{line: 0, character: 0, length: 3, token_type: "keyword", modifiers: []},
      %SemanticToken{line: 2, character: 4, length: 5, token_type: "number", modifiers: []}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)
    assert length(data) == 10

    # Token B: deltaLine=2, deltaChar=4 (absolute on new line), number=19
    assert Enum.at(data, 5) == 2   # deltaLine
    assert Enum.at(data, 6) == 4   # deltaChar (absolute on new line)
    assert Enum.at(data, 8) == 19  # number index
  end

  test "unsorted input is sorted by encoder" do
    # Tokens in reverse order -- the encoder should sort them.
    tokens = [
      %SemanticToken{line: 1, character: 0, length: 2, token_type: "number", modifiers: []},
      %SemanticToken{line: 0, character: 0, length: 3, token_type: "keyword", modifiers: []}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)
    assert length(data) == 10

    # After sorting: keyword on line 0 first, number on line 1 second
    assert Enum.at(data, 3) == 15  # first token = keyword
    assert Enum.at(data, 8) == 19  # second token = number
  end

  test "unknown token type is skipped" do
    tokens = [
      %SemanticToken{line: 0, character: 0, length: 3, token_type: "unknownType", modifiers: []},
      %SemanticToken{line: 0, character: 4, length: 2, token_type: "keyword", modifiers: []}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)
    # unknownType should be skipped, leaving only one 5-tuple
    assert length(data) == 5
  end

  test "modifier bitmask: readonly = bit 2 = value 4" do
    tokens = [
      %SemanticToken{line: 0, character: 0, length: 3, token_type: "variable", modifiers: ["readonly"]}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)
    assert Enum.at(data, 4) == 4  # readonly = bit 2 = value 4
  end

  test "multiple modifiers combine as bitmask" do
    # "declaration" = bit 0 = 1, "readonly" = bit 2 = 4, combined = 5
    tokens = [
      %SemanticToken{line: 0, character: 0, length: 3, token_type: "variable",
                     modifiers: ["declaration", "readonly"]}
    ]

    data = Capabilities.encode_semantic_tokens(tokens)
    assert Enum.at(data, 4) == 5  # 1 | 4 = 5
  end
end
