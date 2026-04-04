defmodule CodingAdventures.MosaicEmitWebcomponentTest do
  use ExUnit.Case

  alias CodingAdventures.MosaicEmitWebcomponent
  alias CodingAdventures.MosaicVM

  # ---------------------------------------------------------------------------
  # IR builder helpers
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

  defp run(ir) do
    {:ok, result} = MosaicVM.run(ir, MosaicEmitWebcomponent, MosaicEmitWebcomponent.initial_state())
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
    assert Code.ensure_loaded?(CodingAdventures.MosaicEmitWebcomponent)
  end

  test "returns {:ok, result} with files list" do
    ir = make_ir("Card", [], make_node("Column"))
    {:ok, result} = MosaicVM.run(ir, MosaicEmitWebcomponent, MosaicEmitWebcomponent.initial_state())
    assert is_list(result.files)
    assert length(result.files) == 1
  end

  test "filename is mosaic-kebab-case.ts" do
    ir = make_ir("ProfileCard", [], make_node("Column"))
    result = run(ir)
    assert hd(result.files).filename == "mosaic-profile-card.ts"
  end

  test "PascalCase to kebab-case conversion for multi-word names" do
    ir = make_ir("HowItWorks", [], make_node("Column"))
    result = run(ir)
    assert hd(result.files).filename == "mosaic-how-it-works.ts"
  end

  test "file header contains auto-generated warning" do
    ir = make_ir("Button", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "AUTO-GENERATED from Button.mosaic")
  end

  test "file contains class extending HTMLElement" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "export class MosaicCardElement extends HTMLElement")
  end

  test "file contains customElements.define call" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "customElements.define('mosaic-card', MosaicCardElement)")
  end

  test "file contains shadow DOM attachment in constructor" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "attachShadow({ mode: 'open' })")
  end

  test "file contains _render() method" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _render(): void {")
  end

  test "file contains _escapeHtml() method" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _escapeHtml(s: string): string {")
  end

  # ---------------------------------------------------------------------------
  # Tests: primitive element mapping
  # ---------------------------------------------------------------------------

  test "Column emits display:flex flex-direction:column" do
    ir = make_ir("C", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "display:flex")
    assert String.contains?(content, "flex-direction:column")
  end

  test "Box emits position:relative" do
    ir = make_ir("B", [], make_node("Box"))
    content = first_file_content(ir)
    assert String.contains?(content, "position:relative")
  end

  # ---------------------------------------------------------------------------
  # Tests: slots → backing fields
  # ---------------------------------------------------------------------------

  test "text slot gets string backing field" do
    ir = make_ir("P", [make_slot("title", :text)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _title: string")
  end

  test "bool slot gets boolean backing field" do
    ir = make_ir("P", [make_slot("show", :bool)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _show: boolean")
  end

  test "number slot gets number backing field" do
    ir = make_ir("P", [make_slot("count", :number)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _count: number")
  end

  # ---------------------------------------------------------------------------
  # Tests: observedAttributes
  # ---------------------------------------------------------------------------

  test "observable slots are listed in observedAttributes" do
    ir = make_ir("P", [make_slot("title", :text)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "'title'")
    assert String.contains?(content, "observedAttributes")
  end

  test "node slot is NOT in observedAttributes" do
    ir = make_ir("P", [make_slot("action", :node)], make_node("Column"))
    content = first_file_content(ir)
    refute String.contains?(content, "observedAttributes(): string[] {\n    return ['action']")
  end

  # ---------------------------------------------------------------------------
  # Tests: property setters
  # ---------------------------------------------------------------------------

  test "text slot gets getter and setter" do
    ir = make_ir("P", [make_slot("label", :text)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "set label(")
    assert String.contains?(content, "get label()")
  end

  test "image slot setter validates javascript: scheme" do
    ir = make_ir("P", [make_slot("src", :image)], make_node("Image"))
    content = first_file_content(ir)
    assert String.contains?(content, "javascript:")
  end

  # ---------------------------------------------------------------------------
  # Tests: when block
  # ---------------------------------------------------------------------------

  test "when block emits if statement" do
    slots = [make_slot("show", :bool)]
    child = make_node("Text")
    root = make_node("Column", children: [{:when_block, "show", [{:node, child}]}])
    ir = make_ir("W", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "if (this._show) {")
  end

  # ---------------------------------------------------------------------------
  # Tests: each block
  # ---------------------------------------------------------------------------

  test "each block emits forEach loop" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    slots = [make_slot("items", list_type)]
    inner = make_node("Text")
    root = make_node("Column", children: [{:each_block, "items", "item", [{:node, inner}]}])
    ir = make_ir("E", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "this._items.forEach(item =>")
  end

  # ---------------------------------------------------------------------------
  # Tests: slot ref as child
  # ---------------------------------------------------------------------------

  test "slot ref child emits named slot element" do
    slots = [make_slot("action", :node)]
    root = make_node("Column", children: [{:slot_ref, "action"}])
    ir = make_ir("SR", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "<slot name=\"action\">")
  end

  # ---------------------------------------------------------------------------
  # Tests: style property → type scale CSS
  # ---------------------------------------------------------------------------

  test "style: enum triggers type scale CSS const" do
    props = [%{name: "style", value: %{kind: :enum, namespace: "heading", member: "large"}}]
    ir = make_ir("TS", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "MOSAIC_TYPE_SCALE_CSS")
    assert String.contains?(content, "mosaic-heading-large")
  end

  # ---------------------------------------------------------------------------
  # Tests: connectedCallback
  # ---------------------------------------------------------------------------

  test "file contains connectedCallback that calls _render" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "connectedCallback(): void { this._render(); }")
  end
end
