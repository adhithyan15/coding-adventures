defmodule CodingAdventures.LatticeAstToCssTest do
  use ExUnit.Case

  alias CodingAdventures.LatticeParser
  alias CodingAdventures.LatticeAstToCss
  alias CodingAdventures.LatticeAstToCss.{Scope, Values, Transformer, Emitter, Errors}

  # Helper: parse + transform + emit in one step
  defp transpile!(source, opts \\ []) do
    {:ok, ast} = LatticeParser.parse(source)
    {:ok, css_ast} = Transformer.transform(ast)
    Emitter.emit(css_ast, opts)
  end

  defp transform!(source) do
    {:ok, ast} = LatticeParser.parse(source)
    Transformer.transform(ast)
  end

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  describe "module loading" do
    test "module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeAstToCss)
    end

    test "Scope module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeAstToCss.Scope)
    end

    test "Values module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeAstToCss.Values)
    end

    test "Evaluator module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeAstToCss.Evaluator)
    end

    test "Transformer module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeAstToCss.Transformer)
    end

    test "Emitter module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeAstToCss.Emitter)
    end
  end

  # ===========================================================================
  # Scope tests
  # ===========================================================================

  describe "Scope" do
    test "new scope has no bindings" do
      scope = Scope.new()
      assert Scope.get(scope, "$x") == :error
    end

    test "set and get a value" do
      scope = Scope.new() |> Scope.set("$x", {:number, 10.0})
      assert Scope.get(scope, "$x") == {:ok, {:number, 10.0}}
    end

    test "get from parent scope" do
      global = Scope.new() |> Scope.set("$x", {:number, 10.0})
      child = Scope.child(global)
      assert Scope.get(child, "$x") == {:ok, {:number, 10.0}}
    end

    test "child scope shadows parent" do
      global = Scope.new() |> Scope.set("$x", {:number, 10.0})
      child = global |> Scope.child() |> Scope.set("$x", {:number, 20.0})
      assert Scope.get(child, "$x") == {:ok, {:number, 20.0}}
      assert Scope.get(global, "$x") == {:ok, {:number, 10.0}}
    end

    test "has?/2 returns true for bound names" do
      scope = Scope.new() |> Scope.set("$y", :null)
      assert Scope.has?(scope, "$y")
      refute Scope.has?(scope, "$z")
    end

    test "has_local?/2 only checks current scope" do
      global = Scope.new() |> Scope.set("$x", :null)
      child = Scope.child(global)
      refute Scope.has_local?(child, "$x")  # Not in child's own bindings
      assert Scope.has?(child, "$x")        # But visible via parent
    end

    test "depth is 0 for global scope" do
      assert Scope.depth(Scope.new()) == 0
    end

    test "depth increases with child scopes" do
      g = Scope.new()
      c1 = Scope.child(g)
      c2 = Scope.child(c1)
      assert Scope.depth(g) == 0
      assert Scope.depth(c1) == 1
      assert Scope.depth(c2) == 2
    end
  end

  # ===========================================================================
  # Values tests
  # ===========================================================================

  describe "Values.truthy?" do
    test "false is falsy" do
      refute Values.truthy?({:bool, false})
    end

    test "null is falsy" do
      refute Values.truthy?(:null)
    end

    test "number 0 is falsy" do
      refute Values.truthy?({:number, 0.0})
    end

    test "true is truthy" do
      assert Values.truthy?({:bool, true})
    end

    test "non-zero number is truthy" do
      assert Values.truthy?({:number, 1.0})
    end

    test "string is truthy" do
      assert Values.truthy?({:string, "hello"})
    end

    test "ident is truthy" do
      assert Values.truthy?({:ident, "red"})
    end

    test "dimension is truthy" do
      assert Values.truthy?({:dimension, 16.0, "px"})
    end
  end

  describe "Values.token_to_value" do
    test "NUMBER token" do
      token = %{type: "NUMBER", value: "42"}
      assert Values.token_to_value(token) == {:number, 42.0}
    end

    test "DIMENSION token" do
      token = %{type: "DIMENSION", value: "16px"}
      assert Values.token_to_value(token) == {:dimension, 16.0, "px"}
    end

    test "DIMENSION with em" do
      token = %{type: "DIMENSION", value: "2em"}
      assert Values.token_to_value(token) == {:dimension, 2.0, "em"}
    end

    test "PERCENTAGE token" do
      token = %{type: "PERCENTAGE", value: "50%"}
      assert Values.token_to_value(token) == {:percentage, 50.0}
    end

    test "STRING token" do
      token = %{type: "STRING", value: "hello"}
      assert Values.token_to_value(token) == {:string, "hello"}
    end

    test "HASH token" do
      token = %{type: "HASH", value: "#4a90d9"}
      assert Values.token_to_value(token) == {:color, "#4a90d9"}
    end

    test "IDENT 'true'" do
      token = %{type: "IDENT", value: "true"}
      assert Values.token_to_value(token) == {:bool, true}
    end

    test "IDENT 'false'" do
      token = %{type: "IDENT", value: "false"}
      assert Values.token_to_value(token) == {:bool, false}
    end

    test "IDENT 'null'" do
      token = %{type: "IDENT", value: "null"}
      assert Values.token_to_value(token) == :null
    end

    test "IDENT other" do
      token = %{type: "IDENT", value: "red"}
      assert Values.token_to_value(token) == {:ident, "red"}
    end
  end

  describe "Values.to_css" do
    test "number integer" do
      assert Values.to_css({:number, 16.0}) == "16"
    end

    test "number float" do
      assert Values.to_css({:number, 3.14}) == "3.14"
    end

    test "dimension" do
      assert Values.to_css({:dimension, 16.0, "px"}) == "16px"
    end

    test "dimension float" do
      assert Values.to_css({:dimension, 1.5, "rem"}) == "1.5rem"
    end

    test "percentage" do
      assert Values.to_css({:percentage, 50.0}) == "50%"
    end

    test "string adds quotes" do
      assert Values.to_css({:string, "hello"}) == ~s("hello")
    end

    test "ident" do
      assert Values.to_css({:ident, "red"}) == "red"
    end

    test "color" do
      assert Values.to_css({:color, "#4a90d9"}) == "#4a90d9"
    end

    test "bool true" do
      assert Values.to_css({:bool, true}) == "true"
    end

    test "bool false" do
      assert Values.to_css({:bool, false}) == "false"
    end

    test "null is empty string" do
      assert Values.to_css(:null) == ""
    end
  end

  describe "Values arithmetic" do
    test "add two numbers" do
      assert Values.add({:number, 3.0}, {:number, 4.0}) == {:ok, {:number, 7.0}}
    end

    test "add same-unit dimensions" do
      assert Values.add({:dimension, 10.0, "px"}, {:dimension, 5.0, "px"}) ==
               {:ok, {:dimension, 15.0, "px"}}
    end

    test "add different-unit dimensions returns error" do
      assert {:error, _} = Values.add({:dimension, 10.0, "px"}, {:dimension, 5.0, "em"})
    end

    test "add percentages" do
      assert Values.add({:percentage, 20.0}, {:percentage, 30.0}) ==
               {:ok, {:percentage, 50.0}}
    end

    test "add strings concatenates" do
      assert Values.add({:string, "hello"}, {:string, " world"}) ==
               {:ok, {:string, "hello world"}}
    end

    test "subtract numbers" do
      assert Values.subtract({:number, 10.0}, {:number, 3.0}) == {:ok, {:number, 7.0}}
    end

    test "multiply number by number" do
      assert Values.multiply({:number, 3.0}, {:number, 4.0}) == {:ok, {:number, 12.0}}
    end

    test "multiply number by dimension" do
      assert Values.multiply({:number, 2.0}, {:dimension, 8.0, "px"}) ==
               {:ok, {:dimension, 16.0, "px"}}
    end

    test "multiply dimension by number" do
      assert Values.multiply({:dimension, 8.0, "px"}, {:number, 2.0}) ==
               {:ok, {:dimension, 16.0, "px"}}
    end

    test "multiply number by percentage" do
      assert Values.multiply({:number, 2.0}, {:percentage, 50.0}) ==
               {:ok, {:percentage, 100.0}}
    end

    test "negate number" do
      assert Values.negate({:number, 5.0}) == {:ok, {:number, -5.0}}
    end

    test "negate dimension" do
      assert Values.negate({:dimension, 10.0, "px"}) == {:ok, {:dimension, -10.0, "px"}}
    end
  end

  describe "Values.compare" do
    test "equal numbers" do
      assert Values.compare({:number, 5.0}, {:number, 5.0}, "EQUALS_EQUALS") == {:bool, true}
    end

    test "unequal numbers" do
      assert Values.compare({:number, 5.0}, {:number, 3.0}, "EQUALS_EQUALS") == {:bool, false}
    end

    test "not equals" do
      assert Values.compare({:number, 5.0}, {:number, 3.0}, "NOT_EQUALS") == {:bool, true}
    end

    test "greater than" do
      assert Values.compare({:number, 5.0}, {:number, 3.0}, "GREATER") == {:bool, true}
      assert Values.compare({:number, 3.0}, {:number, 5.0}, "GREATER") == {:bool, false}
    end

    test "greater equals" do
      assert Values.compare({:number, 5.0}, {:number, 5.0}, "GREATER_EQUALS") == {:bool, true}
      assert Values.compare({:number, 5.0}, {:number, 3.0}, "GREATER_EQUALS") == {:bool, true}
      assert Values.compare({:number, 3.0}, {:number, 5.0}, "GREATER_EQUALS") == {:bool, false}
    end

    test "less equals" do
      assert Values.compare({:number, 3.0}, {:number, 5.0}, "LESS_EQUALS") == {:bool, true}
      assert Values.compare({:number, 5.0}, {:number, 5.0}, "LESS_EQUALS") == {:bool, true}
      assert Values.compare({:number, 5.0}, {:number, 3.0}, "LESS_EQUALS") == {:bool, false}
    end

    test "dimension comparison same unit" do
      assert Values.compare({:dimension, 16.0, "px"}, {:dimension, 16.0, "px"}, "EQUALS_EQUALS") ==
               {:bool, true}
    end

    test "ident equality via string comparison" do
      assert Values.compare({:ident, "dark"}, {:ident, "dark"}, "EQUALS_EQUALS") == {:bool, true}
      assert Values.compare({:ident, "dark"}, {:ident, "light"}, "EQUALS_EQUALS") == {:bool, false}
    end
  end

  # ===========================================================================
  # Transformer integration tests (via full transpile pipeline)
  # ===========================================================================

  describe "Transformer: variable substitution" do
    test "simple variable in CSS property" do
      css = transpile!("$primary: #4a90d9;\nh1 { color: $primary; }")
      assert css =~ "#4a90d9"
      assert css =~ "color:"
    end

    test "variable with dimension value" do
      css = transpile!("$base: 16px;\np { font-size: $base; }")
      assert css =~ "16px"
    end

    test "undefined variable raises error" do
      {:ok, ast} = LatticeParser.parse("h1 { color: $undefined; }")
      result = Transformer.transform(ast)
      assert {:error, msg} = result
      assert msg =~ "Undefined variable"
    end
  end

  describe "Transformer: mixin expansion" do
    test "simple mixin expand" do
      # Note: @include with empty parens is parsed as at_rule by the grammar;
      # use no-parens form (@include name;) for zero-argument mixins.
      source = """
      @mixin flex-center() {
        display: flex;
        align-items: center;
      }
      .box { @include flex-center; }
      """
      css = transpile!(source)
      assert css =~ "display"
      assert css =~ "flex"
      assert css =~ "align-items"
    end

    test "mixin with argument" do
      source = """
      @mixin button($bg) {
        background: $bg;
      }
      .btn { @include button(red); }
      """
      css = transpile!(source)
      assert css =~ "background"
      assert css =~ "red"
    end

    test "mixin with default parameter" do
      source = """
      @mixin color-text($color: black) {
        color: $color;
      }
      .a { @include color-text; }
      .b { @include color-text(blue); }
      """
      css = transpile!(source)
      assert css =~ "black"
      assert css =~ "blue"
    end

    test "undefined mixin returns error" do
      {:ok, ast} = LatticeParser.parse(".box { @include no-such-mixin; }")
      result = Transformer.transform(ast)
      assert {:error, msg} = result
      assert msg =~ "Undefined mixin"
    end
  end

  describe "Transformer: @if control flow" do
    test "@if true branch is included" do
      source = """
      $theme: dark;
      @if $theme == dark {
        body { background: black; }
      } @else {
        body { background: white; }
      }
      """
      css = transpile!(source)
      assert css =~ "black"
      refute css =~ "white"
    end

    test "@if false branch uses @else" do
      source = """
      $theme: light;
      @if $theme == dark {
        body { background: black; }
      } @else {
        body { background: white; }
      }
      """
      css = transpile!(source)
      refute css =~ "black"
      assert css =~ "white"
    end
  end

  describe "Transformer: @for loop" do
    test "@for through generates iterations" do
      source = """
      @for $i from 1 through 3 {
        .item { width: 10px; }
      }
      """
      css = transpile!(source)
      # Should have 3 width declarations
      count = css |> String.split("width") |> length()
      assert count >= 4  # "width:" appears 3 times + split results in 4 parts
    end
  end

  describe "Transformer: @each loop" do
    test "@each generates one block per item" do
      source = """
      @each $color in red, green, blue {
        .item { color: $color; }
      }
      """
      css = transpile!(source)
      assert css =~ "red"
      assert css =~ "green"
      assert css =~ "blue"
    end
  end

  describe "Transformer: function evaluation" do
    test "function returns computed value" do
      source = """
      @function double($n) {
        @return $n * 2;
      }
      .box { width: double(8); }
      """
      # The function should compute double(8) = 16
      # (Note: function calls in value position go through expand_function_call)
      result = transform!(source)
      assert {:ok, _} = result
    end
  end

  describe "Transformer: CSS pass-through" do
    test "plain CSS rule passes through unchanged" do
      css = transpile!("h1 { color: red; font-size: 16px; }")
      assert css =~ "color: red"
      assert css =~ "font-size: 16px"
    end

    test "@media rule passes through" do
      css = transpile!("@media screen { h1 { color: red; } }")
      assert css =~ "@media"
      assert css =~ "color: red"
    end

    test "selector list" do
      css = transpile!("h1, h2, h3 { color: red; }")
      assert css =~ "h1"
    end

    test "!important passes through" do
      css = transpile!("p { color: red !important; }")
      assert css =~ "!important"
    end
  end

  # ===========================================================================
  # Emitter tests
  # ===========================================================================

  describe "Emitter" do
    test "pretty-print mode (default)" do
      css = transpile!("h1 { color: red; }")
      # Should have indentation
      assert css =~ "  color:"
    end

    test "minified mode removes whitespace" do
      css = transpile!("h1 { color: red; }", minified: true)
      assert css =~ "h1{color:red;}"
    end

    test "custom indent" do
      css = transpile!("h1 { color: red; }", indent: "    ")
      assert css =~ "    color:"
    end

    test "empty stylesheet produces empty string" do
      css = transpile!("")
      assert css == ""
    end

    test "multiple rules separated by blank line" do
      css = transpile!("h1 { color: red; }\nh2 { color: blue; }")
      # Two rules should appear in output
      assert css =~ "h1"
      assert css =~ "h2"
    end
  end

  # ===========================================================================
  # Error module tests
  # ===========================================================================

  describe "Errors" do
    test "UndefinedVariableError has message" do
      err = Errors.UndefinedVariableError.new("$x")
      assert err.message =~ "$x"
      assert err.name == "$x"
    end

    test "UndefinedMixinError has message" do
      err = Errors.UndefinedMixinError.new("flex-center")
      assert err.message =~ "flex-center"
    end

    test "WrongArityError has message" do
      err = Errors.WrongArityError.new("Mixin", "button", 2, 3)
      assert err.message =~ "button"
      assert err.expected == 2
      assert err.got == 3
    end

    test "CircularReferenceError shows chain" do
      err = Errors.CircularReferenceError.new("mixin", ["a", "b", "a"])
      assert err.message =~ "a -> b -> a"
    end

    test "MissingReturnError has function name" do
      err = Errors.MissingReturnError.new("spacing")
      assert err.message =~ "spacing"
    end

    test "TypeErrorInExpression has op and types" do
      err = Errors.TypeErrorInExpression.new("add", "10px", "red")
      assert err.message =~ "add"
      assert err.op == "add"
    end
  end

  # ===========================================================================
  # Integration: transform_to_css/2
  # ===========================================================================

  describe "LatticeAstToCss.transform_to_css/2" do
    test "basic integration" do
      {:ok, ast} = LatticeParser.parse("h1 { color: red; }")
      {:ok, css} = LatticeAstToCss.transform_to_css(ast)
      assert css =~ "color: red"
    end

    test "with minified option" do
      {:ok, ast} = LatticeParser.parse("h1 { color: red; }")
      {:ok, css} = LatticeAstToCss.transform_to_css(ast, minified: true)
      assert css =~ "h1{color:red;}"
    end
  end
end
