# frozen_string_literal: true

# ================================================================
# CodingAdventures::Ls00 — LSP Data Types
# ================================================================
#
# These types mirror the LSP specification's TypeScript type definitions,
# translated to idiomatic Ruby using Struct classes.
#
# The LSP spec lives at:
# https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
#
# # Coordinate System
#
# LSP uses a 0-based, line/character coordinate system. Line 0, character 0 is
# the very first character of the file. This differs from most editors (which
# display 1-based line numbers) and from our lexer (which emits 1-based tokens).
# The LanguageBridge is responsible for converting.
#
# # UTF-16 Code Units
#
# LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
# Unicode codepoints. This is a historical artifact: VS Code is built on
# TypeScript, which uses UTF-16 strings internally. See document_manager.rb
# for the conversion function and a detailed explanation of why this matters.
#
# ================================================================

module CodingAdventures
  module Ls00
    # Position is a cursor position in a document.
    #
    # Both +line+ and +character+ are 0-based. Character is measured in UTF-16
    # code units (see the module doc above for why).
    #
    # Example: in the string "hello guitar-emoji world", the guitar emoji
    # occupies UTF-16 characters 6 and 7 (it requires two UTF-16 surrogates).
    # "world" starts at UTF-16 character 8.
    Position = Struct.new(:line, :character, keyword_init: true)

    # Range is a span of text in a document, from +start+ (inclusive) to
    # +end_pos+ (exclusive).
    #
    # Analogy: think of it like a text selection. Start is where the cursor
    # lands when you click, end_pos is where you drag to.
    #
    # Note: we use +end_pos+ instead of +end+ because +end+ is a Ruby
    # reserved word.
    LspRange = Struct.new(:start, :end_pos, keyword_init: true)

    # Location is a position in a specific file.
    #
    # URI uses the "file://" scheme, e.g., "file:///home/user/main.rb".
    Location = Struct.new(:uri, :range, keyword_init: true)

    # DiagnosticSeverity represents how serious a diagnostic is.
    # These match the LSP integer codes.
    module DiagnosticSeverity
      # A hard error; the code cannot run or compile.
      ERROR = 1
      # Potentially problematic, but not blocking.
      WARNING = 2
      # Informational message.
      INFORMATION = 3
      # A suggestion (e.g., "consider using const").
      HINT = 4
    end

    # Diagnostic is an error, warning, or hint to display in the editor.
    #
    # The editor renders diagnostics as underlined squiggles, with the message
    # shown on hover. Red squiggles = Error, yellow = Warning, blue = Info.
    Diagnostic = Struct.new(:range, :severity, :message, :code, keyword_init: true)

    # Token is a single lexical token from the language's lexer.
    #
    # The bridge's +tokenize+ method returns an array of these. The LSP server
    # uses tokens to provide semantic syntax highlighting (SemanticTokensProvider).
    #
    # Note: +line+ and +column+ are 1-based (matching most lexers). The bridge
    # must convert to 0-based when building SemanticToken values for the LSP
    # response.
    Token = Struct.new(:type, :value, :line, :column, keyword_init: true)

    # TextEdit is a single text replacement in a document.
    #
    # Used by formatting (replace the whole file) and rename (replace each
    # occurrence). +new_text+ replaces the content at +range+. If +new_text+
    # is empty, the range is deleted.
    TextEdit = Struct.new(:range, :new_text, keyword_init: true)

    # WorkspaceEdit groups TextEdits across potentially multiple files.
    #
    # For rename operations that affect a single file, +changes+ will have one
    # key. For multi-file projects, a rename may produce edits across many files.
    WorkspaceEdit = Struct.new(:changes, keyword_init: true) # changes: { uri => [TextEdit] }

    # HoverResult is the content to show in the hover popup.
    #
    # +contents+ is Markdown text. VS Code renders it with syntax highlighting,
    # bold/italic, code blocks, etc. +range+ is optional -- if set, it highlights
    # the symbol in the editor when the hover popup is shown.
    HoverResult = Struct.new(:contents, :range, keyword_init: true)

    # CompletionItemKind classifies completion items so the editor can show
    # the right icon (function icon, variable icon, keyword icon, etc.).
    module CompletionItemKind
      TEXT          = 1
      METHOD        = 2
      FUNCTION      = 3
      CONSTRUCTOR   = 4
      FIELD         = 5
      VARIABLE      = 6
      CLASS         = 7
      INTERFACE     = 8
      MODULE        = 9
      PROPERTY      = 10
      UNIT          = 11
      VALUE         = 12
      ENUM          = 13
      KEYWORD       = 14
      SNIPPET       = 15
      COLOR         = 16
      FILE          = 17
      REFERENCE     = 18
      FOLDER        = 19
      ENUM_MEMBER   = 20
      CONSTANT      = 21
      STRUCT        = 22
      EVENT         = 23
      OPERATOR      = 24
      TYPE_PARAMETER = 25
    end

    # CompletionItem is a single autocomplete suggestion.
    #
    # When the user triggers autocomplete (e.g., by pressing Ctrl+Space or
    # typing after a dot), the editor shows a dropdown list of CompletionItems.
    CompletionItem = Struct.new(:label, :kind, :detail, :documentation,
                               :insert_text, :insert_text_format,
                               keyword_init: true)

    # SemanticToken is one token's contribution to the semantic highlighting pass.
    #
    # Semantic tokens are the "second pass" of syntax highlighting. The editor's
    # grammar-based highlighter (TextMate/tmLanguage) does a fast regex pass
    # first. Semantic tokens layer on top with accurate, context-aware type info.
    #
    # +line+ and +character+ are 0-based. +token_type+ and +modifiers+ reference
    # entries in the legend returned by SemanticTokenLegend (see capabilities.rb).
    SemanticToken = Struct.new(:line, :character, :length, :token_type,
                              :modifiers, keyword_init: true)

    # SymbolKind classifies document symbols for the outline panel.
    # These match the LSP integer codes (1-based).
    module SymbolKind
      FILE           = 1
      MODULE         = 2
      NAMESPACE      = 3
      PACKAGE        = 4
      CLASS          = 5
      METHOD         = 6
      PROPERTY       = 7
      FIELD          = 8
      CONSTRUCTOR    = 9
      ENUM           = 10
      INTERFACE      = 11
      FUNCTION       = 12
      VARIABLE       = 13
      CONSTANT       = 14
      STRING         = 15
      NUMBER         = 16
      BOOLEAN        = 17
      ARRAY          = 18
      OBJECT         = 19
      KEY            = 20
      NULL           = 21
      ENUM_MEMBER    = 22
      STRUCT         = 23
      EVENT          = 24
      OPERATOR       = 25
      TYPE_PARAMETER = 26
    end

    # DocumentSymbol is one entry in the document outline panel.
    #
    # The outline shows a tree of named symbols (functions, classes, variables).
    # +children+ allows nesting: a class symbol can have method symbols as children.
    #
    # +range+ covers the entire symbol (including its body). +selection_range+
    # is the smaller range of just the symbol's name.
    DocumentSymbol = Struct.new(:name, :kind, :range, :selection_range,
                               :children, keyword_init: true)

    # FoldingRange is a collapsible region of the document.
    #
    # The editor shows a collapse arrow in the gutter next to +start_line+. When
    # collapsed, lines start_line+1 through end_line are hidden. +kind+ is one of
    # "region", "imports", or "comment".
    FoldingRange = Struct.new(:start_line, :end_line, :kind, keyword_init: true)

    # ParameterInformation is one parameter in a function signature.
    ParameterInformation = Struct.new(:label, :documentation, keyword_init: true)

    # SignatureInformation is one function overload's full signature.
    SignatureInformation = Struct.new(:label, :documentation, :parameters,
                                     keyword_init: true)

    # SignatureHelpResult is shown as a tooltip when the user is typing a
    # function call.
    #
    # It shows the function signature with the current parameter highlighted.
    # +active_signature+ indexes into +signatures+. +active_parameter+ indexes
    # into that signature's +parameters+.
    SignatureHelpResult = Struct.new(:signatures, :active_signature,
                                    :active_parameter, keyword_init: true)
  end
end
