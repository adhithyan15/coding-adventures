defmodule CodingAdventures.VerilogParser.Grammar.V1995 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: verilog1995.grammar
  # Regenerate with: grammar-tools compile-grammar verilog1995.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "source_text",
          body: {:repetition, {:rule_reference, "description", false}},
          line_number: 45,
        },
        %{
          name: "description",
          body: {:rule_reference, "module_declaration", false},
          line_number: 47,
        },
        %{
          name: "module_declaration",
          body: {:sequence, [
            {:literal, "module"},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "parameter_port_list", false}},
            {:optional, {:rule_reference, "port_list", false}},
            {:rule_reference, "SEMICOLON", true},
            {:repetition, {:rule_reference, "module_item", false}},
            {:literal, "endmodule"},
          ]},
          line_number: 76,
        },
        %{
          name: "parameter_port_list",
          body: {:sequence, [
            {:rule_reference, "HASH", true},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "parameter_declaration", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "parameter_declaration", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 94,
        },
        %{
          name: "parameter_declaration",
          body: {:sequence, [
            {:literal, "parameter"},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 97,
        },
        %{
          name: "localparam_declaration",
          body: {:sequence, [
            {:literal, "localparam"},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 98,
        },
        %{
          name: "port_list",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "port", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "port", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 118,
        },
        %{
          name: "port",
          body: {:sequence, [
            {:optional, {:rule_reference, "port_direction", false}},
            {:optional, {:rule_reference, "net_type", false}},
            {:optional, {:literal, "signed"}},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 120,
        },
        %{
          name: "port_direction",
          body: {:alternation, [
            {:literal, "input"},
            {:literal, "output"},
            {:literal, "inout"},
          ]},
          line_number: 122,
        },
        %{
          name: "net_type",
          body: {:alternation, [
            {:literal, "wire"},
            {:literal, "reg"},
            {:literal, "tri"},
            {:literal, "supply0"},
            {:literal, "supply1"},
          ]},
          line_number: 123,
        },
        %{
          name: "range",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 125,
        },
        %{
          name: "module_item",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "port_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "net_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "reg_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "integer_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "parameter_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "localparam_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:rule_reference, "continuous_assign", false},
            {:rule_reference, "always_construct", false},
            {:rule_reference, "initial_construct", false},
            {:rule_reference, "module_instantiation", false},
            {:rule_reference, "generate_region", false},
            {:rule_reference, "function_declaration", false},
            {:rule_reference, "task_declaration", false},
          ]},
          line_number: 142,
        },
        %{
          name: "port_declaration",
          body: {:sequence, [
            {:rule_reference, "port_direction", false},
            {:optional, {:rule_reference, "net_type", false}},
            {:optional, {:literal, "signed"}},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "name_list", false},
          ]},
          line_number: 177,
        },
        %{
          name: "net_declaration",
          body: {:sequence, [
            {:rule_reference, "net_type", false},
            {:optional, {:literal, "signed"}},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "name_list", false},
          ]},
          line_number: 179,
        },
        %{
          name: "reg_declaration",
          body: {:sequence, [
            {:literal, "reg"},
            {:optional, {:literal, "signed"}},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "name_list", false},
          ]},
          line_number: 180,
        },
        %{
          name: "integer_declaration",
          body: {:sequence, [
            {:literal, "integer"},
            {:rule_reference, "name_list", false},
          ]},
          line_number: 181,
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
          line_number: 182,
        },
        %{
          name: "continuous_assign",
          body: {:sequence, [
            {:literal, "assign"},
            {:rule_reference, "assignment", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "assignment", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 201,
        },
        %{
          name: "assignment",
          body: {:sequence, [
            {:rule_reference, "lvalue", false},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 202,
        },
        %{
          name: "lvalue",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:optional, {:rule_reference, "range_select", false}},
            ]},
            {:rule_reference, "concatenation", false},
          ]},
          line_number: 206,
        },
        %{
          name: "range_select",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "expression", false},
            {:optional, {:sequence, [
                {:rule_reference, "COLON", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 209,
        },
        %{
          name: "always_construct",
          body: {:sequence, [
            {:literal, "always"},
            {:rule_reference, "AT", true},
            {:rule_reference, "sensitivity_list", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 246,
        },
        %{
          name: "initial_construct",
          body: {:sequence, [
            {:literal, "initial"},
            {:rule_reference, "statement", false},
          ]},
          line_number: 247,
        },
        %{
          name: "sensitivity_list",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "sensitivity_item", false},
              {:repetition, {:sequence, [
                  {:group, {:alternation, [
                      {:literal, "or"},
                      {:rule_reference, "COMMA", true},
                    ]}},
                  {:rule_reference, "sensitivity_item", false},
                ]}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "STAR", true},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 249,
        },
        %{
          name: "sensitivity_item",
          body: {:sequence, [
            {:optional, {:alternation, [
                {:literal, "posedge"},
                {:literal, "negedge"},
              ]}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 253,
        },
        %{
          name: "statement",
          body: {:alternation, [
            {:rule_reference, "block_statement", false},
            {:rule_reference, "if_statement", false},
            {:rule_reference, "case_statement", false},
            {:rule_reference, "for_statement", false},
            {:sequence, [
              {:rule_reference, "blocking_assignment", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "nonblocking_assignment", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "task_call", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 262,
        },
        %{
          name: "block_statement",
          body: {:sequence, [
            {:literal, "begin"},
            {:optional, {:sequence, [
                {:rule_reference, "COLON", true},
                {:rule_reference, "NAME", true},
              ]}},
            {:repetition, {:rule_reference, "statement", false}},
            {:literal, "end"},
          ]},
          line_number: 278,
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
          line_number: 289,
        },
        %{
          name: "case_statement",
          body: {:sequence, [
            {:group, {:alternation, [
                {:literal, "case"},
                {:literal, "casex"},
                {:literal, "casez"},
              ]}},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:repetition, {:rule_reference, "case_item", false}},
            {:literal, "endcase"},
          ]},
          line_number: 304,
        },
        %{
          name: "case_item",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "expression_list", false},
              {:rule_reference, "COLON", true},
              {:rule_reference, "statement", false},
            ]},
            {:sequence, [
              {:literal, "default"},
              {:optional, {:rule_reference, "COLON", true}},
              {:rule_reference, "statement", false},
            ]},
          ]},
          line_number: 309,
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
          line_number: 312,
        },
        %{
          name: "for_statement",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "blocking_assignment", false},
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "blocking_assignment", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "statement", false},
          ]},
          line_number: 316,
        },
        %{
          name: "blocking_assignment",
          body: {:sequence, [
            {:rule_reference, "lvalue", false},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 320,
        },
        %{
          name: "nonblocking_assignment",
          body: {:sequence, [
            {:rule_reference, "lvalue", false},
            {:rule_reference, "LESS_EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 321,
        },
        %{
          name: "task_call",
          body: {:sequence, [
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
          line_number: 324,
        },
        %{
          name: "module_instantiation",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "parameter_value_assignment", false}},
            {:rule_reference, "instance", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "instance", false},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 343,
        },
        %{
          name: "parameter_value_assignment",
          body: {:sequence, [
            {:rule_reference, "HASH", true},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 346,
        },
        %{
          name: "instance",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "port_connections", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 348,
        },
        %{
          name: "port_connections",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "named_port_connection", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "named_port_connection", false},
                ]}},
            ]},
            {:optional, {:sequence, [
                {:rule_reference, "expression", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "expression", false},
                  ]}},
              ]}},
          ]},
          line_number: 350,
        },
        %{
          name: "named_port_connection",
          body: {:sequence, [
            {:rule_reference, "DOT", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "expression", false}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 353,
        },
        %{
          name: "generate_region",
          body: {:sequence, [
            {:literal, "generate"},
            {:repetition, {:rule_reference, "generate_item", false}},
            {:literal, "endgenerate"},
          ]},
          line_number: 380,
        },
        %{
          name: "generate_item",
          body: {:alternation, [
            {:rule_reference, "genvar_declaration", false},
            {:rule_reference, "generate_for", false},
            {:rule_reference, "generate_if", false},
            {:rule_reference, "module_item", false},
          ]},
          line_number: 382,
        },
        %{
          name: "genvar_declaration",
          body: {:sequence, [
            {:literal, "genvar"},
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 387,
        },
        %{
          name: "generate_for",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "genvar_assignment", false},
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "genvar_assignment", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "generate_block", false},
          ]},
          line_number: 389,
        },
        %{
          name: "generate_if",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "generate_block", false},
            {:optional, {:sequence, [
                {:literal, "else"},
                {:rule_reference, "generate_block", false},
              ]}},
          ]},
          line_number: 393,
        },
        %{
          name: "generate_block",
          body: {:alternation, [
            {:sequence, [
              {:literal, "begin"},
              {:optional, {:sequence, [
                  {:rule_reference, "COLON", true},
                  {:rule_reference, "NAME", true},
                ]}},
              {:repetition, {:rule_reference, "generate_item", false}},
              {:literal, "end"},
            ]},
            {:rule_reference, "generate_item", false},
          ]},
          line_number: 396,
        },
        %{
          name: "genvar_assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 399,
        },
        %{
          name: "function_declaration",
          body: {:sequence, [
            {:literal, "function"},
            {:optional, {:rule_reference, "range", false}},
            {:rule_reference, "NAME", true},
            {:rule_reference, "SEMICOLON", true},
            {:repetition, {:rule_reference, "function_item", false}},
            {:rule_reference, "statement", false},
            {:literal, "endfunction"},
          ]},
          line_number: 418,
        },
        %{
          name: "function_item",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "port_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "reg_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "integer_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "parameter_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
          ]},
          line_number: 423,
        },
        %{
          name: "task_declaration",
          body: {:sequence, [
            {:literal, "task"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "SEMICOLON", true},
            {:repetition, {:rule_reference, "task_item", false}},
            {:rule_reference, "statement", false},
            {:literal, "endtask"},
          ]},
          line_number: 428,
        },
        %{
          name: "task_item",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "port_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "reg_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "integer_declaration", false},
              {:rule_reference, "SEMICOLON", true},
            ]},
          ]},
          line_number: 433,
        },
        %{
          name: "expression",
          body: {:rule_reference, "ternary_expr", false},
          line_number: 461,
        },
        %{
          name: "ternary_expr",
          body: {:sequence, [
            {:rule_reference, "or_expr", false},
            {:optional, {:sequence, [
                {:rule_reference, "QUESTION", true},
                {:rule_reference, "expression", false},
                {:rule_reference, "COLON", true},
                {:rule_reference, "ternary_expr", false},
              ]}},
          ]},
          line_number: 467,
        },
        %{
          name: "or_expr",
          body: {:sequence, [
            {:rule_reference, "and_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "LOGIC_OR", true},
                {:rule_reference, "and_expr", false},
              ]}},
          ]},
          line_number: 470,
        },
        %{
          name: "and_expr",
          body: {:sequence, [
            {:rule_reference, "bit_or_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "LOGIC_AND", true},
                {:rule_reference, "bit_or_expr", false},
              ]}},
          ]},
          line_number: 471,
        },
        %{
          name: "bit_or_expr",
          body: {:sequence, [
            {:rule_reference, "bit_xor_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "PIPE", true},
                {:rule_reference, "bit_xor_expr", false},
              ]}},
          ]},
          line_number: 474,
        },
        %{
          name: "bit_xor_expr",
          body: {:sequence, [
            {:rule_reference, "bit_and_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "CARET", true},
                {:rule_reference, "bit_and_expr", false},
              ]}},
          ]},
          line_number: 475,
        },
        %{
          name: "bit_and_expr",
          body: {:sequence, [
            {:rule_reference, "equality_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "AMP", true},
                {:rule_reference, "equality_expr", false},
              ]}},
          ]},
          line_number: 476,
        },
        %{
          name: "equality_expr",
          body: {:sequence, [
            {:rule_reference, "relational_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "EQUALS_EQUALS", true},
                    {:rule_reference, "NOT_EQUALS", true},
                    {:rule_reference, "CASE_EQ", true},
                    {:rule_reference, "CASE_NEQ", true},
                  ]}},
                {:rule_reference, "relational_expr", false},
              ]}},
          ]},
          line_number: 480,
        },
        %{
          name: "relational_expr",
          body: {:sequence, [
            {:rule_reference, "shift_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "LESS_THAN", true},
                    {:rule_reference, "LESS_EQUALS", true},
                    {:rule_reference, "GREATER_THAN", true},
                    {:rule_reference, "GREATER_EQUALS", true},
                  ]}},
                {:rule_reference, "shift_expr", false},
              ]}},
          ]},
          line_number: 487,
        },
        %{
          name: "shift_expr",
          body: {:sequence, [
            {:rule_reference, "additive_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "LEFT_SHIFT", true},
                    {:rule_reference, "RIGHT_SHIFT", true},
                    {:rule_reference, "ARITH_LEFT_SHIFT", true},
                    {:rule_reference, "ARITH_RIGHT_SHIFT", true},
                  ]}},
                {:rule_reference, "additive_expr", false},
              ]}},
          ]},
          line_number: 492,
        },
        %{
          name: "additive_expr",
          body: {:sequence, [
            {:rule_reference, "multiplicative_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "multiplicative_expr", false},
              ]}},
          ]},
          line_number: 497,
        },
        %{
          name: "multiplicative_expr",
          body: {:sequence, [
            {:rule_reference, "power_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                    {:rule_reference, "PERCENT", true},
                  ]}},
                {:rule_reference, "power_expr", false},
              ]}},
          ]},
          line_number: 498,
        },
        %{
          name: "power_expr",
          body: {:sequence, [
            {:rule_reference, "unary_expr", false},
            {:optional, {:sequence, [
                {:rule_reference, "POWER", true},
                {:rule_reference, "unary_expr", false},
              ]}},
          ]},
          line_number: 499,
        },
        %{
          name: "unary_expr",
          body: {:alternation, [
            {:sequence, [
              {:group, {:alternation, [
                  {:rule_reference, "PLUS", true},
                  {:rule_reference, "MINUS", true},
                  {:rule_reference, "BANG", true},
                  {:rule_reference, "TILDE", true},
                  {:rule_reference, "AMP", true},
                  {:rule_reference, "PIPE", true},
                  {:rule_reference, "CARET", true},
                  {:sequence, [
                    {:rule_reference, "TILDE", true},
                    {:rule_reference, "AMP", true},
                  ]},
                  {:sequence, [
                    {:rule_reference, "TILDE", true},
                    {:rule_reference, "PIPE", true},
                  ]},
                  {:sequence, [
                    {:rule_reference, "TILDE", true},
                    {:rule_reference, "CARET", true},
                  ]},
                ]}},
              {:rule_reference, "unary_expr", false},
            ]},
            {:rule_reference, "primary", false},
          ]},
          line_number: 511,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "SIZED_NUMBER", true},
            {:rule_reference, "REAL_NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "SYSTEM_ID", true},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "concatenation", false},
            {:rule_reference, "replication", false},
            {:sequence, [
              {:rule_reference, "primary", false},
              {:rule_reference, "LBRACKET", true},
              {:rule_reference, "expression", false},
              {:optional, {:sequence, [
                  {:rule_reference, "COLON", true},
                  {:rule_reference, "expression", false},
                ]}},
              {:rule_reference, "RBRACKET", true},
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
          ]},
          line_number: 521,
        },
        %{
          name: "concatenation",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 537,
        },
        %{
          name: "replication",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "concatenation", false},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 543,
        },
      ],
      version: 0,
    }
  end
end
