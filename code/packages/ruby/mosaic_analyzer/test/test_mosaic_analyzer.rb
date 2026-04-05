# frozen_string_literal: true

# ================================================================
# Tests for the Mosaic Analyzer
# ================================================================
#
# The analyzer walks a Mosaic AST and produces a typed MosaicIR.
#
# We test:
#   - Component name extraction
#   - Slot type resolution (text, number, bool, image, color, node)
#   - Slot with default values
#   - list<T> slot types
#   - Component type slots
#   - Node tree structure (tag, is_primitive)
#   - Property assignments (strings, dims, colors, slot refs, idents)
#   - slot_reference as children (@header;)
#   - when blocks
#   - each blocks
#   - Import declarations
# ================================================================

require "minitest/autorun"
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_mosaic_parser"
require "coding_adventures_mosaic_analyzer"

class TestMosaicAnalyzer < Minitest::Test
  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  def analyze(source)
    CodingAdventures::MosaicAnalyzer.analyze(source)
  end

  # ------------------------------------------------------------------
  # Version
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil CodingAdventures::MosaicAnalyzer::VERSION
  end

  # ------------------------------------------------------------------
  # Minimal component
  # ------------------------------------------------------------------

  def test_minimal_component_returns_ir
    ir = analyze("component Label { Text { } }")
    assert_kind_of CodingAdventures::MosaicAnalyzer::MosaicIR, ir
  end

  def test_component_name
    ir = analyze("component ProfileCard { Column { } }")
    assert_equal "ProfileCard", ir.component.name
  end

  def test_no_imports
    ir = analyze("component Label { Text { } }")
    assert_empty ir.imports
  end

  # ------------------------------------------------------------------
  # Slot type resolution
  # ------------------------------------------------------------------

  def test_slot_text_type
    ir = analyze("component C { slot title: text; Text { } }")
    slot = ir.component.slots.find { |s| s.name == "title" }
    refute_nil slot
    assert_equal({ kind: "text" }, slot.type)
  end

  def test_slot_number_type
    ir = analyze("component C { slot count: number; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "number" }, slot.type)
  end

  def test_slot_bool_type
    ir = analyze("component C { slot visible: bool; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "bool" }, slot.type)
  end

  def test_slot_image_type
    ir = analyze("component C { slot avatar: image; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "image" }, slot.type)
  end

  def test_slot_color_type
    ir = analyze("component C { slot bg: color; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "color" }, slot.type)
  end

  def test_slot_node_type
    ir = analyze("component C { slot content: node; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "node" }, slot.type)
  end

  def test_slot_component_type
    ir = analyze("component C { slot action: Button; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "component", name: "Button" }, slot.type)
  end

  def test_slot_list_type
    ir = analyze("component C { slot items: list<text>; Text { } }")
    slot = ir.component.slots.first
    assert_equal "list", slot.type[:kind]
    assert_equal({ kind: "text" }, slot.type[:element_type])
  end

  def test_slot_list_of_component
    ir = analyze("component C { slot buttons: list<Button>; Text { } }")
    slot = ir.component.slots.first
    assert_equal "list", slot.type[:kind]
    assert_equal "component", slot.type[:element_type][:kind]
    assert_equal "Button", slot.type[:element_type][:name]
  end

  # ------------------------------------------------------------------
  # Slot required/optional
  # ------------------------------------------------------------------

  def test_slot_without_default_is_required
    ir = analyze("component C { slot title: text; Text { } }")
    slot = ir.component.slots.first
    assert_equal true, slot.required
    assert_nil slot.default_value
  end

  def test_slot_with_default_is_optional
    ir = analyze("component C { slot count: number = 0; Text { } }")
    slot = ir.component.slots.first
    assert_equal false, slot.required
    refute_nil slot.default_value
    assert_equal "number", slot.default_value[:kind]
    assert_equal 0.0, slot.default_value[:value]
  end

  def test_slot_with_bool_default_true
    ir = analyze("component C { slot visible: bool = true; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "bool", value: true }, slot.default_value)
  end

  def test_slot_with_bool_default_false
    ir = analyze("component C { slot flag: bool = false; Text { } }")
    slot = ir.component.slots.first
    assert_equal({ kind: "bool", value: false }, slot.default_value)
  end

  def test_slot_with_string_default
    ir = analyze('component C { slot label: text = "hello"; Text { } }')
    slot = ir.component.slots.first
    assert_equal({ kind: "string", value: "hello" }, slot.default_value)
  end

  # ------------------------------------------------------------------
  # Node tree
  # ------------------------------------------------------------------

  def test_root_node_tag
    ir = analyze("component C { Column { } }")
    assert_equal "Column", ir.component.tree.tag
  end

  def test_primitive_node_flag
    ir = analyze("component C { Column { } }")
    assert_equal true, ir.component.tree.is_primitive
  end

  def test_non_primitive_node_flag
    ir = analyze("component C { Button { } }")
    assert_equal false, ir.component.tree.is_primitive
  end

  # ------------------------------------------------------------------
  # Property assignments
  # ------------------------------------------------------------------

  def test_string_property
    source = 'component C { Text { content: "hello"; } }'
    ir = analyze(source)
    prop = ir.component.tree.properties.find { |p| p.name == "content" }
    refute_nil prop
    assert_equal({ kind: "string", value: "hello" }, prop.value)
  end

  def test_dimension_property
    source = "component C { Column { padding: 16dp; } }"
    ir = analyze(source)
    prop = ir.component.tree.properties.find { |p| p.name == "padding" }
    refute_nil prop
    assert_equal "dimension", prop.value[:kind]
    assert_equal 16.0, prop.value[:value]
    assert_equal "dp", prop.value[:unit]
  end

  def test_color_property
    source = "component C { Column { background: #2563eb; } }"
    ir = analyze(source)
    prop = ir.component.tree.properties.find { |p| p.name == "background" }
    refute_nil prop
    assert_equal({ kind: "color_hex", value: "#2563eb" }, prop.value)
  end

  def test_slot_ref_property
    source = "component C { slot title: text; Text { content: @title; } }"
    ir = analyze(source)
    prop = ir.component.tree.properties.find { |p| p.name == "content" }
    refute_nil prop
    assert_equal({ kind: "slot_ref", slot_name: "title" }, prop.value)
  end

  def test_ident_property
    source = "component C { Column { align: center; } }"
    ir = analyze(source)
    prop = ir.component.tree.properties.find { |p| p.name == "align" }
    refute_nil prop
    assert_equal "ident", prop.value[:kind]
    assert_equal "center", prop.value[:value]
  end

  def test_enum_property
    source = "component C { Text { style: heading.large; } }"
    ir = analyze(source)
    prop = ir.component.tree.properties.find { |p| p.name == "style" }
    refute_nil prop
    assert_equal "enum", prop.value[:kind]
    assert_equal "heading", prop.value[:namespace]
    assert_equal "large", prop.value[:member]
  end

  # ------------------------------------------------------------------
  # Child nodes
  # ------------------------------------------------------------------

  def test_nested_child_node
    source = "component C { Column { Text { } } }"
    ir = analyze(source)
    child = ir.component.tree.children.first
    refute_nil child
    assert_equal "node", child[:kind]
    assert_equal "Text", child[:node].tag
  end

  # ------------------------------------------------------------------
  # Slot references as children
  # ------------------------------------------------------------------

  def test_slot_ref_child
    source = "component C { slot header: node; Column { @header; } }"
    ir = analyze(source)
    child = ir.component.tree.children.first
    refute_nil child
    assert_equal "slot_ref", child[:kind]
    assert_equal "header", child[:slot_name]
  end

  # ------------------------------------------------------------------
  # when blocks
  # ------------------------------------------------------------------

  def test_when_block
    source = <<~MOSAIC
      component C {
        slot show: bool;
        Column {
          when @show {
            Text { content: "visible"; }
          }
        }
      }
    MOSAIC
    ir = analyze(source)
    child = ir.component.tree.children.first
    refute_nil child
    assert_equal "when", child[:kind]
    assert_equal "show", child[:slot_name]
    refute_empty child[:children]
  end

  # ------------------------------------------------------------------
  # each blocks
  # ------------------------------------------------------------------

  def test_each_block
    source = <<~MOSAIC
      component List {
        slot items: list<text>;
        Column {
          each @items as item {
            Text { content: @item; }
          }
        }
      }
    MOSAIC
    ir = analyze(source)
    child = ir.component.tree.children.first
    refute_nil child
    assert_equal "each", child[:kind]
    assert_equal "items", child[:slot_name]
    assert_equal "item", child[:item_name]
    refute_empty child[:children]
  end

  # ------------------------------------------------------------------
  # Import declarations
  # ------------------------------------------------------------------

  def test_import_declaration
    source = <<~MOSAIC
      import Button from "./button.mosaic";
      component Card { Column { } }
    MOSAIC
    ir = analyze(source)
    assert_equal 1, ir.imports.length
    imp = ir.imports.first
    assert_equal "Button", imp.component_name
    assert_nil imp.alias
    assert_equal "./button.mosaic", imp.path
  end

  def test_import_with_alias
    source = <<~MOSAIC
      import Card as InfoCard from "./card.mosaic";
      component Page { Column { } }
    MOSAIC
    ir = analyze(source)
    imp = ir.imports.first
    assert_equal "Card", imp.component_name
    assert_equal "InfoCard", imp.alias
    assert_equal "./card.mosaic", imp.path
  end

  # ------------------------------------------------------------------
  # Multiple slots
  # ------------------------------------------------------------------

  def test_multiple_slots
    source = <<~MOSAIC
      component Card {
        slot title: text;
        slot count: number;
        slot visible: bool = true;
        Text { }
      }
    MOSAIC
    ir = analyze(source)
    assert_equal 3, ir.component.slots.length
    assert_equal "title", ir.component.slots[0].name
    assert_equal "count", ir.component.slots[1].name
    assert_equal "visible", ir.component.slots[2].name
    assert_equal false, ir.component.slots[2].required
  end

  # ------------------------------------------------------------------
  # Error cases
  # ------------------------------------------------------------------

  def test_analysis_error_on_invalid_mosaic
    # An invalid mosaic source causes a parse error which is raised
    assert_raises(StandardError) do
      analyze("not valid mosaic source text here")
    end
  end
end
