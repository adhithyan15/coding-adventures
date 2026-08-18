defmodule CodingAdventures.JavaParser.Grammar.V7 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: java7.grammar
  # Regenerate with: grammar-tools compile-grammar java7.grammar
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
          line_number: 191,
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
          line_number: 206,
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
          line_number: 224,
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
          line_number: 241,
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
          line_number: 260,
        },
        %{
          name: "annotations",
          body: {:repetition, {:rule_reference, "annotation", false}},
          line_number: 266,
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
          line_number: 271,
        },
        %{
          name: "element_value_pair",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "element_value", false},
          ]},
          line_number: 273,
        },
        %{
          name: "element_value",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:rule_reference, "element_value_array", false},
            {:rule_reference, "expression", false},
          ]},
          line_number: 287,
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
          line_number: 296,
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
          line_number: 315,
        },
        %{
          name: "annotation_type_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "annotation_type_element_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 318,
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
          line_number: 320,
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
          line_number: 328,
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
          line_number: 349,
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
          line_number: 357,
        },
        %{
          name: "class_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "class_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 371,
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
          line_number: 373,
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
          line_number: 397,
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
          line_number: 401,
        },
        %{
          name: "interface_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "interface_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 409,
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
          line_number: 411,
        },
        %{
          name: "interface_field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 421,
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
          line_number: 425,
        },
        %{
          name: "interface_method_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "abstract"},
          ]},
          line_number: 428,
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
          line_number: 435,
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
          line_number: 462,
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
          line_number: 469,
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
          line_number: 471,
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
          line_number: 476,
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
          line_number: 500,
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
          line_number: 502,
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
          line_number: 508,
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
          line_number: 538,
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
          line_number: 540,
        },
        %{
          name: "type_arguments_or_diamond",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LESS_THAN", true},
              {:rule_reference, "GREATER_THAN", true},
            ]},
            {:rule_reference, "type_arguments", false},
          ]},
          line_number: 554,
        },
        %{
          name: "field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 570,
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
          line_number: 572,
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
          line_number: 586,
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
          line_number: 588,
        },
        %{
          name: "variable_initializer",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "array_initializer", false},
          ]},
          line_number: 592,
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
          line_number: 600,
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
          line_number: 617,
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
          line_number: 620,
        },
        %{
          name: "result_type",
          body: {:alternation, [
            {:literal, "void"},
            {:rule_reference, "type", false},
          ]},
          line_number: 633,
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
          line_number: 639,
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
          line_number: 656,
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
          line_number: 660,
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
          line_number: 662,
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
          line_number: 666,
        },
        %{
          name: "method_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 670,
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
          line_number: 686,
        },
        %{
          name: "constructor_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
          ]},
          line_number: 690,
        },
        %{
          name: "constructor_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "explicit_constructor_invocation", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 695,
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
          line_number: 699,
        },
        %{
          name: "static_initializer",
          body: {:sequence, [
            {:literal, "static"},
            {:rule_reference, "block", false},
          ]},
          line_number: 719,
        },
        %{
          name: "instance_initializer",
          body: {:rule_reference, "block", false},
          line_number: 721,
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
          line_number: 742,
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
          line_number: 748,
        },
        %{
          name: "class_type",
          body: {:sequence, [
            {:rule_reference, "qualified_name", false},
            {:optional, {:rule_reference, "type_arguments", false}},
          ]},
          line_number: 765,
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
            {:rule_reference, "try_with_resources_statement", false},
            {:rule_reference, "try_statement", false},
            {:rule_reference, "throw_statement", false},
            {:rule_reference, "return_statement", false},
            {:rule_reference, "break_statement", false},
            {:rule_reference, "continue_statement", false},
            {:rule_reference, "synchronized_statement", false},
            {:rule_reference, "assert_statement", false},
            {:rule_reference, "labelled_statement", false},
          ]},
          line_number: 786,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 810,
        },
        %{
          name: "block_statement",
          body: {:alternation, [
            {:rule_reference, "var_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 814,
        },
        %{
          name: "var_declaration",
          body: {:rule_reference, "local_variable_declaration_statement", false},
          line_number: 828,
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
          line_number: 830,
        },
        %{
          name: "empty_statement",
          body: {:rule_reference, "SEMICOLON", true},
          line_number: 836,
        },
        %{
          name: "expression_statement",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 843,
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
          line_number: 850,
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
          line_number: 854,
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
          line_number: 858,
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
          line_number: 870,
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
          line_number: 873,
        },
        %{
          name: "for_update",
          body: {:rule_reference, "expression_list", false},
          line_number: 876,
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
          line_number: 878,
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
          line_number: 890,
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
          line_number: 913,
        },
        %{
          name: "switch_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "switch_block_statement_group", false}},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 915,
        },
        %{
          name: "switch_block_statement_group",
          body: {:sequence, [
            {:rule_reference, "switch_label", false},
            {:repetition, {:rule_reference, "switch_label", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
          ]},
          line_number: 917,
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
          line_number: 919,
        },
        %{
          name: "try_with_resources_statement",
          body: {:sequence, [
            {:literal, "try"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "resource_list", false},
            {:optional, {:rule_reference, "SEMICOLON", true}},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "block", false},
            {:repetition, {:rule_reference, "catch_clause", false}},
            {:optional, {:rule_reference, "finally_clause", false}},
          ]},
          line_number: 998,
        },
        %{
          name: "resource_list",
          body: {:sequence, [
            {:rule_reference, "resource", false},
            {:repetition, {:sequence, [
                {:rule_reference, "SEMICOLON", true},
                {:rule_reference, "resource", false},
              ]}},
          ]},
          line_number: 1015,
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
          line_number: 1030,
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
          line_number: 1078,
        },
        %{
          name: "catch_clause",
          body: {:sequence, [
            {:literal, "catch"},
            {:rule_reference, "LPAREN", true},
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "catch_type", false},
            {:rule_reference, "NAME", true},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "block", false},
          ]},
          line_number: 1099,
        },
        %{
          name: "catch_type",
          body: {:sequence, [
            {:rule_reference, "class_type", false},
            {:repetition, {:sequence, [
                {:rule_reference, "PIPE", true},
                {:rule_reference, "class_type", false},
              ]}},
          ]},
          line_number: 1111,
        },
        %{
          name: "finally_clause",
          body: {:sequence, [
            {:literal, "finally"},
            {:rule_reference, "block", false},
          ]},
          line_number: 1113,
        },
        %{
          name: "throw_statement",
          body: {:sequence, [
            {:literal, "throw"},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1129,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1133,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1146,
        },
        %{
          name: "continue_statement",
          body: {:sequence, [
            {:literal, "continue"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1148,
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
          line_number: 1152,
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
          line_number: 1159,
        },
        %{
          name: "labelled_statement",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1165,
        },
        %{
          name: "expression",
          body: {:rule_reference, "assignment_expression", false},
          line_number: 1212,
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
          line_number: 1214,
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
          line_number: 1217,
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
          line_number: 1234,
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
          line_number: 1243,
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
          line_number: 1251,
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
          line_number: 1257,
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
          line_number: 1263,
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
          line_number: 1269,
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
          line_number: 1279,
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
          line_number: 1287,
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
          line_number: 1302,
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
          line_number: 1310,
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
          line_number: 1318,
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
          line_number: 1325,
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
          line_number: 1331,
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
          line_number: 1342,
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
          line_number: 1351,
        },
        %{
          name: "primary_expression",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:rule_reference, "primary_suffix", false}},
          ]},
          line_number: 1380,
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
              {:optional, {:rule_reference, "type_arguments_or_diamond", false}},
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
          line_number: 1384,
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
              {:rule_reference, "class_type_for_new", false},
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
          line_number: 1408,
        },
        %{
          name: "class_type_for_new",
          body: {:sequence, [
            {:rule_reference, "qualified_name", false},
            {:optional, {:rule_reference, "type_arguments_or_diamond", false}},
          ]},
          line_number: 1430,
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
          line_number: 1434,
        },
        %{
          name: "array_creation_type",
          body: {:alternation, [
            {:rule_reference, "primitive_type", false},
            {:rule_reference, "class_type", false},
          ]},
          line_number: 1447,
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
          line_number: 1450,
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
          line_number: 1473,
        },
      ],
      version: 1,
    }
  end
  end
