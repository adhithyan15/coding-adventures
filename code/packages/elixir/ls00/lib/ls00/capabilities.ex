defmodule Ls00.Capabilities do
  @moduledoc """
  Build LSP capabilities and encode semantic tokens.

  ## What Are Capabilities?

  During the LSP initialize handshake, the server sends back a "capabilities"
  object telling the editor which LSP features it supports. The editor uses this
  to decide which requests to send. If a capability is absent, the editor won't
  send the corresponding requests -- so no "Go to Definition" button appears
  unless definitionProvider is true.

  Building capabilities dynamically (based on the bridge module's exported
  functions) means the server is always honest about what it can do.

  ## Semantic Token Legend

  Semantic tokens use a compact binary encoding. Instead of sending
  `{"type":"keyword"}` per token, LSP sends an integer index into a legend.
  The legend must be declared in the capabilities so the editor knows what
  each index means.
  """

  alias Ls00.Types.SemanticToken

  @doc """
  Inspect the bridge module at runtime and return the LSP capabilities map.

  Uses `function_exported?/3` to check which optional callbacks the bridge
  module implements. Only advertises capabilities for features the bridge
  actually supports.
  """
  @spec build_capabilities(module()) :: map()
  def build_capabilities(bridge_module) do
    # textDocumentSync=2 means "incremental": the editor sends only changed
    # ranges, not the full file, on every keystroke.
    caps = %{"textDocumentSync" => 2}

    caps = if function_exported?(bridge_module, :hover, 2),
      do: Map.put(caps, "hoverProvider", true), else: caps

    caps = if function_exported?(bridge_module, :definition, 3),
      do: Map.put(caps, "definitionProvider", true), else: caps

    caps = if function_exported?(bridge_module, :references, 4),
      do: Map.put(caps, "referencesProvider", true), else: caps

    caps = if function_exported?(bridge_module, :completion, 2),
      do: Map.put(caps, "completionProvider", %{
        "triggerCharacters" => [" ", "."]
      }), else: caps

    caps = if function_exported?(bridge_module, :rename, 3),
      do: Map.put(caps, "renameProvider", true), else: caps

    caps = if function_exported?(bridge_module, :document_symbols, 1),
      do: Map.put(caps, "documentSymbolProvider", true), else: caps

    caps = if function_exported?(bridge_module, :folding_ranges, 1),
      do: Map.put(caps, "foldingRangeProvider", true), else: caps

    caps = if function_exported?(bridge_module, :signature_help, 2),
      do: Map.put(caps, "signatureHelpProvider", %{
        "triggerCharacters" => ["(", ","]
      }), else: caps

    caps = if function_exported?(bridge_module, :format, 1),
      do: Map.put(caps, "documentFormattingProvider", true), else: caps

    caps = if function_exported?(bridge_module, :semantic_tokens, 2),
      do: Map.put(caps, "semanticTokensProvider", %{
        "legend" => semantic_token_legend(),
        "full" => true
      }), else: caps

    caps
  end

  # ---------------------------------------------------------------------------
  # Semantic Token Legend
  # ---------------------------------------------------------------------------

  @doc """
  Return the full legend for all supported semantic token types and modifiers.

  The legend is sent once in the capabilities response. Afterwards, each
  semantic token is encoded as an integer index into this legend rather than
  a string. This makes the per-token encoding much smaller.

  The ordering matters: index 0 in `tokenTypes` corresponds to "namespace",
  index 1 to "type", etc. These match the standard LSP token types.
  """
  @spec semantic_token_legend() :: map()
  def semantic_token_legend do
    %{
      # Standard LSP token types (in the order VS Code expects them).
      "tokenTypes" => [
        "namespace",     # 0
        "type",          # 1
        "class",         # 2
        "enum",          # 3
        "interface",     # 4
        "struct",        # 5
        "typeParameter", # 6
        "parameter",     # 7
        "variable",      # 8
        "property",      # 9
        "enumMember",    # 10
        "event",         # 11
        "function",      # 12
        "method",        # 13
        "macro",         # 14
        "keyword",       # 15
        "modifier",      # 16
        "comment",       # 17
        "string",        # 18
        "number",        # 19
        "regexp",        # 20
        "operator",      # 21
        "decorator"      # 22
      ],
      # Standard LSP token modifiers (bitmask flags).
      "tokenModifiers" => [
        "declaration",    # bit 0
        "definition",     # bit 1
        "readonly",       # bit 2
        "static",         # bit 3
        "deprecated",     # bit 4
        "abstract",       # bit 5
        "async",          # bit 6
        "modification",   # bit 7
        "documentation",  # bit 8
        "defaultLibrary"  # bit 9
      ]
    }
  end

  @doc """
  Return the integer index for a semantic token type string.

  Returns `-1` if the type is not in the legend (the caller should skip such tokens).
  """
  @spec token_type_index(String.t()) :: integer()
  def token_type_index(token_type) do
    legend = semantic_token_legend()
    case Enum.find_index(legend["tokenTypes"], &(&1 == token_type)) do
      nil -> -1
      idx -> idx
    end
  end

  @doc """
  Return the bitmask for a list of modifier strings.

  The LSP semantic tokens encoding represents modifiers as a bitmask:
    - "declaration" -> bit 0 -> value 1
    - "definition"  -> bit 1 -> value 2
    - both          -> value 3 (bitwise OR)

  Unknown modifiers are silently ignored.
  """
  @spec token_modifier_mask([String.t()]) :: non_neg_integer()
  def token_modifier_mask(modifiers) do
    legend = semantic_token_legend()
    modifier_list = legend["tokenModifiers"]

    Enum.reduce(modifiers, 0, fn modifier, mask ->
      case Enum.find_index(modifier_list, &(&1 == modifier)) do
        nil -> mask
        idx -> Bitwise.bor(mask, Bitwise.bsl(1, idx))
      end
    end)
  end

  @doc """
  Convert a list of SemanticToken structs to the LSP compact integer encoding.

  ## The LSP Semantic Token Encoding

  LSP encodes semantic tokens as a flat array of integers, grouped in 5-tuples:

      [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]

  Where "delta" means: the difference from the PREVIOUS token's position.
  This delta encoding makes most values small (often 0 or 1), which compresses
  well and is efficient to parse.

  ## Example

  Three tokens on different lines:

      Token A: line=0, char=0, len=3, type="keyword",  modifiers=[]
      Token B: line=0, char=4, len=5, type="function", modifiers=["declaration"]
      Token C: line=1, char=0, len=8, type="variable", modifiers=[]

  Encoded as:

      [0, 0, 3, 15, 0,   # A: deltaLine=0, deltaChar=0
       0, 4, 5, 12, 1,   # B: deltaLine=0, deltaChar=4
       1, 0, 8,  8, 0]   # C: deltaLine=1, deltaChar=0

  Note: when deltaLine > 0, deltaStartChar is relative to column 0 of the new
  line (i.e., absolute for that line). When deltaLine == 0, deltaStartChar is
  relative to the previous token's start character.
  """
  @spec encode_semantic_tokens([SemanticToken.t()]) :: [integer()]
  def encode_semantic_tokens([]), do: []
  def encode_semantic_tokens(nil), do: []

  def encode_semantic_tokens(tokens) do
    # Sort by (line, character) ascending. The delta encoding requires tokens
    # to be in document order -- otherwise the deltas would be negative.
    sorted =
      tokens
      |> Enum.sort_by(fn tok -> {tok.line, tok.character} end)

    {data, _prev_line, _prev_char} =
      Enum.reduce(sorted, {[], 0, 0}, fn tok, {acc, prev_line, prev_char} ->
        type_idx = token_type_index(tok.token_type)

        if type_idx == -1 do
          # Unknown token type -- skip it.
          {acc, prev_line, prev_char}
        else
          delta_line = tok.line - prev_line

          delta_char =
            if delta_line == 0 do
              # Same line: character offset is relative to previous token.
              tok.character - prev_char
            else
              # Different line: character offset is absolute (relative to line start).
              tok.character
            end

          mod_mask = token_modifier_mask(tok.modifiers || [])

          five_tuple = [delta_line, delta_char, tok.length, type_idx, mod_mask]
          {acc ++ five_tuple, tok.line, tok.character}
        end
      end)

    data
  end
end
