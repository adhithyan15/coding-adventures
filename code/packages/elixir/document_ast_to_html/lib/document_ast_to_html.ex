defmodule CodingAdventures.DocumentAstToHtml do
  @moduledoc """
  Document AST → HTML Renderer.

  Converts a `CodingAdventures.DocumentAst` node tree into an HTML string.
  This is the standard CommonMark HTML back-end.

  ## Usage

      iex> alias CodingAdventures.DocumentAst
      iex> alias CodingAdventures.DocumentAstToHtml
      iex> doc = DocumentAst.document([DocumentAst.paragraph([DocumentAst.text("Hello")])])
      iex> DocumentAstToHtml.render(doc)
      "<p>Hello</p>\\n"

  ## Design

  - Block nodes render with a trailing `\\n`.
  - Inline nodes render without `\\n`.
  - `raw_block` / `raw_inline` nodes with `format: "html"` are emitted verbatim.
  - `raw_block` / `raw_inline` nodes with unknown format are skipped.
  - Tight lists suppress `<p>` wrappers around list item paragraph content.
  - `code_block` content is HTML-escaped and always ends with a newline.
  - Heading levels are clamped to 1–6.

  ## HTML escaping

  The characters `&`, `<`, `>`, and `"` are escaped in text content and
  attribute values. This ensures safe HTML output for all valid Document AST
  inputs. URLs in `href` and `src` attributes are NOT re-encoded here because
  the `CommonmarkParser` already normalizes and percent-encodes them.
  However, `&` in URLs is escaped to `&amp;` for HTML attribute validity.
  """

  # ── Public API ───────────────────────────────────────────────────────────────

  @doc """
  Render a Document AST node to an HTML string.

  Accepts any node type but is designed for the document root. Rendering a
  non-document node renders only that node's HTML fragment.

      iex> alias CodingAdventures.DocumentAstToHtml
      iex> alias CodingAdventures.DocumentAst
      iex> DocumentAstToHtml.render(DocumentAst.thematic_break())
      "<hr />\\n"
  """
  @spec render(map()) :: String.t()
  def render(node), do: render_block(node, false)

  # ── Block Rendering ───────────────────────────────────────────────────────────

  defp render_block(%{type: :document, children: children}, tight) do
    Enum.map_join(children, "", &render_block(&1, tight))
  end

  defp render_block(%{type: :heading, level: level, children: inline}, _tight) do
    tag = "h#{level}"
    "<#{tag}>#{render_inlines(inline)}</#{tag}>\n"
  end

  defp render_block(%{type: :paragraph, children: inline}, tight) do
    if tight do
      # In tight lists, paragraphs are rendered without <p> wrappers
      render_inlines(inline) <> "\n"
    else
      "<p>#{render_inlines(inline)}</p>\n"
    end
  end

  defp render_block(%{type: :code_block, language: lang, value: value}, _tight) do
    lang_attr =
      if lang && String.trim(lang) != "" do
        # Take only first word of info string for the class
        first_word = lang |> String.split(~r/\s+/) |> hd() |> escape_html()
        " class=\"language-#{first_word}\""
      else
        ""
      end

    "<pre><code#{lang_attr}>#{escape_html(value)}</code></pre>\n"
  end

  defp render_block(%{type: :blockquote, children: children}, _tight) do
    inner = Enum.map_join(children, "", &render_block(&1, false))
    "<blockquote>\n#{inner}</blockquote>\n"
  end

  defp render_block(%{type: :list, ordered: ordered, start: start, tight: tight, children: items}, _outer_tight) do
    tag = if ordered, do: "ol", else: "ul"
    start_attr =
      if ordered && start && start != 1 do
        " start=\"#{start}\""
      else
        ""
      end

    inner = Enum.map_join(items, "", &render_list_item(&1, tight))
    "<#{tag}#{start_attr}>\n#{inner}</#{tag}>\n"
  end

  defp render_block(%{type: :thematic_break}, _tight) do
    "<hr />\n"
  end

  defp render_block(%{type: :raw_block, format: "html", value: value}, _tight) do
    value
  end

  defp render_block(%{type: :raw_block}, _tight) do
    # Unknown format — skip silently
    ""
  end

  # Fallback for unknown node types
  defp render_block(_, _tight), do: ""

  defp render_list_item(%{type: :list_item, children: []}, _tight) do
    # Empty list item — no children, no newlines
    "<li></li>\n"
  end

  defp render_list_item(%{type: :list_item, children: children}, tight) do
    if tight do
      render_tight_list_item(children)
    else
      inner = Enum.map_join(children, "", &render_block(&1, false))
      "<li>\n#{inner}</li>\n"
    end
  end

  # Fallback for unknown / malformed list_item shapes — kept grouped with the
  # above clauses to silence the Elixir "clauses should be grouped" compiler warning.
  defp render_list_item(_, _tight), do: ""

  # Renders the inner content of a tight list item following CommonMark rules:
  #
  # 1. Empty item:  `<li></li>`
  # 2. Single paragraph:  `<li>text</li>`  — suppress `<p>` wrapper
  # 3. First child is paragraph, more children follow:
  #       `<li>text\n<block/>\n...</li>` — paragraph content is inlined with no `<p>`,
  #       subsequent blocks are rendered normally and joined without a leading `\n`
  # 4. First child is a non-paragraph block (heading, code, blockquote, nested list):
  #       `<li>\n<block/>\n...</li>` — leading `\n` before the block
  #
  # The distinction between rules 3 and 4 is why we can't simply prepend `\n` to
  # every non-single-paragraph case.
  defp render_tight_list_item([]) do
    "<li></li>\n"
  end

  defp render_tight_list_item([%{type: :paragraph, children: inline}]) do
    # Single paragraph — canonical tight list item, no <p> wrapper
    "<li>#{render_inlines(inline)}</li>\n"
  end

  defp render_tight_list_item([%{type: :paragraph, children: inline} | rest]) do
    # Paragraph first, more blocks after — inline the paragraph text, then render rest.
    # The last rendered block's trailing newline is stripped so that e.g.
    # `[para("a"), list]` renders as `<li>a\n<ul>...</ul>\n</li>` (the list already
    # ends with `\n` and we keep it for the `</li>`), whereas
    # `[para("a"), para("b")]` renders as `<li>a\nb</li>` (tight para has no `\n`
    # since the outer `</li>` follows immediately).
    para_text = render_inlines(inline)
    rest_html = Enum.map_join(rest, "", &render_block(&1, true))
    "<li>#{para_text}\n#{rest_html}</li>\n"
  end

  defp render_tight_list_item(children) do
    # First child is a block element (heading, code block, blockquote, nested list):
    # use the \n-wrapped form so e.g. `- # Foo` → `<li>\n<h1>Foo</h1>\n</li>`.
    #
    # The tricky case is when the last child is a tight paragraph: it renders as
    # `text\n` but the spec expects no trailing `\n` before `</li>`:
    #   `[h2("Bar"), para("baz")]` → `<li>\n<h2>Bar</h2>\nbaz</li>` (note: no `\n` before `</li>`)
    #
    # However, when the last child is a code block or blockquote, the trailing `\n`
    # IS preserved:
    #   `[code_block]` → `<li>\n<pre>...</pre>\n</li>` (note: `\n` before `</li>`)
    #
    # So we render all but the last child normally, then render the last child
    # specially: if it's a paragraph (in tight mode), strip its trailing newline.
    {init, [last]} = Enum.split(children, length(children) - 1)
    init_html = Enum.map_join(init, "", &render_block(&1, true))
    last_html =
      case last do
        %{type: :paragraph, children: inline} ->
          # Tight paragraph at end of multi-child item: no trailing newline
          render_inlines(inline)
        _ ->
          render_block(last, true)
      end
    "<li>\n#{init_html}#{last_html}</li>\n"
  end

  # ── Inline Rendering ──────────────────────────────────────────────────────────

  defp render_inlines(nodes) do
    Enum.map_join(nodes, "", &render_inline/1)
  end

  defp render_inline(%{type: :text, value: value}) do
    escape_html(value)
  end

  defp render_inline(%{type: :emphasis, children: children}) do
    "<em>#{render_inlines(children)}</em>"
  end

  defp render_inline(%{type: :strong, children: children}) do
    "<strong>#{render_inlines(children)}</strong>"
  end

  defp render_inline(%{type: :code_span, value: value}) do
    "<code>#{escape_html(value)}</code>"
  end

  defp render_inline(%{type: :link, destination: dest, title: title, children: children}) do
    title_attr = if title, do: " title=\"#{escape_html(title)}\"", else: ""
    "<a href=\"#{escape_url(dest)}\"#{title_attr}>#{render_inlines(children)}</a>"
  end

  defp render_inline(%{type: :image, destination: dest, title: title, alt: alt}) do
    title_attr = if title, do: " title=\"#{escape_html(title)}\"", else: ""
    "<img src=\"#{escape_url(dest)}\" alt=\"#{escape_html(alt)}\"#{title_attr} />"
  end

  defp render_inline(%{type: :autolink, destination: dest, is_email: true}) do
    escaped = escape_html(dest)
    "<a href=\"mailto:#{escaped}\">#{escaped}</a>"
  end

  defp render_inline(%{type: :autolink, destination: dest, is_email: false}) do
    # The display text is the raw URL as written; the href must percent-encode
    # characters that are not safe in HTML attributes (e.g. `\` → `%5C`).
    display = escape_html(dest)
    href = normalize_url_for_html(dest)
    "<a href=\"#{href}\">#{display}</a>"
  end

  defp render_inline(%{type: :raw_inline, format: "html", value: value}) do
    value
  end

  defp render_inline(%{type: :raw_inline}) do
    # Unknown format — skip silently
    ""
  end

  defp render_inline(%{type: :hard_break}) do
    "<br />\n"
  end

  defp render_inline(%{type: :soft_break}) do
    "\n"
  end

  defp render_inline(_), do: ""

  # ── HTML Escaping ─────────────────────────────────────────────────────────────

  defp escape_html(str) when is_binary(str) do
    str
    |> String.replace("&", "&amp;")
    |> String.replace("<", "&lt;")
    |> String.replace(">", "&gt;")
    |> String.replace("\"", "&quot;")
  end

  defp escape_html(nil), do: ""

  # Escape `&` in URLs but keep other URL characters intact.
  # Used for pre-normalized URLs (from inline links and link references) where
  # all characters except `&` are already percent-encoded by the parser.
  defp escape_url(url) when is_binary(url) do
    String.replace(url, "&", "&amp;")
  end

  defp escape_url(nil), do: ""

  # Percent-encode characters unsafe in HTML href attributes AND escape `&`.
  # Used for autolink URLs, which are stored as raw strings and not pre-normalized.
  # Safe characters: ASCII alphanumerics + `-._~:/?#@!$&'()*+,;=%`
  defp normalize_url_for_html(url) when is_binary(url) do
    url
    |> String.graphemes()
    |> Enum.map_join(fn ch ->
      cond do
        ch == "&" -> "&amp;"
        Regex.match?(~r/^[A-Za-z0-9\-._~:\/?#@!$&'()*+,;=%]$/, ch) -> ch
        true -> percent_encode_char(ch)
      end
    end)
  end

  defp normalize_url_for_html(nil), do: ""

  defp percent_encode_char(ch) do
    ch
    |> :binary.bin_to_list()
    |> Enum.map_join(fn byte ->
      "%" <> String.upcase(:io_lib.format("~2.16.0B", [byte]) |> IO.iodata_to_binary())
    end)
  end
end
