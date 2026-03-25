defmodule CodingAdventures.DocumentAst do
  @moduledoc """
  Document AST — Format-Agnostic Intermediate Representation

  The Document AST is the "LLVM IR of documents". It sits between front-end
  parsers (Markdown, RST, HTML, DOCX) and back-end renderers (HTML, PDF,
  plain text, LaTeX). Every front-end produces this IR; every back-end
  consumes it.

  With a shared IR, N front-ends × M back-ends requires only N + M
  implementations instead of N × M:

      Markdown ────────────────────────────────► HTML
      reStructuredText ────► Document AST ────► PDF
      HTML ────────────────────────────────────► Plain text
      DOCX ────────────────────────────────────► DOCX

  Spec: TE00 — Document AST

  ## Design principles

  1. **Semantic, not notational** — nodes carry meaning, not syntax
  2. **Resolved, not deferred** — all link references resolved before IR
  3. **Format-agnostic** — `raw_block`/`raw_inline` carry a `format` tag
  4. **Minimal and stable** — only universal document concepts

  ## Node representation

  In Elixir, nodes are plain maps with a `:type` key that discriminates
  the node kind. This is the idiomatic Elixir approach — no classes, no
  OOP, just data.

  Block node types: `:document`, `:heading`, `:paragraph`, `:code_block`,
  `:blockquote`, `:list`, `:list_item`, `:thematic_break`, `:raw_block`

  Inline node types: `:text`, `:emphasis`, `:strong`, `:code_span`,
  `:link`, `:image`, `:autolink`, `:raw_inline`, `:hard_break`, `:soft_break`

  ## No `link_definition` in the IR

  Link definitions (`[label]: url "title"`) are a Markdown-specific parse
  artifact. The front-end resolves them into `LinkNode` values before
  emitting the IR. The IR never contains unresolved references.

  ## `raw_block` / `raw_inline` instead of `html_block` / `html_inline`

  The CommonMark AST has `html_block` and `html_inline` nodes. The Document
  AST replaces them with `raw_block` and `raw_inline` nodes that carry a
  `format` field (`"html"`, `"latex"`, etc.). This extends the concept to
  any target format. Back-ends skip nodes with an unknown format.
  """

  # ── Type specs ──────────────────────────────────────────────────────────────

  @type document_node :: %{
          type: :document,
          children: [block_node()]
        }

  @type heading_node :: %{
          type: :heading,
          level: 1..6,
          children: [inline_node()]
        }

  @type paragraph_node :: %{
          type: :paragraph,
          children: [inline_node()]
        }

  @type code_block_node :: %{
          type: :code_block,
          language: String.t() | nil,
          value: String.t()
        }

  @type blockquote_node :: %{
          type: :blockquote,
          children: [block_node()]
        }

  @type list_node :: %{
          type: :list,
          ordered: boolean(),
          start: non_neg_integer() | nil,
          tight: boolean(),
          children: [list_item_node()]
        }

  @type list_item_node :: %{
          type: :list_item,
          children: [block_node()]
        }

  @type thematic_break_node :: %{type: :thematic_break}

  @type raw_block_node :: %{
          type: :raw_block,
          format: String.t(),
          value: String.t()
        }

  @type block_node ::
          document_node()
          | heading_node()
          | paragraph_node()
          | code_block_node()
          | blockquote_node()
          | list_node()
          | list_item_node()
          | thematic_break_node()
          | raw_block_node()

  @type text_node :: %{type: :text, value: String.t()}
  @type emphasis_node :: %{type: :emphasis, children: [inline_node()]}
  @type strong_node :: %{type: :strong, children: [inline_node()]}
  @type code_span_node :: %{type: :code_span, value: String.t()}

  @type link_node :: %{
          type: :link,
          destination: String.t(),
          title: String.t() | nil,
          children: [inline_node()]
        }

  @type image_node :: %{
          type: :image,
          destination: String.t(),
          title: String.t() | nil,
          alt: String.t()
        }

  @type autolink_node :: %{
          type: :autolink,
          destination: String.t(),
          is_email: boolean()
        }

  @type raw_inline_node :: %{
          type: :raw_inline,
          format: String.t(),
          value: String.t()
        }

  @type hard_break_node :: %{type: :hard_break}
  @type soft_break_node :: %{type: :soft_break}

  @type inline_node ::
          text_node()
          | emphasis_node()
          | strong_node()
          | code_span_node()
          | link_node()
          | image_node()
          | autolink_node()
          | raw_inline_node()
          | hard_break_node()
          | soft_break_node()

  @type ast_node :: block_node() | inline_node()

  # ── Block node constructors ──────────────────────────────────────────────────

  @doc """
  Create a document root node.

  The document node is the root of every IR value. An empty document has an
  empty children list. DocumentNode is the only node type that cannot appear
  as a child of another node.

      iex> CodingAdventures.DocumentAst.document([])
      %{type: :document, children: []}
  """
  @spec document([block_node()]) :: document_node()
  def document(children) when is_list(children) do
    %{type: :document, children: children}
  end

  @doc """
  Create a heading node (level 1–6).

  Semantically corresponds to `<h1>`–`<h6>` in HTML.

      iex> CodingAdventures.DocumentAst.heading(1, [CodingAdventures.DocumentAst.text("Hello")])
      %{type: :heading, level: 1, children: [%{type: :text, value: "Hello"}]}
  """
  @spec heading(1..6, [inline_node()]) :: heading_node()
  def heading(level, children) when level in 1..6 and is_list(children) do
    %{type: :heading, level: level, children: children}
  end

  @doc """
  Create a paragraph node.

      iex> CodingAdventures.DocumentAst.paragraph([])
      %{type: :paragraph, children: []}
  """
  @spec paragraph([inline_node()]) :: paragraph_node()
  def paragraph(children) when is_list(children) do
    %{type: :paragraph, children: children}
  end

  @doc """
  Create a fenced or indented code block node.

  The `value` always ends with a newline. The `language` is `nil` when no
  info string was provided.

      iex> CodingAdventures.DocumentAst.code_block("elixir", "IO.puts \\"hello\\"\\n")
      %{type: :code_block, language: "elixir", value: "IO.puts \\"hello\\"\\n"}
  """
  @spec code_block(String.t() | nil, String.t()) :: code_block_node()
  def code_block(language, value) do
    %{type: :code_block, language: language, value: value}
  end

  @doc """
  Create a blockquote node.

      iex> CodingAdventures.DocumentAst.blockquote([])
      %{type: :blockquote, children: []}
  """
  @spec blockquote([block_node()]) :: blockquote_node()
  def blockquote(children) when is_list(children) do
    %{type: :blockquote, children: children}
  end

  @doc """
  Create a list node (ordered or unordered).

  The `tight` flag controls whether `<p>` wrappers are suppressed in HTML.
  `start` is the first item number for ordered lists; `nil` for unordered.

      iex> CodingAdventures.DocumentAst.list(false, nil, true, [])
      %{type: :list, ordered: false, start: nil, tight: true, children: []}
  """
  @spec list(boolean(), non_neg_integer() | nil, boolean(), [list_item_node()]) :: list_node()
  def list(ordered, start, tight, children) when is_list(children) do
    %{type: :list, ordered: ordered, start: start, tight: tight, children: children}
  end

  @doc """
  Create a list item node.

      iex> CodingAdventures.DocumentAst.list_item([])
      %{type: :list_item, children: []}
  """
  @spec list_item([block_node()]) :: list_item_node()
  def list_item(children) when is_list(children) do
    %{type: :list_item, children: children}
  end

  @doc """
  Create a thematic break node (horizontal rule).

      iex> CodingAdventures.DocumentAst.thematic_break()
      %{type: :thematic_break}
  """
  @spec thematic_break() :: thematic_break_node()
  def thematic_break do
    %{type: :thematic_break}
  end

  @doc """
  Create a raw block node for format-specific passthrough content.

  The `format` field identifies the target back-end (e.g. `"html"`, `"latex"`).
  Back-ends that do not recognise `format` must skip this node silently.

      iex> CodingAdventures.DocumentAst.raw_block("html", "<div>raw</div>\\n")
      %{type: :raw_block, format: "html", value: "<div>raw</div>\\n"}
  """
  @spec raw_block(String.t(), String.t()) :: raw_block_node()
  def raw_block(format, value) do
    %{type: :raw_block, format: format, value: value}
  end

  # ── Inline node constructors ─────────────────────────────────────────────────

  @doc """
  Create a plain text node.

  All HTML character references are decoded before being stored. Adjacent
  text nodes are automatically merged during inline parsing.

      iex> CodingAdventures.DocumentAst.text("Hello & world")
      %{type: :text, value: "Hello & world"}
  """
  @spec text(String.t()) :: text_node()
  def text(value) when is_binary(value) do
    %{type: :text, value: value}
  end

  @doc """
  Create an emphasis node (`<em>`).

      iex> CodingAdventures.DocumentAst.emphasis([CodingAdventures.DocumentAst.text("hello")])
      %{type: :emphasis, children: [%{type: :text, value: "hello"}]}
  """
  @spec emphasis([inline_node()]) :: emphasis_node()
  def emphasis(children) when is_list(children) do
    %{type: :emphasis, children: children}
  end

  @doc """
  Create a strong node (`<strong>`).

      iex> CodingAdventures.DocumentAst.strong([CodingAdventures.DocumentAst.text("bold")])
      %{type: :strong, children: [%{type: :text, value: "bold"}]}
  """
  @spec strong([inline_node()]) :: strong_node()
  def strong(children) when is_list(children) do
    %{type: :strong, children: children}
  end

  @doc """
  Create a code span node.

      iex> CodingAdventures.DocumentAst.code_span("const x = 1")
      %{type: :code_span, value: "const x = 1"}
  """
  @spec code_span(String.t()) :: code_span_node()
  def code_span(value) when is_binary(value) do
    %{type: :code_span, value: value}
  end

  @doc """
  Create a link node with a fully resolved destination URL.

  The `destination` is always a fully resolved URL — never a `[label]`
  reference. The `title` is an optional tooltip string (or `nil`).

      iex> CodingAdventures.DocumentAst.link("https://example.com", nil, [])
      %{type: :link, destination: "https://example.com", title: nil, children: []}
  """
  @spec link(String.t(), String.t() | nil, [inline_node()]) :: link_node()
  def link(destination, title, children) when is_list(children) do
    %{type: :link, destination: destination, title: title, children: children}
  end

  @doc """
  Create an image node.

  The `alt` field is a plain-text fallback description (all inline markup
  stripped). The `destination` is always the fully resolved URL.

      iex> CodingAdventures.DocumentAst.image("cat.png", nil, "a cat")
      %{type: :image, destination: "cat.png", title: nil, alt: "a cat"}
  """
  @spec image(String.t(), String.t() | nil, String.t()) :: image_node()
  def image(destination, title, alt) do
    %{type: :image, destination: destination, title: title, alt: alt}
  end

  @doc """
  Create an autolink node (URL or email address).

  The `is_email` flag distinguishes email autolinks (which need `mailto:`)
  from URL autolinks in HTML rendering.

      iex> CodingAdventures.DocumentAst.autolink("user@example.com", true)
      %{type: :autolink, destination: "user@example.com", is_email: true}
  """
  @spec autolink(String.t(), boolean()) :: autolink_node()
  def autolink(destination, is_email) do
    %{type: :autolink, destination: destination, is_email: is_email}
  end

  @doc """
  Create a raw inline node for format-specific passthrough content.

  Same contract as `raw_block/2` but for inline contexts.

      iex> CodingAdventures.DocumentAst.raw_inline("html", "<em>raw</em>")
      %{type: :raw_inline, format: "html", value: "<em>raw</em>"}
  """
  @spec raw_inline(String.t(), String.t()) :: raw_inline_node()
  def raw_inline(format, value) do
    %{type: :raw_inline, format: format, value: value}
  end

  @doc """
  Create a hard break node (forced line break).

  Forces `<br />` in HTML. In Markdown, produced by two or more trailing
  spaces before a newline, or a backslash immediately before a newline.

      iex> CodingAdventures.DocumentAst.hard_break()
      %{type: :hard_break}
  """
  @spec hard_break() :: hard_break_node()
  def hard_break do
    %{type: :hard_break}
  end

  @doc """
  Create a soft break node (soft line break).

  In HTML, soft breaks render as newlines (browsers collapse to spaces).
  In plain text, they render as literal newlines.

      iex> CodingAdventures.DocumentAst.soft_break()
      %{type: :soft_break}
  """
  @spec soft_break() :: soft_break_node()
  def soft_break do
    %{type: :soft_break}
  end
end
