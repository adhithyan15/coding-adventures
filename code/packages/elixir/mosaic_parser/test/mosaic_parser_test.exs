defmodule CodingAdventures.MosaicParserTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.MosaicParser
  alias CodingAdventures.Parser.ASTNode

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Recursively collect every `rule_name` from the AST tree so we can assert
  # that certain grammar rules were matched without hard-coding deep paths.
  defp collect_rule_names(%ASTNode{rule_name: name, children: children}) do
    child_names =
      children
      |> Enum.filter(&ASTNode.ast_node?/1)
      |> Enum.flat_map(&collect_rule_names/1)

    [name | child_names]
  end

  # ---------------------------------------------------------------------------
  # create_parser/0 — grammar introspection
  # ---------------------------------------------------------------------------

  describe "create_parser/0" do
    test "returns a ParserGrammar struct" do
      grammar = MosaicParser.create_parser()
      assert is_list(grammar.rules)
    end

    test "grammar contains the top-level 'file' rule" do
      grammar = MosaicParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)
      assert "file" in rule_names
    end

    test "grammar contains structural rules" do
      grammar = MosaicParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)

      assert "component_decl" in rule_names
      assert "slot_decl" in rule_names
      assert "node_element" in rule_names
      assert "node_tree" in rule_names
      assert "node_content" in rule_names
      assert "property_assignment" in rule_names
    end

    test "grammar contains auxiliary rules" do
      grammar = MosaicParser.create_parser()
      rule_names = Enum.map(grammar.rules, & &1.name)

      assert "slot_ref" in rule_names
      assert "slot_type" in rule_names
      assert "property_value" in rule_names
      assert "when_block" in rule_names
      assert "each_block" in rule_names
      assert "import_decl" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # Minimal component — the simplest valid Mosaic file
  # ---------------------------------------------------------------------------
  # A valid Mosaic file must have exactly one component with a root node.
  # "component Foo { Box { } }" is the smallest valid input.

  describe "parse/1 — minimal component" do
    test "parses the smallest valid component" do
      {:ok, ast} = MosaicParser.parse("component Foo { Box { } }")
      assert ast.rule_name == "file"
    end

    test "root node contains a component_decl" do
      {:ok, ast} = MosaicParser.parse("component Foo { Box { } }")
      rule_names = collect_rule_names(ast)
      assert "component_decl" in rule_names
    end

    test "root node contains a node_element" do
      {:ok, ast} = MosaicParser.parse("component Foo { Box { } }")
      rule_names = collect_rule_names(ast)
      assert "node_element" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # Slot declarations
  # ---------------------------------------------------------------------------
  # Slots are typed named inputs to the component.
  # `slot title: text;` declares a slot named "title" of primitive type "text".

  describe "parse/1 — slot declarations" do
    test "parses a component with a single text slot" do
      source = """
      component Label {
        slot title: text;
        Text { }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
      rule_names = collect_rule_names(ast)
      assert "slot_decl" in rule_names
    end

    test "parses a component with multiple slots" do
      source = """
      component Card {
        slot header: text;
        slot count: number;
        slot visible: bool;
        Box { }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
      rule_names = collect_rule_names(ast)
      assert "slot_decl" in rule_names
    end

    test "parses a slot with a default number value" do
      source = """
      component Counter {
        slot count: number = 0;
        Text { }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
      rule_names = collect_rule_names(ast)
      assert "default_value" in rule_names
    end

    test "parses a slot with a keyword type (node)" do
      source = """
      component Container {
        slot child: node;
        Box { }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
    end
  end

  # ---------------------------------------------------------------------------
  # Node tree — property assignments
  # ---------------------------------------------------------------------------
  # Properties are `name: value;` pairs inside a node's body.

  describe "parse/1 — property assignments" do
    test "parses a property with a keyword value" do
      # `align: center;` — NAME: NAME; but in the grammar `align` and `center`
      # are NAMEs (not keywords), so this is property_assignment with NAME values.
      source = "component Foo { Box { padding: 0; } }"
      {:ok, ast} = MosaicParser.parse(source)
      rule_names = collect_rule_names(ast)
      assert "property_assignment" in rule_names
    end

    test "parses a property with a color value" do
      source = "component Foo { Box { background: #2563eb; } }"
      {:ok, ast} = MosaicParser.parse(source)
      rule_names = collect_rule_names(ast)
      assert "property_assignment" in rule_names
    end

    test "parses a property with a dimension value" do
      source = "component Foo { Box { padding: 16dp; } }"
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
    end

    test "parses a property with a string value" do
      source = ~S(component Foo { Text { label: "hello"; } })
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
    end

    test "parses a property whose name is a keyword (color)" do
      # The grammar explicitly allows KEYWORD as a property name so that slot
      # type keywords like "color" can also be used as layout property names.
      # E.g., `color: #fff;` — here "color" is a KEYWORD used as a prop name.
      source = "component Foo { Box { color: #fff; } }"
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
    end

    test "parses a slot reference as a property value" do
      # `content: @title;` — the value is a slot_ref (@NAME)
      source = """
      component Label {
        slot title: text;
        Text { content: @title; }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      rule_names = collect_rule_names(ast)
      assert "slot_ref" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # Nested nodes
  # ---------------------------------------------------------------------------

  describe "parse/1 — nested nodes" do
    test "parses a node with a single child node" do
      source = """
      component Profile {
        Column {
          Text { }
        }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
    end

    test "parses deeply nested nodes" do
      source = """
      component Feed {
        Column {
          Row {
            Image { }
            Column {
              Text { }
              Text { }
            }
          }
        }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      assert ast.rule_name == "file"
    end
  end

  # ---------------------------------------------------------------------------
  # Slot references as child nodes
  # ---------------------------------------------------------------------------
  # `@actions;` places a slot of type node/component into the visual tree.

  describe "parse/1 — slot references as children" do
    test "parses a slot reference inside a node body" do
      source = """
      component ProfileCard {
        slot actions: node;
        Column {
          @actions;
        }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      rule_names = collect_rule_names(ast)
      assert "slot_reference" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # when_block — conditional rendering
  # ---------------------------------------------------------------------------
  # `when @show-header { … }` renders a subtree only when the bool slot is true.

  describe "parse/1 — when blocks" do
    test "parses a when block" do
      source = """
      component Feed {
        slot show-header: bool;
        Column {
          when @show-header {
            Text { }
          }
        }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      rule_names = collect_rule_names(ast)
      assert "when_block" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # each_block — iteration
  # ---------------------------------------------------------------------------
  # `each @items as item { … }` repeats a subtree for each element in a list.

  describe "parse/1 — each blocks" do
    test "parses an each block" do
      source = """
      component List {
        slot items: list<text>;
        Column {
          each @items as item {
            Text { }
          }
        }
      }
      """
      {:ok, ast} = MosaicParser.parse(source)
      rule_names = collect_rule_names(ast)
      assert "each_block" in rule_names
    end
  end

  # ---------------------------------------------------------------------------
  # Error cases
  # ---------------------------------------------------------------------------

  describe "parse/1 — error cases" do
    test "errors on empty input" do
      result = MosaicParser.parse("")
      assert match?({:error, _}, result)
    end

    test "errors on unclosed brace" do
      result = MosaicParser.parse("component Foo { Box { }")
      # Either a lex error or a parse error — both are :error tuples
      assert match?({:error, _}, result)
    end

    test "errors on missing component keyword" do
      result = MosaicParser.parse("Foo { Box { } }")
      assert match?({:error, _}, result)
    end

    test "errors on invalid character in source" do
      result = MosaicParser.parse("component Foo { ` }")
      assert match?({:error, _}, result)
    end
  end
end
