# frozen_string_literal: true

# ================================================================
# Test Suite for CodingAdventures::LatticeTranspiler
# ================================================================
#
# Integration tests covering the full Lattice -> CSS pipeline.
# Tests verify the final CSS output for various Lattice constructs.
# ================================================================

require "minitest/autorun"
require "coding_adventures_lattice_transpiler"

TP = CodingAdventures::LatticeTranspiler
ATC = CodingAdventures::LatticeAstToCss

class TestLatticeTranspilerVersion < Minitest::Test
  def test_version_exists
    refute_nil TP::VERSION
  end

  def test_version_is_string
    assert_kind_of String, TP::VERSION
  end
end

class TestLatticeTranspilerInterface < Minitest::Test
  def test_transpile_returns_string
    result = TP.transpile("h1 { color: red; }")
    assert_kind_of String, result
  end

  def test_transpile_ends_with_newline
    result = TP.transpile("h1 { color: red; }")
    assert result.end_with?("\n")
  end

  def test_empty_source_returns_empty
    result = TP.transpile("")
    assert_equal "", result
  end

  def test_minified_option
    result = TP.transpile("h1 { color: red; }", minified: true)
    assert_includes result, "h1{color:red;}"
    refute_includes result, "\n"
  end

  def test_custom_indent
    result = TP.transpile("h1 { color: red; }", indent: "    ")
    assert_includes result, "    color: red"
  end
end

class TestLatticeTranspilerCSSPassthrough < Minitest::Test
  def test_simple_rule
    css = TP.transpile("h1 { color: red; }")
    assert_includes css, "h1"
    assert_includes css, "color: red"
  end

  def test_multiple_declarations
    css = TP.transpile("h1 { color: red; font-size: 16px; }")
    assert_includes css, "color: red"
    assert_includes css, "font-size: 16px"
  end

  def test_class_selector
    css = TP.transpile(".btn { padding: 8px; }")
    assert_includes css, ".btn"
    assert_includes css, "padding: 8px"
  end

  def test_id_selector
    css = TP.transpile("#main { display: block; }")
    assert_includes css, "#main"
  end

  def test_pseudo_class
    css = TP.transpile("a:hover { color: blue; }")
    assert_includes css, "a:hover"
  end

  def test_pseudo_element
    css = TP.transpile("p::before { content: ''; }")
    assert_includes css, "p::before"
  end

  def test_media_query
    css = TP.transpile("@media (max-width: 768px) { h1 { font-size: 14px; } }")
    assert_includes css, "@media"
  end

  def test_import_at_rule
    css = TP.transpile('@import "reset.css";')
    assert_includes css, "@import"
  end

  def test_important
    css = TP.transpile("h1 { color: red !important; }")
    assert_includes css, "!important"
  end
end

class TestLatticeTranspilerVariables < Minitest::Test
  def test_variable_substitution
    css = TP.transpile("$color: red;\nh1 { color: $color; }")
    assert_includes css, "color: red"
    refute_includes css, "$color"
  end

  def test_dimension_variable
    css = TP.transpile("$size: 16px;\nh1 { font-size: $size; }")
    assert_includes css, "font-size: 16px"
  end

  def test_color_variable
    css = TP.transpile("$bg: #4a90d9;\nh1 { background: $bg; }")
    assert_includes css, "background: #4a90d9"
  end

  def test_variable_in_multiple_places
    css = TP.transpile("$c: red;\nh1 { color: $c; }\nh2 { color: $c; }")
    assert_equal 2, css.scan("color: red").size
  end

  def test_undefined_variable_raises
    assert_raises(ATC::LatticeUndefinedVariableError) do
      TP.transpile("h1 { color: $missing; }")
    end
  end
end

class TestLatticeTranspilerMixins < Minitest::Test
  def test_simple_mixin_include
    css = TP.transpile("@mixin centered { text-align: center; }\nh1 { @include centered; }")
    assert_includes css, "text-align: center"
    refute_includes css, "@mixin"
    refute_includes css, "@include"
  end

  def test_mixin_with_argument
    css = TP.transpile("@mixin color($c) { color: $c; }\nh1 { @include color(blue); }")
    assert_includes css, "color: blue"
  end

  def test_mixin_with_default_param
    css = TP.transpile("@mixin pad($p: 8px) { padding: $p; }\nh1 { @include pad; }")
    assert_includes css, "padding: 8px"
  end

  def test_mixin_with_multiple_params
    source = "@mixin box($bg, $p: 10px) { background: $bg; padding: $p; }\n.card { @include box(white); }"
    css = TP.transpile(source)
    assert_includes css, "background: white"
    assert_includes css, "padding: 10px"
  end

  def test_mixin_defined_after_use
    css = TP.transpile("h1 { @include late; }\n@mixin late { color: green; }")
    assert_includes css, "color: green"
  end

  def test_undefined_mixin_raises
    assert_raises(ATC::LatticeUndefinedMixinError) do
      TP.transpile("h1 { @include ghost; }")
    end
  end

  def test_wrong_arity_raises
    source = "@mixin btn($a, $b) { color: $a; }\nh1 { @include btn(red); }"
    assert_raises(ATC::LatticeWrongArityError) do
      TP.transpile(source)
    end
  end

  def test_circular_mixin_raises
    source = "@mixin a { @include b; }\n@mixin b { @include a; }\nh1 { @include a; }"
    assert_raises(ATC::LatticeCircularReferenceError) do
      TP.transpile(source)
    end
  end
end

class TestLatticeTranspilerControlFlow < Minitest::Test
  def test_if_true_condition
    css = TP.transpile("$flag: true;\n@if $flag { h1 { color: red; } }")
    assert_includes css, "color: red"
  end

  def test_if_false_condition_skipped
    css = TP.transpile("$flag: false;\n@if $flag { h1 { color: red; } }")
    refute_includes css, "color: red"
  end

  def test_if_else
    css = TP.transpile("$night: false;\n@if $night { body { background: black; } } @else { body { background: white; } }")
    assert_includes css, "background: white"
    refute_includes css, "background: black"
  end

  def test_if_equality
    css = TP.transpile("$theme: dark;\n@if $theme == dark { body { color: white; } }")
    assert_includes css, "color: white"
  end

  def test_for_through_produces_n_rules
    css = TP.transpile("@for $i from 1 through 5 { h1 { color: red; } }")
    assert_equal 5, css.scan("color: red").size
  end

  def test_for_to_exclusive
    css = TP.transpile("@for $i from 1 to 4 { h1 { color: red; } }")
    assert_equal 3, css.scan("color: red").size
  end

  def test_each_iterates_list
    css = TP.transpile("@each $c in red, green, blue { h1 { color: red; } }")
    assert_equal 3, css.scan("color: red").size
  end
end

class TestLatticeTranspilerFunctions < Minitest::Test
  def test_function_basic_call
    source = "@function spacing($n) { @return $n * 8px; }\nh1 { padding: spacing(2); }"
    css = TP.transpile(source)
    assert_includes css, "16px"
  end

  def test_function_returns_number
    source = "@function triple($n) { @return $n * 3; }\nh1 { z-index: triple(4); }"
    css = TP.transpile(source)
    assert_includes css, "12"
  end

  def test_missing_return_raises
    source = "@function bad($x) { $y: $x; }\nh1 { color: bad(red); }"
    assert_raises(ATC::LatticeMissingReturnError) do
      TP.transpile(source)
    end
  end

  def test_circular_function_raises
    source = "@function a($x) { @return b($x); }\n@function b($x) { @return a($x); }\nh1 { z-index: a(1); }"
    assert_raises(ATC::LatticeCircularReferenceError) do
      TP.transpile(source)
    end
  end
end

class TestLatticeTranspilerIntegration < Minitest::Test
  def test_full_theme_stylesheet
    source = <<~LATTICE
      $primary: #4a90d9;
      $secondary: #7b68ee;
      $base-font: 16px;

      @mixin flex-center {
        display: flex;
        align-items: center;
        justify-content: center;
      }

      @function scale($n) {
        @return $n * $base-font;
      }

      .container {
        @include flex-center;
        padding: scale(2);
      }

      h1 {
        color: $primary;
        font-size: $base-font;
      }

      h2 {
        color: $secondary;
      }
    LATTICE

    css = TP.transpile(source)

    assert_includes css, "display: flex"
    assert_includes css, "align-items: center"
    assert_includes css, "justify-content: center"
    assert_includes css, "color: #4a90d9"
    assert_includes css, "color: #7b68ee"
    assert_includes css, "font-size: 16px"
    assert_includes css, "32px"  # scale(2) = 2 * 16px

    refute_includes css, "@mixin"
    refute_includes css, "@function"
    refute_includes css, "@include"
    refute_includes css, "$primary"
    refute_includes css, "$secondary"
  end

  def test_conditional_responsive
    source = <<~LATTICE
      $mobile: true;

      @if $mobile {
        body { font-size: 14px; }
      } @else {
        body { font-size: 16px; }
      }
    LATTICE

    css = TP.transpile(source)
    assert_includes css, "font-size: 14px"
    refute_includes css, "font-size: 16px"
  end

  def test_minified_output_is_compact
    source = "$c: red;\nh1 { color: $c; }\nh2 { color: $c; }"
    css = TP.transpile(source, minified: true)
    refute_includes css, "\n"
    assert_includes css, "h1{color:red;}"
    assert_includes css, "h2{color:red;}"
  end

  def test_css_functions_pass_through
    css = TP.transpile("h1 { background: linear-gradient(to right, red, blue); }")
    assert_includes css, "linear-gradient"
  end

  def test_calc_function_pass_through
    css = TP.transpile("h1 { width: calc(100% - 20px); }")
    assert_includes css, "calc("
  end
end
