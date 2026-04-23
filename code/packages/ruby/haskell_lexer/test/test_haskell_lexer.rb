# frozen_string_literal: true

require_relative "test_helper"

class TestHaskellLexer < Minitest::Test
  def test_default_version_points_to_haskell2010
    path = CodingAdventures::HaskellLexer.resolve_tokens_path(nil)
    assert_match(%r{haskell/haskell2010\.tokens$}, path)
  end

  def test_layout_tokens_are_emitted
    tokens = CodingAdventures::HaskellLexer.tokenize("let\n  x = y\nin x")
    type_names = tokens.map(&:type_name)
    assert_includes type_names, "VIRTUAL_LBRACE"
    assert_includes type_names, "VIRTUAL_RBRACE"
  end

  def test_explicit_version_is_supported
    tokens = CodingAdventures::HaskellLexer.tokenize("x", version: "98")
    assert_equal "NAME", tokens.first.type_name
  end
end
