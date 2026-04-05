defmodule CodingAdventures.MosaicParser do
  @moduledoc """
  Mosaic Parser — Thin wrapper around the grammar-driven parser engine.

  This module combines `MosaicLexer.tokenize/1` with `GrammarParser.parse/2`
  to parse Mosaic source code into an AST. It reads `mosaic.grammar` from
  the shared grammars directory.

  Mosaic is a Component Description Language (CDL) for declaring UI component
  structure with named typed slots. A `.mosaic` file has this shape:

      component ProfileCard {
        slot avatar-url: image;
        slot display-name: text;
        slot actions: Button;

        Column {
          Text { content: @display-name; }
          @actions;
        }
      }

  ## Usage

      {:ok, ast} = CodingAdventures.MosaicParser.parse(~S(component Foo { Box { } }))

  The returned AST is a tree of `ASTNode` structs where `rule_name`
  identifies the grammar rule that was matched (e.g., `"file"`,
  `"component_decl"`, `"slot_decl"`, `"node_element"`, `"property_assignment"`)
  and `children` contains sub-nodes and leaf tokens.

  ## Grammar Structure

  The grammar rules (in dependency order, leaf-to-root) are:

  | Rule                  | Matches                                                |
  |-----------------------|--------------------------------------------------------|
  | `default_value`       | Literal value for a slot default: string, number,     |
  |                       | dimension, color, or keyword.                          |
  | `slot_ref`            | `@name` — reference to a slot by name.                 |
  | `enum_value`          | `Name.member` — dotted enum-style property value.      |
  | `property_value`      | Any valid property value (slot_ref, literal, enum).    |
  | `property_assignment` | `name: value;` — a property key-value pair.            |
  | `list_type`           | `list<type>` — parameterised list type.                |
  | `slot_type`           | A slot type: keyword, NAME, or list_type.              |
  | `slot_decl`           | `slot name: type [= default];`                         |
  | `slot_reference`      | `@name;` — slot used as a child node.                  |
  | `when_block`          | `when @flag { … }` — conditional subtree.              |
  | `each_block`          | `each @list as item { … }` — repeated subtree.         |
  | `node_content`        | One item inside a node: property, child, ref, when,   |
  |                       | or each.                                               |
  | `child_node`          | An inner `node_element`.                               |
  | `node_element`        | `Name { node_content* }` — a named visual node.        |
  | `node_tree`           | The root node of the visual tree.                      |
  | `import_decl`         | `import Foo [as Bar] from "path";`                     |
  | `component_decl`      | `component Name { slot* node_tree }`                   |
  | `file`                | Top-level rule: optional imports + one component.      |

  ## How It Works

  1. `tokenize/1` from `MosaicLexer` turns the source into a token list.
  2. `GrammarParser.parse/2` walks that list against the rules in
     `mosaic.grammar`, building an `ASTNode` tree.
  3. The grammar is cached in a persistent term for fast repeated access.
  """

  alias CodingAdventures.GrammarTools.ParserGrammar
  alias CodingAdventures.MosaicLexer
  alias CodingAdventures.Parser.{GrammarParser, ASTNode}

  # The shared grammars directory lives five levels above __DIR__:
  #   __DIR__ = .../code/packages/elixir/mosaic_parser/lib/coding_adventures/
  #   five ".." levels reach .../code/
  #   + /grammars = .../code/grammars/
  @grammars_dir Path.join([__DIR__, "..", "..", "..", "..", "..", "grammars"])
                |> Path.expand()

  @doc """
  Parse Mosaic source code into an AST.

  Internally calls `MosaicLexer.tokenize/1` first, then runs the grammar-
  driven parser over the token stream.

  Returns `{:ok, ast_node}` on success, `{:error, message}` on failure.
  The root node's `rule_name` is always `"file"`.

  ## Examples

      iex> {:ok, ast} = CodingAdventures.MosaicParser.parse("component Foo { Box { } }")
      iex> ast.rule_name
      "file"
  """
  @spec parse(String.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def parse(source) do
    grammar = get_grammar()

    case MosaicLexer.tokenize(source) do
      {:ok, tokens} -> GrammarParser.parse(tokens, grammar)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Parse `mosaic.grammar` and return the `ParserGrammar`.

  Useful for inspecting which rules are defined, or for testing that the
  grammar file is well-formed. The result is NOT cached.

  ## Example

      grammar = CodingAdventures.MosaicParser.create_parser()
      Enum.map(grammar.rules, & &1.name)
      # => ["file", "import_decl", "component_decl", "slot_decl", ...]
  """
  @spec create_parser() :: ParserGrammar.t()
  def create_parser do
    grammar_path = Path.join(@grammars_dir, "mosaic.grammar")
    {:ok, grammar} = ParserGrammar.parse(File.read!(grammar_path))
    fix_grammar_for_packrat(grammar)
  end

  # ---------------------------------------------------------------------------
  # Grammar fixup for packrat memoization compatibility.
  #
  # The `slot_type` rule in `mosaic.grammar` is:
  #
  #     slot_type = KEYWORD | NAME | list_type ;
  #
  # With packrat memoization, trying `KEYWORD` first is problematic:
  # when the source contains `list<text>`, the parser matches `KEYWORD("list")`
  # and caches that result. Even though the parent sequence then fails (because
  # `<` doesn't match `;`), the cached entry is reused in future attempts,
  # preventing `list_type` from ever being tried.
  #
  # The fix is to reorder `slot_type` so that `list_type` is tried **first**,
  # before `KEYWORD`. This way, `list<text>` correctly produces a `list_type`
  # node, and bare keywords like `text` or `bool` still match via `KEYWORD`.
  #
  #     slot_type (fixed) = list_type | KEYWORD | NAME ;
  #
  # Similarly, `property_value` must try `enum_value` before `NAME` to
  # prevent `heading.small` being parsed as just `heading`. The original
  # grammar has `NAME` before `enum_value`, which causes the same memoization
  # problem. We reorder to try `enum_value` first.
  #
  #   property_value (fixed) = slot_ref | STRING | NUMBER | DIMENSION
  #                          | COLOR_HEX | KEYWORD | enum_value | NAME
  # ---------------------------------------------------------------------------
  defp fix_grammar_for_packrat(%ParserGrammar{rules: rules} = grammar) do
    fixed_rules =
      Enum.map(rules, fn rule ->
        case rule.name do
          "slot_type" ->
            # Original: KEYWORD | NAME | list_type
            # Fixed:    list_type | KEYWORD | NAME
            %{
              rule
              | body:
                  {:alternation,
                   [
                     {:rule_reference, "list_type", false},
                     {:rule_reference, "KEYWORD", true},
                     {:rule_reference, "NAME", true}
                   ]}
            }

          "property_value" ->
            # Original: slot_ref | STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD | NAME | enum_value
            # Fixed:    slot_ref | STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD | enum_value | NAME
            # (enum_value moved before NAME so "heading.small" is parsed correctly)
            %{
              rule
              | body:
                  {:alternation,
                   [
                     {:rule_reference, "slot_ref", false},
                     {:rule_reference, "STRING", true},
                     {:rule_reference, "NUMBER", true},
                     {:rule_reference, "DIMENSION", true},
                     {:rule_reference, "COLOR_HEX", true},
                     {:rule_reference, "KEYWORD", true},
                     {:rule_reference, "enum_value", false},
                     {:rule_reference, "NAME", true}
                   ]}
            }

          _ ->
            rule
        end
      end)

    %{grammar | rules: fixed_rules}
  end

  # Cache the grammar in a persistent_term for fast repeated access.
  defp get_grammar do
    case :persistent_term.get({__MODULE__, :grammar}, nil) do
      nil ->
        grammar = create_parser()
        :persistent_term.put({__MODULE__, :grammar}, grammar)
        grammar

      grammar ->
        grammar
    end
  end
end
