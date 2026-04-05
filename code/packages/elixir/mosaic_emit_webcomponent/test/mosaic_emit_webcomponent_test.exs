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
  defp make_slot(name, kind, default_value) when is_atom(kind) do
    %{name: name, type: %{kind: kind}, default_value: default_value}
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

  test "Row emits display:flex flex-direction:row" do
    ir = make_ir("R", [], make_node("Row"))
    content = first_file_content(ir)
    assert String.contains?(content, "flex-direction:row")
  end

  test "Box emits position:relative" do
    ir = make_ir("B", [], make_node("Box"))
    content = first_file_content(ir)
    assert String.contains?(content, "position:relative")
  end

  test "Text maps to span element" do
    ir = make_ir("T", [], make_node("Text"))
    content = first_file_content(ir)
    assert String.contains?(content, "<span")
  end

  test "Image is self-closing" do
    ir = make_ir("I", [], make_node("Image"))
    content = first_file_content(ir)
    assert String.contains?(content, "<img")
  end

  test "Spacer emits flex:1" do
    ir = make_ir("S", [], make_node("Spacer"))
    content = first_file_content(ir)
    assert String.contains?(content, "flex:1")
  end

  test "Scroll emits overflow:auto" do
    ir = make_ir("SC", [], make_node("Scroll"))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow:auto")
  end

  test "Divider emits hr with border styles" do
    ir = make_ir("D", [], make_node("Divider"))
    content = first_file_content(ir)
    assert String.contains?(content, "<hr")
    assert String.contains?(content, "border:none")
  end

  test "unknown primitive tag defaults to div" do
    ir = make_ir("UK", [], make_node("Unknown"))
    content = first_file_content(ir)
    assert String.contains?(content, "<div")
  end

  test "non-primitive node uses div as html tag" do
    ir = make_ir("Wrapper", [], make_node("MyButton", is_primitive: false))
    content = first_file_content(ir)
    assert String.contains?(content, "<div")
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

  test "image slot gets string backing field" do
    ir = make_ir("P", [make_slot("src", :image)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _src: string")
  end

  test "color slot gets string backing field" do
    ir = make_ir("P", [make_slot("tint", :color)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "private _tint: string")
  end

  test "node slot gets HTMLElement | null backing field" do
    ir = make_ir("P", [make_slot("action", :node)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "HTMLElement | null")
  end

  test "component slot gets HTMLElement | null backing field" do
    comp_type = %{kind: :component, name: "MyCard"}
    ir = make_ir("P", [make_slot("card", comp_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "HTMLElement | null")
  end

  test "list<text> slot gets string[] backing field" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    ir = make_ir("P", [make_slot("items", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "string[]")
  end

  test "list<number> slot gets number[] backing field" do
    list_type = %{kind: :list, element_type: %{kind: :number}}
    ir = make_ir("P", [make_slot("vals", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "number[]")
  end

  test "list<node> slot gets Element[] backing field" do
    list_type = %{kind: :list, element_type: %{kind: :node}}
    ir = make_ir("P", [make_slot("nodes", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "Element[]")
  end

  # ---------------------------------------------------------------------------
  # Tests: default values in backing fields
  # ---------------------------------------------------------------------------

  test "text slot with default value has default in field" do
    ir = make_ir("P", [make_slot("label", :text, %{kind: :string, value: "Click"})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "'Click'")
  end

  test "number slot with default value has default in field" do
    ir = make_ir("P", [make_slot("count", :number, %{kind: :number, value: 5})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "= 5")
  end

  test "bool slot with default value has default in field" do
    ir = make_ir("P", [make_slot("active", :bool, %{kind: :bool, value: true})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "= true")
  end

  # ---------------------------------------------------------------------------
  # Tests: observedAttributes
  # ---------------------------------------------------------------------------

  test "text slot is listed in observedAttributes" do
    ir = make_ir("P", [make_slot("title", :text)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "'title'")
    assert String.contains?(content, "observedAttributes")
  end

  test "number slot is listed in observedAttributes" do
    ir = make_ir("P", [make_slot("count", :number)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "'count'")
    assert String.contains?(content, "observedAttributes")
  end

  test "bool slot is listed in observedAttributes" do
    ir = make_ir("P", [make_slot("active", :bool)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "'active'")
    assert String.contains?(content, "observedAttributes")
  end

  test "node slot is NOT in observedAttributes" do
    ir = make_ir("P", [make_slot("action", :node)], make_node("Column"))
    content = first_file_content(ir)
    refute String.contains?(content, "observedAttributes(): string[] {\n    return ['action']")
  end

  test "attributeChangedCallback emitted when observable slots present" do
    ir = make_ir("P", [make_slot("title", :text)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "attributeChangedCallback")
  end

  test "number slot case in attributeChangedCallback uses parseFloat" do
    ir = make_ir("P", [make_slot("count", :number)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "parseFloat")
  end

  test "bool slot case in attributeChangedCallback uses value !== null" do
    ir = make_ir("P", [make_slot("active", :bool)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "value !== null")
  end

  # ---------------------------------------------------------------------------
  # Tests: property setters/getters
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

  test "list slot setter emits set without getter" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    ir = make_ir("P", [make_slot("items", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "set items(")
  end

  test "node slot setter emits _projectSlot" do
    ir = make_ir("P", [make_slot("action", :node)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "_projectSlot")
    assert String.contains?(content, "_projectSlot")
  end

  test "node slot triggers _projectSlot and disconnectedCallback" do
    ir = make_ir("P", [make_slot("action", :node)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "disconnectedCallback")
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

  test "each block with node list emits indexed slot forEach" do
    list_type = %{kind: :list, element_type: %{kind: :node}}
    slots = [make_slot("nodes", list_type)]
    inner = make_node("Column")
    root = make_node("Column", children: [{:each_block, "nodes", "node", [{:node, inner}]}])
    ir = make_ir("EN", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "_i)")
    assert String.contains?(content, "<slot name=")
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

  test "style: string triggers type scale CSS const" do
    props = [%{name: "style", value: %{kind: :string, value: "body-medium"}}]
    ir = make_ir("TS", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "MOSAIC_TYPE_SCALE_CSS")
    assert String.contains?(content, "mosaic-body-medium")
  end

  # ---------------------------------------------------------------------------
  # Tests: connectedCallback
  # ---------------------------------------------------------------------------

  test "file contains connectedCallback that calls _render" do
    ir = make_ir("Card", [], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "connectedCallback(): void { this._render(); }")
  end

  # ---------------------------------------------------------------------------
  # Tests: properties → CSS styles
  # ---------------------------------------------------------------------------

  test "padding maps to padding style" do
    props = [%{name: "padding", value: %{kind: :dimension, value: 16, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "padding:16px")
  end

  test "padding-left maps to padding-left style" do
    props = [%{name: "padding-left", value: %{kind: :dimension, value: 8, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "padding-left:8px")
  end

  test "padding-right maps to padding-right style" do
    props = [%{name: "padding-right", value: %{kind: :dimension, value: 8, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "padding-right:8px")
  end

  test "padding-top maps to padding-top style" do
    props = [%{name: "padding-top", value: %{kind: :dimension, value: 4, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "padding-top:4px")
  end

  test "padding-bottom maps to padding-bottom style" do
    props = [%{name: "padding-bottom", value: %{kind: :dimension, value: 4, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "padding-bottom:4px")
  end

  test "gap maps to gap style" do
    props = [%{name: "gap", value: %{kind: :dimension, value: 12, unit: :dp}}]
    ir = make_ir("P", [], make_node("Row", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "gap:12px")
  end

  test "width fill maps to 100%" do
    props = [%{name: "width", value: %{kind: :string, value: "fill"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "width:100%")
  end

  test "width wrap maps to fit-content" do
    props = [%{name: "width", value: %{kind: :string, value: "wrap"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "width:fit-content")
  end

  test "height dimension maps to height style" do
    props = [%{name: "height", value: %{kind: :dimension, value: 50, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "height:50px")
  end

  test "height percent dimension maps to percent value" do
    props = [%{name: "height", value: %{kind: :dimension, value: 100, unit: :percent}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "height:100%")
  end

  test "min-width maps to min-width style" do
    props = [%{name: "min-width", value: %{kind: :dimension, value: 100, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "min-width:100px")
  end

  test "max-width maps to max-width style" do
    props = [%{name: "max-width", value: %{kind: :dimension, value: 200, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "max-width:200px")
  end

  test "min-height maps to min-height style" do
    props = [%{name: "min-height", value: %{kind: :dimension, value: 50, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "min-height:50px")
  end

  test "max-height maps to max-height style" do
    props = [%{name: "max-height", value: %{kind: :dimension, value: 300, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "max-height:300px")
  end

  test "overflow visible maps to visible CSS" do
    props = [%{name: "overflow", value: %{kind: :string, value: "visible"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow:visible")
  end

  test "overflow hidden maps to hidden CSS" do
    props = [%{name: "overflow", value: %{kind: :string, value: "hidden"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow:hidden")
  end

  test "overflow scroll maps to auto CSS" do
    props = [%{name: "overflow", value: %{kind: :string, value: "scroll"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow:auto")
  end

  test "background color maps to background-color rgba" do
    props = [%{name: "background", value: %{kind: :color, r: 37, g: 99, b: 235, a: 255}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "background-color:rgba(37, 99, 235")
  end

  test "corner-radius maps to border-radius" do
    props = [%{name: "corner-radius", value: %{kind: :dimension, value: 8, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "border-radius:8px")
  end

  test "border-width maps to border-width and border-style" do
    props = [%{name: "border-width", value: %{kind: :dimension, value: 1, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "border-width:1px")
    assert String.contains?(content, "border-style:solid")
  end

  test "border-color maps to border-color rgba" do
    props = [%{name: "border-color", value: %{kind: :color, r: 0, g: 0, b: 0, a: 255}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "border-color:rgba(0, 0, 0,")
  end

  test "opacity maps to opacity style" do
    props = [%{name: "opacity", value: %{kind: :number, value: 0.5}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "opacity:0.5")
  end

  test "shadow low maps to box-shadow" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "low"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "box-shadow:0 1px 3px")
  end

  test "shadow high maps to box-shadow" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "high"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "box-shadow:0 8px 24px")
  end

  test "visible false maps to display:none" do
    props = [%{name: "visible", value: %{kind: :bool, value: false}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "display:none")
  end

  test "Text content with string emits static text" do
    props = [%{name: "content", value: %{kind: :string, value: "Hello"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "Hello")
  end

  test "Text content with slot_ref emits escapeHtml expression" do
    slots = [make_slot("label", :text)]
    props = [%{name: "content", value: %{kind: :slot_ref, slot_name: "label"}}]
    ir = make_ir("T", slots, make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "_escapeHtml(this._label)")
  end

  test "color property maps to color CSS" do
    props = [%{name: "color", value: %{kind: :color, r: 255, g: 0, b: 0, a: 255}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "color:rgba(255, 0, 0,")
  end

  test "text-align start maps to left CSS" do
    props = [%{name: "text-align", value: %{kind: :string, value: "start"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "text-align:left")
  end

  test "text-align center maps to center CSS" do
    props = [%{name: "text-align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "text-align:center")
  end

  test "text-align end maps to right CSS" do
    props = [%{name: "text-align", value: %{kind: :string, value: "end"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "text-align:right")
  end

  test "font-weight bold is preserved" do
    props = [%{name: "font-weight", value: %{kind: :string, value: "bold"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "font-weight:bold")
  end

  test "max-lines emits webkit line clamp styles" do
    props = [%{name: "max-lines", value: %{kind: :number, value: 3}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "-webkit-line-clamp:3")
    assert String.contains?(content, "overflow:hidden")
  end

  test "source string on Image emits src attribute" do
    props = [%{name: "source", value: %{kind: :string, value: "https://example.com/img.png"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "src=")
    assert String.contains?(content, "example.com")
  end

  test "source slot_ref on Image emits placeholder replaced with _validateUrl" do
    slots = [make_slot("src", :image)]
    props = [%{name: "source", value: %{kind: :slot_ref, slot_name: "src"}}]
    ir = make_ir("P", slots, make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "_validateUrl")
  end

  test "size on Image emits width and height" do
    props = [%{name: "size", value: %{kind: :dimension, value: 48, unit: :dp}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "width:48px")
    assert String.contains?(content, "height:48px")
  end

  test "shape circle on Image emits border-radius 50%" do
    props = [%{name: "shape", value: %{kind: :string, value: "circle"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "border-radius:50%")
  end

  test "shape rounded on Image emits border-radius 8px" do
    props = [%{name: "shape", value: %{kind: :string, value: "rounded"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "border-radius:8px")
  end

  test "fit on Image emits object-fit style" do
    props = [%{name: "fit", value: %{kind: :string, value: "cover"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "object-fit:cover")
  end

  test "a11y-label string emits aria-label attribute" do
    props = [%{name: "a11y-label", value: %{kind: :string, value: "Close"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-label=\"Close\"")
  end

  test "a11y-label slot_ref emits aria placeholder replaced with _escapeHtml" do
    slots = [make_slot("label", :text)]
    props = [%{name: "a11y-label", value: %{kind: :slot_ref, slot_name: "label"}}]
    ir = make_ir("P", slots, make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "_escapeHtml")
  end

  test "a11y-role: heading changes Text to h2" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "heading"}}]
    ir = make_ir("P", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "<h2")
    refute String.contains?(content, "<span")
  end

  test "a11y-role: none emits aria-hidden" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "none"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-hidden=\"true\"")
  end

  test "a11y-role: image emits role=img" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "image"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "role=\"img\"")
  end

  test "a11y-role arbitrary value emits role attribute" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "button"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "role=\"button\"")
  end

  test "a11y-hidden: true emits aria-hidden" do
    props = [%{name: "a11y-hidden", value: %{kind: :bool, value: true}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-hidden=\"true\"")
  end

  test "align Column center maps to align-items:center" do
    props = [%{name: "align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "align-items:center")
  end

  test "align Row end maps to align-items:flex-end" do
    props = [%{name: "align", value: %{kind: :string, value: "end"}}]
    ir = make_ir("P", [], make_node("Row", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "align-items:flex-end")
  end

  test "align Box center adds display:flex" do
    props = [%{name: "align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    # Box starts with position:relative; align adds display:flex
    assert String.contains?(content, "display:flex")
  end

  test "align Column center-vertical maps to justify-content:center" do
    props = [%{name: "align", value: %{kind: :string, value: "center-vertical"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "justify-content:center")
  end
end
