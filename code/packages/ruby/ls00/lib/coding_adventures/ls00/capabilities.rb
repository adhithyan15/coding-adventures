# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00 — Capabilities, SemanticTokenLegend, Encoding
# ================================================================
#
# # What Are Capabilities?
#
# During the LSP initialize handshake, the server sends back a
# "capabilities" object telling the editor which LSP features it
# supports. The editor uses this to decide which requests to send.
# If a capability is absent, the editor won't send the corresponding
# requests -- so no "Go to Definition" button appears unless
# definitionProvider is true.
#
# Building capabilities dynamically (based on the bridge's respond_to?
# checks) means the server is always honest about what it can do.
#
# # Semantic Token Legend
#
# Semantic tokens use a compact binary encoding. Instead of sending
# {"type":"keyword"} per token, LSP sends an integer index into a legend.
# The legend must be declared in the capabilities so the editor knows
# what each index means.
#
# ================================================================

module CodingAdventures
  module Ls00
    # build_capabilities inspects the bridge at runtime and returns the LSP
    # capabilities hash to include in the initialize response.
    #
    # Uses Ruby's +respond_to?+ to check which optional methods the bridge
    # implements. Only advertises capabilities for features the bridge
    # actually supports.
    def self.build_capabilities(bridge)
      # textDocumentSync=2 means "incremental": the editor sends only changed
      # ranges, not the full file, on every keystroke.
      caps = { "textDocumentSync" => 2 }

      caps["hoverProvider"] = true if bridge.respond_to?(:hover)
      caps["definitionProvider"] = true if bridge.respond_to?(:definition)
      caps["referencesProvider"] = true if bridge.respond_to?(:references)

      if bridge.respond_to?(:completion)
        caps["completionProvider"] = { "triggerCharacters" => [" ", "."] }
      end

      caps["renameProvider"] = true if bridge.respond_to?(:rename)
      caps["documentSymbolProvider"] = true if bridge.respond_to?(:document_symbols)
      caps["foldingRangeProvider"] = true if bridge.respond_to?(:folding_ranges)

      if bridge.respond_to?(:signature_help)
        caps["signatureHelpProvider"] = { "triggerCharacters" => ["(", ","] }
      end

      caps["documentFormattingProvider"] = true if bridge.respond_to?(:format)

      if bridge.respond_to?(:semantic_tokens)
        caps["semanticTokensProvider"] = {
          "legend" => semantic_token_legend,
          "full" => true
        }
      end

      caps
    end

    # SemanticTokenLegendData holds the legend arrays for semantic tokens.
    SemanticTokenLegendData = Struct.new(:token_types, :token_modifiers, keyword_init: true)

    # semantic_token_legend returns the full legend for all supported semantic
    # token types and modifiers.
    #
    # The ordering matters: index 0 in token_types corresponds to "namespace",
    # index 1 to "type", etc. These match the standard LSP token types.
    def self.semantic_token_legend
      {
        "tokenTypes" => TOKEN_TYPES,
        "tokenModifiers" => TOKEN_MODIFIERS
      }
    end

    # Standard LSP token types (in the order VS Code expects them).
    # Source: https://code.visualstudio.com/api/language-extensions/semantic-highlight-guide
    TOKEN_TYPES = [
      "namespace",      # 0
      "type",           # 1
      "class",          # 2
      "enum",           # 3
      "interface",      # 4
      "struct",         # 5
      "typeParameter",  # 6
      "parameter",      # 7
      "variable",       # 8
      "property",       # 9
      "enumMember",     # 10
      "event",          # 11
      "function",       # 12
      "method",         # 13
      "macro",          # 14
      "keyword",        # 15
      "modifier",       # 16
      "comment",        # 17
      "string",         # 18
      "number",         # 19
      "regexp",         # 20
      "operator",       # 21
      "decorator"       # 22
    ].freeze

    # Standard LSP token modifiers (bitmask flags).
    # tokenModifier[0] = "declaration" -> bit 0 (value 1)
    # tokenModifier[1] = "definition"  -> bit 1 (value 2)
    TOKEN_MODIFIERS = [
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
    ].freeze

    # token_type_index returns the integer index for a semantic token type
    # string. Returns -1 if the type is not in the legend.
    def self.token_type_index(token_type)
      idx = TOKEN_TYPES.index(token_type)
      idx.nil? ? -1 : idx
    end

    # token_modifier_mask returns the bitmask for a list of modifier strings.
    #
    # The LSP semantic tokens encoding represents modifiers as a bitmask:
    #   "declaration" -> bit 0 -> value 1
    #   "definition"  -> bit 1 -> value 2
    #   both          -> value 3 (bitwise OR)
    #
    # Unknown modifiers are silently ignored.
    def self.token_modifier_mask(modifiers)
      return 0 if modifiers.nil? || modifiers.empty?

      mask = 0
      modifiers.each do |mod|
        idx = TOKEN_MODIFIERS.index(mod)
        mask |= (1 << idx) if idx
      end
      mask
    end

    # encode_semantic_tokens converts an array of SemanticToken values to the
    # LSP compact integer encoding.
    #
    # # The LSP Semantic Token Encoding
    #
    # LSP encodes semantic tokens as a flat array of integers, grouped in
    # 5-tuples:
    #
    #   [deltaLine, deltaStartChar, length, tokenTypeIndex, tokenModifierBitmask, ...]
    #
    # Where "delta" means: the difference from the PREVIOUS token's position.
    # This delta encoding makes most values small (often 0 or 1).
    #
    # Note: when deltaLine > 0, deltaStartChar is relative to column 0 of the
    # new line. When deltaLine == 0, deltaStartChar is relative to the previous
    # token's start character.
    def self.encode_semantic_tokens(tokens)
      return [] if tokens.nil? || tokens.empty?

      # Sort by (line, character) ascending. The delta encoding requires tokens
      # to be in document order.
      sorted = tokens.sort_by { |t| [t.line, t.character] }

      data = []
      prev_line = 0
      prev_char = 0

      sorted.each do |tok|
        type_idx = token_type_index(tok.token_type)
        next if type_idx == -1 # unknown token type -- skip

        delta_line = tok.line - prev_line
        delta_char = if delta_line == 0
                       # Same line: character offset is relative to previous token.
                       tok.character - prev_char
                     else
                       # Different line: character offset is absolute.
                       tok.character
                     end

        mod_mask = token_modifier_mask(tok.modifiers)

        data.push(delta_line, delta_char, tok.length, type_idx, mod_mask)

        prev_line = tok.line
        prev_char = tok.character
      end

      data
    end
  end
end
