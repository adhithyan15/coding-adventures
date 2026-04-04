defmodule CodingAdventures.MosaicVMTest do
  use ExUnit.Case

  alias CodingAdventures.MosaicVM

  # ---------------------------------------------------------------------------
  # MockRenderer — records every callback invocation into a list.
  #
  # State is a list of tagged tuples built up in reverse (then reversed at the
  # end). This gives us a call-order log we can assert on.
  # ---------------------------------------------------------------------------

  defmodule MockRenderer do
    @behaviour CodingAdventures.MosaicVM.Renderer

    @impl true
    def begin_component(state, name, slots), do: [{:begin_component, name, slots} | state]

    @impl true
    def end_component(state) do
      {:ok, %{log: Enum.reverse(state)}}
    end

    @impl true
    def begin_node(state, node_type, is_primitive, props), do: [{:begin_node, node_type, is_primitive, props} | state]

    @impl true
    def end_node(state), do: [{:end_node} | state]

    @impl true
    def render_slot_child(state, slot_name, slot_type), do: [{:render_slot_child, slot_name, slot_type} | state]

    @impl true
    def begin_when(state, slot_name), do: [{:begin_when, slot_name} | state]

    @impl true
    def end_when(state), do: [{:end_when} | state]

    @impl true
    def begin_each(state, slot_name, item_name), do: [{:begin_each, slot_name, item_name} | state]

    @impl true
    def end_each(state), do: [{:end_each} | state]
  end

  # ---------------------------------------------------------------------------
  # Helpers to build minimal IR maps (mirrors MosaicAnalyzer output shape)
  # ---------------------------------------------------------------------------

  defp make_node(tag, opts \\ []) do
    %{
      tag: tag,
      is_primitive: Keyword.get(opts, :is_primitive, true),
      properties: Keyword.get(opts, :properties, []),
      children: Keyword.get(opts, :children, [])
    }
  end

  defp make_slot(name, kind) when is_atom(kind) do
    %{name: name, type: %{kind: kind}, default_value: nil}
  end

  defp make_slot(name, type_map) when is_map(type_map) do
    %{name: name, type: type_map, default_value: nil}
  end

  defp make_ir(component_name, slots, tree) do
    %{component: %{name: component_name, slots: slots, tree: tree}}
  end

  # Run the mock and extract the call log.
  defp run_mock(ir), do: run_mock(ir, [])
  defp run_mock(ir, initial), do: elem(MosaicVM.run(ir, MockRenderer, initial), 1).log

  # ---------------------------------------------------------------------------
  # Tests: module loads
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicVM)
  end

  test "renderer behaviour module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicVM.Renderer)
  end

  # ---------------------------------------------------------------------------
  # Tests: minimal component
  # ---------------------------------------------------------------------------

  test "minimal component — begin/end_component called" do
    ir = make_ir("Simple", [], make_node("Column"))
    log = run_mock(ir)

    assert Enum.at(log, 0) == {:begin_component, "Simple", []}
    assert List.last(log) == {:end_node}
  end

  test "minimal component — root node begin/end called in correct order" do
    ir = make_ir("Card", [], make_node("Column"))
    log = run_mock(ir)

    # begin_component → begin_node → end_node
    assert {:begin_component, "Card", []} = Enum.at(log, 0)
    assert {:begin_node, "Column", true, []} = Enum.at(log, 1)
    assert {:end_node} = Enum.at(log, 2)
    assert length(log) == 3
  end

  test "run/3 returns {:ok, map}" do
    ir = make_ir("Foo", [], make_node("Box"))
    assert {:ok, %{log: _}} = MosaicVM.run(ir, MockRenderer, [])
  end

  # ---------------------------------------------------------------------------
  # Tests: nested children
  # ---------------------------------------------------------------------------

  test "nested child nodes — depth-first order" do
    child = make_node("Text")
    root = make_node("Column", children: [{:node, child}])
    ir = make_ir("Nested", [], root)
    log = run_mock(ir)

    assert {:begin_component, "Nested", []} = Enum.at(log, 0)
    assert {:begin_node, "Column", true, []} = Enum.at(log, 1)
    assert {:begin_node, "Text", true, []} = Enum.at(log, 2)
    assert {:end_node} = Enum.at(log, 3)   # Text closed
    assert {:end_node} = Enum.at(log, 4)   # Column closed
  end

  test "multiple children at same level — source order preserved" do
    root = make_node("Row", children: [
      {:node, make_node("Text")},
      {:node, make_node("Image")}
    ])
    ir = make_ir("Multi", [], root)
    log = run_mock(ir)

    node_types = for {:begin_node, tag, _, _} <- log, do: tag
    assert node_types == ["Row", "Text", "Image"]
  end

  # ---------------------------------------------------------------------------
  # Tests: slot ref as child
  # ---------------------------------------------------------------------------

  test "slot ref child — render_slot_child called with correct slot name" do
    slots = [make_slot("action", :node)]
    root = make_node("Column", children: [{:slot_ref, "action"}])
    ir = make_ir("WithSlot", slots, root)
    log = run_mock(ir)

    assert Enum.any?(log, fn
      {:render_slot_child, "action", %{kind: :node}} -> true
      _ -> false
    end)
  end

  test "slot ref child — slot type is passed to render_slot_child" do
    slots = [make_slot("header", :text)]
    root = make_node("Column", children: [{:slot_ref, "header"}])
    ir = make_ir("Header", slots, root)
    log = run_mock(ir)

    assert {:render_slot_child, "header", %{kind: :text}} in log
  end

  # ---------------------------------------------------------------------------
  # Tests: when blocks
  # ---------------------------------------------------------------------------

  test "when block — begin_when called before children, end_when after" do
    child = make_node("Text")
    root = make_node("Column", children: [{:when_block, "show", [{:node, child}]}])
    slots = [make_slot("show", :bool)]
    ir = make_ir("Conditional", slots, root)
    log = run_mock(ir)

    begin_idx = Enum.find_index(log, &match?({:begin_when, "show"}, &1))
    begin_node_idx = Enum.find_index(log, &match?({:begin_node, "Text", _, _}, &1))
    end_when_idx = Enum.find_index(log, &match?({:end_when}, &1))

    assert begin_idx < begin_node_idx
    assert begin_node_idx < end_when_idx
  end

  test "when block — empty body still emits begin/end_when" do
    root = make_node("Column", children: [{:when_block, "show", []}])
    slots = [make_slot("show", :bool)]
    ir = make_ir("EmptyWhen", slots, root)
    log = run_mock(ir)

    assert {:begin_when, "show"} in log
    assert {:end_when} in log
  end

  # ---------------------------------------------------------------------------
  # Tests: each blocks
  # ---------------------------------------------------------------------------

  test "each block — begin_each called before children, end_each after" do
    child = make_node("Text")
    list_type = %{kind: :list, element_type: %{kind: :text}}
    slots = [make_slot("items", list_type)]
    root = make_node("Column", children: [{:each_block, "items", "item", [{:node, child}]}])
    ir = make_ir("EachTest", slots, root)
    log = run_mock(ir)

    begin_each_idx = Enum.find_index(log, &match?({:begin_each, "items", "item"}, &1))
    begin_node_idx = Enum.find_index(log, &match?({:begin_node, "Text", _, _}, &1))
    end_each_idx = Enum.find_index(log, &match?({:end_each}, &1))

    assert begin_each_idx < begin_node_idx
    assert begin_node_idx < end_each_idx
  end

  test "each block — slot names and item names are passed" do
    list_type = %{kind: :list, element_type: %{kind: :number}}
    slots = [make_slot("scores", list_type)]
    root = make_node("Column", children: [{:each_block, "scores", "score", []}])
    ir = make_ir("ScoreList", slots, root)
    log = run_mock(ir)

    assert {:begin_each, "scores", "score"} in log
  end

  # ---------------------------------------------------------------------------
  # Tests: value normalization — string / number / bool / ident
  # ---------------------------------------------------------------------------

  test "string value passes through unchanged" do
    props = [%{name: "content", value: %{kind: :string, value: "hello"}}]
    ir = make_ir("S", [], make_node("Text", properties: props))
    log = run_mock(ir)

    [{:begin_node, "Text", _, resolved_props} | _] = Enum.drop_while(log, &(not match?({:begin_node, "Text", _, _}, &1)))
    assert [%{name: "content", value: %{kind: :string, value: "hello"}}] = resolved_props
  end

  test "number value passes through unchanged" do
    props = [%{name: "opacity", value: %{kind: :number, value: 0.5}}]
    ir = make_ir("N", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "opacity", value: %{kind: :number, value: 0.5}}] = resolved
  end

  test "bool value passes through unchanged" do
    props = [%{name: "visible", value: %{kind: :bool, value: false}}]
    ir = make_ir("B", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "visible", value: %{kind: :bool, value: false}}] = resolved
  end

  test "ident value is folded into string kind" do
    props = [%{name: "overflow", value: %{kind: :ident, value: "hidden"}}]
    ir = make_ir("I", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "overflow", value: %{kind: :string, value: "hidden"}}] = resolved
  end

  # ---------------------------------------------------------------------------
  # Tests: value normalization — color_hex
  # ---------------------------------------------------------------------------

  test "color_hex #rrggbb → resolved color with alpha 255" do
    props = [%{name: "background", value: %{kind: :color_hex, value: "#2563eb"}}]
    ir = make_ir("C1", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "background", value: %{kind: :color, r: 37, g: 99, b: 235, a: 255}}] = resolved
  end

  test "color_hex #rgb → each digit doubled, alpha 255" do
    props = [%{name: "color", value: %{kind: :color_hex, value: "#fff"}}]
    ir = make_ir("C2", [], make_node("Text", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Text", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "color", value: %{kind: :color, r: 255, g: 255, b: 255, a: 255}}] = resolved
  end

  test "color_hex #rrggbbaa → all four channels parsed" do
    props = [%{name: "color", value: %{kind: :color_hex, value: "#ffffffff"}}]
    ir = make_ir("C3", [], make_node("Text", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Text", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "color", value: %{kind: :color, r: 255, g: 255, b: 255, a: 255}}] = resolved
  end

  test "color_hex #00000000 → transparent black" do
    props = [%{name: "background", value: %{kind: :color_hex, value: "#00000000"}}]
    ir = make_ir("C4", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "background", value: %{kind: :color, r: 0, g: 0, b: 0, a: 0}}] = resolved
  end

  # ---------------------------------------------------------------------------
  # Tests: value normalization — dimension
  # ---------------------------------------------------------------------------

  test "dimension 16dp → resolved dimension with unit :dp" do
    props = [%{name: "padding", value: %{kind: :dimension, value: 16, unit: "dp"}}]
    ir = make_ir("D1", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "padding", value: %{kind: :dimension, value: 16, unit: :dp}}] = resolved
  end

  test "dimension 100% → resolved dimension with unit :percent" do
    props = [%{name: "width", value: %{kind: :dimension, value: 100, unit: "%"}}]
    ir = make_ir("D2", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "width", value: %{kind: :dimension, value: 100, unit: :percent}}] = resolved
  end

  test "dimension 1.5sp → resolved dimension with unit :sp" do
    props = [%{name: "gap", value: %{kind: :dimension, value: 1.5, unit: "sp"}}]
    ir = make_ir("D3", [], make_node("Row", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Row", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "gap", value: %{kind: :dimension, value: 1.5, unit: :sp}}] = resolved
  end

  # ---------------------------------------------------------------------------
  # Tests: slot_ref resolution — component slot
  # ---------------------------------------------------------------------------

  test "slot_ref value resolves to component slot with slot_type" do
    slots = [make_slot("title", :text)]
    props = [%{name: "content", value: %{kind: :slot_ref, slot_name: "title"}}]
    root = make_node("Text", properties: props)
    ir = make_ir("SlotRef", slots, root)
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Text", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "content", value: %{kind: :slot_ref, slot_name: "title", slot_type: %{kind: :text}, is_loop_var: false}}] = resolved
  end

  test "slot_ref to loop variable resolves with is_loop_var: true" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    slots = [make_slot("labels", list_type)]
    props = [%{name: "content", value: %{kind: :slot_ref, slot_name: "label"}}]
    inner_node = make_node("Text", properties: props)
    root = make_node("Column", children: [{:each_block, "labels", "label", [{:node, inner_node}]}])
    ir = make_ir("LoopVar", slots, root)
    log = run_mock(ir)

    begin_text = Enum.find(log, &match?({:begin_node, "Text", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_text
    assert [%{name: "content", value: %{kind: :slot_ref, slot_name: "label", is_loop_var: true}}] = resolved
  end

  # ---------------------------------------------------------------------------
  # Tests: enum values
  # ---------------------------------------------------------------------------

  test "enum value passes through unchanged" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "high"}}]
    ir = make_ir("E", [], make_node("Box", properties: props))
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Box", _, _}, &1))
    {:begin_node, _, _, resolved} = begin_node
    assert [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "high"}}] = resolved
  end

  # ---------------------------------------------------------------------------
  # Tests: non-primitive node
  # ---------------------------------------------------------------------------

  test "non-primitive node — is_primitive is false" do
    root = make_node("Button", is_primitive: false)
    ir = make_ir("Wrap", [], root)
    log = run_mock(ir)

    begin_node = Enum.find(log, &match?({:begin_node, "Button", _, _}, &1))
    assert {:begin_node, "Button", false, []} = begin_node
  end
end
