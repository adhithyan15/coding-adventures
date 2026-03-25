defmodule CodingAdventures.SqlExecutionEngine.DataSource do
  @moduledoc """
  The DataSource behaviour — the pluggable data layer for the SQL engine.

  Any module that implements this behaviour can be queried by the SQL execution
  engine.  Think of it as a minimal "storage engine" interface, analogous to
  MySQL's InnoDB or SQLite's B-tree layer but at a much higher level of
  abstraction.

  ## The two callbacks

  ### `schema/1`

  Returns the list of column names for a given table as a list of plain
  strings.  Column order matters: it defines the "natural" column order that
  `SELECT *` will use.

      schema("employees")
      # => ["id", "name", "dept_id", "salary", "active"]

  If the table does not exist, raise `TableNotFoundError`.

  ### `scan/1`

  Returns all rows for a table as a list of maps.  Each map uses string keys
  (the column names) and Elixir native values.

  | SQL Type  | Elixir Value      |
  |-----------|-------------------|
  | INTEGER   | integer()         |
  | FLOAT     | float()           |
  | VARCHAR   | binary() / String |
  | BOOLEAN   | true / false      |
  | NULL      | nil               |

      scan("employees")
      # => [
      #   %{"id" => 1, "name" => "Alice", "dept_id" => 1, ...},
      #   …
      # ]

  If the table does not exist, raise `TableNotFoundError`.

  ## Why a behaviour and not a protocol?

  A **behaviour** is appropriate here because:
  - We have multiple callbacks (`schema` + `scan`) that conceptually form a
    single "interface contract" rather than a single dispatch function.
  - The data source is a stateless module that is passed as a module reference
    (`data_source = InMemorySource`), not a data structure.
  - Protocols dispatch on the *type of a value*; behaviours dispatch on the
    *module* — and our data sources are modules.

  ## Implementing a DataSource

  ```elixir
  defmodule MyCSVSource do
    @behaviour CodingAdventures.SqlExecutionEngine.DataSource

    @impl true
    def schema("users"), do: ["id", "name", "email"]

    @impl true
    def scan("users") do
      File.stream!("users.csv")
      |> CSV.decode!(headers: true)
      |> Enum.to_list()
    end
  end
  ```
  """

  alias CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError

  @doc """
  Return the ordered list of column names for `table_name`.

  ## Raises
  - `TableNotFoundError` if the table is unknown.
  """
  @callback schema(table_name :: String.t()) :: [String.t()]

  @doc """
  Return every row from `table_name` as a list of `%{column => value}` maps.

  ## Raises
  - `TableNotFoundError` if the table is unknown.
  """
  @callback scan(table_name :: String.t()) :: [%{String.t() => term()}]

  # Make TableNotFoundError available to implementations without a separate alias.
  defdelegate table_not_found!(table_name), to: TableNotFoundError, as: :exception
end
