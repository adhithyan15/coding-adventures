defmodule CodingAdventures.HaskellParser.Grammar.V1_1 do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: haskell1.1.grammar
  # Regenerate with: grammar-tools compile-grammar haskell1.1.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "file",
          body: {:repetition, {:sequence, [
              {:rule_reference, "declaration", false},
              {:optional, {:rule_reference, "layout_sep", false}},
            ]}},
          line_number: 10,
        },
        %{
          name: "declaration",
          body: {:alternation, [
            {:rule_reference, "module_decl", false},
            {:rule_reference, "let_decl", false},
            {:rule_reference, "do_decl", false},
            {:rule_reference, "expr_decl", false},
          ]},
          line_number: 11,
        },
        %{
          name: "layout_open",
          body: {:alternation, [
            {:rule_reference, "VIRTUAL_LBRACE", true},
            {:rule_reference, "LBRACE", true},
            {:literal, "{"},
          ]},
          line_number: 18,
        },
        %{
          name: "layout_close",
          body: {:alternation, [
            {:rule_reference, "VIRTUAL_RBRACE", true},
            {:rule_reference, "RBRACE", true},
            {:literal, "}"},
          ]},
          line_number: 19,
        },
        %{
          name: "layout_sep",
          body: {:alternation, [
            {:rule_reference, "VIRTUAL_SEMICOLON", true},
            {:rule_reference, "SEMICOLON", true},
            {:rule_reference, "NEWLINE", true},
          ]},
          line_number: 20,
        },
        %{
          name: "module_decl",
          body: {:sequence, [
            {:literal, "module"},
            {:rule_reference, "module_name", false},
            {:literal, "where"},
            {:rule_reference, "layout_open", false},
            {:rule_reference, "module_body", false},
            {:rule_reference, "layout_close", false},
          ]},
          line_number: 22,
        },
        %{
          name: "module_name",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "DOT", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 23,
        },
        %{
          name: "module_body",
          body: {:repetition, {:sequence, [
              {:rule_reference, "declaration", false},
              {:optional, {:rule_reference, "layout_sep", false}},
            ]}},
          line_number: 24,
        },
        %{
          name: "let_decl",
          body: {:sequence, [
            {:literal, "let"},
            {:rule_reference, "layout_open", false},
            {:rule_reference, "let_bindings", false},
            {:rule_reference, "layout_close", false},
            {:literal, "in"},
            {:rule_reference, "expr_decl", false},
          ]},
          line_number: 26,
        },
        %{
          name: "let_bindings",
          body: {:repetition, {:sequence, [
              {:rule_reference, "binding", false},
              {:optional, {:rule_reference, "layout_sep", false}},
            ]}},
          line_number: 27,
        },
        %{
          name: "binding",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expr_decl", false},
          ]},
          line_number: 28,
        },
        %{
          name: "do_decl",
          body: {:sequence, [
            {:literal, "do"},
            {:rule_reference, "layout_open", false},
            {:repetition, {:sequence, [
                {:rule_reference, "expr_decl", false},
                {:optional, {:rule_reference, "layout_sep", false}},
              ]}},
            {:rule_reference, "layout_close", false},
          ]},
          line_number: 30,
        },
        %{
          name: "expr_decl",
          body: {:alternation, [
            {:rule_reference, "lambda_expr", false},
            {:rule_reference, "app_expr", false},
            {:rule_reference, "NAME", true},
            {:rule_reference, "INTEGER", true},
            {:rule_reference, "FLOAT", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "CHARACTER", true},
          ]},
          line_number: 32,
        },
        %{
          name: "lambda_expr",
          body: {:sequence, [
            {:rule_reference, "LAMBDA", true},
            {:repetition, {:rule_reference, "NAME", true}},
            {:rule_reference, "RARROW", true},
            {:rule_reference, "expr_decl", false},
          ]},
          line_number: 34,
        },
        %{
          name: "app_expr",
          body: {:sequence, [
            {:rule_reference, "atom_expr", false},
            {:repetition, {:rule_reference, "atom_expr", false}},
          ]},
          line_number: 35,
        },
        %{
          name: "atom_expr",
          body: {:alternation, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "INTEGER", true},
            {:rule_reference, "FLOAT", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "CHARACTER", true},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr_decl", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr_list", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "LBRACKET", true},
              {:optional, {:rule_reference, "expr_list", false}},
              {:rule_reference, "RBRACKET", true},
            ]},
          ]},
          line_number: 36,
        },
        %{
          name: "expr_list",
          body: {:sequence, [
            {:rule_reference, "expr_decl", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expr_decl", false},
              ]}},
          ]},
          line_number: 45,
        },
      ],
      version: 1,
    }
  end
  end
