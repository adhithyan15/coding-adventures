defmodule CodingAdventures.CommonmarkParser.InlineParser do
  @moduledoc """
  Inline-Level Parser — Phase 2 of CommonMark parsing.

  Transforms raw inline content strings (produced by the block parser) into
  inline node trees: emphasis, strong, links, images, code spans, autolinks,
  hard breaks, soft breaks, and plain text.

  ## Algorithm: Delimiter Stack (CommonMark Appendix A)

  The delimiter stack algorithm handles `*`, `_`, `[`, `![`, and `]` specially.
  All other characters are emitted directly as text. The algorithm:

  1. Scan left to right. When a `*` or `_` run is found, push a delimiter
     onto the stack tracking: character, count, left-flanking, right-flanking.

  2. When `[` or `![` is found, push a "link opener" delimiter.

  3. When `]` is found, look back through the delimiter stack for a matching
     `[` or `![`. If found, try to parse an inline link `(url)` or look up
     the label in link references. If a link is formed, wrap the content between
     opener and closer as a link/image node.

  4. After the full text is scanned, process emphasis/strong pairs: scan
     right to left for closers, for each closer scan left for a matching opener
     with correct flanking rules and divisibility-by-3 rule.

  5. Convert all delimiters to nodes, concatenate adjacent text nodes.

  ## Key Data Structures

  The parser maintains a "node array" (a list indexed by position) where each
  entry is either a `{:text, string}`, `{:node, map}`, or
  `{:delim, char, count, can_open, can_close}` entry.

  After the emphasis pass, unmatched delimiters are converted to text nodes.
  """

  alias CodingAdventures.CommonmarkParser.Scanner
  alias CodingAdventures.CommonmarkParser.Entities
  alias CodingAdventures.DocumentAst

  # ── Public API ───────────────────────────────────────────────────────────────

  @doc """
  Parse an inline content string into a list of inline nodes.

  `refs` is the link reference map produced by the block parser (normalized
  label → `%{destination: url, title: title_or_nil}`).
  """
  @spec parse(String.t(), map()) :: [DocumentAst.inline_node()]
  def parse(text, refs \\ %{}) do
    items = scan_items(text, refs)
    items = resolve_emphasis(items)
    items_to_nodes(items)
  end

  # ── Item types ────────────────────────────────────────────────────────────────
  # {:text, string}                          — literal text
  # {:node, map}                             — already-resolved inline node
  # {:delim, char, count, can_open, can_close} — delimiter run (*, _)
  # {:opener, id, char}                      — link/image opener ([ or ![)
  # {:break, :hard | :soft}                  — line break

  # ── Scanning Pass ─────────────────────────────────────────────────────────────

  defp scan_items(text, refs) do
    do_scan(Scanner.new(text), [], refs)
    |> Enum.reverse()
  end

  defp do_scan(scanner, acc, refs) do
    if Scanner.done?(scanner) do
      acc
    else
      ch = Scanner.peek(scanner)

      cond do
        ch == "\\" -> scan_backslash(scanner, acc, refs)
        ch == "`" -> scan_code_span(scanner, acc, refs)
        ch == "<" -> scan_angle(scanner, acc, refs)
        ch == "*" or ch == "_" -> scan_delim_run(scanner, acc, refs)
        ch == "[" -> scan_bracket_open(scanner, acc, refs)
        ch == "!" && Scanner.peek(scanner, 1) == "[" -> scan_image_open(scanner, acc, refs)
        ch == "]" -> scan_bracket_close(scanner, acc, refs, text_from_scanner(scanner))
        ch == "&" -> scan_entity(scanner, acc, refs)
        ch == "\n" -> scan_newline(scanner, acc, refs)
        true ->
          {c, s2} = Scanner.advance(scanner)
          do_scan(s2, append_text(acc, c), refs)
      end
    end
  end

  # Full source text (needed for flanking calculations)
  defp text_from_scanner(scanner), do: scanner.source

  # ── Backslash ─────────────────────────────────────────────────────────────────

  defp scan_backslash(scanner, acc, refs) do
    s1 = Scanner.skip(scanner, 1)
    if Scanner.done?(s1) do
      do_scan(s1, append_text(acc, "\\"), refs)
    else
      ch = Scanner.peek(s1)
      {_, s2} = Scanner.advance(s1)
      if ch == "\n" do
        prev_text = last_text(acc)
        trimmed = String.trim_trailing(prev_text)
        acc2 = replace_last_text(acc, trimmed)
        do_scan(s2, [{:break, :hard} | acc2], refs)
      else
        if Scanner.ascii_punctuation?(ch) do
          do_scan(s2, append_text(acc, ch), refs)
        else
          do_scan(s2, append_text(acc, "\\" <> ch), refs)
        end
      end
    end
  end

  # ── Newline ───────────────────────────────────────────────────────────────────

  defp scan_newline(scanner, acc, refs) do
    {_, s2} = Scanner.advance(scanner)
    prev = last_text(acc)
    if String.ends_with?(prev, "  ") or String.ends_with?(prev, "\t") do
      trimmed = String.trim_trailing(prev)
      acc2 = replace_last_text(acc, trimmed)
      do_scan(s2, [{:break, :hard} | acc2], refs)
    else
      trimmed = String.trim_trailing(prev, " ")
      acc2 = replace_last_text(acc, trimmed)
      do_scan(s2, [{:break, :soft} | acc2], refs)
    end
  end

  # ── Code Span ─────────────────────────────────────────────────────────────────

  defp scan_code_span(scanner, acc, refs) do
    {ticks, s2} = Scanner.consume_while(scanner, fn c -> c == "`" end)
    n = byte_size(ticks)
    rest = Scanner.rest(s2)

    case find_matching_ticks(rest, n) do
      {content, skip_len} ->
        s3 = Scanner.skip(s2, skip_len)
        normalized = normalize_code_span(content)
        do_scan(s3, [{:node, DocumentAst.code_span(normalized)} | acc], refs)
      nil ->
        do_scan(s2, append_text(acc, ticks), refs)
    end
  end

  defp find_matching_ticks(text, n) do
    find_ticks_loop(text, 0, n)
  end

  defp find_ticks_loop(text, pos, n) do
    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)
      if ch == "`" do
        {count, end_pos} = count_run(text, pos, "`")
        if count == n do
          {binary_part(text, 0, pos), end_pos}
        else
          find_ticks_loop(text, pos + count, n)
        end
      else
        find_ticks_loop(text, pos + byte_size(ch), n)
      end
    end
  end

  defp count_run(text, pos, char) do
    count_run_loop(text, pos, char, 0)
  end

  defp count_run_loop(text, pos, char, count) do
    if pos < byte_size(text) && binary_part(text, pos, 1) == char do
      count_run_loop(text, pos + 1, char, count + 1)
    else
      {count, pos}
    end
  end

  defp normalize_code_span(content) do
    s = String.replace(content, "\n", " ")
    if String.length(s) >= 2 &&
       String.starts_with?(s, " ") &&
       String.ends_with?(s, " ") &&
       String.trim(s) != "" do
      String.slice(s, 1, String.length(s) - 2)
    else
      s
    end
  end

  # ── Angle Bracket ─────────────────────────────────────────────────────────────

  defp scan_angle(scanner, acc, refs) do
    rest = Scanner.rest(scanner)

    cond do
      # Autolink URL
      Regex.match?(~r/^<[A-Za-z][A-Za-z0-9+\-.]{1,31}:[^\s<>]*>/, rest) ->
        [full, url] = Regex.run(~r/^<([A-Za-z][A-Za-z0-9+\-.]{1,31}:[^\s<>]*)>/, rest)
        s2 = Scanner.skip(scanner, byte_size(full))
        do_scan(s2, [{:node, DocumentAst.autolink(url, false)} | acc], refs)

      # Autolink email
      Regex.match?(~r/^<[a-zA-Z0-9.!#$%&'*+\/=?^_`{|}~\-]+@[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*>/, rest) ->
        [full, email] = Regex.run(~r/^<([a-zA-Z0-9.!#$%&'*+\/=?^_`{|}~\-]+@[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*)>/, rest)
        s2 = Scanner.skip(scanner, byte_size(full))
        do_scan(s2, [{:node, DocumentAst.autolink(email, true)} | acc], refs)

      # HTML comment
      String.starts_with?(rest, "<!--") ->
        # Comment must not start with >, ->, or contain --
        case scan_html_comment(rest) do
          {html, len} ->
            s2 = Scanner.skip(scanner, len)
            do_scan(s2, [{:node, DocumentAst.raw_inline("html", html)} | acc], refs)
          nil -> do_scan(Scanner.skip(scanner, 1), append_text(acc, "<"), refs)
        end

      # Processing instruction
      String.starts_with?(rest, "<?") ->
        case scan_until(rest, "?>") do
          {html, len} ->
            s2 = Scanner.skip(scanner, len)
            do_scan(s2, [{:node, DocumentAst.raw_inline("html", html)} | acc], refs)
          nil -> do_scan(Scanner.skip(scanner, 1), append_text(acc, "<"), refs)
        end

      # CDATA
      String.starts_with?(rest, "<![CDATA[") ->
        case scan_until(rest, "]]>") do
          {html, len} ->
            s2 = Scanner.skip(scanner, len)
            do_scan(s2, [{:node, DocumentAst.raw_inline("html", html)} | acc], refs)
          nil -> do_scan(Scanner.skip(scanner, 1), append_text(acc, "<"), refs)
        end

      # Declaration
      Regex.match?(~r/^<![A-Z]/, rest) ->
        case scan_until(rest, ">") do
          {html, len} ->
            s2 = Scanner.skip(scanner, len)
            do_scan(s2, [{:node, DocumentAst.raw_inline("html", html)} | acc], refs)
          nil -> do_scan(Scanner.skip(scanner, 1), append_text(acc, "<"), refs)
        end

      # HTML tag
      Regex.match?(~r/^<\/?[A-Za-z]/, rest) ->
        case scan_html_tag(rest) do
          {html, len} ->
            s2 = Scanner.skip(scanner, len)
            do_scan(s2, [{:node, DocumentAst.raw_inline("html", html)} | acc], refs)
          nil -> do_scan(Scanner.skip(scanner, 1), append_text(acc, "<"), refs)
        end

      true ->
        do_scan(Scanner.skip(scanner, 1), append_text(acc, "<"), refs)
    end
  end

  defp scan_html_comment(text) do
    # CommonMark spec: HTML comment = "<!--" + content + "-->" where:
    #   - content does not start with ">"
    #   - content does not start with "->"
    #   - content does not end with "-" (i.e., the close is exactly "-->", not "--->" etc.)
    #
    # Note: the spec allows "--" within the comment body as long as it is not the
    # close sequence. We scan for "-->" (three characters) and validate the start.
    # Spec example 625: `<!-- this is a --\ncomment... -->` is valid raw HTML because
    # the `--` mid-comment is not immediately followed by ">".
    # Spec examples 626: `<!--> foo -->` and `<!--->`  are invalid (start with `>` and `->`).
    if byte_size(text) < 7 do
      nil
    else
      content_start = 4  # position after "<!--"

      if content_start >= byte_size(text) do
        nil
      else
        first = safe_at(text, content_start)
        second = if content_start + 1 < byte_size(text), do: safe_at(text, content_start + 1), else: ""

        cond do
          # CommonMark §6.11 / cmark behaviour:
          # When content starts with ">" or "->", cmark emits "<!--" + the
          # invalid starter as a raw HTML fragment (rather than escaping "<").
          # Example 626: "<!--> foo -->" → raw "<!-->" + text " foo -->"
          #              "<!---> foo -->" → raw "<!--->" + text " foo -->"
          first == ">" ->
            {"<!--" <> ">", 5}

          first == "-" && second == ">" ->
            {"<!--" <> "->", 6}

          true ->
            # Find the closing "-->"
            case scan_until_pattern(text, content_start, "-->") do
              {_, end_pos} ->
                # Ensure the comment doesn't end with "--->" (content ends with "-")
                # The content just before "-->" must not be "-"
                close_start = end_pos - 3  # position of "-->"
                content_end = close_start   # position just before "-->"
                content_ends_with_dash = content_end > content_start &&
                  safe_at(text, content_end - 1) == "-"
                if content_ends_with_dash do
                  nil
                else
                  {binary_part(text, 0, end_pos), end_pos}
                end
              nil -> nil
            end
        end
      end
    end
  end

  defp scan_until_pattern(text, pos, pattern) do
    case :binary.match(text, pattern, scope: {pos, byte_size(text) - pos}) do
      {found_pos, _} -> {binary_part(text, 0, found_pos), found_pos + byte_size(pattern)}
      :nomatch -> nil
    end
  end

  defp scan_until(text, close) do
    case :binary.match(text, close) do
      {pos, len} ->
        end_pos = pos + len
        {binary_part(text, 0, end_pos), end_pos}
      :nomatch -> nil
    end
  end

  defp scan_html_tag(text) do
    # Per CommonMark spec, inline raw HTML tags must be:
    #   - Open tags: <tagname attrs... /> or <tagname attrs...>
    #   - Closing tags: </tagname> with NO attributes and NO extra whitespace
    # Closing tags with attributes (e.g. </a href="foo">) are NOT valid raw HTML.
    # Attribute values can span newlines per CommonMark spec (examples 642, 643).
    # Use `[^']*` and `[^"]*` (no newline restriction) for attribute value scanning.
    open_re = ~r/^<[A-Za-z][A-Za-z0-9\-]*(?:\s+[a-zA-Z_:][a-zA-Z0-9_:.\-]*(?:\s*=\s*(?:[^\s"'=<>`]+|'[^']*'|"[^"]*"))?)* *\/?>/s
    close_re = ~r/^<\/[A-Za-z][A-Za-z0-9\-]*\s*>/

    cond do
      Regex.match?(~r/^<\//, text) ->
        case Regex.run(close_re, text) do
          [m] -> {m, byte_size(m)}
          nil -> nil
        end

      true ->
        case Regex.run(open_re, text) do
          [m] -> {m, byte_size(m)}
          nil -> nil
        end
    end
  end

  # ── Delimiter Run (* or _) ────────────────────────────────────────────────────

  defp scan_delim_run(scanner, acc, refs) do
    source = scanner.source
    pos = scanner.pos
    ch = Scanner.peek(scanner)
    {run, s2} = Scanner.consume_while(scanner, fn c -> c == ch end)
    count = byte_size(run)

    # Determine flanking
    prev_ch = if pos == 0, do: nil, else: safe_at_before(source, pos)
    next_ch = safe_at_opt(source, s2.pos)

    prev_ws = is_nil(prev_ch) || Scanner.unicode_whitespace?(prev_ch)
    prev_pu = !is_nil(prev_ch) && Scanner.unicode_punctuation?(prev_ch)
    next_ws = is_nil(next_ch) || Scanner.unicode_whitespace?(next_ch)
    next_pu = !is_nil(next_ch) && Scanner.unicode_punctuation?(next_ch)

    left_flank = !next_ws && (!next_pu || prev_ws || prev_pu)
    right_flank = !prev_ws && (!prev_pu || next_ws || next_pu)

    {can_open, can_close} =
      if ch == "*" do
        {left_flank, right_flank}
      else
        # underscore: stricter rules
        o = left_flank && (!right_flank || prev_pu)
        c = right_flank && (!left_flank || next_pu)
        {o, c}
      end

    delim = {:delim, ch, count, can_open, can_close}
    do_scan(s2, [delim | acc], refs)
  end

  # ── Link/Image Openers ────────────────────────────────────────────────────────

  defp scan_bracket_open(scanner, acc, refs) do
    {_, s2} = Scanner.advance(scanner)  # skip [
    # Store the source position right after "[" so we can extract the raw
    # inner text for link label comparison (spec §4.7: no backslash processing).
    do_scan(s2, [{:opener, make_ref(), "[", s2.pos} | acc], refs)
  end

  defp scan_image_open(scanner, acc, refs) do
    s2 = Scanner.skip(scanner, 2)  # skip ![
    do_scan(s2, [{:opener, make_ref(), "![", s2.pos} | acc], refs)
  end

  # ── Link Closer ] ─────────────────────────────────────────────────────────────

  defp scan_bracket_close(scanner, acc, refs, _source) do
    closer_pos = scanner.pos  # byte position of the `]` before advancing
    {_, s2} = Scanner.advance(scanner)  # skip ]

    # CommonMark algorithm: check if the top bracket opener is deactivated (dead).
    # A dead opener is one that was deactivated after a successful nested link was
    # formed (via `deactivate_link_openers`). When the top opener is dead, the `]`
    # cannot form a link through it — emit `]` as literal and convert the dead
    # opener to its text character. This prevents the `![` image opener (or any
    # outer opener) from accidentally consuming the inner `]`.
    #
    # Example: `![[[foo](uri1)](uri2)](uri3)`
    # After `[[foo](uri1)](uri2)` forms a link, the `[` opener (opener_A) is
    # deactivated. The next `]` should NOT find the outer `![` opener; it should
    # see the dead opener_A at the top of the stack and emit `]` as literal.
    case find_top_bracket_opener(acc) do
      {:dead, new_acc} ->
        # Top bracket opener is dead — emit ] as literal without looking for links
        do_scan(s2, append_text(new_acc, "]"), refs)

      :active_or_none ->
        # Find nearest active link opener in acc (reversed)
        case find_link_opener_in(acc) do
          nil ->
            # No opener: emit literal ]
            do_scan(s2, append_text(acc, "]"), refs)

          {_opener_ref, opener_char, opener_src_pos, before_acc, inner_acc} ->
            # inner_acc is in LEFT-TO-RIGHT order (find_opener_loop builds it this way).
            # before_acc is in newest-first (acc) order.
            # Try inline link, full reference, collapsed reference, shortcut reference
            rest_after_close = Scanner.rest(s2)

            # Compute raw label text from source for reference lookup.
            # Per CommonMark spec §4.7, link labels are compared using raw source
            # text (no backslash escaping is applied). The source between the
            # opener's "[" (exclusive) and the closer's "]" (exclusive) is used.
            raw_label_text =
              if opener_src_pos != nil do
                binary_part(scanner.source, opener_src_pos, closer_pos - opener_src_pos)
              else
                items_to_plain_text(inner_acc)
              end

            # For rendering link text and shortcut label comparison, use processed text.
            label_text = items_to_plain_text(inner_acc)

            result = try_resolve_link(rest_after_close, raw_label_text, label_text, opener_char, refs)

            case result do
              {destination, title, consumed} ->
                # Resolve emphasis in inner content (already in left-to-right order)
                inner_items = resolve_emphasis(inner_acc)
                inner_nodes = items_to_nodes(inner_items)

                node =
                  if opener_char == "![" do
                    alt = extract_plain_text(inner_nodes)
                    DocumentAst.image(destination, title, alt)
                  else
                    DocumentAst.link(destination, title, inner_nodes)
                  end

                s3 = Scanner.skip(s2, consumed)
                # Deactivate any link openers in before_acc when it's a link
                before_acc2 =
                  if opener_char == "[" do
                    deactivate_link_openers(before_acc)
                  else
                    before_acc
                  end

                do_scan(s3, [{:node, node} | before_acc2], refs)

              nil ->
                # No valid link — emit [ (or ![ for images) and ] as literal text.
                # We must restore the acc in newest-first format:
                # acc = [inner_newest, ..., inner_oldest, open_text, outer_items...]
                # inner_acc is in left-to-right (oldest-first) order, so reverse it.
                #
                # Important: use {:text, open_text} (not {:opener_dead, open_text}) here.
                # {:opener_dead, ...} is reserved for openers deactivated by a SUCCESSFUL
                # inner link (via `deactivate_link_openers`). The dead-top-opener check in
                # `find_top_bracket_opener` only pops {:opener_dead, ...} items. Failed
                # openers are just text and must NOT be confused with deactivated openers.
                open_text = if opener_char == "![", do: "![", else: "["
                acc2 = Enum.reverse(inner_acc) ++ [{:text, open_text} | before_acc]
                do_scan(s2, append_text(acc2, "]"), refs)
            end
        end
    end
  end

  # Returns {:dead, acc_without_dead_opener} if the top bracket opener is dead,
  # or :active_or_none otherwise.
  defp find_top_bracket_opener(acc) do
    find_top_loop(acc, [])
  end

  defp find_top_loop([], _before), do: :active_or_none
  defp find_top_loop([{:opener_dead, text} | rest], before) do
    # Dead opener found at top of the bracket stack.
    # Convert it to a plain text item (so its "[" character is preserved in
    # the output) and return the modified acc. The caller will then append "]"
    # as literal text — the net effect is that neither bracket forms a link.
    {:dead, Enum.reverse(before) ++ [{:text, text} | rest]}
  end
  defp find_top_loop([{:opener, _, _, _} | _] = acc, before) do
    # Active opener found — not dead
    _ = before
    _ = acc
    :active_or_none
  end
  defp find_top_loop([{:opener, _, _} | _] = acc, before) do
    _ = before
    _ = acc
    :active_or_none
  end
  defp find_top_loop([item | rest], before) do
    # Non-opener item — keep scanning
    find_top_loop(rest, [item | before])
  end

  defp find_link_opener_in(acc) do
    find_opener_loop(acc, [])
  end

  defp find_opener_loop([], _before), do: nil
  defp find_opener_loop([{:opener, ref, char, src_pos} | rest], before) do
    {ref, char, src_pos, rest, before}
  end
  # Fallback for old-style openers without source position (shouldn't occur)
  defp find_opener_loop([{:opener, ref, char} | rest], before) do
    {ref, char, nil, rest, before}
  end
  defp find_opener_loop([{:opener_dead, text} | rest], before) do
    # Preserve the dead opener's text (e.g. "[") so it can be emitted as literal
    # text if the enclosing bracket also fails to form a link.
    find_opener_loop(rest, [{:opener_dead, text} | before])
  end
  defp find_opener_loop([item | rest], before) do
    find_opener_loop(rest, [item | before])
  end

  defp deactivate_link_openers(acc) do
    Enum.map(acc, fn
      {:opener, _ref, "[", _src_pos} -> {:opener_dead, "["}
      {:opener, _ref, ch} when ch == "[" -> {:opener_dead, "["}
      item -> item
    end)
  end

  # ── Link Resolution ───────────────────────────────────────────────────────────
  #
  # raw_label_text: the raw source text between [ and ] (no backslash processing)
  # label_text:     the processed/rendered text (backslash escapes applied)
  #
  # Per CommonMark spec §4.7, link label matching uses the raw source text
  # (specifically: backslash escapes are NOT applied for label comparison).
  # So `[foo\!]` as a shortcut ref looks up "foo\!" not "foo!".

  defp try_resolve_link(rest, raw_label_text, label_text, _opener_char, refs) do
    cond do
      # Inline link: (...)
      String.starts_with?(rest, "(") ->
        case parse_inline_dest(rest) do
          {dest, title, consumed} -> {dest, title, consumed}
          nil ->
            # Inline link failed — try reference forms using raw label
            try_reference_lookup(rest, raw_label_text, label_text, refs)
        end

      # Full reference: [label] or Collapsed reference: []
      String.starts_with?(rest, "[") ->
        case parse_link_label(rest) do
          {label, consumed} ->
            if String.trim(label) == "" do
              # Collapsed reference: [] — use raw inner text as label
              norm = Scanner.normalize_link_label(raw_label_text)
              case Map.get(refs, norm) do
                %{destination: dest, title: title} -> {Scanner.normalize_url(dest), title, consumed}
                nil -> nil
              end
            else
              norm = Scanner.normalize_link_label(label)
              case Map.get(refs, norm) do
                %{destination: dest, title: title} -> {Scanner.normalize_url(dest), title, consumed}
                nil ->
                  # Full ref label not found — do NOT fall through to shortcut.
                  # Per spec, `[foo][bar]` where `[bar]` is not a valid ref
                  # should return nil (making `[foo][bar]` literal), not
                  # try `[foo]` as a shortcut (which could match differently).
                  nil
              end
            end
          nil ->
            # No valid label (e.g. `[` inside) — return nil (treat as literal)
            nil
        end

      true ->
        try_shortcut(rest, raw_label_text, label_text, refs)
    end
  end

  defp try_reference_lookup(rest, raw_label_text, label_text, refs) do
    cond do
      String.starts_with?(rest, "[") ->
        case parse_link_label(rest) do
          {label, consumed} ->
            norm = Scanner.normalize_link_label(label)
            case Map.get(refs, norm) do
              %{destination: dest, title: title} -> {Scanner.normalize_url(dest), title, consumed}
              nil -> try_shortcut(rest, raw_label_text, label_text, refs)
            end
          nil -> try_shortcut(rest, raw_label_text, label_text, refs)
        end
      true ->
        try_shortcut(rest, raw_label_text, label_text, refs)
    end
  end

  defp try_shortcut(rest, raw_label_text, label_text, refs) do
    # Collapsed reference: []
    if String.starts_with?(rest, "[]") do
      norm = Scanner.normalize_link_label(raw_label_text)
      case Map.get(refs, norm) do
        %{destination: dest, title: title} -> {Scanner.normalize_url(dest), title, 2}
        nil -> try_bare_shortcut(raw_label_text, label_text, refs)
      end
    else
      try_bare_shortcut(raw_label_text, label_text, refs)
    end
  end

  defp try_bare_shortcut(raw_label_text, _label_text, refs) do
    norm = Scanner.normalize_link_label(raw_label_text)
    case Map.get(refs, norm) do
      %{destination: dest, title: title} -> {Scanner.normalize_url(dest), title, 0}
      nil -> nil
    end
  end

  # ── Entity References ─────────────────────────────────────────────────────────

  defp scan_entity(scanner, acc, refs) do
    rest = Scanner.rest(scanner)

    cond do
      Regex.match?(~r/^&[a-zA-Z][a-zA-Z0-9]{1,31};/, rest) ->
        [m] = Regex.run(~r/^&[a-zA-Z][a-zA-Z0-9]{1,31};/, rest)
        decoded = Entities.decode_entity(m)
        do_scan(Scanner.skip(scanner, byte_size(m)), append_text(acc, decoded), refs)

      Regex.match?(~r/^&#[0-9]{1,7};/, rest) ->
        [m] = Regex.run(~r/^&#[0-9]{1,7};/, rest)
        decoded = Entities.decode_entity(m)
        do_scan(Scanner.skip(scanner, byte_size(m)), append_text(acc, decoded), refs)

      Regex.match?(~r/^&#[xX][0-9a-fA-F]{1,6};/, rest) ->
        [m] = Regex.run(~r/^&#[xX][0-9a-fA-F]{1,6};/, rest)
        decoded = Entities.decode_entity(m)
        do_scan(Scanner.skip(scanner, byte_size(m)), append_text(acc, decoded), refs)

      true ->
        do_scan(Scanner.skip(scanner, 1), append_text(acc, "&"), refs)
    end
  end

  # ── Emphasis Resolution (CommonMark Appendix A) ────────────────────────────────

  # Items is a list in LEFT-TO-RIGHT order.
  # We implement the CommonMark algorithm:
  # - scan right to left for closers
  # - for each closer scan back left for an opener
  # - if found, wrap content in emphasis/strong node

  defp resolve_emphasis(items) do
    # Convert to array form for indexed access
    arr = List.to_tuple(items)
    n = tuple_size(arr)

    # Process emphasis iteratively
    arr = resolve_emphasis_pass(arr, n)

    # Convert back to list
    Tuple.to_list(arr)
  end

  defp resolve_emphasis_pass(arr, n) do
    # Find all closers and try to match them with openers
    # We scan from left to right, processing each closer
    resolve_closers(arr, n, 0)
  end

  defp resolve_closers(arr, n, pos) when pos >= n, do: arr
  defp resolve_closers(arr, n, pos) do
    item = elem(arr, pos)

    case item do
      {:delim, char, count, can_open_closer, can_close}
      when can_close and count > 0 and (char == "*" or char == "_") ->
        case find_opener(arr, pos - 1, char, count, can_open_closer) do
          nil ->
            resolve_closers(arr, n, pos + 1)

          opener_pos ->
            {:delim, ^char, opener_count, _co, _cc} = elem(arr, opener_pos)
            use_count = determine_use_count(opener_count, count)

            # Create emphasis node
            inner = collect_items_between(arr, opener_pos, pos)
            inner_resolved = resolve_emphasis(inner)
            inner_nodes = items_to_nodes(inner_resolved)

            node =
              if use_count >= 2 do
                DocumentAst.strong(inner_nodes)
              else
                DocumentAst.emphasis(inner_nodes)
              end

            # Update opener count
            new_opener_count = opener_count - use_count
            new_closer_count = count - use_count

            opener_new =
              if new_opener_count == 0 do
                {:resolved}
              else
                {:delim, char, new_opener_count, true, false}
              end

            closer_new =
              if new_closer_count == 0 do
                {:resolved}
              else
                {:delim, char, new_closer_count, false, true}
              end

            # Build new array replacing the range [opener_pos..pos] with the node
            arr2 = replace_range(arr, n, opener_pos, pos, opener_new, node, closer_new)
            new_n = tuple_size(arr2)

            # Find the position of the node in the new array to continue from there
            # The node is at opener_pos + (1 if opener_new != {:resolved} else 0)
            node_pos = if new_opener_count == 0, do: opener_pos, else: opener_pos + 1

            resolve_closers(arr2, new_n, node_pos + 1)
        end

      _ ->
        resolve_closers(arr, n, pos + 1)
    end
  end

  defp find_opener(_arr, pos, _char, _closer_count, _closer_can_open) when pos < 0, do: nil
  defp find_opener(arr, pos, char, closer_count, closer_can_open) do
    item = elem(arr, pos)
    case item do
      {:delim, ^char, count, can_open, can_close} when can_open and count > 0 ->
        # Rule of 3 (CommonMark spec rule 14): When the sum of opener and closer
        # run lengths is a multiple of 3, and NEITHER is individually a multiple
        # of 3, the match is forbidden.
        #
        # The catch: the rule uses the RUN lengths if the delimiter is ambiguous
        # (can both open and close). If either the opener OR the closer is
        # ambiguous, we use the full run lengths. Otherwise (both are unambiguous),
        # we use the actual USE counts (which are always 1 or 2 per match).
        #
        # Why this matters:
        #   `*foo**bar*`: inner `**` has can_close=true (also a potential closer).
        #     Outer `*` closer checks `**` opener: full counts = 2+1=3 → blocked.
        #     Falls through to `*` opener. sum=1+1=2 → passes. `<em>foo**bar</em>`.
        #   `**foo*`: `**` opener has can_close=false. `*` closer has can_open=false.
        #     Both unambiguous → use use_count=1 for both: sum=2 → passes. `*<em>foo</em>`.
        #   `*foo *bar**`: inner `*` (can_close=false) vs `**` closer (can_open=false):
        #     Both unambiguous → use_count=1: sum=2 → passes. Nested em formed.
        #   `*foo**bar*`: `**` is ambiguous (can_close=true) and `*` closer is
        #     NOT ambiguous (can_open=false). Opener `**` triggers full-count check:
        #     2+1=3 → blocked. Correct.
        use_count = min(count, min(closer_count, 2))
        ambiguous = can_close || closer_can_open
        {check_opener, check_closer} =
          if ambiguous do
            {count, closer_count}
          else
            {use_count, use_count}
          end

        if passes_rule_of_3(check_opener, check_closer) do
          pos
        else
          find_opener(arr, pos - 1, char, closer_count, closer_can_open)
        end

      # Skip over link openers (don't cross them for emphasis)
      {:opener, _, "[" <> _} ->
        find_opener(arr, pos - 1, char, closer_count, closer_can_open)

      _ ->
        find_opener(arr, pos - 1, char, closer_count, closer_can_open)
    end
  end

  defp passes_rule_of_3(opener_count, closer_count) do
    sum = opener_count + closer_count
    rem(sum, 3) != 0 || rem(opener_count, 3) == 0 || rem(closer_count, 3) == 0
  end

  defp determine_use_count(opener_count, closer_count) do
    # We can only use 2 (form `<strong>`) if both sides have at least 2 delimiters.
    # The rule of 3 applies to the USED counts (always 1 or 2 per match), not the
    # original run lengths. Using 2 from each side: sum=4, which is never divisible
    # by 3, so the rule of 3 never blocks double-use. Using 1 from each side: sum=2,
    # also never divisible by 3. Therefore, the rule of 3 does NOT restrict which
    # emphasis type (em vs strong) we form — it only restricts whether an opener can
    # be found at all (handled in `find_opener`). We simply pick 2 when possible.
    if opener_count >= 2 && closer_count >= 2 do
      2
    else
      1
    end
  end

  defp collect_items_between(arr, from_pos, to_pos) do
    Enum.map((from_pos + 1)..(to_pos - 1)//1, fn i -> elem(arr, i) end)
  end

  defp replace_range(arr, n, opener_pos, closer_pos, opener_new, node, closer_new) do
    before = Enum.map(0..(opener_pos - 1)//1, fn i -> elem(arr, i) end)
    after_list = Enum.map((closer_pos + 1)..(n - 1)//1, fn i -> elem(arr, i) end)

    middle =
      []
      |> maybe_add(opener_new)
      |> (fn l -> l ++ [{:node, node}] end).()
      |> (fn l -> l ++ maybe_list(closer_new) end).()

    List.to_tuple(before ++ middle ++ after_list)
  end

  defp maybe_add(list, {:resolved}), do: list
  defp maybe_add(list, item), do: list ++ [item]

  defp maybe_list({:resolved}), do: []
  defp maybe_list(item), do: [item]

  # ── Convert Items to Nodes ────────────────────────────────────────────────────

  defp items_to_nodes(items) do
    items
    |> Enum.flat_map(&item_to_nodes/1)
    |> merge_text_nodes()
    |> Enum.filter(fn
      %{type: :text, value: ""} -> false
      _ -> true
    end)
  end

  defp item_to_nodes({:text, t}) do
    # Entities are already decoded at scan time by `scan_entity/3`. Do NOT call
    # `decode_entities` here — it would decode entities that arose from backslash
    # escapes (e.g. `\&ouml;` → `&ouml;` via backslash escape → incorrectly `ö`).
    if t == "", do: [], else: [DocumentAst.text(t)]
  end

  defp item_to_nodes({:node, n}), do: [n]
  defp item_to_nodes({:break, :hard}), do: [DocumentAst.hard_break()]
  defp item_to_nodes({:break, :soft}), do: [DocumentAst.soft_break()]

  defp item_to_nodes({:delim, ch, count, _co, _cc}) do
    t = if ch == "![", do: "![", else: String.duplicate(ch, count)
    if t == "", do: [], else: [DocumentAst.text(t)]
  end

  defp item_to_nodes({:opener, _, ch}) do
    literal = if ch == "![", do: "![", else: "["
    [DocumentAst.text(literal)]
  end

  defp item_to_nodes({:opener, _, ch, _src_pos}) do
    literal = if ch == "![", do: "![", else: "["
    [DocumentAst.text(literal)]
  end

  defp item_to_nodes({:opener_dead, text}) when text != "" and text != nil, do: [DocumentAst.text(text)]
  defp item_to_nodes({:opener_dead, _}), do: []
  defp item_to_nodes({:opener_dead}), do: []
  defp item_to_nodes({:resolved}), do: []
  defp item_to_nodes(_), do: []

  defp merge_text_nodes(nodes) do
    Enum.reduce(nodes, [], fn node, acc ->
      case {node, acc} do
        {%{type: :text, value: v2}, [%{type: :text, value: v1} | rest]} ->
          [DocumentAst.text(v1 <> v2) | rest]
        _ -> [node | acc]
      end
    end)
    |> Enum.reverse()
  end

  # ── Link Destination / Label Parsers ─────────────────────────────────────────

  defp parse_inline_dest(text) do
    # text starts with "("
    pos = 1  # skip (
    pos = skip_ws_chars(text, pos)

    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)
      {dest, dest_type, pos2} =
        if ch == "<" do
          case scan_angle_dest(text, pos + 1) do
            # angle dest: backslash already unescaped inside do_angle_dest
            {d, p} -> {d, :angle, p}
            nil -> {nil, :angle, pos}
          end
        else
          {d, p} = scan_bare_dest(text, pos, 0)
          # bare dest: backslash NOT yet unescaped — must apply unescape + entity decode
          {d, :bare, p}
        end

      if dest == nil do
        nil
      else
        pos3 = skip_ws_chars(text, pos2)
        parse_dest_tail(text, dest, dest_type, pos3)
      end
    end
  end

  # Normalize a destination URL: apply appropriate unescaping then percent-encode.
  # Angle destinations have backslash escapes already applied by `do_angle_dest`.
  # Bare destinations still have raw `\x` sequences that must be unescaped first.
  # HTML entities in URLs are decoded to Unicode and then percent-encoded.
  defp normalize_dest(dest, :bare) do
    dest
    |> apply_backslash_escapes()
    |> Entities.decode_entities()
    |> Scanner.normalize_url()
  end

  defp normalize_dest(dest, :angle) do
    dest
    |> Entities.decode_entities()
    |> Scanner.normalize_url()
  end

  defp parse_dest_tail(text, dest, dest_type, pos) do
    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)
      cond do
        ch == ")" ->
          {normalize_dest(dest, dest_type), nil, pos + 1}

        ch in ["\"", "'", "("] ->
          close = if ch == "(", do: ")", else: ch
          case scan_title_str(text, pos + 1, close) do
            {title_raw, end_pos} ->
              title = Entities.decode_entities(apply_backslash_escapes(title_raw))
              pos2 = skip_ws_chars(text, end_pos)
              if pos2 < byte_size(text) && safe_at(text, pos2) == ")" do
                {normalize_dest(dest, dest_type), title, pos2 + 1}
              else
                nil
              end
            nil -> nil
          end

        true -> nil
      end
    end
  end

  defp scan_angle_dest(text, pos) do
    if pos >= byte_size(text), do: nil,
    else: do_angle_dest(text, pos, "")
  end

  defp do_angle_dest(text, pos, acc) do
    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)
      cond do
        ch == ">" -> {acc, pos + 1}
        ch == "<" or ch == "\n" -> nil
        ch == "\\" and pos + 1 < byte_size(text) ->
          next = safe_at(text, pos + 1)
          do_angle_dest(text, pos + 2, acc <> next)
        true ->
          do_angle_dest(text, pos + byte_size(ch), acc <> ch)
      end
    end
  end

  defp scan_bare_dest(text, pos, depth) do
    if pos >= byte_size(text) do
      {binary_part(text, 1, pos - 1), pos}
    else
      ch = safe_at(text, pos)
      cond do
        ch == "(" -> scan_bare_dest(text, pos + 1, depth + 1)
        ch == ")" && depth == 0 ->
          start = find_dest_start(text)
          {binary_part(text, start, pos - start), pos}
        ch == ")" -> scan_bare_dest(text, pos + 1, depth - 1)
        ch == "\\" and pos + 1 < byte_size(text) -> scan_bare_dest(text, pos + 2, depth)
        ch in [" ", "\t", "\n", "\r", "\f"] ->
          start = find_dest_start(text)
          {binary_part(text, start, pos - start), pos}
        true -> scan_bare_dest(text, pos + byte_size(ch), depth)
      end
    end
  end

  defp find_dest_start(text) do
    # Find the position after optional whitespace from pos 1
    skip_ws_chars(text, 1)
  end

  defp scan_title_str(text, pos, close) do
    scan_title_loop(text, pos, pos, close)
  end

  defp scan_title_loop(text, pos, start, close) do
    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)
      cond do
        ch == "\\" and pos + 1 < byte_size(text) ->
          scan_title_loop(text, pos + 2, start, close)
        ch == "\n" and close == ")" -> nil
        ch == close ->
          {binary_part(text, start, pos - start), pos + 1}
        true ->
          scan_title_loop(text, pos + byte_size(ch), start, close)
      end
    end
  end

  defp parse_link_label(text) do
    if byte_size(text) < 2 || binary_part(text, 0, 1) != "[" do
      nil
    else
      scan_label_loop(text, 1)
    end
  end

  defp scan_label_loop(text, pos) do
    if pos >= byte_size(text) do
      nil
    else
      ch = safe_at(text, pos)
      cond do
        ch == "\\" and pos + 1 < byte_size(text) -> scan_label_loop(text, pos + 2)
        ch == "[" -> nil  # no nested brackets
        ch == "]" ->
          label = binary_part(text, 1, pos - 1)
          if byte_size(label) > 999, do: nil, else: {label, pos + 1}
        true -> scan_label_loop(text, pos + byte_size(ch))
      end
    end
  end

  defp skip_ws_chars(text, pos) do
    if pos < byte_size(text) do
      ch = safe_at(text, pos)
      if ch in [" ", "\t", "\n", "\r"] do
        skip_ws_chars(text, pos + 1)
      else
        pos
      end
    else
      pos
    end
  end

  # ── Backslash Escape Application (for title/dest) ─────────────────────────────

  @doc false
  def apply_backslash_escapes(text) do
    apply_bs_loop(text, 0, "")
  end

  defp apply_bs_loop(text, pos, acc) do
    if pos >= byte_size(text) do
      acc
    else
      ch = safe_at(text, pos)
      if ch == "\\" && pos + 1 < byte_size(text) do
        next = safe_at(text, pos + 1)
        if Scanner.ascii_punctuation?(next) do
          apply_bs_loop(text, pos + 2, acc <> next)
        else
          apply_bs_loop(text, pos + 1, acc <> ch)
        end
      else
        apply_bs_loop(text, pos + byte_size(ch), acc <> ch)
      end
    end
  end

  # ── Text Helpers ──────────────────────────────────────────────────────────────

  defp append_text([], t), do: [{:text, t}]
  defp append_text([{:text, prev} | rest], t), do: [{:text, prev <> t} | rest]
  defp append_text(acc, t), do: [{:text, t} | acc]

  defp last_text([{:text, t} | _]), do: t
  defp last_text(_), do: ""

  defp replace_last_text([{:text, _} | rest], ""), do: rest
  defp replace_last_text([{:text, _} | rest], t), do: [{:text, t} | rest]
  defp replace_last_text(acc, _), do: acc

  defp items_to_plain_text(items) do
    Enum.map_join(items, "", fn
      {:text, t} -> t
      {:node, n} -> extract_plain_text([n])
      {:delim, ch, count, _, _} -> String.duplicate(ch, count)
      {:opener, _, ch} -> ch
      {:opener, _, ch, _} -> ch
      {:opener_dead, t} -> t
      {:break, _} -> " "
      _ -> ""
    end)
  end

  defp extract_plain_text(nodes) do
    Enum.map_join(nodes, "", fn
      %{type: :text, value: v} -> v
      %{type: :emphasis, children: c} -> extract_plain_text(c)
      %{type: :strong, children: c} -> extract_plain_text(c)
      %{type: :code_span, value: v} -> v
      %{type: :link, children: c} -> extract_plain_text(c)
      %{type: :image, alt: a} -> a
      %{type: :soft_break} -> "\n"
      %{type: :hard_break} -> "\n"
      _ -> ""
    end)
  end

  # ── Character Helpers ─────────────────────────────────────────────────────────

  defp safe_at(text, pos) do
    if pos >= byte_size(text) do
      ""
    else
      case String.next_codepoint(binary_part(text, pos, byte_size(text) - pos)) do
        {ch, _} -> ch
        nil -> ""
      end
    end
  end

  defp safe_at_opt(text, pos) do
    if pos >= byte_size(text) do
      nil
    else
      case String.next_codepoint(binary_part(text, pos, byte_size(text) - pos)) do
        {ch, _} -> ch
        nil -> nil
      end
    end
  end

  defp safe_at_before(text, pos) do
    # Get the codepoint immediately before byte position `pos`
    # We need to scan backwards — for ASCII this is pos-1, for multibyte we need to search
    if pos <= 0 do
      nil
    else
      # Scan backwards to find the start of the codepoint
      find_codepoint_before(text, pos - 1)
    end
  end

  defp find_codepoint_before(_text, pos) when pos < 0, do: nil
  defp find_codepoint_before(text, pos) do
    byte = :binary.at(text, pos)
    if Bitwise.band(byte, 0b11000000) == 0b10000000 do
      # Continuation byte, go further back
      find_codepoint_before(text, pos - 1)
    else
      case String.next_codepoint(binary_part(text, pos, byte_size(text) - pos)) do
        {ch, _} -> ch
        nil -> nil
      end
    end
  end
end
