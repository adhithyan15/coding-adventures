defmodule CodingAdventures.AlgolParser.Grammar.Algol60 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: algol60.grammar
  # Regenerate with: grammar-tools compile-grammar algol60.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.

  alias CodingAdventures.GrammarTools.ParserGrammar

  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:rule_reference, "block", false},
          line_number: 47,
        },
        %{
          name: "block",
          body: {:sequence, [
            {:literal, "begin"},
            {:repetition, {:sequence, [
                {:rule_reference, "declaration", false},
                {:rule_reference, "SEMICOLON", true},
              ]}},
            {:repetition, {:rule_reference, "SEMICOLON", true}},
            {:optional, {:sequence, [
                {:rule_reference, "statement", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "SEMICOLON", true},
                    {:optional, {:rule_reference, "statement", false}},
                  ]}},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 53,
        },
        %{
          name: "declaration",
          body: {:alternation, [
            {:rule_reference, "type_decl", false},
            {:rule_reference, "own_decl", false},
            {:rule_reference, "own_array_decl", false},
            {:rule_reference, "array_decl", false},
            {:rule_reference, "switch_decl", false},
            {:rule_reference, "procedure_decl", false},
          ]},
          line_number: 60,
        },
        %{
          name: "type_decl",
          body: {:sequence, [
            {:rule_reference, "type", false},
            {:rule_reference, "ident_list", false},
          ]},
          line_number: 71,
        },
        %{
          name: "own_decl",
          body: {:sequence, [
            {:literal, "own"},
            {:rule_reference, "type", false},
            {:rule_reference, "ident_list", false},
          ]},
          line_number: 76,
        },
        %{
          name: "own_array_decl",
          body: {:sequence, [
            {:literal, "own"},
            {:optional, {:rule_reference, "type", false}},
            {:literal, "array"},
            {:rule_reference, "array_segment", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "array_segment", false},
              ]}},
          ]},
          line_number: 81,
        },
        %{
          name: "type",
          body: {:alternation, [
            {:literal, "integer"},
            {:literal, "real"},
            {:literal, "boolean"},
            {:literal, "string"},
          ]},
          line_number: 83,
        },
        %{
          name: "ident_list",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 85,
        },
        %{
          name: "array_decl",
          body: {:sequence, [
            {:optional, {:rule_reference, "type", false}},
            {:literal, "array"},
            {:rule_reference, "array_segment", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "array_segment", false},
              ]}},
          ]},
          line_number: 93,
        },
        %{
          name: "array_segment",
          body: {:sequence, [
            {:rule_reference, "ident_list", false},
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "bound_pair", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "bound_pair", false},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 95,
        },
        %{
          name: "bound_pair",
          body: {:sequence, [
            {:rule_reference, "arith_expr", false},
            {:rule_reference, "COLON", true},
            {:rule_reference, "arith_expr", false},
          ]},
          line_number: 99,
        },
        %{
          name: "switch_decl",
          body: {:sequence, [
            {:literal, "switch"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "ASSIGN", true},
            {:rule_reference, "switch_list", false},
          ]},
          line_number: 104,
        },
        %{
          name: "switch_list",
          body: {:sequence, [
            {:rule_reference, "desig_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "desig_expr", false},
              ]}},
          ]},
          line_number: 106,
        },
        %{
          name: "procedure_decl",
          body: {:sequence, [
            {:optional, {:rule_reference, "type", false}},
            {:literal, "procedure"},
            {:rule_reference, "NAME", true},
            {:optional, {:rule_reference, "formal_params", false}},
            {:rule_reference, "SEMICOLON", true},
            {:optional, {:rule_reference, "value_part", false}},
            {:repetition, {:rule_reference, "spec_part", false}},
            {:rule_reference, "proc_body", false},
          ]},
          line_number: 113,
        },
        %{
          name: "formal_params",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "ident_list", false}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 118,
        },
        %{
          name: "value_part",
          body: {:sequence, [
            {:literal, "value"},
            {:rule_reference, "ident_list", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 123,
        },
        %{
          name: "spec_part",
          body: {:sequence, [
            {:rule_reference, "specifier", false},
            {:rule_reference, "ident_list", false},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 130,
        },
        %{
          name: "specifier",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "type", false},
              {:literal, "array"},
            ]},
            {:sequence, [
              {:rule_reference, "type", false},
              {:literal, "procedure"},
            ]},
            {:literal, "array"},
            {:literal, "label"},
            {:literal, "switch"},
            {:literal, "procedure"},
            {:rule_reference, "type", false},
          ]},
          line_number: 132,
        },
        %{
          name: "proc_body",
          body: {:alternation, [
            {:rule_reference, "block", false},
            {:rule_reference, "statement", false},
          ]},
          line_number: 140,
        },
        %{
          name: "statement",
          body: {:alternation, [
            {:sequence, [
              {:repetition, {:sequence, [
                  {:rule_reference, "label", false},
                  {:rule_reference, "COLON", true},
                ]}},
              {:rule_reference, "unlabeled_stmt", false},
            ]},
            {:sequence, [
              {:repetition, {:sequence, [
                  {:rule_reference, "label", false},
                  {:rule_reference, "COLON", true},
                ]}},
              {:rule_reference, "cond_stmt", false},
            ]},
          ]},
          line_number: 152,
        },
        %{
          name: "label",
          body: {:alternation, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "INTEGER_LIT", true},
          ]},
          line_number: 155,
        },
        %{
          name: "unlabeled_stmt",
          body: {:alternation, [
            {:rule_reference, "assign_stmt", false},
            {:rule_reference, "dummy_stmt", false},
            {:rule_reference, "goto_stmt", false},
            {:rule_reference, "proc_stmt", false},
            {:rule_reference, "compound_stmt", false},
            {:rule_reference, "block", false},
            {:rule_reference, "for_stmt", false},
          ]},
          line_number: 165,
        },
        %{
          name: "dummy_stmt",
          body: {:alternation, [
            {:positive_lookahead, {:rule_reference, "SEMICOLON", true}},
            {:positive_lookahead, {:literal, "end"}},
            {:positive_lookahead, {:literal, "else"}},
          ]},
          line_number: 175,
        },
        %{
          name: "cond_stmt",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "bool_expr", false},
            {:literal, "then"},
            {:rule_reference, "unlabeled_stmt", false},
            {:optional, {:sequence, [
                {:literal, "else"},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 181,
        },
        %{
          name: "compound_stmt",
          body: {:sequence, [
            {:literal, "begin"},
            {:repetition, {:rule_reference, "SEMICOLON", true}},
            {:optional, {:sequence, [
                {:rule_reference, "statement", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "SEMICOLON", true},
                    {:optional, {:rule_reference, "statement", false}},
                  ]}},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 185,
        },
        %{
          name: "assign_stmt",
          body: {:sequence, [
            {:rule_reference, "left_part", false},
            {:repetition, {:rule_reference, "left_part", false}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 191,
        },
        %{
          name: "left_part",
          body: {:sequence, [
            {:rule_reference, "variable", false},
            {:rule_reference, "ASSIGN", true},
          ]},
          line_number: 193,
        },
        %{
          name: "goto_stmt",
          body: {:alternation, [
            {:sequence, [
              {:literal, "goto"},
              {:rule_reference, "desig_expr", false},
            ]},
            {:sequence, [
              {:literal, "go"},
              {:literal, "to"},
              {:rule_reference, "desig_expr", false},
            ]},
          ]},
          line_number: 197,
        },
        %{
          name: "proc_stmt",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "actual_params", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
          ]},
          line_number: 202,
        },
        %{
          name: "actual_params",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 204,
        },
        %{
          name: "for_stmt",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "variable", false},
            {:rule_reference, "ASSIGN", true},
            {:rule_reference, "for_list", false},
            {:literal, "do"},
            {:rule_reference, "statement", false},
          ]},
          line_number: 212,
        },
        %{
          name: "for_list",
          body: {:sequence, [
            {:rule_reference, "for_elem", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "for_elem", false},
              ]}},
          ]},
          line_number: 214,
        },
        %{
          name: "for_elem",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "arith_expr", false},
              {:literal, "step"},
              {:rule_reference, "arith_expr", false},
              {:literal, "until"},
              {:rule_reference, "arith_expr", false},
            ]},
            {:sequence, [
              {:rule_reference, "arith_expr", false},
              {:literal, "while"},
              {:rule_reference, "bool_expr", false},
            ]},
            {:rule_reference, "arith_expr", false},
          ]},
          line_number: 218,
        },
        %{
          name: "expression",
          body: {:alternation, [
            {:sequence, [
              {:literal, "if"},
              {:rule_reference, "bool_expr", false},
              {:literal, "then"},
              {:rule_reference, "expression", false},
              {:literal, "else"},
              {:rule_reference, "expression", false},
            ]},
            {:rule_reference, "expr_eqv", false},
          ]},
          line_number: 250,
        },
        %{
          name: "expr_eqv",
          body: {:sequence, [
            {:rule_reference, "expr_impl", false},
            {:repetition, {:sequence, [
                {:literal, "eqv"},
                {:rule_reference, "expr_impl", false},
              ]}},
          ]},
          line_number: 253,
        },
        %{
          name: "expr_impl",
          body: {:sequence, [
            {:rule_reference, "expr_or", false},
            {:repetition, {:sequence, [
                {:literal, "impl"},
                {:rule_reference, "expr_or", false},
              ]}},
          ]},
          line_number: 254,
        },
        %{
          name: "expr_or",
          body: {:sequence, [
            {:rule_reference, "expr_and", false},
            {:repetition, {:sequence, [
                {:literal, "or"},
                {:rule_reference, "expr_and", false},
              ]}},
          ]},
          line_number: 255,
        },
        %{
          name: "expr_and",
          body: {:sequence, [
            {:rule_reference, "expr_not", false},
            {:repetition, {:sequence, [
                {:literal, "and"},
                {:rule_reference, "expr_not", false},
              ]}},
          ]},
          line_number: 256,
        },
        %{
          name: "expr_not",
          body: {:alternation, [
            {:sequence, [
              {:literal, "not"},
              {:rule_reference, "expr_not", false},
            ]},
            {:rule_reference, "expr_cmp", false},
          ]},
          line_number: 257,
        },
        %{
          name: "expr_cmp",
          body: {:sequence, [
            {:rule_reference, "expr_add", false},
            {:optional, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "EQ", true},
                    {:rule_reference, "NEQ", true},
                    {:rule_reference, "LT", true},
                    {:rule_reference, "LEQ", true},
                    {:rule_reference, "GT", true},
                    {:rule_reference, "GEQ", true},
                  ]}},
                {:rule_reference, "expr_add", false},
              ]}},
          ]},
          line_number: 258,
        },
        %{
          name: "expr_add",
          body: {:sequence, [
            {:optional, {:alternation, [
                {:rule_reference, "PLUS", true},
                {:rule_reference, "MINUS", true},
              ]}},
            {:rule_reference, "expr_mul", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "expr_mul", false},
              ]}},
          ]},
          line_number: 259,
        },
        %{
          name: "expr_mul",
          body: {:sequence, [
            {:rule_reference, "expr_pow", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                    {:literal, "div"},
                    {:literal, "mod"},
                  ]}},
                {:rule_reference, "expr_pow", false},
              ]}},
          ]},
          line_number: 260,
        },
        %{
          name: "expr_pow",
          body: {:sequence, [
            {:rule_reference, "expr_atom", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "CARET", true},
                    {:rule_reference, "POWER", true},
                  ]}},
                {:rule_reference, "expr_atom", false},
              ]}},
          ]},
          line_number: 261,
        },
        %{
          name: "expr_atom",
          body: {:alternation, [
            {:rule_reference, "INTEGER_LIT", true},
            {:rule_reference, "REAL_LIT", true},
            {:rule_reference, "STRING_LIT", true},
            {:literal, "true"},
            {:literal, "false"},
            {:rule_reference, "proc_call", false},
            {:rule_reference, "variable", false},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expression", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 262,
        },
        %{
          name: "arith_expr",
          body: {:alternation, [
            {:sequence, [
              {:literal, "if"},
              {:rule_reference, "bool_expr", false},
              {:literal, "then"},
              {:rule_reference, "arith_expr", false},
              {:literal, "else"},
              {:rule_reference, "arith_expr", false},
            ]},
            {:rule_reference, "simple_arith", false},
          ]},
          line_number: 274,
        },
        %{
          name: "simple_arith",
          body: {:sequence, [
            {:optional, {:alternation, [
                {:rule_reference, "PLUS", true},
                {:rule_reference, "MINUS", true},
              ]}},
            {:rule_reference, "term", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "term", false},
              ]}},
          ]},
          line_number: 278,
        },
        %{
          name: "term",
          body: {:sequence, [
            {:rule_reference, "factor", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                    {:literal, "div"},
                    {:literal, "mod"},
                  ]}},
                {:rule_reference, "factor", false},
              ]}},
          ]},
          line_number: 283,
        },
        %{
          name: "factor",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "CARET", true},
                    {:rule_reference, "POWER", true},
                  ]}},
                {:rule_reference, "primary", false},
              ]}},
          ]},
          line_number: 289,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "INTEGER_LIT", true},
            {:rule_reference, "REAL_LIT", true},
            {:rule_reference, "STRING_LIT", true},
            {:literal, "true"},
            {:literal, "false"},
            {:rule_reference, "proc_call", false},
            {:rule_reference, "variable", false},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "arith_expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 291,
        },
        %{
          name: "bool_expr",
          body: {:alternation, [
            {:sequence, [
              {:literal, "if"},
              {:rule_reference, "bool_expr", false},
              {:literal, "then"},
              {:rule_reference, "bool_expr", false},
              {:literal, "else"},
              {:rule_reference, "bool_expr", false},
            ]},
            {:rule_reference, "simple_bool", false},
          ]},
          line_number: 309,
        },
        %{
          name: "simple_bool",
          body: {:sequence, [
            {:rule_reference, "implication", false},
            {:repetition, {:sequence, [
                {:literal, "eqv"},
                {:rule_reference, "implication", false},
              ]}},
          ]},
          line_number: 312,
        },
        %{
          name: "implication",
          body: {:sequence, [
            {:rule_reference, "bool_term", false},
            {:repetition, {:sequence, [
                {:literal, "impl"},
                {:rule_reference, "bool_term", false},
              ]}},
          ]},
          line_number: 314,
        },
        %{
          name: "bool_term",
          body: {:sequence, [
            {:rule_reference, "bool_factor", false},
            {:repetition, {:sequence, [
                {:literal, "or"},
                {:rule_reference, "bool_factor", false},
              ]}},
          ]},
          line_number: 316,
        },
        %{
          name: "bool_factor",
          body: {:sequence, [
            {:rule_reference, "bool_secondary", false},
            {:repetition, {:sequence, [
                {:literal, "and"},
                {:rule_reference, "bool_secondary", false},
              ]}},
          ]},
          line_number: 318,
        },
        %{
          name: "bool_secondary",
          body: {:alternation, [
            {:sequence, [
              {:literal, "not"},
              {:rule_reference, "bool_secondary", false},
            ]},
            {:rule_reference, "bool_primary", false},
          ]},
          line_number: 320,
        },
        %{
          name: "bool_primary",
          body: {:alternation, [
            {:rule_reference, "relation", false},
            {:literal, "true"},
            {:literal, "false"},
            {:rule_reference, "proc_call", false},
            {:rule_reference, "variable", false},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "bool_expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 322,
        },
        %{
          name: "relation",
          body: {:sequence, [
            {:rule_reference, "simple_arith", false},
            {:group, {:alternation, [
                {:rule_reference, "EQ", true},
                {:rule_reference, "NEQ", true},
                {:rule_reference, "LT", true},
                {:rule_reference, "LEQ", true},
                {:rule_reference, "GT", true},
                {:rule_reference, "GEQ", true},
              ]}},
            {:rule_reference, "simple_arith", false},
          ]},
          line_number: 332,
        },
        %{
          name: "desig_expr",
          body: {:alternation, [
            {:sequence, [
              {:literal, "if"},
              {:rule_reference, "bool_expr", false},
              {:literal, "then"},
              {:rule_reference, "desig_expr", false},
              {:literal, "else"},
              {:rule_reference, "desig_expr", false},
            ]},
            {:rule_reference, "simple_desig", false},
          ]},
          line_number: 337,
        },
        %{
          name: "simple_desig",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:rule_reference, "LBRACKET", true},
              {:rule_reference, "arith_expr", false},
              {:rule_reference, "RBRACKET", true},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "desig_expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "label", false},
          ]},
          line_number: 340,
        },
        %{
          name: "variable",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LBRACKET", true},
                {:rule_reference, "subscripts", false},
                {:rule_reference, "RBRACKET", true},
              ]}},
          ]},
          line_number: 352,
        },
        %{
          name: "subscripts",
          body: {:sequence, [
            {:rule_reference, "arith_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "arith_expr", false},
              ]}},
          ]},
          line_number: 354,
        },
        %{
          name: "proc_call",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:rule_reference, "actual_params", false}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 359,
        },
      ],
      version: 1,
    }
  end
end
