# frozen_string_literal: true

# ================================================================
# Test Suite for CodingAdventures::LatticeParser
# ================================================================
#
# We parse Lattice source text and verify the resulting AST
# has the expected rule_name and child structure.
#
# Coverage strategy:
# - Version constant
# - parse / create_parser interface
# - All Lattice constructs: variables, mixins, control flow, functions, @use
# - All CSS constructs: qualified rules, declarations, at-rules
# - @include directive (both forms)
# - Selector types: type, class, id, pseudo, attribute
# - Value types: dimensions, percentages, strings, colors, functions
# ================================================================

require "minitest/autorun"
require "coding_adventures_lattice_parser"

# Helper: find all AST nodes with a given rule_name via depth-first search.
def find_nodes(node, rule_name, results = [])
  return results unless node.respond_to?(:rule_name)

  results << node if node.rule_name == rule_name
  if node.respond_to?(:children)
    node.children.each { |c| find_nodes(c, rule_name, results) }
  end
  results
end

# Helper: get the first child node matching a rule_name.
def find_node(node, rule_name)
  find_nodes(node, rule_name).first
end

# Helper: collect all token values in a node's subtree.
def collect_token_values(node)
  results = []
  if node.respond_to?(:children)
    node.children.each do |child|
      if child.respond_to?(:value) && !child.respond_to?(:rule_name)
        results << child.value
      else
        results.concat(collect_token_values(child))
      end
    end
  elsif node.respond_to?(:value)
    results << node.value
  end
  results
end

class TestLatticeParserVersion < Minitest::Test
  def test_version_exists
    refute_nil CodingAdventures::LatticeParser::VERSION
  end

  def test_version_is_string
    assert_kind_of String, CodingAdventures::LatticeParser::VERSION
  end
end

class TestLatticeParserInterface < Minitest::Test
  def test_parse_returns_ast_node
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; }")
    assert_respond_to ast, :rule_name
    assert_respond_to ast, :children
  end

  def test_parse_root_is_stylesheet
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; }")
    assert_equal "stylesheet", ast.rule_name
  end

  def test_create_parser_returns_parser
    parser = CodingAdventures::LatticeParser.create_parser("$x: 10px;")
    assert_respond_to parser, :parse
  end

  def test_create_parser_produces_same_ast
    source = "$color: red;"
    direct_ast = CodingAdventures::LatticeParser.parse(source)
    parser = CodingAdventures::LatticeParser.create_parser(source)
    via_parser = parser.parse
    assert_equal direct_ast.rule_name, via_parser.rule_name
  end

  def test_empty_source_parses
    ast = CodingAdventures::LatticeParser.parse("")
    assert_equal "stylesheet", ast.rule_name
  end
end

class TestLatticeParserVariables < Minitest::Test
  # $primary: #4a90d9;
  def test_variable_declaration_node_exists
    ast = CodingAdventures::LatticeParser.parse("$primary: #4a90d9;")
    nodes = find_nodes(ast, "variable_declaration")
    assert_equal 1, nodes.size
  end

  # The variable token is $primary.
  def test_variable_declaration_name
    ast = CodingAdventures::LatticeParser.parse("$primary: #4a90d9;")
    var_node = find_node(ast, "variable_declaration")
    refute_nil var_node
    var_token = var_node.children.find { |c| !c.respond_to?(:rule_name) && c.type.to_s == "VARIABLE" }
    refute_nil var_token
    assert_equal "$primary", var_token.value
  end

  # Multiple variable declarations.
  def test_multiple_variable_declarations
    source = "$a: 1px;\n$b: 2px;\n$c: 3px;"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "variable_declaration")
    assert_equal 3, nodes.size
  end

  # Variable reference in a declaration value.
  def test_variable_reference_in_value
    source = "$color: red;\nh1 { color: $color; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    value_lists = find_nodes(ast, "value_list")
    all_tokens = value_lists.flat_map { |vl| collect_token_values(vl) }
    assert_includes all_tokens, "$color"
  end
end

class TestLatticeParserCSSRules < Minitest::Test
  # h1 { color: red; } produces a qualified_rule.
  def test_simple_qualified_rule
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; }")
    nodes = find_nodes(ast, "qualified_rule")
    assert_equal 1, nodes.size
  end

  # qualified_rule contains selector_list and block.
  def test_qualified_rule_structure
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; }")
    qr = find_node(ast, "qualified_rule")
    child_rules = qr.children.map { |c| c.rule_name if c.respond_to?(:rule_name) }.compact
    assert_includes child_rules, "selector_list"
    assert_includes child_rules, "block"
  end

  # Declaration: property: value;
  def test_declaration_in_block
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; }")
    decl = find_node(ast, "declaration")
    refute_nil decl
  end

  # Property name is correct.
  def test_declaration_property_name
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; }")
    prop = find_node(ast, "property")
    refute_nil prop
    token = prop.children.find { |c| !c.respond_to?(:rule_name) }
    assert_equal "color", token.value
  end

  # Multiple declarations.
  def test_multiple_declarations
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red; font-size: 16px; margin: 0; }")
    decls = find_nodes(ast, "declaration")
    assert_equal 3, decls.size
  end

  # Class selector: .btn { }
  def test_class_selector
    ast = CodingAdventures::LatticeParser.parse(".btn { color: blue; }")
    assert_equal "stylesheet", ast.rule_name
    nodes = find_nodes(ast, "qualified_rule")
    assert_equal 1, nodes.size
  end

  # ID selector: #main { }
  def test_id_selector
    ast = CodingAdventures::LatticeParser.parse("#main { display: block; }")
    assert_equal "stylesheet", ast.rule_name
    nodes = find_nodes(ast, "qualified_rule")
    assert_equal 1, nodes.size
  end

  # !important declaration.
  def test_important_declaration
    ast = CodingAdventures::LatticeParser.parse("h1 { color: red !important; }")
    prio = find_node(ast, "priority")
    refute_nil prio
  end
end

class TestLatticeParserMixins < Minitest::Test
  # @mixin definition produces mixin_definition node.
  def test_mixin_definition_node
    source = "@mixin button($bg) { background: $bg; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "mixin_definition")
    assert_equal 1, nodes.size
  end

  # @mixin with default parameter.
  def test_mixin_with_default_param
    source = "@mixin button($bg, $fg: white) { background: $bg; color: $fg; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "mixin_definition")
    assert_equal 1, nodes.size
    params = find_nodes(ast, "mixin_param")
    assert_equal 2, params.size
  end

  # @include directive (with FUNCTION form).
  def test_include_with_function_form
    source = "@mixin btn($bg) { background: $bg; }\n.x { @include btn(red); }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "include_directive")
    assert_equal 1, nodes.size
  end

  # @include directive (IDENT form, no parens).
  def test_include_with_ident_form
    source = "@mixin clearfix { content: ''; }\n.x { @include clearfix; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "include_directive")
    assert_equal 1, nodes.size
  end
end

class TestLatticeParserControlFlow < Minitest::Test
  # @if directive.
  def test_if_directive_node
    source = "@if $theme == dark { body { background: black; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "if_directive")
    assert_equal 1, nodes.size
  end

  # @if with @else.
  def test_if_else_directive
    source = "h1 { @if $x > 0 { color: green; } @else { color: red; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "if_directive")
    assert_equal 1, nodes.size
  end

  # @for directive.
  def test_for_directive_node
    source = "@for $i from 1 through 3 { h1 { font-size: 16px; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "for_directive")
    assert_equal 1, nodes.size
  end

  # @for with "to" (exclusive).
  def test_for_directive_to
    source = "@for $i from 1 to 5 { h1 { color: red; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "for_directive")
    assert_equal 1, nodes.size
  end

  # @each directive.
  def test_each_directive_node
    source = "@each $color in red, green, blue { h1 { color: red; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "each_directive")
    assert_equal 1, nodes.size
  end
end

class TestLatticeParserFunctions < Minitest::Test
  # @function definition.
  def test_function_definition_node
    source = "@function spacing($n) { @return $n * 8px; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "function_definition")
    assert_equal 1, nodes.size
  end

  # @return directive.
  def test_return_directive_node
    source = "@function spacing($n) { @return $n * 8px; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "return_directive")
    assert_equal 1, nodes.size
  end

  # function_body contains function_body_item.
  def test_function_body_structure
    source = "@function double($x) { @return $x * 2; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    body = find_node(ast, "function_body")
    refute_nil body
    items = find_nodes(ast, "function_body_item")
    assert_equal 1, items.size
  end
end

class TestLatticeParserModules < Minitest::Test
  # @use directive with just a string.
  def test_use_directive_simple
    source = '@use "colors";'
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "use_directive")
    assert_equal 1, nodes.size
  end

  # @use with "as" alias.
  def test_use_directive_with_alias
    source = '@use "utils/mixins" as m;'
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "use_directive")
    assert_equal 1, nodes.size
  end
end

class TestLatticeParserAtRules < Minitest::Test
  # @media at-rule with block.
  def test_media_at_rule
    source = "@media (max-width: 768px) { h1 { font-size: 14px; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "at_rule")
    refute_empty nodes
  end

  # @import at-rule with semicolon.
  def test_import_at_rule
    source = '@import "reset.css";'
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "at_rule")
    refute_empty nodes
  end
end

class TestLatticeParserExpressions < Minitest::Test
  # Arithmetic expression: $n * 8px
  def test_multiplicative_expression
    source = "@function f($n) { @return $n * 8px; }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "lattice_multiplicative")
    refute_empty nodes
  end

  # Comparison expression: $a == $b
  def test_comparison_expression
    source = "@if $x == $y { h1 { color: red; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "lattice_comparison")
    refute_empty nodes
  end

  # OR expression: $a or $b
  def test_or_expression
    source = "@if $a or $b { h1 { color: red; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "lattice_or_expr")
    refute_empty nodes
  end

  # AND expression: $a and $b
  def test_and_expression
    source = "@if $a and $b { h1 { color: red; } }"
    ast = CodingAdventures::LatticeParser.parse(source)
    nodes = find_nodes(ast, "lattice_and_expr")
    refute_empty nodes
  end
end

class TestLatticeParserIntegration < Minitest::Test
  # Full Lattice stylesheet with multiple constructs.
  def test_full_stylesheet
    source = <<~LATTICE
      $primary: #4a90d9;
      $secondary: #7b68ee;

      @mixin button($bg, $fg: white) {
        background: $bg;
        color: $fg;
        padding: 8px 16px;
      }

      h1 {
        color: $primary;
      }

      .btn {
        @include button($primary);
      }
    LATTICE
    ast = CodingAdventures::LatticeParser.parse(source)
    assert_equal "stylesheet", ast.rule_name
    assert_equal 2, find_nodes(ast, "variable_declaration").size
    assert_equal 1, find_nodes(ast, "mixin_definition").size
    assert_equal 2, find_nodes(ast, "qualified_rule").size
    assert_equal 1, find_nodes(ast, "include_directive").size
  end
end
