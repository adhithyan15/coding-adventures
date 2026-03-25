defmodule CodingAdventures.SqlExecutionEngine.Join do
  @moduledoc """
  Join algorithms for the SQL execution engine.

  ## Supported join types

  All five standard SQL join types are implemented via nested-loop join —
  the simplest correct algorithm.  For the small in-memory data sets typical
  of this engine, nested-loop is perfectly adequate.

  | Join Type   | What it returns                                             |
  |-------------|-------------------------------------------------------------|
  | INNER JOIN  | Only rows where the ON condition is TRUE on both sides      |
  | LEFT JOIN   | All left rows; right side filled with NULLs if no match     |
  | RIGHT JOIN  | All right rows; left side filled with NULLs if no match     |
  | FULL JOIN   | All rows from both sides; NULLs on the unmatched side       |
  | CROSS JOIN  | Cartesian product — every left row paired with every right   |

  ## Nested-loop join explained

                    left_rows
                       │
                  ┌────┴────┐
                  │         │
             right_rows   right_rows   ← inner loop repeats for every left row
                  │         │
              ON predicate evaluated for each (left, right) pair
                  │         │
              matched pairs ─────────────────────► output

  This is O(N×M) in time and O(1) in extra space (excluding output).
  Hash join or sort-merge join would be more efficient for large data,
  but that optimisation belongs in a later layer of the engine.

  ## Row context merging

  Each row is a map `%{"table.col" => value, "col" => value, ...}`.
  Joining two rows means merging their maps.  If two columns share a bare
  name (`name` from employees and `name` from departments), the right row's
  bare key overwrites the left's.  Qualified keys (`employees.name`,
  `departments.name`) remain unambiguous.

  ## NULL rows for outer joins

  When a left row has no matching right row (LEFT JOIN), we need a "null
  row" where all right-side columns are nil.  We construct this by taking
  the right schema and building a map of `"col" => nil` and
  `"table.col" => nil` for all right columns.
  """

  alias CodingAdventures.SqlExecutionEngine.Expression

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Apply a single JOIN clause to the current set of rows.

  ## Parameters
  - `left_rows`    — list of row context maps from the left side (the FROM table
    or the result of prior joins)
  - `right_rows`   — list of row context maps from the right table
  - `right_schema` — ordered list of column names for the right table
  - `right_alias`  — string alias used for the right table (for null row keys)
  - `join_type`    — one of `"INNER"`, `"LEFT"`, `"RIGHT"`, `"FULL"`, `"CROSS"`
  - `on_expr`      — the ASTNode for the ON condition
  """
  @spec apply_join(
          left_rows :: [map()],
          right_rows :: [map()],
          right_schema :: [String.t()],
          right_alias :: String.t(),
          join_type :: String.t(),
          on_expr :: term()
        ) :: [map()]
  def apply_join(left_rows, right_rows, right_schema, right_alias, join_type, on_expr) do
    case join_type do
      "INNER" -> inner_join(left_rows, right_rows, on_expr)
      "LEFT"  -> left_join(left_rows, right_rows, right_schema, right_alias, on_expr)
      "RIGHT" -> right_join(left_rows, right_rows, right_schema, right_alias, on_expr)
      "FULL"  -> full_join(left_rows, right_rows, right_schema, right_alias, on_expr)
      "CROSS" -> cross_join(left_rows, right_rows)
      other   -> raise "Unknown join type: #{other}"
    end
  end

  # ---------------------------------------------------------------------------
  # INNER JOIN
  # ---------------------------------------------------------------------------
  #
  # For each (left, right) pair, include it only if the ON condition is true.

  defp inner_join(left_rows, right_rows, on_expr) do
    for left <- left_rows,
        right <- right_rows,
        merged = Map.merge(left, right),
        Expression.eval_expr(on_expr, merged) == true do
      merged
    end
  end

  # ---------------------------------------------------------------------------
  # LEFT JOIN (LEFT OUTER JOIN)
  # ---------------------------------------------------------------------------
  #
  # All left rows are included.  If no right row matches the ON condition,
  # the right columns are filled with nil.

  defp left_join(left_rows, right_rows, right_schema, right_alias, on_expr) do
    null_right = null_row(right_schema, right_alias)

    Enum.flat_map(left_rows, fn left ->
      matches =
        Enum.filter(right_rows, fn right ->
          merged = Map.merge(left, right)
          Expression.eval_expr(on_expr, merged) == true
        end)

      case matches do
        [] ->
          # No match — emit left row with all right columns as nil
          [Map.merge(left, null_right)]

        _ ->
          Enum.map(matches, fn right -> Map.merge(left, right) end)
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # RIGHT JOIN (RIGHT OUTER JOIN)
  # ---------------------------------------------------------------------------
  #
  # RIGHT JOIN is the mirror of LEFT JOIN.  We swap left and right, run a
  # LEFT JOIN, then re-merge in the original order.
  #
  # Because our row context maps are keyed by "table.col" and bare "col",
  # the merge order matters: right-side bare keys dominate in the output
  # of a LEFT JOIN over (right, left).  We invert back to (left, right) order
  # by re-merging: Map.merge(left, right).
  #
  # In practice, for the purposes of this engine, RIGHT JOIN is rarely used,
  # but it is part of the SQL standard and our test suite.

  defp right_join(left_rows, right_rows, _right_schema, _right_alias, on_expr) do
    # We need the left schema/alias to build null rows on the left side.
    # Extract from the first left row's keys.
    null_left = if left_rows == [], do: %{}, else: null_row_from_keys(hd(left_rows))

    Enum.flat_map(right_rows, fn right ->
      matches =
        Enum.filter(left_rows, fn left ->
          merged = Map.merge(left, right)
          Expression.eval_expr(on_expr, merged) == true
        end)

      case matches do
        [] ->
          [Map.merge(null_left, right)]

        _ ->
          Enum.map(matches, fn left -> Map.merge(left, right) end)
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # FULL JOIN (FULL OUTER JOIN)
  # ---------------------------------------------------------------------------
  #
  # Union of LEFT JOIN and RIGHT JOIN, with duplicates removed.
  # The simplest correct implementation:
  #   1. Compute left join result
  #   2. Find all right rows that had no match in any left row
  #   3. Append right-unmatched rows with null left columns

  defp full_join(left_rows, right_rows, right_schema, right_alias, on_expr) do
    null_right = null_row(right_schema, right_alias)
    null_left = if left_rows == [], do: %{}, else: null_row_from_keys(hd(left_rows))

    # LEFT JOIN part
    left_result =
      Enum.flat_map(left_rows, fn left ->
        matches =
          Enum.filter(right_rows, fn right ->
            merged = Map.merge(left, right)
            Expression.eval_expr(on_expr, merged) == true
          end)

        case matches do
          [] -> [Map.merge(left, null_right)]
          _ -> Enum.map(matches, fn right -> Map.merge(left, right) end)
        end
      end)

    # Find right rows that matched NOTHING on the left
    unmatched_right =
      Enum.filter(right_rows, fn right ->
        not Enum.any?(left_rows, fn left ->
          merged = Map.merge(left, right)
          Expression.eval_expr(on_expr, merged) == true
        end)
      end)
      |> Enum.map(fn right -> Map.merge(null_left, right) end)

    left_result ++ unmatched_right
  end

  # ---------------------------------------------------------------------------
  # CROSS JOIN
  # ---------------------------------------------------------------------------
  #
  # Cartesian product — no ON condition.  Every left row paired with every
  # right row.  This is useful for generating combinations.

  defp cross_join(left_rows, right_rows) do
    for left <- left_rows, right <- right_rows do
      Map.merge(left, right)
    end
  end

  # ---------------------------------------------------------------------------
  # Null row helpers
  # ---------------------------------------------------------------------------

  # Build a "null row" for a table given its schema and alias.
  # Used to fill the "missing" side in outer joins.
  #
  # For a table aliased as "e" with schema ["id", "name", "salary"]:
  #   %{"e.id" => nil, "e.name" => nil, "e.salary" => nil,
  #     "id" => nil, "name" => nil, "salary" => nil}
  defp null_row(schema, table_alias) do
    Enum.reduce(schema, %{}, fn col, acc ->
      acc
      |> Map.put("#{table_alias}.#{col}", nil)
      |> Map.put(col, nil)
    end)
  end

  # Build a null row from an existing row's keys by setting all values to nil.
  # Used for RIGHT JOIN where we don't have the schema separately.
  defp null_row_from_keys(row) do
    Map.new(row, fn {k, _v} -> {k, nil} end)
  end
end
