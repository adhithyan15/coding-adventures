defmodule CodingAdventures.AsciidocParser.InlineParser do
  @moduledoc """
  AsciiDoc inline parser — left-to-right binary scanner.

  ## Responsibility

  Converts a raw inline string (the text content of a paragraph, heading, or
  list item) into a list of inline Document AST nodes:
  `[:text, :strong, :emphasis, :code_span, :link, :image, :autolink,
   :hard_break, :soft_break]`.

  ## Algorithm

  The parser uses a recursive `scan/3` function with binary pattern matching.
  Each clause peels off one recognised construct from the front of the binary,
  emits the corresponding node, and recurses on the remainder.

  ```
  scan(remaining, nodes_acc, text_buf)
    → when remaining is empty: finalize(nodes_acc, text_buf)
    → when "  \\n" prefix:     hard_break
    → when "\\\\\\n" prefix:   hard_break
    → when "\\n" prefix:       soft_break
    → when "`" prefix:         code_span (verbatim — no inner parsing)
    → when "**" prefix:        strong unconstrained (find closing "**")
    → when "__" prefix:        emphasis unconstrained (find closing "__")
    → when "*" prefix:         strong constrained (find closing "*")
    → when "_" prefix:         emphasis constrained (find closing "_")
    → when "link:" prefix:     link macro (url[text])
    → when "image:" prefix:    image macro (url[alt])
    → when "<<" prefix:        cross-reference (find ">>")
    → when "https://" prefix:  URL, optionally followed by [text]
    → when "http://" prefix:   URL, optionally followed by [text]
    → otherwise:               append single codepoint to text_buf
  ```

  ## AsciiDoc bold/italic convention

  **Critical difference from Markdown:** In AsciiDoc:
  - `*bold*`    → `:strong`   (maps to `<strong>` / `<b>`)
  - `_italic_`  → `:emphasis` (maps to `<em>` / `<i>`)

  This is the *opposite* of CommonMark's `*text*` = emphasis convention.

  ## Constrained vs unconstrained

  AsciiDoc has two flavours of emphasis/strong:

  - **Constrained** (`*word*`, `_word_`): delimiters must be at a word
    boundary. V1 simplification: we just find the next matching delimiter.
  - **Unconstrained** (`**word**`, `__word__`): can appear anywhere, including
    mid-word. Checked first to avoid misidentifying `**` as `*` + `*`.
  """

  alias CodingAdventures.DocumentAst

  @doc """
  Parse a raw inline string into a list of inline Document AST nodes.

  Processes AsciiDoc inline markup (bold, italic, code spans, links, images,
  xrefs, autolinks, and line breaks) and returns the nodes in document order.

      iex> CodingAdventures.AsciidocParser.InlineParser.parse("Hello *world*")
      [
        %{type: :text, value: "Hello "},
        %{type: :strong, children: [%{type: :text, value: "world"}]}
      ]

      iex> CodingAdventures.AsciidocParser.InlineParser.parse("_italic_")
      [%{type: :emphasis, children: [%{type: :text, value: "italic"}]}]

      iex> CodingAdventures.AsciidocParser.InlineParser.parse("")
      []
  """
  @spec parse(String.t()) :: [DocumentAst.inline_node()]
  def parse(text) when is_binary(text) do
    scan(text, [], "")
  end

  # ── Scanner ──────────────────────────────────────────────────────────────────

  # Base case — nothing left to scan
  defp scan("", acc, buf), do: finalize(acc, buf)

  # Hard break — two trailing spaces before a newline
  defp scan("  \n" <> rest, acc, buf) do
    acc = flush_text(acc, buf)
    scan(rest, [DocumentAst.hard_break() | acc], "")
  end

  # Hard break — backslash before a newline (AsciiDoc line continuation)
  defp scan("\\\n" <> rest, acc, buf) do
    acc = flush_text(acc, buf)
    scan(rest, [DocumentAst.hard_break() | acc], "")
  end

  # Soft break — newline inside a paragraph
  defp scan("\n" <> rest, acc, buf) do
    acc = flush_text(acc, buf)
    scan(rest, [DocumentAst.soft_break() | acc], "")
  end

  # Code span — backtick (verbatim content, no inner parsing)
  defp scan("`" <> rest, acc, buf) do
    case find_closing(rest, "`") do
      {content, tail} ->
        acc = flush_text(acc, buf)
        scan(tail, [DocumentAst.code_span(content) | acc], "")

      nil ->
        # No closing backtick — treat as literal character
        scan(rest, acc, buf <> "`")
    end
  end

  # Strong unconstrained — double asterisk (checked before single asterisk)
  defp scan("**" <> rest, acc, buf) do
    case find_closing(rest, "**") do
      {content, tail} ->
        acc = flush_text(acc, buf)
        inner = parse(content)
        scan(tail, [DocumentAst.strong(inner) | acc], "")

      nil ->
        scan(rest, acc, buf <> "**")
    end
  end

  # Emphasis unconstrained — double underscore
  defp scan("__" <> rest, acc, buf) do
    case find_closing(rest, "__") do
      {content, tail} ->
        acc = flush_text(acc, buf)
        inner = parse(content)
        scan(tail, [DocumentAst.emphasis(inner) | acc], "")

      nil ->
        scan(rest, acc, buf <> "__")
    end
  end

  # Strong constrained — single asterisk (AsciiDoc bold, NOT CommonMark italic)
  defp scan("*" <> rest, acc, buf) do
    case find_closing(rest, "*") do
      {content, tail} ->
        acc = flush_text(acc, buf)
        inner = parse(content)
        scan(tail, [DocumentAst.strong(inner) | acc], "")

      nil ->
        scan(rest, acc, buf <> "*")
    end
  end

  # Emphasis constrained — single underscore (AsciiDoc italic)
  defp scan("_" <> rest, acc, buf) do
    case find_closing(rest, "_") do
      {content, tail} ->
        acc = flush_text(acc, buf)
        inner = parse(content)
        scan(tail, [DocumentAst.emphasis(inner) | acc], "")

      nil ->
        scan(rest, acc, buf <> "_")
    end
  end

  # Image macro — image:url[alt] (checked before link: to avoid prefix clash)
  defp scan("image:" <> rest, acc, buf) do
    case parse_macro(rest) do
      {url, alt, tail} ->
        acc = flush_text(acc, buf)
        scan(tail, [DocumentAst.image(url, nil, alt) | acc], "")

      nil ->
        scan(rest, acc, buf <> "image:")
    end
  end

  # Link macro — link:url[text]
  defp scan("link:" <> rest, acc, buf) do
    case parse_macro(rest) do
      {url, text, tail} ->
        acc = flush_text(acc, buf)
        inner = parse(text)
        scan(tail, [DocumentAst.link(url, nil, inner) | acc], "")

      nil ->
        scan(rest, acc, buf <> "link:")
    end
  end

  # Cross-reference — <<anchor>> or <<anchor,text>>
  defp scan("<<" <> rest, acc, buf) do
    case find_closing(rest, ">>") do
      {content, tail} ->
        acc = flush_text(acc, buf)
        node = parse_xref(content)
        scan(tail, [node | acc], "")

      nil ->
        scan(rest, acc, buf <> "<<")
    end
  end

  # HTTPS URL — may be bare autolink or link macro with [text]
  defp scan("https://" <> rest, acc, buf) do
    {url_rest, remaining} = consume_url_chars(rest)
    full_url = "https://" <> url_rest
    acc = flush_text(acc, buf)

    case remaining do
      "[" <> bracket_rest ->
        case find_closing(bracket_rest, "]") do
          {link_text, tail} ->
            inner = parse(link_text)
            scan(tail, [DocumentAst.link(full_url, nil, inner) | acc], "")

          nil ->
            scan(remaining, [DocumentAst.autolink(full_url, false) | acc], "")
        end

      _ ->
        scan(remaining, [DocumentAst.autolink(full_url, false) | acc], "")
    end
  end

  # HTTP URL — bare autolink or [text] link
  defp scan("http://" <> rest, acc, buf) do
    {url_rest, remaining} = consume_url_chars(rest)
    full_url = "http://" <> url_rest
    acc = flush_text(acc, buf)

    case remaining do
      "[" <> bracket_rest ->
        case find_closing(bracket_rest, "]") do
          {link_text, tail} ->
            inner = parse(link_text)
            scan(tail, [DocumentAst.link(full_url, nil, inner) | acc], "")

          nil ->
            scan(remaining, [DocumentAst.autolink(full_url, false) | acc], "")
        end

      _ ->
        scan(remaining, [DocumentAst.autolink(full_url, false) | acc], "")
    end
  end

  # Default — append the next UTF-8 codepoint to the text buffer
  defp scan(<<char::utf8, rest::binary>>, acc, buf) do
    scan(rest, acc, buf <> <<char::utf8>>)
  end

  # ── Helpers ──────────────────────────────────────────────────────────────────

  # Flush the text buffer as a text node (prepended to the accumulator).
  # Returns the updated accumulator. If the buffer is empty, acc is unchanged.
  defp flush_text(acc, ""), do: acc
  defp flush_text(acc, buf), do: [DocumentAst.text(buf) | acc]

  # Finalise the scan: flush remaining text, reverse accumulator.
  defp finalize(acc, buf) do
    acc |> flush_text(buf) |> Enum.reverse()
  end

  # Find the first occurrence of `delim` in `text` using a simple split.
  # Returns `{content_before, rest_after_delim}` or `nil` if not found.
  #
  # This is intentionally simple (v1): it finds the next occurrence of the
  # delimiter, not the "correct" constrained-markup boundary. That is
  # sufficient for the vast majority of real AsciiDoc documents.
  defp find_closing(text, delim) do
    case :binary.split(text, delim) do
      [before, rest] -> {before, rest}
      _ -> nil
    end
  end

  # Parse a macro of the form `url[content]rest`.
  # Returns `{url, content, rest}` or `nil`.
  defp parse_macro(text) do
    case :binary.split(text, "[") do
      [url, bracket_rest] ->
        case find_closing(bracket_rest, "]") do
          {content, tail} -> {url, content, tail}
          nil -> nil
        end

      _ ->
        nil
    end
  end

  # Parse a cross-reference: "anchor" or "anchor,text".
  # Returns a DocumentAst.link node pointing to "#anchor".
  defp parse_xref(content) do
    case String.split(content, ",", parts: 2) do
      [anchor, text] ->
        inner = parse(String.trim(text))
        DocumentAst.link("#" <> String.trim(anchor), nil, inner)

      [anchor] ->
        inner = [DocumentAst.text(anchor)]
        DocumentAst.link("#" <> anchor, nil, inner)
    end
  end

  # Consume URL-valid characters: everything except whitespace, `[`, `]`, `<`, `>`, `"`.
  # Returns `{url_chars, remaining_binary}`.
  defp consume_url_chars(text) do
    do_consume_url(text, "")
  end

  defp do_consume_url("", acc), do: {acc, ""}

  defp do_consume_url(<<char::utf8, rest::binary>>, acc) do
    if url_char?(char) do
      do_consume_url(rest, acc <> <<char::utf8>>)
    else
      {acc, <<char::utf8>> <> rest}
    end
  end

  # Characters not allowed in bare URLs
  defp url_char?(?\s), do: false
  defp url_char?(?[), do: false
  defp url_char?(?]), do: false
  defp url_char?(?<), do: false
  defp url_char?(?>), do: false
  defp url_char?(?"), do: false
  defp url_char?(?'), do: false
  defp url_char?(_), do: true
end
