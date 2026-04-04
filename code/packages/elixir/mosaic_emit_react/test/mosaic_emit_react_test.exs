defmodule CodingAdventures.MosaicEmitReactTest do
  use ExUnit.Case

  alias CodingAdventures.MosaicEmitReact
  alias CodingAdventures.MosaicVM

  # ---------------------------------------------------------------------------
  # IR builder helpers (mirrors MosaicAnalyzer output shape)
  # ---------------------------------------------------------------------------

  defp make_node(tag, opts \\ []) do
    %{
      tag: tag,
      is_primitive: Keyword.get(opts, :is_primitive, true),
      properties: Keyword.get(opts, :properties, []),
      children: Keyword.get(opts, :children, [])
    }
  end

  defp make_slot(name, type_map) when is_map(type_map) do
    %{name: name, type: type_map, default_value: nil}
  end
  defp make_slot(name, kind) when is_atom(kind) do
    %{name: name, type: %{kind: kind}, default_value: nil}
  end
  defp make_slot(name, kind, default_value) do
    %{name: name, type: %{kind: kind}, default_value: default_value}
  end

  defp make_ir(component_name, slots, tree) do
    %{component: %{name: component_name, slots: slots, tree: tree}}
  end

  defp run(ir) do
    {:ok, result} = MosaicVM.run(ir, MosaicEmitReact, MosaicEmitReact.initial_state())
    result
  end

  defp first_file_content(ir) do
    result = run(ir)
    hd(result.files).content
  end

  # ---------------------------------------------------------------------------
  # Tests: basic structure
  # ---------------------------------------------------------------------------

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicEmitReact)
  end

  test "returns {:ok, result} with files list" do
    ir = make_ir("Simple", [], make_node("Column"))
    {:ok, result} = MosaicVM.run(ir, MosaicEmitReact, MosaicEmitReact.initial_state())
    assert is_list(result.files)
    assert length(result.files) == 1
  end

  test "filename is ComponentName.tsx" do
    ir = make_ir("ProfileCard", [], make_node("Column"))
    result = run(ir)
    assert hd(result.files).filename == "ProfileCard.tsx"
  end

  test "file header contains auto-generated warning" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "AUTO-GENERATED from Card.mosaic")
  end

  test "file includes React import" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "import React from \"react\";")
  end

  test "file includes props interface" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "interface CardProps {")
  end

  test "file includes exported function" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "export function Card({")
  end

  # ---------------------------------------------------------------------------
  # Tests: primitive element mapping
  # ---------------------------------------------------------------------------

  test "Column maps to div with flex column styles" do
    ir = make_ir("C", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "display: \"flex\"")
    assert String.contains?(content, "flexDirection: \"column\"")
  end

  test "Row maps to div with flex row styles" do
    ir = make_ir("R", [], make_node("Row"))
    content = first_file_content(ir)
    assert String.contains?(content, "flexDirection: \"row\"")
  end

  test "Box maps to div with position relative" do
    ir = make_ir("B", [], make_node("Box"))
    content = first_file_content(ir)
    assert String.contains?(content, "position: \"relative\"")
  end

  test "Text maps to span" do
    ir = make_ir("T", [], make_node("Text"))
    content = first_file_content(ir)
    assert String.contains?(content, "<span")
  end

  test "Image maps to self-closing img" do
    ir = make_ir("I", [], make_node("Image"))
    content = first_file_content(ir)
    assert String.contains?(content, "<img")
  end

  test "Spacer maps to div with flex 1" do
    ir = make_ir("S", [], make_node("Spacer"))
    content = first_file_content(ir)
    assert String.contains?(content, "flex: 1")
  end

  test "Divider maps to self-closing hr with border styles" do
    ir = make_ir("D", [], make_node("Divider"))
    content = first_file_content(ir)
    assert String.contains?(content, "<hr")
    assert String.contains?(content, "border:")
  end

  # ---------------------------------------------------------------------------
  # Tests: slots → props interface
  # ---------------------------------------------------------------------------

  test "text slot maps to string in props interface" do
    ir = make_ir("P", [make_slot("title", :text)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "title: string;")
  end

  test "number slot maps to number in props interface" do
    ir = make_ir("P", [make_slot("count", :number)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "count: number;")
  end

  test "bool slot maps to boolean in props interface" do
    ir = make_ir("P", [make_slot("show", :bool)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "show: boolean;")
  end

  test "node slot maps to React.ReactNode in props interface" do
    ir = make_ir("P", [make_slot("children", :node)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "children: React.ReactNode;")
  end

  test "image slot maps to string in props interface" do
    ir = make_ir("P", [make_slot("avatar", :image)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "avatar: string;")
  end

  test "list<text> slot maps to string[] in props interface" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    ir = make_ir("P", [make_slot("items", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "items: string[];")
  end

  test "slot with default value gets optional marker and comment" do
    ir = make_ir("P", [make_slot("label", :text, %{kind: :string, value: "Click"})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "label?: string;")
    assert String.contains?(content, "// default: \"Click\"")
  end

  # ---------------------------------------------------------------------------
  # Tests: property → style mapping
  # ---------------------------------------------------------------------------

  test "padding dimension maps to padding style" do
    props = [%{name: "padding", value: %{kind: :dimension, value: 16, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "padding: \"16px\"")
  end

  test "background color maps to backgroundColor rgba" do
    props = [%{name: "background", value: %{kind: :color, r: 37, g: 99, b: 235, a: 255}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "backgroundColor: \"rgba(37, 99, 235, 1.0")
  end

  test "a11y-role: heading changes Text tag to h2" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "heading"}}]
    ir = make_ir("P", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "<h2")
    refute String.contains?(content, "<span")
  end

  test "a11y-hidden: true emits aria-hidden" do
    props = [%{name: "a11y-hidden", value: %{kind: :bool, value: true}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-hidden=\"true\"")
  end

  test "a11y-role: image emits role=\"img\"" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "image"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "role=\"img\"")
  end

  # ---------------------------------------------------------------------------
  # Tests: conditional (when) block
  # ---------------------------------------------------------------------------

  test "when block emits JSX conditional expression" do
    child = make_node("Text")
    slots = [%{name: "show", type: %{kind: :bool}, default_value: nil}]
    root = make_node("Column", children: [{:when_block, "show", [{:node, child}]}])
    ir = make_ir("W", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "{show && (")
  end

  # ---------------------------------------------------------------------------
  # Tests: each block
  # ---------------------------------------------------------------------------

  test "each block emits .map() expression with item name" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    slots = [make_slot("labels", list_type)]
    inner = make_node("Text")
    root = make_node("Column", children: [{:each_block, "labels", "label", [{:node, inner}]}])
    ir = make_ir("E", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "labels.map((label, _index)")
  end

  test "each block wraps children in React.Fragment with key" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    slots = [make_slot("items", list_type)]
    inner = make_node("Text")
    root = make_node("Column", children: [{:each_block, "items", "item", [{:node, inner}]}])
    ir = make_ir("E2", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "React.Fragment key={_index}")
  end

  # ---------------------------------------------------------------------------
  # Tests: slot ref as child
  # ---------------------------------------------------------------------------

  test "slot ref child renders as {slotName} in JSX" do
    slots = [%{name: "action", type: %{kind: :node}, default_value: nil}]
    root = make_node("Column", children: [{:slot_ref, "action"}])
    ir = make_ir("SR", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "{action}")
  end

  # ---------------------------------------------------------------------------
  # Tests: type-scale CSS import
  # ---------------------------------------------------------------------------

  test "style: enum property triggers type-scale CSS import" do
    props = [%{name: "style", value: %{kind: :enum, namespace: "heading", member: "large"}}]
    ir = make_ir("TS", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "import \"./mosaic-type-scale.css\";")
    assert String.contains?(content, "mosaic-heading-large")
  end

  # ---------------------------------------------------------------------------
  # Tests: color format
  # ---------------------------------------------------------------------------

  test "color_hex #fff resolves and renders as rgba" do
    props = [%{name: "color", value: %{kind: :color, r: 255, g: 255, b: 255, a: 255}}]
    ir = make_ir("CF", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "rgba(255, 255, 255,")
  end
end
