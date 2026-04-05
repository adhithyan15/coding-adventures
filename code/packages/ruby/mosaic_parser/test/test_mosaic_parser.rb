# frozen_string_literal: true

# ================================================================
# Tests for the Mosaic Parser
# ================================================================
#
# These tests verify that the grammar-driven parser, when fed
# mosaic.grammar and a mosaic.tokens token stream, correctly
# builds an AST for Mosaic source text.
#
# We are not testing the parser engine (tested in the parser gem)
# but verifying that the Mosaic grammar correctly structures:
#
#   - File-level imports and component declaration
#   - Slot declarations with types and optional defaults
#   - Node trees with property assignments
#   - Slot references (@slotName)
#   - when blocks (conditional rendering)
#   - each blocks (iteration)
# ================================================================

require "minitest/autorun"
require "coding_adventures_grammar_tools"
require "coding_adventures_lexer"
require "coding_adventures_parser"
require "coding_adventures_mosaic_lexer"
require "coding_adventures_mosaic_parser"

class TestMosaicParser < Minitest::Test
  # ------------------------------------------------------------------
  # Helpers
  # ------------------------------------------------------------------

  def parse(source)
    CodingAdventures::MosaicParser.parse(source)
  end

  # Walk the AST collecting all rule names
  def rule_names(node)
    names = [node.rule_name]
    node.children.each do |child|
      names.concat(rule_names(child)) if child.respond_to?(:rule_name)
    end
    names
  end

  # Find first child ASTNode with a given rule_name
  def find_child(node, rule)
    node.children.find { |c| c.respond_to?(:rule_name) && c.rule_name == rule }
  end

  # Collect all ASTNode children with a given rule_name (direct only)
  def find_children(node, rule)
    node.children.select { |c| c.respond_to?(:rule_name) && c.rule_name == rule }
  end

  # Collect all token values of a given type from direct children
  def token_values(node, type)
    node.children
      .reject { |c| c.respond_to?(:rule_name) }
      .select { |c| c.type == type }
      .map(&:value)
  end

  # ------------------------------------------------------------------
  # Version and grammar path
  # ------------------------------------------------------------------

  def test_version_exists
    refute_nil CodingAdventures::MosaicParser::VERSION
  end

  def test_grammar_path_exists
    assert File.exist?(CodingAdventures::MosaicParser::MOSAIC_GRAMMAR_PATH),
      "mosaic.grammar should exist"
  end

  # ------------------------------------------------------------------
  # Minimal component
  # ------------------------------------------------------------------

  def test_minimal_component_parses
    source = 'component Label { Text { } }'
    ast = parse(source)
    assert_equal "file", ast.rule_name
  end

  def test_file_contains_component_decl
    source = 'component Label { Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    refute_nil comp, "Expected component_decl in file"
  end

  def test_component_name_token
    source = 'component ProfileCard { Column { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    names = token_values(comp, "NAME")
    assert_includes names, "ProfileCard"
  end

  # ------------------------------------------------------------------
  # Slot declarations
  # ------------------------------------------------------------------

  def test_slot_declaration
    source = 'component Card { slot title: text; Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    slot = find_child(comp, "slot_decl")
    refute_nil slot, "Expected slot_decl"
  end

  def test_slot_name_and_type
    source = 'component Card { slot title: text; Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    slot = find_child(comp, "slot_decl")
    names = token_values(slot, "NAME")
    assert_includes names, "title"
    slot_type = find_child(slot, "slot_type")
    refute_nil slot_type
  end

  def test_multiple_slots
    source = <<~MOSAIC
      component Card {
        slot title: text;
        slot count: number;
        Text { }
      }
    MOSAIC
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    slots = find_children(comp, "slot_decl")
    assert_equal 2, slots.length
  end

  def test_slot_with_default_value
    source = 'component Card { slot count: number = 0; Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    slot = find_child(comp, "slot_decl")
    default_val = find_child(slot, "default_value")
    refute_nil default_val, "Expected default_value in slot"
  end

  def test_slot_bool_type
    source = 'component Card { slot visible: bool = true; Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    slot = find_child(comp, "slot_decl")
    refute_nil find_child(slot, "slot_type")
    refute_nil find_child(slot, "default_value")
  end

  def test_slot_list_type
    source = 'component Card { slot items: list<text>; Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    slot = find_child(comp, "slot_decl")
    slot_type = find_child(slot, "slot_type")
    list_type = find_child(slot_type, "list_type")
    refute_nil list_type, "Expected list_type in slot_type"
  end

  # ------------------------------------------------------------------
  # Node tree
  # ------------------------------------------------------------------

  def test_node_tree_exists
    source = 'component Label { Text { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    node_tree = find_child(comp, "node_tree")
    refute_nil node_tree
  end

  def test_node_element_tag
    source = 'component Label { Column { } }'
    ast = parse(source)
    comp = find_child(ast, "component_decl")
    node_tree = find_child(comp, "node_tree")
    element = find_child(node_tree, "node_element")
    names = token_values(element, "NAME")
    assert_includes names, "Column"
  end

  # ------------------------------------------------------------------
  # Property assignments
  # ------------------------------------------------------------------

  def test_property_assignment
    source = 'component Label { Text { content: "hello"; } }'
    ast = parse(source)
    rules = rule_names(ast)
    assert_includes rules, "property_assignment"
  end

  def test_slot_ref_as_property
    source = 'component Label { slot title: text; Text { content: @title; } }'
    ast = parse(source)
    rules = rule_names(ast)
    assert_includes rules, "slot_ref"
  end

  def test_dimension_property
    source = 'component Card { Column { padding: 16dp; } }'
    ast = parse(source)
    rules = rule_names(ast)
    assert_includes rules, "property_assignment"
  end

  # ------------------------------------------------------------------
  # Nested children
  # ------------------------------------------------------------------

  def test_nested_nodes
    source = 'component Card { Column { Row { Text { } } } }'
    ast = parse(source)
    rules = rule_names(ast)
    # Should have multiple node_element occurrences
    assert rules.count("node_element") >= 3
  end

  # ------------------------------------------------------------------
  # when blocks
  # ------------------------------------------------------------------

  def test_when_block
    source = <<~MOSAIC
      component Card {
        slot show: bool;
        Column {
          when @show {
            Text { content: "visible"; }
          }
        }
      }
    MOSAIC
    ast = parse(source)
    rules = rule_names(ast)
    assert_includes rules, "when_block"
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
    ast = parse(source)
    rules = rule_names(ast)
    assert_includes rules, "each_block"
  end

  # ------------------------------------------------------------------
  # Import declarations
  # ------------------------------------------------------------------

  def test_import_decl
    source = <<~MOSAIC
      import Button from "./button.mosaic";
      component Card { Column { } }
    MOSAIC
    ast = parse(source)
    import_decl = find_child(ast, "import_decl")
    refute_nil import_decl, "Expected import_decl at file level"
  end

  def test_import_with_alias
    source = <<~MOSAIC
      import Card as InfoCard from "./card.mosaic";
      component Page { Column { } }
    MOSAIC
    ast = parse(source)
    import_decl = find_child(ast, "import_decl")
    refute_nil import_decl
    names = token_values(import_decl, "NAME")
    assert_includes names, "Card"
    assert_includes names, "InfoCard"
  end
end
