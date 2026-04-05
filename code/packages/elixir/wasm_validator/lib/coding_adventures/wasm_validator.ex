defmodule CodingAdventures.WasmValidator do
  @moduledoc """
  WebAssembly 1.0 module validator.

  ## What Does Validation Do?

  Validation is the step between parsing and execution. A parsed WASM module
  is syntactically correct (the binary format is well-formed), but may still
  contain semantic errors:

  - A function references a type index that doesn't exist.
  - An import refers to a kind that doesn't match.
  - Memory limits exceed the WASM spec maximum (65536 pages = 4 GiB).
  - An export references a function index out of bounds.

  The validator checks these constraints and produces a `ValidatedModule`
  struct that the execution engine can trust.

  ## Functional Design

  This is a pure-functional validator: no side effects, no mutable state.
  Input is a `WasmModule` struct, output is `{:ok, %ValidatedModule{}}` or
  `{:error, reason}`.

  ## Usage

      {:ok, validated} = CodingAdventures.WasmValidator.validate(module)
      validated.module       # the original module
      validated.func_types   # combined list of all function type signatures
  """

  alias CodingAdventures.WasmTypes.WasmModule

  # Maximum number of memory pages allowed by the WASM 1.0 spec.
  @max_memory_pages 65_536

  defmodule ValidatedModule do
    @moduledoc """
    A validated WASM module with resolved type information.

    ## Fields

    - `module` -- the original parsed WasmModule
    - `func_types` -- combined list of function type signatures (imports + module)
    - `num_imported_funcs` -- how many of the func_types are imported
    """
    defstruct [:module, func_types: [], num_imported_funcs: 0]
  end

  @doc """
  Validate a parsed WASM module.

  Returns `{:ok, %ValidatedModule{}}` if the module is valid, or
  `{:error, reason}` if validation fails.

  ## Checks Performed

  1. Memory limits are within spec bounds (max 65536 pages).
  2. Function type indices are valid (within the type section).
  3. Export indices are within bounds.
  4. The start function index (if present) is valid.

  ## Example

      {:ok, validated} = CodingAdventures.WasmValidator.validate(module)
  """
  @spec validate(WasmModule.t()) :: {:ok, ValidatedModule.t()} | {:error, String.t()}
  def validate(%WasmModule{} = wasm_module) do
    with :ok <- validate_memories(wasm_module),
         :ok <- validate_function_types(wasm_module),
         :ok <- validate_exports(wasm_module),
         :ok <- validate_start(wasm_module) do
      # Build the combined function type array (imports first, then module functions).
      {func_types, num_imported_funcs} = build_func_types(wasm_module)

      {:ok,
       %ValidatedModule{
         module: wasm_module,
         func_types: func_types,
         num_imported_funcs: num_imported_funcs
       }}
    end
  end

  # ---------------------------------------------------------------------------
  # Memory Validation
  # ---------------------------------------------------------------------------

  defp validate_memories(%WasmModule{memories: memories}) do
    Enum.reduce_while(memories, :ok, fn mem, :ok ->
      cond do
        mem.limits.min > @max_memory_pages ->
          {:halt, {:error, "memory minimum #{mem.limits.min} exceeds max pages #{@max_memory_pages}"}}

        mem.limits.max != nil and mem.limits.max > @max_memory_pages ->
          {:halt, {:error, "memory maximum #{mem.limits.max} exceeds max pages #{@max_memory_pages}"}}

        mem.limits.max != nil and mem.limits.min > mem.limits.max ->
          {:halt, {:error, "memory minimum #{mem.limits.min} exceeds maximum #{mem.limits.max}"}}

        true ->
          {:cont, :ok}
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Function Type Validation
  # ---------------------------------------------------------------------------

  defp validate_function_types(%WasmModule{functions: functions, types: types}) do
    num_types = length(types)

    Enum.reduce_while(functions, :ok, fn type_idx, :ok ->
      if type_idx >= num_types do
        {:halt, {:error, "function references type index #{type_idx} but only #{num_types} types exist"}}
      else
        {:cont, :ok}
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Export Validation
  # ---------------------------------------------------------------------------

  defp validate_exports(%WasmModule{} = m) do
    num_funcs = count_imported_funcs(m) + length(m.functions)
    num_tables = count_imported_tables(m) + length(m.tables)
    num_memories = count_imported_memories(m) + length(m.memories)
    num_globals = count_imported_globals(m) + length(m.globals)

    Enum.reduce_while(m.exports, :ok, fn exp, :ok ->
      case exp.kind do
        :function when exp.index >= num_funcs ->
          {:halt, {:error, "export '#{exp.name}' references function #{exp.index} but only #{num_funcs} exist"}}
        :table when exp.index >= num_tables ->
          {:halt, {:error, "export '#{exp.name}' references table #{exp.index} but only #{num_tables} exist"}}
        :memory when exp.index >= num_memories ->
          {:halt, {:error, "export '#{exp.name}' references memory #{exp.index} but only #{num_memories} exist"}}
        :global when exp.index >= num_globals ->
          {:halt, {:error, "export '#{exp.name}' references global #{exp.index} but only #{num_globals} exist"}}
        _ ->
          {:cont, :ok}
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Start Function Validation
  # ---------------------------------------------------------------------------

  defp validate_start(%WasmModule{start: nil}), do: :ok

  defp validate_start(%WasmModule{} = m) do
    num_funcs = count_imported_funcs(m) + length(m.functions)

    if m.start >= num_funcs do
      {:error, "start function index #{m.start} out of bounds (#{num_funcs} functions)"}
    else
      :ok
    end
  end

  # ---------------------------------------------------------------------------
  # Build Combined Function Type Array
  # ---------------------------------------------------------------------------

  defp build_func_types(%WasmModule{} = m) do
    # Imported functions come first, then module-defined functions.
    import_types =
      m.imports
      |> Enum.filter(fn imp -> imp.kind == :function end)
      |> Enum.map(fn imp ->
        case imp.type_info do
          {:function, type_idx} -> Enum.at(m.types, type_idx)
          _ -> nil
        end
      end)
      |> Enum.reject(&is_nil/1)

    module_types =
      m.functions
      |> Enum.map(fn type_idx -> Enum.at(m.types, type_idx) end)
      |> Enum.reject(&is_nil/1)

    {import_types ++ module_types, length(import_types)}
  end

  # ---------------------------------------------------------------------------
  # Import Counting Helpers
  # ---------------------------------------------------------------------------

  defp count_imported_funcs(%WasmModule{imports: imports}) do
    Enum.count(imports, fn imp -> imp.kind == :function end)
  end

  defp count_imported_tables(%WasmModule{imports: imports}) do
    Enum.count(imports, fn imp -> imp.kind == :table end)
  end

  defp count_imported_memories(%WasmModule{imports: imports}) do
    Enum.count(imports, fn imp -> imp.kind == :memory end)
  end

  defp count_imported_globals(%WasmModule{imports: imports}) do
    Enum.count(imports, fn imp -> imp.kind == :global end)
  end
end
