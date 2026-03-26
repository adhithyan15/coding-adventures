// AUTO-GENERATED FILE - DO NOT EDIT
#![allow(clippy::all)]
use grammar_tools::parser_grammar::{ParserGrammar, GrammarRule, GrammarElement};

pub fn StarlarkGrammar() -> ParserGrammar {
    ParserGrammar {
        version: 1,
        rules: vec![
            GrammarRule {
                name: "file".to_string(),
                line_number: 34,
                body: GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "NEWLINE".to_string() }, GrammarElement::RuleReference { name: "statement".to_string() }] }) },
            },
            GrammarRule {
                name: "statement".to_string(),
                line_number: 48,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "compound_stmt".to_string() }, GrammarElement::RuleReference { name: "simple_stmt".to_string() }] },
            },
            GrammarRule {
                name: "simple_stmt".to_string(),
                line_number: 52,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "small_stmt".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "SEMICOLON".to_string() }, GrammarElement::RuleReference { name: "small_stmt".to_string() }] }) }, GrammarElement::TokenReference { name: "NEWLINE".to_string() }] },
            },
            GrammarRule {
                name: "small_stmt".to_string(),
                line_number: 54,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "return_stmt".to_string() }, GrammarElement::RuleReference { name: "break_stmt".to_string() }, GrammarElement::RuleReference { name: "continue_stmt".to_string() }, GrammarElement::RuleReference { name: "pass_stmt".to_string() }, GrammarElement::RuleReference { name: "load_stmt".to_string() }, GrammarElement::RuleReference { name: "assign_stmt".to_string() }] },
            },
            GrammarRule {
                name: "return_stmt".to_string(),
                line_number: 68,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "return".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) }] },
            },
            GrammarRule {
                name: "break_stmt".to_string(),
                line_number: 71,
                body: GrammarElement::Literal { value: "break".to_string() },
            },
            GrammarRule {
                name: "continue_stmt".to_string(),
                line_number: 74,
                body: GrammarElement::Literal { value: "continue".to_string() },
            },
            GrammarRule {
                name: "pass_stmt".to_string(),
                line_number: 79,
                body: GrammarElement::Literal { value: "pass".to_string() },
            },
            GrammarRule {
                name: "load_stmt".to_string(),
                line_number: 88,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "load".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "load_arg".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "load_arg".to_string(),
                line_number: 89,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::TokenReference { name: "STRING".to_string() }] }, GrammarElement::TokenReference { name: "STRING".to_string() }] },
            },
            GrammarRule {
                name: "assign_stmt".to_string(),
                line_number: 110,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression_list".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "assign_op".to_string() }, GrammarElement::RuleReference { name: "augmented_assign_op".to_string() }] }) }, GrammarElement::RuleReference { name: "expression_list".to_string() }] }) }] },
            },
            GrammarRule {
                name: "assign_op".to_string(),
                line_number: 113,
                body: GrammarElement::TokenReference { name: "EQUALS".to_string() },
            },
            GrammarRule {
                name: "augmented_assign_op".to_string(),
                line_number: 115,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "MINUS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "STAR_EQUALS".to_string() }, GrammarElement::TokenReference { name: "SLASH_EQUALS".to_string() }, GrammarElement::TokenReference { name: "FLOOR_DIV_EQUALS".to_string() }, GrammarElement::TokenReference { name: "PERCENT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "AMP_EQUALS".to_string() }, GrammarElement::TokenReference { name: "PIPE_EQUALS".to_string() }, GrammarElement::TokenReference { name: "CARET_EQUALS".to_string() }, GrammarElement::TokenReference { name: "LEFT_SHIFT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "RIGHT_SHIFT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "DOUBLE_STAR_EQUALS".to_string() }] },
            },
            GrammarRule {
                name: "compound_stmt".to_string(),
                line_number: 124,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "if_stmt".to_string() }, GrammarElement::RuleReference { name: "for_stmt".to_string() }, GrammarElement::RuleReference { name: "def_stmt".to_string() }] },
            },
            GrammarRule {
                name: "if_stmt".to_string(),
                line_number: 136,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "suite".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "elif".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "suite".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "else".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "suite".to_string() }] }) }] },
            },
            GrammarRule {
                name: "for_stmt".to_string(),
                line_number: 150,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "for".to_string() }, GrammarElement::RuleReference { name: "loop_vars".to_string() }, GrammarElement::Literal { value: "in".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "suite".to_string() }] },
            },
            GrammarRule {
                name: "loop_vars".to_string(),
                line_number: 156,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }) }] },
            },
            GrammarRule {
                name: "def_stmt".to_string(),
                line_number: 166,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "def".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "parameters".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "suite".to_string() }] },
            },
            GrammarRule {
                name: "suite".to_string(),
                line_number: 177,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "simple_stmt".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NEWLINE".to_string() }, GrammarElement::TokenReference { name: "INDENT".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "statement".to_string() }) }, GrammarElement::TokenReference { name: "DEDENT".to_string() }] }] },
            },
            GrammarRule {
                name: "parameters".to_string(),
                line_number: 198,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "parameter".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "parameter".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] },
            },
            GrammarRule {
                name: "parameter".to_string(),
                line_number: 200,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOUBLE_STAR".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }, GrammarElement::TokenReference { name: "NAME".to_string() }] },
            },
            GrammarRule {
                name: "expression_list".to_string(),
                line_number: 234,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] },
            },
            GrammarRule {
                name: "expression".to_string(),
                line_number: 239,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "lambda_expr".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "or_expr".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::RuleReference { name: "or_expr".to_string() }, GrammarElement::Literal { value: "else".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }] },
            },
            GrammarRule {
                name: "lambda_expr".to_string(),
                line_number: 244,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "lambda".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "lambda_params".to_string() }) }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "lambda_params".to_string(),
                line_number: 245,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "lambda_param".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "lambda_param".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] },
            },
            GrammarRule {
                name: "lambda_param".to_string(),
                line_number: 246,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOUBLE_STAR".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }] },
            },
            GrammarRule {
                name: "or_expr".to_string(),
                line_number: 250,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "and_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "or".to_string() }, GrammarElement::RuleReference { name: "and_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "and_expr".to_string(),
                line_number: 254,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "not_expr".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "and".to_string() }, GrammarElement::RuleReference { name: "not_expr".to_string() }] }) }] },
            },
            GrammarRule {
                name: "not_expr".to_string(),
                line_number: 258,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "not".to_string() }, GrammarElement::RuleReference { name: "not_expr".to_string() }] }, GrammarElement::RuleReference { name: "comparison".to_string() }] },
            },
            GrammarRule {
                name: "comparison".to_string(),
                line_number: 267,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "bitwise_or".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "comp_op".to_string() }, GrammarElement::RuleReference { name: "bitwise_or".to_string() }] }) }] },
            },
            GrammarRule {
                name: "comp_op".to_string(),
                line_number: 269,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "EQUALS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "NOT_EQUALS".to_string() }, GrammarElement::TokenReference { name: "LESS_THAN".to_string() }, GrammarElement::TokenReference { name: "GREATER_THAN".to_string() }, GrammarElement::TokenReference { name: "LESS_EQUALS".to_string() }, GrammarElement::TokenReference { name: "GREATER_EQUALS".to_string() }, GrammarElement::Literal { value: "in".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "not".to_string() }, GrammarElement::Literal { value: "in".to_string() }] }] },
            },
            GrammarRule {
                name: "bitwise_or".to_string(),
                line_number: 275,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "bitwise_xor".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "PIPE".to_string() }, GrammarElement::RuleReference { name: "bitwise_xor".to_string() }] }) }] },
            },
            GrammarRule {
                name: "bitwise_xor".to_string(),
                line_number: 276,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "bitwise_and".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "CARET".to_string() }, GrammarElement::RuleReference { name: "bitwise_and".to_string() }] }) }] },
            },
            GrammarRule {
                name: "bitwise_and".to_string(),
                line_number: 277,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "shift".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "AMP".to_string() }, GrammarElement::RuleReference { name: "shift".to_string() }] }) }] },
            },
            GrammarRule {
                name: "shift".to_string(),
                line_number: 280,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "arith".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "LEFT_SHIFT".to_string() }, GrammarElement::TokenReference { name: "RIGHT_SHIFT".to_string() }] }) }, GrammarElement::RuleReference { name: "arith".to_string() }] }) }] },
            },
            GrammarRule {
                name: "arith".to_string(),
                line_number: 284,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "term".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }] }) }, GrammarElement::RuleReference { name: "term".to_string() }] }) }] },
            },
            GrammarRule {
                name: "term".to_string(),
                line_number: 289,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "factor".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::TokenReference { name: "SLASH".to_string() }, GrammarElement::TokenReference { name: "FLOOR_DIV".to_string() }, GrammarElement::TokenReference { name: "PERCENT".to_string() }] }) }, GrammarElement::RuleReference { name: "factor".to_string() }] }) }] },
            },
            GrammarRule {
                name: "factor".to_string(),
                line_number: 295,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "PLUS".to_string() }, GrammarElement::TokenReference { name: "MINUS".to_string() }, GrammarElement::TokenReference { name: "TILDE".to_string() }] }) }, GrammarElement::RuleReference { name: "factor".to_string() }] }, GrammarElement::RuleReference { name: "power".to_string() }] },
            },
            GrammarRule {
                name: "power".to_string(),
                line_number: 303,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "primary".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOUBLE_STAR".to_string() }, GrammarElement::RuleReference { name: "factor".to_string() }] }) }] },
            },
            GrammarRule {
                name: "primary".to_string(),
                line_number: 320,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "atom".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: "suffix".to_string() }) }] },
            },
            GrammarRule {
                name: "suffix".to_string(),
                line_number: 322,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOT".to_string() }, GrammarElement::TokenReference { name: "NAME".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::RuleReference { name: "subscript".to_string() }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "arguments".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] }] },
            },
            GrammarRule {
                name: "subscript".to_string(),
                line_number: 334,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "expression".to_string() }) }] }) }] }] },
            },
            GrammarRule {
                name: "atom".to_string(),
                line_number: 343,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::TokenReference { name: "INT".to_string() }, GrammarElement::TokenReference { name: "FLOAT".to_string() }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "STRING".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: "STRING".to_string() }) }] }, GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::Literal { value: "True".to_string() }, GrammarElement::Literal { value: "False".to_string() }, GrammarElement::Literal { value: "None".to_string() }, GrammarElement::RuleReference { name: "list_expr".to_string() }, GrammarElement::RuleReference { name: "dict_expr".to_string() }, GrammarElement::RuleReference { name: "paren_expr".to_string() }] },
            },
            GrammarRule {
                name: "list_expr".to_string(),
                line_number: 359,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACKET".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "list_body".to_string() }) }, GrammarElement::TokenReference { name: "RBRACKET".to_string() }] },
            },
            GrammarRule {
                name: "list_body".to_string(),
                line_number: 361,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::RuleReference { name: "comp_clause".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] }] },
            },
            GrammarRule {
                name: "dict_expr".to_string(),
                line_number: 367,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LBRACE".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "dict_body".to_string() }) }, GrammarElement::TokenReference { name: "RBRACE".to_string() }] },
            },
            GrammarRule {
                name: "dict_body".to_string(),
                line_number: 369,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "dict_entry".to_string() }, GrammarElement::RuleReference { name: "comp_clause".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "dict_entry".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "dict_entry".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] }] },
            },
            GrammarRule {
                name: "dict_entry".to_string(),
                line_number: 372,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COLON".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "paren_expr".to_string(),
                line_number: 379,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "LPAREN".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: "paren_body".to_string() }) }, GrammarElement::TokenReference { name: "RPAREN".to_string() }] },
            },
            GrammarRule {
                name: "paren_body".to_string(),
                line_number: 381,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::RuleReference { name: "comp_clause".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "expression".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] }) }] }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
            GrammarRule {
                name: "comp_clause".to_string(),
                line_number: 397,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "comp_for".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![GrammarElement::RuleReference { name: "comp_for".to_string() }, GrammarElement::RuleReference { name: "comp_if".to_string() }] }) }] },
            },
            GrammarRule {
                name: "comp_for".to_string(),
                line_number: 399,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "for".to_string() }, GrammarElement::RuleReference { name: "loop_vars".to_string() }, GrammarElement::Literal { value: "in".to_string() }, GrammarElement::RuleReference { name: "or_expr".to_string() }] },
            },
            GrammarRule {
                name: "comp_if".to_string(),
                line_number: 401,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::Literal { value: "if".to_string() }, GrammarElement::RuleReference { name: "or_expr".to_string() }] },
            },
            GrammarRule {
                name: "arguments".to_string(),
                line_number: 420,
                body: GrammarElement::Sequence { elements: vec![GrammarElement::RuleReference { name: "argument".to_string() }, GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "COMMA".to_string() }, GrammarElement::RuleReference { name: "argument".to_string() }] }) }, GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: "COMMA".to_string() }) }] },
            },
            GrammarRule {
                name: "argument".to_string(),
                line_number: 422,
                body: GrammarElement::Alternation { choices: vec![GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "DOUBLE_STAR".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "STAR".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }, GrammarElement::Sequence { elements: vec![GrammarElement::TokenReference { name: "NAME".to_string() }, GrammarElement::TokenReference { name: "EQUALS".to_string() }, GrammarElement::RuleReference { name: "expression".to_string() }] }, GrammarElement::RuleReference { name: "expression".to_string() }] },
            },
        ],
    }
}
