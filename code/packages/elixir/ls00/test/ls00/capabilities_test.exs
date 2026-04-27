defmodule Ls00.CapabilitiesTest do
  use ExUnit.Case, async: true

  @moduledoc """
  Tests for capability building and detection.
  """

  alias Ls00.Capabilities

  # MinimalBridge: only required callbacks.
  defmodule MinimalBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(_source), do: {:ok, []}

    @impl true
    def parse(source), do: {:ok, source, []}
  end

  # MockBridge: implements hover and document_symbols.
  defmodule MockBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(_source), do: {:ok, []}

    @impl true
    def parse(source), do: {:ok, source, []}

    @impl true
    def hover(_ast, _pos) do
      {:ok, %Ls00.Types.HoverResult{contents: "test hover"}}
    end

    @impl true
    def document_symbols(_ast) do
      {:ok, []}
    end
  end

  # FullBridge: implements all optional callbacks.
  defmodule FullBridge do
    @behaviour Ls00.LanguageBridge

    @impl true
    def tokenize(_source), do: {:ok, []}

    @impl true
    def parse(source), do: {:ok, source, []}

    @impl true
    def hover(_ast, _pos), do: {:ok, nil}

    @impl true
    def definition(_ast, _pos, _uri), do: {:ok, nil}

    @impl true
    def references(_ast, _pos, _uri, _include_decl), do: {:ok, []}

    @impl true
    def completion(_ast, _pos), do: {:ok, []}

    @impl true
    def rename(_ast, _pos, _new_name), do: {:ok, nil}

    @impl true
    def semantic_tokens(_source, _tokens), do: {:ok, []}

    @impl true
    def document_symbols(_ast), do: {:ok, []}

    @impl true
    def folding_ranges(_ast), do: {:ok, []}

    @impl true
    def signature_help(_ast, _pos), do: {:ok, nil}

    @impl true
    def format(_source), do: {:ok, []}
  end

  test "minimal bridge: only textDocumentSync advertised" do
    caps = Capabilities.build_capabilities(MinimalBridge)

    assert caps["textDocumentSync"] == 2

    # Optional capabilities should NOT be present
    optional = [
      "hoverProvider", "definitionProvider", "referencesProvider",
      "completionProvider", "renameProvider", "documentSymbolProvider",
      "foldingRangeProvider", "signatureHelpProvider",
      "documentFormattingProvider", "semanticTokensProvider"
    ]

    for cap <- optional do
      refute Map.has_key?(caps, cap),
        "minimal bridge should not advertise #{cap}"
    end
  end

  test "mock bridge: advertises hover and documentSymbol" do
    caps = Capabilities.build_capabilities(MockBridge)

    assert caps["hoverProvider"] == true
    assert caps["documentSymbolProvider"] == true
    refute Map.has_key?(caps, "definitionProvider")
  end

  test "full bridge: all capabilities advertised" do
    caps = Capabilities.build_capabilities(FullBridge)

    expected = [
      "textDocumentSync",
      "hoverProvider",
      "definitionProvider",
      "referencesProvider",
      "completionProvider",
      "renameProvider",
      "documentSymbolProvider",
      "foldingRangeProvider",
      "signatureHelpProvider",
      "documentFormattingProvider",
      "semanticTokensProvider"
    ]

    for cap <- expected do
      assert Map.has_key?(caps, cap),
        "expected capability #{cap} for full bridge"
    end
  end

  test "semantic tokens provider includes legend and full flag" do
    caps = Capabilities.build_capabilities(FullBridge)
    stp = caps["semanticTokensProvider"]

    assert stp["full"] == true
    assert is_map(stp["legend"])
    assert is_list(stp["legend"]["tokenTypes"])
    assert is_list(stp["legend"]["tokenModifiers"])
  end

  test "semantic token legend has required types" do
    legend = Capabilities.semantic_token_legend()

    assert length(legend["tokenTypes"]) > 0
    assert length(legend["tokenModifiers"]) > 0

    required = ["keyword", "string", "number", "variable", "function"]
    for rt <- required do
      assert rt in legend["tokenTypes"],
        "legend missing required type #{rt}"
    end
  end
end
