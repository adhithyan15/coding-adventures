defmodule CodingAdventures.LatticeAstToCss.Scope do
  @moduledoc """
  Lexical scope chain for Lattice variables, mixins, and functions.

  ## Why Lexical Scoping?

  CSS has no scope — everything is global. Lattice adds variables, mixins,
  and functions, which need scoping rules to prevent name collisions and
  enable local reasoning.

  Lattice uses **lexical (static) scoping**, meaning a variable's scope is
  determined by where it appears in the source text, not by runtime call order.
  This is the same model used by JavaScript, Python, and most modern languages.

  ## How It Works

  Each `{ }` block creates a new child scope. Variables declared inside a
  block are local to that scope. Looking up a variable walks up the parent
  chain until found:

      $color: red;              # global scope (depth 0)
      .parent {                 # child scope (depth 1)
          $color: blue;         # shadows the global $color
          color: $color;        # => blue (found at depth 1)
          .child {              # grandchild scope (depth 2)
              color: $color;    # => blue (inherited from depth 1)
          }
      }
      .sibling {                # another child scope (depth 1)
          color: $color;        # => red (global, not affected by .parent)
      }

  ## Implementation

  Implemented as a persistent, immutable linked list of scope nodes. Each
  node is a struct with:
  - `bindings` — a `Map` from names to values
  - `parent` — the enclosing scope, or `nil` for the global scope

  Since Elixir data is immutable, "setting" a value in a scope returns a
  new scope struct — it does not mutate the original. This is the functional
  style.

  ## Special Scoping Rules

  - **Mixin expansion** creates a child scope of the *caller's* scope, giving
    mixins access to the caller's variables (like closures in JavaScript).

  - **Function evaluation** creates an **isolated** scope whose parent is the
    global scope, NOT the caller's scope. This prevents functions from
    accidentally depending on where they're called from — they only see their
    own parameters and global definitions.

  ## Example

      global = Scope.new()
      global = Scope.set(global, "$color", "red")

      block = Scope.child(global)
      block = Scope.set(block, "$color", "blue")

      {:ok, "blue"} = Scope.get(block, "$color")   # local
      {:ok, "red"}  = Scope.get(global, "$color")  # unchanged

      nested = Scope.child(block)
      {:ok, "blue"} = Scope.get(nested, "$color")  # inherited from parent
  """

  @enforce_keys []
  defstruct bindings: %{}, parent: nil

  @type t :: %__MODULE__{
          bindings: map(),
          parent: t() | nil
        }

  @doc """
  Create a new global scope (no parent) or a scope with a given parent.

  ## Examples

      global = Scope.new()
      child = Scope.new(global)
  """
  @spec new(t() | nil) :: t()
  def new(parent \\ nil), do: %__MODULE__{parent: parent}

  @doc """
  Look up a name in this scope or any ancestor scope.

  Walks up the parent chain until the name is found. Returns `{:ok, value}`
  if found, `:error` if not found anywhere.

  This is the core of lexical scoping — a variable declared in an outer
  scope is visible in all inner scopes unless shadowed.

  ## Examples

      scope = Scope.new() |> Scope.set("$x", 42)
      {:ok, 42} = Scope.get(scope, "$x")
      :error    = Scope.get(scope, "$y")
  """
  @spec get(t(), String.t()) :: {:ok, any()} | :error
  def get(%__MODULE__{bindings: bindings, parent: parent}, name) do
    case Map.fetch(bindings, name) do
      {:ok, _} = found ->
        found

      :error when not is_nil(parent) ->
        get(parent, name)

      :error ->
        :error
    end
  end

  @doc """
  Bind a name to a value in this scope.

  Always binds in the *current* scope, never in a parent scope. This means
  a child scope can shadow a parent's binding without modifying the parent.

  Returns the updated scope (functional style — does not mutate).

  ## Examples

      scope = Scope.new()
      scope = Scope.set(scope, "$x", 10)
      {:ok, 10} = Scope.get(scope, "$x")
  """
  @spec set(t(), String.t(), any()) :: t()
  def set(%__MODULE__{} = scope, name, value) do
    %{scope | bindings: Map.put(scope.bindings, name, value)}
  end

  @doc """
  Check whether a name exists anywhere in the scope chain.

  Returns `true` if the name is bound anywhere, `false` otherwise.
  """
  @spec has?(t(), String.t()) :: boolean()
  def has?(%__MODULE__{} = scope, name) do
    case get(scope, name) do
      {:ok, _} -> true
      :error -> false
    end
  end

  @doc """
  Check whether a name exists in *this* scope only (not parents).

  Useful for detecting re-declarations and shadowing.
  """
  @spec has_local?(t(), String.t()) :: boolean()
  def has_local?(%__MODULE__{bindings: bindings}, name) do
    Map.has_key?(bindings, name)
  end

  @doc """
  Bind a name to a value in the root (global) scope.

  Walks up the parent chain to find the root scope (the one with
  no parent), then sets the binding there. This implements the
  `!global` flag in Lattice variable declarations.

  When `!global` is used inside a deeply nested scope (e.g., inside
  a mixin inside a `@for` loop), the variable is set at the top level,
  making it visible everywhere.

  Returns the updated scope chain. Since Elixir data is immutable,
  this creates a new chain of scope nodes from root to the current
  scope.

  ## Examples

      global = Scope.new() |> Scope.set("$theme", "light")
      child = Scope.child(global)
      child = Scope.set_global(child, "$theme", "dark")
      # The root scope now has $theme = "dark"
  """
  @spec set_global(t(), String.t(), any()) :: t()
  def set_global(%__MODULE__{parent: nil} = scope, name, value) do
    # We ARE the root — set directly
    %{scope | bindings: Map.put(scope.bindings, name, value)}
  end

  def set_global(%__MODULE__{parent: parent} = scope, name, value) do
    # Recurse to root, then rebuild the chain on the way back
    new_parent = set_global(parent, name, value)
    %{scope | parent: new_parent}
  end

  @doc """
  Create a new child scope with `parent` as the enclosing scope.

  The child inherits all bindings from the parent chain via `get/2`, but
  `set/3` calls on the child only affect the child.

  ## Examples

      global = Scope.new() |> Scope.set("$x", 10)
      child  = Scope.child(global)
      {:ok, 10} = Scope.get(child, "$x")  # inherited

      child = Scope.set(child, "$x", 20)  # shadow
      {:ok, 20} = Scope.get(child, "$x")  # local shadow
      {:ok, 10} = Scope.get(global, "$x") # global unchanged
  """
  @spec child(t()) :: t()
  def child(%__MODULE__{} = parent), do: new(parent)

  @doc """
  The depth of this scope in the chain (0 = global).

  The global scope has depth 0. Each `child/1` call adds 1.
  """
  @spec depth(t()) :: non_neg_integer()
  def depth(%__MODULE__{parent: nil}), do: 0
  def depth(%__MODULE__{parent: parent}), do: 1 + depth(parent)
end
