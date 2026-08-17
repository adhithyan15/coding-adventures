defmodule CodingAdventures.ExcelParser.Grammar do
  # AUTO-GENERATED FILE — DO NOT EDIT
  # Source: excel.grammar
  # Regenerate with: grammar-tools compile-grammar excel.grammar
  #
  # This file embeds a ParserGrammar as native Elixir data structures.
  # Call parser_grammar/0 instead of reading and parsing the .grammar file.
  
  alias CodingAdventures.GrammarTools.ParserGrammar
  
  def parser_grammar do
    %ParserGrammar{
      rules: [
        %{
          name: "formula",
          body: {:sequence, [
            {:rule_reference, "ws", false},
            {:optional, {:sequence, [
                {:rule_reference, "EQUALS", true},
                {:rule_reference, "ws", false},
              ]}},
            {:rule_reference, "expression", false},
            {:rule_reference, "ws", false},
          ]},
          line_number: 15,
        },
        %{
          name: "ws",
          body: {:repetition, {:rule_reference, "SPACE", true}},
          line_number: 17,
        },
        %{
          name: "req_space",
          body: {:sequence, [
            {:rule_reference, "SPACE", true},
            {:repetition, {:rule_reference, "SPACE", true}},
          ]},
          line_number: 18,
        },
        %{
          name: "expression",
          body: {:rule_reference, "comparison_expr", false},
          line_number: 20,
        },
        %{
          name: "comparison_expr",
          body: {:sequence, [
            {:rule_reference, "concat_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "comparison_op", false},
                {:rule_reference, "ws", false},
                {:rule_reference, "concat_expr", false},
              ]}},
          ]},
          line_number: 22,
        },
        %{
          name: "comparison_op",
          body: {:alternation, [
            {:rule_reference, "EQUALS", true},
            {:rule_reference, "NOT_EQUALS", true},
            {:rule_reference, "LESS_THAN", true},
            {:rule_reference, "LESS_EQUALS", true},
            {:rule_reference, "GREATER_THAN", true},
            {:rule_reference, "GREATER_EQUALS", true},
          ]},
          line_number: 23,
        },
        %{
          name: "concat_expr",
          body: {:sequence, [
            {:rule_reference, "additive_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "AMP", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "additive_expr", false},
              ]}},
          ]},
          line_number: 26,
        },
        %{
          name: "additive_expr",
          body: {:sequence, [
            {:rule_reference, "multiplicative_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:group, {:alternation, [
                    {:rule_reference, "PLUS", true},
                    {:rule_reference, "MINUS", true},
                  ]}},
                {:rule_reference, "ws", false},
                {:rule_reference, "multiplicative_expr", false},
              ]}},
          ]},
          line_number: 27,
        },
        %{
          name: "multiplicative_expr",
          body: {:sequence, [
            {:rule_reference, "power_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:group, {:alternation, [
                    {:rule_reference, "STAR", true},
                    {:rule_reference, "SLASH", true},
                  ]}},
                {:rule_reference, "ws", false},
                {:rule_reference, "power_expr", false},
              ]}},
          ]},
          line_number: 28,
        },
        %{
          name: "power_expr",
          body: {:sequence, [
            {:rule_reference, "unary_expr", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "CARET", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "unary_expr", false},
              ]}},
          ]},
          line_number: 29,
        },
        %{
          name: "unary_expr",
          body: {:sequence, [
            {:repetition, {:sequence, [
                {:rule_reference, "prefix_op", false},
                {:rule_reference, "ws", false},
              ]}},
            {:rule_reference, "postfix_expr", false},
          ]},
          line_number: 30,
        },
        %{
          name: "prefix_op",
          body: {:alternation, [
            {:rule_reference, "PLUS", true},
            {:rule_reference, "MINUS", true},
          ]},
          line_number: 31,
        },
        %{
          name: "postfix_expr",
          body: {:sequence, [
            {:rule_reference, "primary", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "PERCENT", true},
              ]}},
          ]},
          line_number: 32,
        },
        %{
          name: "primary",
          body: {:alternation, [
            {:rule_reference, "parenthesized_expression", false},
            {:rule_reference, "constant", false},
            {:rule_reference, "function_call", false},
            {:rule_reference, "structure_reference", false},
            {:rule_reference, "reference_expression", false},
            {:rule_reference, "bang_reference", false},
            {:rule_reference, "bang_name", false},
            {:rule_reference, "name_reference", false},
          ]},
          line_number: 34,
        },
        %{
          name: "parenthesized_expression",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "ws", false},
            {:rule_reference, "expression", false},
            {:rule_reference, "ws", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 43,
        },
        %{
          name: "constant",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "ERROR_CONSTANT", true},
            {:rule_reference, "array_constant", false},
          ]},
          line_number: 45,
        },
        %{
          name: "array_constant",
          body: {:sequence, [
            {:rule_reference, "LBRACE", true},
            {:rule_reference, "ws", false},
            {:rule_reference, "array_row", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "SEMICOLON", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "array_row", false},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "SEMICOLON", true},
              ]}},
            {:rule_reference, "ws", false},
            {:rule_reference, "RBRACE", true},
          ]},
          line_number: 47,
        },
        %{
          name: "array_row",
          body: {:sequence, [
            {:rule_reference, "array_item", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COMMA", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "array_item", false},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COMMA", true},
              ]}},
          ]},
          line_number: 48,
        },
        %{
          name: "array_item",
          body: {:alternation, [
            {:rule_reference, "NUMBER", true},
            {:rule_reference, "STRING", true},
            {:rule_reference, "KEYWORD", true},
            {:rule_reference, "ERROR_CONSTANT", true},
          ]},
          line_number: 49,
        },
        %{
          name: "function_call",
          body: {:sequence, [
            {:rule_reference, "function_name", false},
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "ws", false},
            {:optional, {:rule_reference, "function_argument_list", false}},
            {:rule_reference, "ws", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 51,
        },
        %{
          name: "function_name",
          body: {:alternation, [
            {:rule_reference, "FUNCTION_NAME", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 52,
        },
        %{
          name: "function_argument_list",
          body: {:sequence, [
            {:rule_reference, "function_argument", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COMMA", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "function_argument", false},
              ]}},
            {:optional, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COMMA", true},
              ]}},
          ]},
          line_number: 53,
        },
        %{
          name: "function_argument",
          body: {:optional, {:rule_reference, "expression", false}},
          line_number: 54,
        },
        %{
          name: "reference_expression",
          body: {:rule_reference, "union_reference", false},
          line_number: 56,
        },
        %{
          name: "union_reference",
          body: {:sequence, [
            {:rule_reference, "intersection_reference", false},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COMMA", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "intersection_reference", false},
              ]}},
          ]},
          line_number: 57,
        },
        %{
          name: "intersection_reference",
          body: {:sequence, [
            {:rule_reference, "range_reference", false},
            {:repetition, {:sequence, [
                {:rule_reference, "req_space", false},
                {:rule_reference, "range_reference", false},
              ]}},
          ]},
          line_number: 58,
        },
        %{
          name: "range_reference",
          body: {:sequence, [
            {:rule_reference, "reference_primary", false},
            {:optional, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COLON", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "reference_primary", false},
              ]}},
          ]},
          line_number: 59,
        },
        %{
          name: "reference_primary",
          body: {:alternation, [
            {:rule_reference, "parenthesized_reference", false},
            {:rule_reference, "prefixed_reference", false},
            {:rule_reference, "external_reference", false},
            {:rule_reference, "structure_reference", false},
            {:rule_reference, "a1_reference", false},
            {:rule_reference, "bang_reference", false},
            {:rule_reference, "bang_name", false},
            {:rule_reference, "name_reference", false},
          ]},
          line_number: 61,
        },
        %{
          name: "parenthesized_reference",
          body: {:sequence, [
            {:rule_reference, "LPAREN", true},
            {:rule_reference, "ws", false},
            {:rule_reference, "reference_expression", false},
            {:rule_reference, "ws", false},
            {:rule_reference, "RPAREN", true},
          ]},
          line_number: 70,
        },
        %{
          name: "prefixed_reference",
          body: {:sequence, [
            {:rule_reference, "REF_PREFIX", true},
            {:group, {:alternation, [
                {:rule_reference, "a1_reference", false},
                {:rule_reference, "name_reference", false},
                {:rule_reference, "structure_reference", false},
              ]}},
          ]},
          line_number: 71,
        },
        %{
          name: "external_reference",
          body: {:rule_reference, "REF_PREFIX", true},
          line_number: 72,
        },
        %{
          name: "bang_reference",
          body: {:sequence, [
            {:rule_reference, "BANG", true},
            {:group, {:alternation, [
                {:rule_reference, "CELL", true},
                {:rule_reference, "COLUMN_REF", true},
                {:rule_reference, "ROW_REF", true},
                {:rule_reference, "NUMBER", true},
              ]}},
          ]},
          line_number: 73,
        },
        %{
          name: "bang_name",
          body: {:sequence, [
            {:rule_reference, "BANG", true},
            {:rule_reference, "name_reference", false},
          ]},
          line_number: 74,
        },
        %{
          name: "name_reference",
          body: {:rule_reference, "NAME", true},
          line_number: 75,
        },
        %{
          name: "column_reference",
          body: {:sequence, [
            {:optional, {:rule_reference, "DOLLAR", true}},
            {:group, {:alternation, [
                {:rule_reference, "COLUMN_REF", true},
                {:rule_reference, "NAME", true},
              ]}},
          ]},
          line_number: 77,
        },
        %{
          name: "row_reference",
          body: {:sequence, [
            {:optional, {:rule_reference, "DOLLAR", true}},
            {:group, {:alternation, [
                {:rule_reference, "ROW_REF", true},
                {:rule_reference, "NUMBER", true},
              ]}},
          ]},
          line_number: 78,
        },
        %{
          name: "a1_reference",
          body: {:alternation, [
            {:rule_reference, "CELL", true},
            {:rule_reference, "column_reference", false},
            {:rule_reference, "row_reference", false},
            {:rule_reference, "COLUMN_REF", true},
            {:rule_reference, "ROW_REF", true},
            {:rule_reference, "NAME", true},
            {:rule_reference, "NUMBER", true},
          ]},
          line_number: 80,
        },
        %{
          name: "structure_reference",
          body: {:sequence, [
            {:optional, {:rule_reference, "table_name", false}},
            {:rule_reference, "intra_table_reference", false},
          ]},
          line_number: 82,
        },
        %{
          name: "table_name",
          body: {:alternation, [
            {:rule_reference, "TABLE_NAME", true},
            {:rule_reference, "NAME", true},
          ]},
          line_number: 83,
        },
        %{
          name: "intra_table_reference",
          body: {:alternation, [
            {:rule_reference, "STRUCTURED_KEYWORD", true},
            {:rule_reference, "structured_column_range", false},
            {:sequence, [
              {:rule_reference, "LBRACKET", true},
              {:rule_reference, "ws", false},
              {:optional, {:rule_reference, "inner_structure_reference", false}},
              {:rule_reference, "ws", false},
              {:rule_reference, "RBRACKET", true},
            ]},
          ]},
          line_number: 84,
        },
        %{
          name: "inner_structure_reference",
          body: {:alternation, [
            {:sequence, [
              {:rule_reference, "structured_keyword_list", false},
              {:optional, {:sequence, [
                  {:rule_reference, "ws", false},
                  {:rule_reference, "COMMA", true},
                  {:rule_reference, "ws", false},
                  {:rule_reference, "structured_column_range", false},
                ]}},
            ]},
            {:rule_reference, "structured_column_range", false},
          ]},
          line_number: 87,
        },
        %{
          name: "structured_keyword_list",
          body: {:sequence, [
            {:rule_reference, "STRUCTURED_KEYWORD", true},
            {:repetition, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COMMA", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "STRUCTURED_KEYWORD", true},
              ]}},
          ]},
          line_number: 89,
        },
        %{
          name: "structured_column_range",
          body: {:sequence, [
            {:rule_reference, "structured_column", false},
            {:optional, {:sequence, [
                {:rule_reference, "ws", false},
                {:rule_reference, "COLON", true},
                {:rule_reference, "ws", false},
                {:rule_reference, "structured_column", false},
              ]}},
          ]},
          line_number: 90,
        },
        %{
          name: "structured_column",
          body: {:alternation, [
            {:rule_reference, "STRUCTURED_COLUMN", true},
            {:sequence, [
              {:rule_reference, "AT", true},
              {:rule_reference, "STRUCTURED_COLUMN", true},
            ]},
          ]},
          line_number: 91,
        },
      ],
      version: 1,
    }
  end
end
