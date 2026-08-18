defmodule CodingAdventures.FSharpParser.Grammar.V2_0 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: fsharp2.0.grammar
  # Regenerate with: grammar-tools compile-grammar fsharp2.0.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.

  alias CodingAdventures.GrammarTools.ParserGrammar

  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "compilation_unit",
          body: {:repetition, {:alternation, [
              {:rule_reference, "NEWLINE", true},
              {:rule_reference, "decorated_declaration", false},
            ]}},
          line_number: 9,
        },
        %{
          name: "decorated_declaration",
          body: {:sequence, [
            {:repetition, {:sequence, [
                {:rule_reference, "attribute_section", false},
                {:rule_reference, "NEWLINE", true},
              ]}},
            {:rule_reference, "declaration_body", false},
          ]},
          line_number: 11,
        },
        %{
          name: "declaration_body",
          body: {:alternation, [
            {:rule_reference, "module_declaration", false},
            {:rule_reference, "namespace_declaration", false},
            {:rule_reference, "open_declaration", false},
            {:rule_reference, "let_binding", false},
            {:rule_reference, "use_binding", false},
            {:rule_reference, "type_declaration", false},
            {:rule_reference, "member_declaration", false},
            {:rule_reference, "do_binding", false},
            {:rule_reference, "expression", false},
          ]},
          line_number: 13,
        },
        %{
          name: "module_declaration",
          body: {:sequence, [
            {:literal, "module"},
            {:repetition, {:rule_reference, "module_modifier", false}},
            {:optional, {:literal, "rec"}},
            {:rule_reference, "qualified_name", false},
            {:optional, {:sequence, [
                {:rule_reference, "EQUALS", true},
                {:repetition, {:rule_reference, "NEWLINE", true}},
              ]}},
          ]},
          line_number: 23,
        },
        %{
          name: "module_modifier",
          body: {:alternation, [
            {:literal, "private"},
            {:literal, "internal"},
          ]},
          line_number: 25,
        },
        %{
          name: "namespace_declaration",
          body: {:sequence, [
            {:literal, "namespace"},
            {:repetition, {:rule_reference, "namespace_modifier", false}},
            {:rule_reference, "qualified_name", false},
          ]},
          line_number: 28,
        },
        %{
          name: "namespace_modifier",
          body: {:alternation, [
            {:literal, "private"},
            {:literal, "internal"},
          ]},
          line_number: 30,
        },
        %{
          name: "open_declaration",
          body: {:sequence, [
            {:literal, "open"},
            {:rule_reference, "qualified_name", false},
          ]},
          line_number: 33,
        },
        %{
          name: "let_binding",
          body: {:sequence, [
            {:literal, "let"},
            {:optional, {:literal, "rec"}},
            {:repetition, {:rule_reference, "binding_modifier", false}},
            {:rule_reference, "binding_clause", false},
            {:repetition, {:sequence, [
                {:literal, "and"},
                {:repetition, {:rule_reference, "binding_modifier", false}},
                {:rule_reference, "binding_clause", false},
              ]}},
          ]},
          line_number: 35,
        },
        %{
          name: "use_binding",
          body: {:sequence, [
            {:literal, "use"},
            {:optional, {:literal, "rec"}},
            {:repetition, {:rule_reference, "binding_modifier", false}},
            {:rule_reference, "binding_clause", false},
            {:repetition, {:sequence, [
                {:literal, "and"},
                {:repetition, {:rule_reference, "binding_modifier", false}},
                {:rule_reference, "binding_clause", false},
              ]}},
          ]},
          line_number: 37,
        },
        %{
          name: "binding_modifier",
          body: {:alternation, [
            {:literal, "inline"},
            {:literal, "mutable"},
          ]},
          line_number: 39,
        },
        %{
          name: "binding_clause",
          body: {:sequence, [
            {:rule_reference, "pattern", false},
            {:repetition, {:rule_reference, "pattern", false}},
            {:optional, {:rule_reference, "type_annotation", false}},
            {:rule_reference, "EQUALS", true},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 42,
        },
        %{
          name: "do_binding",
          body: {:sequence, [
            {:literal, "do"},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 44,
        },
        %{
          name: "member_declaration",
          body: {:sequence, [
            {:optional, {:rule_reference, "member_modifier", false}},
            {:literal, "member"},
            {:rule_reference, "qualified_name", false},
            {:optional, {:rule_reference, "parameter_list", false}},
            {:optional, {:rule_reference, "type_annotation", false}},
            {:optional, {:sequence, [
                {:rule_reference, "EQUALS", true},
                {:repetition, {:rule_reference, "NEWLINE", true}},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 46,
        },
        %{
          name: "member_modifier",
          body: {:alternation, [
            {:literal, "static"},
            {:literal, "override"},
            {:literal, "default"},
            {:literal, "abstract"},
            {:literal, "new"},
          ]},
          line_number: 48,
        },
        %{
          name: "type_declaration",
          body: {:sequence, [
            {:literal, "type"},
            {:repetition, {:rule_reference, "type_modifier", false}},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "generic_parameters", false}},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "parameter_list", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "EQUALS", true},
                {:repetition, {:rule_reference, "NEWLINE", true}},
              ]}},
            {:rule_reference, "type_definition", false},
          ]},
          line_number: 54,
        },
        %{
          name: "type_modifier",
          body: {:alternation, [
            {:literal, "private"},
            {:literal, "public"},
            {:literal, "internal"},
          ]},
          line_number: 56,
        },
        %{
          name: "generic_parameters",
          body: {:sequence, [
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "type_parameter", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "type_parameter", false},
              ]}},
            {:rule_reference, "GREATER_THAN", true},
          ]},
          line_number: 60,
        },
        %{
          name: "type_parameter",
          body: {:alternation, [
            {:rule_reference, "TYPEVAR", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 62,
        },
        %{
          name: "type_definition",
          body: {:alternation, [
            {:rule_reference, "class_type", false},
            {:rule_reference, "interface_type", false},
            {:rule_reference, "struct_type", false},
            {:rule_reference, "record_type", false},
            {:rule_reference, "union_type", false},
            {:rule_reference, "alias_type", false},
          ]},
          line_number: 65,
        },
        %{
          name: "class_type",
          body: {:sequence, [
            {:literal, "class"},
            {:repetition, {:alternation, [
                {:rule_reference, "NEWLINE", true},
                {:rule_reference, "declaration_body", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 72,
        },
        %{
          name: "interface_type",
          body: {:sequence, [
            {:literal, "interface"},
            {:repetition, {:alternation, [
                {:rule_reference, "NEWLINE", true},
                {:rule_reference, "declaration_body", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 74,
        },
        %{
          name: "struct_type",
          body: {:sequence, [
            {:literal, "struct"},
            {:repetition, {:alternation, [
                {:rule_reference, "NEWLINE", true},
                {:rule_reference, "declaration_body", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 76,
        },
        %{
          name: "record_type",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "field_declaration", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "field_separator", false},
                    {:rule_reference, "field_declaration", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 78,
        },
        %{
          name: "field_declaration",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "type_expression", false},
          ]},
          line_number: 80,
        },
        %{
          name: "union_type",
          body: {:sequence, [
            {:rule_reference, "union_case", false},
            {:repetition, {:sequence, [
                {:rule_reference, "case_separator", false},
                {:rule_reference, "union_case", false},
              ]}},
          ]},
          line_number: 82,
        },
        %{
          name: "union_case",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "of"},
                {:rule_reference, "type_expression", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "type_expression", false},
                  ]}},
              ]}},
          ]},
          line_number: 84,
        },
        %{
          name: "alias_type",
          body: {:rule_reference, "type_expression", false},
          line_number: 86,
        },
        %{
          name: "field_separator",
          body: {:alternation, [
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "NEWLINE", true},
          ]},
          line_number: 88,
        },
        %{
          name: "case_separator",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "NEWLINE", true},
              {:optional, {:rule_reference, "PIPE", true}},
            ]},
            {:rule_reference, "PIPE", true},
          ]},
          line_number: 91,
        },
        %{
          name: "attribute_section",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "attribute", false},
            {:repetition, {:sequence, [
                {:rule_reference, "SEMICOLON", true},
                {:rule_reference, "attribute", false},
              ]}},
            {:rule_reference, "GREATER_THAN", true},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 94,
        },
        %{
          name: "attribute",
          body: {:sequence, [
            {:optional, {:sequence, [
                {:rule_reference, "attribute_target", false},
                {:rule_reference, "COLON", true},
              ]}},
            {:rule_reference, "qualified_name", false},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "attribute_arguments", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
          ]},
          line_number: 96,
        },
        %{
          name: "attribute_target",
          body: {:alternation, [
            {:literal, "assembly"},
            {:literal, "field"},
            {:literal, "method"},
            {:literal, "module"},
            {:literal, "param"},
            {:literal, "property"},
            {:literal, "return"},
            {:literal, "type"},
            {:literal, "event"},
          ]},
          line_number: 98,
        },
        %{
          name: "attribute_arguments",
          body: {:sequence, [
            {:rule_reference, "attribute_argument", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "attribute_argument", false},
              ]}},
          ]},
          line_number: 108,
        },
        %{
          name: "attribute_argument",
          body: {:sequence, [
            {:optional, {:sequence, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "EQUALS", true},
              ]}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 110,
        },
        %{
          name: "parameter_list",
          body: {:sequence, [
            {:rule_reference, "pattern", false},
            {:repetition, {:rule_reference, "pattern", false}},
          ]},
          line_number: 112,
        },
        %{
          name: "type_annotation",
          body: {:sequence, [
            {:rule_reference, "COLON", true},
            {:rule_reference, "type_expression", false},
          ]},
          line_number: 114,
        },
        %{
          name: "type_expression",
          body: {:rule_reference, "function_type", false},
          line_number: 116,
        },
        %{
          name: "function_type",
          body: {:sequence, [
            {:rule_reference, "type_product", false},
            {:optional, {:sequence, [
                {:rule_reference, "ARROW", true},
                {:rule_reference, "function_type", false},
              ]}},
          ]},
          line_number: 118,
        },
        %{
          name: "type_product",
          body: {:sequence, [
            {:rule_reference, "type_application", false},
            {:repetition, {:sequence, [
                {:rule_reference, "STAR", true},
                {:rule_reference, "type_application", false},
              ]}},
          ]},
          line_number: 120,
        },
        %{
          name: "type_application",
          body: {:sequence, [
            {:rule_reference, "type_atom", false},
            {:repetition, {:rule_reference, "type_atom", false}},
          ]},
          line_number: 122,
        },
        %{
          name: "generic_type_arguments",
          body: {:sequence, [
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "type_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "type_expression", false},
              ]}},
            {:rule_reference, "GREATER_THAN", true},
          ]},
          line_number: 124,
        },
        %{
          name: "type_atom",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "qualified_name", false},
              {:optional, {:rule_reference, "generic_type_arguments", false}},
            ]},
            {:rule_reference, "TYPEVAR", true},
            {:rule_reference, "tuple_type", false},
            {:rule_reference, "unit_type", false},
            {:rule_reference, "record_type", false},
            {:rule_reference, "parenthesized_type", false},
          ]},
          line_number: 126,
        },
        %{
          name: "tuple_type",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "type_expression", false},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "type_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "type_expression", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 133,
        },
        %{
          name: "unit_type",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 135,
        },
        %{
          name: "parenthesized_type",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "type_expression", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 137,
        },
        %{
          name: "qualified_name",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "DOT", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 139,
        },
        %{
          name: "expression",
          body: {:alternation, [
            {:rule_reference, "if_expression", false},
            {:rule_reference, "match_expression", false},
            {:rule_reference, "let_expression", false},
            {:rule_reference, "function_expression", false},
            {:rule_reference, "fun_expression", false},
            {:rule_reference, "for_expression", false},
            {:rule_reference, "while_expression", false},
            {:rule_reference, "sequence_expression", false},
          ]},
          line_number: 141,
        },
        %{
          name: "sequence_expression",
          body: {:sequence, [
            {:rule_reference, "infix_expression", false},
            {:repetition, {:alternation, [
                {:sequence, [
                  {:rule_reference, "NEWLINE", true},
                  {:rule_reference, "infix_expression", false},
                ]},
                {:sequence, [
                  {:rule_reference, "SEMICOLON", true},
                  {:rule_reference, "infix_expression", false},
                ]},
              ]}},
          ]},
          line_number: 150,
        },
        %{
          name: "infix_expression",
          body: {:sequence, [
            {:rule_reference, "application_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "infix_operator", false},
                {:rule_reference, "application_expression", false},
              ]}},
          ]},
          line_number: 152,
        },
        %{
          name: "infix_operator",
          body: {:alternation, [
            {:rule_reference, "PIPE_RIGHT", true},
            {:rule_reference, "PIPE_LEFT", true},
            {:rule_reference, "COMPOSE_RIGHT", true},
            {:rule_reference, "COMPOSE_LEFT", true},
            {:rule_reference, "NOT_EQUALS", true},
            {:rule_reference, "LESS_EQUALS", true},
            {:rule_reference, "GREATER_EQUALS", true},
            {:rule_reference, "EQUALS_EQUALS", true},
            {:rule_reference, "COLON_EQUALS", true},
            {:rule_reference, "DOUBLE_COLON", true},
            {:rule_reference, "LARROW", true},
            {:rule_reference, "ARROW", true},
            {:rule_reference, "DOT_DOT", true},
            {:rule_reference, "AND_AND", true},
            {:rule_reference, "OR_OR", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
            {:rule_reference, "STAR", true},
            {:rule_reference, "SLASH", true},
            {:rule_reference, "PERCENT", true},
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "GREATER_THAN", true},
            {:rule_reference, "AMPERSAND", true},
            {:rule_reference, "PIPE", true},
            {:rule_reference, "CARET", true},
          ]},
          line_number: 154,
        },
        %{
          name: "application_expression",
          body: {:sequence, [
            {:rule_reference, "prefix_expression", false},
            {:repetition, {:rule_reference, "prefix_expression", false}},
          ]},
          line_number: 181,
        },
        %{
          name: "prefix_expression",
          body: {:sequence, [
            {:optional, {:rule_reference, "unary_operator", false}},
            {:rule_reference, "atomic_expression", false},
          ]},
          line_number: 183,
        },
        %{
          name: "unary_operator",
          body: {:alternation, [
            {:rule_reference, "MINUS", true},
            {:rule_reference, "PLUS", true},
            {:rule_reference, "BANG", true},
          ]},
          line_number: 185,
        },
        %{
          name: "atomic_expression",
          body: {:alternation, [
            {:rule_reference, "computation_expression", false},
            {:rule_reference, "if_expression", false},
            {:rule_reference, "match_expression", false},
            {:rule_reference, "let_expression", false},
            {:rule_reference, "function_expression", false},
            {:rule_reference, "fun_expression", false},
            {:rule_reference, "for_expression", false},
            {:rule_reference, "while_expression", false},
            {:rule_reference, "unit_expression", false},
            {:rule_reference, "tuple_expression", false},
            {:rule_reference, "list_expression", false},
            {:rule_reference, "array_expression", false},
            {:rule_reference, "record_expression", false},
            {:rule_reference, "parenthesized_expression", false},
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "CHAR", true},
            {:rule_reference, "TRUE", true},
            {:rule_reference, "FALSE", true},
            {:rule_reference, "NULL", true},
          ]},
          line_number: 189,
        },
        %{
          name: "computation_expression",
          body: {:sequence, [
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "expression", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 211,
        },
        %{
          name: "unit_expression",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 213,
        },
        %{
          name: "tuple_expression",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 215,
        },
        %{
          name: "list_expression",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:negative_lookahead, {:rule_reference, "LESS_THAN", true}},
            {:optional, {:sequence, [
                {:rule_reference, "expression", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "element_separator", false},
                    {:rule_reference, "expression", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 217,
        },
        %{
          name: "array_expression",
          body: {:sequence, [
            {:rule_reference, "ARRAY_LBRACKET", true},
            {:optional, {:sequence, [
                {:rule_reference, "expression", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "element_separator", false},
                    {:rule_reference, "expression", false},
                  ]}},
              ]}},
            {:rule_reference, "ARRAY_RBRACKET", true},
          ]},
          line_number: 219,
        },
        %{
          name: "record_expression",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "field_assignment", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "field_separator", false},
                    {:rule_reference, "field_assignment", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 221,
        },
        %{
          name: "field_assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 223,
        },
        %{
          name: "parenthesized_expression",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 225,
        },
        %{
          name: "element_separator",
          body: {:alternation, [
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "NEWLINE", true},
          ]},
          line_number: 227,
        },
        %{
          name: "if_expression",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "expression", false},
            {:literal, "then"},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
            {:optional, {:sequence, [
                {:literal, "else"},
                {:repetition, {:rule_reference, "NEWLINE", true}},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 230,
        },
        %{
          name: "match_expression",
          body: {:sequence, [
            {:literal, "match"},
            {:rule_reference, "expression", false},
            {:literal, "with"},
            {:optional, {:rule_reference, "case_separator", false}},
            {:rule_reference, "match_case", false},
            {:repetition, {:sequence, [
                {:rule_reference, "case_separator", false},
                {:rule_reference, "match_case", false},
              ]}},
          ]},
          line_number: 232,
        },
        %{
          name: "match_case",
          body: {:sequence, [
            {:rule_reference, "pattern", false},
            {:optional, {:sequence, [
                {:literal, "when"},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "ARROW", true},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 234,
        },
        %{
          name: "fun_expression",
          body: {:sequence, [
            {:literal, "fun"},
            {:rule_reference, "parameter_list", false},
            {:rule_reference, "ARROW", true},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 236,
        },
        %{
          name: "function_expression",
          body: {:sequence, [
            {:literal, "function"},
            {:optional, {:rule_reference, "case_separator", false}},
            {:rule_reference, "match_case", false},
            {:repetition, {:sequence, [
                {:rule_reference, "case_separator", false},
                {:rule_reference, "match_case", false},
              ]}},
          ]},
          line_number: 238,
        },
        %{
          name: "let_expression",
          body: {:sequence, [
            {:literal, "let"},
            {:optional, {:literal, "rec"}},
            {:repetition, {:rule_reference, "binding_modifier", false}},
            {:rule_reference, "binding_clause", false},
            {:repetition, {:sequence, [
                {:literal, "and"},
                {:repetition, {:rule_reference, "binding_modifier", false}},
                {:rule_reference, "binding_clause", false},
              ]}},
            {:literal, "in"},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 240,
        },
        %{
          name: "for_expression",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "pattern", false},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
            {:group, {:alternation, [
                {:literal, "to"},
                {:literal, "downto"},
              ]}},
            {:rule_reference, "expression", false},
            {:literal, "do"},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 242,
        },
        %{
          name: "while_expression",
          body: {:sequence, [
            {:literal, "while"},
            {:rule_reference, "expression", false},
            {:literal, "do"},
            {:repetition, {:rule_reference, "NEWLINE", true}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 244,
        },
        %{
          name: "pattern",
          body: {:sequence, [
            {:rule_reference, "pattern_atom", false},
            {:optional, {:sequence, [
                {:rule_reference, "COLON", true},
                {:rule_reference, "type_expression", false},
              ]}},
            {:optional, {:sequence, [
                {:literal, "as"},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 246,
        },
        %{
          name: "pattern_atom",
          body: {:alternation, [
            {:rule_reference, "wildcard_pattern", false},
            {:rule_reference, "literal_pattern", false},
            {:rule_reference, "tuple_pattern", false},
            {:rule_reference, "list_pattern", false},
            {:rule_reference, "array_pattern", false},
            {:rule_reference, "record_pattern", false},
            {:rule_reference, "unit_pattern", false},
            {:rule_reference, "parenthesized_pattern", false},
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "TYPEVAR", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 248,
        },
        %{
          name: "wildcard_pattern",
          body: {:rule_reference, "UNDERSCORE", true},
          line_number: 260,
        },
        %{
          name: "literal_pattern",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "CHAR", true},
            {:rule_reference, "TRUE", true},
            {:rule_reference, "FALSE", true},
            {:rule_reference, "NULL", true},
          ]},
          line_number: 262,
        },
        %{
          name: "tuple_pattern",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "pattern", false},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "pattern", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "pattern", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 269,
        },
        %{
          name: "list_pattern",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:optional, {:sequence, [
                {:rule_reference, "pattern", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "element_separator", false},
                    {:rule_reference, "pattern", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 271,
        },
        %{
          name: "array_pattern",
          body: {:sequence, [
            {:rule_reference, "ARRAY_LBRACKET", true},
            {:optional, {:sequence, [
                {:rule_reference, "pattern", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "element_separator", false},
                    {:rule_reference, "pattern", false},
                  ]}},
              ]}},
            {:rule_reference, "ARRAY_RBRACKET", true},
          ]},
          line_number: 273,
        },
        %{
          name: "record_pattern",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "field_pattern", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "field_separator", false},
                    {:rule_reference, "field_pattern", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 275,
        },
        %{
          name: "field_pattern",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "pattern", false},
          ]},
          line_number: 277,
        },
        %{
          name: "unit_pattern",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 279,
        },
        %{
          name: "parenthesized_pattern",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "pattern", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 281,
        },
      ],
      version: 1,
    }
  end
end
