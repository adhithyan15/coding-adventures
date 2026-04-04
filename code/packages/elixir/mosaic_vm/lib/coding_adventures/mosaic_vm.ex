defmodule CodingAdventures.MosaicVM do
  @moduledoc """
  Generic tree-walking driver for Mosaic compiler backends.

  The MosaicVM is the fourth stage of the Mosaic compiler pipeline:

    Source text → Lexer → Parser → Analyzer → MosaicIR → **VM** → Backend → Target code

  ## Responsibilities

  1. Traverse the MosaicIR tree depth-first.
  2. Normalize every MosaicValue into a ResolvedValue (hex → RGBA, dimension → {value, unit}).
  3. Track the SlotContext (component slots + active each-loop scopes).
  4. Call `MosaicVM.Renderer` callbacks in strict open-before-close order.

  ## What the VM does NOT do

  The VM is agnostic about output format. It has no knowledge of React, Web
  Components, SwiftUI, or any other platform. Backends own the output — the VM
  only drives the traversal and normalizes values.

  ## Traversal order

      begin_component(state, name, slots)
        begin_node(state, root_type, is_primitive, props)
          [for each child:]
            begin_node(state, child_type, …) … end_node(state)   ← child nodes
            render_slot_child(state, slot_name, slot_type)        ← @slotName; children
            begin_when(state, slot_name) … end_when(state)        ← when blocks
            begin_each(state, slot_name, item_name) … end_each(state) ← each blocks
        end_node(state)
      end_component(state) → {:ok, result}

  ## Value normalization

  | MosaicValue kind  | → ResolvedValue kind | What changes                  |
  |-------------------|----------------------|-------------------------------|
  | :color_hex        | :color               | Parsed into r,g,b,a integers  |
  | :dimension        | :dimension           | Split into numeric value+unit |
  | :ident            | :string              | Folded — no semantic change   |
  | :slot_ref         | :slot_ref            | Gains slot_type and is_loop_var |
  | :string/:number/:bool/:enum | unchanged  | Passed through as-is         |

  Note: `:number` values used as property values are passed through as `:number`.
  When the analyzer produces a `:number` value for a dimension context, it should
  emit `:dimension` directly. The VM faithfully normalizes what it receives.

  ## Color parsing

  | Source     | r  | g  | b  | a   |
  |------------|----|----|----|----|
  | #rgb       | rr | gg | bb | 255 |
  | #rrggbb    | rr | gg | bb | 255 |
  | #rrggbbaa  | rr | gg | bb | aa  |

  ## Usage

      ir = CodingAdventures.MosaicAnalyzer.analyze!(source)
      {:ok, result} = CodingAdventures.MosaicVM.run(ir, MyBackend, initial_state)
      result.files  # → [%{filename: "Foo.tsx", content: "..."}]
  """

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Traverse the IR tree, calling renderer callbacks in depth-first order.

  - `ir` — a MosaicIR map from the analyzer: `%{component: %{name:, slots:, tree:}}`
  - `renderer_module` — a module that implements `CodingAdventures.MosaicVM.Renderer`
  - `initial_state` — the backend's initial accumulator (any term)

  Returns `{:ok, result}` from the renderer's `end_component/1`.
  """
  @spec run(map(), module(), any()) :: {:ok, map()}
  def run(ir, renderer_module, initial_state) do
    component = ir.component

    # Build the root slot context. componentSlots is a map from name → slot map
    # for O(1) lookups during traversal. loop_scopes starts empty.
    ctx = %{
      component_slots: Map.new(component.slots, fn s -> {s.name, s} end),
      loop_scopes: []
    }

    state0 = renderer_module.begin_component(initial_state, component.name, component.slots)
    state1 = walk_node(component.tree, ctx, renderer_module, state0)
    renderer_module.end_component(state1)
  end

  # ---------------------------------------------------------------------------
  # Tree Traversal (private)
  # ---------------------------------------------------------------------------

  # Walk a single node: resolve properties, call begin_node, walk children, call end_node.
  defp walk_node(el, ctx, renderer, state) do
    # Resolve every property before calling begin_node so the renderer receives
    # fully normalized values without needing to parse hex strings or dimension units.
    resolved_props =
      Enum.map(el.properties, fn prop ->
        %{name: prop.name, value: resolve_value(prop.value, ctx)}
      end)

    state1 = renderer.begin_node(state, el.tag, el.is_primitive, resolved_props)
    state2 = Enum.reduce(el.children, state1, fn child, acc -> walk_child(child, ctx, renderer, acc) end)
    renderer.end_node(state2)
  end

  # Dispatch a single child to the appropriate renderer method.
  defp walk_child({:node, el}, ctx, renderer, state) do
    walk_node(el, ctx, renderer, state)
  end

  defp walk_child({:slot_ref, slot_name}, ctx, renderer, state) do
    # @slotName; used as a child element (not a property value).
    # Look up the declared type so the renderer knows what kind of content to project.
    slot = Map.fetch!(ctx.component_slots, slot_name)
    renderer.render_slot_child(state, slot_name, slot.type)
  end

  defp walk_child({:when_block, slot_name, children}, ctx, renderer, state) do
    # when @flag { … } — conditional rendering.
    state1 = renderer.begin_when(state, slot_name)
    state2 = Enum.reduce(children, state1, fn child, acc -> walk_child(child, ctx, renderer, acc) end)
    renderer.end_when(state2)
  end

  defp walk_child({:each_block, slot_name, item_name, children}, ctx, renderer, state) do
    # each @items as item { … } — iteration.
    # 1. Find the list slot and extract the element type (T in list<T>).
    list_slot = Map.fetch!(ctx.component_slots, slot_name)

    unless list_slot.type.kind == :list do
      raise "MosaicVMError: each block references @#{slot_name} but it is not a list type"
    end

    element_type = list_slot.type.element_type

    # 2. Call begin_each before pushing loop scope (matches TypeScript behavior).
    state1 = renderer.begin_each(state, slot_name, item_name)

    # 3. Push loop scope so @item references inside the block resolve correctly.
    inner_ctx = %{
      ctx
      | loop_scopes: ctx.loop_scopes ++ [%{item_name: item_name, element_type: element_type}]
    }

    # 4. Walk children with the updated context.
    state2 = Enum.reduce(children, state1, fn child, acc -> walk_child(child, inner_ctx, renderer, acc) end)

    # 5. end_each (loop scope is discarded with inner_ctx).
    renderer.end_each(state2)
  end

  # ---------------------------------------------------------------------------
  # Value Resolution (private)
  # ---------------------------------------------------------------------------

  @doc false
  # Normalize a MosaicValue into a ResolvedValue.
  # Called on every property value before passing to the renderer.
  def resolve_value(%{kind: :string, value: v}, _ctx), do: %{kind: :string, value: v}
  def resolve_value(%{kind: :number, value: v}, _ctx), do: %{kind: :number, value: v}
  def resolve_value(%{kind: :bool, value: v}, _ctx), do: %{kind: :bool, value: v}

  # Bare identifiers (e.g., `fill`, `wrap`) are folded into string — no semantic change.
  def resolve_value(%{kind: :ident, value: v}, _ctx), do: %{kind: :string, value: v}

  def resolve_value(%{kind: :dimension, value: v, unit: u}, _ctx) do
    # The analyzer already parsed "16dp" into %{value: 16, unit: "dp"}.
    # Validate the unit and pass through.
    unit = normalize_unit(u)
    %{kind: :dimension, value: v, unit: unit}
  end

  def resolve_value(%{kind: :color_hex, value: hex}, _ctx) do
    parse_color(hex)
  end

  def resolve_value(%{kind: :enum, namespace: ns, member: m}, _ctx) do
    %{kind: :enum, namespace: ns, member: m}
  end

  def resolve_value(%{kind: :slot_ref, slot_name: slot_name}, ctx) do
    resolve_slot_ref(slot_name, ctx)
  end

  # Fallback: pass unknown value kinds through as-is (permissive mode).
  def resolve_value(v, _ctx), do: v

  # Parse a hex color string (#rgb, #rrggbb, #rrggbbaa) into RGBA integers.
  defp parse_color(hex) do
    h = String.slice(hex, 1..-1//1)  # strip leading '#'

    {r, g, b, a} =
      case String.length(h) do
        3 ->
          # Three-digit shorthand: #rgb → each digit doubled
          <<r1::binary-size(1), g1::binary-size(1), b1::binary-size(1)>> = h
          r = String.to_integer(r1 <> r1, 16)
          g = String.to_integer(g1 <> g1, 16)
          b = String.to_integer(b1 <> b1, 16)
          {r, g, b, 255}

        6 ->
          r = String.to_integer(String.slice(h, 0, 2), 16)
          g = String.to_integer(String.slice(h, 2, 2), 16)
          b = String.to_integer(String.slice(h, 4, 2), 16)
          {r, g, b, 255}

        8 ->
          r = String.to_integer(String.slice(h, 0, 2), 16)
          g = String.to_integer(String.slice(h, 2, 2), 16)
          b = String.to_integer(String.slice(h, 4, 2), 16)
          a = String.to_integer(String.slice(h, 6, 2), 16)
          {r, g, b, a}

        _ ->
          raise "MosaicVMError: Invalid color hex: #{hex}"
      end

    %{kind: :color, r: r, g: g, b: b, a: a}
  end

  # Normalize dimension unit string to an atom.
  # Unknown units are passed through as-is (permissive mode).
  defp normalize_unit("dp"), do: :dp
  defp normalize_unit("sp"), do: :sp
  defp normalize_unit("%"),  do: :percent
  defp normalize_unit(u),    do: u

  # Resolve a slot reference: check loop scopes innermost-first, then component slots.
  defp resolve_slot_ref(slot_name, ctx) do
    # Check active loop scopes, innermost first (list is in push-back order, so reverse).
    loop_match =
      ctx.loop_scopes
      |> Enum.reverse()
      |> Enum.find(fn scope -> scope.item_name == slot_name end)

    if loop_match do
      %{
        kind: :slot_ref,
        slot_name: slot_name,
        slot_type: loop_match.element_type,
        is_loop_var: true
      }
    else
      # Fall back to component slots.
      slot = Map.get(ctx.component_slots, slot_name)
      unless slot do
        raise "MosaicVMError: Unresolved slot reference: @#{slot_name}"
      end
      %{
        kind: :slot_ref,
        slot_name: slot_name,
        slot_type: slot.type,
        is_loop_var: false
      }
    end
  end
end
