defmodule Ls00.Types do
  @moduledoc """
  All shared LSP data types used across the server.

  These types mirror the LSP specification's TypeScript type definitions,
  translated to idiomatic Elixir structs. The LSP spec lives at:
  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/

  ## Coordinate System

  LSP uses a 0-based, line/character coordinate system. Line 0, character 0 is
  the very first character of the file. This differs from most editors (which
  display 1-based line numbers) and from most lexers (which emit 1-based tokens).
  The `Ls00.LanguageBridge` is responsible for converting.

  ## UTF-16 Code Units

  LSP's "character" offset is measured in UTF-16 CODE UNITS, not bytes or
  Unicode codepoints. This is a historical artifact: VS Code is built on
  TypeScript, which uses UTF-16 strings internally. See `Ls00.DocumentManager`
  for the conversion function and a detailed explanation of why this matters.
  """

  # ---------------------------------------------------------------------------
  # Position
  # ---------------------------------------------------------------------------

  defmodule Position do
    @moduledoc """
    A cursor position in a document.

    Both `line` and `character` are 0-based. `character` is measured in UTF-16
    code units (see the module doc above for why).

    ## Example

    In the string "hello world", the 'w' in world is at
    `%Position{line: 0, character: 6}`.
    """
    @enforce_keys [:line, :character]
    defstruct [:line, :character]

    @type t :: %__MODULE__{
            line: non_neg_integer(),
            character: non_neg_integer()
          }
  end

  # ---------------------------------------------------------------------------
  # Range
  # ---------------------------------------------------------------------------

  defmodule Range do
    @moduledoc """
    A span of text in a document, from `start` (inclusive) to `end_pos` (exclusive).

    Analogy: think of it like a text selection. `start` is where the cursor lands
    when you click, `end_pos` is where you drag to.
    """
    @enforce_keys [:start, :end_pos]
    defstruct [:start, :end_pos]

    @type t :: %__MODULE__{
            start: Ls00.Types.Position.t(),
            end_pos: Ls00.Types.Position.t()
          }
  end

  # ---------------------------------------------------------------------------
  # Location
  # ---------------------------------------------------------------------------

  defmodule Location do
    @moduledoc """
    A position in a specific file.

    URI uses the "file://" scheme, e.g., "file:///home/user/main.ex".
    """
    @enforce_keys [:uri, :range]
    defstruct [:uri, :range]

    @type t :: %__MODULE__{
            uri: String.t(),
            range: Ls00.Types.Range.t()
          }
  end

  # ---------------------------------------------------------------------------
  # Diagnostic Severity
  # ---------------------------------------------------------------------------
  #
  # These match the LSP integer codes:
  #   1 = Error   (red squiggles)
  #   2 = Warning (yellow squiggles)
  #   3 = Information (blue)
  #   4 = Hint (subtle)

  @doc "Error severity (1) -- a hard error; the code cannot run or compile."
  def severity_error, do: 1

  @doc "Warning severity (2) -- potentially problematic, but not blocking."
  def severity_warning, do: 2

  @doc "Information severity (3) -- informational message."
  def severity_information, do: 3

  @doc "Hint severity (4) -- a suggestion (e.g., 'consider using const')."
  def severity_hint, do: 4

  # ---------------------------------------------------------------------------
  # Diagnostic
  # ---------------------------------------------------------------------------

  defmodule Diagnostic do
    @moduledoc """
    An error, warning, or hint to display in the editor.

    The editor renders diagnostics as underlined squiggles, with the message
    shown on hover. Red squiggles = Error, yellow = Warning, blue = Info.
    """
    @enforce_keys [:range, :severity, :message]
    defstruct [:range, :severity, :message, :code]

    @type t :: %__MODULE__{
            range: Ls00.Types.Range.t(),
            severity: integer(),
            message: String.t(),
            code: String.t() | nil
          }
  end

  # ---------------------------------------------------------------------------
  # Token
  # ---------------------------------------------------------------------------

  defmodule Token do
    @moduledoc """
    A single lexical token from the language's lexer.

    The bridge's `tokenize/1` callback returns a list of these. The LSP server
    uses tokens to provide semantic syntax highlighting (SemanticTokensProvider).

    Note: `line` and `column` are 1-based (matching most lexers). The bridge
    must convert to 0-based when building `SemanticToken` values for the LSP
    response.
    """
    @enforce_keys [:type, :value, :line, :column]
    defstruct [:type, :value, :line, :column]

    @type t :: %__MODULE__{
            type: String.t(),
            value: String.t(),
            line: pos_integer(),
            column: pos_integer()
          }
  end

  # ---------------------------------------------------------------------------
  # TextEdit
  # ---------------------------------------------------------------------------

  defmodule TextEdit do
    @moduledoc """
    A single text replacement in a document.

    Used by formatting (replace the whole file) and rename (replace each
    occurrence). `new_text` replaces the content at `range`. If `new_text` is
    empty, the range is deleted.
    """
    @enforce_keys [:range, :new_text]
    defstruct [:range, :new_text]

    @type t :: %__MODULE__{
            range: Ls00.Types.Range.t(),
            new_text: String.t()
          }
  end

  # ---------------------------------------------------------------------------
  # WorkspaceEdit
  # ---------------------------------------------------------------------------

  defmodule WorkspaceEdit do
    @moduledoc """
    Groups TextEdits across potentially multiple files.

    For rename operations that affect a single file, `changes` will have one key.
    For multi-file projects, a rename may produce edits across many files.
    """
    @enforce_keys [:changes]
    defstruct [:changes]

    @type t :: %__MODULE__{
            changes: %{String.t() => [Ls00.Types.TextEdit.t()]}
          }
  end

  # ---------------------------------------------------------------------------
  # HoverResult
  # ---------------------------------------------------------------------------

  defmodule HoverResult do
    @moduledoc """
    The content to show in the hover popup.

    `contents` is Markdown text. VS Code renders it with syntax highlighting,
    bold/italic, code blocks, etc. `range` is optional -- if set, it highlights
    the symbol in the editor when the hover popup is shown.
    """
    @enforce_keys [:contents]
    defstruct [:contents, :range]

    @type t :: %__MODULE__{
            contents: String.t(),
            range: Ls00.Types.Range.t() | nil
          }
  end

  # ---------------------------------------------------------------------------
  # CompletionItemKind
  # ---------------------------------------------------------------------------
  #
  # These classify completion items so the editor can show the right icon
  # (function icon, variable icon, keyword icon, etc.).

  @completion_text 1
  @completion_method 2
  @completion_function 3
  @completion_constructor 4
  @completion_field 5
  @completion_variable 6
  @completion_class 7
  @completion_interface 8
  @completion_module 9
  @completion_property 10
  @completion_unit 11
  @completion_value 12
  @completion_enum 13
  @completion_keyword 14
  @completion_snippet 15
  @completion_color 16
  @completion_file 17
  @completion_reference 18
  @completion_folder 19
  @completion_enum_member 20
  @completion_constant 21
  @completion_struct 22
  @completion_event 23
  @completion_operator 24
  @completion_type_parameter 25

  def completion_text, do: @completion_text
  def completion_method, do: @completion_method
  def completion_function, do: @completion_function
  def completion_constructor, do: @completion_constructor
  def completion_field, do: @completion_field
  def completion_variable, do: @completion_variable
  def completion_class, do: @completion_class
  def completion_interface, do: @completion_interface
  def completion_module, do: @completion_module
  def completion_property, do: @completion_property
  def completion_unit, do: @completion_unit
  def completion_value, do: @completion_value
  def completion_enum, do: @completion_enum
  def completion_keyword, do: @completion_keyword
  def completion_snippet, do: @completion_snippet
  def completion_color, do: @completion_color
  def completion_file, do: @completion_file
  def completion_reference, do: @completion_reference
  def completion_folder, do: @completion_folder
  def completion_enum_member, do: @completion_enum_member
  def completion_constant, do: @completion_constant
  def completion_struct, do: @completion_struct
  def completion_event, do: @completion_event
  def completion_operator, do: @completion_operator
  def completion_type_parameter, do: @completion_type_parameter

  # ---------------------------------------------------------------------------
  # CompletionItem
  # ---------------------------------------------------------------------------

  defmodule CompletionItem do
    @moduledoc """
    A single autocomplete suggestion.

    When the user triggers autocomplete (e.g., by pressing Ctrl+Space or typing
    after a dot), the editor shows a dropdown list of CompletionItems.
    """
    @enforce_keys [:label]
    defstruct [:label, :kind, :detail, :documentation, :insert_text, :insert_text_format]

    @type t :: %__MODULE__{
            label: String.t(),
            kind: integer() | nil,
            detail: String.t() | nil,
            documentation: String.t() | nil,
            insert_text: String.t() | nil,
            insert_text_format: integer() | nil
          }
  end

  # ---------------------------------------------------------------------------
  # SemanticToken
  # ---------------------------------------------------------------------------

  defmodule SemanticToken do
    @moduledoc """
    One token's contribution to the semantic highlighting pass.

    Semantic tokens are the "second pass" of syntax highlighting. The editor's
    grammar-based highlighter (TextMate/tmLanguage) does a fast regex pass first.
    Semantic tokens layer on top with accurate, context-aware type information.

    `line` and `character` are 0-based. `token_type` and `modifiers` reference
    entries in the legend returned by `Ls00.Capabilities.semantic_token_legend/0`.
    """
    @enforce_keys [:line, :character, :length, :token_type]
    defstruct [:line, :character, :length, :token_type, modifiers: []]

    @type t :: %__MODULE__{
            line: non_neg_integer(),
            character: non_neg_integer(),
            length: non_neg_integer(),
            token_type: String.t(),
            modifiers: [String.t()]
          }
  end

  # ---------------------------------------------------------------------------
  # SymbolKind
  # ---------------------------------------------------------------------------
  #
  # These classify document symbols for the outline panel.
  # They match the LSP integer codes (1-based).

  @symbol_file 1
  @symbol_module 2
  @symbol_namespace 3
  @symbol_package 4
  @symbol_class 5
  @symbol_method 6
  @symbol_property 7
  @symbol_field 8
  @symbol_constructor 9
  @symbol_enum 10
  @symbol_interface 11
  @symbol_function 12
  @symbol_variable 13
  @symbol_constant 14
  @symbol_string 15
  @symbol_number 16
  @symbol_boolean 17
  @symbol_array 18
  @symbol_object 19
  @symbol_key 20
  @symbol_null 21
  @symbol_enum_member 22
  @symbol_struct 23
  @symbol_event 24
  @symbol_operator 25
  @symbol_type_parameter 26

  def symbol_file, do: @symbol_file
  def symbol_module, do: @symbol_module
  def symbol_namespace, do: @symbol_namespace
  def symbol_package, do: @symbol_package
  def symbol_class, do: @symbol_class
  def symbol_method, do: @symbol_method
  def symbol_property, do: @symbol_property
  def symbol_field, do: @symbol_field
  def symbol_constructor, do: @symbol_constructor
  def symbol_enum, do: @symbol_enum
  def symbol_interface, do: @symbol_interface
  def symbol_function, do: @symbol_function
  def symbol_variable, do: @symbol_variable
  def symbol_constant, do: @symbol_constant
  def symbol_string, do: @symbol_string
  def symbol_number, do: @symbol_number
  def symbol_boolean, do: @symbol_boolean
  def symbol_array, do: @symbol_array
  def symbol_object, do: @symbol_object
  def symbol_key, do: @symbol_key
  def symbol_null, do: @symbol_null
  def symbol_enum_member, do: @symbol_enum_member
  def symbol_struct, do: @symbol_struct
  def symbol_event, do: @symbol_event
  def symbol_operator, do: @symbol_operator
  def symbol_type_parameter, do: @symbol_type_parameter

  # ---------------------------------------------------------------------------
  # DocumentSymbol
  # ---------------------------------------------------------------------------

  defmodule DocumentSymbol do
    @moduledoc """
    One entry in the document outline panel.

    The outline shows a tree of named symbols (functions, classes, variables).
    `children` allows nesting: a class symbol can have method symbols as children.

    `range` covers the entire symbol (including its body). `selection_range` is
    the smaller range of just the symbol's name.
    """
    @enforce_keys [:name, :kind, :range, :selection_range]
    defstruct [:name, :kind, :range, :selection_range, children: []]

    @type t :: %__MODULE__{
            name: String.t(),
            kind: integer(),
            range: Ls00.Types.Range.t(),
            selection_range: Ls00.Types.Range.t(),
            children: [t()]
          }
  end

  # ---------------------------------------------------------------------------
  # FoldingRange
  # ---------------------------------------------------------------------------

  defmodule FoldingRange do
    @moduledoc """
    A collapsible region of the document.

    The editor shows a collapse arrow in the gutter next to `start_line`. When
    collapsed, lines start_line+1 through end_line are hidden. `kind` is one of
    "region", "imports", or "comment".
    """
    @enforce_keys [:start_line, :end_line]
    defstruct [:start_line, :end_line, :kind]

    @type t :: %__MODULE__{
            start_line: non_neg_integer(),
            end_line: non_neg_integer(),
            kind: String.t() | nil
          }
  end

  # ---------------------------------------------------------------------------
  # ParameterInformation
  # ---------------------------------------------------------------------------

  defmodule ParameterInformation do
    @moduledoc "One parameter in a function signature."
    @enforce_keys [:label]
    defstruct [:label, :documentation]

    @type t :: %__MODULE__{
            label: String.t(),
            documentation: String.t() | nil
          }
  end

  # ---------------------------------------------------------------------------
  # SignatureInformation
  # ---------------------------------------------------------------------------

  defmodule SignatureInformation do
    @moduledoc "One function overload's full signature."
    @enforce_keys [:label]
    defstruct [:label, :documentation, parameters: []]

    @type t :: %__MODULE__{
            label: String.t(),
            documentation: String.t() | nil,
            parameters: [Ls00.Types.ParameterInformation.t()]
          }
  end

  # ---------------------------------------------------------------------------
  # SignatureHelpResult
  # ---------------------------------------------------------------------------

  defmodule SignatureHelpResult do
    @moduledoc """
    Shown as a tooltip when the user is typing a function call.

    It shows the function signature with the current parameter highlighted.
    `active_signature` indexes into `signatures`. `active_parameter` indexes
    into that signature's parameters.
    """
    @enforce_keys [:signatures, :active_signature, :active_parameter]
    defstruct [:signatures, :active_signature, :active_parameter]

    @type t :: %__MODULE__{
            signatures: [Ls00.Types.SignatureInformation.t()],
            active_signature: non_neg_integer(),
            active_parameter: non_neg_integer()
          }
  end

  # ---------------------------------------------------------------------------
  # TextChange (internal, used by DocumentManager)
  # ---------------------------------------------------------------------------

  defmodule TextChange do
    @moduledoc """
    Describes one incremental change to a document.

    If `range` is nil, `new_text` replaces the ENTIRE document content (full sync).
    If `range` is non-nil, `new_text` replaces just the specified range
    (incremental sync).
    """
    @enforce_keys [:new_text]
    defstruct [:range, :new_text]

    @type t :: %__MODULE__{
            range: Ls00.Types.Range.t() | nil,
            new_text: String.t()
          }
  end

  # ---------------------------------------------------------------------------
  # Document (internal, used by DocumentManager)
  # ---------------------------------------------------------------------------

  defmodule Document do
    @moduledoc """
    An open file tracked by the DocumentManager.
    """
    @enforce_keys [:uri, :text, :version]
    defstruct [:uri, :text, :version]

    @type t :: %__MODULE__{
            uri: String.t(),
            text: String.t(),
            version: integer()
          }
  end

  # ---------------------------------------------------------------------------
  # ParseResult (internal, used by ParseCache)
  # ---------------------------------------------------------------------------

  defmodule ParseResult do
    @moduledoc """
    The outcome of parsing one version of a document.

    Even on parse error, we store the partial AST and diagnostics so that other
    features (hover, folding, symbols) can still work on the valid portions.
    """
    defstruct [:ast, diagnostics: [], err: nil]

    @type t :: %__MODULE__{
            ast: any(),
            diagnostics: [Ls00.Types.Diagnostic.t()],
            err: String.t() | nil
          }
  end
end
