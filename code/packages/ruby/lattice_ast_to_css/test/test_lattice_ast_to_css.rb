# frozen_string_literal: true

# ================================================================
# Test Suite for CodingAdventures::LatticeAstToCss
# ================================================================
#
# Coverage strategy:
# - Version constant
# - Error classes: all 10 error types
# - ScopeChain: get/set/has/child/depth/has_local
# - Value types: LatticeNumber, LatticeDimension, etc. (all 9)
# - LatticeAstToCss.truthy? (all falsy/truthy cases)
# - token_to_value: all token types
# - value_to_css: all value types
# - ExpressionEvaluator: all arithmetic/comparison/logical operations
# - LatticeTransformer: three-pass transform
# - CSSEmitter: all rule types (pretty and minified)
#
# We test the full pipeline via LatticeParser to produce real ASTs,
# then LatticeTransformer + CSSEmitter to produce CSS text.
# ================================================================

require "minitest/autorun"
require "coding_adventures_lattice_ast_to_css"
require "coding_adventures_lattice_parser"

# Alias common namespaces for brevity.
ATC = CodingAdventures::LatticeAstToCss

# Helper: parse Lattice source and transform to CSS text.
def transpile(source, minified: false, indent: "  ")
  ast = CodingAdventures::LatticeParser.parse(source)
  transformer = ATC::LatticeTransformer.new
  css_ast = transformer.transform(ast)
  emitter = ATC::CSSEmitter.new(indent: indent, minified: minified)
  emitter.emit(css_ast)
end

# ================================================================
class TestVersion < Minitest::Test
  def test_version_exists
    refute_nil ATC::VERSION
  end

  def test_version_is_string
    assert_kind_of String, ATC::VERSION
  end
end

# ================================================================
class TestErrorClasses < Minitest::Test
  def test_lattice_error_base
    err = ATC::LatticeError.new("something went wrong", 3, 7)
    assert_equal "something went wrong", err.message
    assert_equal 3, err.line
    assert_equal 7, err.column
    assert_includes err.to_s, "line 3"
  end

  def test_lattice_error_no_location
    err = ATC::LatticeError.new("oops")
    assert_equal 0, err.line
    refute_includes err.to_s, "line"
  end

  def test_module_not_found
    err = ATC::LatticeModuleNotFoundError.new("colors", 1, 1)
    assert_equal "colors", err.module_name
    assert_includes err.message, "colors"
    assert_kind_of ATC::LatticeError, err
  end

  def test_return_outside_function
    err = ATC::LatticeReturnOutsideFunctionError.new(5, 3)
    assert_includes err.message, "@return"
    assert_kind_of ATC::LatticeError, err
  end

  def test_undefined_variable_error
    err = ATC::LatticeUndefinedVariableError.new("$color", 2, 4)
    assert_equal "$color", err.name
    assert_includes err.message, "$color"
  end

  def test_undefined_mixin_error
    err = ATC::LatticeUndefinedMixinError.new("button", 3, 1)
    assert_equal "button", err.name
    assert_includes err.message, "button"
  end

  def test_undefined_function_error
    err = ATC::LatticeUndefinedFunctionError.new("spacing", 1, 5)
    assert_equal "spacing", err.name
    assert_includes err.message, "spacing"
  end

  def test_wrong_arity_error
    err = ATC::LatticeWrongArityError.new("Mixin", "button", 2, 3)
    assert_equal "button", err.name
    assert_equal 2, err.expected
    assert_equal 3, err.got
    assert_includes err.message, "2"
    assert_includes err.message, "3"
  end

  def test_circular_reference_error
    err = ATC::LatticeCircularReferenceError.new("mixin", ["a", "b", "a"])
    assert_equal ["a", "b", "a"], err.chain
    assert_includes err.message, "a -> b -> a"
  end

  def test_type_error_in_expression
    err = ATC::LatticeTypeErrorInExpression.new("add", "10px", "red")
    assert_equal "add", err.op
    assert_includes err.message, "10px"
    assert_includes err.message, "red"
  end

  def test_unit_mismatch_error
    err = ATC::LatticeUnitMismatchError.new("px", "s")
    assert_includes err.message, "px"
    assert_includes err.message, "s"
  end

  def test_missing_return_error
    err = ATC::LatticeMissingReturnError.new("spacing")
    assert_equal "spacing", err.name
    assert_includes err.message, "spacing"
  end

  def test_all_errors_inherit_from_lattice_error
    errors = [
      ATC::LatticeModuleNotFoundError.new("x"),
      ATC::LatticeReturnOutsideFunctionError.new,
      ATC::LatticeUndefinedVariableError.new("x"),
      ATC::LatticeUndefinedMixinError.new("x"),
      ATC::LatticeUndefinedFunctionError.new("x"),
      ATC::LatticeWrongArityError.new("M", "x", 1, 2),
      ATC::LatticeCircularReferenceError.new("m", ["a"]),
      ATC::LatticeTypeErrorInExpression.new("add", "a", "b"),
      ATC::LatticeUnitMismatchError.new("px", "em"),
      ATC::LatticeMissingReturnError.new("x")
    ]
    errors.each do |err|
      assert_kind_of ATC::LatticeError, err, "#{err.class} should inherit from LatticeError"
    end
  end
end

# ================================================================
class TestScopeChain < Minitest::Test
  def test_global_scope_depth
    scope = ATC::ScopeChain.new
    assert_equal 0, scope.depth
  end

  def test_child_scope_depth
    scope = ATC::ScopeChain.new
    child = scope.child
    assert_equal 1, child.depth
    grandchild = child.child
    assert_equal 2, grandchild.depth
  end

  def test_set_and_get
    scope = ATC::ScopeChain.new
    scope.set("$color", "red")
    assert_equal "red", scope.get("$color")
  end

  def test_get_returns_nil_for_missing
    scope = ATC::ScopeChain.new
    assert_nil scope.get("$nonexistent")
  end

  def test_child_inherits_from_parent
    parent = ATC::ScopeChain.new
    parent.set("$color", "red")
    child = parent.child
    assert_equal "red", child.get("$color")
  end

  def test_child_shadows_parent
    parent = ATC::ScopeChain.new
    parent.set("$color", "red")
    child = parent.child
    child.set("$color", "blue")
    assert_equal "blue", child.get("$color")
    assert_equal "red", parent.get("$color")
  end

  def test_grandchild_inherits
    global = ATC::ScopeChain.new
    global.set("$x", 42)
    child = global.child
    grandchild = child.child
    assert_equal 42, grandchild.get("$x")
  end

  def test_has_returns_true_for_existing
    scope = ATC::ScopeChain.new
    scope.set("$y", 10)
    assert scope.has?("$y")
  end

  def test_has_returns_false_for_missing
    scope = ATC::ScopeChain.new
    refute scope.has?("$z")
  end

  def test_has_walks_parent_chain
    parent = ATC::ScopeChain.new
    parent.set("$a", 1)
    child = parent.child
    assert child.has?("$a")
  end

  def test_has_local_only_checks_current_scope
    parent = ATC::ScopeChain.new
    parent.set("$a", 1)
    child = parent.child
    refute child.has_local?("$a")
    child.set("$a", 2)
    assert child.has_local?("$a")
  end

  def test_sibling_scopes_dont_share
    parent = ATC::ScopeChain.new
    sib1 = parent.child
    sib2 = parent.child
    sib1.set("$x", "from-sib1")
    assert_nil sib2.get("$x")
  end
end

# ================================================================
class TestValueTypes < Minitest::Test
  def test_lattice_number_to_s_integer
    assert_equal "42", ATC::LatticeNumber.new(42.0).to_s
  end

  def test_lattice_number_to_s_float
    assert_equal "3.14", ATC::LatticeNumber.new(3.14).to_s
  end

  def test_lattice_number_truthy
    assert ATC::LatticeNumber.new(1.0).truthy?
    refute ATC::LatticeNumber.new(0.0).truthy?
  end

  def test_lattice_dimension_to_s
    assert_equal "16px", ATC::LatticeDimension.new(16.0, "px").to_s
    assert_equal "2.5em", ATC::LatticeDimension.new(2.5, "em").to_s
  end

  def test_lattice_percentage_to_s
    assert_equal "50%", ATC::LatticePercentage.new(50.0).to_s
    assert_equal "33.33%", ATC::LatticePercentage.new(33.33).to_s
  end

  def test_lattice_string_to_s
    assert_equal '"hello"', ATC::LatticeString.new("hello").to_s
  end

  def test_lattice_ident_to_s
    assert_equal "red", ATC::LatticeIdent.new("red").to_s
  end

  def test_lattice_color_to_s
    assert_equal "#4a90d9", ATC::LatticeColor.new("#4a90d9").to_s
  end

  def test_lattice_bool_to_s
    assert_equal "true", ATC::LatticeBool.new(true).to_s
    assert_equal "false", ATC::LatticeBool.new(false).to_s
  end

  def test_lattice_null_to_s
    assert_equal "", ATC::LatticeNull.new.to_s
  end

  def test_lattice_list_to_s
    items = [ATC::LatticeIdent.new("red"), ATC::LatticeIdent.new("blue")]
    assert_equal "red, blue", ATC::LatticeList.new(items).to_s
  end

  def test_truthy_values
    assert ATC.truthy?(ATC::LatticeNumber.new(1.0))
    assert ATC.truthy?(ATC::LatticeDimension.new(16.0, "px"))
    assert ATC.truthy?(ATC::LatticePercentage.new(50.0))
    assert ATC.truthy?(ATC::LatticeString.new("hello"))
    assert ATC.truthy?(ATC::LatticeIdent.new("red"))
    assert ATC.truthy?(ATC::LatticeColor.new("#fff"))
    assert ATC.truthy?(ATC::LatticeBool.new(true))
    assert ATC.truthy?(ATC::LatticeList.new([]))
  end

  def test_falsy_values
    refute ATC.truthy?(ATC::LatticeBool.new(false))
    refute ATC.truthy?(ATC::LatticeNull.new)
    refute ATC.truthy?(ATC::LatticeNumber.new(0.0))
  end
end

# ================================================================
class TestTokenConversion < Minitest::Test
  # Helper: create a fake token.
  def tok(type, value)
    Struct.new(:type, :value).new(type, value)
  end

  def test_number_token
    result = ATC.token_to_value(tok("NUMBER", "42"))
    assert_instance_of ATC::LatticeNumber, result
    assert_in_delta 42.0, result.value
  end

  def test_dimension_token
    result = ATC.token_to_value(tok("DIMENSION", "16px"))
    assert_instance_of ATC::LatticeDimension, result
    assert_in_delta 16.0, result.value
    assert_equal "px", result.unit
  end

  def test_dimension_token_em
    result = ATC.token_to_value(tok("DIMENSION", "2em"))
    assert_equal "em", result.unit
  end

  def test_percentage_token
    result = ATC.token_to_value(tok("PERCENTAGE", "50%"))
    assert_instance_of ATC::LatticePercentage, result
    assert_in_delta 50.0, result.value
  end

  def test_string_token
    result = ATC.token_to_value(tok("STRING", "hello"))
    assert_instance_of ATC::LatticeString, result
    assert_equal "hello", result.value
  end

  def test_hash_token
    result = ATC.token_to_value(tok("HASH", "#fff"))
    assert_instance_of ATC::LatticeColor, result
    assert_equal "#fff", result.value
  end

  def test_ident_token_plain
    result = ATC.token_to_value(tok("IDENT", "red"))
    assert_instance_of ATC::LatticeIdent, result
    assert_equal "red", result.value
  end

  def test_ident_token_true
    result = ATC.token_to_value(tok("IDENT", "true"))
    assert_instance_of ATC::LatticeBool, result
    assert result.value
  end

  def test_ident_token_false
    result = ATC.token_to_value(tok("IDENT", "false"))
    assert_instance_of ATC::LatticeBool, result
    refute result.value
  end

  def test_ident_token_null
    result = ATC.token_to_value(tok("IDENT", "null"))
    assert_instance_of ATC::LatticeNull, result
  end
end

# ================================================================
class TestTransformerBasics < Minitest::Test
  # Plain CSS should pass through unchanged.
  def test_plain_css_passthrough
    css = transpile("h1 { color: red; }")
    assert_includes css, "h1"
    assert_includes css, "color: red"
  end

  # Variable declaration + reference.
  def test_variable_substitution
    css = transpile("$primary: red;\nh1 { color: $primary; }")
    assert_includes css, "color: red"
    refute_includes css, "$primary"
  end

  # Multiple variables.
  def test_multiple_variables
    css = transpile("$a: red;\n$b: blue;\nh1 { color: $a; }\np { color: $b; }")
    assert_includes css, "color: red"
    assert_includes css, "color: blue"
  end

  # Variable with dimension value.
  def test_variable_with_dimension
    css = transpile("$size: 16px;\nh1 { font-size: $size; }")
    assert_includes css, "font-size: 16px"
  end

  # Variable with hash color.
  def test_variable_with_color
    css = transpile("$primary: #4a90d9;\nh1 { color: $primary; }")
    assert_includes css, "color: #4a90d9"
  end

  # Undefined variable raises error.
  def test_undefined_variable_raises
    assert_raises(ATC::LatticeUndefinedVariableError) do
      transpile("h1 { color: $nonexistent; }")
    end
  end

  # @use directives are silently skipped.
  def test_use_directive_skipped
    css = transpile('@use "colors";h1 { color: red; }')
    refute_includes css, "@use"
    assert_includes css, "color: red"
  end
end

# ================================================================
class TestTransformerMixins < Minitest::Test
  def test_simple_mixin
    source = "@mixin red-text { color: red; }\nh1 { @include red-text; }"
    css = transpile(source)
    assert_includes css, "color: red"
    refute_includes css, "@mixin"
    refute_includes css, "@include"
  end

  def test_mixin_with_parameter
    source = "@mixin color-text($c) { color: $c; }\nh1 { @include color-text(blue); }"
    css = transpile(source)
    assert_includes css, "color: blue"
  end

  def test_mixin_with_default_param
    source = "@mixin box($p: 8px) { padding: $p; }\nh1 { @include box; }"
    css = transpile(source)
    assert_includes css, "padding: 8px"
  end

  def test_mixin_default_overridden
    source = "@mixin box($p: 8px) { padding: $p; }\nh1 { @include box(16px); }"
    css = transpile(source)
    assert_includes css, "padding: 16px"
  end

  def test_undefined_mixin_raises
    assert_raises(ATC::LatticeUndefinedMixinError) do
      transpile("h1 { @include nonexistent; }")
    end
  end

  def test_wrong_arity_raises
    source = "@mixin btn($a, $b) { color: $a; }\nh1 { @include btn(red); }"
    assert_raises(ATC::LatticeWrongArityError) do
      transpile(source)
    end
  end

  def test_circular_mixin_raises
    source = "@mixin a { @include b; }\n@mixin b { @include a; }\nh1 { @include a; }"
    assert_raises(ATC::LatticeCircularReferenceError) do
      transpile(source)
    end
  end

  def test_mixin_defined_after_use
    # Mixins can be defined after use (Pass 1 collects all first).
    source = "h1 { @include late; }\n@mixin late { color: green; }"
    css = transpile(source)
    assert_includes css, "color: green"
  end
end

# ================================================================
class TestTransformerControlFlow < Minitest::Test
  def test_if_true_branch
    source = "$x: true;\n@if $x { h1 { color: green; } }"
    css = transpile(source)
    assert_includes css, "color: green"
  end

  def test_if_false_branch_skipped
    source = "$flag: false;\n@if $flag { h1 { color: red; } }"
    css = transpile(source)
    refute_includes css, "color: red"
  end

  def test_if_else
    source = "$dark: false;\n@if $dark { h1 { color: white; } } @else { h1 { color: black; } }"
    css = transpile(source)
    assert_includes css, "color: black"
    refute_includes css, "color: white"
  end

  def test_if_equality_check
    source = "$theme: dark;\n@if $theme == dark { h1 { background: black; } }"
    css = transpile(source)
    assert_includes css, "background: black"
  end

  def test_if_not_equal
    source = "$x: 0;\n@if $x != 0 { h1 { color: red; } } @else { h1 { color: blue; } }"
    css = transpile(source)
    assert_includes css, "color: blue"
    refute_includes css, "color: red"
  end

  def test_for_through
    source = "@for $i from 1 through 3 { h1 { font-size: 16px; } }"
    css = transpile(source)
    # Should produce 3 rules.
    assert_equal 3, css.scan("font-size: 16px").size
  end

  def test_for_to_exclusive
    source = "@for $i from 1 to 3 { h1 { font-size: 16px; } }"
    css = transpile(source)
    # to is exclusive: 1, 2 (not 3).
    assert_equal 2, css.scan("font-size: 16px").size
  end

  def test_each_directive
    source = "@each $color in red, green, blue { h1 { color: red; } }"
    css = transpile(source)
    # Each iteration produces one rule.
    assert_equal 3, css.scan("color: red").size
  end
end

# ================================================================
class TestTransformerFunctions < Minitest::Test
  def test_function_basic
    source = "@function double($x) { @return $x * 2; }\nh1 { font-size: double(8px); }"
    css = transpile(source)
    assert_includes css, "16px"
  end

  def test_function_number_multiplication
    source = "@function triple($n) { @return $n * 3; }\nh1 { z-index: triple(5); }"
    css = transpile(source)
    assert_includes css, "15"
  end

  def test_function_missing_return_raises
    source = "@function noop($x) { $y: $x; }\nh1 { color: noop(red); }"
    assert_raises(ATC::LatticeMissingReturnError) do
      transpile(source)
    end
  end

  def test_circular_function_raises
    source = "@function a($x) { @return b($x); }\n@function b($x) { @return a($x); }\nh1 { color: a(1); }"
    assert_raises(ATC::LatticeCircularReferenceError) do
      transpile(source)
    end
  end

  def test_function_defined_after_use
    source = "h1 { padding: spacing(2); }\n@function spacing($n) { @return $n * 8px; }"
    css = transpile(source)
    assert_includes css, "16px"
  end
end

# ================================================================
class TestCSSEmitter < Minitest::Test
  def test_pretty_print_has_indentation
    css = transpile("h1 { color: red; }")
    assert_includes css, "  color: red"
  end

  def test_pretty_print_has_newlines
    css = transpile("h1 { color: red; }")
    assert_includes css, "\n"
  end

  def test_minified_no_whitespace_between_rules
    css = transpile("h1 { color: red; }", minified: true)
    # Minified output should have no newlines and no extra spaces.
    refute_includes css, "\n"
    assert_includes css, "h1{color:red;}"
  end

  def test_pretty_print_blank_line_between_rules
    css = transpile("h1 { color: red; }\nh2 { color: blue; }")
    assert_includes css, "\n\n"
  end

  def test_minified_multiple_rules
    css = transpile("h1 { color: red; }\nh2 { color: blue; }", minified: true)
    assert_includes css, "h1{color:red;}"
    assert_includes css, "h2{color:blue;}"
  end

  def test_custom_indent
    css = transpile("h1 { color: red; }", indent: "    ")
    assert_includes css, "    color: red"
  end

  def test_selector_list_comma_separation
    css = transpile("h1, h2 { color: red; }")
    assert_includes css, "h1, h2"
  end

  def test_declaration_semicolon
    css = transpile("h1 { color: red; }")
    assert_includes css, "color: red;"
  end

  def test_important_declaration
    css = transpile("h1 { color: red !important; }")
    assert_includes css, "!important"
  end

  def test_at_rule_media
    css = transpile("@media (max-width: 768px) { h1 { font-size: 14px; } }")
    assert_includes css, "@media"
    assert_includes css, "font-size: 14px"
  end

  def test_at_rule_import
    css = transpile('@import "reset.css";')
    assert_includes css, "@import"
  end

  def test_function_call_in_value
    css = transpile("h1 { color: rgb(255, 0, 0); }")
    assert_includes css, "rgb("
  end

  def test_pseudo_class
    css = transpile("a:hover { color: blue; }")
    assert_includes css, "a:hover"
  end

  def test_class_selector
    css = transpile(".btn { padding: 8px; }")
    assert_includes css, ".btn"
  end

  def test_id_selector
    css = transpile("#main { display: block; }")
    assert_includes css, "#main"
  end

  def test_empty_source_returns_empty
    css = transpile("")
    assert_equal "", css
  end
end

# ================================================================
class TestExpressionEvaluator < Minitest::Test
  # Helper: evaluate a Lattice expression via @function/@return pipeline.
  #
  # Creates a zero-argument function that returns the given expression,
  # calls it with empty parens (test_fn()), and transpiles the whole
  # source. The result should contain the evaluated expression value.
  #
  # NOTE: Lattice functions must be called with parens — "test_fn()" —
  # so that the lexer emits a FUNCTION token, which the grammar expects
  # for function_call. "test_fn" without parens is an IDENT (CSS value).
  def eval_expr(expr_source, vars = {})
    var_decls = vars.map { |k, v| "#{k}: #{v};" }.join("\n")
    source = "#{var_decls}\n@function test_fn() { @return #{expr_source}; }\nh1 { z-index: test_fn(); }"
    transpile(source)
  end

  def test_number_literal
    css = eval_expr("42")
    assert_includes css, "42"
  end

  def test_dimension_literal
    css = eval_expr("16px")
    assert_includes css, "16px"
  end

  def test_addition
    css = eval_expr("8px + 8px")
    assert_includes css, "16px"
  end

  def test_subtraction
    css = eval_expr("10px - 4px")
    assert_includes css, "6px"
  end

  def test_multiplication_number
    css = eval_expr("3 * 4")
    assert_includes css, "12"
  end

  def test_multiplication_dimension
    css = eval_expr("2 * 8px")
    assert_includes css, "16px"
  end

  def test_type_error_raises
    source = "@function f() { @return 10px + red; }\nh1 { z-index: f(); }"
    assert_raises(ATC::LatticeTypeErrorInExpression) do
      transpile(source)
    end
  end
end

# ================================================================
class TestFullPipeline < Minitest::Test
  def test_comprehensive_lattice_stylesheet
    source = <<~LATTICE
      $primary: #4a90d9;
      $pad: 8px;

      @mixin center {
        text-align: center;
      }

      h1 {
        color: $primary;
        padding: $pad;
        @include center;
      }
    LATTICE
    css = transpile(source)
    assert_includes css, "color: #4a90d9"
    assert_includes css, "padding: 8px"
    assert_includes css, "text-align: center"
    refute_includes css, "@mixin"
    refute_includes css, "@include"
    refute_includes css, "$primary"
  end

  def test_conditional_theme_output
    source = <<~LATTICE
      $theme: dark;

      @if $theme == dark {
        body { background: black; color: white; }
      } @else {
        body { background: white; color: black; }
      }
    LATTICE
    css = transpile(source)
    assert_includes css, "background: black"
    assert_includes css, "color: white"
    refute_includes css, "background: white"
  end
end
