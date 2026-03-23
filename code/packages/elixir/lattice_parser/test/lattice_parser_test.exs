defmodule CodingAdventures.LatticeParserTest do
  use ExUnit.Case

  alias CodingAdventures.LatticeParser
  alias CodingAdventures.Parser.ASTNode

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  defp parse!(source) do
    {:ok, ast} = LatticeParser.parse(source)
    ast
  end

  # Recursively find all nodes with the given rule_name in a tree
  defp find_all(%ASTNode{rule_name: rule, children: children}, target_rule) do
    matches = if rule == target_rule, do: [%ASTNode{rule_name: rule, children: children}], else: []
    child_matches = Enum.flat_map(children, fn
      %ASTNode{} = child -> find_all(child, target_rule)
      _ -> []
    end)
    matches ++ child_matches
  end

  # Collect all token values of a given type from a node tree
  defp collect_token_values(%ASTNode{children: children}, token_type) do
    Enum.flat_map(children, fn
      %ASTNode{} = child -> collect_token_values(child, token_type)
      %{type: ^token_type, value: value} -> [value]
      _ -> []
    end)
  end

  # ---------------------------------------------------------------------------
  # Module loading
  # ---------------------------------------------------------------------------

  describe "module loading" do
    test "module loads" do
      assert Code.ensure_loaded?(CodingAdventures.LatticeParser)
    end

    test "create_parser/0 returns a ParserGrammar" do
      grammar = LatticeParser.create_parser()
      # ParserGrammar.rules is a list of rule maps (not a map keyed by name)
      assert is_list(grammar.rules)
      assert length(grammar.rules) > 0
      # Each entry has a :name and :body key
      first = hd(grammar.rules)
      assert is_binary(first.name)
    end
  end

  # ---------------------------------------------------------------------------
  # Root node
  # ---------------------------------------------------------------------------

  describe "root node" do
    test "empty source produces stylesheet node" do
      ast = parse!("")
      assert %ASTNode{rule_name: "stylesheet"} = ast
    end

    test "stylesheet children are rule nodes" do
      ast = parse!("h1 { color: red; }")
      rules = find_all(ast, "rule")
      assert length(rules) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice: Variable Declarations
  # ---------------------------------------------------------------------------

  describe "variable_declaration" do
    test "simple variable declaration" do
      ast = parse!("$color: red;")
      var_decls = find_all(ast, "variable_declaration")
      assert length(var_decls) > 0
    end

    test "variable declaration is inside a lattice_rule" do
      ast = parse!("$color: red;")
      lattice_rules = find_all(ast, "lattice_rule")
      assert length(lattice_rules) > 0
    end

    test "variable name is preserved as VARIABLE token" do
      ast = parse!("$primary: #4a90d9;")
      var_tokens = collect_token_values(ast, "VARIABLE")
      assert "$primary" in var_tokens
    end

    test "multiple variable declarations" do
      ast = parse!("""
        $a: 1px;
        $b: 2em;
        $c: red;
      """)
      var_decls = find_all(ast, "variable_declaration")
      assert length(var_decls) == 3
    end

    test "variable with dimension value" do
      ast = parse!("$base: 16px;")
      var_decls = find_all(ast, "variable_declaration")
      assert length(var_decls) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice: Mixin Definitions
  # ---------------------------------------------------------------------------

  describe "mixin_definition" do
    test "simple mixin with no params" do
      ast = parse!("@mixin clearfix() { content: ''; }")
      mixin_defs = find_all(ast, "mixin_definition")
      assert length(mixin_defs) > 0
    end

    test "mixin with one parameter" do
      ast = parse!("@mixin button($bg) { background: $bg; }")
      mixin_defs = find_all(ast, "mixin_definition")
      assert length(mixin_defs) > 0
    end

    test "mixin with default parameter" do
      ast = parse!("@mixin button($bg, $fg: white) { background: $bg; color: $fg; }")
      mixin_params = find_all(ast, "mixin_params")
      assert length(mixin_params) > 0
    end

    test "mixin params are captured" do
      ast = parse!("@mixin flex($dir: row) { flex-direction: $dir; }")
      mixin_param = find_all(ast, "mixin_param")
      assert length(mixin_param) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice: Include Directive
  # ---------------------------------------------------------------------------

  describe "include_directive" do
    test "@include with function call form" do
      ast = parse!(".btn { @include button(red); }")
      includes = find_all(ast, "include_directive")
      assert length(includes) > 0
    end

    test "@include with bare ident form" do
      ast = parse!(".btn { @include clearfix; }")
      includes = find_all(ast, "include_directive")
      assert length(includes) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice: Function Definitions
  # ---------------------------------------------------------------------------

  describe "function_definition" do
    test "simple function definition" do
      ast = parse!("@function double($n) { @return $n * 2; }")
      func_defs = find_all(ast, "function_definition")
      assert length(func_defs) > 0
    end

    test "function body contains return_directive" do
      ast = parse!("@function double($n) { @return $n * 2; }")
      return_dirs = find_all(ast, "return_directive")
      assert length(return_dirs) > 0
    end

    test "function body item is captured" do
      ast = parse!("@function noop($x) { $y: $x; @return $y; }")
      body_items = find_all(ast, "function_body_item")
      assert length(body_items) >= 2
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice: @use Directive
  # ---------------------------------------------------------------------------

  describe "use_directive" do
    test "simple @use" do
      ast = parse!(~s(@use "colors";))
      use_dirs = find_all(ast, "use_directive")
      assert length(use_dirs) > 0
    end

    test "@use with alias" do
      ast = parse!(~s(@use "utils/mixins" as m;))
      use_dirs = find_all(ast, "use_directive")
      assert length(use_dirs) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Lattice: Control Flow (@if / @for / @each)
  # ---------------------------------------------------------------------------

  describe "if_directive" do
    test "@if with equality condition" do
      ast = parse!("""
        @if $theme == dark {
          body { background: black; }
        }
      """)
      if_dirs = find_all(ast, "if_directive")
      assert length(if_dirs) > 0
    end

    test "@if with @else" do
      ast = parse!("""
        @if $x == 1 {
          .a { color: red; }
        } @else {
          .a { color: blue; }
        }
      """)
      if_dirs = find_all(ast, "if_directive")
      assert length(if_dirs) > 0
    end
  end

  describe "for_directive" do
    test "@for loop with through" do
      ast = parse!("""
        @for $i from 1 through 3 {
          .item { width: 10px; }
        }
      """)
      for_dirs = find_all(ast, "for_directive")
      assert length(for_dirs) > 0
    end
  end

  describe "each_directive" do
    test "@each loop" do
      ast = parse!("""
        @each $color in red, green, blue {
          .text { color: $color; }
        }
      """)
      each_dirs = find_all(ast, "each_directive")
      assert length(each_dirs) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # CSS: Qualified Rules (selectors + blocks)
  # ---------------------------------------------------------------------------

  describe "qualified_rule" do
    test "simple type selector" do
      ast = parse!("h1 { color: red; }")
      qr = find_all(ast, "qualified_rule")
      assert length(qr) > 0
    end

    test "class selector" do
      ast = parse!(".button { background: blue; }")
      qr = find_all(ast, "qualified_rule")
      assert length(qr) > 0
    end

    test "selector with declaration" do
      ast = parse!("p { font-size: 16px; }")
      decls = find_all(ast, "declaration")
      assert length(decls) > 0
    end

    test "multiple declarations in one rule" do
      ast = parse!("div { color: red; background: blue; margin: 0; }")
      decls = find_all(ast, "declaration")
      assert length(decls) == 3
    end

    test "selector list (comma-separated)" do
      ast = parse!("h1, h2, h3 { color: red; }")
      selector_lists = find_all(ast, "selector_list")
      assert length(selector_lists) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # CSS: Declarations
  # ---------------------------------------------------------------------------

  describe "declaration" do
    test "ident property" do
      ast = parse!("p { color: red; }")
      decls = find_all(ast, "declaration")
      assert length(decls) == 1
    end

    test "custom property" do
      ast = parse!("p { --my-color: blue; }")
      decls = find_all(ast, "declaration")
      assert length(decls) == 1
    end

    test "!important priority" do
      ast = parse!("p { color: red !important; }")
      priorities = find_all(ast, "priority")
      assert length(priorities) > 0
    end

    test "variable in value" do
      ast = parse!("p { color: $brand; }")
      decls = find_all(ast, "declaration")
      assert length(decls) == 1
    end
  end

  # ---------------------------------------------------------------------------
  # CSS: At-Rules
  # ---------------------------------------------------------------------------

  describe "at_rule" do
    test "@media rule" do
      ast = parse!("@media screen { h1 { color: red; } }")
      at_rules = find_all(ast, "at_rule")
      assert length(at_rules) > 0
    end

    test "@import rule" do
      ast = parse!(~s(@import "reset.css";))
      at_rules = find_all(ast, "at_rule")
      assert length(at_rules) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Expressions
  # ---------------------------------------------------------------------------

  describe "lattice_expression" do
    test "expression in @if" do
      ast = parse!("@if $x > 5 { .big { font-size: 24px; } }")
      exprs = find_all(ast, "lattice_expression")
      assert length(exprs) > 0
    end

    test "arithmetic in @return" do
      ast = parse!("@function double($n) { @return $n * 2; }")
      exprs = find_all(ast, "lattice_expression")
      assert length(exprs) > 0
    end
  end

  # ---------------------------------------------------------------------------
  # Realistic Lattice programs
  # ---------------------------------------------------------------------------

  describe "complete Lattice programs" do
    test "variable + qualified rule" do
      ast = parse!("""
        $primary: #4a90d9;
        h1 { color: $primary; }
      """)
      assert %ASTNode{rule_name: "stylesheet"} = ast
      var_decls = find_all(ast, "variable_declaration")
      qr = find_all(ast, "qualified_rule")
      assert length(var_decls) == 1
      assert length(qr) == 1
    end

    test "mixin definition and include" do
      # Note: @include with empty parens falls through to at_rule in the grammar
      # (include_directive requires at least one arg for the FUNCTION form).
      # Use the IDENT form (no parens) which is fully supported.
      ast = parse!("""
        @mixin flex-center() {
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .card {
          @include flex-center;
          padding: 20px;
        }
      """)
      mixin_defs = find_all(ast, "mixin_definition")
      includes = find_all(ast, "include_directive")
      assert length(mixin_defs) == 1
      assert length(includes) == 1
    end

    test "function definition and call in value" do
      ast = parse!("""
        @function spacing($n) {
          @return $n * 8px;
        }
        .card { padding: spacing(2); }
      """)
      func_defs = find_all(ast, "function_definition")
      assert length(func_defs) == 1
    end

    test "complex stylesheet with mixins, variables, and rules" do
      ast = parse!("""
        $base-size: 16px;
        $primary: #4a90d9;

        @mixin button($bg, $fg: white) {
          background: $bg;
          color: $fg;
          padding: 8px 16px;
          border-radius: 4px;
        }

        .btn-primary {
          @include button($primary);
        }

        .btn-danger {
          @include button(red);
        }
      """)

      assert %ASTNode{rule_name: "stylesheet"} = ast
      var_decls = find_all(ast, "variable_declaration")
      mixin_defs = find_all(ast, "mixin_definition")
      includes = find_all(ast, "include_directive")
      assert length(var_decls) == 2
      assert length(mixin_defs) == 1
      assert length(includes) == 2
    end
  end

  # ---------------------------------------------------------------------------
  # Error handling
  # ---------------------------------------------------------------------------

  describe "error handling" do
    test "returns tuple (not crash) on any input" do
      result = LatticeParser.parse("{ }")
      assert match?({:ok, _}, result) or match?({:error, _}, result)
    end
  end
end
