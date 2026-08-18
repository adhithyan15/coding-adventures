defmodule CodingAdventures.JavaParser.Grammar.V5 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: java5.grammar
  # Regenerate with: grammar-tools compile-grammar java5.grammar
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
          line_number: 96,
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
          line_number: 97,
        },
        %{
          name: "compilation_unit",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:rule_reference, "package_declaration", false}},
            {:repetition, {:rule_reference, "import_declaration", false}},
            {:repetition, {:rule_reference, "type_declaration", false}},
          ]},
          line_number: 98,
        },
        %{
          name: "package_declaration",
          body: {:sequence, [
            {:literal, "package"},
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 125,
        },
        %{
          name: "import_declaration",
          body: {:sequence, [
            {:literal, "import"},
            {:optional, {:literal, "static"}},
            {:rule_reference, "qualified_name", false},
            {:optional, {:sequence, [
                {:rule_reference, "DOT", true},
                {:rule_reference, "STAR", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 154,
        },
        %{
          name: "type_declaration",
          body: {:alternation, [
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "enum_declaration", false},
            {:rule_reference, "annotation_type_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 171,
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
          line_number: 193,
        },
        %{
          name: "annotation",
          body: {:sequence, [
            {:rule_reference, "AT", true},
            {:rule_reference, "qualified_name", false},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:alternation, [
                    {:rule_reference, "element_value_pairs", false},
                    {:rule_reference, "element_value", false},
                  ]}},
                {:rule_reference, "RPAREN", true},
              ]}},
          ]},
          line_number: 241,
        },
        %{
          name: "annotations",
          body: {:repetition, {:rule_reference, "annotation", false}},
          line_number: 247,
        },
        %{
          name: "element_value_pairs",
          body: {:sequence, [
            {:rule_reference, "element_value_pair", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "element_value_pair", false},
              ]}},
          ]},
          line_number: 252,
        },
        %{
          name: "element_value_pair",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "element_value", false},
          ]},
          line_number: 254,
        },
        %{
          name: "element_value",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:rule_reference, "element_value_array", false},
            {:rule_reference, "expression", false},
          ]},
          line_number: 273,
        },
        %{
          name: "element_value_array",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "element_value", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "element_value", false},
                  ]}},
              ]}},
            {:optional, {:rule_reference, "COMMA", true}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 283,
        },
        %{
          name: "annotation_type_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "class_modifier", false}},
            {:rule_reference, "AT", true},
            {:literal, "interface"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "annotation_type_body", false},
          ]},
          line_number: 313,
        },
        %{
          name: "annotation_type_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "annotation_type_element_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 316,
        },
        %{
          name: "annotation_type_element_declaration",
          body: {:alternation, [
            {:rule_reference, "annotation_element_declaration", false},
            {:rule_reference, "field_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "enum_declaration", false},
            {:rule_reference, "annotation_type_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 324,
        },
        %{
          name: "annotation_element_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "method_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "RPAREN", true},
            {:optional, {:sequence, [
                {:literal, "default"},
                {:rule_reference, "element_value", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 342,
        },
        %{
          name: "class_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "class_modifier", false}},
            {:literal, "class"},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "type_parameters", false}},
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
          line_number: 377,
        },
        %{
          name: "class_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
            {:literal, "abstract"},
            {:literal, "final"},
            {:literal, "static"},
            {:literal, "strictfp"},
          ]},
          line_number: 390,
        },
        %{
          name: "class_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "class_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 413,
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
            {:rule_reference, "enum_declaration", false},
            {:rule_reference, "annotation_type_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 415,
        },
        %{
          name: "interface_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "interface_modifier", false}},
            {:literal, "interface"},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:optional, {:sequence, [
                {:literal, "extends"},
                {:rule_reference, "interface_type_list", false},
              ]}},
            {:rule_reference, "interface_body", false},
          ]},
          line_number: 455,
        },
        %{
          name: "interface_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
            {:literal, "abstract"},
            {:literal, "static"},
            {:literal, "strictfp"},
          ]},
          line_number: 459,
        },
        %{
          name: "interface_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "interface_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 467,
        },
        %{
          name: "interface_body_declaration",
          body: {:alternation, [
            {:rule_reference, "interface_method_declaration", false},
            {:rule_reference, "interface_field_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "enum_declaration", false},
            {:rule_reference, "annotation_type_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 469,
        },
        %{
          name: "interface_field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 481,
        },
        %{
          name: "interface_method_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "interface_method_modifier", false}},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "result_type", false},
            {:rule_reference, "method_declarator", false},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 487,
        },
        %{
          name: "interface_method_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "abstract"},
          ]},
          line_number: 490,
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
          line_number: 498,
        },
        %{
          name: "enum_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "class_modifier", false}},
            {:literal, "enum"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "implements"},
                {:rule_reference, "interface_type_list", false},
              ]}},
            {:rule_reference, "enum_body", false},
          ]},
          line_number: 555,
        },
        %{
          name: "enum_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "enum_constant_list", false}},
            {:optional, {:rule_reference, "COMMA", true}},
            {:optional, {:sequence, [
                {:rule_reference, "SEMICOLON", true},
                {:repetition, {:rule_reference, "class_body_declaration", false}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 572,
        },
        %{
          name: "enum_constant_list",
          body: {:sequence, [
            {:rule_reference, "enum_constant", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "enum_constant", false},
              ]}},
          ]},
          line_number: 574,
        },
        %{
          name: "enum_constant",
          body: {:sequence, [
            {:rule_reference, "annotations", false},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "argument_list", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:optional, {:rule_reference, "class_body", false}},
          ]},
          line_number: 588,
        },
        %{
          name: "type_parameters",
          body: {:sequence, [
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "type_parameter", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "type_parameter", false},
              ]}},
            {:rule_reference, "GREATER_THAN", true},
          ]},
          line_number: 672,
        },
        %{
          name: "type_parameter",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "extends"},
                {:rule_reference, "bound", false},
              ]}},
          ]},
          line_number: 674,
        },
        %{
          name: "bound",
          body: {:sequence, [
            {:rule_reference, "class_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "AMPERSAND", true},
                {:rule_reference, "class_type", false},
              ]}},
          ]},
          line_number: 691,
        },
        %{
          name: "type_arguments",
          body: {:sequence, [
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "type_argument", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "type_argument", false},
              ]}},
            {:rule_reference, "GREATER_THAN", true},
          ]},
          line_number: 721,
        },
        %{
          name: "type_argument",
          body: {:alternation, [
            {:rule_reference, "type", false},
            {:sequence, [
              {:rule_reference, "QUESTION", true},
              {:optional, {:sequence, [
                  {:group, {:alternation, [
                      {:literal, "extends"},
                      {:literal, "super"},
                    ]}},
                  {:rule_reference, "type", false},
                ]}},
            ]},
          ]},
          line_number: 723,
        },
        %{
          name: "field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 743,
        },
        %{
          name: "field_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
            {:literal, "static"},
            {:literal, "final"},
            {:literal, "transient"},
            {:literal, "volatile"},
          ]},
          line_number: 745,
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
          line_number: 763,
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
          line_number: 765,
        },
        %{
          name: "variable_initializer",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "array_initializer", false},
          ]},
          line_number: 770,
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
          line_number: 780,
        },
        %{
          name: "method_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "method_modifier", false}},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "result_type", false},
            {:rule_reference, "method_declarator", false},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "method_body", false},
          ]},
          line_number: 822,
        },
        %{
          name: "method_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
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
          line_number: 825,
        },
        %{
          name: "result_type",
          body: {:alternation, [
            {:literal, "void"},
            {:rule_reference, "type", false},
          ]},
          line_number: 838,
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
          line_number: 848,
        },
        %{
          name: "formal_parameter_list",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "formal_parameter", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "formal_parameter", false},
                ]}},
            ]},
            {:sequence, [
              {:rule_reference, "formal_parameter", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "formal_parameter", false},
                ]}},
              {:rule_reference, "COMMA", true},
              {:rule_reference, "varargs_parameter", false},
            ]},
            {:rule_reference, "varargs_parameter", false},
          ]},
          line_number: 886,
        },
        %{
          name: "formal_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "RBRACKET", true},
              ]}},
          ]},
          line_number: 890,
        },
        %{
          name: "varargs_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "ELLIPSIS", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 892,
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
          line_number: 899,
        },
        %{
          name: "method_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 904,
        },
        %{
          name: "constructor_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "constructor_modifier", false}},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "formal_parameter_list", false}},
            {:rule_reference, "RPAREN", true},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "constructor_body", false},
          ]},
          line_number: 933,
        },
        %{
          name: "constructor_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
          ]},
          line_number: 937,
        },
        %{
          name: "constructor_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "explicit_constructor_invocation", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 945,
        },
        %{
          name: "explicit_constructor_invocation",
          body: {:alternation, [
            {:sequence, [
              {:optional, {:rule_reference, "type_arguments", false}},
              {:literal, "this"},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:optional, {:rule_reference, "type_arguments", false}},
              {:literal, "super"},
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "argument_list", false}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "SEMICOLON", true},
            ]},
          ]},
          line_number: 955,
        },
        %{
          name: "static_initializer",
          body: {:sequence, [
            {:literal, "static"},
            {:rule_reference, "block", false},
          ]},
          line_number: 988,
        },
        %{
          name: "instance_initializer",
          body: {:rule_reference, "block", false},
          line_number: 990,
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
          line_number: 1026,
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
          line_number: 1039,
        },
        %{
          name: "class_type",
          body: {:sequence, [
            {:rule_reference, "qualified_name", false},
            {:optional, {:rule_reference, "type_arguments", false}},
          ]},
          line_number: 1067,
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
            {:rule_reference, "enhanced_for_statement", false},
            {:rule_reference, "switch_statement", false},
            {:rule_reference, "try_statement", false},
            {:rule_reference, "throw_statement", false},
            {:rule_reference, "return_statement", false},
            {:rule_reference, "break_statement", false},
            {:rule_reference, "continue_statement", false},
            {:rule_reference, "synchronized_statement", false},
            {:rule_reference, "assert_statement", false},
            {:rule_reference, "labelled_statement", false},
          ]},
          line_number: 1092,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 1122,
        },
        %{
          name: "block_statement",
          body: {:alternation, [
            {:rule_reference, "var_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1128,
        },
        %{
          name: "var_declaration",
          body: {:rule_reference, "local_variable_declaration_statement", false},
          line_number: 1143,
        },
        %{
          name: "local_variable_declaration_statement",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1145,
        },
        %{
          name: "empty_statement",
          body: {:rule_reference, "SEMICOLON", true},
          line_number: 1152,
        },
        %{
          name: "expression_statement",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1166,
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
          line_number: 1181,
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
          line_number: 1192,
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
          line_number: 1204,
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
          line_number: 1223,
        },
        %{
          name: "for_init",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:optional, {:literal, "final"}},
              {:rule_reference, "type", false},
              {:rule_reference, "variable_declarators", false},
            ]},
            {:optional, {:rule_reference, "expression_list", false}},
          ]},
          line_number: 1229,
        },
        %{
          name: "for_update",
          body: {:rule_reference, "expression_list", false},
          line_number: 1235,
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
          line_number: 1241,
        },
        %{
          name: "enhanced_for_statement",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "LPAREN", true},
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1278,
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
          line_number: 1303,
        },
        %{
          name: "switch_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "switch_block_statement_group", false}},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 1305,
        },
        %{
          name: "switch_block_statement_group",
          body: {:sequence, [
            {:rule_reference, "switch_label", false},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
          ]},
          line_number: 1307,
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
          line_number: 1309,
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
          line_number: 1332,
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
          line_number: 1335,
        },
        %{
          name: "finally_clause",
          body: {:sequence, [
            {:literal, "finally"},
            {:rule_reference, "block", false},
          ]},
          line_number: 1337,
        },
        %{
          name: "throw_statement",
          body: {:sequence, [
            {:literal, "throw"},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1345,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1352,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1367,
        },
        %{
          name: "continue_statement",
          body: {:sequence, [
            {:literal, "continue"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1369,
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
          line_number: 1381,
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
          line_number: 1401,
        },
        %{
          name: "labelled_statement",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1415,
        },
        %{
          name: "expression",
          body: {:rule_reference, "assignment_expression", false},
          line_number: 1473,
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
          line_number: 1475,
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
          line_number: 1478,
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
          line_number: 1498,
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
          line_number: 1508,
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
          line_number: 1516,
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
          line_number: 1524,
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
          line_number: 1532,
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
          line_number: 1540,
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
          line_number: 1557,
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
          line_number: 1569,
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
          line_number: 1583,
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
          line_number: 1594,
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
          line_number: 1605,
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
          line_number: 1622,
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
          line_number: 1628,
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
          line_number: 1654,
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
          line_number: 1665,
        },
        %{
          name: "primary_expression",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:rule_reference, "primary_suffix", false}},
          ]},
          line_number: 1700,
        },
        %{
          name: "primary_suffix",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "DOT", true},
              {:optional, {:rule_reference, "type_arguments", false}},
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
              {:literal, "super"},
            ]},
            {:sequence, [
              {:rule_reference, "DOT", true},
              {:literal, "new"},
              {:optional, {:rule_reference, "type_arguments", false}},
              {:rule_reference, "NAME", true},
              {:optional, {:rule_reference, "type_arguments", false}},
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
          line_number: 1710,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "literal", false},
            {:literal, "this"},
            {:sequence, [
              {:literal, "super"},
              {:rule_reference, "DOT", true},
              {:optional, {:rule_reference, "type_arguments", false}},
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
              {:optional, {:rule_reference, "type_arguments", false}},
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
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1731,
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
          line_number: 1746,
        },
        %{
          name: "array_creation_type",
          body: {:alternation, [
            {:rule_reference, "primitive_type", false},
            {:rule_reference, "class_type", false},
          ]},
          line_number: 1764,
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
          line_number: 1769,
        },
        %{
          name: "literal",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "CHAR", true},
            {:rule_reference, "STRING", true},
            {:literal, "true"},
            {:literal, "false"},
            {:literal, "null"},
          ]},
          line_number: 1790,
        },
      ],
      version: 1,
    }
  end
  end
