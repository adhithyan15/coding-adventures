defmodule CodingAdventures.AsciidocParser do
  @moduledoc """
  AsciiDoc parser — converts AsciiDoc text to a Document AST.

  This module is the public entry point. It delegates block-level parsing to
  `BlockParser` and inline parsing (called from within BlockParser) to
  `InlineParser`.

  ## Usage

      iex> CodingAdventures.AsciidocParser.parse("= Hello\n\nWorld")
      %{type: :document, children: [
        %{type: :heading, level: 1, children: [%{type: :text, value: "Hello"}]},
        %{type: :paragraph, children: [%{type: :text, value: "World"}]}
      ]}

  ## Architecture

  ```
  AsciiDoc text
       |
       v
  BlockParser.parse_blocks/1   <- line-by-line state machine
       |  (calls InlineParser for each inline context)
       v
  Document AST
  ```

  The two-module design mirrors `CommonmarkParser` and `GfmParser` in this
  repo. The top-level module is intentionally thin — it just wires `parse/1`
  to `BlockParser.parse_blocks/1` and wraps the result in a document node.

  ## AsciiDoc vs Markdown — key differences

  | Feature | AsciiDoc | CommonMark |
  |---------|----------|------------|
  | Bold    | `*text*` | `**text**` |
  | Italic  | `_text_` | `*text*`   |
  | Code    | backtick code backtick | backtick code backtick |
  | Heading | `= Title` | `# Title`  |
  | Code block | `----` fence | backtick fence |

  The most surprising difference: `*bold*` in AsciiDoc maps to `:strong`, not
  `:emphasis`. This is the opposite of CommonMark's convention.
  """

  alias CodingAdventures.DocumentAst
  alias CodingAdventures.AsciidocParser.BlockParser

  @doc """
  Parse an AsciiDoc string and return a Document AST node.

  The returned value is a `%{type: :document, children: [...]}` map as defined
  by `CodingAdventures.DocumentAst`. All block and inline content is fully
  resolved — no unresolved references remain.

      iex> doc = CodingAdventures.AsciidocParser.parse("Hello *world*")
      iex> doc.type
      :document
      iex> [para] = doc.children
      iex> para.type
      :paragraph

  The `parse/1` function accepts any binary string, including the empty string
  (which yields a document with an empty children list).
  """
  @spec parse(String.t()) :: DocumentAst.document_node()
  def parse(text) when is_binary(text) do
    blocks = BlockParser.parse_blocks(text)
    DocumentAst.document(blocks)
  end
end
