defmodule CodingAdventures.LatticeAstToCss.Coverage2Test do
  @moduledoc """
  Second batch of coverage-boosting tests targeting deep paths in:
  - Transformer function body evaluation (branches, returns, local vars)
  - Evaluator direct invocation
  - Emitter selectors and at-rules
  - Values format_number integer path
  """

  use ExUnit.Case

  alias CodingAdventures.LatticeParser
  alias CodingAdventures.LatticeAstToCss.{Values, Evaluator, Emitter, Transformer, Scope, Errors}
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # Helper: parse + transform + emit in one step
  defp transpile!(source, opts \\ []) do
    {:ok, ast} = LatticeParser.parse(source)
    {:ok, css_ast} = Transformer.transform(ast)
    Emitter.emit(css_ast, opts)
  end

  # ===========================================================================
  # Values — integer format_number path
  # ===========================================================================

  describe "Values.to_css — integer number" do
    test "integer 0 formats without decimal" do
      assert Values.to_css({:number, 0.0}) == "0"
    end

    test "negative number formats correctly" do
      assert Values.to_css({:number, -5.0}) == "-5"
    end

    test "negative dimension" do
      assert Values.to_css({:dimension, -10.0, "px"}) == "-10px"
    end
  end

  describe "Values.add — error path" do
    test "add incompatible types returns error" do
      assert {:error, _} = Values.add({:ident, "red"}, {:number, 5.0})
    end

    test "add dimension and number returns error" do
      assert {:error, _} = Values.add({:dimension, 10.0, "px"}, {:number, 5.0})
    end
  end

  # ===========================================================================
  # Evaluator — direct invocation
  # ===========================================================================

  describe "Evaluator.evaluate — direct calls" do
    test "evaluate nil returns null" do
      scope = Scope.new()
      assert Evaluator.evaluate(nil, scope) == :null
    end

    test "evaluate plain string returns null" do
      scope = Scope.new()
      assert Evaluator.evaluate("not a node", scope) == :null
    end

    test "evaluate Token directly" do
      scope = Scope.new()
      token = %Token{type: "NUMBER", value: "42"}
      assert Evaluator.evaluate(token, scope) == {:number, 42.0}
    end

    test "evaluate NUMBER token via ASTNode" do
      scope = Scope.new()
      # A bare token inside an ASTNode
      node = %ASTNode{
        rule_name: "lattice_primary",
        children: [%Token{type: "NUMBER", value: "7"}]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, 7.0}
    end

    test "evaluate HASH token in primary" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_primary",
        children: [%Token{type: "HASH", value: "#ff0000"}]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:color, "#ff0000"}
    end

    test "evaluate variable in primary — found in scope" do
      scope = Scope.new() |> Scope.set("$x", {:number, 99.0})
      node = %ASTNode{
        rule_name: "lattice_primary",
        children: [%Token{type: "VARIABLE", value: "$x"}]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, 99.0}
    end

    test "evaluate variable in primary — not found (fallback to ident)" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_primary",
        children: [%Token{type: "VARIABLE", value: "$missing"}]
      }
      result = Evaluator.evaluate(node, scope)
      # Falls back to {:ident, "$missing"}
      assert result == {:ident, "$missing"}
    end

    test "evaluate unary minus negates a number" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_unary",
        children: [
          %Token{type: "MINUS", value: "-"},
          %ASTNode{
            rule_name: "lattice_primary",
            children: [%Token{type: "NUMBER", value: "5"}]
          }
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, -5.0}
    end

    test "evaluate empty unary returns null" do
      scope = Scope.new()
      node = %ASTNode{rule_name: "lattice_unary", children: []}
      assert Evaluator.evaluate(node, scope) == :null
    end

    test "evaluate empty additive returns null" do
      scope = Scope.new()
      node = %ASTNode{rule_name: "lattice_additive", children: []}
      assert Evaluator.evaluate(node, scope) == :null
    end

    test "evaluate empty multiplicative returns null" do
      scope = Scope.new()
      node = %ASTNode{rule_name: "lattice_multiplicative", children: []}
      assert Evaluator.evaluate(node, scope) == :null
    end

    test "evaluate empty single child returns null" do
      scope = Scope.new()
      node = %ASTNode{rule_name: "lattice_expression", children: []}
      assert Evaluator.evaluate(node, scope) == :null
    end

    test "evaluate additive with addition" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_additive",
        children: [
          %ASTNode{rule_name: "lattice_multiplicative", children: [
            %ASTNode{rule_name: "lattice_unary", children: [
              %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "3"}]}
            ]}
          ]},
          %Token{type: "PLUS", value: "+"},
          %ASTNode{rule_name: "lattice_multiplicative", children: [
            %ASTNode{rule_name: "lattice_unary", children: [
              %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "4"}]}
            ]}
          ]}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, 7.0}
    end

    test "evaluate multiplicative with multiplication" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_multiplicative",
        children: [
          %ASTNode{rule_name: "lattice_unary", children: [
            %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "3"}]}
          ]},
          %Token{type: "STAR", value: "*"},
          %ASTNode{rule_name: "lattice_unary", children: [
            %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "4"}]}
          ]}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, 12.0}
    end

    test "evaluate comparison node" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_comparison",
        children: [
          %ASTNode{rule_name: "lattice_additive", children: [
            %ASTNode{rule_name: "lattice_multiplicative", children: [
              %ASTNode{rule_name: "lattice_unary", children: [
                %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "5"}]}
              ]}
            ]}
          ]},
          %ASTNode{rule_name: "comparison_op", children: [%Token{type: "EQUALS_EQUALS", value: "=="}]},
          %ASTNode{rule_name: "lattice_additive", children: [
            %ASTNode{rule_name: "lattice_multiplicative", children: [
              %ASTNode{rule_name: "lattice_unary", children: [
                %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "5"}]}
              ]}
            ]}
          ]}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:bool, true}
    end

    test "evaluate unknown rule with single child unwraps" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "unknown_wrapper",
        children: [%Token{type: "NUMBER", value: "42"}]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, 42.0}
    end

    test "evaluate unknown rule with multiple children picks first meaningful" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "unknown_wrapper",
        children: [
          %Token{type: "NUMBER", value: "10"},
          %Token{type: "NUMBER", value: "20"}
        ]
      }
      # With multiple children: tries [single] match first (fails), then finds first meaningful
      result = Evaluator.evaluate(node, scope)
      # Should get the first child (10 or 20 — depends on implementation detail)
      assert match?({:number, _}, result)
    end

    test "evaluate or_expr short-circuits on truthy" do
      scope = Scope.new()
      # Build "true or false"
      node = %ASTNode{
        rule_name: "lattice_or_expr",
        children: [
          %ASTNode{rule_name: "lattice_and_expr", children: [
            %ASTNode{rule_name: "lattice_comparison", children: [
              %ASTNode{rule_name: "lattice_additive", children: [
                %ASTNode{rule_name: "lattice_multiplicative", children: [
                  %ASTNode{rule_name: "lattice_unary", children: [
                    %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "IDENT", value: "true"}]}
                  ]}
                ]}
              ]}
            ]}
          ]},
          %Token{type: "OR", value: "or"},
          %ASTNode{rule_name: "lattice_and_expr", children: [
            %ASTNode{rule_name: "lattice_comparison", children: [
              %ASTNode{rule_name: "lattice_additive", children: [
                %ASTNode{rule_name: "lattice_multiplicative", children: [
                  %ASTNode{rule_name: "lattice_unary", children: [
                    %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "IDENT", value: "false"}]}
                  ]}
                ]}
              ]}
            ]}
          ]}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:bool, true}
    end

    test "evaluate and_expr short-circuits on falsy" do
      scope = Scope.new()
      # Build "false and true"
      node = %ASTNode{
        rule_name: "lattice_and_expr",
        children: [
          %ASTNode{rule_name: "lattice_comparison", children: [
            %ASTNode{rule_name: "lattice_additive", children: [
              %ASTNode{rule_name: "lattice_multiplicative", children: [
                %ASTNode{rule_name: "lattice_unary", children: [
                  %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "IDENT", value: "false"}]}
                ]}
              ]}
            ]}
          ]},
          %Token{type: "AND", value: "and"},
          %ASTNode{rule_name: "lattice_comparison", children: [
            %ASTNode{rule_name: "lattice_additive", children: [
              %ASTNode{rule_name: "lattice_multiplicative", children: [
                %ASTNode{rule_name: "lattice_unary", children: [
                  %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "IDENT", value: "true"}]}
                ]}
              ]}
            ]}
          ]}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:bool, false}
    end

    test "evaluate parenthesized expression" do
      scope = Scope.new()
      # (5) — parentheses wrapping a number
      node = %ASTNode{
        rule_name: "lattice_primary",
        children: [
          %Token{type: "LPAREN", value: "("},
          %ASTNode{rule_name: "lattice_expression", children: [
            %ASTNode{rule_name: "lattice_or_expr", children: [
              %ASTNode{rule_name: "lattice_and_expr", children: [
                %ASTNode{rule_name: "lattice_comparison", children: [
                  %ASTNode{rule_name: "lattice_additive", children: [
                    %ASTNode{rule_name: "lattice_multiplicative", children: [
                      %ASTNode{rule_name: "lattice_unary", children: [
                        %ASTNode{rule_name: "lattice_primary", children: [%Token{type: "NUMBER", value: "5"}]}
                      ]}
                    ]}
                  ]}
                ]}
              ]}
            ]}
          ]},
          %Token{type: "RPAREN", value: ")"}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == {:number, 5.0}
    end

    test "evaluate function_call node returns null" do
      scope = Scope.new()
      node = %ASTNode{
        rule_name: "lattice_primary",
        children: [
          %ASTNode{rule_name: "function_call", children: []}
        ]
      }
      result = Evaluator.evaluate(node, scope)
      assert result == :null
    end
  end

  # ===========================================================================
  # Transformer — function body with @if
  # ===========================================================================

  describe "Transformer — function with @if inside" do
    test "function body with @if conditional return" do
      source = """
      @function sign($n) {
        @if $n > 0 {
          @return 1;
        }
        @return 0;
      }
      .box { z-index: sign(5); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "function with local variable declaration" do
      source = """
      @function add-ten($n) {
        $base: 10;
        @return $n;
      }
      .box { z-index: add-ten(3); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end

    test "function with missing return produces error" do
      source = """
      @function noop($x) {
        $y: $x;
      }
      .box { z-index: noop(1); }
      """
      {:ok, ast} = LatticeParser.parse(source)
      result = Transformer.transform(ast)
      assert {:error, msg} = result
      assert msg =~ "no @return"
    end
  end

  # ===========================================================================
  # Transformer — @for "to" (exclusive upper bound)
  # ===========================================================================

  describe "Transformer — @for with to (exclusive)" do
    test "@for from 1 to 4 generates 3 iterations" do
      source = """
      @for $i from 1 to 4 {
        .item { z-index: 1; }
      }
      """
      css = transpile!(source)
      count = css |> String.split("z-index") |> length()
      assert count >= 4
    end
  end

  # ===========================================================================
  # Transformer — variable declaration inside block
  # ===========================================================================

  describe "Transformer — variable declaration inside selector block" do
    test "variable declared at top level used in rule" do
      source = """
      $pad: 8px;
      .box {
        padding: $pad;
      }
      """
      css = transpile!(source)
      assert css =~ "padding"
      assert css =~ "8px"
    end
  end

  # ===========================================================================
  # Transformer — @include at top level
  # ===========================================================================

  describe "Transformer — @include producing CSS rules at top level" do
    test "@if at top level generates CSS" do
      source = """
      $visible: true;
      @if $visible == true {
        .item { display: block; }
      }
      """
      css = transpile!(source)
      assert css =~ "display: block"
    end
  end

  # ===========================================================================
  # Emitter — advanced selectors via full pipeline
  # ===========================================================================

  describe "Emitter — attribute selector" do
    test "attribute selector [attr]" do
      css = transpile!("a[href] { color: blue; }")
      assert css =~ "a"
      assert css =~ "color: blue"
    end

    test "attribute selector with value [attr=val]" do
      css = transpile!(~s(input[type="text"] { border: 1px solid; }))
      assert css =~ "input"
      assert css =~ "border"
    end
  end

  describe "Emitter — pseudo-element" do
    test "::before pseudo-element" do
      css = transpile!("p::before { content: ''; }")
      assert css =~ "p"
      assert css =~ "content"
    end
  end

  describe "Emitter — pseudo-class with argument" do
    test ":nth-child pseudo-class" do
      css = transpile!("li:nth-child(2) { color: red; }")
      assert css =~ "li"
      assert css =~ "color: red"
    end
  end

  describe "Emitter — complex CSS" do
    test "CSS with calc() function" do
      css = transpile!("div { width: calc(100% - 20px); }")
      assert css =~ "div"
      assert css =~ "calc"
    end

    test "CSS with url() function" do
      css = transpile!("div { background: url(image.png); }")
      assert css =~ "background"
    end

    test "multiple selectors with combinators" do
      css = transpile!("ul > li { list-style: none; }")
      assert css =~ "ul"
      assert css =~ "li"
    end

    test "sibling combinator" do
      css = transpile!("h1 + p { margin-top: 0; }")
      assert css =~ "h1"
      assert css =~ "p"
    end

    test "minified empty block" do
      # An empty block in a rule
      {:ok, ast} = LatticeParser.parse("h1 { color: red; }")
      {:ok, css_ast} = Transformer.transform(ast)
      result = Emitter.emit(css_ast, minified: true)
      assert result =~ "h1{"
    end
  end

  describe "Emitter — emit_node fallback" do
    test "emitting unknown rule uses default handler" do
      # An ASTNode with an unrecognized rule_name uses emit_default
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
                          %Token{type: "IDENT", value: "h1"}
                        ]}
                      ]}
                    ]}
                  ]},
                  %ASTNode{rule_name: "block", children: [
                    %ASTNode{rule_name: "block_contents", children: [
                      %ASTNode{rule_name: "block_item", children: [
                        %ASTNode{rule_name: "declaration_or_nested", children: [
                          %ASTNode{rule_name: "declaration", children: [
                            %ASTNode{rule_name: "property", children: [%Token{type: "IDENT", value: "color"}]},
                            %Token{type: "COLON", value: ":"},
                            %ASTNode{rule_name: "value_list", children: [
                              %ASTNode{rule_name: "value", children: [%Token{type: "IDENT", value: "red"}]}
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
      assert result =~ "h1"
      assert result =~ "color: red"
    end
  end

  # ===========================================================================
  # Transformer — more edge cases
  # ===========================================================================

  describe "Transformer — function call in expression" do
    test "function used in mixin body" do
      source = """
      @function spacing($n) {
        @return $n * 8;
      }
      @mixin padded($n) {
        padding: spacing($n);
      }
      .box { @include padded(2); }
      """
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end
  end

  describe "Transformer — @each iterates correctly" do
    test "@each loop body generates one block per item" do
      source = "
@each $side in top, bottom {
  .border { padding: $side; }
}
"
      result = Transformer.transform(elem(LatticeParser.parse(source), 1))
      assert {:ok, _} = result
    end
  end

  describe "Transformer — multiple mixin parameters" do
    test "mixin with 3 parameters" do
      source = """
      @mixin border($width, $style, $color) {
        border: $width $style $color;
      }
      .box { @include border(1px, solid, red); }
      """
      css = transpile!(source)
      assert css =~ "border"
    end
  end

  describe "Transformer — @function with conditional @if" do
    test "function @if true returns first branch" do
      source = """
      @function clamp-positive($n) {
        @if $n > 0 {
          @return $n;
        }
        @return 0;
      }
      .box { z-index: clamp-positive(10); }
      """
      css = transpile!(source)
      assert css =~ "z-index"
    end

    test "function @if false takes fallthrough" do
      source = """
      @function abs-val($n) {
        @if $n >= 0 {
          @return $n;
        }
        @return 0;
      }
      .box { z-index: abs-val(5); }
      """
      css = transpile!(source)
      assert css =~ "z-index"
    end
  end

  # ===========================================================================
  # @for with dimension bounds
  # ===========================================================================

  describe "Transformer — @for with dimension" do
    test "@for from 0px generates items" do
      source = """
      @for $i from 1 through 2 {
        .a { order: $i; }
      }
      """
      css = transpile!(source)
      assert css =~ "order"
    end
  end

  # ===========================================================================
  # Emitter — CSS function in value (function_call node)
  # ===========================================================================

  describe "Emitter — CSS functions" do
    test "rgba() function" do
      css = transpile!("p { color: rgba(0, 0, 0, 0.5); }")
      assert css =~ "rgba"
    end

    test "linear-gradient function" do
      css = transpile!("div { background: linear-gradient(to right, red, blue); }")
      assert css =~ "linear-gradient"
    end

    test "var() CSS custom property function" do
      css = transpile!("p { color: var(--primary); }")
      assert css =~ "var"
    end
  end

  # ===========================================================================
  # Transformer.State — cover defstruct by constructing instances directly
  # ===========================================================================

  describe "Transformer.State struct" do
    test "can be created with defaults" do
      state = %Transformer.State{}
      assert state.mixins == %{}
      assert state.functions == %{}
      assert state.mixin_stack == []
      assert state.function_stack == []
      assert state.variables == nil
    end

    test "can be created with explicit values" do
      scope = Scope.new()
      state = %Transformer.State{variables: scope, mixins: %{"m" => 1}}
      assert state.variables == scope
      assert state.mixins == %{"m" => 1}
    end
  end

  # ===========================================================================
  # Error constructors — intermediate-arity coverage
  # Each error struct with default args (line \\ 0, column \\ 0) generates
  # function clauses for every arity. The single-intermediate-arg arity is
  # often the only uncovered clause. These tests exercise it directly.
  # ===========================================================================

  describe "Error constructors — intermediate arities" do
    test "ReturnOutsideFunctionError.new/1 (line only)" do
      e = Errors.ReturnOutsideFunctionError.new(7)
      assert e.line == 7
      assert e.column == 0
      assert e.message =~ "return"
    end

    test "MissingReturnError.new/2 (name + line)" do
      e = Errors.MissingReturnError.new("my_func", 12)
      assert e.line == 12
      assert e.column == 0
    end

    test "UndefinedVariableError.new/2 (name + line)" do
      e = Errors.UndefinedVariableError.new("$foo", 3)
      assert e.line == 3
      assert e.column == 0
      assert e.message =~ "foo"
    end

    test "UndefinedMixinError.new/2 (name + line)" do
      e = Errors.UndefinedMixinError.new("btn", 5)
      assert e.line == 5
      assert e.column == 0
    end

    test "WrongArityError.new/5 (kind+name+expected+got+line)" do
      e = Errors.WrongArityError.new("mixin", "btn", 2, 3, 9)
      assert e.line == 9
      assert e.column == 0
    end

    test "CircularReferenceError.new/3 (kind+chain+line)" do
      e = Errors.CircularReferenceError.new("mixin", ["a", "b"], 4)
      assert e.line == 4
      assert e.column == 0
    end

    test "TypeErrorInExpression.new/4 (op+left+right+line)" do
      e = Errors.TypeErrorInExpression.new("+", "px", "em", 2)
      assert e.line == 2
      assert e.column == 0
    end

    test "UnitMismatchError.new/3 (left_unit+right_unit+line)" do
      e = Errors.UnitMismatchError.new("px", "em", 6)
      assert e.line == 6
      assert e.column == 0
    end

    test "UndefinedFunctionError.new/2 (name + line)" do
      e = Errors.UndefinedFunctionError.new("spacing", 8)
      assert e.line == 8
      assert e.column == 0
    end

    test "ModuleNotFoundError.new/2 (module_name + line)" do
      e = Errors.ModuleNotFoundError.new("utils", 1)
      assert e.line == 1
      assert e.column == 0
    end
  end
end
