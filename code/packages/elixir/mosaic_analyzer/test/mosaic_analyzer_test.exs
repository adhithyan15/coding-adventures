defmodule CodingAdventures.MosaicAnalyzerTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.MosaicAnalyzer
  alias CodingAdventures.MosaicAnalyzer.{MosaicComponent, MosaicSlot, MosaicNode, MosaicImport}

  # ===========================================================================
  # Helper — parse and assert success
  # ===========================================================================

  defp ok!(source) do
    case MosaicAnalyzer.analyze(source) do
      {:ok, component} -> component
      {:error, msg} -> flunk("Expected {:ok, component}, got {:error, #{inspect(msg)}}")
    end
  end

  # ===========================================================================
  # 1. Module loads
  # ===========================================================================

  test "module loads" do
    assert Code.ensure_loaded?(CodingAdventures.MosaicAnalyzer)
  end

  # ===========================================================================
  # 2. Minimal component
  # ===========================================================================

  test "analyze/1 — minimal component with no slots" do
    # The simplest possible component: just a Box with no slots or properties.
    component = ok!("component Foo { Box { } }")

    assert %MosaicComponent{} = component
    assert component.name == "Foo"
    assert component.slots == []
    assert component.imports == []
    assert %MosaicNode{} = component.root
    assert component.root.node_type == "Box"
    assert component.root.is_primitive == true
    assert component.root.properties == []
    assert component.root.children == []
  end

  test "analyze/1 — returns {:ok, component} tuple" do
    result = MosaicAnalyzer.analyze("component Bar { Text { } }")
    assert {:ok, %MosaicComponent{name: "Bar"}} = result
  end

  test "analyze/1 — error on syntactically invalid source" do
    # Missing closing brace — parser should fail.
    result = MosaicAnalyzer.analyze("component Foo { Box {")
    assert {:error, _msg} = result
  end

  # ===========================================================================
  # 3. Slot declarations — primitive types
  # ===========================================================================

  test "slot declaration — text type" do
    component = ok!("""
    component Card {
      slot title: text;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "title", type: {:primitive, "text"}, required: true}] =
             component.slots
  end

  test "slot declaration — number type" do
    component = ok!("""
    component Counter {
      slot count: number;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "count", type: {:primitive, "number"}, required: true}] =
             component.slots
  end

  test "slot declaration — bool type" do
    component = ok!("""
    component Toggle {
      slot visible: bool;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "visible", type: {:primitive, "bool"}, required: true}] =
             component.slots
  end

  test "slot declaration — image type" do
    component = ok!("""
    component Avatar {
      slot avatar-url: image;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "avatar-url", type: {:primitive, "image"}, required: true}] =
             component.slots
  end

  test "slot declaration — color type" do
    component = ok!("""
    component Badge {
      slot bg-color: color;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "bg-color", type: {:primitive, "color"}, required: true}] =
             component.slots
  end

  test "slot declaration — node type (flexible)" do
    component = ok!("""
    component Wrapper {
      slot content: node;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "content", type: {:primitive, "node"}, required: true}] =
             component.slots
  end

  # ===========================================================================
  # 4. List slot types
  # ===========================================================================

  test "slot declaration — list<text> type" do
    component = ok!("""
    component List {
      slot items: list<text>;
      Column { }
    }
    """)

    assert [%MosaicSlot{name: "items", type: {:list, {:primitive, "text"}}, required: true}] =
             component.slots
  end

  test "slot declaration — list<node> type" do
    component = ok!("""
    component Shelf {
      slot cards: list<node>;
      Row { }
    }
    """)

    assert [%MosaicSlot{name: "cards", type: {:list, {:primitive, "node"}}, required: true}] =
             component.slots
  end

  # ===========================================================================
  # 5. Component slot type (from imports)
  # ===========================================================================

  test "slot declaration — component type" do
    component = ok!("""
    component Page {
      slot action: Button;
      Box { }
    }
    """)

    assert [%MosaicSlot{name: "action", type: {:component, "Button"}, required: true}] =
             component.slots
  end

  # ===========================================================================
  # 6. Slot default values
  # ===========================================================================

  test "slot with number default value" do
    component = ok!("""
    component Counter {
      slot count: number = 0;
      Box { }
    }
    """)

    [slot] = component.slots
    assert slot.required == false
    assert slot.default_value == {:number, 0.0}
  end

  test "slot with bool default value — true" do
    component = ok!("""
    component Toggle {
      slot visible: bool = true;
      Box { }
    }
    """)

    [slot] = component.slots
    assert slot.required == false
    assert slot.default_value == {:bool, true}
  end

  test "slot with bool default value — false" do
    component = ok!("""
    component Toggle {
      slot visible: bool = false;
      Box { }
    }
    """)

    [slot] = component.slots
    assert slot.default_value == {:bool, false}
  end

  # ===========================================================================
  # 7. Property assignments
  # ===========================================================================

  test "property assignment — dimension value" do
    component = ok!("""
    component Padded {
      Box {
        padding: 16dp;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "padding"
    assert prop.value == {:dimension, 16.0, "dp"}
  end

  test "property assignment — slot_ref value" do
    component = ok!("""
    component Label {
      slot title: text;
      Text {
        content: @title;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "content"
    assert prop.value == {:slot_ref, "title"}
  end

  test "property assignment — hex color value" do
    component = ok!("""
    component Colored {
      Box {
        background: #2563eb;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "background"
    assert prop.value == {:color_hex, "#2563eb"}
  end

  test "property assignment — ident value (bare keyword)" do
    component = ok!("""
    component Aligned {
      Box {
        align: center;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "align"
    assert prop.value == {:ident, "center"}
  end

  test "property assignment — number value" do
    component = ok!("""
    component Opaque {
      Box {
        opacity: 0.5;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "opacity"
    assert prop.value == {:number, 0.5}
  end

  # ===========================================================================
  # 8. Nested nodes
  # ===========================================================================

  test "nested node children" do
    component = ok!("""
    component Layout {
      Column {
        Row { }
        Text { }
      }
    }
    """)

    root = component.root
    assert root.node_type == "Column"
    assert length(root.children) == 2
    [{:node, row}, {:node, text}] = root.children
    assert row.node_type == "Row"
    assert text.node_type == "Text"
  end

  # ===========================================================================
  # 9. When blocks
  # ===========================================================================

  test "when block in node children" do
    component = ok!("""
    component Conditional {
      slot show-header: bool;
      Column {
        when @show-header {
          Text { }
        }
      }
    }
    """)

    [{:when_block, slot_name, block_children}] = component.root.children
    assert slot_name == "show-header"
    assert [{:node, %MosaicNode{node_type: "Text"}}] = block_children
  end

  # ===========================================================================
  # 10. Each blocks
  # ===========================================================================

  test "each block in node children" do
    component = ok!("""
    component ItemList {
      slot items: list<text>;
      Column {
        each @items as item {
          Row { }
        }
      }
    }
    """)

    [{:each_block, slot_name, item_name, block_children}] = component.root.children
    assert slot_name == "items"
    assert item_name == "item"
    assert [{:node, %MosaicNode{node_type: "Row"}}] = block_children
  end

  # ===========================================================================
  # 11. Slot references as children
  # ===========================================================================

  test "slot reference used as child" do
    component = ok!("""
    component Page {
      slot header: node;
      Column {
        @header;
      }
    }
    """)

    [{:slot_ref_child, slot_name}] = component.root.children
    assert slot_name == "header"
  end

  # ===========================================================================
  # 12. Import collection
  # ===========================================================================

  test "import declaration is collected" do
    # A non-primitive node type (Button) should appear in imports.
    component = ok!("""
    import Button from "./button.mosaic";
    component Page {
      Box { }
    }
    """)

    assert [%MosaicImport{name: "Button"}] = component.imports
  end

  test "import with alias uses alias as name" do
    component = ok!("""
    import Card as InfoCard from "./card.mosaic";
    component Page {
      Box { }
    }
    """)

    assert [%MosaicImport{name: "InfoCard"}] = component.imports
  end

  # ===========================================================================
  # 13. is_primitive flag on nodes
  # ===========================================================================

  test "primitive nodes have is_primitive true" do
    # These are all the standard primitive nodes.
    primitives = ["Row", "Column", "Box", "Stack", "Text", "Image", "Icon", "Spacer", "Divider", "Scroll"]

    for tag <- primitives do
      component = ok!("component Test { #{tag} { } }")
      assert component.root.is_primitive == true,
             "Expected #{tag} to be primitive"
    end
  end

  test "non-primitive node type has is_primitive false" do
    component = ok!("component Page { Button { } }")
    assert component.root.is_primitive == false
  end

  # ===========================================================================
  # 14. Multiple slots
  # ===========================================================================

  test "multiple slot declarations are collected in order" do
    component = ok!("""
    component Card {
      slot title: text;
      slot subtitle: text;
      slot count: number;
      Box { }
    }
    """)

    assert length(component.slots) == 3
    [s1, s2, s3] = component.slots
    assert s1.name == "title"
    assert s2.name == "subtitle"
    assert s3.name == "count"
  end

  # ===========================================================================
  # 15. analyze_ast/1 with pre-parsed AST
  # ===========================================================================

  test "analyze_ast/1 accepts a pre-parsed file ASTNode" do
    {:ok, ast} = CodingAdventures.MosaicParser.parse("component Foo { Box { } }")
    result = MosaicAnalyzer.analyze_ast(ast)
    assert {:ok, %MosaicComponent{name: "Foo"}} = result
  end

  test "analyze_ast/1 returns error for wrong root rule" do
    fake_ast = %CodingAdventures.Parser.ASTNode{rule_name: "not_a_file", children: []}
    assert {:error, msg} = MosaicAnalyzer.analyze_ast(fake_ast)
    assert msg =~ "file"
  end

  # ===========================================================================
  # 16. Property values — all remaining value kinds
  # ===========================================================================

  test "property assignment — string value" do
    component = ok!("""
    component Named {
      Text {
        label: "Hello";
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "label"
    assert prop.value == {:string, "Hello"}
  end

  test "property assignment — bool true value" do
    component = ok!("""
    component Check {
      Box {
        visible: true;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.value == {:bool, true}
  end

  test "property assignment — bool false value" do
    component = ok!("""
    component Check {
      Box {
        visible: false;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.value == {:bool, false}
  end

  test "property assignment — enum value (namespace.member)" do
    component = ok!("""
    component Aligned {
      Box {
        style: heading.small;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "style"
    assert prop.value == {:enum_val, "heading", "small"}
  end

  # ===========================================================================
  # 17. Default values — additional types
  # ===========================================================================

  test "slot with string default value" do
    component = ok!("""
    component Welcome {
      slot greeting: text = "Hello, World!";
      Text { content: @greeting; }
    }
    """)

    [slot] = component.slots
    assert slot.required == false
    assert slot.default_value == {:string, "Hello, World!"}
  end

  test "slot with color_hex default value" do
    component = ok!("""
    component Themed {
      slot accent: color = #ff5733;
      Box { }
    }
    """)

    [slot] = component.slots
    assert slot.required == false
    assert slot.default_value == {:color_hex, "#ff5733"}
  end

  test "slot with dimension default value" do
    component = ok!("""
    component Spaced {
      slot gap: number = 8;
      Box { }
    }
    """)

    [slot] = component.slots
    assert slot.required == false
    assert slot.default_value == {:number, 8.0}
  end

  # ===========================================================================
  # 18. Multiple children in when/each blocks
  # ===========================================================================

  test "when block with multiple child nodes" do
    component = ok!("""
    component MultiWhen {
      slot show: bool;
      Column {
        when @show {
          Text { }
          Row { }
        }
      }
    }
    """)

    [{:when_block, "show", children}] = component.root.children
    assert length(children) == 2
    [{:node, %MosaicNode{node_type: "Text"}}, {:node, %MosaicNode{node_type: "Row"}}] = children
  end

  test "each block with nested node content" do
    component = ok!("""
    component DetailList {
      slot entries: list<text>;
      Column {
        each @entries as entry {
          Row {
            Text { content: @entry; }
          }
        }
      }
    }
    """)

    [{:each_block, "entries", "entry", [{:node, row}]}] = component.root.children
    assert row.node_type == "Row"
    [{:node, text}] = row.children
    assert text.node_type == "Text"
  end

  # ===========================================================================
  # 19. Scroll, Stack, Icon, Spacer, Divider — full primitive list
  # ===========================================================================

  test "Scroll is a primitive node" do
    component = ok!("component S { Scroll { } }")
    assert component.root.is_primitive == true
  end

  test "Stack is a primitive node" do
    component = ok!("component S { Stack { } }")
    assert component.root.is_primitive == true
  end

  test "Spacer is a primitive node" do
    component = ok!("component S { Spacer { } }")
    assert component.root.is_primitive == true
  end

  test "Divider is a primitive node" do
    component = ok!("component S { Divider { } }")
    assert component.root.is_primitive == true
  end

  test "Icon is a primitive node" do
    component = ok!("component S { Icon { } }")
    assert component.root.is_primitive == true
  end

  # ===========================================================================
  # 20. Property with keyword as property name (e.g., "color: ...")
  # ===========================================================================

  test "property with keyword as property name" do
    # 'color' is a KEYWORD but is also valid as a property name (e.g., SVG color attr)
    component = ok!("""
    component Colorful {
      Box {
        color: #abc123;
      }
    }
    """)

    [prop] = component.root.properties
    assert prop.name == "color"
    assert prop.value == {:color_hex, "#abc123"}
  end

  # ===========================================================================
  # 21. list<node> slots work end to end
  # ===========================================================================

  test "list<node> slot type with each block" do
    component = ok!("""
    component NodeList {
      slot items: list<node>;
      Column {
        each @items as item {
          Box { }
        }
      }
    }
    """)

    [slot] = component.slots
    assert slot.type == {:list, {:primitive, "node"}}
    [{:each_block, "items", "item", _}] = component.root.children
  end
end
