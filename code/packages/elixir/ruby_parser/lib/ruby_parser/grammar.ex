defmodule CodingAdventures.RubyParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: ruby.grammar
  # Regenerate with: grammar-tools compile-grammar ruby.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.

  alias CodingAdventures.GrammarTools.ParserGrammar

  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "program",
          body: {:repetition, {:rule_reference, "statement", false}},
          line_number: 27,
        },
        %{
          name: "statement",
          body: {:alternation, [
            {:rule_reference, "endless_def_statement", false},
            {:rule_reference, "def_statement", false},
            {:rule_reference, "class_statement", false},
            {:rule_reference, "module_statement", false},
            {:rule_reference, "if_statement", false},
            {:rule_reference, "unless_statement", false},
            {:rule_reference, "while_statement", false},
            {:rule_reference, "until_statement", false},
            {:rule_reference, "for_statement", false},
            {:rule_reference, "case_statement", false},
            {:rule_reference, "begin_statement", false},
            {:rule_reference, "return_statement", false},
            {:rule_reference, "break_statement", false},
            {:rule_reference, "next_statement", false},
            {:rule_reference, "redo_statement", false},
            {:rule_reference, "retry_statement", false},
            {:rule_reference, "yield_statement", false},
            {:rule_reference, "alias_statement", false},
            {:rule_reference, "undef_statement", false},
            {:rule_reference, "multi_assignment", false},
            {:rule_reference, "modifier_statement", false},
            {:rule_reference, "rightward_assignment", false},
            {:rule_reference, "index_assignment", false},
            {:rule_reference, "assignment", false},
            {:rule_reference, "defined_expression", false},
            {:rule_reference, "method_with_block", false},
            {:sequence, [
              {:rule_reference, "method_call", false},
              {:negative_lookahead, {:rule_reference, "PLUS", true}},
              {:negative_lookahead, {:rule_reference, "MINUS", true}},
              {:negative_lookahead, {:rule_reference, "STAR", true}},
              {:negative_lookahead, {:rule_reference, "SLASH", true}},
              {:negative_lookahead, {:literal, "<<"}},
              {:negative_lookahead, {:literal, "=="}},
              {:negative_lookahead, {:literal, "!="}},
              {:negative_lookahead, {:literal, "<="}},
              {:negative_lookahead, {:literal, ">="}},
              {:negative_lookahead, {:literal, "<"}},
              {:negative_lookahead, {:literal, ">"}},
              {:negative_lookahead, {:literal, "&&"}},
              {:negative_lookahead, {:literal, "||"}},
              {:negative_lookahead, {:literal, "and"}},
              {:negative_lookahead, {:literal, "or"}},
              {:negative_lookahead, {:literal, "..."}},
              {:negative_lookahead, {:literal, ".."}},
              {:negative_lookahead, {:literal, "?"}},
            ]},
            {:rule_reference, "method_call_no_paren", false},
            {:rule_reference, "expression_stmt", false},
          ]},
          line_number: 28,
        },
        %{
          name: "multi_assignment",
          body: {:sequence, [
            {:rule_reference, "mlhs_target", false},
            {:rule_reference, "COMMA", true},
            {:rule_reference, "mlhs_target", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "mlhs_target", false},
              ]}},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 83,
        },
        %{
          name: "mlhs_target",
          body: {:sequence, [
            {:optional, {:literal, "*"}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 84,
        },
        %{
          name: "modifier_statement",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "assignment", false},
                {:rule_reference, "method_call_no_paren", false},
                {:sequence, [
                  {:rule_reference, "method_call", false},
                  {:negative_lookahead, {:rule_reference, "PLUS", true}},
                  {:negative_lookahead, {:rule_reference, "MINUS", true}},
                  {:negative_lookahead, {:rule_reference, "STAR", true}},
                  {:negative_lookahead, {:rule_reference, "SLASH", true}},
                  {:negative_lookahead, {:literal, "<<"}},
                  {:negative_lookahead, {:literal, "=="}},
                  {:negative_lookahead, {:literal, "!="}},
                  {:negative_lookahead, {:literal, "<="}},
                  {:negative_lookahead, {:literal, ">="}},
                  {:negative_lookahead, {:literal, "<"}},
                  {:negative_lookahead, {:literal, ">"}},
                  {:negative_lookahead, {:literal, "&&"}},
                  {:negative_lookahead, {:literal, "||"}},
                  {:negative_lookahead, {:literal, "and"}},
                  {:negative_lookahead, {:literal, "or"}},
                  {:negative_lookahead, {:literal, "..."}},
                  {:negative_lookahead, {:literal, ".."}},
                  {:negative_lookahead, {:literal, "?"}},
                ]},
                {:rule_reference, "expression_stmt", false},
              ]}},
            {:group, {:alternation, [
                {:literal, "if_modifier"},
                {:literal, "unless_modifier"},
                {:literal, "while_modifier"},
                {:literal, "until_modifier"},
              ]}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 120,
        },
        %{
          name: "def_statement",
          body: {:sequence, [
            {:literal, "def"},
            {:optional, {:rule_reference, "def_receiver", false}},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "params", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "rescue"}},
                {:negative_lookahead, {:literal, "ensure"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:repetition, {:rule_reference, "rescue_clause", false}},
            {:optional, {:rule_reference, "ensure_clause", false}},
            {:literal, "end"},
          ]},
          line_number: 144,
        },
        %{
          name: "def_receiver",
          body: {:sequence, [
            {:rule_reference, "singleton_receiver", false},
            {:literal, "."},
          ]},
          line_number: 150,
        },
        %{
          name: "endless_def_statement",
          body: {:sequence, [
            {:literal, "def"},
            {:optional, {:rule_reference, "def_receiver", false}},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "params", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 159,
        },
        %{
          name: "class_statement",
          body: {:alternation, [
            {:sequence, [
              {:literal, "class"},
              {:literal, "<<"},
              {:rule_reference, "singleton_receiver", false},
              {:repetition, {:sequence, [
                  {:negative_lookahead, {:literal, "end"}},
                  {:rule_reference, "statement", false},
                ]}},
              {:literal, "end"},
            ]},
            {:sequence, [
              {:literal, "class"},
              {:rule_reference, "NAME", true},
              {:optional, {:sequence, [
                  {:literal, "<"},
                  {:rule_reference, "NAME", true},
                ]}},
              {:repetition, {:sequence, [
                  {:negative_lookahead, {:literal, "end"}},
                  {:rule_reference, "statement", false},
                ]}},
              {:literal, "end"},
            ]},
          ]},
          line_number: 180,
        },
        %{
          name: "singleton_receiver",
          body: {:alternation, [
            {:literal, "self"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 182,
        },
        %{
          name: "module_statement",
          body: {:sequence, [
            {:literal, "module"},
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 183,
        },
        %{
          name: "method_with_block",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "KEYWORD", true},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:sequence, [
                    {:rule_reference, "expression", false},
                    {:repetition, {:sequence, [
                        {:rule_reference, "COMMA", true},
                        {:rule_reference, "expression", false},
                      ]}},
                  ]}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:rule_reference, "block", false},
          ]},
          line_number: 185,
        },
        %{
          name: "block",
          body: {:alternation, [
            {:rule_reference, "do_block", false},
            {:rule_reference, "brace_block", false},
          ]},
          line_number: 186,
        },
        %{
          name: "do_block",
          body: {:sequence, [
            {:literal, "do"},
            {:optional, {:rule_reference, "block_params", false}},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 187,
        },
        %{
          name: "brace_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "block_params", false}},
            {:repetition, {:rule_reference, "statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 188,
        },
        %{
          name: "block_params",
          body: {:sequence, [
            {:literal, "|"},
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
            {:optional, {:sequence, [
                {:literal, ";"},
                {:rule_reference, "NAME", true},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "NAME", true},
                  ]}},
              ]}},
            {:literal, "|"},
          ]},
          line_number: 198,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:optional, {:rule_reference, "expression", false}},
          ]},
          line_number: 200,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:optional, {:rule_reference, "expression", false}},
          ]},
          line_number: 201,
        },
        %{
          name: "next_statement",
          body: {:sequence, [
            {:literal, "next"},
            {:optional, {:rule_reference, "expression", false}},
          ]},
          line_number: 202,
        },
        %{
          name: "redo_statement",
          body: {:literal, "redo"},
          line_number: 206,
        },
        %{
          name: "retry_statement",
          body: {:literal, "retry"},
          line_number: 210,
        },
        %{
          name: "alias_statement",
          body: {:sequence, [
            {:literal, "alias"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 221,
        },
        %{
          name: "undef_statement",
          body: {:sequence, [
            {:literal, "undef"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 233,
        },
        %{
          name: "yield_statement",
          body: {:sequence, [
            {:literal, "yield"},
            {:optional, {:rule_reference, "yield_args", false}},
          ]},
          line_number: 255,
        },
        %{
          name: "yield_args",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:optional, {:sequence, [
                  {:rule_reference, "call_arg", false},
                  {:repetition, {:sequence, [
                      {:rule_reference, "COMMA", true},
                      {:rule_reference, "call_arg", false},
                    ]}},
                ]}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "call_arg", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "call_arg", false},
                ]}},
            ]},
          ]},
          line_number: 256,
        },
        %{
          name: "super_args",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "LPAREN", true},
              {:optional, {:sequence, [
                  {:rule_reference, "call_arg", false},
                  {:repetition, {:sequence, [
                      {:rule_reference, "COMMA", true},
                      {:rule_reference, "call_arg", false},
                    ]}},
                ]}},
              {:rule_reference, "RPAREN", true},
            ]},
            {:sequence, [
              {:rule_reference, "call_arg", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "call_arg", false},
                ]}},
            ]},
          ]},
          line_number: 283,
        },
        %{
          name: "params",
          body: {:alternation, [
            {:literal, "..."},
            {:sequence, [
              {:rule_reference, "param", false},
              {:repetition, {:sequence, [
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "param", false},
                ]}},
            ]},
          ]},
          line_number: 312,
        },
        %{
          name: "param",
          body: {:sequence, [
            {:optional, {:alternation, [
                {:literal, "*"},
                {:literal, "**"},
              ]}},
            {:rule_reference, "NAME", true},
            {:optional, {:alternation, [
                {:sequence, [
                  {:rule_reference, "COLON", true},
                  {:optional, {:rule_reference, "expression", false}},
                ]},
                {:sequence, [
                  {:rule_reference, "EQUALS", true},
                  {:rule_reference, "expression", false},
                ]},
              ]}},
          ]},
          line_number: 357,
        },
        %{
          name: "if_statement",
          body: {:sequence, [
            {:literal, "if"},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "else"}},
                {:negative_lookahead, {:literal, "elsif"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:repetition, {:rule_reference, "elsif_clause", false}},
            {:optional, {:rule_reference, "else_clause", false}},
            {:literal, "end"},
          ]},
          line_number: 358,
        },
        %{
          name: "elsif_clause",
          body: {:sequence, [
            {:literal, "elsif"},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "else"}},
                {:negative_lookahead, {:literal, "elsif"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 359,
        },
        %{
          name: "else_clause",
          body: {:sequence, [
            {:literal, "else"},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 360,
        },
        %{
          name: "unless_statement",
          body: {:sequence, [
            {:literal, "unless"},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "else"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:optional, {:rule_reference, "else_clause", false}},
            {:literal, "end"},
          ]},
          line_number: 361,
        },
        %{
          name: "while_statement",
          body: {:sequence, [
            {:literal, "while"},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 362,
        },
        %{
          name: "until_statement",
          body: {:sequence, [
            {:literal, "until"},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 363,
        },
        %{
          name: "for_statement",
          body: {:sequence, [
            {:literal, "for"},
            {:rule_reference, "NAME", true},
            {:literal, "in"},
            {:rule_reference, "expression", false},
            {:optional, {:literal, "do"}},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:literal, "end"},
          ]},
          line_number: 371,
        },
        %{
          name: "case_statement",
          body: {:sequence, [
            {:literal, "case"},
            {:rule_reference, "expression", false},
            {:repetition, {:alternation, [
                {:rule_reference, "when_clause", false},
                {:rule_reference, "in_clause", false},
              ]}},
            {:optional, {:rule_reference, "else_clause", false}},
            {:literal, "end"},
          ]},
          line_number: 394,
        },
        %{
          name: "when_clause",
          body: {:sequence, [
            {:literal, "when"},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "when"}},
                {:negative_lookahead, {:literal, "in"}},
                {:negative_lookahead, {:literal, "else"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 395,
        },
        %{
          name: "in_clause",
          body: {:sequence, [
            {:literal, "in"},
            {:rule_reference, "pattern", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "when"}},
                {:negative_lookahead, {:literal, "in"}},
                {:negative_lookahead, {:literal, "else"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 417,
        },
        %{
          name: "pattern",
          body: {:alternation, [
            {:rule_reference, "array_pattern", false},
            {:rule_reference, "hash_pattern", false},
            {:rule_reference, "class_pattern", false},
            {:rule_reference, "pin_pattern", false},
            {:rule_reference, "literal_pattern", false},
            {:rule_reference, "binding_pattern", false},
          ]},
          line_number: 418,
        },
        %{
          name: "literal_pattern",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "symbol_literal", false},
            {:rule_reference, "KEYWORD", true},
          ]},
          line_number: 419,
        },
        %{
          name: "binding_pattern",
          body: {:rule_reference, "NAME", true},
          line_number: 420,
        },
        %{
          name: "array_pattern",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:optional, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "splat_pattern", false},
                    {:rule_reference, "pattern", false},
                  ]}},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:group, {:alternation, [
                        {:rule_reference, "splat_pattern", false},
                        {:rule_reference, "pattern", false},
                      ]}},
                  ]}},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 421,
        },
        %{
          name: "hash_pattern",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "hash_pattern_pair", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "hash_pattern_pair", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 422,
        },
        %{
          name: "hash_pattern_pair",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:optional, {:rule_reference, "pattern", false}},
          ]},
          line_number: 423,
        },
        %{
          name: "splat_pattern",
          body: {:sequence, [
            {:literal, "*"},
            {:optional, {:rule_reference, "NAME", true}},
          ]},
          line_number: 430,
        },
        %{
          name: "pin_pattern",
          body: {:sequence, [
            {:literal, "^"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 435,
        },
        %{
          name: "class_pattern",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "LPAREN", true},
            {:optional, {:sequence, [
                {:rule_reference, "pattern", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "pattern", false},
                  ]}},
              ]}},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 441,
        },
        %{
          name: "begin_statement",
          body: {:sequence, [
            {:literal, "begin"},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "rescue"}},
                {:negative_lookahead, {:literal, "ensure"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
            {:repetition, {:rule_reference, "rescue_clause", false}},
            {:optional, {:rule_reference, "ensure_clause", false}},
            {:literal, "end"},
          ]},
          line_number: 462,
        },
        %{
          name: "rescue_clause",
          body: {:sequence, [
            {:literal, "rescue"},
            {:optional, {:alternation, [
                {:sequence, [
                  {:literal, "=>"},
                  {:rule_reference, "NAME", true},
                ]},
                {:sequence, [
                  {:rule_reference, "exception_list", false},
                  {:literal, "=>"},
                  {:rule_reference, "NAME", true},
                ]},
              ]}},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "rescue"}},
                {:negative_lookahead, {:literal, "ensure"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 476,
        },
        %{
          name: "exception_list",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 477,
        },
        %{
          name: "ensure_clause",
          body: {:sequence, [
            {:literal, "ensure"},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 478,
        },
        %{
          name: "index_write_receiver_postfix",
          body: {:alternation, [
            {:rule_reference, "dot_call", false},
            {:rule_reference, "scope_resolution", false},
            {:rule_reference, "index_suffix", false},
          ]},
          line_number: 531,
        },
        %{
          name: "index_assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:repetition, {:sequence, [
                {:rule_reference, "index_write_receiver_postfix", false},
                {:positive_lookahead, {:rule_reference, "index_write_receiver_postfix", false}},
              ]}},
            {:rule_reference, "index_suffix", false},
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "expression", false},
          ]},
          line_number: 532,
        },
        %{
          name: "assignment",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:group, {:alternation, [
                {:rule_reference, "EQUALS", true},
                {:literal, "+="},
                {:literal, "-="},
                {:literal, "*="},
                {:literal, "/="},
                {:literal, "%="},
                {:literal, "**="},
                {:literal, "<<="},
                {:literal, ">>="},
                {:literal, "&="},
                {:literal, "|="},
                {:literal, "^="},
                {:literal, "||="},
                {:literal, "&&="},
              ]}},
            {:rule_reference, "expression", false},
          ]},
          line_number: 533,
        },
        %{
          name: "rightward_assignment",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:literal, "=>"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 552,
        },
        %{
          name: "method_call",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:sequence, [
                  {:negative_lookahead, {:literal, "super"}},
                  {:rule_reference, "KEYWORD", true},
                ]},
              ]}},
            {:rule_reference, "LPAREN", true},
            {:optional, {:sequence, [
                {:rule_reference, "call_arg", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "call_arg", false},
                  ]}},
              ]}},
            {:rule_reference, "RPAREN", true},
            {:repetition, {:rule_reference, "dot_call", false}},
          ]},
          line_number: 569,
        },
        %{
          name: "dot_call",
          body: {:sequence, [
            {:literal, "."},
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "KEYWORD", true},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:sequence, [
                    {:rule_reference, "call_arg", false},
                    {:repetition, {:sequence, [
                        {:rule_reference, "COMMA", true},
                        {:rule_reference, "call_arg", false},
                      ]}},
                  ]}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:optional, {:rule_reference, "block", false}},
          ]},
          line_number: 570,
        },
        %{
          name: "scope_resolution",
          body: {:sequence, [
            {:literal, "::"},
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "KEYWORD", true},
              ]}},
          ]},
          line_number: 578,
        },
        %{
          name: "call_arg",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:rule_reference, "COLON", true},
              {:rule_reference, "expression", false},
            ]},
            {:sequence, [
              {:optional, {:alternation, [
                  {:literal, "*"},
                  {:literal, "**"},
                  {:literal, "&"},
                ]}},
              {:rule_reference, "expression", false},
            ]},
          ]},
          line_number: 633,
        },
        %{
          name: "method_call_no_paren",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:sequence, [
                  {:negative_lookahead, {:literal, "super"}},
                  {:rule_reference, "KEYWORD", true},
                ]},
              ]}},
            {:negative_lookahead, {:literal, "<"}},
            {:negative_lookahead, {:literal, ">"}},
            {:negative_lookahead, {:literal, "<="}},
            {:negative_lookahead, {:literal, ">="}},
            {:negative_lookahead, {:literal, "!="}},
            {:negative_lookahead, {:literal, "&&"}},
            {:negative_lookahead, {:literal, "||"}},
            {:negative_lookahead, {:literal, "<<"}},
            {:negative_lookahead, {:literal, "[ line start"}},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 681,
        },
        %{
          name: "expression_stmt",
          body: {:rule_reference, "expression", false},
          line_number: 684,
        },
        %{
          name: "expression",
          body: {:rule_reference, "ternary", false},
          line_number: 791,
        },
        %{
          name: "ternary",
          body: {:sequence, [
            {:rule_reference, "range", false},
            {:optional, {:sequence, [
                {:literal, "?"},
                {:rule_reference, "expression", false},
                {:literal, ":"},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 792,
        },
        %{
          name: "range",
          body: {:alternation, [
            {:sequence, [
              {:group, {:alternation, [
                  {:literal, "..."},
                  {:literal, ".."},
                ]}},
              {:rule_reference, "logical_or", false},
            ]},
            {:sequence, [
              {:rule_reference, "logical_or", false},
              {:optional, {:sequence, [
                  {:group, {:alternation, [
                      {:literal, "..."},
                      {:literal, ".."},
                    ]}},
                  {:optional, {:rule_reference, "logical_or", false}},
                ]}},
            ]},
          ]},
          line_number: 793,
        },
        %{
          name: "logical_or",
          body: {:sequence, [
            {:rule_reference, "logical_and", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:literal, "||"},
                    {:literal, "or"},
                  ]}},
                {:rule_reference, "logical_and", false},
              ]}},
          ]},
          line_number: 794,
        },
        %{
          name: "logical_and",
          body: {:sequence, [
            {:rule_reference, "logical_not", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:literal, "&&"},
                    {:literal, "and"},
                  ]}},
                {:rule_reference, "logical_not", false},
              ]}},
          ]},
          line_number: 795,
        },
        %{
          name: "logical_not",
          body: {:sequence, [
            {:repetition, {:group, {:alternation, [
                  {:literal, "!"},
                  {:literal, "not"},
                ]}}},
            {:rule_reference, "comparison", false},
          ]},
          line_number: 802,
        },
        %{
          name: "comparison",
          body: {:sequence, [
            {:rule_reference, "shift", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:literal, "=="},
                    {:literal, "!="},
                    {:literal, "<="},
                    {:literal, ">="},
                    {:literal, "<"},
                    {:literal, ">"},
                  ]}},
                {:rule_reference, "shift", false},
              ]}},
          ]},
          line_number: 818,
        },
        %{
          name: "shift",
          body: {:sequence, [
            {:rule_reference, "sum", false},
            {:repetition, {:sequence, [
                {:literal, "<<"},
                {:rule_reference, "sum", false},
              ]}},
          ]},
          line_number: 819,
        },
        %{
          name: "sum",
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
          line_number: 820,
        },
        %{
          name: "term",
          body: {:sequence, [
            {:rule_reference, "factor", false},
            {:repetition, {:sequence, [
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                  ]}},
                {:rule_reference, "factor", false},
              ]}},
          ]},
          line_number: 821,
        },
        %{
          name: "super_expr",
          body: {:sequence, [
            {:literal, "super"},
            {:optional, {:rule_reference, "super_args", false}},
          ]},
          line_number: 890,
        },
        %{
          name: "index_suffix",
          body: {:sequence, [
            {:negative_lookahead, {:literal, "[ line start"}},
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 918,
        },
        %{
          name: "factor",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "defined_expression", false},
                {:rule_reference, "lambda_literal", false},
                {:rule_reference, "super_expr", false},
                {:rule_reference, "method_call", false},
                {:rule_reference, "NUMBER", true},
                {:rule_reference, "STRING", true},
                {:rule_reference, "NAME", true},
                {:group, {:sequence, [
                    {:negative_lookahead, {:literal, "end"}},
                    {:negative_lookahead, {:literal, "rescue"}},
                    {:negative_lookahead, {:literal, "ensure"}},
                    {:negative_lookahead, {:literal, "else"}},
                    {:negative_lookahead, {:literal, "elsif"}},
                    {:negative_lookahead, {:literal, "when"}},
                    {:negative_lookahead, {:literal, "then"}},
                    {:negative_lookahead, {:literal, "in"}},
                    {:negative_lookahead, {:literal, "do"}},
                    {:rule_reference, "KEYWORD", true},
                  ]}},
                {:rule_reference, "symbol_literal", false},
                {:rule_reference, "array_literal", false},
                {:rule_reference, "hash_literal", false},
                {:sequence, [
                  {:rule_reference, "LPAREN", true},
                  {:rule_reference, "expression", false},
                  {:rule_reference, "RPAREN", true},
                ]},
                {:rule_reference, "unary_minus", false},
              ]}},
            {:repetition, {:alternation, [
                {:rule_reference, "dot_call", false},
                {:rule_reference, "scope_resolution", false},
                {:rule_reference, "index_suffix", false},
              ]}},
          ]},
          line_number: 919,
        },
        %{
          name: "lambda_literal",
          body: {:sequence, [
            {:literal, "->"},
            {:optional, {:sequence, [
                {:rule_reference, "LPAREN", true},
                {:optional, {:rule_reference, "params", false}},
                {:rule_reference, "RPAREN", true},
              ]}},
            {:rule_reference, "block", false},
          ]},
          line_number: 938,
        },
        %{
          name: "unary_minus",
          body: {:sequence, [
            {:rule_reference, "MINUS", true},
            {:rule_reference, "factor", false},
          ]},
          line_number: 939,
        },
        %{
          name: "defined_expression",
          body: {:sequence, [
            {:literal, "defined?"},
            {:rule_reference, "factor", false},
          ]},
          line_number: 950,
        },
        %{
          name: "symbol_literal",
          body: {:sequence, [
            {:literal, ":"},
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "KEYWORD", true},
                {:rule_reference, "STRING", true},
              ]}},
          ]},
          line_number: 957,
        },
        %{
          name: "array_literal",
          body: {:sequence, [
            {:rule_reference, "LBRACKET", true},
            {:optional, {:sequence, [
                {:rule_reference, "expression", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "expression", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 958,
        },
        %{
          name: "hash_literal",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:sequence, [
                {:rule_reference, "hash_entry", false},
                {:repetition, {:sequence, [
                    {:rule_reference, "COMMA", true},
                    {:rule_reference, "hash_entry", false},
                  ]}},
              ]}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 959,
        },
        %{
          name: "hash_entry",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:rule_reference, "COLON", true},
              {:rule_reference, "expression", false},
            ]},
            {:sequence, [
              {:rule_reference, "NAME", true},
              {:rule_reference, "COLON", true},
            ]},
            {:sequence, [
              {:rule_reference, "expression", false},
              {:literal, "=>"},
              {:rule_reference, "expression", false},
            ]},
          ]},
          line_number: 960,
        },
      ],
      version: 1,
    }
  end
end
