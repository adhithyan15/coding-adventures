defmodule CodingAdventures.DartmouthBasicParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: dartmouth_basic.grammar
  # Regenerate with: grammar-tools compile-grammar dartmouth_basic.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:rule_reference, "line", false}},
          line_number: 70,
        },
        %{
          name: "line",
          body: {:sequence, [
            {:rule_reference, "LINE_NUM", true},
            {:optional, {:rule_reference, "statement", false}},
            {:rule_reference, "NEWLINE", true},
          ]},
          line_number: 81,
        },
        %{
          name: "statement",
          body: {:alternation, [
            {:rule_reference, "let_stmt", false},
            {:rule_reference, "print_stmt", false},
            {:rule_reference, "input_stmt", false},
            {:rule_reference, "if_stmt", false},
            {:rule_reference, "goto_stmt", false},
            {:rule_reference, "gosub_stmt", false},
            {:rule_reference, "return_stmt", false},
            {:rule_reference, "for_stmt", false},
            {:rule_reference, "next_stmt", false},
            {:rule_reference, "end_stmt", false},
            {:rule_reference, "stop_stmt", false},
            {:rule_reference, "rem_stmt", false},
            {:rule_reference, "read_stmt", false},
            {:rule_reference, "data_stmt", false},
            {:rule_reference, "restore_stmt", false},
            {:rule_reference, "dim_stmt", false},
            {:rule_reference, "def_stmt", false},
          ]},
          line_number: 91,
        },
        %{
          name: "let_stmt",
          body: {:sequence, [
            {:literal, "LET"},
            {:rule_reference, "variable", false},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
          ]},
          line_number: 121,
        },
        %{
          name: "print_stmt",
          body: {:sequence, [
            {:literal, "PRINT"},
            {:optional, {:rule_reference, "print_list", false}},
          ]},
          line_number: 137,
        },
        %{
          name: "print_list",
          body: {:sequence, [
            {:rule_reference, "print_item", false},
            {:repetition, {:sequence, [
                {:rule_reference, "print_sep", false},
                {:rule_reference, "print_item", false},
              ]}},
            {:optional, {:rule_reference, "print_sep", false}},
          ]},
          line_number: 139,
        },
        %{
          name: "print_item",
          body: {:alternation, [
            {:rule_reference, "STRING", true},
            {:rule_reference, "expr", false},
          ]},
          line_number: 141,
        },
        %{
          name: "print_sep",
          body: {:alternation, [
            {:rule_reference, "COMMA", true},
            {:rule_reference, "SEMICOLON", true},
          ]},
          line_number: 143,
        },
        %{
          name: "input_stmt",
          body: {:sequence, [
            {:literal, "INPUT"},
            {:rule_reference, "variable", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "variable", false},
              ]}},
          ]},
          line_number: 155,
        },
        %{
          name: "if_stmt",
          body: {:sequence, [
            {:literal, "IF"},
            {:rule_reference, "expr", false},
            {:rule_reference, "relop", false},
            {:rule_reference, "expr", false},
            {:literal, "THEN"},
            {:rule_reference, "NUMBER", true},
          ]},
          line_number: 170,
        },
        %{
          name: "relop",
          body: {:alternation, [
            {:rule_reference, "EQ", true},
            {:rule_reference, "LT", true},
            {:rule_reference, "GT", true},
            {:rule_reference, "LE", true},
            {:rule_reference, "GE", true},
            {:rule_reference, "NE", true},
          ]},
          line_number: 172,
        },
        %{
          name: "goto_stmt",
          body: {:sequence, [
            {:literal, "GOTO"},
            {:rule_reference, "NUMBER", true},
          ]},
          line_number: 183,
        },
        %{
          name: "gosub_stmt",
          body: {:sequence, [
            {:literal, "GOSUB"},
            {:rule_reference, "NUMBER", true},
          ]},
          line_number: 198,
        },
        %{
          name: "return_stmt",
          body: {:literal, "RETURN"},
          line_number: 200,
        },
        %{
          name: "for_stmt",
          body: {:sequence, [
            {:literal, "FOR"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
            {:literal, "TO"},
            {:rule_reference, "expr", false},
            {:optional, {:sequence, [
                {:literal, "STEP"},
                {:rule_reference, "expr", false},
              ]}},
          ]},
          line_number: 222,
        },
        %{
          name: "next_stmt",
          body: {:sequence, [
            {:literal, "NEXT"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 224,
        },
        %{
          name: "end_stmt",
          body: {:literal, "END"},
          line_number: 233,
        },
        %{
          name: "stop_stmt",
          body: {:literal, "STOP"},
          line_number: 234,
        },
        %{
          name: "rem_stmt",
          body: {:literal, "REM"},
          line_number: 247,
        },
        %{
          name: "read_stmt",
          body: {:sequence, [
            {:literal, "READ"},
            {:rule_reference, "variable", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "variable", false},
              ]}},
          ]},
          line_number: 263,
        },
        %{
          name: "data_stmt",
          body: {:sequence, [
            {:literal, "DATA"},
            {:rule_reference, "NUMBER", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NUMBER", true},
              ]}},
          ]},
          line_number: 265,
        },
        %{
          name: "restore_stmt",
          body: {:literal, "RESTORE"},
          line_number: 267,
        },
        %{
          name: "dim_stmt",
          body: {:sequence, [
            {:literal, "DIM"},
            {:rule_reference, "dim_decl", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "dim_decl", false},
              ]}},
          ]},
          line_number: 280,
        },
        %{
          name: "dim_decl",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "NUMBER", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NUMBER", true},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 282,
        },
        %{
          name: "def_stmt",
          body: {:sequence, [
            {:literal, "DEF"},
            {:rule_reference, "USER_FN", true},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "RPAREN", true},
            {:rule_reference, "EQ", true},
            {:rule_reference, "expr", false},
          ]},
          line_number: 295,
        },
        %{
          name: "variable",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "expr", false},
                ]}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 312,
        },
        %{
          name: "expr",
          body: {:sequence, [
            {:rule_reference, "term", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "term", false},
              ]}},
          ]},
          line_number: 335,
        },
        %{
          name: "term",
          body: {:sequence, [
            {:rule_reference, "power", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                  ]}},
                {:rule_reference, "power", false},
              ]}},
          ]},
          line_number: 337,
        },
        %{
          name: "power",
          body: {:sequence, [
            {:rule_reference, "unary", false},
            {:optional, {:sequence, [
                {:rule_reference, "CARET", true},
                {:rule_reference, "power", false},
              ]}},
          ]},
          line_number: 343,
        },
        %{
          name: "unary",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "MINUS", true},
              {:rule_reference, "primary", false},
            ]},
            {:rule_reference, "primary", false},
          ]},
          line_number: 348,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:sequence, [
              {:rule_reference, "BUILTIN_FN", true},
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "USER_FN", true},
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
            {:rule_reference, "variable", false},
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:rule_reference, "expr", false},
              {:rule_reference, "RPAREN", true},
            ]},
          ]},
          line_number: 366,
        },
      ],
      version: 1,
    }
  end
end
