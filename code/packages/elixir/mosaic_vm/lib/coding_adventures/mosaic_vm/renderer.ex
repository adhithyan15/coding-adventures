defmodule CodingAdventures.MosaicVM.Renderer do
  @moduledoc """
  The backend protocol for Mosaic compiler code generators.

  Every backend that wants to consume the MosaicVM tree walk must implement
  this behaviour. The VM calls the callbacks in strict depth-first,
  open-before-close order:

    begin_component
      begin_node (root)
        begin_node (child) … end_node (child)
        begin_when … end_when
        begin_each … end_each
        render_slot_child (for @slotName; child references)
      end_node (root)
    end_component → {:ok, result}

  ## State accumulation pattern

  Unlike the TypeScript version which uses a mutable class, the Elixir
  convention is to pass a `state` term through each callback and return
  the updated state. Backends start with an `initial_state` value and
  accumulate output into it — typically a map with lists or string buffers.

  ## Contract

  - `begin_component/3` receives the component name, slot list, and initial
    state. It returns the updated state.
  - `end_component/1` receives the final state and returns `{:ok, result}`
    where `result` is a map of `%{files: [%{filename: …, content: …}]}`.
  - All other callbacks receive state and return updated state.
  """

  @type slot :: map()
  @type mosaic_type :: map()

  # ---------------------------------------------------------------------------
  # Lifecycle: component
  # ---------------------------------------------------------------------------

  @doc """
  Called once before any tree traversal.

  Use this to initialize output buffers and emit file headers.

  - `name` — PascalCase component name, e.g. `"ProfileCard"`
  - `slots` — all declared slot maps from the analyzer IR
  - `state` — the accumulator state coming in
  """
  @callback begin_component(state :: any, name :: String.t(), slots :: list(slot())) :: any

  @doc """
  Called once after all tree traversal is complete.

  Returns `{:ok, result}` where result is `%{files: [%{filename: …, content: …}]}`.
  """
  @callback end_component(state :: any) :: {:ok, result :: map()}

  # ---------------------------------------------------------------------------
  # Lifecycle: node
  # ---------------------------------------------------------------------------

  @doc """
  Called when entering a node element.

  - `node_type` — element type, e.g. `"Row"`, `"Text"`, `"Button"`
  - `is_primitive` — true for built-in elements, false for imported components
  - `props` — list of `%{name: …, value: …}` resolved property maps
  """
  @callback begin_node(state :: any, node_type :: String.t(), is_primitive :: boolean(), props :: list(map())) :: any

  @doc """
  Called when leaving a node element (after all its children are processed).
  """
  @callback end_node(state :: any) :: any

  # ---------------------------------------------------------------------------
  # Children
  # ---------------------------------------------------------------------------

  @doc """
  Called when a slot reference appears as a **child** of a node (the `@slot;` form).

  - `slot_name` — the referenced slot, e.g. `"action"`
  - `slot_type` — the declared MosaicType map for that slot
  """
  @callback render_slot_child(state :: any, slot_name :: String.t(), slot_type :: mosaic_type()) :: any

  # ---------------------------------------------------------------------------
  # Conditional rendering
  # ---------------------------------------------------------------------------

  @doc """
  Called when entering a `when @flag { … }` block.
  """
  @callback begin_when(state :: any, slot_name :: String.t()) :: any

  @doc """
  Called when leaving a `when` block.
  """
  @callback end_when(state :: any) :: any

  # ---------------------------------------------------------------------------
  # Iteration
  # ---------------------------------------------------------------------------

  @doc """
  Called when entering an `each @items as item { … }` block.

  - `slot_name` — the list slot being iterated
  - `item_name` — the loop variable name
  """
  @callback begin_each(state :: any, slot_name :: String.t(), item_name :: String.t()) :: any

  @doc """
  Called when leaving an `each` block.
  """
  @callback end_each(state :: any) :: any
end
