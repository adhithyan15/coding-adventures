defmodule CodingAdventures.RegisterVM.Scope do
  @moduledoc """
  Scope chain management — the runtime representation of variable scoping.

  ## Two kinds of variables

  This VM distinguishes two levels of scope:

  1. **Globals** — a flat map of name → value. All top-level variables
     live here. Accessed by `LdaGlobal` / `StaGlobal`. No parent chain.

  2. **Context slots** — a linked list of context frames, one per
     lexically enclosing function. Used for closures: when a function
     "captures" a variable from an outer function, both share the same
     context frame slot.

  ## Analogy: Filing cabinet vs. nested folders

  Think of globals as a large flat filing cabinet. Everything has a unique
  name and lives at the top level — easy to look up, nothing nested.

  Context slots are like nested folders on a hard drive. Each function has
  its own folder. Captured variables are files that live in a parent folder.
  Walking up the `parent` chain is like `cd ..` until you reach the right folder.

  ## Context frame structure

      %{
        slots: {nil, nil, nil},   # tuple of slot values (indexed by position)
        parent: parent_context    # reference to enclosing function's context
      }

  ## Why tuples for slots?

  Tuples give O(1) indexed access via `elem/2` and `put_elem/3`. Lists
  would require O(n) traversal. Since slot counts are small but accesses
  are frequent, tuples are the right choice.
  """

  # ---------------------------------------------------------------------------
  # Global scope
  # ---------------------------------------------------------------------------

  @doc """
  Creates a new empty global variable map.

  ## Examples

      iex> globals = Scope.new_globals()
      iex> globals
      %{}
  """
  def new_globals, do: %{}

  @doc """
  Looks up a global variable by name.

  Returns `{:ok, value}` if found, `:error` if not defined.
  We use the tagged-tuple convention so callers can distinguish
  "undefined variable" from "variable whose value is nil."

  ## Examples

      iex> g = Scope.set_global(%{}, "x", 42)
      iex> Scope.get_global(g, "x")
      {:ok, 42}
      iex> Scope.get_global(g, "y")
      :error
  """
  def get_global(globals, name) do
    case Map.fetch(globals, name) do
      {:ok, value} -> {:ok, value}
      :error -> :error
    end
  end

  @doc """
  Sets a global variable, returning the updated globals map.

  Because Elixir data is immutable, "setting" a global means returning
  a new map with the key added or updated. The caller must capture this
  return value — the original map is unchanged.

  ## Examples

      iex> g = Scope.new_globals()
      iex> g2 = Scope.set_global(g, "answer", 42)
      iex> Scope.get_global(g2, "answer")
      {:ok, 42}
  """
  def set_global(globals, name, value) do
    Map.put(globals, name, value)
  end

  # ---------------------------------------------------------------------------
  # Context (closure scope) frames
  # ---------------------------------------------------------------------------

  @doc """
  Creates a new context frame with `slot_count` slots, all initialised to nil.

  The `parent` argument links this context to the enclosing function's context,
  forming the scope chain. Pass `nil` for the outermost context.

  ## Examples

      iex> ctx = Scope.new_context(nil, 3)
      iex> ctx.slots
      {nil, nil, nil}
      iex> ctx.parent
      nil
  """
  def new_context(parent, slot_count) do
    %{
      slots: Tuple.duplicate(nil, max(slot_count, 0)),
      parent: parent
    }
  end

  @doc """
  Reads a context slot, walking `depth` links up the parent chain first.

  Depth 0 means "this function's own context."
  Depth 1 means "the enclosing function's context."
  And so on.

  Returns `{:ok, value}` or `:error` if depth is out of range.

  ## Examples

      iex> inner = Scope.new_context(nil, 2)
      iex> outer = Scope.new_context(inner, 2)
      iex> {:ok, _} = Scope.get_slot(outer, 0, 0)
  """
  def get_slot(context, depth, idx) do
    case walk_chain(context, depth) do
      nil -> :error
      target_ctx ->
        if idx >= 0 and idx < tuple_size(target_ctx.slots) do
          {:ok, elem(target_ctx.slots, idx)}
        else
          :error
        end
    end
  end

  @doc """
  Writes a value to a context slot, returning the updated top-level context.

  Because context frames are immutable Elixir maps, updating a slot deep
  in the chain requires rebuilding the path from the top. This is O(depth)
  but depth is typically very small (1–3).

  Returns `{:ok, updated_context}` or `:error`.
  """
  def set_slot(context, depth, idx, value) do
    case set_slot_recursive(context, depth, idx, value) do
      {:ok, updated} -> {:ok, updated}
      :error -> :error
    end
  end

  # ---------------------------------------------------------------------------
  # Private helpers
  # ---------------------------------------------------------------------------

  # Walk `depth` parent links from `context`, returning the target context frame.
  # Returns nil if the chain is too short.
  defp walk_chain(context, 0), do: context
  defp walk_chain(nil, _depth), do: nil
  defp walk_chain(%{parent: parent}, depth), do: walk_chain(parent, depth - 1)

  # Recursively update a slot at `depth` levels deep, rebuilding each parent
  # reference as we unwind the recursion.
  defp set_slot_recursive(nil, _depth, _idx, _value), do: :error

  defp set_slot_recursive(context, 0, idx, value) do
    if idx >= 0 and idx < tuple_size(context.slots) do
      new_slots = put_elem(context.slots, idx, value)
      {:ok, %{context | slots: new_slots}}
    else
      :error
    end
  end

  defp set_slot_recursive(context, depth, idx, value) do
    case set_slot_recursive(context.parent, depth - 1, idx, value) do
      {:ok, new_parent} -> {:ok, %{context | parent: new_parent}}
      :error -> :error
    end
  end
end
