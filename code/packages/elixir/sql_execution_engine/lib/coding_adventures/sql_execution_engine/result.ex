defmodule CodingAdventures.SqlExecutionEngine.Result do
  @moduledoc """
  The result of executing a SELECT query.

  ## Fields

  - `columns` — List of strings: the display names of each output column, in
    order.  These come from column aliases (`AS name`), qualified names
    (`table.col`), or bare expression text.

  - `rows` — List of lists of values.  Each inner list corresponds to one
    output row, with values aligned to `columns` by index.

  ## Example

      %QueryResult{
        columns: ["id", "employee_name", "salary"],
        rows: [
          [1, "Alice", 90000],
          [3, "Carol", 95000]
        ]
      }

  ## Design decision: list-of-lists, not list-of-maps

  We use `[col, …]` lists for rows rather than `%{col => value}` maps
  because:

  1. Ordering is explicit — SQL SELECT imposes a strict column order.
  2. Projections can produce duplicate column names (`SELECT a, a FROM t`),
     which are legal SQL but ambiguous as map keys.
  3. Result sets are typically consumed by iteration (rendering a table,
     sending over a wire protocol), so positional access is natural.

  Callers who need named access can zip columns + one row:
      Enum.zip(result.columns, row) |> Map.new()
  """

  defstruct [:columns, :rows]

  @type t :: %__MODULE__{
          columns: [String.t()],
          rows: [[term()]]
        }
end
