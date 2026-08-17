defmodule CodingAdventures.JavaParser.Grammar.V8 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: java8.grammar
  # Regenerate with: grammar-tools compile-grammar java8.grammar
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
          line_number: 167,
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
          line_number: 168,
        },
        %{
          name: "compilation_unit",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:rule_reference, "package_declaration", false}},
            {:repetition, {:rule_reference, "import_declaration", false}},
            {:repetition, {:rule_reference, "type_declaration", false}},
          ]},
          line_number: 169,
        },
        %{
          name: "package_declaration",
          body: {:sequence, [
            {:literal, "package"},
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 192,
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
          line_number: 211,
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
          line_number: 231,
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
          line_number: 251,
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
          line_number: 288,
        },
        %{
          name: "annotations",
          body: {:repetition, {:rule_reference, "annotation", false}},
          line_number: 293,
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
          line_number: 298,
        },
        %{
          name: "element_value_pair",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "element_value", false},
          ]},
          line_number: 300,
        },
        %{
          name: "element_value",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:rule_reference, "element_value_array", false},
            {:rule_reference, "expression", false},
          ]},
          line_number: 314,
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
          line_number: 320,
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
          line_number: 340,
        },
        %{
          name: "annotation_type_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "annotation_type_element_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 343,
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
          line_number: 345,
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
          line_number: 353,
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
          line_number: 375,
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
          line_number: 380,
        },
        %{
          name: "class_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "class_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 392,
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
          line_number: 394,
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
          line_number: 467,
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
          line_number: 471,
        },
        %{
          name: "interface_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "interface_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 479,
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
          line_number: 488,
        },
        %{
          name: "interface_field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 499,
        },
        %{
          name: "interface_method_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "interface_method_modifier", false}},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "result_type", false},
            {:rule_reference, "method_declarator", false},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:group, {:alternation, [
                {:rule_reference, "block", false},
                {:rule_reference, "SEMICOLON", true},
              ]}},
          ]},
          line_number: 528,
        },
        %{
          name: "interface_method_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "abstract"},
            {:literal, "default"},
            {:literal, "static"},
            {:literal, "strictfp"},
          ]},
          line_number: 532,
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
          line_number: 543,
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
          line_number: 586,
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
          line_number: 590,
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
          line_number: 592,
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
          line_number: 594,
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
          line_number: 649,
        },
        %{
          name: "type_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:literal, "extends"},
                {:rule_reference, "bound", false},
              ]}},
          ]},
          line_number: 654,
        },
        %{
          name: "bound",
          body: {:sequence, [
            {:rule_reference, "annotated_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "AMPERSAND", true},
                {:rule_reference, "annotated_type", false},
              ]}},
          ]},
          line_number: 659,
        },
        %{
          name: "type_arguments",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LESS_THAN", true},
              {:rule_reference, "GREATER_THAN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LESS_THAN", true},
              {:rule_reference, "type_argument", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "type_argument", false},
                ]}},
              {:rule_reference, "GREATER_THAN", true},
            ]},
          ]},
          line_number: 666,
        },
        %{
          name: "type_argument",
          body: {:alternation, [
            {:rule_reference, "annotated_type", false},
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:rule_reference, "QUESTION", true},
              {:optional, {:sequence, [
                  {:group, {:alternation, [
                      {:literal, "extends"},
                      {:literal, "super"},
                    ]}},
                  {:rule_reference, "annotated_type", false},
                ]}},
            ]},
          ]},
          line_number: 672,
        },
        %{
          name: "annotated_type",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:rule_reference, "type", false},
          ]},
          line_number: 717,
        },
        %{
          name: "field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 737,
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
          line_number: 739,
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
          line_number: 753,
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
          line_number: 755,
        },
        %{
          name: "variable_initializer",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "array_initializer", false},
          ]},
          line_number: 757,
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
          line_number: 763,
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
          line_number: 788,
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
          line_number: 791,
        },
        %{
          name: "result_type",
          body: {:alternation, [
            {:literal, "void"},
            {:rule_reference, "type", false},
          ]},
          line_number: 802,
        },
        %{
          name: "method_declarator",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:sequence, [
                {:rule_reference, "receiver_parameter", false},
                {:rule_reference, "COMMA", true},
              ]}},
            {:optional, {:rule_reference, "formal_parameter_list", false}},
            {:rule_reference, "RPAREN", true},
            {:repetition, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "RBRACKET", true},
              ]}},
          ]},
          line_number: 813,
        },
        %{
          name: "receiver_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:rule_reference, "type", false},
            {:optional, {:sequence, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "DOT", true},
              ]}},
            {:literal, "this"},
          ]},
          line_number: 824,
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
          line_number: 841,
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
          line_number: 845,
        },
        %{
          name: "varargs_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:repetition, {:rule_reference, "annotation", false}},
            {:rule_reference, "ELLIPSIS", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 847,
        },
        %{
          name: "throws_clause",
          body: {:sequence, [
            {:literal, "throws"},
            {:rule_reference, "annotated_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "annotated_type", false},
              ]}},
          ]},
          line_number: 854,
        },
        %{
          name: "method_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 858,
        },
        %{
          name: "constructor_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "constructor_modifier", false}},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:sequence, [
                {:rule_reference, "receiver_parameter", false},
                {:rule_reference, "COMMA", true},
              ]}},
            {:optional, {:rule_reference, "formal_parameter_list", false}},
            {:rule_reference, "RPAREN", true},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "constructor_body", false},
          ]},
          line_number: 879,
        },
        %{
          name: "constructor_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
          ]},
          line_number: 883,
        },
        %{
          name: "constructor_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "explicit_constructor_invocation", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 888,
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
          line_number: 895,
        },
        %{
          name: "static_initializer",
          body: {:sequence, [
            {:literal, "static"},
            {:rule_reference, "block", false},
          ]},
          line_number: 924,
        },
        %{
          name: "instance_initializer",
          body: {:rule_reference, "block", false},
          line_number: 926,
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
          line_number: 955,
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
          line_number: 964,
        },
        %{
          name: "class_type",
          body: {:sequence, [
            {:rule_reference, "qualified_name", false},
            {:optional, {:rule_reference, "type_arguments", false}},
          ]},
          line_number: 985,
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
            {:rule_reference, "try_with_resources_statement", false},
            {:rule_reference, "throw_statement", false},
            {:rule_reference, "return_statement", false},
            {:rule_reference, "break_statement", false},
            {:rule_reference, "continue_statement", false},
            {:rule_reference, "synchronized_statement", false},
            {:rule_reference, "assert_statement", false},
            {:rule_reference, "labelled_statement", false},
          ]},
          line_number: 1007,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 1031,
        },
        %{
          name: "block_statement",
          body: {:alternation, [
            {:rule_reference, "var_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1033,
        },
        %{
          name: "var_declaration",
          body: {:rule_reference, "local_variable_declaration_statement", false},
          line_number: 1047,
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
          line_number: 1049,
        },
        %{
          name: "empty_statement",
          body: {:rule_reference, "SEMICOLON", true},
          line_number: 1053,
        },
        %{
          name: "expression_statement",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1060,
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
          line_number: 1066,
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
          line_number: 1070,
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
          line_number: 1074,
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
          line_number: 1082,
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
          line_number: 1085,
        },
        %{
          name: "for_update",
          body: {:rule_reference, "expression_list", false},
          line_number: 1088,
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
          line_number: 1090,
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
          line_number: 1109,
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
          line_number: 1132,
        },
        %{
          name: "switch_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "switch_block_statement_group", false}},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 1134,
        },
        %{
          name: "switch_block_statement_group",
          body: {:sequence, [
            {:rule_reference, "switch_label", false},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
          ]},
          line_number: 1136,
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
          line_number: 1138,
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
          line_number: 1174,
        },
        %{
          name: "catch_clause",
          body: {:sequence, [
            {:literal, "catch"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "catch_formal_parameter", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "block", false},
          ]},
          line_number: 1180,
        },
        %{
          name: "catch_formal_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "catch_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "PIPE", true},
                {:rule_reference, "catch_type", false},
              ]}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1187,
        },
        %{
          name: "catch_type",
          body: {:rule_reference, "class_type", false},
          line_number: 1189,
        },
        %{
          name: "finally_clause",
          body: {:sequence, [
            {:literal, "finally"},
            {:rule_reference, "block", false},
          ]},
          line_number: 1191,
        },
        %{
          name: "try_with_resources_statement",
          body: {:sequence, [
            {:literal, "try"},
            {:rule_reference, "resource_specification", false},
            {:rule_reference, "block", false},
            {:repetition, {:rule_reference, "catch_clause", false}},
            {:optional, {:rule_reference, "finally_clause", false}},
          ]},
          line_number: 1233,
        },
        %{
          name: "resource_specification",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "resource", false},
            {:repetition, {:sequence, [
                {:rule_reference, "SEMICOLON", true},
                {:rule_reference, "resource", false},
              ]}},
            {:optional, {:rule_reference, "SEMICOLON", true}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 1246,
        },
        %{
          name: "resource",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "type", false},
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 1248,
        },
        %{
          name: "throw_statement",
          body: {:sequence, [
            {:literal, "throw"},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1252,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1256,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1269,
        },
        %{
          name: "continue_statement",
          body: {:sequence, [
            {:literal, "continue"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1271,
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
          line_number: 1275,
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
          line_number: 1282,
        },
        %{
          name: "labelled_statement",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1286,
        },
        %{
          name: "expression",
          body: {:rule_reference, "assignment_expression", false},
          line_number: 1402,
        },
        %{
          name: "assignment_expression",
          body: {:alternation, [
            {:rule_reference, "lambda_expression", false},
            {:sequence, [
              {:rule_reference, "conditional_expression", false},
              {:optional, {:sequence, [
                  {:rule_reference, "assignment_operator", false},
                  {:rule_reference, "assignment_expression", false},
                ]}},
            ]},
          ]},
          line_number: 1404,
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
          line_number: 1408,
        },
        %{
          name: "lambda_expression",
          body: {:sequence, [
            {:rule_reference, "lambda_parameters", false},
            {:rule_reference, "ARROW", true},
            {:rule_reference, "lambda_body", false},
          ]},
          line_number: 1506,
        },
        %{
          name: "lambda_parameters",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "formal_parameter_list", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "inferred_parameter_list", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1520,
        },
        %{
          name: "inferred_parameter_list",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 1534,
        },
        %{
          name: "lambda_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "expression", false},
          ]},
          line_number: 1546,
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
          line_number: 1615,
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
          line_number: 1623,
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
          line_number: 1629,
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
          line_number: 1633,
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
          line_number: 1637,
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
          line_number: 1641,
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
          line_number: 1647,
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
                  {:rule_reference, "annotated_type", false},
                ]},
              ]}},
          ]},
          line_number: 1658,
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
          line_number: 1665,
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
          line_number: 1670,
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
          line_number: 1675,
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
          line_number: 1682,
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
          line_number: 1688,
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
              {:rule_reference, "annotated_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "AMPERSAND", true},
                  {:rule_reference, "annotated_type", false},
                ]}},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "unary_expression_not_plus_minus", false},
            ]},
          ]},
          line_number: 1712,
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
          line_number: 1718,
        },
        %{
          name: "primary_expression",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:rule_reference, "primary_suffix", false}},
          ]},
          line_number: 1737,
        },
        %{
          name: "primary_suffix",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "DOUBLE_COLON", true},
              {:optional, {:rule_reference, "type_arguments", false}},
              {:group, {:alternation, [
                  {:rule_reference, "NAME", true},
                  {:literal, "new"},
                ]}},
            ]},
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
          line_number: 1756,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "literal", false},
            {:literal, "this"},
            {:sequence, [
              {:literal, "super"},
              {:rule_reference, "DOUBLE_COLON", true},
              {:optional, {:rule_reference, "type_arguments", false}},
              {:group, {:alternation, [
                  {:rule_reference, "NAME", true},
                  {:literal, "new"},
                ]}},
            ]},
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
              {:rule_reference, "primitive_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "DOT", true},
              {:literal, "class"},
            ]},
            {:sequence, [
              {:rule_reference, "primitive_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "DOUBLE_COLON", true},
              {:literal, "new"},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1789,
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
          line_number: 1804,
        },
        %{
          name: "array_creation_type",
          body: {:alternation, [
            {:rule_reference, "primitive_type", false},
            {:rule_reference, "class_type", false},
          ]},
          line_number: 1814,
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
          line_number: 1817,
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
          line_number: 1837,
        },
      ],
      version: 1,
    }
  end
  end
