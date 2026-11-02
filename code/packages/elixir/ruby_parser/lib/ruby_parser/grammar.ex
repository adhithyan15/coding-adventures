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
              {:negative_lookahead, {:literal, "+"}},
              {:negative_lookahead, {:literal, "-"}},
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
          line_number: 87,
        },
        %{
          name: "mlhs_target",
          body: {:sequence, [
            {:optional, {:literal, "*"}},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 88,
        },
        %{
          name: "modifier_statement",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "assignment", false},
                {:rule_reference, "method_call_no_paren", false},
                {:sequence, [
                  {:rule_reference, "method_call", false},
                  {:negative_lookahead, {:literal, "+"}},
                  {:negative_lookahead, {:literal, "-"}},
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
          line_number: 124,
        },
        %{
          name: "def_statement",
          body: {:sequence, [
            {:literal, "def"},
            {:optional, {:rule_reference, "def_receiver", false}},
            {:rule_reference, "NAME", true},
            {:optional, {:sequence, [
                {:negative_lookahead, {:literal, "( line start"}},
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
          line_number: 148,
        },
        %{
          name: "def_receiver",
          body: {:sequence, [
            {:rule_reference, "singleton_receiver", false},
            {:literal, "."},
          ]},
          line_number: 154,
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
          line_number: 163,
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
          line_number: 184,
        },
        %{
          name: "singleton_receiver",
          body: {:alternation, [
            {:literal, "self"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 186,
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
          line_number: 187,
        },
        %{
          name: "method_with_block",
          body: {:sequence, [
            {:group, {:alternation, [
                {:rule_reference, "NAME", true},
                {:rule_reference, "KEYWORD", true},
              ]}},
            {:optional, {:sequence, [
                {:negative_lookahead, {:literal, "( line start"}},
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
          line_number: 189,
        },
        %{
          name: "block",
          body: {:alternation, [
            {:rule_reference, "do_block", false},
            {:rule_reference, "brace_block", false},
          ]},
          line_number: 190,
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
          line_number: 191,
        },
        %{
          name: "brace_block",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:optional, {:rule_reference, "block_params", false}},
            {:repetition, {:rule_reference, "statement", false}},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 192,
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
          line_number: 202,
        },
        %{
          name: "return_statement",
          body: {:sequence, [
            {:literal, "return"},
            {:group, {:alternation, [
                {:literal, ";"},
                {:optional, {:rule_reference, "expression", false}},
              ]}},
          ]},
          line_number: 209,
        },
        %{
          name: "break_statement",
          body: {:sequence, [
            {:literal, "break"},
            {:group, {:alternation, [
                {:literal, ";"},
                {:optional, {:rule_reference, "expression", false}},
              ]}},
          ]},
          line_number: 210,
        },
        %{
          name: "next_statement",
          body: {:sequence, [
            {:literal, "next"},
            {:group, {:alternation, [
                {:literal, ";"},
                {:optional, {:rule_reference, "expression", false}},
              ]}},
          ]},
          line_number: 211,
        },
        %{
          name: "redo_statement",
          body: {:literal, "redo"},
          line_number: 215,
        },
        %{
          name: "retry_statement",
          body: {:literal, "retry"},
          line_number: 219,
        },
        %{
          name: "alias_statement",
          body: {:sequence, [
            {:literal, "alias"},
            {:rule_reference, "NAME", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 230,
        },
        %{
          name: "undef_statement",
          body: {:sequence, [
            {:literal, "undef"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 242,
        },
        %{
          name: "yield_statement",
          body: {:sequence, [
            {:literal, "yield"},
            {:group, {:alternation, [
                {:literal, ";"},
                {:optional, {:rule_reference, "yield_args", false}},
              ]}},
          ]},
          line_number: 264,
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
          line_number: 265,
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
          line_number: 292,
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
          line_number: 321,
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
          line_number: 366,
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
          line_number: 367,
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
          line_number: 368,
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
          line_number: 369,
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
          line_number: 370,
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
          line_number: 371,
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
          line_number: 372,
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
          line_number: 380,
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
          line_number: 403,
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
          line_number: 404,
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
          line_number: 426,
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
          line_number: 427,
        },
        %{
          name: "literal_pattern",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "symbol_literal", false},
            {:rule_reference, "KEYWORD", true},
          ]},
          line_number: 428,
        },
        %{
          name: "binding_pattern",
          body: {:rule_reference, "NAME", true},
          line_number: 429,
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
          line_number: 430,
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
          line_number: 431,
        },
        %{
          name: "hash_pattern_pair",
          body: {:sequence, [
            {:rule_reference, "NAME", true},
            {:rule_reference, "COLON", true},
            {:optional, {:rule_reference, "pattern", false}},
          ]},
          line_number: 432,
        },
        %{
          name: "splat_pattern",
          body: {:sequence, [
            {:literal, "*"},
            {:optional, {:rule_reference, "NAME", true}},
          ]},
          line_number: 439,
        },
        %{
          name: "pin_pattern",
          body: {:sequence, [
            {:literal, "^"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 444,
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
          line_number: 450,
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
          line_number: 471,
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
                  {:group, {:alternation, [
                      {:sequence, [
                        {:literal, "=>"},
                        {:rule_reference, "NAME", true},
                      ]},
                      {:literal, ";"},
                    ]}},
                ]},
                {:literal, ";"},
              ]}},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "rescue"}},
                {:negative_lookahead, {:literal, "ensure"}},
                {:negative_lookahead, {:literal, "end"}},
                {:rule_reference, "statement", false},
              ]}},
          ]},
          line_number: 491,
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
          line_number: 492,
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
          line_number: 493,
        },
        %{
          name: "index_write_receiver_postfix",
          body: {:alternation, [
            {:rule_reference, "dot_call", false},
            {:rule_reference, "scope_resolution", false},
            {:rule_reference, "index_suffix", false},
          ]},
          line_number: 546,
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
          line_number: 547,
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
          line_number: 548,
        },
        %{
          name: "rightward_assignment",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:literal, "=>"},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 567,
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
            {:negative_lookahead, {:literal, "( line start"}},
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
          line_number: 588,
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
                {:negative_lookahead, {:literal, "( line start"}},
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
          line_number: 589,
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
          line_number: 597,
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
          line_number: 652,
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
            {:negative_lookahead, {:literal, "( line start"}},
            {:negative_lookahead, {:literal, "- line start"}},
            {:negative_lookahead, {:literal, "+ line start"}},
            {:rule_reference, "expression", false},
            {:repetition, {:sequence, [
                {:rule_reference, "COMMA", true},
                {:rule_reference, "expression", false},
              ]}},
          ]},
          line_number: 706,
        },
        %{
          name: "expression_stmt",
          body: {:sequence, [
            {:rule_reference, "expression", false},
            {:optional, {:literal, ";"}},
          ]},
          line_number: 710,
        },
        %{
          name: "expression",
          body: {:rule_reference, "ternary", false},
          line_number: 817,
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
          line_number: 818,
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
          line_number: 819,
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
          line_number: 820,
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
          line_number: 821,
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
          line_number: 828,
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
          line_number: 844,
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
          line_number: 845,
        },
        %{
          name: "sum",
          body: {:sequence, [
            {:rule_reference, "term", false},
            {:repetition, {:sequence, [
                {:negative_lookahead, {:literal, "- line start"}},
                {:negative_lookahead, {:literal, "+ line start"}},
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "term", false},
              ]}},
          ]},
          line_number: 850,
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
          line_number: 851,
        },
        %{
          name: "super_expr",
          body: {:sequence, [
            {:literal, "super"},
            {:group, {:alternation, [
                {:literal, ";"},
                {:optional, {:rule_reference, "super_args", false}},
              ]}},
          ]},
          line_number: 923,
        },
        %{
          name: "index_suffix",
          body: {:sequence, [
            {:negative_lookahead, {:literal, "[ line start"}},
            {:rule_reference, "LBRACKET", true},
            {:rule_reference, "expression", false},
            {:rule_reference, "RBRACKET", true},
          ]},
          line_number: 951,
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
                    {:negative_lookahead, {:literal, "if_modifier"}},
                    {:negative_lookahead, {:literal, "unless_modifier"}},
                    {:negative_lookahead, {:literal, "while_modifier"}},
                    {:negative_lookahead, {:literal, "until_modifier"}},
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
                {:rule_reference, "unary_plus", false},
              ]}},
            {:repetition, {:alternation, [
                {:rule_reference, "dot_call", false},
                {:rule_reference, "scope_resolution", false},
                {:rule_reference, "index_suffix", false},
              ]}},
          ]},
          line_number: 955,
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
          line_number: 974,
        },
        %{
          name: "unary_minus",
          body: {:sequence, [
            {:rule_reference, "MINUS", true},
            {:rule_reference, "factor", false},
          ]},
          line_number: 975,
        },
        %{
          name: "unary_plus",
          body: {:sequence, [
            {:literal, "+ line start"},
            {:rule_reference, "factor", false},
          ]},
          line_number: 982,
        },
        %{
          name: "defined_expression",
          body: {:sequence, [
            {:literal, "defined?"},
            {:rule_reference, "factor", false},
          ]},
          line_number: 993,
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
          line_number: 1000,
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
          line_number: 1001,
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
          line_number: 1002,
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
          line_number: 1003,
        },
      ],
      version: 1,
    }
  end
end
