defmodule CodingAdventures.Asciidoc do
  @moduledoc """
  AsciiDoc → HTML pipeline.

  A thin convenience wrapper that chains `AsciidocParser` and
  `DocumentAstToHtml` into a single `to_html/1` function.

  ## Architecture

  The Document AST is the format-agnostic intermediate representation (IR)
  shared by all parsers and renderers in this project. This package simply
  connects the two existing packages:

  ```
  AsciiDoc text
       │
       ▼
  AsciidocParser.parse/1        (block + inline parsing)
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

  ## AsciiDoc markup reminder

  | AsciiDoc         | HTML output                  |
  |------------------|------------------------------|
  | `= Title`        | `<h1>Title</h1>`             |
  | `== Section`     | `<h2>Section</h2>`           |
  | `*bold*`         | `<strong>bold</strong>`      |
  | `_italic_`       | `<em>italic</em>`            |
  | `` `code` ``     | `<code>code</code>`          |
  | `link:url[text]` | `<a href="url">text</a>`     |
  | `image:url[alt]` | `<img src="url" alt="alt">` |
  | `'''`            | `<hr />`                     |

  ## Usage

      iex> CodingAdventures.Asciidoc.to_html("= Hello\\n\\nWorld\\n")
      "<h1>Hello</h1>\\n<p>World</p>\\n"

      iex> CodingAdventures.Asciidoc.to_html("*bold* and _italic_\\n")
      "<p><strong>bold</strong> and <em>italic</em></p>\\n"
  """

  alias CodingAdventures.AsciidocParser
  alias CodingAdventures.DocumentAstToHtml

  @doc """
  Convert an AsciiDoc string to an HTML string.

  Parses the input with the full AsciiDoc block + inline parser and renders it
  to HTML using the standard Document AST → HTML back-end.

  The returned string will have a trailing newline matching the convention used
  by all renderers in this project.

      iex> CodingAdventures.Asciidoc.to_html("''\\n''\\n'''\\n")
      "<hr />\\n"

      iex> CodingAdventures.Asciidoc.to_html("")
      ""
  """
  @spec to_html(String.t()) :: String.t()
  def to_html(text) when is_binary(text) do
    doc = AsciidocParser.parse(text)
    DocumentAstToHtml.render(doc)
  end
end
