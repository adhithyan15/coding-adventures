defmodule CodingAdventures.LZ78.TrieCursor do
  @moduledoc """
  A step-by-step cursor for navigating a byte-keyed trie.

  Unlike a full trie API (which operates on complete keys), `TrieCursor`
  maintains a current position and advances one byte at a time. This is the
  core abstraction for streaming dictionary algorithms:

  - **LZ78** (CMP01): `step/2` → emit token on miss, `insert/3` new entry
  - **LZW**  (CMP03): same pattern with a pre-seeded 256-entry alphabet

  ## Design

  The trie is stored as an arena: a flat `%{node_id => {dict_id, children}}` map
  where `node_id` is a non-negative integer assigned in insertion order. Node 0
  is the root (always present). `children` is a `%{byte => node_id}` map.

  Because Elixir data is immutable, each operation returns a new cursor struct.

  ## Usage

      cursor = TrieCursor.new()
      for byte <- data do
        case TrieCursor.step(cursor, byte) do
          {:ok, cursor} ->
            cursor  # followed edge — keep going
          :miss ->
            emit_token(TrieCursor.dict_id(cursor), byte)
            cursor = TrieCursor.insert(cursor, byte, next_id)
            TrieCursor.reset(cursor)
        end
      end
      unless TrieCursor.at_root?(cursor), do: emit_flush_token(cursor)
  """

  # ─── Struct ────────────────────────────────────────────────────────────────

  @typedoc "An arena-based trie cursor. Nodes are indexed by non-negative integer."
  @type t :: %__MODULE__{
    nodes:   %{non_neg_integer() => {non_neg_integer(), %{byte() => non_neg_integer()}}},
    current: non_neg_integer(),
    size:    non_neg_integer()
  }

  # nodes:   %{id => {dict_id, children}}
  # current: id of the node the cursor is sitting on
  # size:    next available node id (also equals total node count)
  defstruct nodes: %{0 => {0, %{}}}, current: 0, size: 1

  # ─── Constructor ───────────────────────────────────────────────────────────

  @doc "Create a new `TrieCursor` with an empty trie. Cursor starts at root."
  @spec new() :: t()
  def new, do: %__MODULE__{}

  # ─── Navigation ────────────────────────────────────────────────────────────

  @doc """
  Try to follow the child edge for `byte` from the current position.

  Returns `{:ok, new_cursor}` if the edge exists and advances the cursor.
  Returns `:miss` if no such edge exists (cursor unchanged).

  ## Examples

      iex> cursor = TrieCursor.new()
      iex> TrieCursor.step(cursor, ?A)
      :miss

      iex> cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      iex> {:ok, advanced} = TrieCursor.step(cursor, ?A)
      iex> TrieCursor.dict_id(advanced)
      1
  """
  @spec step(t(), byte()) :: {:ok, t()} | :miss
  def step(%__MODULE__{nodes: nodes, current: current} = cursor, byte) do
    {_dict_id, children} = Map.fetch!(nodes, current)
    case Map.fetch(children, byte) do
      {:ok, child_id} -> {:ok, %{cursor | current: child_id}}
      :error          -> :miss
    end
  end

  @doc """
  Add a child edge for `byte` at the current position with the given `dict_id`.

  Does not advance the cursor — call `reset/1` to return to root.

  ## Examples

      iex> cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      iex> TrieCursor.dict_id(cursor)
      0
  """
  @spec insert(t(), byte(), non_neg_integer()) :: t()
  def insert(%__MODULE__{nodes: nodes, current: current, size: size} = cursor, byte, dict_id) do
    new_id = size
    {current_dict_id, old_children} = Map.fetch!(nodes, current)
    new_children = Map.put(old_children, byte, new_id)
    new_nodes =
      nodes
      |> Map.put(new_id, {dict_id, %{}})
      |> Map.put(current, {current_dict_id, new_children})
    %{cursor | nodes: new_nodes, size: size + 1}
  end

  @doc """
  Reset the cursor to the trie root.

  ## Examples

      iex> cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      iex> {:ok, advanced} = TrieCursor.step(cursor, ?A)
      iex> TrieCursor.at_root?(TrieCursor.reset(advanced))
      true
  """
  @spec reset(t()) :: t()
  def reset(%__MODULE__{} = cursor), do: %{cursor | current: 0}

  # ─── Queries ───────────────────────────────────────────────────────────────

  @doc """
  Dictionary ID at the current cursor position.

  Returns `0` when the cursor is at root (representing the empty sequence).

  ## Examples

      iex> TrieCursor.dict_id(TrieCursor.new())
      0
  """
  @spec dict_id(t()) :: non_neg_integer()
  def dict_id(%__MODULE__{nodes: nodes, current: current}) do
    {dict_id, _children} = Map.fetch!(nodes, current)
    dict_id
  end

  @doc """
  Returns `true` if the cursor is at the root node.

  ## Examples

      iex> TrieCursor.at_root?(TrieCursor.new())
      true
  """
  @spec at_root?(t()) :: boolean()
  def at_root?(%__MODULE__{current: current}), do: current == 0

  # ─── Enumerable (DFS traversal) ────────────────────────────────────────────

  @doc """
  Returns all `{path, dict_id}` pairs in the trie (depth-first order).

  `path` is a list of bytes from root to the node. Only nodes with
  `dict_id > 0` (i.e., nodes added via `insert/3`) are yielded.

  ## Examples

      iex> cursor = TrieCursor.new() |> TrieCursor.insert(?A, 1)
      iex> TrieCursor.entries(cursor)
      {[65], 1}
      # (single element returned by next/1)
  """
  @spec to_list(t()) :: [{[byte()], non_neg_integer()}]
  def to_list(%__MODULE__{nodes: nodes}) do
    collect_entries(nodes, 0, []) |> Enum.sort_by(fn {_path, id} -> id end)
  end

  defp collect_entries(nodes, node_id, path) do
    {dict_id, children} = Map.fetch!(nodes, node_id)
    own =
      if dict_id > 0, do: [{Enum.reverse(path), dict_id}], else: []
    child_entries =
      children
      |> Enum.flat_map(fn {byte, child_id} ->
        collect_entries(nodes, child_id, [byte | path])
      end)
    own ++ child_entries
  end
end
