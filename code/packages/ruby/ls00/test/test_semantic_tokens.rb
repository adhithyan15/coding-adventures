# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Semantic Token Encoding Tests
# ================================================================
#
# Tests for the compact delta-encoded format that LSP uses for
# semantic tokens. See capabilities.rb for the encoding details.
#
# ================================================================

class TestSemanticTokens < Minitest::Test
  def test_empty
    data = CodingAdventures::Ls00.encode_semantic_tokens(nil)
    assert_empty data

    data2 = CodingAdventures::Ls00.encode_semantic_tokens([])
    assert_empty data2
  end

  # A single keyword token at the start of the file.
  # Expected: [deltaLine=0, deltaChar=0, length=5, typeIndex=15 (keyword), modifiers=0]
  def test_single_token
    tokens = [
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 0, length: 5, token_type: "keyword", modifiers: []
      )
    ]
    data = CodingAdventures::Ls00.encode_semantic_tokens(tokens)

    assert_equal 5, data.length
    assert_equal 0, data[0]  # deltaLine
    assert_equal 0, data[1]  # deltaChar
    assert_equal 5, data[2]  # length
    assert_equal 15, data[3] # keyword index
    assert_equal 0, data[4]  # modifiers
  end

  # Two tokens on the same line.
  # Token A: keyword at char 0, length 3
  # Token B: function at char 4, length 4, with "declaration" modifier
  def test_multiple_tokens_same_line
    tokens = [
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 0, length: 3, token_type: "keyword", modifiers: nil
      ),
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 4, length: 4, token_type: "function", modifiers: ["declaration"]
      )
    ]
    data = CodingAdventures::Ls00.encode_semantic_tokens(tokens)

    assert_equal 10, data.length

    # Token A: deltaLine=0, deltaChar=0, length=3, keyword(15), mods=0
    assert_equal [0, 0, 3, 15, 0], data[0..4]

    # Token B: deltaLine=0, deltaChar=4, length=4, function(12), mods=1 (declaration=bit0)
    assert_equal [0, 4, 4, 12, 1], data[5..9]
  end

  # Two tokens on different lines.
  # Token A: keyword on line 0
  # Token B: number on line 2, char 4
  def test_multiple_lines
    tokens = [
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 0, length: 3, token_type: "keyword", modifiers: nil
      ),
      CodingAdventures::Ls00::SemanticToken.new(
        line: 2, character: 4, length: 5, token_type: "number", modifiers: nil
      )
    ]
    data = CodingAdventures::Ls00.encode_semantic_tokens(tokens)

    assert_equal 10, data.length
    # Token B: deltaLine=2, deltaChar=4 (absolute on new line), number=19
    assert_equal 2, data[5]   # deltaLine
    assert_equal 4, data[6]   # deltaChar (absolute on new line)
    assert_equal 19, data[8]  # number index
  end

  # Tokens given in reverse order -- the encoder should sort them.
  def test_unsorted_input
    tokens = [
      CodingAdventures::Ls00::SemanticToken.new(
        line: 1, character: 0, length: 2, token_type: "number", modifiers: nil
      ),
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 0, length: 3, token_type: "keyword", modifiers: nil
      )
    ]
    data = CodingAdventures::Ls00.encode_semantic_tokens(tokens)

    assert_equal 10, data.length
    # After sorting: keyword on line 0 first, number on line 1 second
    assert_equal 15, data[3] # first token should be keyword (15)
    assert_equal 19, data[8] # second token should be number (19)
  end

  # Unknown token type should be skipped.
  def test_unknown_token_type
    tokens = [
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 0, length: 3, token_type: "unknownType", modifiers: nil
      ),
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 4, length: 2, token_type: "keyword", modifiers: nil
      )
    ]
    data = CodingAdventures::Ls00.encode_semantic_tokens(tokens)

    # unknownType should be skipped, leaving only one 5-tuple
    assert_equal 5, data.length
  end

  # "readonly" modifier is bit 2 (index 2), value = 4.
  def test_modifier_bitmask
    tokens = [
      CodingAdventures::Ls00::SemanticToken.new(
        line: 0, character: 0, length: 3, token_type: "variable", modifiers: ["readonly"]
      )
    ]
    data = CodingAdventures::Ls00.encode_semantic_tokens(tokens)

    assert_equal 4, data[4] # readonly = bit 2 = value 4
  end
end
