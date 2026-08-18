defmodule CodingAdventures.JavaParser.Grammar.V1_4 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: java1.4.grammar
  # Regenerate with: grammar-tools compile-grammar java1.4.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:rule_reference, "program_item", false}},
          line_number: 86,
        },
        %{
          name: "program_item",
          body: {:alternation, [
            {:rule_reference, "package_declaration", false},
            {:rule_reference, "import_declaration", false},
            {:rule_reference, "type_declaration", false},
            {:rule_reference, "method_declaration", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 87,
        },
        %{
          name: "compilation_unit",
          body: {:sequence, [
            {:optional, {:rule_reference, "package_declaration", false}},
            {:repetition, {:rule_reference, "import_declaration", false}},
            {:repetition, {:rule_reference, "type_declaration", false}},
          ]},
          line_number: 88,
        },
        %{
          name: "package_declaration",
          body: {:sequence, [
            {:literal, "package"},
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 107,
        },
        %{
          name: "import_declaration",
          body: {:sequence, [
            {:literal, "import"},
            {:rule_reference, "qualified_name", false},
            {:optional, {:sequence, [
                {:rule_reference, "DOT", true},
                {:rule_reference, "STAR", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 128,
        },
        %{
          name: "type_declaration",
          body: {:alternation, [
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 144,
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
          line_number: 164,
        },
        %{
          name: "class_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "class_modifier", false}},
            {:literal, "class"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "extends"},
                {:rule_reference, "class_type", false},
              ]}},
            {:optional, {:sequence, [
                {:literal, "implements"},
                {:rule_reference, "interface_type_list", false},
              ]}},
            {:rule_reference, "class_body", false},
          ]},
          line_number: 200,
        },
        %{
          name: "class_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "abstract"},
            {:literal, "final"},
            {:literal, "strictfp"},
          ]},
          line_number: 205,
        },
        %{
          name: "class_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "class_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 232,
        },
        %{
          name: "class_body_declaration",
          body: {:alternation, [
            {:rule_reference, "static_initializer", false},
            {:rule_reference, "instance_initializer", false},
            {:rule_reference, "constructor_declaration", false},
            {:rule_reference, "method_declaration", false},
            {:rule_reference, "field_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 234,
        },
        %{
          name: "instance_initializer",
          body: {:rule_reference, "block", false},
          line_number: 271,
        },
        %{
          name: "interface_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "interface_modifier", false}},
            {:literal, "interface"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "extends"},
                {:rule_reference, "interface_type_list", false},
              ]}},
            {:rule_reference, "interface_body", false},
          ]},
          line_number: 309,
        },
        %{
          name: "interface_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "abstract"},
            {:literal, "strictfp"},
          ]},
          line_number: 313,
        },
        %{
          name: "interface_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "interface_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 317,
        },
        %{
          name: "interface_body_declaration",
          body: {:alternation, [
            {:rule_reference, "interface_method_declaration", false},
            {:rule_reference, "interface_field_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 319,
        },
        %{
          name: "interface_field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "constant_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 328,
        },
        %{
          name: "constant_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "static"},
            {:literal, "final"},
          ]},
          line_number: 330,
        },
        %{
          name: "interface_method_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "interface_method_modifier", false}},
            {:rule_reference, "result_type", false},
            {:rule_reference, "method_declarator", false},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 337,
        },
        %{
          name: "interface_method_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "abstract"},
          ]},
          line_number: 340,
        },
        %{
          name: "interface_type_list",
          body: {:sequence, [
            {:rule_reference, "class_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "class_type", false},
              ]}},
          ]},
          line_number: 346,
        },
        %{
          name: "field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 366,
        },
        %{
          name: "field_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
            {:literal, "static"},
            {:literal, "final"},
            {:literal, "transient"},
            {:literal, "volatile"},
          ]},
          line_number: 368,
        },
        %{
          name: "variable_declarators",
          body: {:sequence, [
            {:rule_reference, "variable_declarator", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "variable_declarator", false},
              ]}},
          ]},
          line_number: 385,
        },
        %{
          name: "variable_declarator",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "RBRACKET", true},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "EQUALS", true},
                {:rule_reference, "variable_initializer", false},
              ]}},
          ]},
          line_number: 387,
        },
        %{
          name: "variable_initializer",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "array_initializer", false},
          ]},
          line_number: 392,
        },
        %{
          name: "array_initializer",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "variable_initializer", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "variable_initializer", false},
                  ]}},
              ]}},
            {:optional, {:rule_reference, "COMMA", true}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 402,
        },
        %{
          name: "method_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "method_modifier", false}},
            {:rule_reference, "result_type", false},
            {:rule_reference, "method_declarator", false},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "method_body", false},
          ]},
          line_number: 437,
        },
        %{
          name: "method_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
            {:literal, "static"},
            {:literal, "abstract"},
            {:literal, "final"},
            {:literal, "synchronized"},
            {:literal, "native"},
            {:literal, "strictfp"},
          ]},
          line_number: 440,
        },
        %{
          name: "result_type",
          body: {:alternation, [
            {:literal, "void"},
            {:rule_reference, "type", false},
          ]},
          line_number: 452,
        },
        %{
          name: "method_declarator",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "formal_parameter_list", false}},
            {:rule_reference, "RPAREN", true},
            {:repetition, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "RBRACKET", true},
              ]}},
          ]},
          line_number: 462,
        },
        %{
          name: "formal_parameter_list",
          body: {:sequence, [
            {:rule_reference, "formal_parameter", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "formal_parameter", false},
              ]}},
          ]},
          line_number: 472,
        },
        %{
          name: "formal_parameter",
          body: {:sequence, [
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "RBRACKET", true},
              ]}},
          ]},
          line_number: 474,
        },
        %{
          name: "throws_clause",
          body: {:sequence, [
            {:literal, "throws"},
            {:rule_reference, "class_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "class_type", false},
              ]}},
          ]},
          line_number: 482,
        },
        %{
          name: "method_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 487,
        },
        %{
          name: "constructor_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "constructor_modifier", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "formal_parameter_list", false}},
            {:rule_reference, "RPAREN", true},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "constructor_body", false},
          ]},
          line_number: 508,
        },
        %{
          name: "constructor_modifier",
          body: {:alternation, [
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
          ]},
          line_number: 512,
        },
        %{
          name: "constructor_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "explicit_constructor_invocation", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 519,
        },
        %{
          name: "explicit_constructor_invocation",
          body: {:alternation, [
            {:sequence, [
              {:literal, "this"},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:literal, "super"},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "primary_expression", false},
              {:rule_reference, "DOT", true},
              {:literal, "super"},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "SEMICOLON", true},
            ]},
          ]},
          line_number: 529,
        },
        %{
          name: "static_initializer",
          body: {:sequence, [
            {:literal, "static"},
            {:rule_reference, "block", false},
          ]},
          line_number: 549,
        },
        %{
          name: "type",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "primitive_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
            ]},
            {:sequence, [
              {:rule_reference, "class_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
            ]},
          ]},
          line_number: 578,
        },
        %{
          name: "primitive_type",
          body: {:alternation, [
            {:literal, "boolean"},
            {:literal, "byte"},
            {:literal, "short"},
            {:literal, "int"},
            {:literal, "long"},
            {:literal, "char"},
            {:literal, "float"},
            {:literal, "double"},
          ]},
          line_number: 586,
        },
        %{
          name: "class_type",
          body: {:rule_reference, "qualified_name", false},
          line_number: 599,
        },
        %{
          name: "statement",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "var_declaration", false},
            {:rule_reference, "empty_statement", false},
            {:rule_reference, "expression_statement", false},
            {:rule_reference, "if_statement", false},
            {:rule_reference, "while_statement", false},
            {:rule_reference, "do_while_statement", false},
            {:rule_reference, "for_statement", false},
            {:rule_reference, "switch_statement", false},
            {:rule_reference, "try_statement", false},
            {:rule_reference, "throw_statement", false},
            {:rule_reference, "return_statement", false},
            {:rule_reference, "break_statement", false},
            {:rule_reference, "continue_statement", false},
            {:rule_reference, "synchronized_statement", false},
            {:rule_reference, "labelled_statement", false},
            {:rule_reference, "assert_statement", false},
          ]},
          line_number: 619,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 648,
        },
        %{
          name: "block_statement",
          body: {:alternation, [
            {:rule_reference, "var_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 653,
        },
        %{
          name: "var_declaration",
          body: {:rule_reference, "local_variable_declaration_statement", false},
          line_number: 674,
        },
        %{
          name: "local_variable_declaration_statement",
          body: {:sequence, [
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 676,
        },
        %{
          name: "empty_statement",
          body: {:rule_reference, "SEMICOLON", true},
          line_number: 683,
        },
        %{
          name: "expression_statement",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 697,
        },
        %{
          name: "if_statement",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "statement", false},
            {:optional, {:sequence, [
                {:literal, "else"},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 714,
        },
        %{
          name: "while_statement",
          body: {:sequence, [
            {:literal, "while"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 730,
        },
        %{
          name: "do_while_statement",
          body: {:sequence, [
            {:literal, "do"},
            {:rule_reference, "statement", false},
            {:literal, "while"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 742,
        },
        %{
          name: "for_statement",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "for_init", false},
            {:rule_reference, "SEMICOLON", true},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
            {:optional, {:rule_reference, "for_update", false}},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 766,
        },
        %{
          name: "for_init",
          body: {:alternation, [
            {:sequence, [
              {:optional, {:literal, "final"}},
              {:rule_reference, "type", false},
              {:rule_reference, "variable_declarators", false},
            ]},
            {:optional, {:rule_reference, "expression_list", false}},
          ]},
          line_number: 772,
        },
        %{
          name: "for_update",
          body: {:rule_reference, "expression_list", false},
          line_number: 778,
        },
        %{
          name: "expression_list",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 784,
        },
        %{
          name: "switch_statement",
          body: {:sequence, [
            {:literal, "switch"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "switch_block", false},
          ]},
          line_number: 802,
        },
        %{
          name: "switch_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "switch_block_statement_group", false}},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 804,
        },
        %{
          name: "switch_block_statement_group",
          body: {:sequence, [
            {:rule_reference, "switch_label", false},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
          ]},
          line_number: 806,
        },
        %{
          name: "switch_label",
          body: {:alternation, [
            {:sequence, [
              {:literal, "case"},
              {:rule_reference, "expression", false},
              {:rule_reference, "COLON", true},
            ]},
            {:sequence, [
              {:literal, "default"},
              {:rule_reference, "COLON", true},
            ]},
          ]},
          line_number: 808,
        },
        %{
          name: "try_statement",
          body: {:sequence, [
            {:literal, "try"},
            {:rule_reference, "block", false},
            {:group, {:alternation, [
                {:sequence, [
                  {:rule_reference, "catch_clause", false},
                  {:repetition, {:rule_reference, "catch_clause", false}},
                  {:optional, {:rule_reference, "finally_clause", false}},
                ]},
                {:rule_reference, "finally_clause", false},
              ]}},
          ]},
          line_number: 845,
        },
        %{
          name: "catch_clause",
          body: {:sequence, [
            {:literal, "catch"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "formal_parameter", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "block", false},
          ]},
          line_number: 848,
        },
        %{
          name: "finally_clause",
          body: {:sequence, [
            {:literal, "finally"},
            {:rule_reference, "block", false},
          ]},
          line_number: 850,
        },
        %{
          name: "throw_statement",
          body: {:sequence, [
            {:literal, "throw"},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 861,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 868,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 883,
        },
        %{
          name: "continue_statement",
          body: {:sequence, [
            {:literal, "continue"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 885,
        },
        %{
          name: "synchronized_statement",
          body: {:sequence, [
            {:literal, "synchronized"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "block", false},
          ]},
          line_number: 904,
        },
        %{
          name: "labelled_statement",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 918,
        },
        %{
          name: "assert_statement",
          body: {:sequence, [
            {:literal, "assert"},
            {:rule_reference, "expression", false},
            {:optional, {:sequence, [
                {:rule_reference, "COLON", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 967,
        },
        %{
          name: "expression",
          body: {:rule_reference, "assignment_expression", false},
          line_number: 1021,
        },
        %{
          name: "assignment_expression",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "unary_expression", false},
              {:rule_reference, "assignment_operator", false},
              {:rule_reference, "assignment_expression", false},
            ]},
            {:rule_reference, "conditional_expression", false},
          ]},
          line_number: 1023,
        },
        %{
          name: "assignment_operator",
          body: {:alternation, [
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "PLUS_EQUALS", true},
            {:rule_reference, "MINUS_EQUALS", true},
            {:rule_reference, "STAR_EQUALS", true},
            {:rule_reference, "SLASH_EQUALS", true},
            {:rule_reference, "PERCENT_EQUALS", true},
            {:rule_reference, "AMPERSAND_EQUALS", true},
            {:rule_reference, "PIPE_EQUALS", true},
            {:rule_reference, "CARET_EQUALS", true},
            {:rule_reference, "LEFT_SHIFT_EQUALS", true},
            {:rule_reference, "RIGHT_SHIFT_EQUALS", true},
            {:rule_reference, "UNSIGNED_RIGHT_SHIFT_EQUALS", true},
          ]},
          line_number: 1026,
        },
        %{
          name: "conditional_expression",
          body: {:sequence, [
            {:rule_reference, "logical_or_expression", false},
            {:optional, {:sequence, [
                {:rule_reference, "QUESTION", true},
                {:rule_reference, "assignment_expression", false},
                {:rule_reference, "COLON", true},
                {:rule_reference, "assignment_expression", false},
              ]}},
          ]},
          line_number: 1046,
        },
        %{
          name: "logical_or_expression",
          body: {:sequence, [
            {:rule_reference, "logical_and_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "OR_OR", true},
                {:rule_reference, "logical_and_expression", false},
              ]}},
          ]},
          line_number: 1056,
        },
        %{
          name: "logical_and_expression",
          body: {:sequence, [
            {:rule_reference, "bitwise_or_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "AND_AND", true},
                {:rule_reference, "bitwise_or_expression", false},
              ]}},
          ]},
          line_number: 1064,
        },
        %{
          name: "bitwise_or_expression",
          body: {:sequence, [
            {:rule_reference, "bitwise_xor_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "PIPE", true},
                {:rule_reference, "bitwise_xor_expression", false},
              ]}},
          ]},
          line_number: 1072,
        },
        %{
          name: "bitwise_xor_expression",
          body: {:sequence, [
            {:rule_reference, "bitwise_and_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "CARET", true},
                {:rule_reference, "bitwise_and_expression", false},
              ]}},
          ]},
          line_number: 1080,
        },
        %{
          name: "bitwise_and_expression",
          body: {:sequence, [
            {:rule_reference, "equality_expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "AMPERSAND", true},
                {:rule_reference, "equality_expression", false},
              ]}},
          ]},
          line_number: 1088,
        },
        %{
          name: "equality_expression",
          body: {:sequence, [
            {:rule_reference, "relational_expression", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "EQUALS_EQUALS", true},
                    {:rule_reference, "NOT_EQUALS", true},
                  ]}},
                {:rule_reference, "relational_expression", false},
              ]}},
          ]},
          line_number: 1099,
        },
        %{
          name: "relational_expression",
          body: {:sequence, [
            {:rule_reference, "shift_expression", false},
            {:repetition, {:alternation, [
                {:sequence, [
                  {:group, {:alternation, [
                      {:rule_reference, "LESS_THAN", true},
                      {:rule_reference, "GREATER_THAN", true},
                      {:rule_reference, "LESS_EQUALS", true},
                      {:rule_reference, "GREATER_EQUALS", true},
                    ]}},
                  {:rule_reference, "shift_expression", false},
                ]},
                {:sequence, [
                  {:literal, "instanceof"},
                  {:rule_reference, "type", false},
                ]},
              ]}},
          ]},
          line_number: 1114,
        },
        %{
          name: "shift_expression",
          body: {:sequence, [
            {:rule_reference, "additive_expression", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "LEFT_SHIFT", true},
                    {:rule_reference, "RIGHT_SHIFT", true},
                    {:rule_reference, "UNSIGNED_RIGHT_SHIFT", true},
                  ]}},
                {:rule_reference, "additive_expression", false},
              ]}},
          ]},
          line_number: 1132,
        },
        %{
          name: "additive_expression",
          body: {:sequence, [
            {:rule_reference, "multiplicative_expression", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "multiplicative_expression", false},
              ]}},
          ]},
          line_number: 1143,
        },
        %{
          name: "multiplicative_expression",
          body: {:sequence, [
            {:rule_reference, "unary_expression", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                    {:rule_reference, "PERCENT", true},
                  ]}},
                {:rule_reference, "unary_expression", false},
              ]}},
          ]},
          line_number: 1155,
        },
        %{
          name: "unary_expression",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "PLUS_PLUS", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:sequence, [
              {:rule_reference, "MINUS_MINUS", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:sequence, [
              {:rule_reference, "PLUS", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:sequence, [
              {:rule_reference, "MINUS", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:rule_reference, "unary_expression_not_plus_minus", false},
          ]},
          line_number: 1177,
        },
        %{
          name: "unary_expression_not_plus_minus",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "TILDE", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:sequence, [
              {:rule_reference, "BANG", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:rule_reference, "cast_expression", false},
            {:rule_reference, "postfix_expression", false},
          ]},
          line_number: 1183,
        },
        %{
          name: "cast_expression",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "primitive_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "unary_expression", false},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "class_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "unary_expression_not_plus_minus", false},
            ]},
          ]},
          line_number: 1201,
        },
        %{
          name: "postfix_expression",
          body: {:sequence, [
            {:rule_reference, "primary_expression", false},
            {:repetition, {:alternation, [
                {:rule_reference, "PLUS_PLUS", true},
                {:rule_reference, "MINUS_MINUS", true},
              ]}},
          ]},
          line_number: 1212,
        },
        %{
          name: "primary_expression",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:rule_reference, "primary_suffix", false}},
          ]},
          line_number: 1233,
        },
        %{
          name: "primary_suffix",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "DOT", true},
              {:rule_reference, "NAME", true},
            ]},
            {:sequence, [
              {:rule_reference, "DOT", true},
              {:literal, "class"},
            ]},
            {:sequence, [
              {:rule_reference, "DOT", true},
              {:literal, "this"},
            ]},
            {:sequence, [
              {:rule_reference, "DOT", true},
              {:literal, "new"},
              {:rule_reference, "NAME", true},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:optional, {:rule_reference, "class_body", false}},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LBRACKET", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RBRACKET", true},
            ]},
          ]},
          line_number: 1235,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "literal", false},
            {:literal, "this"},
            {:sequence, [
              {:literal, "super"},
              {:rule_reference, "DOT", true},
              {:rule_reference, "NAME", true},
            ]},
            {:sequence, [
              {:literal, "super"},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:literal, "new"},
              {:rule_reference, "class_type", false},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:optional, {:rule_reference, "class_body", false}},
            ]},
            {:sequence, [
              {:literal, "new"},
              {:rule_reference, "array_creation_type", false},
              {:rule_reference, "array_dimension_exprs", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
            ]},
            {:sequence, [
              {:literal, "new"},
              {:rule_reference, "array_creation_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "array_initializer", false},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "primitive_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "DOT", true},
              {:literal, "class"},
            ]},
            {:sequence, [
              {:literal, "void"},
              {:rule_reference, "DOT", true},
              {:literal, "class"},
            ]},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1271,
        },
        %{
          name: "argument_list",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 1287,
        },
        %{
          name: "array_creation_type",
          body: {:alternation, [
            {:rule_reference, "primitive_type", false},
            {:rule_reference, "class_type", false},
          ]},
          line_number: 1301,
        },
        %{
          name: "array_dimension_exprs",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RBRACKET", true},
            {:repetition, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "expression", false},
                {:rule_reference, "RBRACKET", true},
              ]}},
          ]},
          line_number: 1306,
        },
        %{
          name: "literal",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "CHARACTER", true},
            {:rule_reference, "STRING", true},
            {:literal, "true"},
            {:literal, "false"},
            {:literal, "null"},
          ]},
          line_number: 1327,
        },
      ],
      version: 1,
    }
  end
  end
