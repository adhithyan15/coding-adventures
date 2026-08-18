defmodule CodingAdventures.JavaParser.Grammar.V14 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: java14.grammar
  # Regenerate with: grammar-tools compile-grammar java14.grammar
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
          line_number: 108,
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
          line_number: 109,
        },
        %{
          name: "compilation_unit",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:rule_reference, "package_declaration", false}},
            {:repetition, {:rule_reference, "import_declaration", false}},
            {:repetition, {:rule_reference, "type_declaration", false}},
          ]},
          line_number: 110,
        },
        %{
          name: "package_declaration",
          body: {:sequence, [
            {:literal, "package"},
            {:rule_reference, "qualified_name", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 126,
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
          line_number: 141,
        },
        %{
          name: "type_declaration",
          body: {:alternation, [
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "interface_declaration", false},
            {:rule_reference, "enum_declaration", false},
            {:rule_reference, "annotation_type_declaration", false},
            {:rule_reference, "record_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 164,
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
          line_number: 183,
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
          line_number: 206,
        },
        %{
          name: "annotations",
          body: {:repetition, {:rule_reference, "annotation", false}},
          line_number: 208,
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
          line_number: 210,
        },
        %{
          name: "element_value_pair",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "element_value", false},
          ]},
          line_number: 212,
        },
        %{
          name: "element_value",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:rule_reference, "element_value_array", false},
            {:rule_reference, "expression", false},
          ]},
          line_number: 220,
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
          line_number: 224,
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
          line_number: 240,
        },
        %{
          name: "annotation_type_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "annotation_type_element_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 243,
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
          line_number: 245,
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
          line_number: 253,
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
          line_number: 274,
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
          line_number: 279,
        },
        %{
          name: "class_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "class_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 288,
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
            {:rule_reference, "record_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 294,
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
          line_number: 340,
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
          line_number: 344,
        },
        %{
          name: "interface_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "interface_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 352,
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
            {:rule_reference, "record_declaration", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 357,
        },
        %{
          name: "interface_field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 366,
        },
        %{
          name: "interface_method_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "interface_method_modifier", false}},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "result_type", false},
            {:rule_reference, "method_declarator", false},
            {:optional, {:rule_reference, "throws_clause", false}},
            {:rule_reference, "method_body", false},
          ]},
          line_number: 381,
        },
        %{
          name: "interface_method_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "private"},
            {:literal, "abstract"},
            {:literal, "default"},
            {:literal, "static"},
            {:literal, "strictfp"},
          ]},
          line_number: 384,
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
          line_number: 392,
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
          line_number: 412,
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
          line_number: 416,
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
          line_number: 418,
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
          line_number: 420,
        },
        %{
          name: "record_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "class_modifier", false}},
            {:literal, "record"},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "type_parameters", false}},
            {:rule_reference, "record_header", false},
            {:optional, {:sequence, [
                {:literal, "implements"},
                {:rule_reference, "interface_type_list", false},
              ]}},
            {:rule_reference, "record_body", false},
          ]},
          line_number: 498,
        },
        %{
          name: "record_header",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "record_component_list", false}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 516,
        },
        %{
          name: "record_component_list",
          body: {:sequence, [
            {:rule_reference, "record_component", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "record_component", false},
              ]}},
          ]},
          line_number: 518,
        },
        %{
          name: "record_component",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 520,
        },
        %{
          name: "record_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "record_body_declaration", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 543,
        },
        %{
          name: "record_body_declaration",
          body: {:alternation, [
            {:rule_reference, "compact_constructor", false},
            {:rule_reference, "class_body_declaration", false},
          ]},
          line_number: 545,
        },
        %{
          name: "compact_constructor",
          body: {:sequence, [
            {:repetition, {:rule_reference, "constructor_modifier", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "constructor_body", false},
          ]},
          line_number: 561,
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
          line_number: 587,
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
          line_number: 589,
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
          line_number: 591,
        },
        %{
          name: "type_arguments",
          body: {:sequence, [
            {:rule_reference, "LESS_THAN", true},
            {:optional, {:sequence, [
                {:rule_reference, "type_argument", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "type_argument", false},
                  ]}},
              ]}},
            {:rule_reference, "GREATER_THAN", true},
          ]},
          line_number: 602,
        },
        %{
          name: "type_argument",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:rule_reference, "type", false},
            ]},
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
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
          line_number: 604,
        },
        %{
          name: "field_declaration",
          body: {:sequence, [
            {:repetition, {:rule_reference, "field_modifier", false}},
            {:rule_reference, "type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 622,
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
          line_number: 624,
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
          line_number: 633,
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
          line_number: 635,
        },
        %{
          name: "variable_initializer",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "array_initializer", false},
          ]},
          line_number: 637,
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
          line_number: 640,
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
          line_number: 659,
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
          line_number: 662,
        },
        %{
          name: "result_type",
          body: {:alternation, [
            {:literal, "void"},
            {:rule_reference, "type", false},
          ]},
          line_number: 673,
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
          line_number: 676,
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
          line_number: 693,
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
          line_number: 697,
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
          line_number: 699,
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
          line_number: 701,
        },
        %{
          name: "method_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 703,
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
          line_number: 720,
        },
        %{
          name: "constructor_modifier",
          body: {:alternation, [
            {:rule_reference, "annotation", false},
            {:literal, "public"},
            {:literal, "protected"},
            {:literal, "private"},
          ]},
          line_number: 724,
        },
        %{
          name: "constructor_body",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "explicit_constructor_invocation", false}},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 729,
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
          line_number: 737,
        },
        %{
          name: "static_initializer",
          body: {:sequence, [
            {:literal, "static"},
            {:rule_reference, "block", false},
          ]},
          line_number: 749,
        },
        %{
          name: "instance_initializer",
          body: {:rule_reference, "block", false},
          line_number: 751,
        },
        %{
          name: "type",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:rule_reference, "primitive_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
            ]},
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:rule_reference, "class_type", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
            ]},
          ]},
          line_number: 777,
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
          line_number: 780,
        },
        %{
          name: "class_type",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:rule_reference, "qualified_name", false},
            {:optional, {:rule_reference, "type_arguments", false}},
          ]},
          line_number: 794,
        },
        %{
          name: "local_var_type",
          body: {:alternation, [
            {:rule_reference, "type", false},
            {:literal, "var"},
          ]},
          line_number: 812,
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
            {:rule_reference, "yield_statement", false},
            {:rule_reference, "synchronized_statement", false},
            {:rule_reference, "assert_statement", false},
            {:rule_reference, "labelled_statement", false},
          ]},
          line_number: 859,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "block_statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 891,
        },
        %{
          name: "block_statement",
          body: {:alternation, [
            {:rule_reference, "var_declaration", false},
            {:rule_reference, "class_declaration", false},
            {:rule_reference, "record_declaration", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 893,
        },
        %{
          name: "var_declaration",
          body: {:rule_reference, "local_variable_declaration_statement", false},
          line_number: 907,
        },
        %{
          name: "local_variable_declaration_statement",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "local_var_type", false},
            {:rule_reference, "variable_declarators", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 909,
        },
        %{
          name: "empty_statement",
          body: {:rule_reference, "SEMICOLON", true},
          line_number: 914,
        },
        %{
          name: "expression_statement",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 922,
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
          line_number: 929,
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
          line_number: 933,
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
          line_number: 937,
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
          line_number: 950,
        },
        %{
          name: "for_init",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:optional, {:literal, "final"}},
              {:rule_reference, "local_var_type", false},
              {:rule_reference, "variable_declarators", false},
            ]},
            {:optional, {:rule_reference, "expression_list", false}},
          ]},
          line_number: 953,
        },
        %{
          name: "for_update",
          body: {:rule_reference, "expression_list", false},
          line_number: 956,
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
          line_number: 958,
        },
        %{
          name: "enhanced_for_statement",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "LPAREN", true},
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "local_var_type", false},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 970,
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
          line_number: 1060,
        },
        %{
          name: "switch_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:alternation, [
                {:rule_reference, "switch_rule", false},
                {:rule_reference, "switch_block_statement_group", false},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 1074,
        },
        %{
          name: "switch_rule",
          body: {:sequence, [
            {:rule_reference, "switch_label", false},
            {:rule_reference, "ARROW", true},
            {:group, {:alternation, [
                {:sequence, [
                  {:rule_reference, "expression", false},
                  {:rule_reference, "SEMICOLON", true},
                ]},
                {:rule_reference, "block", false},
                {:sequence, [
                  {:literal, "throw"},
                  {:rule_reference, "expression", false},
                  {:rule_reference, "SEMICOLON", true},
                ]},
              ]}},
          ]},
          line_number: 1092,
        },
        %{
          name: "switch_block_statement_group",
          body: {:sequence, [
            {:rule_reference, "switch_label", false},
            {:rule_reference, "COLON", true},
            {:repetition, {:sequence, [
                {:rule_reference, "switch_label", false},
                {:rule_reference, "COLON", true},
              ]}},
            {:repetition, {:rule_reference, "block_statement", false}},
          ]},
          line_number: 1101,
        },
        %{
          name: "switch_label",
          body: {:alternation, [
            {:sequence, [
              {:literal, "case"},
              {:rule_reference, "case_constants", false},
            ]},
            {:literal, "default"},
          ]},
          line_number: 1116,
        },
        %{
          name: "case_constants",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 1125,
        },
        %{
          name: "yield_statement",
          body: {:sequence, [
            {:literal, "yield"},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1165,
        },
        %{
          name: "try_statement",
          body: {:alternation, [
            {:sequence, [
              {:literal, "try"},
              {:rule_reference, "resource_specification", false},
              {:rule_reference, "block", false},
              {:repetition, {:rule_reference, "catch_clause", false}},
              {:optional, {:rule_reference, "finally_clause", false}},
            ]},
            {:sequence, [
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
          ]},
          line_number: 1191,
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
          line_number: 1197,
        },
        %{
          name: "resource",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:optional, {:literal, "final"}},
              {:rule_reference, "local_var_type", false},
              {:rule_reference, "NAME", true},
              {:rule_reference, "EQUALS", true},
              {:rule_reference, "expression", false},
            ]},
            {:rule_reference, "qualified_name", false},
          ]},
          line_number: 1199,
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
          line_number: 1204,
        },
        %{
          name: "catch_formal_parameter",
          body: {:sequence, [
            {:repetition, {:rule_reference, "annotation", false}},
            {:optional, {:literal, "final"}},
            {:rule_reference, "catch_type", false},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1206,
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
          line_number: 1208,
        },
        %{
          name: "finally_clause",
          body: {:sequence, [
            {:literal, "finally"},
            {:rule_reference, "block", false},
          ]},
          line_number: 1210,
        },
        %{
          name: "throw_statement",
          body: {:sequence, [
            {:literal, "throw"},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1214,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1218,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1231,
        },
        %{
          name: "continue_statement",
          body: {:sequence, [
            {:literal, "continue"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 1233,
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
          line_number: 1237,
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
          line_number: 1244,
        },
        %{
          name: "labelled_statement",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 1248,
        },
        %{
          name: "expression",
          body: {:alternation, [
            {:rule_reference, "lambda_expression", false},
            {:rule_reference, "assignment_expression", false},
          ]},
          line_number: 1297,
        },
        %{
          name: "lambda_expression",
          body: {:sequence, [
            {:rule_reference, "lambda_parameters", false},
            {:rule_reference, "ARROW", true},
            {:rule_reference, "lambda_body", false},
          ]},
          line_number: 1300,
        },
        %{
          name: "lambda_parameters",
          body: {:alternation, [
            {:rule_reference, "NAME", true},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:optional, {:rule_reference, "lambda_parameter_list", false}},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 1302,
        },
        %{
          name: "lambda_parameter_list",
          body: {:sequence, [
            {:rule_reference, "lambda_parameter", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "lambda_parameter", false},
              ]}},
          ]},
          line_number: 1305,
        },
        %{
          name: "lambda_parameter",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:optional, {:literal, "final"}},
              {:rule_reference, "type", false},
              {:rule_reference, "NAME", true},
            ]},
            {:sequence, [
              {:repetition, {:rule_reference, "annotation", false}},
              {:optional, {:literal, "final"}},
              {:rule_reference, "NAME", true},
            ]},
          ]},
          line_number: 1307,
        },
        %{
          name: "lambda_body",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 1310,
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
          line_number: 1317,
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
          line_number: 1320,
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
          line_number: 1335,
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
          line_number: 1342,
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
          line_number: 1348,
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
          line_number: 1352,
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
          line_number: 1356,
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
          line_number: 1360,
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
          line_number: 1364,
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
          line_number: 1372,
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
          line_number: 1379,
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
          line_number: 1384,
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
          line_number: 1389,
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
          line_number: 1394,
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
          line_number: 1400,
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
                  {:rule_reference, "AMPERSAND", true},
                  {:rule_reference, "class_type", false},
                ]}},
              {:repetition, {:sequence, [
                  {:rule_reference, "LBRACKET", true},
                  {:rule_reference, "RBRACKET", true},
                ]}},
              {:rule_reference, "RPAREN", true},
              {:rule_reference, "unary_expression_not_plus_minus", false},
            ]},
          ]},
          line_number: 1414,
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
          line_number: 1420,
        },
        %{
          name: "primary_expression",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:rule_reference, "primary_suffix", false}},
          ]},
          line_number: 1442,
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
          line_number: 1448,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "switch_expression", false},
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
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 1466,
        },
        %{
          name: "switch_expression",
          body: {:sequence, [
            {:literal, "switch"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "switch_block", false},
          ]},
          line_number: 1509,
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
          line_number: 1511,
        },
        %{
          name: "array_creation_type",
          body: {:alternation, [
            {:rule_reference, "primitive_type", false},
            {:rule_reference, "class_type", false},
          ]},
          line_number: 1519,
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
          line_number: 1522,
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
          line_number: 1551,
        },
      ],
      version: 1,
    }
  end
  end
