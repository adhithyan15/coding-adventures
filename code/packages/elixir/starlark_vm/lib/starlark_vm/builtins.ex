defmodule CodingAdventures.StarlarkVm.Builtins do
  @moduledoc """
  Starlark Built-in Functions — The standard library of Starlark.

  ## Chapter 1: What Are Built-in Functions?

  Built-in functions are functions that are always available in Starlark without
  importing them. They're implemented in Elixir (the host language) rather than
  in Starlark bytecode. When the VM encounters a call to `len(x)` or
  `range(10)`, it dispatches to the Elixir function registered here.

  ## Chapter 2: Starlark vs Python Built-ins

  Starlark's built-ins are a strict subset of Python's, with some restrictions:

  - `sorted()` always returns a new list (no in-place sort)
  - `range()` returns a list, not a lazy range object
  - `type()` returns a string, not a type object
  - `print()` returns nil (output is captured by the VM)
  - No `eval()`, `exec()`, `globals()`, `locals()` (security)

  ## Chapter 3: The 23 Built-in Functions

  ### Type functions
  `type`, `bool`, `int`, `float`, `str`

  ### Collection functions
  `len`, `list`, `dict`, `tuple`, `range`, `sorted`, `reversed`, `enumerate`, `zip`

  ### Logic and math
  `min`, `max`, `abs`, `all`, `any`

  ### String/utility
  `repr`, `hasattr`, `getattr`

  ### I/O
  `print`
  """

  alias CodingAdventures.VirtualMachine.Errors
  alias CodingAdventures.StarlarkVm.Handlers

  # ===========================================================================
  # Type Functions
  # ===========================================================================

  @doc "type(x) — Return the type name as a string."
  def builtin_type([value], _vm) do
    cond do
      is_nil(value) -> "NoneType"
      is_boolean(value) -> "bool"
      is_integer(value) -> "int"
      is_float(value) -> "float"
      is_binary(value) -> "string"
      is_list(value) -> "list"
      match?(%Handlers.StarlarkFunction{}, value) -> "function"
      is_map(value) -> "dict"
      is_tuple(value) -> "tuple"
      true -> "unknown"
    end
  end

  def builtin_type(args, _vm) do
    raise Errors.VMTypeError, "type() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "bool(x) — Convert to boolean."
  def builtin_bool([value], _vm) do
    Handlers.truthy?(value)
  end

  def builtin_bool(args, _vm) do
    raise Errors.VMTypeError, "bool() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "int(x) — Convert to integer."
  def builtin_int([value], _vm) do
    cond do
      is_integer(value) -> value
      is_boolean(value) -> if(value, do: 1, else: 0)
      is_float(value) -> trunc(value)
      is_binary(value) -> String.to_integer(value)
      true -> raise Errors.VMTypeError, "int() argument must be a string or number"
    end
  end

  def builtin_int([value, base_val], _vm) when is_binary(value) do
    String.to_integer(value, base_val)
  end

  def builtin_int(args, _vm) do
    raise Errors.VMTypeError, "int() takes 1 or 2 arguments (#{length(args)} given)"
  end

  @doc "float(x) — Convert to float."
  def builtin_float([value], _vm) do
    cond do
      is_float(value) -> value
      is_integer(value) -> value / 1
      is_binary(value) -> String.to_float(value)
      true -> raise Errors.VMTypeError, "float() argument must be a string or number"
    end
  end

  def builtin_float(args, _vm) do
    raise Errors.VMTypeError, "float() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "str(x) — Convert to string representation."
  def builtin_str([value], _vm) do
    Handlers.starlark_repr(value)
  end

  def builtin_str(args, _vm) do
    raise Errors.VMTypeError, "str() takes exactly 1 argument (#{length(args)} given)"
  end

  # ===========================================================================
  # Collection Functions
  # ===========================================================================

  @doc "len(x) — Return the length of a collection or string."
  def builtin_len([value], _vm) do
    cond do
      is_binary(value) -> String.length(value)
      is_list(value) -> length(value)
      is_map(value) -> map_size(value)
      is_tuple(value) -> tuple_size(value)
      true -> raise Errors.VMTypeError, "object of type '#{builtin_type([value], nil)}' has no len()"
    end
  end

  def builtin_len(args, _vm) do
    raise Errors.VMTypeError, "len() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "list(x) — Convert an iterable to a list."
  def builtin_list([], _vm), do: []
  def builtin_list([value], _vm) do
    cond do
      is_list(value) -> value
      is_tuple(value) -> Tuple.to_list(value)
      is_binary(value) -> String.graphemes(value)
      is_map(value) -> Map.keys(value)
      true -> raise Errors.VMTypeError, "Cannot convert to list: #{inspect(value)}"
    end
  end

  def builtin_list(args, _vm) do
    raise Errors.VMTypeError, "list() takes at most 1 argument (#{length(args)} given)"
  end

  @doc "dict() — Create a new dictionary."
  def builtin_dict([], _vm), do: %{}
  def builtin_dict([pairs], _vm) when is_list(pairs) do
    Enum.reduce(pairs, %{}, fn {k, v}, acc -> Map.put(acc, k, v) end)
  end

  def builtin_dict(args, _vm) do
    raise Errors.VMTypeError, "dict() takes at most 1 argument (#{length(args)} given)"
  end

  @doc "tuple(x) — Convert an iterable to a tuple."
  def builtin_tuple([], _vm), do: {}
  def builtin_tuple([value], _vm) do
    cond do
      is_tuple(value) -> value
      is_list(value) -> List.to_tuple(value)
      is_binary(value) -> value |> String.graphemes() |> List.to_tuple()
      true -> raise Errors.VMTypeError, "Cannot convert to tuple: #{inspect(value)}"
    end
  end

  def builtin_tuple(args, _vm) do
    raise Errors.VMTypeError, "tuple() takes at most 1 argument (#{length(args)} given)"
  end

  @doc """
  range(stop) or range(start, stop[, step]) — Return a list of integers.

  Unlike Python's lazy range(), Starlark's range() returns a concrete list.
  This is because Starlark forbids lazy evaluation for determinism.
  """
  def builtin_range([stop_val], _vm), do: Enum.to_list(0..(stop_val - 1)//1)
  def builtin_range([start_val, stop_val], _vm) do
    if start_val <= stop_val do
      Enum.to_list(start_val..(stop_val - 1)//1)
    else
      []
    end
  end
  def builtin_range([start_val, stop_val, step_val], _vm) do
    if step_val == 0 do
      raise Errors.VMTypeError, "range() step argument must not be zero"
    end

    # Generate range matching Python semantics
    generate_range(start_val, stop_val, step_val, [])
    |> Enum.reverse()
  end
  def builtin_range(args, _vm) do
    raise Errors.VMTypeError, "range() takes 1 to 3 arguments (#{length(args)} given)"
  end

  defp generate_range(current, stop_val, step_val, acc) when step_val > 0 and current >= stop_val, do: acc
  defp generate_range(current, stop_val, step_val, acc) when step_val < 0 and current <= stop_val, do: acc
  defp generate_range(current, stop_val, step_val, acc) do
    generate_range(current + step_val, stop_val, step_val, [current | acc])
  end

  @doc "sorted(x) — Return a new sorted list."
  def builtin_sorted([iterable], _vm), do: Enum.sort(iterable)
  def builtin_sorted([iterable, reverse_flag], _vm) do
    if reverse_flag, do: Enum.sort(iterable, :desc), else: Enum.sort(iterable)
  end

  def builtin_sorted(args, _vm) do
    raise Errors.VMTypeError, "sorted() takes 1 or 2 arguments (#{length(args)} given)"
  end

  @doc "reversed(x) — Return a reversed list."
  def builtin_reversed([iterable], _vm), do: Enum.reverse(iterable)

  def builtin_reversed(args, _vm) do
    raise Errors.VMTypeError, "reversed() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "enumerate(x[, start]) — Return list of (index, value) pairs."
  def builtin_enumerate([iterable], _vm) do
    iterable |> Enum.with_index() |> Enum.map(fn {v, i} -> {i, v} end)
  end

  def builtin_enumerate([iterable, start_val], _vm) do
    iterable |> Enum.with_index(start_val) |> Enum.map(fn {v, i} -> {i, v} end)
  end

  def builtin_enumerate(args, _vm) do
    raise Errors.VMTypeError, "enumerate() takes 1 or 2 arguments (#{length(args)} given)"
  end

  @doc "zip(*iterables) — Return list of tuples."
  def builtin_zip(args, _vm) do
    Enum.zip(args)
  end

  # ===========================================================================
  # Logic and Math Functions
  # ===========================================================================

  @doc "min(x, y, ...) or min(iterable) — Return the smallest element."
  def builtin_min([iterable], _vm) when is_list(iterable), do: Enum.min(iterable)
  def builtin_min(args, _vm) when length(args) > 1, do: Enum.min(args)

  def builtin_min(_args, _vm) do
    raise Errors.VMTypeError, "min() requires at least 1 argument"
  end

  @doc "max(x, y, ...) or max(iterable) — Return the largest element."
  def builtin_max([iterable], _vm) when is_list(iterable), do: Enum.max(iterable)
  def builtin_max(args, _vm) when length(args) > 1, do: Enum.max(args)

  def builtin_max(_args, _vm) do
    raise Errors.VMTypeError, "max() requires at least 1 argument"
  end

  @doc "abs(x) — Return the absolute value."
  def builtin_abs([value], _vm), do: abs(value)

  def builtin_abs(args, _vm) do
    raise Errors.VMTypeError, "abs() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "all(iterable) — Return true if all elements are truthy."
  def builtin_all([iterable], _vm) do
    Enum.all?(iterable, &Handlers.truthy?/1)
  end

  def builtin_all(args, _vm) do
    raise Errors.VMTypeError, "all() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "any(iterable) — Return true if any element is truthy."
  def builtin_any([iterable], _vm) do
    Enum.any?(iterable, &Handlers.truthy?/1)
  end

  def builtin_any(args, _vm) do
    raise Errors.VMTypeError, "any() takes exactly 1 argument (#{length(args)} given)"
  end

  # ===========================================================================
  # String/Utility Functions
  # ===========================================================================

  @doc "repr(x) — Return a string representation."
  def builtin_repr([value], _vm), do: inspect(value)

  def builtin_repr(args, _vm) do
    raise Errors.VMTypeError, "repr() takes exactly 1 argument (#{length(args)} given)"
  end

  @doc "hasattr(x, name) — Return true if x has the named attribute."
  def builtin_hasattr([obj, name], _vm) when is_map(obj) do
    Map.has_key?(obj, name)
  end

  def builtin_hasattr(_args, _vm), do: false

  @doc "getattr(x, name[, default]) — Get a named attribute."
  def builtin_getattr([obj, name], _vm) when is_map(obj) do
    case Map.fetch(obj, name) do
      {:ok, val} -> val
      :error -> raise Errors.VMTypeError, "Object has no attribute '#{name}'"
    end
  end

  def builtin_getattr([obj, name, default], _vm) when is_map(obj) do
    Map.get(obj, name, default)
  end

  def builtin_getattr(args, _vm) do
    raise Errors.VMTypeError, "getattr() takes 2 or 3 arguments (#{length(args)} given)"
  end

  # ===========================================================================
  # I/O Functions
  # ===========================================================================

  @doc """
  print(*args) — Print arguments.

  In Starlark, print() always returns nil. The output is captured
  by the VM's output list rather than going to stdout.
  """
  def builtin_print(args, vm) do
    output_str = Enum.map(args, &Handlers.starlark_repr/1) |> Enum.join(" ")
    vm = %{vm | output: vm.output ++ [output_str]}
    {nil, vm}
  end

  # ===========================================================================
  # Registration Helper
  # ===========================================================================

  @doc """
  Return a map of all built-in function names to their implementations.

  Used by `create_starlark_vm/0` to register all built-ins with the GenericVM.
  """
  def get_all_builtins do
    %{
      # Type functions
      "type" => &builtin_type/2,
      "bool" => &builtin_bool/2,
      "int" => &builtin_int/2,
      "float" => &builtin_float/2,
      "str" => &builtin_str/2,
      # Collection functions
      "len" => &builtin_len/2,
      "list" => &builtin_list/2,
      "dict" => &builtin_dict/2,
      "tuple" => &builtin_tuple/2,
      "range" => &builtin_range/2,
      "sorted" => &builtin_sorted/2,
      "reversed" => &builtin_reversed/2,
      "enumerate" => &builtin_enumerate/2,
      "zip" => &builtin_zip/2,
      # Logic and math
      "min" => &builtin_min/2,
      "max" => &builtin_max/2,
      "abs" => &builtin_abs/2,
      "all" => &builtin_all/2,
      "any" => &builtin_any/2,
      # String/utility
      "repr" => &builtin_repr/2,
      "hasattr" => &builtin_hasattr/2,
      "getattr" => &builtin_getattr/2,
      # I/O
      "print" => &builtin_print/2
    }
  end
end
