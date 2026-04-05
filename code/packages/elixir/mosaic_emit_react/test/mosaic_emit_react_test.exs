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

  test "Scroll maps to div with overflow auto" do
    ir = make_ir("SC", [], make_node("Scroll"))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow: \"auto\"")
  end

  test "unknown primitive tag defaults to div" do
    ir = make_ir("UK", [], make_node("Unknown"))
    content = first_file_content(ir)
    assert String.contains?(content, "<div")
  end

  # ---------------------------------------------------------------------------
  # Tests: non-primitive (component) nodes
  # ---------------------------------------------------------------------------

  test "non-primitive node uses component name as JSX tag" do
    ir = make_ir("Wrapper", [], make_node("MyButton", is_primitive: false))
    content = first_file_content(ir)
    assert String.contains?(content, "<MyButton")
  end

  test "non-primitive node triggers component import" do
    ir = make_ir("Wrapper", [], make_node("MyButton", is_primitive: false))
    content = first_file_content(ir)
    assert String.contains?(content, "import { MyButton } from \"./MyButton.js\";")
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

  test "color slot maps to string in props interface" do
    ir = make_ir("P", [make_slot("tint", :color)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "tint: string;")
  end

  test "component slot maps to ReactElement in props interface" do
    comp_type = %{kind: :component, name: "MyCard"}
    ir = make_ir("P", [make_slot("card", comp_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "React.ReactElement<MyCardProps>")
    assert String.contains?(content, "import type { MyCardProps } from \"./MyCard.js\";")
  end

  test "list<text> slot maps to string[] in props interface" do
    list_type = %{kind: :list, element_type: %{kind: :text}}
    ir = make_ir("P", [make_slot("items", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "items: string[];")
  end

  test "list<number> slot maps to number[] in props interface" do
    list_type = %{kind: :list, element_type: %{kind: :number}}
    ir = make_ir("P", [make_slot("vals", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "vals: number[];")
  end

  test "list<bool> slot maps to boolean[] in props interface" do
    list_type = %{kind: :list, element_type: %{kind: :bool}}
    ir = make_ir("P", [make_slot("flags", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "flags: boolean[];")
  end

  test "list<image> slot maps to string[] in props interface" do
    list_type = %{kind: :list, element_type: %{kind: :image}}
    ir = make_ir("P", [make_slot("srcs", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "srcs: string[];")
  end

  test "list<color> slot maps to string[] in props interface" do
    list_type = %{kind: :list, element_type: %{kind: :color}}
    ir = make_ir("P", [make_slot("colors", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "colors: string[];")
  end

  test "list<node> slot maps to React.ReactNode[]" do
    list_type = %{kind: :list, element_type: %{kind: :node}}
    ir = make_ir("P", [make_slot("nodes", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "nodes: React.ReactNode[];")
  end

  test "list<component> slot maps to Array<React.ReactElement>" do
    list_type = %{kind: :list, element_type: %{kind: :component, name: "Card"}}
    ir = make_ir("P", [make_slot("cards", list_type)], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "Array<React.ReactElement<CardProps>>")
  end

  test "slot with default string value gets optional marker and comment" do
    ir = make_ir("P", [make_slot("label", :text, %{kind: :string, value: "Click"})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "label?: string;")
    assert String.contains?(content, "// default: \"Click\"")
  end

  test "slot with default number value gets optional marker" do
    ir = make_ir("P", [make_slot("count", :number, %{kind: :number, value: 42})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "count?: number;")
    assert String.contains?(content, "// default: 42")
  end

  test "slot with default bool value gets optional marker" do
    ir = make_ir("P", [make_slot("active", :bool, %{kind: :bool, value: true})], make_node("Column"))
    content = first_file_content(ir)
    assert String.contains?(content, "active?: boolean;")
    assert String.contains?(content, "// default: true")
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

  test "padding-left dimension maps to paddingLeft style" do
    props = [%{name: "padding-left", value: %{kind: :dimension, value: 8, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "paddingLeft: \"8px\"")
  end

  test "padding-right dimension maps to paddingRight style" do
    props = [%{name: "padding-right", value: %{kind: :dimension, value: 8, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "paddingRight: \"8px\"")
  end

  test "padding-top dimension maps to paddingTop style" do
    props = [%{name: "padding-top", value: %{kind: :dimension, value: 4, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "paddingTop: \"4px\"")
  end

  test "padding-bottom dimension maps to paddingBottom style" do
    props = [%{name: "padding-bottom", value: %{kind: :dimension, value: 4, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "paddingBottom: \"4px\"")
  end

  test "gap dimension maps to gap style" do
    props = [%{name: "gap", value: %{kind: :dimension, value: 12, unit: :dp}}]
    ir = make_ir("P", [], make_node("Row", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "gap: \"12px\"")
  end

  test "width fill maps to 100%" do
    props = [%{name: "width", value: %{kind: :string, value: "fill"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "\"100%\"")
  end

  test "width wrap maps to fit-content" do
    props = [%{name: "width", value: %{kind: :string, value: "wrap"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "\"fit-content\"")
  end

  test "height dimension maps to height style" do
    props = [%{name: "height", value: %{kind: :dimension, value: 50, unit: :percent}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "\"50%\"")
  end

  test "min-width dimension maps to minWidth style" do
    props = [%{name: "min-width", value: %{kind: :dimension, value: 100, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "minWidth: \"100px\"")
  end

  test "max-width dimension maps to maxWidth style" do
    props = [%{name: "max-width", value: %{kind: :dimension, value: 200, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "maxWidth: \"200px\"")
  end

  test "min-height dimension maps to minHeight style" do
    props = [%{name: "min-height", value: %{kind: :dimension, value: 50, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "minHeight: \"50px\"")
  end

  test "max-height dimension maps to maxHeight style" do
    props = [%{name: "max-height", value: %{kind: :dimension, value: 300, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "maxHeight: \"300px\"")
  end

  test "overflow visible maps to visible" do
    props = [%{name: "overflow", value: %{kind: :string, value: "visible"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow: \"visible\"")
  end

  test "overflow hidden maps to hidden" do
    props = [%{name: "overflow", value: %{kind: :string, value: "hidden"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow: \"hidden\"")
  end

  test "overflow scroll maps to auto" do
    props = [%{name: "overflow", value: %{kind: :string, value: "scroll"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "overflow: \"auto\"")
  end

  test "background color maps to backgroundColor rgba" do
    props = [%{name: "background", value: %{kind: :color, r: 37, g: 99, b: 235, a: 255}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "backgroundColor: \"rgba(37, 99, 235, 1.0")
  end

  test "corner-radius maps to borderRadius" do
    props = [%{name: "corner-radius", value: %{kind: :dimension, value: 8, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "borderRadius: \"8px\"")
  end

  test "border-width maps to borderWidth and borderStyle" do
    props = [%{name: "border-width", value: %{kind: :dimension, value: 1, unit: :dp}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "borderWidth: \"1px\"")
    assert String.contains?(content, "borderStyle: \"solid\"")
  end

  test "border-color maps to borderColor rgba" do
    props = [%{name: "border-color", value: %{kind: :color, r: 0, g: 0, b: 0, a: 255}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "borderColor:")
  end

  test "opacity maps to opacity style" do
    props = [%{name: "opacity", value: %{kind: :number, value: 0.5}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "opacity: 0.5")
  end

  test "shadow low maps to box shadow" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "low"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "boxShadow:")
    assert String.contains?(content, "0 1px 3px")
  end

  test "shadow medium maps to box shadow" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "medium"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "0 4px 12px")
  end

  test "shadow high maps to box shadow" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "high"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "0 8px 24px")
  end

  test "shadow none maps to none" do
    props = [%{name: "shadow", value: %{kind: :enum, namespace: "elevation", member: "none"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "boxShadow: \"none\"")
  end

  test "visible false maps to display none" do
    props = [%{name: "visible", value: %{kind: :bool, value: false}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "display: \"none\"")
  end

  test "Text content with string literal renders inline" do
    props = [%{name: "content", value: %{kind: :string, value: "Hello"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "Hello")
    assert String.contains?(content, "<span")
  end

  test "Text content with slot_ref renders {slotName}" do
    slots = [make_slot("label", :text)]
    props = [%{name: "content", value: %{kind: :slot_ref, slot_name: "label"}}]
    ir = make_ir("T", slots, make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "{label}")
  end

  test "Text content with number renders number" do
    props = [%{name: "content", value: %{kind: :number, value: 42}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "42")
  end

  test "Text content with bool renders boolean" do
    props = [%{name: "content", value: %{kind: :bool, value: true}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "true")
  end

  test "color property maps to color style rgba" do
    props = [%{name: "color", value: %{kind: :color, r: 255, g: 0, b: 0, a: 255}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "color: \"rgba(255, 0, 0,")
  end

  test "text-align start maps to left" do
    props = [%{name: "text-align", value: %{kind: :string, value: "start"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "textAlign: \"left\"")
  end

  test "text-align center maps to center" do
    props = [%{name: "text-align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "textAlign: \"center\"")
  end

  test "text-align end maps to right" do
    props = [%{name: "text-align", value: %{kind: :string, value: "end"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "textAlign: \"right\"")
  end

  test "font-weight bold is preserved" do
    props = [%{name: "font-weight", value: %{kind: :string, value: "bold"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "fontWeight: \"bold\"")
  end

  test "font-weight 700 is preserved" do
    props = [%{name: "font-weight", value: %{kind: :string, value: "700"}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "fontWeight: \"700\"")
  end

  test "max-lines emits WebkitLineClamp and related styles" do
    props = [%{name: "max-lines", value: %{kind: :number, value: 3}}]
    ir = make_ir("T", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "WebkitLineClamp: 3")
    assert String.contains?(content, "overflow: \"hidden\"")
    assert String.contains?(content, "WebkitBoxOrient:")
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

  test "a11y-role: none emits aria-hidden" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "none"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-hidden=\"true\"")
  end

  test "a11y-role with arbitrary string emits role attribute" do
    props = [%{name: "a11y-role", value: %{kind: :string, value: "button"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "role=\"button\"")
  end

  test "a11y-label string emits aria-label attribute" do
    props = [%{name: "a11y-label", value: %{kind: :string, value: "Close"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-label=")
    assert String.contains?(content, "Close")
  end

  test "a11y-label slot_ref emits aria-label as JSX expression" do
    slots = [make_slot("label", :text)]
    props = [%{name: "a11y-label", value: %{kind: :slot_ref, slot_name: "label"}}]
    ir = make_ir("P", slots, make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "aria-label={label}")
  end

  # ---------------------------------------------------------------------------
  # Tests: Image properties
  # ---------------------------------------------------------------------------

  test "source string on Image emits src attribute" do
    props = [%{name: "source", value: %{kind: :string, value: "https://example.com/img.png"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "src=")
    assert String.contains?(content, "example.com")
  end

  test "source slot_ref on Image emits src JSX expression" do
    slots = [make_slot("src", :image)]
    props = [%{name: "source", value: %{kind: :slot_ref, slot_name: "src"}}]
    ir = make_ir("P", slots, make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "src={src}")
  end

  test "size on Image emits width and height" do
    props = [%{name: "size", value: %{kind: :dimension, value: 48, unit: :dp}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "width: \"48px\"")
    assert String.contains?(content, "height: \"48px\"")
  end

  test "shape circle on Image emits borderRadius 50%" do
    props = [%{name: "shape", value: %{kind: :string, value: "circle"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "borderRadius: \"50%\"")
  end

  test "shape rounded on Image emits borderRadius 8px" do
    props = [%{name: "shape", value: %{kind: :string, value: "rounded"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "borderRadius: \"8px\"")
  end

  test "fit on Image emits objectFit style" do
    props = [%{name: "fit", value: %{kind: :string, value: "cover"}}]
    ir = make_ir("P", [], make_node("Image", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "objectFit: \"cover\"")
  end

  # ---------------------------------------------------------------------------
  # Tests: align property
  # ---------------------------------------------------------------------------

  test "Column align center maps to alignItems center" do
    props = [%{name: "align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "alignItems: \"center\"")
  end

  test "Column align center-vertical maps to justifyContent center" do
    props = [%{name: "align", value: %{kind: :string, value: "center-vertical"}}]
    ir = make_ir("P", [], make_node("Column", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "justifyContent: \"center\"")
  end

  test "Row align center maps to both alignItems and justifyContent center" do
    props = [%{name: "align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("P", [], make_node("Row", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "alignItems: \"center\"")
    assert String.contains?(content, "justifyContent: \"center\"")
  end

  test "Box align center adds display flex and alignItems" do
    props = [%{name: "align", value: %{kind: :string, value: "center"}}]
    ir = make_ir("P", [], make_node("Box", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "display: \"flex\"")
    assert String.contains?(content, "alignItems: \"center\"")
  end

  test "Row align start maps to alignItems flex-start" do
    props = [%{name: "align", value: %{kind: :string, value: "start"}}]
    ir = make_ir("P", [], make_node("Row", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "alignItems: \"flex-start\"")
  end

  # ---------------------------------------------------------------------------
  # Tests: style property (type scale CSS)
  # ---------------------------------------------------------------------------

  test "style: enum property triggers type-scale CSS import" do
    props = [%{name: "style", value: %{kind: :enum, namespace: "heading", member: "large"}}]
    ir = make_ir("TS", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "import \"./mosaic-type-scale.css\";")
    assert String.contains?(content, "mosaic-heading-large")
  end

  test "style: string value triggers type-scale CSS import" do
    props = [%{name: "style", value: %{kind: :string, value: "body-medium"}}]
    ir = make_ir("TS", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "import \"./mosaic-type-scale.css\";")
    assert String.contains?(content, "mosaic-body-medium")
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

  test "when block with multiple children wraps in fragment" do
    child1 = make_node("Text")
    child2 = make_node("Text")
    slots = [%{name: "show", type: %{kind: :bool}, default_value: nil}]
    root = make_node("Column", children: [{:when_block, "show", [{:node, child1}, {:node, child2}]}])
    ir = make_ir("W2", slots, root)
    content = first_file_content(ir)
    assert String.contains?(content, "{show && (")
    assert String.contains?(content, "<>")
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
  # Tests: color format
  # ---------------------------------------------------------------------------

  test "color_hex #fff resolves and renders as rgba" do
    props = [%{name: "color", value: %{kind: :color, r: 255, g: 255, b: 255, a: 255}}]
    ir = make_ir("CF", [], make_node("Text", properties: props))
    content = first_file_content(ir)
    assert String.contains?(content, "rgba(255, 255, 255,")
  end

  # ---------------------------------------------------------------------------
  # Tests: nested nodes build correct JSX
  # ---------------------------------------------------------------------------

  test "nested Column inside Column renders nested divs" do
    inner = make_node("Column")
    outer = make_node("Column", children: [{:node, inner}])
    ir = make_ir("Nested", [], outer)
    content = first_file_content(ir)
    assert content |> String.split("<div") |> length() > 2
  end

  test "empty non-self-closing node renders as self-closing in JSX" do
    ir = make_ir("Empty", [], make_node("Column"))
    content = first_file_content(ir)
    # Column with no children and no text is self-closed
    assert String.contains?(content, "<div") or String.contains?(content, "/>")
  end
end
