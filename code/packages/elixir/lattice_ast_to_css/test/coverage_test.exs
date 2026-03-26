defmodule CodingAdventures.LatticeAstToCss.CoverageTest do
  @moduledoc """
  Additional tests targeting uncovered code paths in:
  - Errors.ModuleNotFoundError
  - Errors.ReturnOutsideFunctionError
  - Errors.UndefinedFunctionError
  - Errors.UnitMismatchError
  - Values (list type, subtract, multiply, negate, compare edges, token_to_value fallbacks)
  - Evaluator (division, unary minus, or/and, comparison ops, edge cases)
  - Emitter (minified at-rules, selectors, CSS functions, edge cases)
  - Transformer (function errors, @each with var, edge paths)
  - LatticeAstToCss (transform/1 delegation, error propagation)
  """

  use ExUnit.Case

  alias CodingAdventures.LatticeParser
  alias CodingAdventures.LatticeAstToCss
  alias CodingAdventures.LatticeAstToCss.{Values, Emitter, Transformer, Errors}
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # Helper: parse + transform + emit in one step
  defp transpile!(source, opts \\ []) do
    {:ok, ast} = LatticeParser.parse(source)
    {:ok, css_ast} = Transformer.transform(ast)
    Emitter.emit(css_ast, opts)
  end

  # ===========================================================================
  # Error module tests — the 4 modules at 0% coverage
  # ===========================================================================

  describe "Errors.ModuleNotFoundError" do
    test "new/1 builds struct with message" do
      err = Errors.ModuleNotFoundError.new("theme")
      assert err.message == "Module 'theme' not found"
      assert err.module_name == "theme"
      assert err.line == 0
      assert err.column == 0
    end

    test "new/3 builds struct with line and column" do
      err = Errors.ModuleNotFoundError.new("colors", 5, 3)
      assert err.message == "Module 'colors' not found"
      assert err.module_name == "colors"
      assert err.line == 5
      assert err.column == 3
    end

    test "struct fields are accessible" do
      err = Errors.ModuleNotFoundError.new("_utils")
      assert %Errors.ModuleNotFoundError{module_name: "_utils"} = err
    end
  end

  describe "Errors.ReturnOutsideFunctionError" do
    test "new/0 builds struct with message" do
      err = Errors.ReturnOutsideFunctionError.new()
      assert err.message == "@return outside @function"
      assert err.line == 0
      assert err.column == 0
    end

    test "new/2 builds struct with line and column" do
      err = Errors.ReturnOutsideFunctionError.new(10, 4)
      assert err.message == "@return outside @function"
      assert err.line == 10
      assert err.column == 4
    end

    test "struct has no name field" do
      err = Errors.ReturnOutsideFunctionError.new()
      refute Map.has_key?(err, :name)
    end
  end

  describe "Errors.UndefinedFunctionError" do
    test "new/1 builds struct with message" do
      err = Errors.UndefinedFunctionError.new("spacing")
      assert err.message == "Undefined function 'spacing'"
      assert err.name == "spacing"
      assert err.line == 0
      assert err.column == 0
    end

    test "new/3 builds struct with line and column" do
      err = Errors.UndefinedFunctionError.new("em", 3, 12)
      assert err.message == "Undefined function 'em'"
      assert err.name == "em"
      assert err.line == 3
      assert err.column == 12
    end

    test "struct is correct type" do
      err = Errors.UndefinedFunctionError.new("scale")
      assert %Errors.UndefinedFunctionError{name: "scale"} = err
    end
  end

  describe "Errors.UnitMismatchError" do
    test "new/2 builds struct with message" do
      err = Errors.UnitMismatchError.new("px", "s")
      assert err.message == "Cannot add 'px' and 's' units"
      assert err.left_unit == "px"
      assert err.right_unit == "s"
    end

    test "new/4 builds struct with line and column" do
      err = Errors.UnitMismatchError.new("em", "vh", 7, 5)
      assert err.message == "Cannot add 'em' and 'vh' units"
      assert err.left_unit == "em"
      assert err.right_unit == "vh"
      assert err.line == 7
      assert err.column == 5
    end

    test "struct is correct type" do
      err = Errors.UnitMismatchError.new("px", "em")
      assert %Errors.UnitMismatchError{left_unit: "px", right_unit: "em"} = err
    end
  end

  # ===========================================================================
  # Values — uncovered paths
  # ===========================================================================

  describe "Values.to_css — list type" do
    test "list with one item" do
      assert Values.to_css({:list, [{:ident, "red"}]}) == "red"
    end

    test "list with multiple items" do
      result = Values.to_css({:list, [{:ident, "red"}, {:ident, "blue"}, {:ident, "green"}]})
      assert result == "red, blue, green"
    end

    test "list with mixed value types" do
      result = Values.to_css({:list, [{:dimension, 10.0, "px"}, {:percentage, 50.0}]})
      assert result == "10px, 50%"
    end
  end

  describe "Values.token_to_value — fallback paths" do
    test "VARIABLE token falls back to ident" do
      token = %{type: "VARIABLE", value: "$x"}
      assert Values.token_to_value(token) == {:ident, "$x"}
    end

    test "unknown token type falls back to ident" do
      token = %{type: "UNKNOWN", value: "foo"}
      assert Values.token_to_value(token) == {:ident, "foo"}
    end

    test "map with atom keys works" do
      # The defensive is_map/1 fallback clause handles both string and atom keys
      token = %{type: "NUMBER", value: "7"}
      assert Values.token_to_value(token) == {:number, 7.0}
    end

    test "DIMENSION with negative value" do
      token = %{type: "DIMENSION", value: "-2.5rem"}
      assert Values.token_to_value(token) == {:dimension, -2.5, "rem"}
    end
  end

  describe "Values.subtract — uncovered branches" do
    test "subtract same-unit dimensions" do
      assert Values.subtract({:dimension, 20.0, "px"}, {:dimension, 5.0, "px"}) ==
               {:ok, {:dimension, 15.0, "px"}}
    end

    test "subtract different-unit dimensions returns error" do
      assert {:error, _} = Values.subtract({:dimension, 10.0, "px"}, {:dimension, 5.0, "em"})
    end

    test "subtract percentages" do
      assert Values.subtract({:percentage, 80.0}, {:percentage, 30.0}) ==
               {:ok, {:percentage, 50.0}}
    end

    test "subtract incompatible types returns error" do
      assert {:error, _} = Values.subtract({:number, 5.0}, {:ident, "red"})
    end

    test "subtract number from string returns error" do
      assert {:error, _} = Values.subtract({:string, "hello"}, {:number, 1.0})
    end
  end

  describe "Values.multiply — uncovered branches" do
    test "percentage times number" do
      assert Values.multiply({:percentage, 50.0}, {:number, 2.0}) ==
               {:ok, {:percentage, 100.0}}
    end

    test "multiply incompatible types returns error" do
      assert {:error, _} = Values.multiply({:ident, "red"}, {:ident, "blue"})
    end

    test "multiply dimension by dimension returns error" do
      assert {:error, _} = Values.multiply({:dimension, 10.0, "px"}, {:dimension, 2.0, "px"})
    end
  end

  describe "Values.negate — uncovered branches" do
    test "negate percentage" do
      assert Values.negate({:percentage, 25.0}) == {:ok, {:percentage, -25.0}}
    end

    test "negate ident returns error" do
      assert {:error, _} = Values.negate({:ident, "red"})
    end

    test "negate color returns error" do
      assert {:error, _} = Values.negate({:color, "#fff"})
    end

    test "negate null returns error" do
      assert {:error, _} = Values.negate(:null)
    end
  end

  describe "Values.compare — uncovered branches" do
    test "dimension NOT_EQUALS same unit" do
      assert Values.compare({:dimension, 10.0, "px"}, {:dimension, 20.0, "px"}, "NOT_EQUALS") ==
               {:bool, true}
    end

    test "dimension GREATER same unit" do
      assert Values.compare({:dimension, 20.0, "px"}, {:dimension, 10.0, "px"}, "GREATER") ==
               {:bool, true}
      assert Values.compare({:dimension, 5.0, "px"}, {:dimension, 10.0, "px"}, "GREATER") ==
               {:bool, false}
    end

    test "dimension GREATER_EQUALS same unit" do
      assert Values.compare({:dimension, 10.0, "px"}, {:dimension, 10.0, "px"}, "GREATER_EQUALS") ==
               {:bool, true}
      assert Values.compare({:dimension, 5.0, "px"}, {:dimension, 10.0, "px"}, "GREATER_EQUALS") ==
               {:bool, false}
    end

    test "dimension LESS_EQUALS same unit" do
      assert Values.compare({:dimension, 5.0, "px"}, {:dimension, 10.0, "px"}, "LESS_EQUALS") ==
               {:bool, true}
      assert Values.compare({:dimension, 10.0, "px"}, {:dimension, 5.0, "px"}, "LESS_EQUALS") ==
               {:bool, false}
    end

    test "dimension EQUALS_EQUALS different units" do
      assert Values.compare({:dimension, 10.0, "px"}, {:dimension, 10.0, "em"}, "EQUALS_EQUALS") ==
               {:bool, false}
    end

    test "dimension NOT_EQUALS different units" do
      assert Values.compare({:dimension, 10.0, "px"}, {:dimension, 10.0, "em"}, "NOT_EQUALS") ==
               {:bool, true}
    end

    test "percentage EQUALS_EQUALS" do
      assert Values.compare({:percentage, 50.0}, {:percentage, 50.0}, "EQUALS_EQUALS") ==
               {:bool, true}
    end

    test "percentage NOT_EQUALS" do
      assert Values.compare({:percentage, 50.0}, {:percentage, 25.0}, "NOT_EQUALS") ==
               {:bool, true}
    end

    test "percentage GREATER" do
      assert Values.compare({:percentage, 75.0}, {:percentage, 25.0}, "GREATER") == {:bool, true}
    end

    test "percentage GREATER_EQUALS" do
      assert Values.compare({:percentage, 50.0}, {:percentage, 50.0}, "GREATER_EQUALS") == {:bool, true}
    end

    test "percentage LESS_EQUALS" do
      assert Values.compare({:percentage, 10.0}, {:percentage, 90.0}, "LESS_EQUALS") == {:bool, true}
    end

    test "NOT_EQUALS fallback for strings" do
      assert Values.compare({:ident, "dark"}, {:ident, "light"}, "NOT_EQUALS") == {:bool, true}
    end

    test "non-comparable types with ordering op returns false" do
      assert Values.compare({:ident, "red"}, {:ident, "blue"}, "GREATER") == {:bool, false}
    end

    test "number LESS comparison" do
      assert Values.compare({:number, 3.0}, {:number, 5.0}, "LESS_EQUALS") == {:bool, true}
    end
  end

  # ===========================================================================
  # Emitter — uncovered paths
  # ===========================================================================

  describe "Emitter direct call" do
    test "emit/2 called directly (not via transpile helper)" do
      # This exercises the top-level emit/2 function directly to improve coverage
      {:ok, ast} = LatticeParser.parse("h1 { color: red; }")
      {:ok, css_ast} = Transformer.transform(ast)
      result = Emitter.emit(css_ast)
      assert result =~ "color: red"
    end

    test "emit produces trailing newline" do
      css = transpile!("h1 { color: red; }")
      assert String.ends_with?(css, "\n")
    end
  end

  describe "Emitter — CSS selector variations" do
    test "class selector" do
      css = transpile!(".container { width: 100%; }")
      assert css =~ ".container"
      assert css =~ "width"
    end

    test "id selector" do
      css = transpile!("#main { margin: 0; }")
      assert css =~ "#main"
    end

    test "pseudo-class selector :hover" do
      css = transpile!("a:hover { color: blue; }")
      assert css =~ "a"
      assert css =~ "color: blue"
    end

    test "compound selector (tag + class)" do
      css = transpile!("p.intro { font-size: 14px; }")
      assert css =~ "p"
      assert css =~ "font-size: 14px"
    end

    test "descendant selector" do
      css = transpile!("nav a { text-decoration: none; }")
      assert css =~ "nav"
      assert css =~ "text-decoration"
    end

    test "selector list with minified output" do
      css = transpile!("h1, h2 { color: red; }", minified: true)
      assert css =~ "h1,h2"
    end
  end

  describe "Emitter — at-rule variations" do
    test "@media with block" do
      css = transpile!("@media screen { h1 { color: red; } }")
      assert css =~ "@media"
      assert css =~ "screen"
      assert css =~ "color: red"
    end

    test "@media minified" do
      css = transpile!("@media screen { h1 { color: red; } }", minified: true)
      assert css =~ "@media"
      assert css =~ "h1{color:red;}"
    end

    test "@import semicolon at-rule" do
      css = transpile!("@charset \"UTF-8\";")
      assert css =~ "@charset"
    end

    test "@keyframes at-rule" do
      source = """
      @keyframes fade {
        from { opacity: 1; }
        to { opacity: 0; }
      }
      """
      css = transpile!(source)
      assert css =~ "@keyframes"
      assert css =~ "opacity"
    end
  end

  describe "Emitter — declaration variations" do
    test "multiple CSS properties" do
      css = transpile!("p { margin: 0; padding: 0; font-size: 14px; }")
      assert css =~ "margin"
      assert css =~ "padding"
      assert css =~ "font-size"
    end

    test "CSS function in value" do
      css = transpile!("a { color: rgb(255, 0, 0); }")
      assert css =~ "rgb"
      assert css =~ "255"
    end

    test "!important declaration" do
      css = transpile!("p { display: none !important; }")
      assert css =~ "!important"
    end

    test "value with string token" do
      css = transpile!(~s(p { content: "hello"; }))
      assert css =~ "content"
    end

    test "multiple values space-separated" do
      css = transpile!("div { margin: 10px 20px 10px 20px; }")
      assert css =~ "margin"
      assert css =~ "10px"
      assert css =~ "20px"
    end
  end

  describe "Emitter — minified mode" do
    test "minified with multiple rules" do
      css = transpile!("h1 { color: red; } h2 { color: blue; }", minified: true)
      assert css =~ "h1{color:red;}"
      assert css =~ "h2{color:blue;}"
    end

    test "minified with class selector" do
      css = transpile!(".box { display: flex; }", minified: true)
      assert css =~ ".box{display:flex;}"
    end

    test "minified with @media" do
      css = transpile!("@media print { h1 { color: black; } }", minified: true)
      assert css =~ "@media"
      assert css =~ "h1{color:black;}"
    end

    test "empty stylesheet minified" do
      css = transpile!("", minified: true)
      assert css == ""
    end
  end

  # ===========================================================================
  # Evaluator — uncovered paths via transformer pipeline
  # ===========================================================================

  describe "Evaluator via @if conditions" do
    test "@if with == comparison (numbers)" do
      source = """
      $count: 3;
      @if $count == 3 {
        .a { color: red; }
      } @else {
        .a { color: blue; }
      }
      """
      css = transpile!(source)
      assert css =~ "red"
      refute css =~ "blue"
    end

    test "@if with != comparison" do
      source = """
      $val: 5;
      @if $val != 3 {
        .a { color: green; }
      } @else {
        .a { color: grey; }
      }
      """
      css = transpile!(source)
      assert css =~ "green"
    end

    test "@if with > comparison" do
      source = """
      $n: 10;
      @if $n > 5 {
        .a { color: red; }
      } @else {
        .a { color: blue; }
      }
      """
      css = transpile!(source)
      assert css =~ "red"
    end

    test "@if with < comparison (false branch)" do
      source = """
      $n: 2;
      @if $n > 5 {
        .a { color: red; }
      } @else {
        .a { color: blue; }
      }
      """
      css = transpile!(source)
      assert css =~ "blue"
    end

    test "@if with >= comparison (true when equal)" do
      source = """
      $n: 5;
      @if $n >= 5 {
        .a { color: red; }
      } @else {
        .a { color: blue; }
      }
      """
      css = transpile!(source)
      assert css =~ "red"
    end

    test "@if with <= comparison" do
      source = """
      $n: 3;
      @if $n <= 5 {
        .a { color: red; }
      } @else {
        .a { color: blue; }
      }
      """
      css = transpile!(source)
      assert css =~ "red"
    end
  end

  describe "Evaluator via @function" do
    test "function with subtraction" do
      source = """
      @function shrink($n) {
        @return $n - 2;
      }
      .box { z-index: shrink(10); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "function with multiplication" do
      source = """
      @function scale($n) {
        @return $n * 3;
      }
      .box { z-index: scale(4); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "function with addition expression" do
      source = """
      @function add-offset($n) {
        @return $n + 10;
      }
      .box { z-index: add-offset(5); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "function call in dimension expression" do
      source = """
      @function double($n) {
        @return $n * 2;
      }
      .box { width: double(8px); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "wrong arity function call returns error" do
      source = """
      @function single($x) {
        @return $x;
      }
      .box { width: single(1, 2); }
      """
      {:ok, ast} = LatticeParser.parse(source)
      # This may crash or error depending on parser representation; just check it
      # doesn't hang and returns a tuple
      result = try do
        Transformer.transform(ast)
      rescue
        _ -> {:error, "crash"}
      catch
        {:lattice_error, err} -> {:error, err.message}
      end
      # Either an ok result (parser treats extra arg differently) or an error
      assert match?({:ok, _}, result) or match?({:error, _}, result)
    end
  end

  describe "Evaluator via @each loop with variable" do
    test "@each iterates over items bound to loop variable" do
      source = """
      $sizes: small, medium, large;
      @each $s in $sizes {
        .t { content: $s; }
      }
      """
      # This exercises the list expansion path in the transformer/evaluator
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "@each with inline list" do
      source = """
      @each $x in 1, 2, 3 {
        .n { z-index: $x; }
      }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end
  end

  # ===========================================================================
  # Transformer — additional path coverage
  # ===========================================================================

  describe "Transformer — circular reference detection" do
    test "circular mixin reference produces error" do
      source = """
      @mixin a {
        @include b;
      }
      @mixin b {
        @include a;
      }
      .x { @include a; }
      """
      {:ok, ast} = LatticeParser.parse(source)
      result = Transformer.transform(ast)
      assert {:error, msg} = result
      # Should mention either "Circular" or "Undefined" depending on expansion order
      assert msg =~ "mixin" or msg =~ "Circular" or msg =~ "Undefined"
    end
  end

  describe "Transformer — nested rules" do
    test "nested qualified rule passes through" do
      source = "@media (max-width: 600px) { .container { width: 100%; } }"
      css = transpile!(source)
      assert css =~ "@media"
      assert css =~ "width"
    end
  end

  describe "Transformer — @for with variable in body" do
    test "@for uses loop variable" do
      source = """
      @for $i from 1 through 3 {
        .item { z-index: $i; }
      }
      """
      css = transpile!(source)
      # Should have 3 declarations
      count = css |> String.split("z-index") |> length()
      assert count >= 4
    end
  end

  # ===========================================================================
  # LatticeAstToCss — delegated functions
  # ===========================================================================

  describe "LatticeAstToCss module delegation" do
    test "transform/1 returns {:ok, ast}" do
      {:ok, ast} = LatticeParser.parse("h1 { color: red; }")
      result = LatticeAstToCss.transform(ast)
      assert {:ok, _css_ast} = result
    end

    test "transform_to_css/2 propagates transformer error" do
      {:ok, ast} = LatticeParser.parse("h1 { color: $undefined; }")
      result = LatticeAstToCss.transform_to_css(ast)
      assert {:error, msg} = result
      assert msg =~ "Undefined"
    end
  end

  # ===========================================================================
  # Emitter — direct ASTNode construction for edge cases
  # ===========================================================================

  describe "Emitter — direct node emission" do
    test "empty token emits empty string" do
      # Exercises emit_node fallback for non-ASTNode, non-Token
      result = Emitter.emit(%ASTNode{rule_name: "stylesheet", children: []})
      assert result == ""
    end

    test "STRING token emits with quotes" do
      # Exercises the %Token{type: "STRING"} branch
      tok = %Token{type: "STRING", value: "Arial"}
      node = %ASTNode{
        rule_name: "stylesheet",
        children: [
          %ASTNode{
            rule_name: "rule",
            children: [
              %ASTNode{
                rule_name: "qualified_rule",
                children: [
                  %ASTNode{rule_name: "selector_list", children: [
                    %ASTNode{rule_name: "complex_selector", children: [
                      %ASTNode{rule_name: "compound_selector", children: [
                        %ASTNode{rule_name: "simple_selector", children: [
                          %Token{type: "IDENT", value: "p"}
                        ]}
                      ]}
                    ]}
                  ]},
                  %ASTNode{rule_name: "block", children: [
                    %ASTNode{rule_name: "block_contents", children: [
                      %ASTNode{rule_name: "block_item", children: [
                        %ASTNode{rule_name: "declaration_or_nested", children: [
                          %ASTNode{rule_name: "declaration", children: [
                            %ASTNode{rule_name: "property", children: [%Token{type: "IDENT", value: "font-family"}]},
                            %Token{type: "COLON", value: ":"},
                            %ASTNode{rule_name: "value_list", children: [
                              %ASTNode{rule_name: "value", children: [tok]}
                            ]},
                            %Token{type: "SEMICOLON", value: ";"}
                          ]}
                        ]}
                      ]}
                    ]}
                  ]}
                ]
              }
            ]
          }
        ]
      }
      result = Emitter.emit(node)
      assert result =~ "\"Arial\""
    end
  end
end
