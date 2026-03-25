defmodule CodingAdventures.CommonmarkParser.BlockParser do
  @moduledoc """
  Block-Level Parser — Phase 1 of CommonMark parsing.

  Splits the input into block-level tokens and builds the structural skeleton
  of the document. The output is a mutable intermediate document tree plus a
  link reference map.

  ## Two-Phase Overview

  CommonMark parsing is inherently two-phase:

    Phase 1 (this module): Block structure
      Input text → lines → block tree with raw inline content strings

    Phase 2 (InlineParser): Inline content
      Each block's raw content → inline nodes (emphasis, links, etc.)

  The phases cannot be merged because block structure determines where
  inline content lives. A `*` that starts a list item is structural;
  a `*` inside paragraph text may be emphasis.

  ## Algorithm

  For each line of input:
  1. Walk the container stack (document → blockquote/list/list_item) to see
     which containers continue on this line.
  2. Handle multi-line block modes: fenced code blocks, HTML blocks.
  3. Detect new block types: ATX headings, thematic breaks, fenced code,
     HTML blocks, blockquotes, list items, indented code, paragraphs.
  4. Accumulate raw inline content in paragraph/heading lines for Phase 2.

  ## HTML Block Types (CommonMark §4.6)

    1. `<script/pre/textarea/style>` — ends on `</tag>`
    2. `<!--` — ends on `-->`
    3. `<?` — ends on `?>`
    4. `<!DECLARATION>` — ends on `>`
    5. `<![CDATA[` — ends on `]]>`
    6. Block-level open/close tag — ends on blank line
    7. Complete tag (not type 6) — ends on blank line
  """

  alias CodingAdventures.CommonmarkParser.Scanner
  alias CodingAdventures.CommonmarkParser.Entities

  # HTML block opening patterns (module-level compiled for performance)
  @html1_open ~r/^<(?:script|pre|textarea|style)(?:\s|>|$)/i
  @html1_close ~r/<\/(?:script|pre|textarea|style)>/i
  @html2_open ~r/^<!--/
  @html2_close ~r/--!?>/
  @html3_open ~r/^<\?/
  @html3_close ~r/\?>/
  @html4_open ~r/^<![A-Z]/
  @html4_close ~r/>/
  @html5_open ~r/^<!\[CDATA\[/
  @html5_close ~r/\]\]>/
  @html6_tags ~w(address article aside base basefont blockquote body
    caption center col colgroup dd details dialog dir div dl dt fieldset
    figcaption figure footer form frame frameset h1 h2 h3 h4 h5 h6 head
    header hr html iframe legend li link main menu menuitem meta nav
    noframes ol optgroup option p param search section summary table tbody
    td tfoot th thead title tr track ul)
  @html6_open Regex.compile!(
    "^</?(?:#{Enum.join(@html6_tags, "|")})(?:\\s|>|/>|$)",
    [:caseless]
  )
  @html7_open_tag ~r/^<[A-Za-z][A-Za-z0-9\-]*(?:\s+[a-zA-Z_:][a-zA-Z0-9_:.\-]*(?:\s*=\s*(?:[^\s"'=<>`]+|'[^'\n]*'|"[^"\n]*"))?)* *\/?>$/
  @html7_close_tag ~r/^<\/[A-Za-z][A-Za-z0-9\-]*\s*>$/

  # ── Public API ───────────────────────────────────────────────────────────────

  @doc """
  Parse a CommonMark document into a block-level tree (Phase 1).

  Returns `{document, link_refs}` where `document` is the intermediate block
  tree and `link_refs` is a map of normalized labels to `%{destination, title}`.
  """
  @spec parse_blocks(String.t()) :: {map(), map()}
  def parse_blocks(input) do
    normalized = input |> String.replace("\r\n", "\n") |> String.replace("\r", "\n")
    lines = String.split(normalized, "\n")

    # Remove trailing empty string from trailing newline
    lines =
      case List.last(lines) do
        "" -> Enum.drop(lines, -1)
        _ -> lines
      end

    root = %{kind: :document, children: []}

    state = %{
      # Stack of open container blocks (document, blockquote, list, list_item)
      containers: [root],
      # Currently open leaf block (paragraph, fenced_code, html_block, indented_code), or nil
      leaf: nil,
      # Multi-line block mode
      mode: :normal,
      # Was the last processed line blank?
      last_blank: false,
      # Which container was innermost during the last blank line?
      last_blank_in: root,
      # Accumulated link reference definitions
      refs: %{}
    }

    state = Enum.reduce(lines, state, &process_line/2)

    # Finalize any remaining open leaf
    state = finalize_leaf_if_open(state)

    {hd(state.containers), state.refs}
  end

  # ── AST Conversion ───────────────────────────────────────────────────────────

  @doc """
  Convert the mutable intermediate document tree to the final Document AST.

  Returns `{document_node, link_refs, raw_inline}` where `raw_inline` maps
  integer IDs to raw inline content strings for Phase 2 to process.
  """
  @spec convert_to_ast(map(), map()) :: {map(), map(), map()}
  def convert_to_ast(doc, refs) do
    {node, raw, _} = conv_block(doc, %{}, 0)
    {node, refs, raw}
  end

  # ── Line Processing ──────────────────────────────────────────────────────────

  defp process_line(raw_line, state) do
    orig_blank = blank?(raw_line)

    # Step 1: Container continuation — strip container markers from the line
    {line, base_col, new_conts, lazy_para} =
      cont_containers(state.containers, raw_line, 0, orig_blank, state.leaf)

    prev_inner = List.last(state.containers)
    state = %{state | containers: new_conts}

    # After stripping markers, re-check blank status
    blank = orig_blank || blank?(line)

    # Step 2: Multi-line block continuation (fenced code, HTML blocks)
    case handle_multiline(state, raw_line, line, base_col, prev_inner, blank, orig_blank) do
      {:continue, state} ->
        state

      {:normal, state, line, base_col, blank} ->
        cur_inner = List.last(state.containers)

        # Step 2b: Finalize leaf if we left its container
        state =
          if cur_inner != prev_inner && state.leaf != nil && !lazy_para do
            finalize_leaf_in(state, prev_inner)
          else
            state
          end

        # Step 3: Lazy paragraph continuation
        if lazy_para && state.leaf != nil && state.leaf.kind == :paragraph do
          leaf2 = %{state.leaf | lines: state.leaf.lines ++ [line]}
          state = update_leaf(state, leaf2)
          %{state | last_blank: false}
        else
          # Close lists that won't continue
          state = maybe_close_list(state, line, blank)
          inner = List.last(state.containers)

          if blank do
            handle_blank(state, inner, raw_line)
          else
            detect_block(state, inner, line, base_col, orig_blank)
          end
        end
    end
  end

  # ── Container Continuation ───────────────────────────────────────────────────

  # Walk the container stack, stripping markers, returning:
  # {stripped_line, base_col, new_containers, lazy_continuation?}
  defp cont_containers(containers, line, base_col, orig_blank, leaf) do
    do_cont(containers, 1, line, base_col, orig_blank, leaf, [hd(containers)], false)
  end

  defp do_cont(conts, idx, line, base_col, _orig_blank, _leaf, new_c, lazy)
       when idx >= length(conts),
       do: {line, base_col, new_c, lazy}

  defp do_cont(conts, idx, line, base_col, orig_blank, leaf, new_c, _lazy) do
    cont = Enum.at(conts, idx)

    case cont.kind do
      :blockquote ->
        {bi, bc} = skip_spaces_n(line, base_col, 3)
        ch = safe_at(line, bi)

        if ch == ">" do
          {stripped, new_bc, _} = strip_bq_marker(line, bi, bc)
          do_cont(conts, idx + 1, stripped, new_bc, orig_blank, leaf, new_c ++ [cont], false)
        else
          # Try lazy paragraph continuation.
          # Note: no indentation limit here — a line with 4+ spaces can still be
          # a lazy continuation of a paragraph in a blockquote. The `< 4` guard
          # would incorrectly exclude lines like `    - bar` (4 spaces + list marker
          # that exceeds 3-space indent limit so parse_list_marker returns nil).
          if leaf != nil && leaf.kind == :paragraph && !orig_blank &&
               !thematic_break?(line) && is_nil(parse_atx_heading(line)) &&
               !fenced_opener?(line) do
            lm = parse_list_marker(line)
            blank_start = lm != nil && blank?(String.slice(line, lm.marker_len..-1//1))

            if is_nil(lm) || blank_start do
              {line, base_col, new_c ++ [cont], true}
            else
              {line, base_col, new_c, false}
            end
          else
            {line, base_col, new_c, false}
          end
        end

      :list ->
        # Lists pass through — tightness handled by list items
        do_cont(conts, idx + 1, line, base_col, orig_blank, leaf, new_c ++ [cont], false)

      :list_item ->
        item = cont
        eff_blank = orig_blank || blank?(line)
        ind = indent_of(line, base_col)

        cond do
          !eff_blank && ind >= item.content_indent ->
            {stripped, new_bc} = strip_indent(line, item.content_indent, base_col)
            do_cont(conts, idx + 1, stripped, new_bc, orig_blank, leaf, new_c ++ [cont], false)

          eff_blank ->
            has_content = length(item.children) > 0 ||
              (leaf != nil && Enum.at(conts, idx) == leaf)

            if has_content do
              do_cont(conts, idx + 1, line, base_col, orig_blank, leaf, new_c ++ [cont], false)
            else
              {line, base_col, new_c, false}
            end

          # Lazy paragraph continuation for list items.
          # No indentation limit — see blockquote lazy continuation comment above.
          leaf != nil && leaf.kind == :paragraph && !orig_blank &&
              !thematic_break?(line) && is_nil(parse_list_marker(line)) &&
              !fenced_opener?(line) &&
              is_nil(parse_atx_heading(line)) ->
            {line, base_col, new_c ++ [cont], true}

          true ->
            {line, base_col, new_c, false}
        end

      _ ->
        {line, base_col, new_c, false}
    end
  end

  # ── Multi-line Block Handling ─────────────────────────────────────────────────

  defp handle_multiline(state, _raw_line, line, base_col, prev_inner, blank, orig_blank) do
    cur_inner = List.last(state.containers)

    cond do
      state.mode == :fenced && state.leaf != nil && state.leaf.kind == :fenced_code ->
        fence = state.leaf

        if cur_inner != prev_inner do
          # Container dropped — force close
          state = close_fenced(state, fence)
          {:normal, state, line, base_col, blank}
        else
          stripped = String.trim_leading(line)
          ch = String.first(fence.fence)
          close_re = Regex.compile!("^" <> Regex.escape(ch) <> "{" <> Integer.to_string(fence.fence_len) <> ",}\\s*$")

          is_close = indent_of(line, base_col) < 4 &&
            Regex.match?(close_re, stripped) &&
            !(ch == "`" && String.starts_with?(stripped, "~")) &&
            !(ch == "~" && String.starts_with?(stripped, "`"))

          if is_close do
            state = close_fenced(state, fence)
            {:continue, %{state | last_blank: orig_blank}}
          else
            {fl, _} = strip_indent(line, fence.base_indent, base_col)
            fence2 = %{fence | lines: fence.lines ++ [fl]}
            state = update_leaf(state, fence2)
            {:continue, %{state | last_blank: orig_blank}}
          end
        end

      state.mode == :html_block && state.leaf != nil && state.leaf.kind == :html_block ->
        html = state.leaf

        if cur_inner != prev_inner do
          # Container dropped — force close
          html2 = %{html | closed: true}
          state = update_leaf(state, html2)
          state = %{state | leaf: nil, mode: :normal}
          {:normal, state, line, base_col, blank}
        else
          html2 = %{html | lines: html.lines ++ [line]}
          closed = html_block_ends?(line, html.html_type)
          html2 = %{html2 | closed: closed}
          state = update_leaf(state, html2)

          state =
            if closed do
              %{state | leaf: nil, mode: :normal}
            else
              %{state | leaf: html2}
            end

          {:continue, %{state | last_blank: orig_blank}}
        end

      true ->
        {:normal, state, line, base_col, blank}
    end
  end

  defp close_fenced(state, fence) do
    fence2 = %{fence | closed: true}
    state = update_leaf(state, fence2)
    %{state | leaf: nil, mode: :normal}
  end

  # ── Blank Line Handling ──────────────────────────────────────────────────────

  defp handle_blank(state, inner, raw_line) do
    # Close paragraph
    state =
      if state.leaf != nil && state.leaf.kind == :paragraph do
        finalize_leaf_in(state, inner)
      else
        state
      end

    # Buffer blank lines in indented code (for later trimming)
    state =
      if state.leaf != nil && state.leaf.kind == :indented_code do
        {stripped, _} = strip_indent(raw_line, 4, 0)
        leaf2 = %{state.leaf | lines: state.leaf.lines ++ [stripped]}
        update_leaf(state, leaf2)
      else
        state
      end

    # Mark blank for list tightness
    state =
      if inner.kind == :list_item do
        update_block(state, inner, &%{&1 | had_blank_line: true})
      else
        state
      end

    state =
      if inner.kind == :list do
        update_block(state, inner, &%{&1 | had_blank_line: true})
      else
        state
      end

    %{state | last_blank: true, last_blank_in: inner}
  end

  # ── Block Detection ──────────────────────────────────────────────────────────

  defp detect_block(state, inner, line, base_col, _orig_blank) do
    # Propagate list tightness from blank lines
    state =
      if state.last_blank && inner.kind == :list &&
           inner(state.last_blank_in, [:list, :list_item]) do
        update_block(state, inner, &%{&1 | tight: false})
      else
        state
      end

    state =
      if state.last_blank && inner.kind == :list_item do
        update_block(state, inner, &%{&1 | had_blank_line: true})
      else
        state
      end

    inner = List.last(state.containers)
    stripped = String.trim_leading(line)
    indent = indent_of(line, base_col)

    try_blocks(state, inner, line, base_col, stripped, indent)
  end

  defp inner(block, kinds), do: block.kind in kinds

  defp try_fenced(state, inner, _line, _base_col, stripped, indent, fence_m) do
    if fence_m != nil && indent < 4 do
      fence_str = hd(fence_m)
      fence_char = String.first(fence_str)
      fence_len = String.length(fence_str)
      info_line = String.slice(stripped, fence_len..-1//1)

      # Backtick fences can't have backtick in info string
      if fence_char == "`" && String.contains?(info_line, "`") do
        nil
      else
        state = close_leaf(state, inner)
        inner = List.last(state.containers)
        info = extract_info_string(stripped)
        fb = %{kind: :fenced_code, fence: String.duplicate(fence_char, fence_len),
               fence_len: fence_len, base_indent: indent, info_string: info,
               lines: [], closed: false}
        state = add_child_to(state, inner, fb)
        %{state | leaf: fb, mode: :fenced, last_blank: false}
      end
    end
  end

  defp try_atx(state, inner, line, _base_col, indent) do
    if indent < 4 do
      case parse_atx_heading(line) do
        %{level: level, content: content} ->
          state = close_leaf(state, inner)
          inner = List.last(state.containers)
          hb = %{kind: :heading, level: level, content: content}
          state = add_child_to(state, inner, hb)
          %{state | leaf: nil, last_blank: false}

        nil ->
          nil
      end
    end
  end

  defp try_thematic_or_setext(state, inner, line, _base_col, indent) do
    if indent < 4 do
      is_tb = thematic_break?(line)
      setext = is_setext_underline(line)

      cond do
        # If in paragraph, setext heading underline takes priority
        is_tb && state.leaf != nil && state.leaf.kind == :paragraph ->
          case setext do
            level when level in [1, 2] ->
              para = state.leaf
              {para2, refs2} = finalize_block(para, inner, state.refs)
              state = update_leaf(state, para2)
              state = %{state | refs: refs2}
              inner = List.last(state.containers)

              if length(para2.lines) > 0 do
                hb = %{kind: :heading, level: level,
                       content: Enum.join(para2.lines, "\n") |> String.trim()}
                state = replace_last_child(state, inner, hb)
                %{state | leaf: nil, last_blank: false}
              else
                # All content was link defs — create thematic break
                state = remove_last_child_from(state, inner)
                inner = List.last(state.containers)
                tb = %{kind: :thematic_break}
                state = add_child_to(state, inner, tb)
                %{state | leaf: nil, last_blank: false}
              end

            _ ->
              # Standard thematic break (non-setext underline)
              state = close_leaf(state, inner)
              inner = List.last(state.containers)
              tb = %{kind: :thematic_break}
              state = add_child_to(state, inner, tb)
              %{state | leaf: nil, last_blank: false}
          end

        is_tb ->
          state = close_leaf(state, inner)
          inner = List.last(state.containers)
          tb = %{kind: :thematic_break}
          state = add_child_to(state, inner, tb)
          %{state | leaf: nil, last_blank: false}

        # Setext underline without thematic break
        setext in [1, 2] && state.leaf != nil && state.leaf.kind == :paragraph ->
          level = setext
          para = state.leaf
          {para2, refs2} = finalize_block(para, inner, state.refs)
          state = update_leaf(state, para2)
          state = %{state | refs: refs2}
          inner = List.last(state.containers)

          if length(para2.lines) > 0 do
            hb = %{kind: :heading, level: level,
                   content: Enum.join(para2.lines, "\n") |> String.trim()}
            state = replace_last_child(state, inner, hb)
            %{state | leaf: nil, last_blank: false}
          else
            # All was link defs — fall through to new paragraph
            state = remove_last_child_from(state, inner)
            inner = List.last(state.containers)
            para_new = %{kind: :paragraph, lines: [line]}
            state = add_child_to(state, inner, para_new)
            %{state | leaf: para_new, last_blank: false}
          end

        true ->
          nil
      end
    end
  end

  defp try_html_block(state, inner, line, _base_col, stripped, indent) do
    if indent < 4 do
      case detect_html_type(stripped) do
        nil ->
          nil

        html_type ->
          # Type 7 cannot interrupt a paragraph
          if html_type == 7 && state.leaf != nil && state.leaf.kind == :paragraph do
            nil
          else
            state = close_leaf(state, inner)
            inner = List.last(state.containers)
            closed = html_block_ends?(line, html_type)
            hb = %{kind: :html_block, html_type: html_type, lines: [line], closed: closed}
            state = add_child_to(state, inner, hb)

            if closed do
              %{state | leaf: nil, last_blank: false}
            else
              %{state | leaf: hb, mode: :html_block, last_blank: false}
            end
          end
      end
    end
  end

  defp try_blockquote(state, inner, line, base_col, stripped, indent) do
    if indent < 4 && String.starts_with?(stripped, ">") do
      state = close_leaf(state, inner)
      inner = List.last(state.containers)

      # Continue or create blockquote
      last = last_child_of(inner)
      {bq, state} =
        if last != nil && last.kind == :blockquote && !state.last_blank do
          {last, state}
        else
          new_bq = %{kind: :blockquote, children: []}
          state = add_child_to(state, inner, new_bq)
          {new_bq, state}
        end

      # Push blockquote onto container stack if not already there
      state =
        if List.last(state.containers) == bq do
          state
        else
          %{state | containers: state.containers ++ [bq]}
        end

      {stripped_content, new_base_col} = strip_bq_line(line, base_col)
      inner = bq

      if blank?(stripped_content) do
        %{state | last_blank: false}
      else
        detect_block(
          %{state | last_blank: false},
          inner, stripped_content, new_base_col, false
        )
      end
    end
  end

  defp try_list_item(state, inner, line, base_col, indent) do
    if indent < 4 do
      case parse_list_marker(line) do
        nil ->
          nil

        marker ->
          blank_start = blank?(String.slice(line, marker.marker_len..-1//1))

          # Can this marker interrupt a paragraph?
          para_in_inner = state.leaf != nil && state.leaf.kind == :paragraph &&
            last_child_of(inner) == state.leaf
          _can_interrupt = (!marker.ordered || marker.start == 1) && !blank_start ||
            !blank_start

          # More precisely (per spec):
          can_interrupt2 =
            (!marker.ordered || marker.start == 1 || find_existing_list(state.containers, inner, marker) != nil) &&
            (!blank_start || !para_in_inner)

          if state.leaf != nil && state.leaf.kind == :paragraph && !can_interrupt2 do
            nil
          else
            {list, state} = get_or_create_list(state, inner, marker)
            _inner = List.last(state.containers)

            # Compute content indent
            normal_indent = marker.marker_len
            reduced_indent = marker.marker_len - marker.space_after + 1
            content_indent = if blank_start || marker.space_after >= 5, do: reduced_indent, else: normal_indent

            {item_content, new_base_col} = compute_item_content(line, marker, base_col)

            # Each list item gets a unique ID so that structurally identical items
            # (e.g. two consecutive empty `- ` items) can be distinguished by
            # `update_block/3`, which uses structural equality (`==`) to find blocks.
            # Without a unique ID, `update_block` would always update the first item
            # whose structure happens to match and leave the correct one unchanged.
            item = %{
              kind: :list_item,
              _id: :erlang.unique_integer([:positive]),
              marker: marker.marker,
              marker_indent: marker.indent,
              content_indent: content_indent,
              children: [],
              had_blank_line: false
            }

            # Add item to list — use a placeholder reference to find the updated list
            item_ref = item
            state = update_block(state, list, fn lst ->
              %{lst | items: lst.items ++ [item_ref]}
            end)

            # The list in state.containers may be stale; find the fresh version
            # by looking it up in the root document's tree
            fresh_list = find_current_list_in_tree(state, list)

            # Push list (if needed) then item onto container stack
            state =
              if last_container_is_list?(state.containers, fresh_list) do
                state
              else
                %{state | containers: state.containers ++ [fresh_list]}
              end

            # Push the item that was added to the list (fresh_list.items last)
            fresh_item = List.last(fresh_list.items)
            state = %{state | containers: state.containers ++ [fresh_item], leaf: nil, last_blank: false}

            if blank_start do
              state
            else
              detect_block(state, fresh_item, item_content, new_base_col, false)
            end
          end
      end
    end
  end

  defp try_indented_code(state, inner, line, base_col, indent) do
    if indent >= 4 && (state.leaf == nil || state.leaf.kind != :paragraph) do
      {stripped, _} = strip_indent(line, 4, base_col)

      if state.leaf != nil && state.leaf.kind == :indented_code do
        leaf2 = %{state.leaf | lines: state.leaf.lines ++ [stripped]}
        state = update_leaf(state, leaf2)
        %{state | last_blank: false}
      else
        state = close_leaf(state, inner)
        inner = List.last(state.containers)
        icb = %{kind: :indented_code, lines: [stripped]}
        state = add_child_to(state, inner, icb)
        %{state | leaf: icb, last_blank: false}
      end
    end
  end

  defp try_blocks(state, inner, line, base_col, stripped, indent) do
    fence_m = Regex.run(~r/^(`{3,}|~{3,})/, stripped)

    with nil <- try_fenced(state, inner, line, base_col, stripped, indent, fence_m),
         nil <- try_atx(state, inner, line, base_col, indent),
         nil <- try_thematic_or_setext(state, inner, line, base_col, indent),
         nil <- try_html_block(state, inner, line, base_col, stripped, indent),
         nil <- try_blockquote(state, inner, line, base_col, stripped, indent),
         nil <- try_list_item(state, inner, line, base_col, indent),
         nil <- try_indented_code(state, inner, line, base_col, indent) do
      # Paragraph
      if state.leaf != nil && state.leaf.kind == :paragraph do
        leaf2 = %{state.leaf | lines: state.leaf.lines ++ [line]}
        state = update_leaf(state, leaf2)
        %{state | last_blank: false}
      else
        state = close_leaf(state, inner)
        inner = List.last(state.containers)
        para = %{kind: :paragraph, lines: [line]}
        state = add_child_to(state, inner, para)
        %{state | leaf: para, last_blank: false}
      end
    end
  end

  # ── Block Finalization ────────────────────────────────────────────────────────

  defp finalize_leaf_if_open(state) do
    if state.leaf != nil do
      inner = List.last(state.containers)
      finalize_leaf_in(state, inner)
    else
      state
    end
  end

  defp finalize_leaf_in(state, container) do
    {leaf2, refs2} = finalize_block(state.leaf, container, state.refs)
    state = update_leaf(state, leaf2)
    %{state | leaf: nil, refs: refs2}
  end

  defp finalize_block(%{kind: :paragraph} = block, _container, refs) do
    text = Enum.join(block.lines, "\n")
    {remaining, new_refs} = extract_link_defs(text, refs)

    lines =
      if String.trim(remaining) == "" do
        []
      else
        ls = String.split(remaining, "\n")
        List.update_at(ls, length(ls) - 1, &String.trim_trailing/1)
      end

    {%{block | lines: lines}, new_refs}
  end

  defp finalize_block(%{kind: :indented_code} = block, _container, refs) do
    lines = block.lines
      |> Enum.reverse()
      |> Enum.drop_while(&(String.trim(&1) == ""))
      |> Enum.reverse()
    {%{block | lines: lines}, refs}
  end

  defp finalize_block(block, _container, refs), do: {block, refs}

  defp close_leaf(state, inner) do
    if state.leaf == nil do
      state
    else
      {leaf2, refs2} = finalize_block(state.leaf, inner, state.refs)
      state = update_leaf(state, leaf2)
      %{state | leaf: nil, refs: refs2}
    end
  end

  # ── Link Reference Definition Parsing ────────────────────────────────────────

  defp extract_link_defs(text, refs) do
    case parse_link_definition(text) do
      nil ->
        {text, refs}

      d ->
        new_refs =
          if Map.has_key?(refs, d.label) do
            refs
          else
            Map.put(refs, d.label, %{destination: d.destination, title: d.title})
          end

        # Use binary_part (byte-based) since chars_consumed is a byte offset.
        consumed = d.chars_consumed
        remaining =
          if consumed >= byte_size(text), do: "",
          else: binary_part(text, consumed, byte_size(text) - consumed)
        extract_link_defs(remaining, new_refs)
    end
  end

  @doc false
  def parse_link_definition(text) do
    case Regex.run(~r/^ {0,3}\[([^\]\\\[]*(?:\\.[^\]\\\[]*)*)\]:/, text) do
      nil ->
        nil

      [match, raw_label] ->
        if String.trim(raw_label) == "" do
          nil
        else
          label = Scanner.normalize_link_label(raw_label)
          pos = byte_size(match)
          parse_ld_dest(text, pos, label)
        end
    end
  end

  defp parse_ld_dest(text, pos, label) do
    {pos, _} = skip_ws_newline(text, pos)

    cond do
      pos >= byte_size(text) ->
        nil

      true ->
        ch = safe_at(text, pos)

        {dest, pos} =
          if ch == "<" do
            parse_angle_dest(text, pos + 1)
          else
            parse_bare_dest(text, pos)
          end

        if dest == nil do
          nil
        else
          before_title = pos

          # Title must be separated from destination by whitespace or newline.
          # If there is no whitespace, skip the title attempt entirely.
          {pos_after, ws_len} = skip_ws_newline(text, pos)
          {title, pos} =
            if ws_len > 0 do
              parse_title(text, pos_after, before_title)
            else
              {nil, before_title}
            end

          # Rest of line after title (or after dest if no title) must be blank.
          rest = binary_part(text, pos, byte_size(text) - pos)
          eol = Regex.run(~r/^[ \t]*(?:\n|$)/, rest)

          if eol != nil do
            %{label: label, destination: dest, title: title,
              chars_consumed: pos + byte_size(hd(eol))}
          else
            if title != nil do
              # Title parsing consumed too much — retry without title.
              rest2 = binary_part(text, before_title, byte_size(text) - before_title)
              eol2 = Regex.run(~r/^[ \t]*(?:\n|$)/, rest2)

              if eol2 != nil do
                %{label: label, destination: dest, title: nil,
                  chars_consumed: before_title + byte_size(hd(eol2))}
              else
                nil
              end
            else
              nil
            end
          end
        end
    end
  end

  defp parse_angle_dest(text, pos) do
    {inner, end_pos} = scan_angle_dest(text, pos, "")

    if inner != nil do
      dest = Scanner.normalize_url(Entities.decode_entities(apply_backslash_escapes(inner)))
      {dest, end_pos}
    else
      {nil, pos}
    end
  end

  defp scan_angle_dest(text, pos, acc) do
    if pos >= byte_size(text) do
      {nil, pos}
    else
      ch = safe_at(text, pos)

      cond do
        ch == "\n" || ch == "<" -> {nil, pos}
        ch == ">" -> {acc, pos + 1}
        ch == "\\" ->
          next = safe_at(text, pos + 1)
          scan_angle_dest(text, pos + 2, acc <> "\\" <> next)
        true ->
          scan_angle_dest(text, pos + byte_size(ch), acc <> ch)
      end
    end
  end

  defp parse_bare_dest(text, pos) do
    start = pos
    end_pos = scan_bare_dest(text, pos, 0)

    if end_pos == start do
      {nil, pos}
    else
      raw = binary_part(text, start, end_pos - start)
      dest = Scanner.normalize_url(Entities.decode_entities(apply_backslash_escapes(raw)))
      {dest, end_pos}
    end
  end

  defp scan_bare_dest(text, pos, depth) do
    if pos >= byte_size(text) do
      pos
    else
      ch = safe_at(text, pos)

      cond do
        ch == "(" -> scan_bare_dest(text, pos + 1, depth + 1)
        ch == ")" && depth == 0 -> pos
        ch == ")" -> scan_bare_dest(text, pos + 1, depth - 1)
        ch == "\\" -> scan_bare_dest(text, pos + 2, depth)
        ch in [" ", "\t", "\n", "\r", "\f"] -> pos
        byte_size(ch) == 1 && :binary.at(ch, 0) <= 0x1F -> pos
        true -> scan_bare_dest(text, pos + byte_size(ch), depth)
      end
    end
  end

  defp parse_title(text, pos, before_title) do
    if pos >= byte_size(text) do
      {nil, before_title}
    else
      ch = safe_at(text, pos)
      close = case ch do
        "\"" -> "\""
        "'" -> "'"
        "(" -> ")"
        _ -> nil
      end

      if close == nil do
        {nil, before_title}
      else
        title_start = pos + 1
        case scan_title(text, title_start, close) do
          {content, end_pos} ->
            decoded = Entities.decode_entities(apply_backslash_escapes(content))
            {decoded, end_pos}
          nil ->
            {nil, before_title}
        end
      end
    end
  end

  defp scan_title(text, pos, close) do
    do_scan_title(text, pos, pos, close)
  end

  defp do_scan_title(text, pos, start, close) do
    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)

      cond do
        ch == "\\" -> do_scan_title(text, pos + 2, start, close)
        ch == close ->
          # Use binary_part (byte-based) since start and pos are byte offsets.
          content = binary_part(text, start, pos - start)
          {content, pos + 1}
        ch == "\n" && close == ")" -> nil
        true -> do_scan_title(text, pos + byte_size(ch), start, close)
      end
    end
  end

  defp skip_ws_newline(text, pos) do
    # Use binary_part (byte-based) not String.slice (char-based) because pos
    # is always a byte offset and text may contain multi-byte Unicode chars.
    rest =
      if pos >= byte_size(text), do: "", else: binary_part(text, pos, byte_size(text) - pos)
    case Regex.run(~r/^[ \t]*\n?[ \t]*/, rest) do
      [m] -> {pos + byte_size(m), byte_size(m)}
      _ -> {pos, 0}
    end
  end

  # ── AST Conversion ────────────────────────────────────────────────────────────

  defp conv_block(%{kind: :document} = b, raw, ctr) do
    {children, raw, ctr} = conv_children(b.children, raw, ctr)
    children = Enum.filter(children, &(&1 != nil))
    {%{type: :document, children: children}, raw, ctr}
  end

  defp conv_block(%{kind: :heading} = b, raw, ctr) do
    raw = Map.put(raw, ctr, b.content)
    {%{type: :heading, level: b.level, children: [], _raw_id: ctr}, raw, ctr + 1}
  end

  defp conv_block(%{kind: :paragraph} = b, raw, ctr) do
    if length(b.lines) == 0 do
      {nil, raw, ctr}
    else
      content = b.lines
        |> Enum.map(&Regex.replace(~r/^[ \t]+/, &1, ""))
        |> Enum.join("\n")
      raw = Map.put(raw, ctr, content)
      {%{type: :paragraph, children: [], _raw_id: ctr}, raw, ctr + 1}
    end
  end

  defp conv_block(%{kind: :fenced_code} = b, raw, ctr) do
    value = if length(b.lines) > 0, do: Enum.join(b.lines, "\n") <> "\n", else: ""
    lang = if b.info_string == "", do: nil, else: b.info_string
    {%{type: :code_block, language: lang, value: value}, raw, ctr}
  end

  defp conv_block(%{kind: :indented_code} = b, raw, ctr) do
    value = Enum.join(b.lines, "\n") <> "\n"
    {%{type: :code_block, language: nil, value: value}, raw, ctr}
  end

  defp conv_block(%{kind: :blockquote} = b, raw, ctr) do
    {children, raw, ctr} = conv_children(b.children, raw, ctr)
    children = Enum.filter(children, &(&1 != nil))
    {%{type: :blockquote, children: children}, raw, ctr}
  end

  defp conv_block(%{kind: :list} = b, raw, ctr) do
    is_tight = b.tight && !b.had_blank_line &&
      !Enum.any?(b.items, &(&1.had_blank_line && length(&1.children) > 1))

    {items, raw, ctr} = conv_children(b.items, raw, ctr)
    items = Enum.filter(items, &(&1 != nil))

    {%{type: :list, ordered: b.ordered, start: (if b.ordered, do: b.start, else: nil),
       tight: is_tight, children: items}, raw, ctr}
  end

  defp conv_block(%{kind: :list_item} = b, raw, ctr) do
    {children, raw, ctr} = conv_children(b.children, raw, ctr)
    children = Enum.filter(children, &(&1 != nil))
    {%{type: :list_item, children: children}, raw, ctr}
  end

  defp conv_block(%{kind: :thematic_break}, raw, ctr) do
    {%{type: :thematic_break}, raw, ctr}
  end

  defp conv_block(%{kind: :html_block} = b, raw, ctr) do
    lines = b.lines
      |> Enum.reverse()
      |> Enum.drop_while(&(String.trim(&1) == ""))
      |> Enum.reverse()

    value = Enum.join(lines, "\n") <> "\n"
    {%{type: :raw_block, format: "html", value: value}, raw, ctr}
  end

  defp conv_block(_, raw, ctr), do: {nil, raw, ctr}

  defp conv_children(children, raw, ctr) do
    Enum.reduce(children, {[], raw, ctr}, fn child, {acc, r, c} ->
      {node, r2, c2} = conv_block(child, r, c)
      {acc ++ [node], r2, c2}
    end)
  end

  # ── Container/Block Helpers ──────────────────────────────────────────────────

  # Add a block as child of `container` in the tree
  defp add_child_to(state, container, block) do
    update_block(state, container, fn c ->
      case c.kind do
        :document -> %{c | children: c.children ++ [block]}
        :blockquote -> %{c | children: c.children ++ [block]}
        :list_item -> %{c | children: c.children ++ [block]}
        _ -> c
      end
    end)
  end

  defp last_child_of(%{kind: k, children: children}) when k in [:document, :blockquote, :list_item] do
    List.last(children)
  end
  defp last_child_of(_), do: nil

  defp remove_last_child_from(state, container) do
    update_block(state, container, fn c ->
      case c.kind do
        k when k in [:document, :blockquote, :list_item] ->
          %{c | children: Enum.drop(c.children, -1)}
        _ -> c
      end
    end)
  end

  defp replace_last_child(state, container, new_block) do
    update_block(state, container, fn c ->
      case c.kind do
        k when k in [:document, :blockquote, :list_item] ->
          %{c | children: Enum.drop(c.children, -1) ++ [new_block]}
        _ -> c
      end
    end)
  end

  # Update a leaf block in-place across the entire container tree
  defp update_leaf(state, new_leaf) do
    old_leaf = state.leaf || new_leaf
    containers = update_all(state.containers, old_leaf, new_leaf)
    %{state | containers: containers, leaf: new_leaf}
  end

  # Update a specific block in the tree identified by object identity
  defp update_block(state, target, fun) do
    containers = Enum.map(state.containers, fn c ->
      if c == target do
        fun.(c)
      else
        deep_update(c, target, fun)
      end
    end)
    %{state | containers: containers}
  end

  defp deep_update(%{kind: k} = block, target, fun) when k in [:document, :blockquote, :list_item] do
    children = Enum.map(block.children, fn ch ->
      if ch == target, do: fun.(ch), else: deep_update(ch, target, fun)
    end)
    %{block | children: children}
  end

  defp deep_update(%{kind: :list} = block, target, fun) do
    items = Enum.map(block.items, fn item ->
      if item == target, do: fun.(item), else: deep_update(item, target, fun)
    end)
    %{block | items: items}
  end

  defp deep_update(block, _target, _fun), do: block

  defp update_all(containers, old_ref, new_val) do
    Enum.map(containers, fn c ->
      if c == old_ref do
        new_val
      else
        deep_update_leaf(c, old_ref, new_val)
      end
    end)
  end

  defp deep_update_leaf(%{kind: k} = block, old, new) when k in [:document, :blockquote, :list_item] do
    children = Enum.map(block.children, fn ch ->
      if ch == old, do: new, else: deep_update_leaf(ch, old, new)
    end)
    %{block | children: children}
  end

  defp deep_update_leaf(%{kind: :list} = block, old, new) do
    items = Enum.map(block.items, fn item ->
      if item == old, do: new, else: deep_update_leaf(item, old, new)
    end)
    %{block | items: items}
  end

  defp deep_update_leaf(block, _old, _new), do: block

  defp find_existing_list(_containers, inner, marker) do
    last_in_inner = last_child_of(inner)
    existing =
      cond do
        inner.kind == :list && inner.ordered == marker.ordered && inner.marker == marker.marker ->
          inner
        last_in_inner != nil && last_in_inner.kind == :list &&
            last_in_inner.ordered == marker.ordered && last_in_inner.marker == marker.marker ->
          last_in_inner
        true ->
          nil
      end
    existing
  end

  defp get_or_create_list(state, inner, marker) do
    existing = find_existing_list(state.containers, inner, marker)

    if existing != nil do
      # Update tightness based on blank lines
      list =
        if existing.had_blank_line || (state.last_blank &&
             inner(state.last_blank_in, [:list, :list_item])) do
          %{existing | tight: false}
        else
          existing
        end

      list = %{list | had_blank_line: false}
      state = update_block(state, existing, fn _ -> list end)
      {list, state}
    else
      state = close_leaf(state, inner)
      inner = List.last(state.containers)
      # Each list gets a unique ID so that nested lists with the same marker
      # (e.g. `- - foo`) can be distinguished by `find_current_list_in_tree/2`,
      # which otherwise would return the outer list first when searching by marker.
      new_list = %{
        kind: :list,
        _id: :erlang.unique_integer([:positive]),
        ordered: marker.ordered,
        marker: marker.marker,
        start: marker.start,
        tight: true,
        items: [],
        had_blank_line: false
      }

      state = add_child_to(state, inner, new_list)
      {new_list, state}
    end
  end

  defp maybe_close_list(state, line, blank) do
    if blank || length(state.containers) <= 1 do
      state
    else
      case List.last(state.containers) do
        %{kind: :list} = lst ->
          marker = parse_list_marker(line)

          if marker != nil && lst.ordered == marker.ordered &&
               lst.marker == marker.marker && !thematic_break?(line) do
            state
          else
            %{state | containers: Enum.drop(state.containers, -1)}
          end

        _ ->
          state
      end
    end
  end

  # Find the freshest list block from the state's container stack.
  # After update_block, the containers stack holds the updated root.
  # We search for a list matching the old block's signature in the tree.
  defp find_current_list_in_tree(state, old_list) do
    find_list_in_block(hd(state.containers), old_list)
  end

  defp find_list_in_block(%{kind: :document, children: children}, target) do
    Enum.find_value(children, fn ch -> find_list_in_block(ch, target) end)
  end

  defp find_list_in_block(%{kind: :blockquote, children: children}, target) do
    Enum.find_value(children, fn ch -> find_list_in_block(ch, target) end)
  end

  defp find_list_in_block(%{kind: :list_item, children: children}, target) do
    Enum.find_value(children, fn ch -> find_list_in_block(ch, target) end)
  end

  defp find_list_in_block(%{kind: :list} = b, target) do
    # If both have a `_id` field, use it for exact identity matching.
    # This is critical when nested lists share the same marker (e.g. `- - foo`):
    # without the ID, a depth-first search would return the outer list because
    # it matches the marker first.
    id_match =
      case {Map.get(b, :_id), Map.get(target, :_id)} do
        {id, id} when not is_nil(id) -> true
        _ -> false
      end

    if id_match do
      b
    else
      # Fall back to marker matching only for lists that predate the _id field
      # or when IDs differ — in that case, recurse into items
      if b.ordered == target.ordered && b.marker == target.marker && is_nil(Map.get(target, :_id)) do
        b
      else
        Enum.find_value(b.items, fn item -> find_list_in_block(item, target) end)
      end
    end
  end

  defp find_list_in_block(_, _), do: nil

  # Check if the last container in the stack is equivalent to the given list
  # (by comparing kind and marker, since list may have been updated)
  defp last_container_is_list?(containers, list) do
    case List.last(containers) do
      %{kind: :list, ordered: o, marker: m} ->
        o == list.ordered && m == list.marker
      _ -> false
    end
  end

  # ── Line Analysis Helpers ─────────────────────────────────────────────────────

  @doc false
  def blank?(line), do: Regex.match?(~r/^\s*$/, line)

  @doc false
  def indent_of(line, base_col \\ 0) do
    do_indent(line, base_col, base_col) - base_col
  end

  defp do_indent("", col, _base), do: col
  defp do_indent(<<?\s, rest::binary>>, col, base), do: do_indent(rest, col + 1, base)
  defp do_indent(<<?\t, rest::binary>>, col, base), do: do_indent(rest, col + (4 - rem(col, 4)), base)
  defp do_indent(_, col, _base), do: col

  @doc false
  def strip_indent(line, n, base_col \\ 0) do
    do_strip(line, n, base_col)
  end

  defp do_strip(line, 0, col), do: {line, col}
  defp do_strip("", _, col), do: {"", col}

  defp do_strip(<<?\s, rest::binary>>, n, col) when n > 0 do
    do_strip(rest, n - 1, col + 1)
  end

  defp do_strip(<<?\t, rest::binary>>, n, col) when n > 0 do
    w = 4 - rem(col, 4)

    if w <= n do
      do_strip(rest, n - w, col + w)
    else
      leftover = w - n
      {String.duplicate(" ", leftover) <> rest, col + n}
    end
  end

  defp do_strip(line, _, col), do: {line, col}

  defp fenced_opener?(line) do
    Regex.match?(~r/^(`{3,}|~{3,})/, String.trim_leading(line))
  end

  @doc false
  def parse_atx_heading(line) do
    case Regex.run(~r/^ {0,3}([#]{1,6})([ \t]|$)(.*)/, line) do
      nil ->
        nil

      [_, hashes, _, rest] ->
        content = String.trim_trailing(rest)
        content = Regex.replace(~r/[ \t][#]+[ \t]*$/, content, "")
        content = if Regex.match?(~r/^[#]+[ \t]*$/, content), do: "", else: content
        %{level: String.length(hashes), content: String.trim(content)}
    end
  end

  @doc false
  def thematic_break?(line) do
    Regex.match?(~r/^ {0,3}((?:\*[ \t]*){3,}|(?:-[ \t]*){3,}|(?:_[ \t]*){3,})\s*$/, line)
  end

  defp is_setext_underline(line) do
    cond do
      Regex.match?(~r/^ {0,3}=+\s*$/, line) -> 1
      Regex.match?(~r/^ {0,3}-+\s*$/, line) -> 2
      true -> nil
    end
  end

  @doc false
  def parse_list_marker(line) do
    unordered = Regex.run(~r/^( {0,3})([-*+])( +|\t|$)/, line)

    if unordered do
      [_, ind, marker, space] = unordered
      %{ordered: false, start: 1, marker: marker,
        marker_len: String.length(ind) + 1 + String.length(space),
        space_after: String.length(space), indent: String.length(ind)}
    else
      ordered = Regex.run(~r/^( {0,3})(\d{1,9})([.)])( +|\t|$)/, line)

      if ordered do
        [_, ind, num_str, delim, space] = ordered
        %{ordered: true, start: String.to_integer(num_str), marker: delim,
          marker_len: String.length(ind) + String.length(num_str) + 1 + String.length(space),
          space_after: String.length(space), indent: String.length(ind)}
      else
        nil
      end
    end
  end

  defp extract_info_string(line) do
    case Regex.run(~r/^[`~]+\s*(.*)$/, line) do
      nil -> ""
      [_, raw] ->
        raw |> String.trim() |> String.split(~r/\s+/) |> List.first("") |>
          then(&Entities.decode_entities(apply_backslash_escapes(&1)))
    end
  end

  @doc false
  def apply_backslash_escapes(s) do
    Regex.replace(~r/\\(.)/, s, fn _, ch ->
      if Scanner.ascii_punctuation?(ch), do: ch, else: "\\" <> ch
    end)
  end

  defp detect_html_type(stripped) do
    cond do
      Regex.match?(@html1_open, stripped) -> 1
      Regex.match?(@html2_open, stripped) -> 2
      Regex.match?(@html3_open, stripped) -> 3
      Regex.match?(@html4_open, stripped) -> 4
      Regex.match?(@html5_open, stripped) -> 5
      Regex.match?(@html6_open, stripped) -> 6
      Regex.match?(@html7_open_tag, stripped) || Regex.match?(@html7_close_tag, stripped) -> 7
      true -> nil
    end
  end

  defp html_block_ends?(line, type) do
    case type do
      1 -> Regex.match?(@html1_close, line)
      2 -> Regex.match?(@html2_close, line)
      3 -> Regex.match?(@html3_close, line)
      4 -> Regex.match?(@html4_close, line)
      5 -> Regex.match?(@html5_close, line)
      n when n in [6, 7] -> Regex.match?(~r/^\s*$/, line)
      _ -> false
    end
  end

  defp skip_spaces_n(line, base_col, max) do
    do_skip_n(line, 0, base_col, max)
  end

  defp do_skip_n(_, idx, col, 0), do: {idx, col}
  defp do_skip_n("", idx, col, _), do: {idx, col}
  defp do_skip_n(<<?\s, rest::binary>>, idx, col, max), do: do_skip_n(rest, idx + 1, col + 1, max - 1)
  defp do_skip_n(_, idx, col, _), do: {idx, col}

  defp strip_bq_marker(line, bi, bc) do
    # bi is position of ">", bc is virtual column
    next_i = bi + 1
    next_bc = bc + 1

    if next_i >= byte_size(line) do
      {"", next_bc, true}
    else
      ch = safe_at(line, next_i)

      cond do
        ch == " " ->
          {String.slice(line, next_i + 1..-1//1), next_bc + 1, true}

        ch == "\t" ->
          w = 4 - rem(next_bc, 4)
          after_tab = next_i + 1

          if w > 1 do
            leftover = w - 1
            stripped = String.duplicate(" ", leftover) <> String.slice(line, after_tab..-1//1)
            {stripped, next_bc + 1, false}
          else
            {String.slice(line, after_tab..-1//1), next_bc + w, true}
          end

        true ->
          {String.slice(line, next_i..-1//1), next_bc, true}
      end
    end
  end

  defp strip_bq_line(line, base_col) do
    {bi, bc} = skip_spaces_n(line, base_col, 3)
    ch = safe_at(line, bi)

    if ch == ">" do
      {stripped, new_bc, _} = strip_bq_marker(line, bi, bc)
      {stripped, new_bc}
    else
      {line, base_col}
    end
  end

  defp compute_item_content(line, marker, base_col) do
    item_content = String.slice(line, marker.marker_len..-1//1)
    new_bc = virtual_col_after(line, marker.marker_len, base_col)

    cond do
      # When space_after >= 5, content_indent is REDUCED to (marker_len - space_after + 1).
      # The actual content starts at marker_len but the effective list base is at
      # content_indent. The gap (marker_len - content_indent = space_after - 1) must be
      # prepended as virtual spaces so that `detect_block` can correctly identify
      # indented code blocks within the list item.
      #
      # Example: `1.     foo` (5 spaces after `1.`)
      #   marker_len = 7, space_after = 5, content_indent = 3
      #   extra_spaces = 7 - 3 = 4
      #   item_content = "    foo" (with 4 prepended spaces)
      #   detect_block sees indent=4 → indented code block, stripped to "foo"
      marker.space_after >= 5 ->
        content_indent = marker.marker_len - marker.space_after + 1
        extra_spaces = marker.marker_len - content_indent  # = space_after - 1
        {String.duplicate(" ", extra_spaces) <> item_content, content_indent}

      marker.space_after == 1 ->
        sep_ch = safe_at(line, marker.marker_len - 1)

        if sep_ch == "\t" do
          sep_col = virtual_col_after(line, marker.marker_len - 1, base_col)
          w = 4 - rem(sep_col, 4)

          if w > 1 do
            {String.duplicate(" ", w - 1) <> item_content, sep_col + 1}
          else
            {item_content, new_bc}
          end
        else
          {item_content, new_bc}
        end

      true ->
        {item_content, new_bc}
    end
  end

  defp virtual_col_after(line, count, start_col) do
    do_vcol(line, 0, count, start_col)
  end

  defp do_vcol(_, idx, max, col) when idx >= max, do: col
  defp do_vcol("", _, _, col), do: col
  defp do_vcol(<<?\t, rest::binary>>, idx, max, col), do: do_vcol(rest, idx + 1, max, col + (4 - rem(col, 4)))
  defp do_vcol(<<_, rest::binary>>, idx, max, col), do: do_vcol(rest, idx + 1, max, col + 1)

  defp safe_at(_str, pos) when pos < 0, do: ""
  defp safe_at(str, pos) when pos >= byte_size(str), do: ""
  defp safe_at(str, pos) do
    case String.next_codepoint(binary_part(str, pos, byte_size(str) - pos)) do
      {ch, _} -> ch
      nil -> ""
    end
  end
end
