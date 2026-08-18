defmodule CodingAdventures.NibParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: nib.grammar
  # Regenerate with: grammar-tools compile-grammar nib.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:rule_reference, "top_decl", false}},
          line_number: 42,
        },
        %{
          name: "top_decl",
          body: {:alternation, [
            {:rule_reference, "const_decl", false},
            {:rule_reference, "static_decl", false},
            {:rule_reference, "fn_decl", false},
          ]},
          line_number: 47,
        },
        %{
          name: "const_decl",
          body: {:sequence, [
            {:literal, "const"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "type", false},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 60,
        },
        %{
          name: "static_decl",
          body: {:sequence, [
            {:literal, "static"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "type", false},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 66,
        },
        %{
          name: "fn_decl",
          body: {:sequence, [
            {:literal, "fn"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "param_list", false}},
            {:rule_reference, "RPAREN", true},
            {:optional, {:sequence, [
                {:rule_reference, "ARROW", true},
                {:rule_reference, "type", false},
              ]}},
            {:rule_reference, "block", false},
          ]},
          line_number: 77,
        },
        %{
          name: "param_list",
          body: {:sequence, [
            {:rule_reference, "param", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "param", false},
              ]}},
          ]},
          line_number: 80,
        },
        %{
          name: "param",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "type", false},
          ]},
          line_number: 87,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:repetition, {:rule_reference, "stmt", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 98,
        },
        %{
          name: "stmt",
          body: {:alternation, [
            {:rule_reference, "let_stmt", false},
            {:rule_reference, "assign_stmt", false},
            {:rule_reference, "return_stmt", false},
            {:rule_reference, "for_stmt", false},
            {:rule_reference, "while_stmt", false},
            {:rule_reference, "if_stmt", false},
            {:rule_reference, "expr_stmt", false},
          ]},
          line_number: 113,
        },
        %{
          name: "let_stmt",
          body: {:sequence, [
            {:literal, "let"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "type", false},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 126,
        },
        %{
          name: "assign_stmt",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 131,
        },
        %{
          name: "return_stmt",
          body: {:sequence, [
            {:literal, "return"},
            {:rule_reference, "expr", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 136,
        },
        %{
          name: "for_stmt",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:rule_reference, "type", false},
            {:literal, "in"},
            {:rule_reference, "expr", false},
            {:rule_reference, "RANGE", true},
            {:rule_reference, "expr", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 159,
        },
        %{
          name: "while_stmt",
          body: {:sequence, [
            {:literal, "while"},
            {:rule_reference, "expr", false},
            {:rule_reference, "block", false},
          ]},
          line_number: 170,
        },
        %{
          name: "if_stmt",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "expr", false},
            {:rule_reference, "block", false},
            {:optional, {:sequence, [
                {:literal, "else"},
                {:rule_reference, "block", false},
              ]}},
          ]},
          line_number: 176,
        },
        %{
          name: "expr_stmt",
          body: {:sequence, [
            {:rule_reference, "expr", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 183,
        },
        %{
          name: "type",
          body: {:alternation, [
            {:literal, "u4"},
            {:literal, "u8"},
            {:literal, "bcd"},
            {:literal, "bool"},
          ]},
          line_number: 218,
        },
        %{
          name: "expr",
          body: {:rule_reference, "or_expr", false},
          line_number: 259,
        },
        %{
          name: "or_expr",
          body: {:sequence, [
            {:rule_reference, "and_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "LOR", true},
                {:rule_reference, "and_expr", false},
              ]}},
          ]},
          line_number: 265,
        },
        %{
          name: "and_expr",
          body: {:sequence, [
            {:rule_reference, "eq_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "LAND", true},
                {:rule_reference, "eq_expr", false},
              ]}},
          ]},
          line_number: 269,
        },
        %{
          name: "eq_expr",
          body: {:sequence, [
            {:rule_reference, "cmp_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "EQ_EQ", true},
                    {:rule_reference, "NEQ", true},
                  ]}},
                {:rule_reference, "cmp_expr", false},
              ]}},
          ]},
          line_number: 274,
        },
        %{
          name: "cmp_expr",
          body: {:sequence, [
            {:rule_reference, "add_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "LT", true},
                    {:rule_reference, "GT", true},
                    {:rule_reference, "LEQ", true},
                    {:rule_reference, "GEQ", true},
                  ]}},
                {:rule_reference, "add_expr", false},
              ]}},
          ]},
          line_number: 280,
        },
        %{
          name: "add_expr",
          body: {:sequence, [
            {:rule_reference, "shift_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                    {:rule_reference, "WRAP_ADD", true},
                    {:rule_reference, "SAT_ADD", true},
                  ]}},
                {:rule_reference, "shift_expr", false},
              ]}},
          ]},
          line_number: 293,
        },
        %{
          name: "shift_expr",
          body: {:sequence, [
            {:rule_reference, "mul_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "SHL", true},
                    {:rule_reference, "SHR", true},
                  ]}},
                {:rule_reference, "mul_expr", false},
              ]}},
          ]},
          line_number: 298,
        },
        %{
          name: "mul_expr",
          body: {:sequence, [
            {:rule_reference, "bitwise_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                    {:rule_reference, "PERCENT", true},
                  ]}},
                {:rule_reference, "bitwise_expr", false},
              ]}},
          ]},
          line_number: 308,
        },
        %{
          name: "bitwise_expr",
          body: {:sequence, [
            {:rule_reference, "unary_expr", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "AMP", true},
                    {:rule_reference, "PIPE", true},
                    {:rule_reference, "CARET", true},
                  ]}},
                {:rule_reference, "unary_expr", false},
              ]}},
          ]},
          line_number: 314,
        },
        %{
          name: "unary_expr",
          body: {:alternation, [
            {:sequence, [
              {:group, {:alternation, [
                  {:rule_reference, "BANG", true},
                  {:rule_reference, "TILDE", true},
                ]}},
              {:rule_reference, "unary_expr", false},
            ]},
            {:rule_reference, "primary", false},
          ]},
          line_number: 322,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "INT_LIT", true},
            {:rule_reference, "HEX_LIT", true},
            {:literal, "true"},
            {:literal, "false"},
            {:rule_reference, "call_expr", false},
            {:rule_reference, "NAME", true},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 330,
        },
        %{
          name: "call_expr",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "arg_list", false}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 353,
        },
        %{
          name: "arg_list",
          body: {:sequence, [
            {:rule_reference, "expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expr", false},
              ]}},
          ]},
          line_number: 356,
        },
      ],
      version: 1,
    }
  end
end
