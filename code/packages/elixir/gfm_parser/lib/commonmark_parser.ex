defmodule CodingAdventures.CommonmarkParser do
  @moduledoc """
  GFM 0.31.2 parser — converts Markdown text to a Document AST.

  ## Usage

      iex> CodingAdventures.CommonmarkParser.parse("# Hello\\n\\nWorld")
      %{type: :document, children: [
        %{type: :heading, level: 1, children: [%{type: :text, value: "Hello"}]},
        %{type: :paragraph, children: [%{type: :text, value: "World"}]}
      ]}

  ## Two-Phase Architecture

  Phase 1 — `BlockParser.parse_blocks/1`:
    Splits input into block-level structure (headings, paragraphs, code blocks,
    blockquotes, lists, etc.). Raw inline content is stored as strings.

  Phase 2 — `InlineParser.parse/2`:
    Transforms each raw inline string into inline nodes (emphasis, links,
    code spans, etc.) using the delimiter stack algorithm.

  ## Spec Conformance

  All 652 GFM 0.31.2 specification examples are supported.
  See https://spec.commonmark.org/0.31.2/ for the full specification.
  """

  alias CodingAdventures.CommonmarkParser.BlockParser
  alias CodingAdventures.CommonmarkParser.InlineParser
  alias CodingAdventures.DocumentAst

  @doc """
  Parse a GitHub Flavored Markdown string and return a Document AST node.

  The returned value is a `%{type: :document, children: [...]}` map as defined
  by `CodingAdventures.DocumentAst`.

      iex> doc = CodingAdventures.CommonmarkParser.parse("Hello *world*")
      iex> doc.type
      :document
      iex> [para] = doc.children
      iex> para.type
      :paragraph
  """
  @spec parse(String.t()) :: DocumentAst.document_node()
  def parse(input) when is_binary(input) do
    # Phase 1: block structure
    {block_doc, refs} = BlockParser.parse_blocks(input)

    # Phase 2: convert block tree to final AST, filling inline content
    # convert_to_ast returns {ast_node, link_refs, raw_inline_map}
    {ast_doc, _refs2, raw_map} = BlockParser.convert_to_ast(block_doc, refs)
    {ast_doc, raw_map} = BlockParser.apply_gfm_block_extensions(ast_doc, raw_map)

    # Phase 3: resolve inline content in each block
    resolve_inline_content(ast_doc, raw_map, refs)
  end

  # Walk the document tree and replace `_raw_id` placeholders with parsed inline nodes
  defp resolve_inline_content(node, raw_map, refs) do
    case node do
      %{type: :document, children: children} ->
        %{node | children: Enum.map(children, &resolve_inline_content(&1, raw_map, refs))}

      %{type: type, _raw_id: raw_id} when type in [:heading, :paragraph, :table_cell] ->
        raw = Map.get(raw_map, raw_id, "")
        inline_nodes = InlineParser.parse(raw, refs)
        node |> Map.delete(:_raw_id) |> Map.put(:children, inline_nodes)

      %{type: :blockquote, children: children} ->
        %{node | children: Enum.map(children, &resolve_inline_content(&1, raw_map, refs))}

      %{type: :list, children: items} ->
        %{node | children: Enum.map(items, &resolve_inline_content(&1, raw_map, refs))}

      %{type: type, children: children} when type in [:list_item, :task_item, :table, :table_row] ->
        %{node | children: Enum.map(children, &resolve_inline_content(&1, raw_map, refs))}

      # code_block, thematic_break, raw_block — no inline content
      _ -> node
    end
  end
end
