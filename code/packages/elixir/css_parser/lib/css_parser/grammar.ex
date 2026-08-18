defmodule CodingAdventures.CssParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: css.grammar
  # Regenerate with: grammar-tools compile-grammar css.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "stylesheet",
          body: {:repetition, {:rule_reference, "rule", false}},
          line_number: 33,
        },
        %{
          name: "rule",
          body: {:alternation, [
            {:rule_reference, "at_rule", false},
            {:rule_reference, "qualified_rule", false},
          ]},
          line_number: 35,
        },
        %{
          name: "at_rule",
          body: {:sequence, [
            {:rule_reference, "AT_KEYWORD", true},
            {:rule_reference, "at_prelude", false},
            {:group, {:alternation, [
                {:rule_reference, "SEMICOLON", true},
                {:rule_reference, "block", false},
              ]}},
          ]},
          line_number: 55,
        },
        %{
          name: "at_prelude",
          body: {:repetition, {:rule_reference, "at_prelude_token", false}},
          line_number: 61,
        },
        %{
          name: "at_prelude_token",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "PERCENTAGE", true},
            {:rule_reference, "HASH", true},
            {:rule_reference, "CUSTOM_PROPERTY", true},
            {:rule_reference, "UNICODE_RANGE", true},
            {:rule_reference, "function_in_prelude", false},
            {:rule_reference, "paren_block", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "SLASH", true},
            {:rule_reference, "DOT", true},
            {:rule_reference, "STAR", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
            {:rule_reference, "GREATER", true},
            {:rule_reference, "TILDE", true},
            {:rule_reference, "PIPE", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "AMPERSAND", true},
            {:rule_reference, "CDO", true},
            {:rule_reference, "CDC", true},
          ]},
          line_number: 63,
        },
        %{
          name: "function_in_prelude",
          body: {:sequence, [
            {:rule_reference, "FUNCTION", true},
            {:rule_reference, "at_prelude_tokens", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 71,
        },
        %{
          name: "paren_block",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "at_prelude_tokens", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 72,
        },
        %{
          name: "at_prelude_tokens",
          body: {:repetition, {:rule_reference, "at_prelude_token", false}},
          line_number: 73,
        },
        %{
          name: "qualified_rule",
          body: {:sequence, [
            {:rule_reference, "selector_list", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 85,
        },
        %{
          name: "selector_list",
          body: {:sequence, [
            {:rule_reference, "complex_selector", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "complex_selector", false},
              ]}},
          ]},
          line_number: 96,
        },
        %{
          name: "complex_selector",
          body: {:sequence, [
            {:rule_reference, "compound_selector", false},
            {:repetition, {:sequence, [
                {:optional, {:rule_reference, "combinator", false}},
                {:rule_reference, "compound_selector", false},
              ]}},
          ]},
          line_number: 105,
        },
        %{
          name: "combinator",
          body: {:alternation, [
            {:rule_reference, "GREATER", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "TILDE", true},
          ]},
          line_number: 112,
        },
        %{
          name: "compound_selector",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "simple_selector", false},
              {:repetition, {:rule_reference, "subclass_selector", false}},
            ]},
            {:sequence, [
              {:rule_reference, "subclass_selector", false},
              {:repetition, {:rule_reference, "subclass_selector", false}},
            ]},
          ]},
          line_number: 124,
        },
        %{
          name: "simple_selector",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "STAR", true},
            {:rule_reference, "AMPERSAND", true},
          ]},
          line_number: 131,
        },
        %{
          name: "subclass_selector",
          body: {:alternation, [
            {:rule_reference, "class_selector", false},
            {:rule_reference, "id_selector", false},
            {:rule_reference, "attribute_selector", false},
            {:rule_reference, "pseudo_class", false},
            {:rule_reference, "pseudo_element", false},
          ]},
          line_number: 139,
        },
        %{
          name: "class_selector",
          body: {:sequence, [
            {:rule_reference, "DOT", true},
            {:rule_reference, "IDENT", true},
          ]},
          line_number: 145,
        },
        %{
          name: "id_selector",
          body: {:rule_reference, "HASH", true},
          line_number: 150,
        },
        %{
          name: "attribute_selector",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "IDENT", true},
            {:optional, {:sequence, [
                {:rule_reference, "attr_matcher", false},
                {:rule_reference, "attr_value", false},
                {:optional, {:rule_reference, "IDENT", true}},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 161,
        },
        %{
          name: "attr_matcher",
          body: {:alternation, [
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "TILDE_EQUALS", true},
            {:rule_reference, "PIPE_EQUALS", true},
            {:rule_reference, "CARET_EQUALS", true},
            {:rule_reference, "DOLLAR_EQUALS", true},
            {:rule_reference, "STAR_EQUALS", true},
          ]},
          line_number: 163,
        },
        %{
          name: "attr_value",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "STRING", true},
          ]},
          line_number: 166,
        },
        %{
          name: "pseudo_class",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "COLON", true},
              {:rule_reference, "FUNCTION", true},
              {:rule_reference, "pseudo_class_args", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "COLON", true},
              {:rule_reference, "IDENT", true},
            ]},
          ]},
          line_number: 173,
        },
        %{
          name: "pseudo_class_args",
          body: {:repetition, {:rule_reference, "pseudo_class_arg", false}},
          line_number: 181,
        },
        %{
          name: "pseudo_class_arg",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "HASH", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "DOT", true},
            {:rule_reference, "STAR", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "AMPERSAND", true},
            {:sequence, [
              {:rule_reference, "FUNCTION", true},
              {:rule_reference, "pseudo_class_args", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LBRACKET", true},
              {:rule_reference, "pseudo_class_args", false},
              {:rule_reference, "RBRACKET", true},
            ]},
          ]},
          line_number: 183,
        },
        %{
          name: "pseudo_element",
          body: {:sequence, [
            {:rule_reference, "COLON_COLON", true},
            {:rule_reference, "IDENT", true},
          ]},
          line_number: 190,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:rule_reference, "block_contents", false},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 200,
        },
        %{
          name: "block_contents",
          body: {:repetition, {:rule_reference, "block_item", false}},
          line_number: 202,
        },
        %{
          name: "block_item",
          body: {:alternation, [
            {:rule_reference, "at_rule", false},
            {:rule_reference, "declaration_or_nested", false},
          ]},
          line_number: 211,
        },
        %{
          name: "declaration_or_nested",
          body: {:alternation, [
            {:rule_reference, "declaration", false},
            {:rule_reference, "qualified_rule", false},
          ]},
          line_number: 217,
        },
        %{
          name: "declaration",
          body: {:sequence, [
            {:rule_reference, "property", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "value_list", false},
            {:optional, {:rule_reference, "priority", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 231,
        },
        %{
          name: "property",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "CUSTOM_PROPERTY", true},
          ]},
          line_number: 233,
        },
        %{
          name: "priority",
          body: {:sequence, [
            {:rule_reference, "BANG", true},
            {:literal, "important"},
          ]},
          line_number: 238,
        },
        %{
          name: "value_list",
          body: {:sequence, [
            {:rule_reference, "value", false},
            {:repetition, {:rule_reference, "value", false}},
          ]},
          line_number: 251,
        },
        %{
          name: "value",
          body: {:alternation, [
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "PERCENTAGE", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "IDENT", true},
            {:rule_reference, "HASH", true},
            {:rule_reference, "CUSTOM_PROPERTY", true},
            {:rule_reference, "UNICODE_RANGE", true},
            {:rule_reference, "function_call", false},
            {:rule_reference, "SLASH", true},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
          ]},
          line_number: 253,
        },
        %{
          name: "function_call",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "FUNCTION", true},
              {:rule_reference, "function_args", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "URL_TOKEN", true},
          ]},
          line_number: 267,
        },
        %{
          name: "function_args",
          body: {:repetition, {:rule_reference, "function_arg", false}},
          line_number: 272,
        },
        %{
          name: "function_arg",
          body: {:alternation, [
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "PERCENTAGE", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "IDENT", true},
            {:rule_reference, "HASH", true},
            {:rule_reference, "CUSTOM_PROPERTY", true},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "SLASH", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
            {:rule_reference, "STAR", true},
            {:sequence, [
              {:rule_reference, "FUNCTION", true},
              {:rule_reference, "function_args", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 274,
        },
      ],
      version: 1,
    }
  end
end
