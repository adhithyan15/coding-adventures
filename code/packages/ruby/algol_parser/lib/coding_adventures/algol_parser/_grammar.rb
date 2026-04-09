# frozen_string_literal: true
# AUTO-GENERATED FILE — DO NOT EDIT
# Source: algol.grammar
# Regenerate with: grammar-tools compile-grammar algol.grammar
#
# This file embeds a ParserGrammar as native Ruby data structures.
# Downstream packages require this file directly instead of reading
# and parsing the .grammar file at runtime.

require "coding_adventures_grammar_tools"

GT = CodingAdventures::GrammarTools unless defined?(GT)

PARSER_GRAMMAR = GT::ParserGrammar.new(
  version: 1,
  rules: [
    # program = block
    GT::GrammarRule.new(
      name: "program",
      body: GT::RuleReference.new(name: "block", is_token: false),
      line_number: 33,
    ),
    # block = BEGIN { declaration SEMICOLON } statement { SEMICOLON statement } END
    GT::GrammarRule.new(
      name: "block",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "BEGIN", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "declaration", is_token: false),
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        ])),
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          GT::RuleReference.new(name: "statement", is_token: false),
        ])),
        GT::RuleReference.new(name: "END", is_token: true),
      ]),
      line_number: 38,
    ),
    # declaration = type_decl | array_decl | switch_decl | procedure_decl
    GT::GrammarRule.new(
      name: "declaration",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "type_decl", is_token: false),
        GT::RuleReference.new(name: "array_decl", is_token: false),
        GT::RuleReference.new(name: "switch_decl", is_token: false),
        GT::RuleReference.new(name: "procedure_decl", is_token: false),
      ]),
      line_number: 44,
    ),
    # type_decl = type ident_list
    GT::GrammarRule.new(
      name: "type_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "type", is_token: false),
        GT::RuleReference.new(name: "ident_list", is_token: false),
      ]),
      line_number: 53,
    ),
    # type = INTEGER | REAL | BOOLEAN | STRING
    GT::GrammarRule.new(
      name: "type",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INTEGER", is_token: true),
        GT::RuleReference.new(name: "REAL", is_token: true),
        GT::RuleReference.new(name: "BOOLEAN", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
      ]),
      line_number: 55,
    ),
    # ident_list = IDENT { COMMA IDENT }
    GT::GrammarRule.new(
      name: "ident_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "NAME", is_token: true),
        ])),
      ]),
      line_number: 57,
    ),
    # array_decl = [ type ] ARRAY array_segment { COMMA array_segment }
    GT::GrammarRule.new(
      name: "array_decl",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "ARRAY", is_token: true),
        GT::RuleReference.new(name: "array_segment", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "array_segment", is_token: false),
        ])),
      ]),
      line_number: 65,
    ),
    # array_segment = ident_list LBRACKET bound_pair { COMMA bound_pair } RBRACKET
    GT::GrammarRule.new(
      name: "array_segment",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "LBRACKET", is_token: true),
        GT::RuleReference.new(name: "bound_pair", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "bound_pair", is_token: false),
        ])),
        GT::RuleReference.new(name: "RBRACKET", is_token: true),
      ]),
      line_number: 67,
    ),
    # bound_pair = arith_expr COLON arith_expr
    GT::GrammarRule.new(
      name: "bound_pair",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arith_expr", is_token: false),
        GT::RuleReference.new(name: "COLON", is_token: true),
        GT::RuleReference.new(name: "arith_expr", is_token: false),
      ]),
      line_number: 71,
    ),
    # switch_decl = SWITCH IDENT ASSIGN switch_list
    GT::GrammarRule.new(
      name: "switch_decl",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "SWITCH", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "ASSIGN", is_token: true),
        GT::RuleReference.new(name: "switch_list", is_token: false),
      ]),
      line_number: 76,
    ),
    # switch_list = desig_expr { COMMA desig_expr }
    GT::GrammarRule.new(
      name: "switch_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "desig_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
        ])),
      ]),
      line_number: 78,
    ),
    # procedure_decl = [ type ] PROCEDURE IDENT [ formal_params ] SEMICOLON
    #                  [ value_part ] { spec_part } proc_body
    GT::GrammarRule.new(
      name: "procedure_decl",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "type", is_token: false)),
        GT::RuleReference.new(name: "PROCEDURE", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "formal_params", is_token: false)),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
        GT::OptionalElement.new(element: GT::RuleReference.new(name: "value_part", is_token: false)),
        GT::Repetition.new(element: GT::RuleReference.new(name: "spec_part", is_token: false)),
        GT::RuleReference.new(name: "proc_body", is_token: false),
      ]),
      line_number: 85,
    ),
    # formal_params = LPAREN ident_list RPAREN
    GT::GrammarRule.new(
      name: "formal_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 88,
    ),
    # value_part = VALUE ident_list SEMICOLON
    GT::GrammarRule.new(
      name: "value_part",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "VALUE", is_token: true),
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 93,
    ),
    # spec_part = specifier ident_list SEMICOLON
    GT::GrammarRule.new(
      name: "spec_part",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "specifier", is_token: false),
        GT::RuleReference.new(name: "ident_list", is_token: false),
        GT::RuleReference.new(name: "SEMICOLON", is_token: true),
      ]),
      line_number: 96,
    ),
    # specifier = INTEGER | REAL | BOOLEAN | STRING | ARRAY | LABEL | SWITCH | PROCEDURE
    GT::GrammarRule.new(
      name: "specifier",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INTEGER", is_token: true),
        GT::RuleReference.new(name: "REAL", is_token: true),
        GT::RuleReference.new(name: "BOOLEAN", is_token: true),
        GT::RuleReference.new(name: "STRING", is_token: true),
        GT::RuleReference.new(name: "ARRAY", is_token: true),
        GT::RuleReference.new(name: "LABEL", is_token: true),
        GT::RuleReference.new(name: "SWITCH", is_token: true),
        GT::RuleReference.new(name: "PROCEDURE", is_token: true),
      ]),
      line_number: 98,
    ),
    # proc_body = block | statement
    GT::GrammarRule.new(
      name: "proc_body",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 100,
    ),
    # statement = [ label COLON ] unlabeled_stmt | [ label COLON ] cond_stmt
    GT::GrammarRule.new(
      name: "statement",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "label", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
          GT::RuleReference.new(name: "unlabeled_stmt", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::OptionalElement.new(element: GT::Sequence.new(elements: [
            GT::RuleReference.new(name: "label", is_token: false),
            GT::RuleReference.new(name: "COLON", is_token: true),
          ])),
          GT::RuleReference.new(name: "cond_stmt", is_token: false),
        ]),
      ]),
      line_number: 108,
    ),
    # label = IDENT | INTEGER_LIT
    GT::GrammarRule.new(
      name: "label",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "INTEGER_LIT", is_token: true),
      ]),
      line_number: 111,
    ),
    # unlabeled_stmt = assign_stmt | goto_stmt | proc_stmt | compound_stmt
    #                | block | for_stmt | empty_stmt
    GT::GrammarRule.new(
      name: "unlabeled_stmt",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "assign_stmt", is_token: false),
        GT::RuleReference.new(name: "goto_stmt", is_token: false),
        GT::RuleReference.new(name: "proc_stmt", is_token: false),
        GT::RuleReference.new(name: "compound_stmt", is_token: false),
        GT::RuleReference.new(name: "block", is_token: false),
        GT::RuleReference.new(name: "for_stmt", is_token: false),
        GT::RuleReference.new(name: "empty_stmt", is_token: false),
      ]),
      line_number: 121,
    ),
    # cond_stmt = IF bool_expr THEN unlabeled_stmt [ ELSE statement ]
    GT::GrammarRule.new(
      name: "cond_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "IF", is_token: true),
        GT::RuleReference.new(name: "bool_expr", is_token: false),
        GT::RuleReference.new(name: "THEN", is_token: true),
        GT::RuleReference.new(name: "unlabeled_stmt", is_token: false),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "ELSE", is_token: true),
          GT::RuleReference.new(name: "statement", is_token: false),
        ])),
      ]),
      line_number: 131,
    ),
    # compound_stmt = BEGIN statement { SEMICOLON statement } END
    GT::GrammarRule.new(
      name: "compound_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "BEGIN", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "SEMICOLON", is_token: true),
          GT::RuleReference.new(name: "statement", is_token: false),
        ])),
        GT::RuleReference.new(name: "END", is_token: true),
      ]),
      line_number: 135,
    ),
    # assign_stmt = left_part { left_part } expression
    GT::GrammarRule.new(
      name: "assign_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "left_part", is_token: false),
        GT::Repetition.new(element: GT::RuleReference.new(name: "left_part", is_token: false)),
        GT::RuleReference.new(name: "expression", is_token: false),
      ]),
      line_number: 140,
    ),
    # left_part = variable ASSIGN
    GT::GrammarRule.new(
      name: "left_part",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::RuleReference.new(name: "ASSIGN", is_token: true),
      ]),
      line_number: 142,
    ),
    # goto_stmt = GOTO desig_expr
    GT::GrammarRule.new(
      name: "goto_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "GOTO", is_token: true),
        GT::RuleReference.new(name: "desig_expr", is_token: false),
      ]),
      line_number: 144,
    ),
    # proc_stmt = IDENT [ LPAREN actual_params RPAREN ]
    GT::GrammarRule.new(
      name: "proc_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "actual_params", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ])),
      ]),
      line_number: 148,
    ),
    # actual_params = expression { COMMA expression }
    GT::GrammarRule.new(
      name: "actual_params",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "expression", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "expression", is_token: false),
        ])),
      ]),
      line_number: 150,
    ),
    # empty_stmt = (empty)
    GT::GrammarRule.new(
      name: "empty_stmt",
      body: GT::Sequence.new(elements: []),
      line_number: 153,
    ),
    # for_stmt = FOR IDENT ASSIGN for_list DO statement
    GT::GrammarRule.new(
      name: "for_stmt",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "FOR", is_token: true),
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "ASSIGN", is_token: true),
        GT::RuleReference.new(name: "for_list", is_token: false),
        GT::RuleReference.new(name: "DO", is_token: true),
        GT::RuleReference.new(name: "statement", is_token: false),
      ]),
      line_number: 161,
    ),
    # for_list = for_elem { COMMA for_elem }
    GT::GrammarRule.new(
      name: "for_list",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "for_elem", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "for_elem", is_token: false),
        ])),
      ]),
      line_number: 163,
    ),
    # for_elem = arith_expr STEP arith_expr UNTIL arith_expr
    #          | arith_expr WHILE bool_expr
    #          | arith_expr
    GT::GrammarRule.new(
      name: "for_elem",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "STEP", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "UNTIL", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "WHILE", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "arith_expr", is_token: false),
      ]),
      line_number: 167,
    ),
    # expression = arith_expr | bool_expr
    GT::GrammarRule.new(
      name: "expression",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "arith_expr", is_token: false),
        GT::RuleReference.new(name: "bool_expr", is_token: false),
      ]),
      line_number: 177,
    ),
    # arith_expr = IF bool_expr THEN simple_arith ELSE arith_expr | simple_arith
    GT::GrammarRule.new(
      name: "arith_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "IF", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::RuleReference.new(name: "THEN", is_token: true),
          GT::RuleReference.new(name: "simple_arith", is_token: false),
          GT::RuleReference.new(name: "ELSE", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "simple_arith", is_token: false),
      ]),
      line_number: 182,
    ),
    # simple_arith = [ PLUS | MINUS ] term { ( PLUS | MINUS ) term }
    GT::GrammarRule.new(
      name: "simple_arith",
      body: GT::Sequence.new(elements: [
        GT::OptionalElement.new(element: GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "PLUS", is_token: true),
          GT::RuleReference.new(name: "MINUS", is_token: true),
        ])),
        GT::RuleReference.new(name: "term", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "PLUS", is_token: true),
            GT::RuleReference.new(name: "MINUS", is_token: true),
          ]),
          GT::RuleReference.new(name: "term", is_token: false),
        ])),
      ]),
      line_number: 186,
    ),
    # term = factor { ( STAR | SLASH | DIV | MOD ) factor }
    GT::GrammarRule.new(
      name: "term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "factor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "STAR", is_token: true),
            GT::RuleReference.new(name: "SLASH", is_token: true),
            GT::RuleReference.new(name: "DIV", is_token: true),
            GT::RuleReference.new(name: "MOD", is_token: true),
          ]),
          GT::RuleReference.new(name: "factor", is_token: false),
        ])),
      ]),
      line_number: 191,
    ),
    # factor = primary { ( CARET | POWER ) primary }
    GT::GrammarRule.new(
      name: "factor",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "primary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::Alternation.new(choices: [
            GT::RuleReference.new(name: "CARET", is_token: true),
            GT::RuleReference.new(name: "POWER", is_token: true),
          ]),
          GT::RuleReference.new(name: "primary", is_token: false),
        ])),
      ]),
      line_number: 197,
    ),
    # primary = INTEGER_LIT | REAL_LIT | STRING_LIT | variable | proc_call
    #         | LPAREN arith_expr RPAREN
    GT::GrammarRule.new(
      name: "primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "INTEGER_LIT", is_token: true),
        GT::RuleReference.new(name: "REAL_LIT", is_token: true),
        GT::RuleReference.new(name: "STRING_LIT", is_token: true),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::RuleReference.new(name: "proc_call", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
      ]),
      line_number: 199,
    ),
    # bool_expr = IF bool_expr THEN simple_bool ELSE bool_expr | simple_bool
    GT::GrammarRule.new(
      name: "bool_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "IF", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::RuleReference.new(name: "THEN", is_token: true),
          GT::RuleReference.new(name: "simple_bool", is_token: false),
          GT::RuleReference.new(name: "ELSE", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "simple_bool", is_token: false),
      ]),
      line_number: 215,
    ),
    # simple_bool = implication { EQV implication }
    GT::GrammarRule.new(
      name: "simple_bool",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "implication", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "EQV", is_token: true),
          GT::RuleReference.new(name: "implication", is_token: false),
        ])),
      ]),
      line_number: 218,
    ),
    # implication = bool_term { IMPL bool_term }
    GT::GrammarRule.new(
      name: "implication",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bool_term", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "IMPL", is_token: true),
          GT::RuleReference.new(name: "bool_term", is_token: false),
        ])),
      ]),
      line_number: 220,
    ),
    # bool_term = bool_factor { OR bool_factor }
    GT::GrammarRule.new(
      name: "bool_term",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bool_factor", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "OR", is_token: true),
          GT::RuleReference.new(name: "bool_factor", is_token: false),
        ])),
      ]),
      line_number: 222,
    ),
    # bool_factor = bool_secondary { AND bool_secondary }
    GT::GrammarRule.new(
      name: "bool_factor",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "bool_secondary", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "AND", is_token: true),
          GT::RuleReference.new(name: "bool_secondary", is_token: false),
        ])),
      ]),
      line_number: 224,
    ),
    # bool_secondary = NOT bool_secondary | bool_primary
    GT::GrammarRule.new(
      name: "bool_secondary",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NOT", is_token: true),
          GT::RuleReference.new(name: "bool_secondary", is_token: false),
        ]),
        GT::RuleReference.new(name: "bool_primary", is_token: false),
      ]),
      line_number: 226,
    ),
    # bool_primary = TRUE | FALSE | variable | proc_call
    #              | LPAREN bool_expr RPAREN | relation
    GT::GrammarRule.new(
      name: "bool_primary",
      body: GT::Alternation.new(choices: [
        GT::RuleReference.new(name: "TRUE", is_token: true),
        GT::RuleReference.new(name: "FALSE", is_token: true),
        GT::RuleReference.new(name: "variable", is_token: false),
        GT::RuleReference.new(name: "proc_call", is_token: false),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "relation", is_token: false),
      ]),
      line_number: 228,
    ),
    # relation = simple_arith ( EQ | NEQ | LT | LEQ | GT | GEQ ) simple_arith
    GT::GrammarRule.new(
      name: "relation",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "simple_arith", is_token: false),
        GT::Alternation.new(choices: [
          GT::RuleReference.new(name: "EQ", is_token: true),
          GT::RuleReference.new(name: "NEQ", is_token: true),
          GT::RuleReference.new(name: "LT", is_token: true),
          GT::RuleReference.new(name: "LEQ", is_token: true),
          GT::RuleReference.new(name: "GT", is_token: true),
          GT::RuleReference.new(name: "GEQ", is_token: true),
        ]),
        GT::RuleReference.new(name: "simple_arith", is_token: false),
      ]),
      line_number: 238,
    ),
    # desig_expr = IF bool_expr THEN simple_desig ELSE desig_expr | simple_desig
    GT::GrammarRule.new(
      name: "desig_expr",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "IF", is_token: true),
          GT::RuleReference.new(name: "bool_expr", is_token: false),
          GT::RuleReference.new(name: "THEN", is_token: true),
          GT::RuleReference.new(name: "simple_desig", is_token: false),
          GT::RuleReference.new(name: "ELSE", is_token: true),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
        ]),
        GT::RuleReference.new(name: "simple_desig", is_token: false),
      ]),
      line_number: 243,
    ),
    # simple_desig = IDENT LBRACKET arith_expr RBRACKET
    #              | LPAREN desig_expr RPAREN
    #              | label
    GT::GrammarRule.new(
      name: "simple_desig",
      body: GT::Alternation.new(choices: [
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "NAME", is_token: true),
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ]),
        GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LPAREN", is_token: true),
          GT::RuleReference.new(name: "desig_expr", is_token: false),
          GT::RuleReference.new(name: "RPAREN", is_token: true),
        ]),
        GT::RuleReference.new(name: "label", is_token: false),
      ]),
      line_number: 246,
    ),
    # variable = IDENT [ LBRACKET subscripts RBRACKET ]
    GT::GrammarRule.new(
      name: "variable",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::OptionalElement.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "LBRACKET", is_token: true),
          GT::RuleReference.new(name: "subscripts", is_token: false),
          GT::RuleReference.new(name: "RBRACKET", is_token: true),
        ])),
      ]),
      line_number: 258,
    ),
    # subscripts = arith_expr { COMMA arith_expr }
    GT::GrammarRule.new(
      name: "subscripts",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "arith_expr", is_token: false),
        GT::Repetition.new(element: GT::Sequence.new(elements: [
          GT::RuleReference.new(name: "COMMA", is_token: true),
          GT::RuleReference.new(name: "arith_expr", is_token: false),
        ])),
      ]),
      line_number: 260,
    ),
    # proc_call = IDENT LPAREN actual_params RPAREN
    GT::GrammarRule.new(
      name: "proc_call",
      body: GT::Sequence.new(elements: [
        GT::RuleReference.new(name: "NAME", is_token: true),
        GT::RuleReference.new(name: "LPAREN", is_token: true),
        GT::RuleReference.new(name: "actual_params", is_token: false),
        GT::RuleReference.new(name: "RPAREN", is_token: true),
      ]),
      line_number: 265,
    ),
  ],
)
