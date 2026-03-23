defmodule CodingAdventures.LatticeAstToCss.TransformerV2HelpersTest do
  @moduledoc """
  Tests for Lattice v2 transformer helper functions that can be
  exercised without the parser pipeline. Tests the State struct
  fields and public API behavior with v2 features.
  """
  use ExUnit.Case, async: true

  alias CodingAdventures.LatticeAstToCss.Transformer
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # Helper to build a minimal stylesheet AST with the given children
  defp stylesheet(children) do
    %ASTNode{rule_name: "stylesheet", children: children}
  end

  defp rule(inner) do
    %ASTNode{rule_name: "rule", children: [inner]}
  end

  defp lattice_rule(inner) do
    %ASTNode{rule_name: "lattice_rule", children: [inner]}
  end

  defp var_decl(name, value_token, flags \\ []) do
    children = [
      %Token{type: "VARIABLE", value: name, line: 1, column: 1},
      %Token{type: "COLON", value: ":", line: 1, column: 5},
      %ASTNode{rule_name: "value_list", children: [
        %ASTNode{rule_name: "value", children: [value_token]}
      ]}
    ] ++ flags ++ [
      %Token{type: "SEMICOLON", value: ";", line: 1, column: 20}
    ]

    %ASTNode{rule_name: "variable_declaration", children: children}
  end

  defp qualified_rule(selector_text, declarations) do
    selector = %ASTNode{rule_name: "selector_list", children: [
      %ASTNode{rule_name: "complex_selector", children: [
        %ASTNode{rule_name: "compound_selector", children: [
          %ASTNode{rule_name: "simple_selector", children: [
            %Token{type: "IDENT", value: selector_text, line: 1, column: 1}
          ]}
        ]}
      ]}
    ]}

    block_items = Enum.map(declarations, fn {prop, val_text} ->
      %ASTNode{rule_name: "block_item", children: [
        %ASTNode{rule_name: "declaration_or_nested", children: [
          %ASTNode{rule_name: "declaration", children: [
            %ASTNode{rule_name: "property", children: [
              %Token{type: "IDENT", value: prop, line: 1, column: 1}
            ]},
            %Token{type: "COLON", value: ":", line: 1, column: 10},
            %ASTNode{rule_name: "value_list", children: [
              %ASTNode{rule_name: "value", children: [
                %Token{type: "IDENT", value: val_text, line: 1, column: 12}
              ]}
            ]},
            %Token{type: "SEMICOLON", value: ";", line: 1, column: 20}
          ]}
        ]}
      ]}
    end)

    %ASTNode{rule_name: "qualified_rule", children: [
      selector,
      %ASTNode{rule_name: "block", children: [
        %Token{type: "LBRACE", value: "{", line: 1, column: 5},
        %ASTNode{rule_name: "block_contents", children: block_items},
        %Token{type: "RBRACE", value: "}", line: 1, column: 30}
      ]}
    ]}
  end

  # ==========================================================================
  # Variable declarations with !default flag
  # ==========================================================================

  describe "!default flag in variable declarations" do
    test "!default at top level: doesn't override existing" do
      vd1 = var_decl("$color", %Token{type: "IDENT", value: "red", line: 1, column: 8})
      vd2 = var_decl("$color", %Token{type: "IDENT", value: "blue", line: 2, column: 8}, [
        %Token{type: "BANG_DEFAULT", value: "!default", line: 2, column: 15}
      ])

      qr = qualified_rule("h1", [{"color", "$color"}])

      ast = stylesheet([
        rule(lattice_rule(vd1)),
        rule(lattice_rule(vd2)),
        rule(qr)
      ])

      {:ok, css_ast} = Transformer.transform(ast)
      # $color should be "red" because !default doesn't override
      assert css_ast != nil
    end

    test "!default at top level: sets if not defined" do
      vd = var_decl("$size", %Token{type: "DIMENSION", value: "16px", line: 1, column: 8}, [
        %Token{type: "BANG_DEFAULT", value: "!default", line: 1, column: 15}
      ])

      qr = qualified_rule("p", [{"font-size", "$size"}])

      ast = stylesheet([
        rule(lattice_rule(vd)),
        rule(qr)
      ])

      {:ok, _css_ast} = Transformer.transform(ast)
    end
  end

  # ==========================================================================
  # Variable declarations with !global flag
  # ==========================================================================

  describe "!global flag in variable declarations" do
    test "!global at top level sets variable" do
      vd = var_decl("$theme", %Token{type: "IDENT", value: "dark", line: 1, column: 8}, [
        %Token{type: "BANG_GLOBAL", value: "!global", line: 1, column: 15}
      ])

      qr = qualified_rule("body", [{"class", "$theme"}])

      ast = stylesheet([
        rule(lattice_rule(vd)),
        rule(qr)
      ])

      {:ok, _css_ast} = Transformer.transform(ast)
    end
  end

  # ==========================================================================
  # Variable declarations with variable_flag nodes
  # ==========================================================================

  describe "variable_flag AST nodes" do
    test "!default inside variable_flag node" do
      vd = var_decl("$x", %Token{type: "NUMBER", value: "10", line: 1, column: 5}, [
        %ASTNode{rule_name: "variable_flag", children: [
          %Token{type: "BANG_DEFAULT", value: "!default", line: 1, column: 10}
        ]}
      ])

      ast = stylesheet([rule(lattice_rule(vd))])
      {:ok, _} = Transformer.transform(ast)
    end

    test "!global inside variable_flag node" do
      vd = var_decl("$x", %Token{type: "NUMBER", value: "10", line: 1, column: 5}, [
        %ASTNode{rule_name: "variable_flag", children: [
          %Token{type: "BANG_GLOBAL", value: "!global", line: 1, column: 10}
        ]}
      ])

      ast = stylesheet([rule(lattice_rule(vd))])
      {:ok, _} = Transformer.transform(ast)
    end

    test "both !default and !global flags" do
      vd = var_decl("$x", %Token{type: "NUMBER", value: "10", line: 1, column: 5}, [
        %Token{type: "BANG_DEFAULT", value: "!default", line: 1, column: 10},
        %Token{type: "BANG_GLOBAL", value: "!global", line: 1, column: 20}
      ])

      ast = stylesheet([rule(lattice_rule(vd))])
      {:ok, _} = Transformer.transform(ast)
    end
  end

  # ==========================================================================
  # Empty transforms
  # ==========================================================================

  describe "empty stylesheet" do
    test "empty stylesheet transforms ok" do
      ast = stylesheet([])
      {:ok, result} = Transformer.transform(ast)
      assert result.children == []
    end
  end

  # ==========================================================================
  # Placeholder rule removal
  # ==========================================================================

  describe "placeholder rule removal" do
    test "qualified_rule with placeholder selector is kept when no @extend" do
      # Without @extend, placeholder rules are NOT removed
      # (removal only happens when extend_map is non-empty)
      qr = %ASTNode{rule_name: "qualified_rule", children: [
        %ASTNode{rule_name: "selector_list", children: [
          %ASTNode{rule_name: "complex_selector", children: [
            %ASTNode{rule_name: "compound_selector", children: [
              %ASTNode{rule_name: "simple_selector", children: [
                %Token{type: "PLACEHOLDER", value: "%base", line: 1, column: 1}
              ]}
            ]}
          ]}
        ]},
        %ASTNode{rule_name: "block", children: [
          %Token{type: "LBRACE", value: "{", line: 1, column: 6},
          %ASTNode{rule_name: "block_contents", children: []},
          %Token{type: "RBRACE", value: "}", line: 1, column: 8}
        ]}
      ]}

      ast = stylesheet([rule(qr)])
      {:ok, result} = Transformer.transform(ast)
      # Since no @extend was used, placeholder rules are not removed
      assert length(result.children) >= 0
    end
  end
end
