defmodule CodingAdventures.Commonmark do
  @moduledoc """
  GFM → HTML pipeline.

  A thin convenience wrapper that chains `CommonmarkParser` and
  `DocumentAstToHtml` into a single `to_html/1` function.

  ## Architecture

  The Document AST is the format-agnostic intermediate representation (IR)
  shared by all parsers and renderers in this project. This package simply
  connects the two existing packages:

  ```
  GFM text
       │
       ▼
  CommonmarkParser.parse/1      (block + inline parsing, ~652 spec examples)
       │
       ▼
  CodingAdventures.DocumentAst  (IR: document node tree)
       │
       ▼
  DocumentAstToHtml.render/1    (HTML back-end)
       │
       ▼
  HTML string
  ```

  ## Usage

      iex> CodingAdventures.Commonmark.to_html("# Hello\\n")
      "<h1>Hello</h1>\\n"

      iex> CodingAdventures.Commonmark.to_html("**bold** and _italic_\\n")
      "<p><strong>bold</strong> and <em>italic</em></p>\\n"
  """

  alias CodingAdventures.CommonmarkParser
  alias CodingAdventures.DocumentAstToHtml

  @doc """
  Convert a GFM string to an HTML string.

  Parses the input with the full GFM 0.31.2 parser and renders it to
  HTML using the standard Document AST → HTML back-end.

      iex> CodingAdventures.Commonmark.to_html("> blockquote\\n")
      "<blockquote>\\n<p>blockquote</p>\\n</blockquote>\\n"
  """
  @spec to_html(String.t()) :: String.t()
  def to_html(markdown) when is_binary(markdown) do
    doc = CommonmarkParser.parse(markdown)
    DocumentAstToHtml.render(doc)
  end
end
