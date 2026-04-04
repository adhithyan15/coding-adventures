defmodule CodingAdventures.AsciidocParser.BlockParser do
  @moduledoc """
  AsciiDoc block-level parser — state machine over lines.

  ## State machine overview

  The parser processes the document one line at a time, maintaining a state
  struct that tracks the current parsing mode, any pending attributes (e.g., a
  `[source,lang]` attribute list), accumulated lines for the current block, and
  the list of completed blocks.

  ```
  State struct:
    %{
      mode:          :normal | :paragraph | :code_block | :literal_block
                   | :passthrough_block | :quote_block
                   | :unordered_list | :ordered_list,
      pending_attrs: nil | %{language: "elixir"},
      current_lines: [String.t()],   # reversed — prepend, reverse on flush
      blocks:        [block_node()]  # reversed — prepend, reverse at end
    }
  ```

  ### Transitions from `:normal`

  ```
  blank              → stay :normal (no-op)
  // comment         → stay :normal (skip line)
  [source,lang]      → stay :normal, set pending_attrs
  = Heading …        → emit heading(1..6), clear pending_attrs
  ''' (≥3 quotes)    → emit thematic_break
  ---- (≥4 dashes)   → switch to :code_block
  .... (≥4 dots)     → switch to :literal_block
  ++++ (4 plusses)   → switch to :passthrough_block
  ____ (≥4 _)        → switch to :quote_block
  * text / ** text   → switch to :unordered_list, push {level, text}
  . text / .. text   → switch to :ordered_list,   push {level, text}
  other text         → switch to :paragraph, push line
  ```

  ### `:paragraph` mode

  Accumulates lines until a blank line or a block-opening token, then flushes
  the accumulated text to a paragraph node (with inline parsing applied to each
  line). If a block-opening token is encountered mid-paragraph, the paragraph is
  flushed first and the line is re-dispatched in `:normal` mode.

  ### Delimited block modes

  Each delimited block (`:code_block`, `:literal_block`, `:passthrough_block`,
  `:quote_block`) accumulates raw lines until its closing delimiter appears.
  Raw content is joined with newlines and a trailing newline is appended.

  Quote block content is re-parsed recursively via `parse_blocks/1`, enabling
  nested block structure inside a quotation.

  ### List modes

  `:unordered_list` and `:ordered_list` modes accumulate `{level, text}` tuples
  until a blank line or a non-list line. `build_nested_list/2` converts the flat
  tuple list into a nested `DocumentAst.list` structure.

  ## Inline parsing

  Inline content (paragraph text, heading text, list item text) is passed to
  `InlineParser.parse/1`, which handles bold, italic, code spans, links, images,
  xrefs, and autolinks.
  """

  alias CodingAdventures.DocumentAst
  alias CodingAdventures.AsciidocParser.InlineParser

  @doc """
  Parse AsciiDoc text and return a list of block-level Document AST nodes.

  This is the top-level entry point called by `CodingAdventures.AsciidocParser.parse/1`.
  It splits the input on line boundaries, runs the state machine, flushes any
  remaining state, and returns the block list in document order.

      iex> alias CodingAdventures.AsciidocParser.BlockParser
      iex> BlockParser.parse_blocks("= Hello\\n\\nParagraph")
      [
        %{type: :heading, level: 1, children: [%{type: :text, value: "Hello"}]},
        %{type: :paragraph, children: [%{type: :text, value: "Paragraph"}]}
      ]
  """
  @spec parse_blocks(String.t()) :: [DocumentAst.block_node()]
  def parse_blocks(text) when is_binary(text) do
    lines = String.split(text, ~r/\r?\n/)
    initial = %{mode: :normal, pending_attrs: nil, current_lines: [], blocks: []}
    state = Enum.reduce(lines, initial, &process_line/2)
    state = flush(state)
    Enum.reverse(state.blocks)
  end

  # ── Line dispatcher ──────────────────────────────────────────────────────────

  # Dispatch a single line based on the current mode.
  # The reducer signature is `process_line(line, state) -> state`.

  defp process_line(line, %{mode: :normal} = state) do
    cond do
      blank?(line) ->
        state

      comment_line?(line) ->
        state

      attr_list_line?(line) ->
        %{state | pending_attrs: parse_attr_list(line)}

      heading_line?(line) ->
        {level, text} = parse_heading_line(line)
        inline = InlineParser.parse(text)
        node = DocumentAst.heading(level, inline)
        %{state | blocks: [node | state.blocks], pending_attrs: nil}

      thematic_break_line?(line) ->
        node = DocumentAst.thematic_break()
        %{state | blocks: [node | state.blocks], pending_attrs: nil}

      listing_delimiter?(line) ->
        %{state | mode: :code_block, current_lines: []}

      literal_delimiter?(line) ->
        %{state | mode: :literal_block, current_lines: []}

      passthrough_delimiter?(line) ->
        %{state | mode: :passthrough_block, current_lines: []}

      quote_delimiter?(line) ->
        %{state | mode: :quote_block, current_lines: []}

      unordered_list_item?(line) ->
        {level, text} = parse_list_item(line, "*")
        %{state | mode: :unordered_list, current_lines: [{level, text}]}

      ordered_list_item?(line) ->
        {level, text} = parse_list_item(line, ".")
        %{state | mode: :ordered_list, current_lines: [{level, text}]}

      true ->
        %{state | mode: :paragraph, current_lines: [line]}
    end
  end

  defp process_line(line, %{mode: :paragraph} = state) do
    cond do
      blank?(line) ->
        state |> flush_paragraph() |> Map.put(:mode, :normal)

      block_opener?(line) ->
        state
        |> flush_paragraph()
        |> Map.put(:mode, :normal)
        |> process_line(line)

      true ->
        %{state | current_lines: state.current_lines ++ [line]}
    end
  end

  defp process_line(line, %{mode: :code_block} = state) do
    if listing_delimiter?(line) do
      flush_code_block(state)
    else
      %{state | current_lines: state.current_lines ++ [line]}
    end
  end

  defp process_line(line, %{mode: :literal_block} = state) do
    if literal_delimiter?(line) do
      flush_literal_block(state)
    else
      %{state | current_lines: state.current_lines ++ [line]}
    end
  end

  defp process_line(line, %{mode: :passthrough_block} = state) do
    if passthrough_delimiter?(line) do
      flush_passthrough_block(state)
    else
      %{state | current_lines: state.current_lines ++ [line]}
    end
  end

  defp process_line(line, %{mode: :quote_block} = state) do
    if quote_delimiter?(line) do
      flush_quote_block(state)
    else
      %{state | current_lines: state.current_lines ++ [line]}
    end
  end

  defp process_line(line, %{mode: :unordered_list} = state) do
    cond do
      blank?(line) ->
        state |> flush_list(false) |> Map.put(:mode, :normal)

      unordered_list_item?(line) ->
        {level, text} = parse_list_item(line, "*")
        %{state | current_lines: state.current_lines ++ [{level, text}]}

      true ->
        state
        |> flush_list(false)
        |> Map.put(:mode, :normal)
        |> process_line(line)
    end
  end

  defp process_line(line, %{mode: :ordered_list} = state) do
    cond do
      blank?(line) ->
        state |> flush_list(true) |> Map.put(:mode, :normal)

      ordered_list_item?(line) ->
        {level, text} = parse_list_item(line, ".")
        %{state | current_lines: state.current_lines ++ [{level, text}]}

      true ->
        state
        |> flush_list(true)
        |> Map.put(:mode, :normal)
        |> process_line(line)
    end
  end

  # ── Flush helpers ────────────────────────────────────────────────────────────

  # Flush whatever is pending based on the current mode, return state in :normal
  defp flush(%{mode: :normal} = state), do: state

  defp flush(%{mode: :paragraph} = state) do
    state |> flush_paragraph() |> Map.put(:mode, :normal)
  end

  defp flush(%{mode: :code_block} = state) do
    # Unclosed code block — emit what we have
    flush_code_block(state)
  end

  defp flush(%{mode: :literal_block} = state) do
    flush_literal_block(state)
  end

  defp flush(%{mode: :passthrough_block} = state) do
    flush_passthrough_block(state)
  end

  defp flush(%{mode: :quote_block} = state) do
    flush_quote_block(state)
  end

  defp flush(%{mode: :unordered_list} = state) do
    state |> flush_list(false) |> Map.put(:mode, :normal)
  end

  defp flush(%{mode: :ordered_list} = state) do
    state |> flush_list(true) |> Map.put(:mode, :normal)
  end

  # Flush accumulated paragraph lines → paragraph node with inline parsing
  defp flush_paragraph(%{current_lines: []} = state) do
    %{state | mode: :normal, current_lines: []}
  end

  defp flush_paragraph(state) do
    # Join lines with newline to allow soft-break detection in inline parser
    text = Enum.join(state.current_lines, "\n")
    inline = InlineParser.parse(text)
    node = DocumentAst.paragraph(inline)
    %{state | blocks: [node | state.blocks], current_lines: [], pending_attrs: nil}
  end

  # Flush code block — respects pending_attrs for language
  defp flush_code_block(%{current_lines: lines, pending_attrs: attrs} = state) do
    language = get_in(attrs || %{}, [:language])
    content = ensure_trailing_newline(Enum.join(lines, "\n"))
    node = DocumentAst.code_block(language, content)
    %{state | mode: :normal, blocks: [node | state.blocks], current_lines: [], pending_attrs: nil}
  end

  # Flush literal block (no language, no pending_attrs)
  defp flush_literal_block(%{current_lines: lines} = state) do
    content = ensure_trailing_newline(Enum.join(lines, "\n"))
    node = DocumentAst.code_block(nil, content)
    %{state | mode: :normal, blocks: [node | state.blocks], current_lines: [], pending_attrs: nil}
  end

  # Flush passthrough block → raw_block("html", ...)
  defp flush_passthrough_block(%{current_lines: lines} = state) do
    content = ensure_trailing_newline(Enum.join(lines, "\n"))
    node = DocumentAst.raw_block("html", content)
    %{state | mode: :normal, blocks: [node | state.blocks], current_lines: [], pending_attrs: nil}
  end

  # Flush quote block → recursively parse content → blockquote(children)
  defp flush_quote_block(%{current_lines: lines} = state) do
    inner_text = Enum.join(lines, "\n")
    children = parse_blocks(inner_text)
    node = DocumentAst.blockquote(children)
    %{state | mode: :normal, blocks: [node | state.blocks], current_lines: [], pending_attrs: nil}
  end

  # Flush list → build nested list structure → list node
  defp flush_list(%{current_lines: items} = state, _ordered) when items == [] do
    %{state | mode: :normal, current_lines: []}
  end

  defp flush_list(%{current_lines: items} = state, ordered) do
    node = build_nested_list(items, ordered)
    %{state | blocks: [node | state.blocks], current_lines: [], pending_attrs: nil}
  end

  # ── Line predicates ──────────────────────────────────────────────────────────

  @doc false
  def blank?(line), do: String.trim(line) == ""

  @doc false
  def comment_line?(line), do: String.starts_with?(line, "//")

  @doc false
  def heading_line?(line) do
    # = Title, == Section, ... ====== Level 6
    # Must match 1–6 equals signs followed by a space
    Regex.match?(~r/^={1,6} /, line)
  end

  @doc false
  def listing_delimiter?(line) do
    # Four or more dashes on a line by themselves
    Regex.match?(~r/^-{4,}\s*$/, line)
  end

  @doc false
  def literal_delimiter?(line) do
    # Four or more dots on a line by themselves
    Regex.match?(~r/^\.{4,}\s*$/, line)
  end

  @doc false
  def passthrough_delimiter?(line) do
    # Exactly four plus signs (or more)
    Regex.match?(~r/^\+{4,}\s*$/, line)
  end

  @doc false
  def quote_delimiter?(line) do
    # Four or more underscores on a line by themselves
    Regex.match?(~r/^_{4,}\s*$/, line)
  end

  @doc false
  def thematic_break_line?(line) do
    # Three or more single-quote characters on a line by themselves
    Regex.match?(~r/^'{3,}\s*$/, line)
  end

  @doc false
  def attr_list_line?(line) do
    # [source,lang], [source, lang], [source], etc.
    String.starts_with?(line, "[") and String.ends_with?(String.trim(line), "]")
  end

  @doc false
  def unordered_list_item?(line) do
    # * text, ** text, *** text — one or more asterisks then a space
    Regex.match?(~r/^\*+ /, line)
  end

  @doc false
  def ordered_list_item?(line) do
    # . text, .. text, ... text — one or more dots then a space
    Regex.match?(~r/^\.+ /, line)
  end

  # block_opener? — true for any line that should interrupt a paragraph
  defp block_opener?(line) do
    heading_line?(line) or
      listing_delimiter?(line) or
      literal_delimiter?(line) or
      passthrough_delimiter?(line) or
      quote_delimiter?(line) or
      thematic_break_line?(line) or
      unordered_list_item?(line) or
      ordered_list_item?(line) or
      comment_line?(line)
  end

  # ── Attribute list parsing ───────────────────────────────────────────────────

  @doc """
  Parse an AsciiDoc attribute list line into a map.

  Currently handles `[source,lang]` and `[source, lang]` patterns to extract
  the language for code blocks. Unknown attribute lists are parsed but may
  return an empty map (they still clear pending state correctly).

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_attr_list("[source,elixir]")
      %{language: "elixir"}

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_attr_list("[source, ruby]")
      %{language: "ruby"}

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_attr_list("[NOTE]")
      %{}
  """
  @spec parse_attr_list(String.t()) :: map()
  def parse_attr_list(line) do
    # Strip brackets
    inner = line |> String.trim() |> String.trim_leading("[") |> String.trim_trailing("]")
    parts = String.split(inner, ",", parts: 2)

    case Enum.map(parts, &String.trim/1) do
      ["source", lang] when lang != "" ->
        %{language: lang}

      _ ->
        %{}
    end
  end

  # ── Heading parsing ──────────────────────────────────────────────────────────

  @doc """
  Parse a heading line into `{level, text}`.

  The level is determined by counting leading `=` signs. The text is everything
  after the space that follows the `=` characters.

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_heading_line("= Title")
      {1, "Title"}

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_heading_line("=== Deep")
      {3, "Deep"}
  """
  @spec parse_heading_line(String.t()) :: {1..6, String.t()}
  def parse_heading_line(line) do
    # Count leading '=' characters
    level = line |> String.graphemes() |> Enum.take_while(&(&1 == "=")) |> length()
    level = min(level, 6)
    # Text starts after the '= ' prefix
    text = String.slice(line, level + 1, String.length(line))
    {level, String.trim(text)}
  end

  # ── List item parsing ────────────────────────────────────────────────────────

  @doc """
  Parse a list item line into `{level, text}`.

  For unordered lists, `marker` is `"*"`. For ordered lists, `marker` is `"."`.
  The level equals the count of repeated marker characters.

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_list_item("* item", "*")
      {1, "item"}

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_list_item("** nested", "*")
      {2, "nested"}

      iex> CodingAdventures.AsciidocParser.BlockParser.parse_list_item(". first", ".")
      {1, "first"}
  """
  @spec parse_list_item(String.t(), String.t()) :: {pos_integer(), String.t()}
  def parse_list_item(line, marker) do
    level = line |> String.graphemes() |> Enum.take_while(&(&1 == marker)) |> length()
    text = String.slice(line, level + 1, String.length(line))
    {level, String.trim(text)}
  end

  # ── Nested list builder ──────────────────────────────────────────────────────

  @doc """
  Build a nested `DocumentAst.list` from a flat list of `{level, text}` items.

  Items at the same level become siblings; items at a deeper level become
  children of the previous item. This is the standard "level-based nesting"
  algorithm used by most lightweight markup processors.

  ## Algorithm

  We process the items linearly and maintain a stack of `{level, children_acc}`
  frames. When the current item's level is deeper than the top of stack, we push
  a new frame. When it's the same, we append to the current frame. When it's
  shallower, we pop frames and fold their accumulated children into a sublist
  on the parent item.

      iex> alias CodingAdventures.AsciidocParser.BlockParser
      iex> items = [{1, "A"}, {2, "A1"}, {2, "A2"}, {1, "B"}]
      iex> list = BlockParser.build_nested_list(items, false)
      iex> list.type
      :list
      iex> length(list.children)
      2
  """
  @spec build_nested_list([{pos_integer(), String.t()}], boolean()) ::
          DocumentAst.list_node()
  def build_nested_list(items, ordered) do
    item_nodes = build_items(items, ordered)
    DocumentAst.list(ordered, if(ordered, do: 1, else: nil), true, item_nodes)
  end

  # Build list_item nodes from a flat list of {level, text} tuples.
  # Groups items at level 1 together, and any deeper-level items immediately
  # following a level-1 item become that item's children (as a nested list).
  defp build_items(items, ordered) do
    items
    |> group_by_top_level()
    |> Enum.map(fn {text, children_tuples} ->
      inline = InlineParser.parse(text)
      para = DocumentAst.paragraph(inline)

      child_blocks =
        if children_tuples == [] do
          [para]
        else
          # Strip one level from nested items and recurse
          stripped = Enum.map(children_tuples, fn {lvl, t} -> {lvl - 1, t} end)
          nested_list = build_nested_list(stripped, ordered)
          [para, nested_list]
        end

      DocumentAst.list_item(child_blocks)
    end)
  end

  # Group a flat list of {level, text} tuples into top-level items with
  # their associated deeper-level children.
  #
  # Returns a list of {top_level_text, [deeper_level_tuples]}.
  #
  # Example:
  #   [{1,"A"}, {2,"A1"}, {2,"A2"}, {1,"B"}]
  #   → [{"A", [{2,"A1"},{2,"A2"}]}, {"B", []}]
  defp group_by_top_level(items) do
    Enum.reduce(items, [], fn {level, text}, acc ->
      if level == 1 do
        [{text, []} | acc]
      else
        case acc do
          [{top_text, children} | rest] ->
            [{top_text, children ++ [{level, text}]} | rest]

          [] ->
            # Orphaned deep item — treat it as level 1
            [{text, []} | acc]
        end
      end
    end)
    |> Enum.reverse()
  end

  # ── Utility ──────────────────────────────────────────────────────────────────

  @doc """
  Ensure `str` ends with a newline character.

  Code block content in the Document AST always ends with `\\n`, following the
  convention established by the CommonMark spec (fenced code blocks always have
  a trailing newline on the final line).

      iex> CodingAdventures.AsciidocParser.BlockParser.ensure_trailing_newline("hello")
      "hello\\n"

      iex> CodingAdventures.AsciidocParser.BlockParser.ensure_trailing_newline("hello\\n")
      "hello\\n"
  """
  @spec ensure_trailing_newline(String.t()) :: String.t()
  def ensure_trailing_newline(""), do: "\n"

  def ensure_trailing_newline(str) do
    if String.ends_with?(str, "\n"), do: str, else: str <> "\n"
  end
end
