# frozen_string_literal: true

require_relative "test_helper"

# ================================================================
# Capabilities Tests
# ================================================================
#
# Tests for the capability advertisement system. The server must
# advertise exactly the features the bridge supports -- no more,
# no less.
#
# ================================================================

class TestCapabilities < Minitest::Test
  # A minimal bridge (tokenize + parse only) should advertise only
  # textDocumentSync. No optional capabilities should appear.
  def test_minimal_bridge
    bridge = MinimalBridge.new
    caps = CodingAdventures::Ls00.build_capabilities(bridge)

    assert_equal 2, caps["textDocumentSync"]

    optional_caps = %w[
      hoverProvider definitionProvider referencesProvider
      completionProvider renameProvider documentSymbolProvider
      foldingRangeProvider signatureHelpProvider
      documentFormattingProvider semanticTokensProvider
    ]

    optional_caps.each do |cap|
      refute caps.key?(cap), "minimal bridge should not advertise #{cap}"
    end
  end

  # MockBridge implements hover and document_symbols.
  def test_mock_bridge
    bridge = MockBridge.new
    caps = CodingAdventures::Ls00.build_capabilities(bridge)

    assert caps.key?("hoverProvider"), "expected hoverProvider for MockBridge"
    assert caps.key?("documentSymbolProvider"), "expected documentSymbolProvider for MockBridge"
  end

  # MockBridge does NOT implement semanticTokensProvider.
  def test_semantic_tokens_not_in_mock
    bridge = MockBridge.new
    caps = CodingAdventures::Ls00.build_capabilities(bridge)
    refute caps.key?("semanticTokensProvider"),
           "MockBridge doesn't implement semantic_tokens, should not be in caps"
  end

  # FullMockBridge implements everything, including semanticTokensProvider.
  def test_semantic_tokens_in_full_bridge
    bridge = FullMockBridge.new
    caps = CodingAdventures::Ls00.build_capabilities(bridge)

    assert caps.key?("semanticTokensProvider"),
           "FullMockBridge implements semantic_tokens, should be in caps"

    st_provider = caps["semanticTokensProvider"]
    assert_equal true, st_provider["full"]
    assert st_provider.key?("legend")
  end

  # FullMockBridge should advertise ALL capabilities.
  def test_full_bridge_all_capabilities
    bridge = FullMockBridge.new
    caps = CodingAdventures::Ls00.build_capabilities(bridge)

    expected = %w[
      textDocumentSync
      hoverProvider definitionProvider referencesProvider
      completionProvider renameProvider documentSymbolProvider
      foldingRangeProvider signatureHelpProvider
      documentFormattingProvider semanticTokensProvider
    ]

    expected.each do |cap|
      assert caps.key?(cap), "expected capability #{cap} for full bridge"
    end
  end

  def test_semantic_token_legend_consistency
    legend = CodingAdventures::Ls00.semantic_token_legend

    refute_empty legend["tokenTypes"]
    refute_empty legend["tokenModifiers"]

    # Check that critical types are present.
    required_types = %w[keyword string number variable function]
    required_types.each do |rt|
      assert_includes legend["tokenTypes"], rt, "legend missing required type #{rt}"
    end
  end
end
