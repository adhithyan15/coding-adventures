defmodule CodingAdventures.LatticeParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: lattice.grammar
  # Regenerate with: grammar-tools compile-grammar lattice.grammar
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
          line_number: 37,
        },
        %{
          name: "rule",
          body: {:alternation, [
            {:rule_reference, "lattice_rule", false},
            {:rule_reference, "at_rule", false},
            {:rule_reference, "qualified_rule", false},
          ]},
          line_number: 39,
        },
        %{
          name: "lattice_rule",
          body: {:alternation, [
            {:rule_reference, "variable_declaration", false},
            {:rule_reference, "mixin_definition", false},
            {:rule_reference, "function_definition", false},
            {:rule_reference, "use_directive", false},
            {:rule_reference, "lattice_control", false},
          ]},
          line_number: 51,
        },
        %{
          name: "variable_declaration",
          body: {:sequence, [
            {:rule_reference, "VARIABLE", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "value_list", false},
            {:optional, {:alternation, [
                {:rule_reference, "BANG_DEFAULT", true},
                {:rule_reference, "BANG_GLOBAL", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 69,
        },
        %{
          name: "mixin_definition",
          body: {:alternation, [
            {:sequence, [
              {:literal, "@mixin"},
              {:rule_reference, "FUNCTION", true},
              {:optional, {:rule_reference, "mixin_params", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "block", false},
            ]},
            {:sequence, [
              {:literal, "@mixin"},
              {:rule_reference, "IDENT", true},
              {:rule_reference, "block", false},
            ]},
          ]},
          line_number: 102,
        },
        %{
          name: "mixin_params",
          body: {:sequence, [
            {:rule_reference, "mixin_param", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "mixin_param", false},
              ]}},
          ]},
          line_number: 105,
        },
        %{
          name: "mixin_param",
          body: {:sequence, [
            {:rule_reference, "VARIABLE", true},
            {:optional, {:sequence, [
                {:rule_reference, "COLON", true},
                {:rule_reference, "mixin_value_list", false},
              ]}},
          ]},
          line_number: 112,
        },
        %{
          name: "mixin_value_list",
          body: {:sequence, [
            {:rule_reference, "mixin_value", false},
            {:repetition, {:rule_reference, "mixin_value", false}},
          ]},
          line_number: 117,
        },
        %{
          name: "mixin_value",
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
            {:rule_reference, "VARIABLE", true},
            {:rule_reference, "SLASH", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
          ]},
          line_number: 119,
        },
        %{
          name: "include_directive",
          body: {:alternation, [
            {:sequence, [
              {:literal, "@include"},
              {:rule_reference, "FUNCTION", true},
              {:optional, {:rule_reference, "include_args", false}},
              {:rule_reference, "RPAREN", true},
              {:group, {:alternation, [
                  {:rule_reference, "SEMICOLON", true},
                  {:rule_reference, "block", false},
                ]}},
            ]},
            {:sequence, [
              {:literal, "@include"},
              {:rule_reference, "IDENT", true},
              {:group, {:alternation, [
                  {:rule_reference, "SEMICOLON", true},
                  {:rule_reference, "block", false},
                ]}},
            ]},
          ]},
          line_number: 130,
        },
        %{
          name: "include_args",
          body: {:sequence, [
            {:rule_reference, "include_arg", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "include_arg", false},
              ]}},
          ]},
          line_number: 133,
        },
        %{
          name: "include_arg",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "VARIABLE", true},
              {:rule_reference, "COLON", true},
              {:rule_reference, "value_list", false},
            ]},
            {:rule_reference, "value_list", false},
          ]},
          line_number: 137,
        },
        %{
          name: "lattice_control",
          body: {:alternation, [
            {:rule_reference, "if_directive", false},
            {:rule_reference, "for_directive", false},
            {:rule_reference, "each_directive", false},
            {:rule_reference, "while_directive", false},
          ]},
          line_number: 160,
        },
        %{
          name: "if_directive",
          body: {:sequence, [
            {:literal, "@if"},
            {:rule_reference, "lattice_expression", false},
            {:rule_reference, "block", false},
            {:repetition, {:sequence, [
                {:literal, "@else"},
                {:literal, "if"},
                {:rule_reference, "lattice_expression", false},
                {:rule_reference, "block", false},
              ]}},
            {:optional, {:sequence, [
                {:literal, "@else"},
                {:rule_reference, "block", false},
              ]}},
          ]},
          line_number: 164,
        },
        %{
          name: "for_directive",
          body: {:sequence, [
            {:literal, "@for"},
            {:rule_reference, "VARIABLE", true},
            {:literal, "from"},
            {:rule_reference, "lattice_expression", false},
            {:group, {:alternation, [
                {:literal, "through"},
                {:literal, "to"},
              ]}},
            {:rule_reference, "lattice_expression", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 171,
        },
        %{
          name: "each_directive",
          body: {:sequence, [
            {:literal, "@each"},
            {:rule_reference, "VARIABLE", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "VARIABLE", true},
              ]}},
            {:literal, "in"},
            {:rule_reference, "each_list", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 176,
        },
        %{
          name: "each_list",
          body: {:sequence, [
            {:rule_reference, "value", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "value", false},
              ]}},
          ]},
          line_number: 179,
        },
        %{
          name: "while_directive",
          body: {:sequence, [
            {:literal, "@while"},
            {:rule_reference, "lattice_expression", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 184,
        },
        %{
          name: "lattice_expression",
          body: {:rule_reference, "lattice_or_expr", false},
          line_number: 203,
        },
        %{
          name: "lattice_or_expr",
          body: {:sequence, [
            {:rule_reference, "lattice_and_expr", false},
            {:repetition, {:sequence, [
                {:literal, "or"},
                {:rule_reference, "lattice_and_expr", false},
              ]}},
          ]},
          line_number: 205,
        },
        %{
          name: "lattice_and_expr",
          body: {:sequence, [
            {:rule_reference, "lattice_comparison", false},
            {:repetition, {:sequence, [
                {:literal, "and"},
                {:rule_reference, "lattice_comparison", false},
              ]}},
          ]},
          line_number: 207,
        },
        %{
          name: "lattice_comparison",
          body: {:sequence, [
            {:rule_reference, "lattice_additive", false},
            {:optional, {:sequence, [
                {:rule_reference, "comparison_op", false},
                {:rule_reference, "lattice_additive", false},
              ]}},
          ]},
          line_number: 209,
        },
        %{
          name: "comparison_op",
          body: {:alternation, [
            {:rule_reference, "EQUALS_EQUALS", true},
            {:rule_reference, "NOT_EQUALS", true},
            {:rule_reference, "GREATER", true},
            {:rule_reference, "GREATER_EQUALS", true},
            {:rule_reference, "LESS", true},
            {:rule_reference, "LESS_EQUALS", true},
          ]},
          line_number: 211,
        },
        %{
          name: "lattice_additive",
          body: {:sequence, [
            {:rule_reference, "lattice_multiplicative", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "lattice_multiplicative", false},
              ]}},
          ]},
          line_number: 214,
        },
        %{
          name: "lattice_multiplicative",
          body: {:sequence, [
            {:rule_reference, "lattice_unary", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                  ]}},
                {:rule_reference, "lattice_unary", false},
              ]}},
          ]},
          line_number: 219,
        },
        %{
          name: "lattice_unary",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "MINUS", true},
              {:rule_reference, "lattice_unary", false},
            ]},
            {:rule_reference, "lattice_primary", false},
          ]},
          line_number: 221,
        },
        %{
          name: "lattice_primary",
          body: {:alternation, [
            {:rule_reference, "VARIABLE", true},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "DIMENSION", true},
            {:rule_reference, "PERCENTAGE", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "IDENT", true},
            {:rule_reference, "HASH", true},
            {:literal, "true"},
            {:literal, "false"},
            {:literal, "null"},
            {:rule_reference, "function_call", false},
            {:rule_reference, "map_literal", false},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "lattice_expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 224,
        },
        %{
          name: "map_literal",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "map_entry", false},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "map_entry", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "map_entry", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 235,
        },
        %{
          name: "map_entry",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "IDENT", true},
                {:rule_reference, "STRING", true},
              ]}},
            {:rule_reference, "COLON", true},
            {:rule_reference, "lattice_expression", false},
          ]},
          line_number: 237,
        },
        %{
          name: "function_definition",
          body: {:alternation, [
            {:sequence, [
              {:literal, "@function"},
              {:rule_reference, "FUNCTION", true},
              {:optional, {:rule_reference, "mixin_params", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "function_body", false},
            ]},
            {:sequence, [
              {:literal, "@function"},
              {:rule_reference, "IDENT", true},
              {:rule_reference, "function_body", false},
            ]},
          ]},
          line_number: 261,
        },
        %{
          name: "function_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "function_body_item", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 264,
        },
        %{
          name: "function_body_item",
          body: {:alternation, [
            {:rule_reference, "variable_declaration", false},
            {:rule_reference, "return_directive", false},
            {:rule_reference, "lattice_control", false},
          ]},
          line_number: 266,
        },
        %{
          name: "return_directive",
          body: {:sequence, [
            {:literal, "@return"},
            {:rule_reference, "lattice_expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 268,
        },
        %{
          name: "use_directive",
          body: {:sequence, [
            {:literal, "@use"},
            {:rule_reference, "STRING", true},
            {:optional, {:sequence, [
                {:literal, "as"},
                {:rule_reference, "IDENT", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 281,
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
          line_number: 294,
        },
        %{
          name: "at_prelude",
          body: {:repetition, {:rule_reference, "at_prelude_token", false}},
          line_number: 296,
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
            {:rule_reference, "VARIABLE", true},
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
          line_number: 298,
        },
        %{
          name: "function_in_prelude",
          body: {:sequence, [
            {:rule_reference, "FUNCTION", true},
            {:rule_reference, "at_prelude_tokens", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 306,
        },
        %{
          name: "paren_block",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "at_prelude_tokens", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 307,
        },
        %{
          name: "at_prelude_tokens",
          body: {:repetition, {:rule_reference, "at_prelude_token", false}},
          line_number: 308,
        },
        %{
          name: "qualified_rule",
          body: {:sequence, [
            {:rule_reference, "selector_list", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 314,
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
          line_number: 320,
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
          line_number: 322,
        },
        %{
          name: "combinator",
          body: {:alternation, [
            {:rule_reference, "GREATER", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "TILDE", true},
          ]},
          line_number: 324,
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
          line_number: 326,
        },
        %{
          name: "simple_selector",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "STAR", true},
            {:rule_reference, "AMPERSAND", true},
            {:rule_reference, "VARIABLE", true},
            {:rule_reference, "PERCENTAGE", true},
          ]},
          line_number: 331,
        },
        %{
          name: "subclass_selector",
          body: {:alternation, [
            {:rule_reference, "class_selector", false},
            {:rule_reference, "id_selector", false},
            {:rule_reference, "placeholder_selector", false},
            {:rule_reference, "attribute_selector", false},
            {:rule_reference, "pseudo_class", false},
            {:rule_reference, "pseudo_element", false},
          ]},
          line_number: 334,
        },
        %{
          name: "placeholder_selector",
          body: {:rule_reference, "PLACEHOLDER", true},
          line_number: 338,
        },
        %{
          name: "class_selector",
          body: {:sequence, [
            {:rule_reference, "DOT", true},
            {:rule_reference, "IDENT", true},
          ]},
          line_number: 340,
        },
        %{
          name: "id_selector",
          body: {:rule_reference, "HASH", true},
          line_number: 342,
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
          line_number: 344,
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
          line_number: 346,
        },
        %{
          name: "attr_value",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "STRING", true},
          ]},
          line_number: 349,
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
          line_number: 351,
        },
        %{
          name: "pseudo_class_args",
          body: {:repetition, {:rule_reference, "pseudo_class_arg", false}},
          line_number: 354,
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
          line_number: 356,
        },
        %{
          name: "pseudo_element",
          body: {:sequence, [
            {:rule_reference, "COLON_COLON", true},
            {:rule_reference, "IDENT", true},
          ]},
          line_number: 361,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:rule_reference, "block_contents", false},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 371,
        },
        %{
          name: "block_contents",
          body: {:repetition, {:rule_reference, "block_item", false}},
          line_number: 373,
        },
        %{
          name: "block_item",
          body: {:alternation, [
            {:rule_reference, "lattice_block_item", false},
            {:rule_reference, "at_rule", false},
            {:rule_reference, "declaration_or_nested", false},
          ]},
          line_number: 375,
        },
        %{
          name: "lattice_block_item",
          body: {:alternation, [
            {:rule_reference, "variable_declaration", false},
            {:rule_reference, "include_directive", false},
            {:rule_reference, "lattice_control", false},
            {:rule_reference, "content_directive", false},
            {:rule_reference, "extend_directive", false},
            {:rule_reference, "at_root_directive", false},
          ]},
          line_number: 381,
        },
        %{
          name: "content_directive",
          body: {:sequence, [
            {:literal, "@content"},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 391,
        },
        %{
          name: "extend_directive",
          body: {:sequence, [
            {:literal, "@extend"},
            {:rule_reference, "selector_list", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 399,
        },
        %{
          name: "at_root_directive",
          body: {:sequence, [
            {:literal, "@at-root"},
            {:group, {:alternation, [
                {:sequence, [
                  {:rule_reference, "selector_list", false},
                  {:rule_reference, "block", false},
                ]},
                {:rule_reference, "block", false},
              ]}},
          ]},
          line_number: 404,
        },
        %{
          name: "declaration_or_nested",
          body: {:alternation, [
            {:rule_reference, "declaration", false},
            {:rule_reference, "qualified_rule", false},
          ]},
          line_number: 406,
        },
        %{
          name: "declaration",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "property", false},
              {:rule_reference, "COLON", true},
              {:rule_reference, "value_list", false},
              {:optional, {:rule_reference, "priority", false}},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "property", false},
              {:rule_reference, "COLON", true},
              {:rule_reference, "block", false},
            ]},
          ]},
          line_number: 415,
        },
        %{
          name: "property",
          body: {:alternation, [
            {:rule_reference, "IDENT", true},
            {:rule_reference, "CUSTOM_PROPERTY", true},
          ]},
          line_number: 418,
        },
        %{
          name: "priority",
          body: {:sequence, [
            {:rule_reference, "BANG", true},
            {:literal, "important"},
          ]},
          line_number: 420,
        },
        %{
          name: "value_list",
          body: {:sequence, [
            {:rule_reference, "value", false},
            {:repetition, {:rule_reference, "value", false}},
          ]},
          line_number: 431,
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
            {:rule_reference, "VARIABLE", true},
            {:rule_reference, "SLASH", true},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
            {:rule_reference, "map_literal", false},
          ]},
          line_number: 433,
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
          line_number: 439,
        },
        %{
          name: "function_args",
          body: {:repetition, {:rule_reference, "function_arg", false}},
          line_number: 442,
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
            {:rule_reference, "VARIABLE", true},
            {:sequence, [
              {:rule_reference, "FUNCTION", true},
              {:rule_reference, "function_args", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 444,
        },
      ],
      version: 1,
    }
  end
end
