defmodule CsvParser do
  @moduledoc """
  CsvParser — A from-scratch, state-machine CSV parser.

  ## What is CSV?

  CSV (Comma-Separated Values) is the most common data interchange format
  in the world. It's how spreadsheets export data, how databases dump tables,
  and how scientists share datasets. Yet it has no single standard — RFC 4180
  (2005) is the closest thing, but real-world CSV files deviate from it
  constantly.

  This parser implements a pragmatic dialect that handles the most common
  real-world cases:

    - First row is the header (defines column names for all following rows)
    - All values returned as **strings** — no type coercion
    - Quoted fields can contain commas, newlines, and `""` (escaped double-quote)
    - Configurable delimiter (default: comma, but tab / semicolon also work)
    - Ragged rows: short rows are padded with `""`, long rows are truncated
    - Unclosed quoted field → `{:error, reason}`

  ## Why CSV Cannot Be Tokenized with a Simple Regex

  Consider this input:

      field1,"field,with,commas",field3

  The commas inside the quoted field are **not** delimiters — but you can
  only know that *after* you've entered quoted mode. This is context-sensitive:
  the meaning of `,` depends on whether the parser is currently inside a
  quoted field.

  Because of this, CSV parsers are traditionally implemented as hand-rolled
  **character-by-character state machines**, not grammar-driven lexers.
  Think of it like reading aloud: when you encounter `"`, you switch into
  "quoted mode" and must treat everything differently until the closing `"`.

  ## State Machine

  The parser uses exactly four states:

      ┌─────────────────────────────────────────────────────────────────────┐
      │                         STATE MACHINE                               │
      │                                                                     │
      │                ┌──────────────┐                                     │
      │                │  FIELD_START │◄──────────────────────────────┐    │
      │                └──────┬───────┘                               │    │
      │                       │                                       │    │
      │          ┌────────────┼────────────────┐                      │    │
      │          │            │                │                      │    │
      │         '"'        other char      COMMA or NEWLINE           │    │
      │          │            │                │                      │    │
      │          ▼            ▼                │                      │    │
      │   ┌────────────┐  ┌────────────┐       │                      │    │
      │   │  IN_QUOTED  │  │ IN_UNQUOTED│       │ emit empty field     │    │
      │   │   _FIELD    │  │   _FIELD   │       └──────────────────────┘    │
      │   └──────┬──────┘  └─────┬──────┘                                   │
      │          │               │                                          │
      │    only '"' is       COMMA → end field ─────────────────────────────┘
      │      special         NEWLINE → end row                              │
      │          │           EOF → end file                                 │
      │          ▼                                                          │
      │  ┌──────────────────┐                                              │
      │  │ IN_QUOTED_MAYBE_ │                                              │
      │  │      END         │                                              │
      │  └──────┬───────────┘                                              │
      │         │                                                          │
      │    ┌────┴────┐                                                     │
      │   '"'    COMMA/NEWLINE/EOF                                         │
      │    │          │                                                    │
      │ escaped    end field ──────────────────────────────────────────────┘
      │  quote
      │ append '"'
      │ back to IN_QUOTED_FIELD
      └─────────────────────────────────────────────────────────────────────┘

  ### State Descriptions

  1. **FIELD_START** — We are at the start of a new field. Peek at the
     first character to decide which path to take:
       - `"` → enter `IN_QUOTED_FIELD` (don't add `"` to buffer)
       - delimiter → emit empty field `""`, stay in `FIELD_START`
       - newline/EOF → end the current row (or file)
       - other char → start `IN_UNQUOTED_FIELD`, append char to buffer

  2. **IN_UNQUOTED_FIELD** — Collect characters for a plain field. Stop on:
       - delimiter → end field, start next
       - newline → end field, end row
       - EOF → end field, end file

  3. **IN_QUOTED_FIELD** — Inside a `"..."` field. Almost everything is
     literal. Only `"` is special — it transitions to `IN_QUOTED_MAYBE_END`.

  4. **IN_QUOTED_MAYBE_END** — Just saw `"` inside a quoted field. The
     *next* character determines what happened:
       - `"` → escaped quote; append one `"` to buffer, return to
             `IN_QUOTED_FIELD`
       - anything else → the field is closed; process the next char
             from `FIELD_START`

  ## Truth Table: IN_QUOTED_MAYBE_END

      Previous | Next char      | Interpretation          | Action
      char     |                |                         |
      ─────────┼────────────────┼─────────────────────────┼─────────────────────
         "     | "              | escaped quote ("")      | emit '"', stay quoted
         "     | ,  (delimiter) | end of quoted field     | emit field, next field
         "     | \\n or \\r       | end of quoted field     | emit field, next row
         "     | EOF            | end of quoted field     | emit field, end file
         "     | other          | malformed (tolerant)    | end field, re-process

  ## Public API

      # Parse with default comma delimiter
      {:ok, rows} = CsvParser.parse_csv("name,age\\nAlice,30\\n")

      # Parse with custom delimiter (tab-separated values)
      {:ok, rows} = CsvParser.parse_csv("name\\tage\\nAlice\\t30", "\\t")

      # Error on unclosed quote
      {:error, reason} = CsvParser.parse_csv(~s(id,value\\n1,"unclosed))

  ## Return Format

  Returns `{:ok, rows}` where `rows` is a list of maps:

      [
        %{"name" => "Alice", "age" => "30"},
        %{"name" => "Bob",   "age" => "25"}
      ]

  The header row is consumed and used as map keys; it does not appear
  in the returned list. An empty file or header-only file returns
  `{:ok, []}`.
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Parse CSV text with the default comma delimiter.

  Returns `{:ok, rows}` on success, `{:error, reason}` on malformed input.

  ## Examples

      iex> CsvParser.parse_csv("name,age\\nAlice,30")
      {:ok, [%{"name" => "Alice", "age" => "30"}]}

      iex> CsvParser.parse_csv("")
      {:ok, []}

      iex> CsvParser.parse_csv("id\\n1")
      {:ok, [%{"id" => "1"}]}
  """
  @spec parse_csv(String.t()) :: {:ok, [map()]} | {:error, String.t()}
  def parse_csv(source) do
    parse_csv(source, ",")
  end

  @doc """
  Parse CSV text with a custom single-character delimiter.

  The delimiter must be a single character string.

  Common alternatives:
  - `"\\t"` — tab (TSV / tab-separated values)
  - `";"` — semicolon (European locale CSV)
  - `"|"` — pipe-separated

  ## Examples

      iex> CsvParser.parse_csv("name\\tage\\nAlice\\t30", "\\t")
      {:ok, [%{"name" => "Alice", "age" => "30"}]}

      iex> CsvParser.parse_csv("a;b\\n1;2", ";")
      {:ok, [%{"a" => "1", "b" => "2"}]}
  """
  @spec parse_csv(String.t(), String.t()) :: {:ok, [map()]} | {:error, String.t()}
  def parse_csv(source, delimiter) when is_binary(source) and is_binary(delimiter) do
    # Extract the delimiter as a single Unicode codepoint (integer).
    # We work character-by-character as a list of codepoints because:
    # 1. Elixir pattern matching on lists is extremely efficient
    # 2. String.to_charlist/1 gives us Unicode-safe character iteration
    # 3. Integer comparison is fast (no string allocations per character)
    delim_char =
      case String.to_charlist(delimiter) do
        [c] ->
          c

        other ->
          raise ArgumentError,
                "delimiter must be exactly one character, got #{inspect(other)}"
      end

    chars = String.to_charlist(source)

    # Kick off the state machine.
    # Initial state: FIELD_START, empty buffers, no rows yet.
    case run(chars, :field_start, [], [], [], delim_char) do
      {:ok, header, data_rows} ->
        rows = build_row_maps(header, data_rows)
        {:ok, rows}

      {:error, _} = err ->
        err
    end
  end

  # ---------------------------------------------------------------------------
  # State machine — run/6
  # ---------------------------------------------------------------------------
  #
  # run(chars, state, field_buf, current_row, rows, delim)
  #
  # chars       — remaining input as a list of codepoints (integers)
  # state       — one of: :field_start | :in_unquoted | :in_quoted |
  #                        :in_quoted_maybe_end
  # field_buf   — characters for the current field, accumulated in REVERSE
  #               order (prepending is O(1); we reverse at field-end)
  # current_row — list of completed field strings for the current row,
  #               in forward order
  # rows        — completed rows accumulated in REVERSE order (reversed at end)
  # delim       — the delimiter codepoint as an integer
  #
  # Returns:
  #   {:ok, header_fields, data_rows}  — success
  #   {:error, reason_string}          — malformed input

  # ─── FIELD_START ─────────────────────────────────────────────────────────

  # EOF at field_start with an in-progress row:
  # The file ended without a trailing newline. Flush current_row.
  # But if current_row is empty, we're between rows — don't emit a spurious
  # empty row (handles files ending with "\n").
  defp run([], :field_start, _buf, current_row, rows, _delim) do
    finalize(rows, current_row)
  end

  # '"' at field_start: enter quoted mode. Don't add the quote to the buffer —
  # the buffer holds field *content*, not the surrounding delimiters.
  defp run([?" | rest], :field_start, buf, row, rows, delim) do
    run(rest, :in_quoted, buf, row, rows, delim)
  end

  # Delimiter at field_start: empty unquoted field.
  # Example: input `a,,b` — when we see the second `,`, we're in FIELD_START
  # with an empty buffer. Emit "" for the empty field and continue.
  defp run([c | rest], :field_start, buf, row, rows, delim) when c == delim do
    new_row = row ++ [buf_to_string(buf)]
    run(rest, :field_start, [], new_row, rows, delim)
  end

  # \r\n at field_start: Windows line ending. Treat as a single newline.
  # We strip the \r here and let the \n case handle the row transition.
  defp run([?\r, ?\n | rest], :field_start, buf, row, rows, delim) do
    run([?\n | rest], :field_start, buf, row, rows, delim)
  end

  # \r alone at field_start: old Mac line ending — treat as newline.
  defp run([?\r | rest], :field_start, buf, row, rows, delim) do
    run([?\n | rest], :field_start, buf, row, rows, delim)
  end

  # \n at field_start with a non-empty row:
  # The last field before this newline was empty (delimiter was immediately
  # followed by newline). Emit the empty field and complete the row.
  defp run([?\n | rest], :field_start, _buf, row, rows, delim) when row != [] do
    completed_row = row ++ [""]
    run(rest, :field_start, [], [], [completed_row | rows], delim)
  end

  # \n at field_start with an empty row:
  # Blank line (or leading newline). Skip it — don't emit an empty row.
  defp run([?\n | rest], :field_start, _buf, [], rows, delim) do
    run(rest, :field_start, [], [], rows, delim)
  end

  # Any other character at field_start: start of an unquoted field.
  defp run([c | rest], :field_start, buf, row, rows, delim) do
    run(rest, :in_unquoted, [c | buf], row, rows, delim)
  end

  # ─── IN_UNQUOTED_FIELD ───────────────────────────────────────────────────

  # EOF in unquoted field: flush the field and the row, then finalize.
  defp run([], :in_unquoted, buf, row, rows, _delim) do
    completed_row = row ++ [buf_to_string(buf)]
    finalize(rows, completed_row)
  end

  # Delimiter in unquoted field: end of this field.
  defp run([c | rest], :in_unquoted, buf, row, rows, delim) when c == delim do
    new_row = row ++ [buf_to_string(buf)]
    run(rest, :field_start, [], new_row, rows, delim)
  end

  # \r\n in unquoted field: Windows newline — end of row.
  defp run([?\r, ?\n | rest], :in_unquoted, buf, row, rows, delim) do
    run([?\n | rest], :in_unquoted, buf, row, rows, delim)
  end

  # \r alone in unquoted field: Mac newline — end of row.
  defp run([?\r | rest], :in_unquoted, buf, row, rows, delim) do
    run([?\n | rest], :in_unquoted, buf, row, rows, delim)
  end

  # \n in unquoted field: end of row.
  defp run([?\n | rest], :in_unquoted, buf, row, rows, delim) do
    completed_row = row ++ [buf_to_string(buf)]
    run(rest, :field_start, [], [], [completed_row | rows], delim)
  end

  # Any other character: accumulate into field buffer (reversed).
  # We prepend (O(1)) rather than append (O(n)) — reversed at field-end.
  defp run([c | rest], :in_unquoted, buf, row, rows, delim) do
    run(rest, :in_unquoted, [c | buf], row, rows, delim)
  end

  # ─── IN_QUOTED_FIELD ─────────────────────────────────────────────────────

  # EOF inside a quoted field: error — unclosed quote.
  # RFC 4180 requires every quoted field to have a matching closing quote.
  defp run([], :in_quoted, _buf, _row, _rows, _delim) do
    {:error, "Unclosed quoted field at end of input"}
  end

  # '"' inside a quoted field: might be end-of-quote or "" escape.
  # Transition to IN_QUOTED_MAYBE_END and look at the next character.
  defp run([?" | rest], :in_quoted, buf, row, rows, delim) do
    run(rest, :in_quoted_maybe_end, buf, row, rows, delim)
  end

  # Any other character inside a quoted field: literal. Append to buffer.
  # This includes commas, newlines, backslashes — everything is literal
  # inside a quoted field except '"'.
  defp run([c | rest], :in_quoted, buf, row, rows, delim) do
    run(rest, :in_quoted, [c | buf], row, rows, delim)
  end

  # ─── IN_QUOTED_MAYBE_END ─────────────────────────────────────────────────

  # Another '"': this is a "" escape. Append one literal '"' to the buffer
  # and return to IN_QUOTED_FIELD.
  #
  # Walkthrough of `"say ""hello"""`:
  #   s a y ' ' " "  h  e  l  l  o  "  "  "
  #   Q Q Q Q  M Q  Q  Q  Q  Q  Q  M  Q  M  ← states (Q=in_quoted, M=maybe_end)
  #
  # First M (after first pair of ""): next='"' → emit '"', back to Q
  # Second M (after `o`): next='"' → emit '"', back to Q
  # Third M (final `"`): next=EOF → end field with content `say "hello"`
  defp run([?" | rest], :in_quoted_maybe_end, buf, row, rows, delim) do
    run(rest, :in_quoted, [?" | buf], row, rows, delim)
  end

  # EOF after a closing '"': the field just ended cleanly.
  defp run([], :in_quoted_maybe_end, buf, row, rows, _delim) do
    completed_row = row ++ [buf_to_string(buf)]
    finalize(rows, completed_row)
  end

  # Delimiter after a closing '"': end of quoted field, next field starts.
  defp run([c | rest], :in_quoted_maybe_end, buf, row, rows, delim) when c == delim do
    new_row = row ++ [buf_to_string(buf)]
    run(rest, :field_start, [], new_row, rows, delim)
  end

  # \r\n after a closing '"': end of row.
  defp run([?\r, ?\n | rest], :in_quoted_maybe_end, buf, row, rows, delim) do
    run([?\n | rest], :in_quoted_maybe_end, buf, row, rows, delim)
  end

  # \r alone after a closing '"': end of row.
  defp run([?\r | rest], :in_quoted_maybe_end, buf, row, rows, delim) do
    run([?\n | rest], :in_quoted_maybe_end, buf, row, rows, delim)
  end

  # \n after a closing '"': end of row.
  defp run([?\n | rest], :in_quoted_maybe_end, buf, row, rows, delim) do
    completed_row = row ++ [buf_to_string(buf)]
    run(rest, :field_start, [], [], [completed_row | rows], delim)
  end

  # Any other char after '"': malformed (RFC 4180 says this is invalid).
  # We tolerate it: end the quoted field and re-process the current char
  # from FIELD_START. Real CSV producers occasionally generate `"value"x`
  # due to bugs; strict rejection would make the parser too fragile.
  defp run([c | rest], :in_quoted_maybe_end, buf, row, rows, delim) do
    new_row = row ++ [buf_to_string(buf)]
    run([c | rest], :field_start, [], new_row, rows, delim)
  end

  # ---------------------------------------------------------------------------
  # Helper: buf_to_string/1
  # ---------------------------------------------------------------------------
  #
  # Convert a reversed codepoint accumulator list into a UTF-8 string.
  #
  # Why reversed? We accumulate characters by *prepending* to a list:
  #
  #   Input:   h  e  l  l  o
  #   Buffer:  [h] → [e,h] → [l,e,h] → [l,l,e,h] → [o,l,l,e,h]
  #
  # Prepending is O(1); appending to a list is O(n). Since we append one
  # character at a time, prepending gives O(n) total work rather than O(n²).
  #
  # We pay a single O(n) reversal and List.to_string/1 conversion at the
  # end of each field — still O(n) overall.
  defp buf_to_string(buf) do
    buf
    |> Enum.reverse()
    |> List.to_string()
  end

  # ---------------------------------------------------------------------------
  # Helper: finalize/2
  # ---------------------------------------------------------------------------
  #
  # Called at EOF. Combines all accumulated rows (in reverse order) with
  # the in-progress final row (if any), reverses to chronological order,
  # and splits into header + data rows.
  #
  # The `rows` list is in reverse order because we prepend completed rows:
  #   After row 1: rows = [[h1,h2,h3]]
  #   After row 2: rows = [[d1,d2,d3], [h1,h2,h3]]
  #   After row 3: rows = [[d2a,...], [d1,...], [h1,...]]
  #
  # After Enum.reverse: [[h1,...], [d1,...], [d2a,...]]
  # First element is header; rest are data rows.
  #
  # Edge cases handled:
  #   - Empty file (no rows at all) → {:ok, [], []}
  #   - Header-only file             → {:ok, header, []}
  #   - File without trailing newline → current_row contains the last row

  defp finalize(rows, current_row) do
    # Combine completed rows with the in-progress last row, reverse to get
    # chronological order (oldest row first).
    all_rows =
      if current_row == [] do
        Enum.reverse(rows)
      else
        Enum.reverse([current_row | rows])
      end

    case all_rows do
      [] ->
        # Empty file: no rows at all.
        {:ok, [], []}

      [header | data_rows] ->
        # First row is the header; the rest are data rows.
        {:ok, header, data_rows}
    end
  end

  # ---------------------------------------------------------------------------
  # Helper: build_row_maps/2
  # ---------------------------------------------------------------------------
  #
  # Convert a list of data rows (each a list of field strings) into a list
  # of maps using the header strings as keys.
  #
  # Handles ragged rows:
  #
  #   Short row: fewer fields than the header → pad with ""
  #
  #     header: ["a", "b", "c"]
  #     row:    ["1", "2"]         ← missing "c"
  #     result: %{"a"=>"1","b"=>"2","c"=>""}
  #
  #   Long row: more fields than the header → truncate
  #
  #     header: ["a", "b"]
  #     row:    ["1", "2", "3"]    ← extra "3" discarded
  #     result: %{"a"=>"1","b"=>"2"}
  #
  # Why not error on ragged rows? Many real-world CSV generators are buggy
  # and produce inconsistent column counts. The spec says: use the header as
  # the authoritative column list; pad or truncate to match.

  defp build_row_maps(_header, []), do: []

  defp build_row_maps(header, data_rows) do
    header_len = length(header)

    Enum.map(data_rows, fn row ->
      normalized = normalize_row(row, header_len)

      header
      |> Enum.zip(normalized)
      |> Enum.into(%{})
    end)
  end

  # Normalize a row to exactly `target_len` fields.
  defp normalize_row(row, target_len) do
    row_len = length(row)

    cond do
      row_len == target_len ->
        row

      row_len < target_len ->
        # Pad with empty strings for each missing column.
        row ++ List.duplicate("", target_len - row_len)

      true ->
        # Truncate to header length.
        Enum.take(row, target_len)
    end
  end
end
