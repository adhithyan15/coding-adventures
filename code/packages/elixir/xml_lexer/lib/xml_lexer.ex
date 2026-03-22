defmodule CodingAdventures.XmlLexer do
  @moduledoc """
  XML Lexer — Tokenizes XML using pattern groups and callback hooks.

  This module is the Elixir port of the Python `xml_lexer` package. It reads
  `xml.tokens` from the shared grammars directory and uses `GrammarLexer.tokenize/3`
  with an on-token callback to handle XML's context-sensitive lexical structure.

  ## The Problem

  XML is context-sensitive at the lexical level. The same character has
  different meaning depending on position:

  - `=` is an attribute delimiter inside `<tag attr="val">`
  - `=` is plain text content outside tags: `1 + 1 = 2`

  A flat pattern list cannot distinguish these contexts. Pattern groups
  solve this by defining separate sets of patterns for each context, and
  a callback function switches between them at runtime.

  ## How It Works

  The `xml.tokens` grammar defines 5 pattern groups:

  - **default** (implicit): Text content, entity refs, tag openers
  - **tag**: Tag names, attributes, equals, quoted values, closers
  - **comment**: Comment text and `-->` delimiter
  - **cdata**: Raw text and `]]>` delimiter
  - **pi**: Processing instruction target, text, and `?>` delimiter

  The callback (`xml_on_token/2`) fires after each token match and returns
  a list of action tuples that control the lexer's group stack and skip
  behaviour:

      default ──OPEN_TAG_START──> tag ──TAG_CLOSE──> default
              ──CLOSE_TAG_START─> tag ──SELF_CLOSE─> default
              ──COMMENT_START───> comment ──COMMENT_END──> default
              ──CDATA_START─────> cdata ──CDATA_END──> default
              ──PI_START────────> pi ──PI_END──> default

  For comment, CDATA, and PI groups, the callback also disables skip
  patterns (so whitespace is preserved as content) and re-enables them
  when leaving the group.

  ## Usage

      {:ok, tokens} = CodingAdventures.XmlLexer.tokenize("<p>Hello &amp; world</p>")
  """

  alias CodingAdventures.GrammarTools.TokenGrammar
  alias CodingAdventures.Lexer.GrammarLexer

  # ---------------------------------------------------------------------------
  # Grammar File Location
  # ---------------------------------------------------------------------------
  #
  # The xml.tokens file lives in code/grammars/ at the repository root.
  # From this file's location (lib/xml_lexer.ex), we navigate:
  #   lib/ -> xml_lexer/ -> elixir/ -> packages/ -> code/ -> grammars/

  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  # ---------------------------------------------------------------------------
  # XML On-Token Callback
  # ---------------------------------------------------------------------------
  #
  # This callback drives the group transitions. It is a pure function of
  # the token type -- it examines the token and returns a list of action
  # tuples that the lexer applies after the callback returns.
  #
  # The pattern is simple:
  # - Opening delimiters push a group
  # - Closing delimiters pop the group
  # - Comment/CDATA/PI groups disable skip (whitespace is content)
  # ---------------------------------------------------------------------------

  @doc """
  On-token callback that switches pattern groups for XML tokenization.

  This function fires after each token match. It examines the token type
  and returns action tuples for group stack manipulation:

  - `OPEN_TAG_START` (`<`) or `CLOSE_TAG_START` (`</`):
    Push the "tag" group so the lexer recognizes tag names, attributes,
    and tag closers.

  - `TAG_CLOSE` (`>`) or `SELF_CLOSE` (`/>`):
    Pop the "tag" group to return to default (text content).

  - `COMMENT_START` (`<!--`):
    Push "comment" group and disable skip (whitespace is significant).

  - `COMMENT_END` (`-->`):
    Pop "comment" group and re-enable skip.

  - `CDATA_START` (`<![CDATA[`):
    Push "cdata" group and disable skip.

  - `CDATA_END` (`]]>`):
    Pop "cdata" group and re-enable skip.

  - `PI_START` (`<?`):
    Push "pi" group and disable skip.

  - `PI_END` (`?>`):
    Pop "pi" group and re-enable skip.

  Returns `[]` (no actions) for all other token types.
  """
  @spec xml_on_token(CodingAdventures.Lexer.Token.t(), GrammarLexer.LexerContext.t()) :: list()
  def xml_on_token(token, _ctx) do
    case token.type do
      # --- Tag boundaries ---
      #
      # When we see `<` or `</`, we're entering a tag. Push the "tag"
      # group so the lexer switches to recognizing tag names, attribute
      # names, equals signs, quoted values, and closing delimiters.
      "OPEN_TAG_START" -> [{:push_group, "tag"}]
      "CLOSE_TAG_START" -> [{:push_group, "tag"}]

      # When we see `>` or `/>`, the tag is complete. Pop back to the
      # default group where text content and entity references are
      # recognized.
      "TAG_CLOSE" -> [:pop_group]
      "SELF_CLOSE" -> [:pop_group]

      # --- Comment boundaries ---
      #
      # Comments preserve all whitespace -- spaces, tabs, and newlines
      # are part of the comment text. We disable skip patterns so the
      # lexer doesn't silently consume whitespace inside comments.
      "COMMENT_START" -> [{:push_group, "comment"}, {:set_skip_enabled, false}]
      "COMMENT_END" -> [:pop_group, {:set_skip_enabled, true}]

      # --- CDATA boundaries ---
      #
      # CDATA sections contain raw character data -- no entity processing,
      # no tag recognition. Everything is literal text. Like comments,
      # whitespace is significant.
      "CDATA_START" -> [{:push_group, "cdata"}, {:set_skip_enabled, false}]
      "CDATA_END" -> [:pop_group, {:set_skip_enabled, true}]

      # --- Processing instruction boundaries ---
      #
      # PIs like <?xml version="1.0"?> contain a target name and
      # optional content. Whitespace between the target and content
      # is significant.
      "PI_START" -> [{:push_group, "pi"}, {:set_skip_enabled, false}]
      "PI_END" -> [:pop_group, {:set_skip_enabled, true}]

      # --- All other tokens ---
      #
      # Most tokens (TEXT, TAG_NAME, ATTR_VALUE, etc.) don't trigger
      # any group transitions. Return an empty list -- no actions.
      _ -> []
    end
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Tokenize XML source code.

  Returns `{:ok, tokens}` on success, `{:error, message}` on failure.
  Each token is a `%Token{type, value, line, column}` struct.

  The token list always ends with an EOF token.

  ## Token Types

  **Default group** (content between tags):
  - `TEXT` -- text content (e.g., "Hello world")
  - `ENTITY_REF` -- entity reference (e.g., "&amp;")
  - `CHAR_REF` -- character reference (e.g., "&#65;", "&#x41;")
  - `OPEN_TAG_START` -- `<`
  - `CLOSE_TAG_START` -- `</`
  - `COMMENT_START` -- `<!--`
  - `CDATA_START` -- `<![CDATA[`
  - `PI_START` -- `<?`

  **Tag group** (inside tags):
  - `TAG_NAME` -- tag or attribute name (e.g., "div", "class")
  - `ATTR_EQUALS` -- `=`
  - `ATTR_VALUE` -- quoted attribute value (e.g., `"main"`)
  - `TAG_CLOSE` -- `>`
  - `SELF_CLOSE` -- `/>`

  **Comment group**:
  - `COMMENT_TEXT` -- comment content
  - `COMMENT_END` -- `-->`

  **CDATA group**:
  - `CDATA_TEXT` -- raw text content
  - `CDATA_END` -- `]]>`

  **Processing instruction group**:
  - `PI_TARGET` -- PI target name (e.g., "xml")
  - `PI_TEXT` -- PI content
  - `PI_END` -- `?>`

  ## Examples

      {:ok, tokens} = CodingAdventures.XmlLexer.tokenize("<p>Hello</p>")
      types = Enum.map(tokens, & &1.type)
      # => ["OPEN_TAG_START", "TAG_NAME", "TAG_CLOSE", "TEXT",
      #     "CLOSE_TAG_START", "TAG_NAME", "TAG_CLOSE", "EOF"]
  """
  @spec tokenize(String.t()) :: {:ok, [CodingAdventures.Lexer.Token.t()]} | {:error, String.t()}
  def tokenize(source) do
    grammar = get_grammar()
    GrammarLexer.tokenize(source, grammar, on_token: &xml_on_token/2)
  end

  @doc """
  Parse the xml.tokens grammar file and return the TokenGrammar.

  This is useful if you want to inspect the grammar or reuse it directly.
  """
  @spec create_lexer() :: TokenGrammar.t()
  def create_lexer do
    tokens_path = Path.join(@grammars_dir, "xml.tokens")
    {:ok, grammar} = TokenGrammar.parse(File.read!(tokens_path))
    grammar
  end

  # Cache the grammar in a persistent_term for fast repeated access.
  # The first call parses the grammar file; subsequent calls return the
  # cached value instantly. This avoids re-reading and re-parsing the
  # file on every tokenize/1 call.
  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_lexer()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
