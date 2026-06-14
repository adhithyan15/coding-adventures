defmodule CodingAdventures.MiniSqlite.Binding do
  @moduledoc false

  alias CodingAdventures.MiniSqlite.Errors.ProgrammingError

  def bind(sql, params) when is_binary(sql) and is_list(params) do
    scan(sql, params, 0, [])
  end

  defp scan(sql, params, pos, acc) when pos >= byte_size(sql) do
    case params do
      [] ->
        {:ok, acc |> Enum.reverse() |> IO.iodata_to_binary()}

      _ ->
        {:error, %ProgrammingError{message: "too many parameters for SQL statement"}}
    end
  end

  defp scan(sql, params, pos, acc) do
    ch = binary_part(sql, pos, 1)

    cond do
      ch in ["'", "\""] ->
        next = read_quoted(sql, pos, ch)
        scan(sql, params, next, [binary_part(sql, pos, next - pos) | acc])

      ch == "-" and pos + 1 < byte_size(sql) and binary_part(sql, pos + 1, 1) == "-" ->
        next = read_line_comment(sql, pos)
        scan(sql, params, next, [binary_part(sql, pos, next - pos) | acc])

      ch == "/" and pos + 1 < byte_size(sql) and binary_part(sql, pos + 1, 1) == "*" ->
        next = read_block_comment(sql, pos)
        scan(sql, params, next, [binary_part(sql, pos, next - pos) | acc])

      ch == "?" ->
        case params do
          [] ->
            {:error, %ProgrammingError{message: "not enough parameters for SQL statement"}}

          [param | rest] ->
            scan(sql, rest, pos + 1, [to_sql_literal(param) | acc])
        end

      true ->
        scan(sql, params, pos + 1, [ch | acc])
    end
  end

  defp read_quoted(sql, pos, quote) do
    do_read_quoted(sql, pos + 1, quote)
  end

  defp do_read_quoted(sql, pos, _quote) when pos >= byte_size(sql), do: byte_size(sql)

  defp do_read_quoted(sql, pos, quote) do
    ch = binary_part(sql, pos, 1)

    cond do
      ch == "\\" ->
        do_read_quoted(sql, min(pos + 2, byte_size(sql)), quote)

      ch == quote ->
        pos + 1

      true ->
        do_read_quoted(sql, pos + 1, quote)
    end
  end

  defp read_line_comment(sql, pos) do
    do_read_line_comment(sql, pos + 2)
  end

  defp do_read_line_comment(sql, pos) when pos >= byte_size(sql), do: byte_size(sql)

  defp do_read_line_comment(sql, pos) do
    if binary_part(sql, pos, 1) == "\n" do
      pos
    else
      do_read_line_comment(sql, pos + 1)
    end
  end

  defp read_block_comment(sql, pos) do
    do_read_block_comment(sql, pos + 2)
  end

  defp do_read_block_comment(sql, pos) when pos + 1 >= byte_size(sql), do: byte_size(sql)

  defp do_read_block_comment(sql, pos) do
    if binary_part(sql, pos, 2) == "*/" do
      pos + 2
    else
      do_read_block_comment(sql, pos + 1)
    end
  end

  defp to_sql_literal(nil), do: "NULL"
  defp to_sql_literal(true), do: "TRUE"
  defp to_sql_literal(false), do: "FALSE"
  defp to_sql_literal(value) when is_integer(value), do: Integer.to_string(value)
  defp to_sql_literal(value) when is_float(value), do: Float.to_string(value)

  defp to_sql_literal(value) when is_binary(value) do
    value
    |> String.replace("\\", "\\\\")
    |> String.replace("'", "\\'")
    |> String.replace("\n", "\\n")
    |> String.replace("\t", "\\t")
    |> then(&"'#{&1}'")
  end
end
