defmodule CodingAdventures.SqlCsvSource.CsvDataSource do
  @moduledoc """
  A `DataSource` implementation that reads tables from CSV files on disk.

  ## The DataSource dispatch model

  The SQL execution engine dispatches data source calls as:

      data_source.schema(table_name)   # where data_source is a module atom
      data_source.scan(table_name)

  This means `data_source` must be a **module** (Elixir atom), not a struct
  instance.  The module must implement the `DataSource` behaviour with
  single-argument callbacks.

  ## How `new/1` works — dynamic module creation

  Because the directory path is runtime configuration but the engine needs a
  module atom, `new/1` uses `Module.create/3` to construct a *unique anonymous
  module* at runtime.  This module:

  - Closes over the `dir` value at creation time (baked into the module body
    as a module attribute).
  - Implements `schema/1` and `scan/1` with the appropriate CSV logic.
  - Is never registered under a human-readable name — it's an anonymous
    one-off module, similar to how anonymous functions close over variables.

  Each call to `new/1` produces a fresh module atom (e.g.
  `:"Elixir.CodingAdventures.SqlCsvSource.CsvDataSource._0"`).  The module
  lives for the lifetime of the BEAM VM process.  For typical use — creating
  one source per directory and executing a handful of queries — this is fine.

  ## Directory convention

  The module is initialised with a directory path.  Every file named
  `<tablename>.csv` in that directory is a table.  The table name is the
  filename without the `.csv` extension.

      /data/csvdb/
        employees.csv    →  table "employees"
        departments.csv  →  table "departments"

  ## Column ordering (`schema/1`)

  Column names are taken from the **header row** of the CSV file, in
  declaration order.  We read the first line of the raw file and split on `,`
  (with whitespace trimming) rather than going through `CsvParser.parse_csv/1`,
  because `parse_csv/1` drops the header row and returns only data rows — there
  is no way to recover the header from its output.

  For CSV files with plain (unquoted) column names — the overwhelming majority
  of real-world CSV files — this simple split is correct and efficient.

  ## Type coercion (`scan/1`)

  CSV stores everything as text.  We apply the following conversions so that
  the SQL engine can evaluate typed predicates:

      CSV text   →  Elixir value
      ──────────────────────────
      ""         →  nil          (empty field = SQL NULL)
      "true"     →  true         (case-sensitive boolean literal)
      "false"    →  false        (case-sensitive boolean literal)
      "42"       →  42           (integer — parses completely, no remainder)
      "3.14"     →  3.14         (float — parses completely, no remainder)
      "123abc"   →  "123abc"     ("fully parseable" guard prevents truncation)
      anything   →  string as-is

  The "no remainder" check (`{n, ""}` pattern) is critical: `Integer.parse("123abc")`
  returns `{123, "abc"}` — we must reject this and treat "123abc" as a string.

  ## Error handling

  If the CSV file for a table does not exist, both `schema/1` and `scan/1` raise
  `CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError`.
  """

  alias CodingAdventures.SqlExecutionEngine.Errors.TableNotFoundError

  # ---------------------------------------------------------------------------
  # new/1 — create a data source module for a directory
  # ---------------------------------------------------------------------------

  @doc """
  Create a new `DataSource` module that reads CSV files from `dir`.

  Returns a module atom that can be passed directly to
  `CodingAdventures.SqlExecutionEngine.execute/2`.

  ## Implementation note

  This function uses `Module.create/3` to construct a fresh anonymous module
  at runtime.  The directory path is baked into the module as a compile-time
  constant (module attribute), not looked up from process state.  This makes
  the module self-contained and safe to pass around across process boundaries.

  ## Example

      source = CsvDataSource.new("test/fixtures")
      # source is a module atom like :"Elixir.CodingAdventures...._3"

      {:ok, result} = SqlExecutionEngine.execute("SELECT * FROM employees", source)
  """
  @spec new(String.t()) :: module()
  def new(dir) when is_binary(dir) do
    # Generate a unique suffix for the module name so repeated calls to new/1
    # don't try to redefine the same module.
    n = next_counter()
    module_name = :"Elixir.CodingAdventures.SqlCsvSource.CsvDataSource._#{n}"

    # Build the AST for the module body.  We inject `dir` as a literal string
    # in the quoted AST — it becomes a module attribute that the schema/1 and
    # scan/1 functions read at call time.
    #
    # Using `Module.create/3` with a quoted body is the standard Elixir idiom
    # for generating modules at runtime.  The result is a fully-compiled BEAM
    # module, indistinguishable from a hand-written one.
    module_body =
      quote do
        @behaviour CodingAdventures.SqlExecutionEngine.DataSource

        # Bake the directory into the module at creation time.
        @dir unquote(dir)

        @impl true
        def schema(table_name) do
          CodingAdventures.SqlCsvSource.CsvDataSource.do_schema(@dir, table_name)
        end

        @impl true
        def scan(table_name) do
          CodingAdventures.SqlCsvSource.CsvDataSource.do_scan(@dir, table_name)
        end
      end

    {:module, mod, _binary, _exports} =
      Module.create(module_name, module_body, Macro.Env.location(__ENV__))

    mod
  end

  # ---------------------------------------------------------------------------
  # do_schema/2 — shared implementation for schema/1
  # ---------------------------------------------------------------------------
  #
  # Called by the dynamically-created module's schema/1.  We make this a
  # public function on CsvDataSource so the generated module AST can reference
  # it by fully-qualified name (you can't call a private function from a
  # different module, even a dynamically-created one).

  @doc false
  def do_schema(dir, table_name) do
    path = csv_path(dir, table_name)

    case File.read(path) do
      {:ok, content} ->
        headers_from_content(content, table_name)

      {:error, _posix} ->
        raise TableNotFoundError, table_name
    end
  end

  # ---------------------------------------------------------------------------
  # do_scan/2 — shared implementation for scan/1
  # ---------------------------------------------------------------------------

  @doc false
  def do_scan(dir, table_name) do
    path = csv_path(dir, table_name)

    case File.read(path) do
      {:ok, content} ->
        case CsvParser.parse_csv(content) do
          {:ok, rows} ->
            # Apply type coercion to every string value in every row.
            Enum.map(rows, fn row ->
              Map.new(row, fn {col, val} -> {col, coerce(val)} end)
            end)

          {:error, reason} ->
            raise TableNotFoundError, "Cannot read table #{table_name}: #{reason}"
        end

      {:error, _posix} ->
        raise TableNotFoundError, table_name
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Construct the file path for a given table name and base directory.
  # Convention: table "employees" → "<dir>/employees.csv"
  defp csv_path(dir, table_name) do
    Path.join(dir, "#{table_name}.csv")
  end

  # Extract the ordered list of column names from raw CSV content.
  #
  # Strategy: split on the first newline to get the header line, then split
  # that line on commas.  We trim whitespace from each column name to handle
  # headers like "id, name, salary" (space after the comma).
  #
  # Why not use CsvParser.parse_csv/1 for this?
  # parse_csv/1 treats the first row as the header and returns it as map *keys*
  # in the data rows.  Map keys in Elixir have no guaranteed iteration order.
  # To get a stable ordered list of column names we must read the raw header
  # line ourselves.
  #
  # Limitation: this approach does not handle quoted column names that contain
  # commas (e.g. `"last,name"`).  In practice CSV column names are never
  # quoted with embedded commas, so this is not a real-world limitation.
  defp headers_from_content(content, table_name) do
    header_line =
      content
      |> String.split(["\r\n", "\n"], parts: 2)
      |> List.first("")
      |> String.trim()

    if header_line == "" do
      raise TableNotFoundError, table_name
    else
      cols =
        header_line
        |> String.split(",")
        |> Enum.map(&String.trim/1)
        |> Enum.reject(&(&1 == ""))

      if cols == [] do
        raise TableNotFoundError, table_name
      else
        cols
      end
    end
  end

  # coerce/1 — convert a raw CSV string to the appropriate Elixir type.
  #
  # The conversion is applied in a specific order to avoid ambiguity:
  #
  #   1. Empty string first — catches "" before any numeric parser sees it.
  #      (Both Integer.parse("") and Float.parse("") return :error anyway,
  #      but being explicit about the nil conversion is clearer.)
  #
  #   2. Boolean literals before numbers — avoids hypothetical "true1"
  #      being misinterpreted (it would fall through both boolean clauses
  #      regardless, but the intention is clearer with this ordering).
  #
  #   3. Integer before float — "42" becomes integer 42, not float 42.0.
  #      Most SQL integer columns in a CSV dump store values like "42", not
  #      "42.0", so this matches real-world expectations.
  #
  #   4. Float — "3.14" or "1.0" (has a decimal point) becomes float.
  #
  #   5. String fallthrough — anything not matched above stays as-is.
  #
  # The `{n, ""}` pattern (empty remainder) is critical for correctness:
  #   Integer.parse("123abc") → {123, "abc"}  ← remainder non-empty → NOT int
  #   Integer.parse("123")    → {123, ""}     ← remainder empty → IS int
  defp coerce(""), do: nil
  defp coerce("true"), do: true
  defp coerce("false"), do: false

  defp coerce(value) do
    case Integer.parse(value) do
      {n, ""} ->
        n

      _ ->
        case Float.parse(value) do
          {f, ""} ->
            f

          _ ->
            value
        end
    end
  end

  # Generate a unique integer suffix for dynamic module names.
  # Stored in the process dictionary so it resets per process (which is
  # fine — module atoms are global to the VM, so a test process and a
  # production process would get independent counters starting at 0; the
  # names would differ only if they were both run in the same process, which
  # they won't be).
  #
  # Actually, to be safe across the lifetime of a long-running VM, we use
  # :erlang.unique_integer/1 which is guaranteed globally unique per VM node.
  defp next_counter do
    :erlang.unique_integer([:positive, :monotonic])
  end
end
