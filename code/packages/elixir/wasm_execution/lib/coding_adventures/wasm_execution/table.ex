defmodule CodingAdventures.WasmExecution.Table do
  @moduledoc """
  WASM table implementation for indirect function calls.

  A WASM table is an array of opaque function references. In WASM 1.0,
  elements are either a valid function index or `nil` (uninitialized).

  Tables enable indirect calls via `call_indirect`: look up a function
  reference by index, verify its type, then call it. This implements
  C function pointers, virtual dispatch, and dynamic linking.

  ## Immutable Design

  The table is a simple Elixir list. Set/get operations return a new
  table struct (functional update).
  """

  alias CodingAdventures.WasmExecution.TrapError

  defstruct elements: [], max_size: nil

  @type t :: %__MODULE__{
          elements: [non_neg_integer() | nil],
          max_size: non_neg_integer() | nil
        }

  @doc "Create a new table with `initial_size` nil entries."
  @spec new(non_neg_integer(), non_neg_integer() | nil) :: t()
  def new(initial_size, max_size \\ nil) do
    %__MODULE__{
      elements: List.duplicate(nil, initial_size),
      max_size: max_size
    }
  end

  @doc "Get the function index at the given table index."
  @spec get(t(), non_neg_integer()) :: non_neg_integer() | nil
  def get(%__MODULE__{elements: elems}, index) do
    if index < 0 or index >= length(elems) do
      raise TrapError,
            "Out of bounds table access: index=#{index}, table size=#{length(elems)}"
    end

    Enum.at(elems, index)
  end

  @doc "Set the function index at the given table index. Returns updated table."
  @spec set(t(), non_neg_integer(), non_neg_integer() | nil) :: t()
  def set(%__MODULE__{elements: elems} = table, index, func_index) do
    if index < 0 or index >= length(elems) do
      raise TrapError,
            "Out of bounds table access: index=#{index}, table size=#{length(elems)}"
    end

    %{table | elements: List.replace_at(elems, index, func_index)}
  end

  @doc "Return the current table size."
  @spec table_size(t()) :: non_neg_integer()
  def table_size(%__MODULE__{elements: elems}), do: length(elems)

  @doc """
  Grow the table by `delta` entries (initialized to nil).
  Returns `{old_size, updated_table}` or `{-1, unchanged_table}`.
  """
  @spec grow(t(), non_neg_integer()) :: {integer(), t()}
  def grow(%__MODULE__{} = table, delta) do
    old_size = length(table.elements)
    new_size = old_size + delta

    if table.max_size != nil and new_size > table.max_size do
      {-1, table}
    else
      new_elements = table.elements ++ List.duplicate(nil, delta)
      {old_size, %{table | elements: new_elements}}
    end
  end
end
