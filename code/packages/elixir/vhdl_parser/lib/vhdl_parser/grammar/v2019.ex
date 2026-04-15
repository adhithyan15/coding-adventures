defmodule CodingAdventures.VhdlParser.Grammar.V2019 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: vhdl2019.grammar
  # Regenerate with: grammar-tools compile-grammar vhdl2019.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "design_file",
          body: {:repetition, {:rule_reference, "design_unit", false}},
          line_number: 67,
        },
        %{
          name: "design_unit",
          body: {:sequence, [
            {:repetition, {:rule_reference, "context_item", false}},
            {:rule_reference, "library_unit", false},
          ]},
          line_number: 69,
        },
        %{
          name: "context_item",
          body: {:alternation, [
            {:rule_reference, "library_clause", false},
            {:rule_reference, "use_clause", false},
          ]},
          line_number: 71,
        },
        %{
          name: "library_clause",
          body: {:sequence, [
            {:literal, "library"},
            {:rule_reference, "name_list", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 74,
        },
        %{
          name: "use_clause",
          body: {:sequence, [
            {:literal, "use"},
            {:rule_reference, "selected_name", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 77,
        },
        %{
          name: "selected_name",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "DOT", true},
                {:group, {:alternation, [
                    {:rule_reference, "NAME", true},
                    {:literal, "all"},
                  ]}},
              ]}},
          ]},
          line_number: 80,
        },
        %{
          name: "name_list",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 82,
        },
        %{
          name: "library_unit",
          body: {:alternation, [
            {:rule_reference, "entity_declaration", false},
            {:rule_reference, "architecture_body", false},
            {:rule_reference, "package_declaration", false},
            {:rule_reference, "package_body", false},
          ]},
          line_number: 84,
        },
        %{
          name: "entity_declaration",
          body: {:sequence, [
            {:literal, "entity"},
            {:rule_reference, "NAME", true},
            {:literal, "is"},
            {:optional, {:rule_reference, "generic_clause", false}},
            {:optional, {:rule_reference, "port_clause", false}},
            {:literal, "end"},
            {:optional, {:literal, "entity"}},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 115,
        },
        %{
          name: "generic_clause",
          body: {:sequence, [
            {:literal, "generic"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "interface_list", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 120,
        },
        %{
          name: "port_clause",
          body: {:sequence, [
            {:literal, "port"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "interface_list", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 121,
        },
        %{
          name: "interface_list",
          body: {:sequence, [
            {:rule_reference, "interface_element", false},
            {:repetition, {:sequence, [
                {:rule_reference, "SEMICOLON", true},
                {:rule_reference, "interface_element", false},
              ]}},
          ]},
          line_number: 126,
        },
        %{
          name: "interface_element",
          body: {:sequence, [
            {:rule_reference, "name_list", false},
            {:rule_reference, "COLON", true},
            {:optional, {:rule_reference, "mode", false}},
            {:rule_reference, "subtype_indication", false},
            {:optional, {:sequence, [
                {:rule_reference, "VAR_ASSIGN", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 127,
        },
        %{
          name: "mode",
          body: {:alternation, [
            {:literal, "in"},
            {:literal, "out"},
            {:literal, "inout"},
            {:literal, "buffer"},
          ]},
          line_number: 135,
        },
        %{
          name: "architecture_body",
          body: {:sequence, [
            {:literal, "architecture"},
            {:rule_reference, "NAME", true},
            {:literal, "of"},
            {:rule_reference, "NAME", true},
            {:literal, "is"},
            {:repetition, {:rule_reference, "block_declarative_item", false}},
            {:literal, "begin"},
            {:repetition, {:rule_reference, "concurrent_statement", false}},
            {:literal, "end"},
            {:optional, {:literal, "architecture"}},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 157,
        },
        %{
          name: "block_declarative_item",
          body: {:alternation, [
            {:rule_reference, "signal_declaration", false},
            {:rule_reference, "constant_declaration", false},
            {:rule_reference, "type_declaration", false},
            {:rule_reference, "subtype_declaration", false},
            {:rule_reference, "component_declaration", false},
            {:rule_reference, "function_declaration", false},
            {:rule_reference, "function_body", false},
            {:rule_reference, "procedure_declaration", false},
            {:rule_reference, "procedure_body", false},
          ]},
          line_number: 163,
        },
        %{
          name: "signal_declaration",
          body: {:sequence, [
            {:literal, "signal"},
            {:rule_reference, "name_list", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "subtype_indication", false},
            {:optional, {:sequence, [
                {:rule_reference, "VAR_ASSIGN", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 192,
        },
        %{
          name: "constant_declaration",
          body: {:sequence, [
            {:literal, "constant"},
            {:rule_reference, "name_list", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "subtype_indication", false},
            {:rule_reference, "VAR_ASSIGN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 194,
        },
        %{
          name: "variable_declaration",
          body: {:sequence, [
            {:literal, "variable"},
            {:rule_reference, "name_list", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "subtype_indication", false},
            {:optional, {:sequence, [
                {:rule_reference, "VAR_ASSIGN", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 196,
        },
        %{
          name: "type_declaration",
          body: {:sequence, [
            {:literal, "type"},
            {:rule_reference, "NAME", true},
            {:literal, "is"},
            {:rule_reference, "type_definition", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 221,
        },
        %{
          name: "subtype_declaration",
          body: {:sequence, [
            {:literal, "subtype"},
            {:rule_reference, "NAME", true},
            {:literal, "is"},
            {:rule_reference, "subtype_indication", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 222,
        },
        %{
          name: "type_definition",
          body: {:alternation, [
            {:rule_reference, "enumeration_type", false},
            {:rule_reference, "array_type", false},
            {:rule_reference, "record_type", false},
          ]},
          line_number: 224,
        },
        %{
          name: "enumeration_type",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "CHAR_LITERAL", true},
              ]}},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:group, {:alternation, [
                    {:rule_reference, "NAME", true},
                    {:rule_reference, "CHAR_LITERAL", true},
                  ]}},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 230,
        },
        %{
          name: "array_type",
          body: {:sequence, [
            {:literal, "array"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "index_constraint", false},
            {:rule_reference, "RPAREN", true},
            {:literal, "of"},
            {:rule_reference, "subtype_indication", false},
          ]},
          line_number: 235,
        },
        %{
          name: "index_constraint",
          body: {:sequence, [
            {:rule_reference, "discrete_range", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "discrete_range", false},
              ]}},
          ]},
          line_number: 237,
        },
        %{
          name: "discrete_range",
          body: {:alternation, [
            {:rule_reference, "subtype_indication", false},
            {:sequence, [
              {:rule_reference, "expression", false},
              {:group, {:alternation, [
                  {:literal, "to"},
                  {:literal, "downto"},
                ]}},
              {:rule_reference, "expression", false},
            ]},
          ]},
          line_number: 238,
        },
        %{
          name: "record_type",
          body: {:sequence, [
            {:literal, "record"},
            {:repetition, {:sequence, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "COLON", true},
                {:rule_reference, "subtype_indication", false},
                {:rule_reference, "SEMICOLON", true},
              ]}},
            {:literal, "end"},
            {:literal, "record"},
            {:optional, {:rule_reference, "NAME", true}},
          ]},
          line_number: 242,
        },
        %{
          name: "subtype_indication",
          body: {:sequence, [
            {:rule_reference, "selected_name", false},
            {:optional, {:rule_reference, "constraint", false}},
          ]},
          line_number: 250,
        },
        %{
          name: "constraint",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:group, {:alternation, [
                  {:literal, "to"},
                  {:literal, "downto"},
                ]}},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:literal, "range"},
              {:rule_reference, "expression", false},
              {:group, {:alternation, [
                  {:literal, "to"},
                  {:literal, "downto"},
                ]}},
              {:rule_reference, "expression", false},
            ]},
          ]},
          line_number: 252,
        },
        %{
          name: "concurrent_statement",
          body: {:alternation, [
            {:rule_reference, "process_statement", false},
            {:rule_reference, "signal_assignment_concurrent", false},
            {:rule_reference, "component_instantiation", false},
            {:rule_reference, "generate_statement", false},
          ]},
          line_number: 267,
        },
        %{
          name: "signal_assignment_concurrent",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LESS_EQUALS", true},
            {:rule_reference, "waveform", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 275,
        },
        %{
          name: "waveform",
          body: {:sequence, [
            {:rule_reference, "waveform_element", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "waveform_element", false},
              ]}},
          ]},
          line_number: 277,
        },
        %{
          name: "waveform_element",
          body: {:rule_reference, "expression", false},
          line_number: 278,
        },
        %{
          name: "process_statement",
          body: {:sequence, [
            {:optional, {:sequence, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "COLON", true},
              ]}},
            {:literal, "process"},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "sensitivity_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:optional, {:literal, "is"}},
            {:repetition, {:rule_reference, "process_declarative_item", false}},
            {:literal, "begin"},
            {:repetition, {:rule_reference, "sequential_statement", false}},
            {:literal, "end"},
            {:literal, "process"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 310,
        },
        %{
          name: "sensitivity_list",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 318,
        },
        %{
          name: "process_declarative_item",
          body: {:alternation, [
            {:rule_reference, "variable_declaration", false},
            {:rule_reference, "constant_declaration", false},
            {:rule_reference, "type_declaration", false},
            {:rule_reference, "subtype_declaration", false},
          ]},
          line_number: 320,
        },
        %{
          name: "sequential_statement",
          body: {:alternation, [
            {:rule_reference, "signal_assignment_seq", false},
            {:rule_reference, "variable_assignment", false},
            {:rule_reference, "if_statement", false},
            {:rule_reference, "case_statement", false},
            {:rule_reference, "loop_statement", false},
            {:rule_reference, "return_statement", false},
            {:rule_reference, "null_statement", false},
          ]},
          line_number: 332,
        },
        %{
          name: "signal_assignment_seq",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LESS_EQUALS", true},
            {:rule_reference, "waveform", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 345,
        },
        %{
          name: "variable_assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "VAR_ASSIGN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 349,
        },
        %{
          name: "if_statement",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "expression", false},
            {:literal, "then"},
            {:repetition, {:rule_reference, "sequential_statement", false}},
            {:repetition, {:sequence, [
                {:literal, "elsif"},
                {:rule_reference, "expression", false},
                {:literal, "then"},
                {:repetition, {:rule_reference, "sequential_statement", false}},
              ]}},
            {:optional, {:sequence, [
                {:literal, "else"},
                {:repetition, {:rule_reference, "sequential_statement", false}},
              ]}},
            {:literal, "end"},
            {:literal, "if"},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 359,
        },
        %{
          name: "case_statement",
          body: {:sequence, [
            {:literal, "case"},
            {:rule_reference, "expression", false},
            {:literal, "is"},
            {:repetition, {:sequence, [
                {:literal, "when"},
                {:rule_reference, "choices", false},
                {:rule_reference, "ARROW", true},
                {:repetition, {:rule_reference, "sequential_statement", false}},
              ]}},
            {:literal, "end"},
            {:literal, "case"},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 375,
        },
        %{
          name: "choices",
          body: {:sequence, [
            {:rule_reference, "choice", false},
            {:repetition, {:sequence, [
                {:rule_reference, "PIPE", true},
                {:rule_reference, "choice", false},
              ]}},
          ]},
          line_number: 379,
        },
        %{
          name: "choice",
          body: {:alternation, [
            {:rule_reference, "expression", false},
            {:rule_reference, "discrete_range", false},
            {:literal, "others"},
          ]},
          line_number: 380,
        },
        %{
          name: "loop_statement",
          body: {:sequence, [
            {:optional, {:sequence, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "COLON", true},
              ]}},
            {:optional, {:alternation, [
                {:sequence, [
                  {:literal, "for"},
                  {:rule_reference, "NAME", true},
                  {:literal, "in"},
                  {:rule_reference, "discrete_range", false},
                ]},
                {:sequence, [
                  {:literal, "while"},
                  {:rule_reference, "expression", false},
                ]},
              ]}},
            {:literal, "loop"},
            {:repetition, {:rule_reference, "sequential_statement", false}},
            {:literal, "end"},
            {:literal, "loop"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 394,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 401,
        },
        %{
          name: "null_statement",
          body: {:sequence, [
            {:literal, "null"},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 402,
        },
        %{
          name: "component_declaration",
          body: {:sequence, [
            {:literal, "component"},
            {:rule_reference, "NAME", true},
            {:optional, {:literal, "is"}},
            {:optional, {:rule_reference, "generic_clause", false}},
            {:optional, {:rule_reference, "port_clause", false}},
            {:literal, "end"},
            {:literal, "component"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 428,
        },
        %{
          name: "component_instantiation",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:sequence, [
                  {:literal, "entity"},
                  {:rule_reference, "selected_name", false},
                  {:optional, {:sequence, [
                      {:rule_reference, "LPAREN", true},
                      {:rule_reference, "NAME", true},
                      {:rule_reference, "RPAREN", true},
                    ]}},
                ]},
              ]}},
            {:optional, {:sequence, [
                {:literal, "generic"},
                {:literal, "map"},
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "association_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:optional, {:sequence, [
                {:literal, "port"},
                {:literal, "map"},
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "association_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 433,
        },
        %{
          name: "association_list",
          body: {:sequence, [
            {:rule_reference, "association_element", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "association_element", false},
              ]}},
          ]},
          line_number: 440,
        },
        %{
          name: "association_element",
          body: {:alternation, [
            {:sequence, [
              {:optional, {:sequence, [
                  {:rule_reference, "NAME", true},
                  {:rule_reference, "ARROW", true},
                ]}},
              {:rule_reference, "expression", false},
            ]},
            {:sequence, [
              {:optional, {:sequence, [
                  {:rule_reference, "NAME", true},
                  {:rule_reference, "ARROW", true},
                ]}},
              {:literal, "open"},
            ]},
          ]},
          line_number: 441,
        },
        %{
          name: "generate_statement",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:group, {:alternation, [
                {:rule_reference, "for_generate", false},
                {:rule_reference, "if_generate", false},
              ]}},
          ]},
          line_number: 464,
        },
        %{
          name: "for_generate",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "NAME", true},
            {:literal, "in"},
            {:rule_reference, "discrete_range", false},
            {:literal, "generate"},
            {:repetition, {:rule_reference, "concurrent_statement", false}},
            {:literal, "end"},
            {:literal, "generate"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 466,
        },
        %{
          name: "if_generate",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "expression", false},
            {:literal, "generate"},
            {:repetition, {:rule_reference, "concurrent_statement", false}},
            {:literal, "end"},
            {:literal, "generate"},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 470,
        },
        %{
          name: "package_declaration",
          body: {:sequence, [
            {:literal, "package"},
            {:rule_reference, "NAME", true},
            {:literal, "is"},
            {:repetition, {:rule_reference, "package_declarative_item", false}},
            {:literal, "end"},
            {:optional, {:literal, "package"}},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 491,
        },
        %{
          name: "package_body",
          body: {:sequence, [
            {:literal, "package"},
            {:literal, "body"},
            {:rule_reference, "NAME", true},
            {:literal, "is"},
            {:repetition, {:rule_reference, "package_body_declarative_item", false}},
            {:literal, "end"},
            {:optional, {:sequence, [
                {:literal, "package"},
                {:literal, "body"},
              ]}},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 495,
        },
        %{
          name: "package_declarative_item",
          body: {:alternation, [
            {:rule_reference, "type_declaration", false},
            {:rule_reference, "subtype_declaration", false},
            {:rule_reference, "constant_declaration", false},
            {:rule_reference, "signal_declaration", false},
            {:rule_reference, "component_declaration", false},
            {:rule_reference, "function_declaration", false},
            {:rule_reference, "procedure_declaration", false},
          ]},
          line_number: 499,
        },
        %{
          name: "package_body_declarative_item",
          body: {:alternation, [
            {:rule_reference, "type_declaration", false},
            {:rule_reference, "subtype_declaration", false},
            {:rule_reference, "constant_declaration", false},
            {:rule_reference, "function_body", false},
            {:rule_reference, "procedure_body", false},
          ]},
          line_number: 507,
        },
        %{
          name: "function_declaration",
          body: {:sequence, [
            {:optional, {:alternation, [
                {:literal, "pure"},
                {:literal, "impure"},
              ]}},
            {:literal, "function"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "interface_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:literal, "return"},
            {:rule_reference, "subtype_indication", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 523,
        },
        %{
          name: "function_body",
          body: {:sequence, [
            {:optional, {:alternation, [
                {:literal, "pure"},
                {:literal, "impure"},
              ]}},
            {:literal, "function"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "interface_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:literal, "return"},
            {:rule_reference, "subtype_indication", false},
            {:literal, "is"},
            {:repetition, {:rule_reference, "process_declarative_item", false}},
            {:literal, "begin"},
            {:repetition, {:rule_reference, "sequential_statement", false}},
            {:literal, "end"},
            {:optional, {:literal, "function"}},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 528,
        },
        %{
          name: "procedure_declaration",
          body: {:sequence, [
            {:literal, "procedure"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "interface_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 537,
        },
        %{
          name: "procedure_body",
          body: {:sequence, [
            {:literal, "procedure"},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:rule_reference, "interface_list", false},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:literal, "is"},
            {:repetition, {:rule_reference, "process_declarative_item", false}},
            {:literal, "begin"},
            {:repetition, {:rule_reference, "sequential_statement", false}},
            {:literal, "end"},
            {:optional, {:literal, "procedure"}},
            {:optional, {:rule_reference, "NAME", true}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 540,
        },
        %{
          name: "expression",
          body: {:rule_reference, "logical_expr", false},
          line_number: 577,
        },
        %{
          name: "logical_expr",
          body: {:sequence, [
            {:rule_reference, "relation", false},
            {:optional, {:sequence, [
                {:rule_reference, "logical_op", false},
                {:rule_reference, "relation", false},
              ]}},
          ]},
          line_number: 584,
        },
        %{
          name: "logical_op",
          body: {:alternation, [
            {:literal, "and"},
            {:literal, "or"},
            {:literal, "xor"},
            {:literal, "nand"},
            {:literal, "nor"},
            {:literal, "xnor"},
          ]},
          line_number: 585,
        },
        %{
          name: "relation",
          body: {:sequence, [
            {:rule_reference, "shift_expr", false},
            {:optional, {:sequence, [
                {:rule_reference, "relational_op", false},
                {:rule_reference, "shift_expr", false},
              ]}},
          ]},
          line_number: 589,
        },
        %{
          name: "relational_op",
          body: {:alternation, [
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "NOT_EQUALS", true},
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "LESS_EQUALS", true},
            {:rule_reference, "GREATER_THAN", true},
            {:rule_reference, "GREATER_EQUALS", true},
          ]},
          line_number: 590,
        },
        %{
          name: "shift_expr",
          body: {:sequence, [
            {:rule_reference, "adding_expr", false},
            {:optional, {:sequence, [
                {:rule_reference, "shift_op", false},
                {:rule_reference, "adding_expr", false},
              ]}},
          ]},
          line_number: 595,
        },
        %{
          name: "shift_op",
          body: {:alternation, [
            {:literal, "sll"},
            {:literal, "srl"},
            {:literal, "sla"},
            {:literal, "sra"},
            {:literal, "rol"},
            {:literal, "ror"},
          ]},
          line_number: 596,
        },
        %{
          name: "adding_expr",
          body: {:sequence, [
            {:rule_reference, "multiplying_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "adding_op", false},
                {:rule_reference, "multiplying_expr", false},
              ]}},
          ]},
          line_number: 600,
        },
        %{
          name: "adding_op",
          body: {:alternation, [
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
            {:rule_reference, "AMPERSAND", true},
          ]},
          line_number: 601,
        },
        %{
          name: "multiplying_expr",
          body: {:sequence, [
            {:rule_reference, "unary_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "multiplying_op", false},
                {:rule_reference, "unary_expr", false},
              ]}},
          ]},
          line_number: 604,
        },
        %{
          name: "multiplying_op",
          body: {:alternation, [
            {:rule_reference, "STAR", true},
            {:rule_reference, "SLASH", true},
            {:literal, "mod"},
            {:literal, "rem"},
          ]},
          line_number: 605,
        },
        %{
          name: "unary_expr",
          body: {:alternation, [
            {:sequence, [
              {:literal, "abs"},
              {:rule_reference, "unary_expr", false},
            ]},
            {:sequence, [
              {:literal, "not"},
              {:rule_reference, "unary_expr", false},
            ]},
            {:sequence, [
              {:group, {:alternation, [
                  {:rule_reference, "PLUS", true},
                  {:rule_reference, "MINUS", true},
                ]}},
              {:rule_reference, "unary_expr", false},
            ]},
            {:rule_reference, "power_expr", false},
          ]},
          line_number: 608,
        },
        %{
          name: "power_expr",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:optional, {:sequence, [
                {:rule_reference, "POWER", true},
                {:rule_reference, "primary", false},
              ]}},
          ]},
          line_number: 614,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "REAL_NUMBER", true},
            {:rule_reference, "BASED_LITERAL", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "CHAR_LITERAL", true},
            {:rule_reference, "BIT_STRING", true},
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:optional, {:sequence, [
                  {:rule_reference, "TICK", true},
                  {:rule_reference, "NAME", true},
                ]}},
            ]},
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:rule_reference, "LPAREN", true},
              {:optional, {:sequence, [
                  {:rule_reference, "expression", false},
                  {:repetition, {:sequence, [
                      {:rule_reference, "COMMA", true},
                      {:rule_reference, "expression", false},
                    ]}},
                ]}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "aggregate", false},
            {:literal, "null"},
          ]},
          line_number: 622,
        },
        %{
          name: "aggregate",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "element_association", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "element_association", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 638,
        },
        %{
          name: "element_association",
          body: {:sequence, [
            {:optional, {:sequence, [
                {:rule_reference, "choices", false},
                {:rule_reference, "ARROW", true},
              ]}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 639,
        },
      ],
      version: 0,
    }
  end
end
