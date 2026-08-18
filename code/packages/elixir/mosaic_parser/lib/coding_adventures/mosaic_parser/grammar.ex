defmodule CodingAdventures.MosaicParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: mosaic.grammar
  # Regenerate with: grammar-tools compile-grammar mosaic.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.

  alias CodingAdventures.GrammarTools.ParserGrammar

  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "file",
          body: {:sequence, [
            {:repetition, {:rule_reference, "import_decl", false}},
            {:rule_reference, "component_decl", false},
          ]},
          line_number: 20,
        },
        %{
          name: "import_decl",
          body: {:sequence, [
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "KEYWORD", true},
                {:rule_reference, "NAME", true},
              ]}},
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 30,
        },
        %{
          name: "component_decl",
          body: {:sequence, [
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "slot_decl", false}},
            {:rule_reference, "node_tree", false},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 48,
        },
        %{
          name: "slot_decl",
          body: {:sequence, [
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "slot_type", false},
            {:optional, {:sequence, [
                {:rule_reference, "EQUALS", true},
                {:rule_reference, "default_value", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 67,
        },
        %{
          name: "slot_type",
          body: {:alternation, [
            {:rule_reference, "list_type", false},
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 69,
        },
        %{
          name: "list_type",
          body: {:sequence, [
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "LANGLE", true},
            {:rule_reference, "slot_type", false},
            {:rule_reference, "RANGLE", true},
          ]},
          line_number: 73,
        },
        %{
          name: "default_value",
          body: {:alternation, [
            {:rule_reference, "STRING", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "COLOR_HEX", true},
            {:rule_reference, "KEYWORD", true},
          ]},
          line_number: 75,
        },
        %{
          name: "node_tree",
          body: {:rule_reference, "node_element", false},
          line_number: 86,
        },
        %{
          name: "node_element",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "node_content", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 88,
        },
        %{
          name: "node_content",
          body: {:alternation, [
            {:rule_reference, "property_assignment", false},
            {:rule_reference, "child_node", false},
            {:rule_reference, "slot_reference", false},
            {:rule_reference, "when_block", false},
            {:rule_reference, "each_block", false},
          ]},
          line_number: 90,
        },
        %{
          name: "property_assignment",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "KEYWORD", true},
              ]}},
            {:rule_reference, "COLON", true},
            {:rule_reference, "property_value", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 107,
        },
        %{
          name: "property_value",
          body: {:alternation, [
            {:rule_reference, "slot_ref", false},
            {:rule_reference, "enum_value", false},
            {:rule_reference, "STRING", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "COLOR_HEX", true},
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 111,
        },
        %{
          name: "slot_ref",
          body: {:sequence, [
            {:rule_reference, "AT", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 122,
        },
        %{
          name: "enum_value",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "DOT", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 124,
        },
        %{
          name: "child_node",
          body: {:rule_reference, "node_element", false},
          line_number: 131,
        },
        %{
          name: "slot_reference",
          body: {:sequence, [
            {:rule_reference, "AT", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 144,
        },
        %{
          name: "when_block",
          body: {:sequence, [
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "slot_ref", false},
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "node_content", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 156,
        },
        %{
          name: "each_block",
          body: {:sequence, [
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "slot_ref", false},
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "node_content", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 170,
        },
      ],
      version: 1,
    }
  end

end
