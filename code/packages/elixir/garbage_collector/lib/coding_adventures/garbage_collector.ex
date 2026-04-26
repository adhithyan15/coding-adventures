defmodule CodingAdventures.GarbageCollector.ConsCell do
  @moduledoc """
  A Lisp cons cell stored on the managed heap.
  """

  defstruct marked: false, car: nil, cdr: nil

  @type t :: %__MODULE__{
          marked: boolean(),
          car: term(),
          cdr: term()
        }
end

defmodule CodingAdventures.GarbageCollector.Symbol do
  @moduledoc """
  An interned named atom stored on the managed heap.
  """

  defstruct marked: false, name: ""

  @type t :: %__MODULE__{
          marked: boolean(),
          name: String.t()
        }
end

defmodule CodingAdventures.GarbageCollector.LispClosure do
  @moduledoc """
  A function closure with code, captured environment, and parameter names.
  """

  defstruct marked: false, code: nil, env: %{}, params: []

  @type t :: %__MODULE__{
          marked: boolean(),
          code: term(),
          env: %{optional(String.t()) => term()},
          params: [String.t()]
        }
end

defmodule CodingAdventures.GarbageCollector.MarkAndSweep do
  @moduledoc """
  Mark-and-sweep GC state.
  """

  defstruct heap: %{},
            next_address: 0x10000,
            total_allocations: 0,
            total_collections: 0,
            total_freed: 0

  @type t :: %__MODULE__{
          heap: %{optional(non_neg_integer()) => map()},
          next_address: non_neg_integer(),
          total_allocations: non_neg_integer(),
          total_collections: non_neg_integer(),
          total_freed: non_neg_integer()
        }
end

defmodule CodingAdventures.GarbageCollector do
  @moduledoc """
  Functional mark-and-sweep garbage collector primitives.

  The API mirrors the Python and Rust packages while following Elixir's
  immutable data model: operations that mutate the heap return an updated GC
  state alongside their result.
  """

  alias __MODULE__.{ConsCell, LispClosure, MarkAndSweep, Symbol}

  @type address :: non_neg_integer()
  @type heap_object :: ConsCell.t() | Symbol.t() | LispClosure.t()
  @type stats :: %{
          total_allocations: non_neg_integer(),
          total_collections: non_neg_integer(),
          total_freed: non_neg_integer(),
          heap_size: non_neg_integer()
        }

  @doc "Create an empty mark-and-sweep garbage collector."
  @spec new() :: MarkAndSweep.t()
  def new, do: %MarkAndSweep{}

  @doc "Create a cons cell heap object."
  @spec cons_cell(term(), term()) :: ConsCell.t()
  def cons_cell(car \\ nil, cdr \\ nil), do: %ConsCell{car: car, cdr: cdr}

  @doc "Create a symbol heap object."
  @spec symbol(String.t()) :: Symbol.t()
  def symbol(name \\ ""), do: %Symbol{name: name}

  @doc "Create a Lisp closure heap object."
  @spec lisp_closure(term(), map(), [String.t()]) :: LispClosure.t()
  def lisp_closure(code \\ nil, env \\ %{}, params \\ []),
    do: %LispClosure{code: code, env: env, params: params}

  @doc "Allocate a heap object and return `{updated_gc, address}`."
  @spec allocate(MarkAndSweep.t(), heap_object()) :: {MarkAndSweep.t(), address()}
  def allocate(%MarkAndSweep{} = gc, %{marked: _} = object) do
    address = gc.next_address

    updated_gc = %{
      gc
      | heap: Map.put(gc.heap, address, object),
        next_address: address + 1,
        total_allocations: gc.total_allocations + 1
    }

    {updated_gc, address}
  end

  @doc "Look up a live heap object by address."
  @spec deref(MarkAndSweep.t(), term()) :: {:ok, heap_object()} | {:error, :invalid_address}
  def deref(%MarkAndSweep{heap: heap}, address) when is_integer(address) do
    case Map.fetch(heap, address) do
      {:ok, object} -> {:ok, object}
      :error -> {:error, :invalid_address}
    end
  end

  def deref(%MarkAndSweep{}, _address), do: {:error, :invalid_address}

  @doc "Look up a live heap object by address, raising on invalid addresses."
  @spec deref!(MarkAndSweep.t(), term()) :: heap_object()
  def deref!(%MarkAndSweep{} = gc, address) do
    case deref(gc, address) do
      {:ok, object} -> object
      {:error, :invalid_address} -> raise KeyError, key: address, term: gc.heap
    end
  end

  @doc "Run a collection cycle and return `{updated_gc, freed_count}`."
  @spec collect(MarkAndSweep.t(), [term()]) :: {MarkAndSweep.t(), non_neg_integer()}
  def collect(%MarkAndSweep{} = gc, roots) when is_list(roots) do
    marked_gc = Enum.reduce(roots, gc, fn root, acc -> mark_value(acc, root) end)

    {survivors, freed} =
      Enum.reduce(marked_gc.heap, {%{}, 0}, fn {address, object}, {heap, freed_count} ->
        if marked?(object) do
          {Map.put(heap, address, set_marked(object, false)), freed_count}
        else
          {heap, freed_count + 1}
        end
      end)

    updated_gc = %{
      marked_gc
      | heap: survivors,
        total_collections: marked_gc.total_collections + 1,
        total_freed: marked_gc.total_freed + freed
    }

    {updated_gc, freed}
  end

  @doc "Return the number of live objects on the heap."
  @spec heap_size(MarkAndSweep.t()) :: non_neg_integer()
  def heap_size(%MarkAndSweep{heap: heap}), do: map_size(heap)

  @doc "Return allocation and collection counters."
  @spec stats(MarkAndSweep.t()) :: stats()
  def stats(%MarkAndSweep{} = gc) do
    %{
      total_allocations: gc.total_allocations,
      total_collections: gc.total_collections,
      total_freed: gc.total_freed,
      heap_size: heap_size(gc)
    }
  end

  @doc "Return true when an address points to a live heap object."
  @spec valid_address?(MarkAndSweep.t(), term()) :: boolean()
  def valid_address?(%MarkAndSweep{heap: heap}, address) when is_integer(address),
    do: Map.has_key?(heap, address)

  def valid_address?(%MarkAndSweep{}, _address), do: false

  @doc "Return heap addresses referenced by a heap object."
  @spec references(term()) :: [address()]
  def references(%ConsCell{car: car, cdr: cdr}) do
    [car, cdr]
    |> Enum.filter(&is_integer/1)
  end

  def references(%LispClosure{env: env}) do
    env
    |> Map.values()
    |> Enum.filter(&is_integer/1)
  end

  def references(_object), do: []

  defp mark_value(%MarkAndSweep{} = gc, value) when is_integer(value) do
    case Map.fetch(gc.heap, value) do
      {:ok, object} ->
        if marked?(object) do
          gc
        else
          marked_object = set_marked(object, true)
          marked_gc = %{gc | heap: Map.put(gc.heap, value, marked_object)}

          Enum.reduce(references(marked_object), marked_gc, fn ref, acc ->
            mark_value(acc, ref)
          end)
        end

      :error ->
        gc
    end
  end

  defp mark_value(%MarkAndSweep{} = gc, value) when is_list(value) do
    Enum.reduce(value, gc, fn item, acc -> mark_value(acc, item) end)
  end

  defp mark_value(%MarkAndSweep{} = gc, value) when is_map(value) do
    if Map.has_key?(value, :__struct__) do
      gc
    else
      value
      |> Map.values()
      |> Enum.reduce(gc, fn item, acc -> mark_value(acc, item) end)
    end
  end

  defp mark_value(%MarkAndSweep{} = gc, _value), do: gc

  defp marked?(%{marked: marked}), do: marked
  defp marked?(_object), do: false

  defp set_marked(%{marked: _} = object, marked), do: %{object | marked: marked}
end

defmodule CodingAdventures.GarbageCollector.SymbolTable do
  @moduledoc """
  Interns symbol names to live GC addresses.

  The table does not keep symbols alive. After collection, stale addresses are
  ignored and the next `intern/3` call allocates a fresh symbol.
  """

  alias CodingAdventures.GarbageCollector

  defstruct table: %{}

  @type t :: %__MODULE__{table: %{optional(String.t()) => GarbageCollector.address()}}

  @doc "Create an empty symbol table."
  @spec new() :: t()
  def new, do: %__MODULE__{}

  @doc "Intern a symbol name and return `{updated_table, updated_gc, address}`."
  @spec intern(t(), GarbageCollector.MarkAndSweep.t(), String.t()) ::
          {t(), GarbageCollector.MarkAndSweep.t(), GarbageCollector.address()}
  def intern(%__MODULE__{} = table, gc, name) when is_binary(name) do
    case Map.fetch(table.table, name) do
      {:ok, address} ->
        if GarbageCollector.valid_address?(gc, address) do
          {table, gc, address}
        else
          allocate_symbol(table, gc, name)
        end

      :error ->
        allocate_symbol(table, gc, name)
    end
  end

  @doc "Look up a live symbol address without allocating."
  @spec lookup(t(), GarbageCollector.MarkAndSweep.t(), String.t()) ::
          {:ok, GarbageCollector.address()} | :error
  def lookup(%__MODULE__{} = table, gc, name) when is_binary(name) do
    case Map.fetch(table.table, name) do
      {:ok, address} ->
        if GarbageCollector.valid_address?(gc, address), do: {:ok, address}, else: :error

      :error ->
        :error
    end
  end

  @doc "Return all currently interned live symbols."
  @spec all_symbols(t(), GarbageCollector.MarkAndSweep.t()) :: %{
          optional(String.t()) => GarbageCollector.address()
        }
  def all_symbols(%__MODULE__{} = table, gc) do
    table.table
    |> Enum.filter(fn {_name, address} -> GarbageCollector.valid_address?(gc, address) end)
    |> Map.new()
  end

  defp allocate_symbol(%__MODULE__{} = table, gc, name) do
    {updated_gc, address} = GarbageCollector.allocate(gc, GarbageCollector.symbol(name))
    updated_table = %{table | table: Map.put(table.table, name, address)}
    {updated_table, updated_gc, address}
  end
end
