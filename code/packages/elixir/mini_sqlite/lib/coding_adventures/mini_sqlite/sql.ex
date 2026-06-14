defmodule CodingAdventures.MiniSqlite.Sql do
  @moduledoc false

  alias CodingAdventures.MiniSqlite.Errors.ProgrammingError

  @create_re ~r/^\s*CREATE\s+TABLE\s+(IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)\s*;?\s*$/is
  @drop_re ~r/^\s*DROP\s+TABLE\s+(IF\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*;?\s*$/is
  @insert_re ~r/^\s*INSERT\s+INTO\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s*\(([^)]*)\))?\s+VALUES\s+(.*?)\s*;?\s*$/is
  @delete_re ~r/^\s*DELETE\s+FROM\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+WHERE\s+(.*?))?\s*;?\s*$/is
  @update_re ~r/^\s*UPDATE\s+([A-Za-z_][A-Za-z0-9_]*)\s+SET\s+(.*?)\s*;?\s*$/is

  def parse_create(sql) do
    case Regex.run(@create_re, sql, capture: :all_but_first) do
      [if_part, table, defs] ->
        columns =
          defs
          |> split_top_level(",")
          |> Enum.map(&identifier_at_start/1)
          |> Enum.reject(&(&1 == nil or &1 == ""))

        if columns == [] do
          {:error, %ProgrammingError{message: "CREATE TABLE requires at least one column"}}
        else
          {:ok, %{table: table, columns: columns, if_not_exists: if_part != ""}}
        end

      _ ->
        {:error, %ProgrammingError{message: "invalid CREATE TABLE statement"}}
    end
  end

  def parse_drop(sql) do
    case Regex.run(@drop_re, sql, capture: :all_but_first) do
      [if_part, table] -> {:ok, %{table: table, if_exists: if_part != ""}}
      _ -> {:error, %ProgrammingError{message: "invalid DROP TABLE statement"}}
    end
  end

  def parse_insert(sql) do
    case Regex.run(@insert_re, sql, capture: :all_but_first) do
      [table, columns_sql, rows_sql] ->
        with {:ok, columns} <- parse_columns(columns_sql),
             {:ok, rows} <- parse_value_rows(rows_sql) do
          {:ok, %{table: table, columns: columns, rows: rows}}
        end

      _ ->
        {:error, %ProgrammingError{message: "invalid INSERT statement"}}
    end
  end

  def parse_update(sql) do
    case Regex.run(@update_re, String.trim(sql), capture: :all_but_first) do
      [table, rest] ->
        {assign_sql, where_sql} = split_top_level_keyword(rest, "WHERE")

        assignments =
          assign_sql
          |> split_top_level(",")
          |> Enum.map(&parse_assignment/1)

        case Enum.find(assignments, &match?({:error, _}, &1)) do
          {:error, error} ->
            {:error, error}

          nil ->
            pairs = Enum.map(assignments, fn {:ok, pair} -> pair end)

            if pairs == [] do
              {:error, %ProgrammingError{message: "UPDATE requires at least one assignment"}}
            else
              {:ok, %{table: table, assignments: pairs, where_sql: where_sql}}
            end
        end

      _ ->
        {:error, %ProgrammingError{message: "invalid UPDATE statement"}}
    end
  end

  def parse_delete(sql) do
    case Regex.run(@delete_re, sql, capture: :all_but_first) do
      [table, where_sql] -> {:ok, %{table: table, where_sql: String.trim(where_sql || "")}}
      _ -> {:error, %ProgrammingError{message: "invalid DELETE statement"}}
    end
  end

  defp parse_columns(""), do: {:ok, []}
  defp parse_columns(nil), do: {:ok, []}

  defp parse_columns(columns_sql) do
    columns =
      columns_sql
      |> split_top_level(",")
      |> Enum.map(&String.trim/1)

    invalid = Enum.find(columns, &(identifier_at_start(&1) != &1))

    if invalid do
      {:error, %ProgrammingError{message: "invalid identifier: #{invalid}"}}
    else
      {:ok, columns}
    end
  end

  defp parse_assignment(assignment) do
    case split_top_level(assignment, "=") do
      [column, value_sql] ->
        column = String.trim(column)

        if identifier_at_start(column) != column do
          {:error, %ProgrammingError{message: "invalid identifier: #{column}"}}
        else
          with {:ok, value} <- parse_literal(value_sql) do
            {:ok, {column, value}}
          end
        end

      _ ->
        {:error, %ProgrammingError{message: "invalid assignment: #{String.trim(assignment)}"}}
    end
  end

  defp parse_value_rows(sql) do
    do_parse_value_rows(String.trim(sql), [])
  end

  defp do_parse_value_rows("", []) do
    {:error, %ProgrammingError{message: "INSERT requires at least one row"}}
  end

  defp do_parse_value_rows("", rows), do: {:ok, Enum.reverse(rows)}

  defp do_parse_value_rows(sql, rows) do
    if not String.starts_with?(sql, "(") do
      {:error, %ProgrammingError{message: "INSERT VALUES rows must be parenthesized"}}
    else
      case find_matching_paren(sql, 0) do
        nil ->
          {:error, %ProgrammingError{message: "unterminated INSERT VALUES row"}}

        close ->
          inside = binary_part(sql, 1, close - 1)

          with {:ok, row} <- parse_value_list(inside) do
            rest = sql |> binary_part(close + 1, byte_size(sql) - close - 1) |> String.trim()

            cond do
              rest == "" ->
                do_parse_value_rows(rest, [row | rows])

              String.starts_with?(rest, ",") ->
                next = rest |> binary_part(1, byte_size(rest) - 1) |> String.trim()
                do_parse_value_rows(next, [row | rows])

              true ->
                {:error, %ProgrammingError{message: "invalid text after INSERT row"}}
            end
          end
      end
    end
  end

  defp parse_value_list(sql) do
    values =
      sql
      |> split_top_level(",")
      |> Enum.map(&parse_literal/1)

    case Enum.find(values, &match?({:error, _}, &1)) do
      {:error, error} -> {:error, error}
      nil -> {:ok, Enum.map(values, fn {:ok, value} -> value end)}
    end
  end

  defp parse_literal(text) do
    value = String.trim(text)
    upper = String.upcase(value)

    cond do
      upper == "NULL" ->
        {:ok, nil}

      upper == "TRUE" ->
        {:ok, true}

      upper == "FALSE" ->
        {:ok, false}

      String.starts_with?(value, "'") and String.ends_with?(value, "'") and byte_size(value) >= 2 ->
        inner = binary_part(value, 1, byte_size(value) - 2)
        {:ok, unescape_string(inner)}

      String.contains?(value, ".") ->
        case Float.parse(value) do
          {number, ""} -> {:ok, number}
          _ -> {:error, %ProgrammingError{message: "expected literal value, got: #{text}"}}
        end

      true ->
        case Integer.parse(value) do
          {number, ""} -> {:ok, number}
          _ -> {:error, %ProgrammingError{message: "expected literal value, got: #{text}"}}
        end
    end
  end

  defp unescape_string(value) do
    value
    |> String.replace("\\n", "\n")
    |> String.replace("\\t", "\t")
    |> String.replace("\\'", "'")
    |> String.replace("\\\\", "\\")
  end

  defp identifier_at_start(text) do
    case Regex.run(~r/^\s*([A-Za-z_][A-Za-z0-9_]*)/, text) do
      [_, identifier] -> identifier
      _ -> nil
    end
  end

  defp split_top_level(text, delimiter) do
    {parts, current, _depth, _quote, _escaped} =
      text
      |> String.graphemes()
      |> Enum.reduce({[], [], 0, nil, false}, fn ch, {parts, current, depth, quote, escaped} ->
        cond do
          quote && escaped ->
            {parts, [ch | current], depth, quote, false}

          quote && ch == "\\" ->
            {parts, [ch | current], depth, quote, true}

          quote && ch == quote ->
            {parts, [ch | current], depth, nil, false}

          quote ->
            {parts, [ch | current], depth, quote, false}

          ch in ["'", "\""] ->
            {parts, [ch | current], depth, ch, false}

          ch == "(" ->
            {parts, [ch | current], depth + 1, nil, false}

          ch == ")" ->
            {parts, [ch | current], max(depth - 1, 0), nil, false}

          depth == 0 and ch == delimiter ->
            {[finish_part(current) | parts], [], depth, nil, false}

          true ->
            {parts, [ch | current], depth, nil, false}
        end
      end)

    [finish_part(current) | parts]
    |> Enum.reverse()
    |> Enum.reject(&(&1 == ""))
  end

  defp finish_part(chars) do
    chars |> Enum.reverse() |> IO.iodata_to_binary() |> String.trim()
  end

  defp split_top_level_keyword(text, keyword) do
    do_split_keyword(text, String.upcase(keyword), byte_size(keyword), 0, 0, nil, false)
  end

  defp do_split_keyword(text, _keyword, _keyword_len, pos, _depth, _quote, _escaped)
       when pos >= byte_size(text) do
    {String.trim(text), ""}
  end

  defp do_split_keyword(text, keyword, keyword_len, pos, depth, quote, escaped) do
    ch = binary_part(text, pos, 1)

    cond do
      quote && escaped ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, depth, quote, false)

      quote && ch == "\\" ->
        do_split_keyword(
          text,
          keyword,
          keyword_len,
          min(pos + 2, byte_size(text)),
          depth,
          quote,
          false
        )

      quote && ch == quote ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, depth, nil, false)

      quote ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, depth, quote, false)

      ch in ["'", "\""] ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, depth, ch, false)

      ch == "(" ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, depth + 1, nil, false)

      ch == ")" ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, max(depth - 1, 0), nil, false)

      depth == 0 and keyword_at?(text, keyword, keyword_len, pos) ->
        left = binary_part(text, 0, pos)
        right = binary_part(text, pos + keyword_len, byte_size(text) - pos - keyword_len)
        {String.trim(left), String.trim(right)}

      true ->
        do_split_keyword(text, keyword, keyword_len, pos + 1, depth, nil, false)
    end
  end

  defp keyword_at?(text, keyword, keyword_len, pos) do
    pos + keyword_len <= byte_size(text) and
      text |> binary_part(pos, keyword_len) |> String.upcase() == keyword and
      boundary?(text, pos - 1) and boundary?(text, pos + keyword_len)
  end

  defp boundary?(_text, index) when index < 0, do: true
  defp boundary?(text, index) when index >= byte_size(text), do: true

  defp boundary?(text, index) do
    not Regex.match?(~r/[A-Za-z0-9_]/, binary_part(text, index, 1))
  end

  defp find_matching_paren(text, open_pos) do
    do_find_paren(text, open_pos, 0, nil, false)
  end

  defp do_find_paren(text, pos, _depth, _quote, _escaped) when pos >= byte_size(text), do: nil

  defp do_find_paren(text, pos, depth, quote, escaped) do
    ch = binary_part(text, pos, 1)

    cond do
      quote && escaped ->
        do_find_paren(text, pos + 1, depth, quote, false)

      quote && ch == "\\" ->
        do_find_paren(text, min(pos + 2, byte_size(text)), depth, quote, false)

      quote && ch == quote ->
        do_find_paren(text, pos + 1, depth, nil, false)

      quote ->
        do_find_paren(text, pos + 1, depth, quote, false)

      ch in ["'", "\""] ->
        do_find_paren(text, pos + 1, depth, ch, false)

      ch == "(" ->
        do_find_paren(text, pos + 1, depth + 1, nil, false)

      ch == ")" and depth == 1 ->
        pos

      ch == ")" ->
        do_find_paren(text, pos + 1, max(depth - 1, 0), nil, false)

      true ->
        do_find_paren(text, pos + 1, depth, nil, false)
    end
  end
end
