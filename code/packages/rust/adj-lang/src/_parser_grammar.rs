// AUTO-GENERATED FILE — DO NOT EDIT
// Source: adj_lang.grammar
// Regenerate with: grammar-tools compile-grammar adj_lang.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"EOF"#.to_string() },
            ] },
            line_number: 20,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"prior_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"contributes_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"interacts_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"uncertain_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"observe_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"relate_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"rule_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"functional_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"context_order_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"query_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"let_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"symbol_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"constrain_latex_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"constrain_asciimath_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"constrain_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"solve_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"check_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"optimize_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"dictionary_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"define_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"rulebook_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"formulabook_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"table_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"statemachine_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"use_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"import_decl"#.to_string() },
            ] },
            line_number: 22,
        },
        GrammarRule {
            name: r#"prior_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"prior"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 55,
        },
        GrammarRule {
            name: r#"contributes_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"contributes"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"from"#.to_string() },
                GrammarElement::RuleReference { name: r#"evidence"#.to_string() },
                GrammarElement::Literal { value: r#"to"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 57,
        },
        GrammarRule {
            name: r#"evidence"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"predicate"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 68,
        },
        GrammarRule {
            name: r#"predicate"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"apply"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQEQ"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 78,
        },
        GrammarRule {
            name: r#"interacts_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"interacts"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"when"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Literal { value: r#"and"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 80,
        },
        GrammarRule {
            name: r#"uncertain_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"uncertain"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 82,
        },
        GrammarRule {
            name: r#"observe_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"observe"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 84,
        },
        GrammarRule {
            name: r#"relate_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"relate"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 97,
        },
        GrammarRule {
            name: r#"rule_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"rule"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Literal { value: r#"head"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Literal { value: r#"when"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"body_literal"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"body_literal"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"priority"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"context"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 119,
        },
        GrammarRule {
            name: r#"body_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"not"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 121,
        },
        GrammarRule {
            name: r#"context_order_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"context_order"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 132,
        },
        GrammarRule {
            name: r#"functional_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"functional"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 143,
        },
        GrammarRule {
            name: r#"query_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"QUESTION"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"lookup_expr"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
            ] },
            line_number: 145,
        },
        GrammarRule {
            name: r#"lookup_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"lookup"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"signed_number"#.to_string() },
                GrammarElement::Literal { value: r#"mode"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Literal { value: r#"give"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
            ] },
            line_number: 156,
        },
        GrammarRule {
            name: r#"signed_number"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"MINUS"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 157,
        },
        GrammarRule {
            name: r#"let_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"let"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 171,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term_expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"term_expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 173,
        },
        GrammarRule {
            name: r#"term_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"factor"#.to_string() },
                    ] }) },
            ] },
            line_number: 175,
        },
        GrammarRule {
            name: r#"factor"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"latex_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"asciimath_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"mathml_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"unicodemath_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"agg"#.to_string() },
                GrammarElement::RuleReference { name: r#"apply"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 177,
        },
        GrammarRule {
            name: r#"agg"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"sum"#.to_string() },
                        GrammarElement::Literal { value: r#"count"#.to_string() },
                        GrammarElement::Literal { value: r#"min"#.to_string() },
                        GrammarElement::Literal { value: r#"max"#.to_string() },
                        GrammarElement::Literal { value: r#"avg"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 179,
        },
        GrammarRule {
            name: r#"apply"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 204,
        },
        GrammarRule {
            name: r#"latex_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"latex"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 211,
        },
        GrammarRule {
            name: r#"asciimath_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"asciimath"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 222,
        },
        GrammarRule {
            name: r#"mathml_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"mathml"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 234,
        },
        GrammarRule {
            name: r#"unicodemath_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"unicodemath"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 246,
        },
        GrammarRule {
            name: r#"symbol_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"symbol"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 256,
        },
        GrammarRule {
            name: r#"constrain_latex_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"constrain"#.to_string() },
                GrammarElement::RuleReference { name: r#"latex_relation"#.to_string() },
            ] },
            line_number: 258,
        },
        GrammarRule {
            name: r#"constrain_asciimath_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"constrain"#.to_string() },
                GrammarElement::RuleReference { name: r#"asciimath_relation"#.to_string() },
            ] },
            line_number: 268,
        },
        GrammarRule {
            name: r#"constrain_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"constrain"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"relop"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 270,
        },
        GrammarRule {
            name: r#"latex_relation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"latex"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 272,
        },
        GrammarRule {
            name: r#"asciimath_relation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"asciimath"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 274,
        },
        GrammarRule {
            name: r#"relop"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQEQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"NE"#.to_string() },
            ] },
            line_number: 276,
        },
        GrammarRule {
            name: r#"solve_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"solve"#.to_string() },
                GrammarElement::Literal { value: r#"for"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 278,
        },
        GrammarRule {
            name: r#"check_decl"#.to_string(),
            body: GrammarElement::Literal { value: r#"check"#.to_string() },
            line_number: 280,
        },
        GrammarRule {
            name: r#"optimize_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"minimize"#.to_string() },
                        GrammarElement::Literal { value: r#"maximize"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 287,
        },
        GrammarRule {
            name: r#"dictionary_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"dictionary"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"define_decl"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 299,
        },
        GrammarRule {
            name: r#"define_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"define"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"define_kind"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"surface_clause"#.to_string() }) },
            ] },
            line_number: 301,
        },
        GrammarRule {
            name: r#"define_kind"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"hypothesis"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"finding"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"values"#.to_string() },
                            GrammarElement::TokenReference { name: r#"LBRACK"#.to_string() },
                            GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                                ] }) },
                            GrammarElement::TokenReference { name: r#"RBRACK"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Literal { value: r#"entity"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"relation"#.to_string() },
                    GrammarElement::Literal { value: r#"from"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    GrammarElement::Literal { value: r#"to"#.to_string() },
                    GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                ] },
            ] },
            line_number: 303,
        },
        GrammarRule {
            name: r#"surface_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"surface"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    ] }) },
            ] },
            line_number: 309,
        },
        GrammarRule {
            name: r#"rulebook_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"rulebook"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 326,
        },
        GrammarRule {
            name: r#"use_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"use"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
            ] },
            line_number: 328,
        },
        GrammarRule {
            name: r#"formulabook_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"formulabook"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"formulabook_item"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 359,
        },
        GrammarRule {
            name: r#"formulabook_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"use_decl"#.to_string() },
                GrammarElement::RuleReference { name: r#"formula_decl"#.to_string() },
            ] },
            line_number: 361,
        },
        GrammarRule {
            name: r#"formula_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"formula"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"formula_params"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"formula_body"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
            ] },
            line_number: 363,
        },
        GrammarRule {
            name: r#"formula_params"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
            ] },
            line_number: 365,
        },
        GrammarRule {
            name: r#"formula_body"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"formula_step"#.to_string() }) },
                    GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                ] },
            ] },
            line_number: 378,
        },
        GrammarRule {
            name: r#"formula_step"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"let"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 380,
        },
        GrammarRule {
            name: r#"table_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"table"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"use_decl"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"columns_decl"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"table_row"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 414,
        },
        GrammarRule {
            name: r#"columns_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"columns"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
            ] },
            line_number: 416,
        },
        GrammarRule {
            name: r#"table_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"row"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"row_item"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"row_item"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
                    ] }) },
            ] },
            line_number: 424,
        },
        GrammarRule {
            name: r#"row_item"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 426,
        },
        GrammarRule {
            name: r#"statemachine_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"statemachine"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"use_decl"#.to_string() }) },
                GrammarElement::Literal { value: r#"initial"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sm_state"#.to_string() }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sm_exit"#.to_string() }) },
                GrammarElement::Literal { value: r#"budget"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"steps"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"annotation"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 439,
        },
        GrammarRule {
            name: r#"sm_state"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"state"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sm_transition"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 448,
        },
        GrammarRule {
            name: r#"sm_transition"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"transition"#.to_string() },
                GrammarElement::Literal { value: r#"on"#.to_string() },
                GrammarElement::RuleReference { name: r#"sm_guard"#.to_string() },
                GrammarElement::Literal { value: r#"to"#.to_string() },
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"do"#.to_string() },
                        GrammarElement::RuleReference { name: r#"sm_action"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"sm_action"#.to_string() },
                            ] }) },
                    ] }) },
            ] },
            line_number: 455,
        },
        GrammarRule {
            name: r#"sm_exit"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"exit"#.to_string() },
                GrammarElement::Literal { value: r#"when"#.to_string() },
                GrammarElement::RuleReference { name: r#"sm_guard"#.to_string() },
                GrammarElement::Literal { value: r#"yield"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            ] },
            line_number: 457,
        },
        GrammarRule {
            name: r#"sm_guard"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"apply"#.to_string() },
                        GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                                GrammarElement::TokenReference { name: r#"EQEQ"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 459,
        },
        GrammarRule {
            name: r#"sm_action"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"assert"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 461,
        },
        GrammarRule {
            name: r#"import_decl"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"import"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 477,
        },
        GrammarRule {
            name: r#"annotation"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"source_annotation"#.to_string() },
                GrammarElement::RuleReference { name: r#"locator_annotation"#.to_string() },
                GrammarElement::RuleReference { name: r#"trust_annotation"#.to_string() },
                GrammarElement::RuleReference { name: r#"cites_annotation"#.to_string() },
                GrammarElement::RuleReference { name: r#"quote_annotation"#.to_string() },
            ] },
            line_number: 489,
        },
        GrammarRule {
            name: r#"source_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"source"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 496,
        },
        GrammarRule {
            name: r#"locator_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"locator"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 497,
        },
        GrammarRule {
            name: r#"trust_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"trust"#.to_string() },
                GrammarElement::RuleReference { name: r#"trust_tier"#.to_string() },
            ] },
            line_number: 498,
        },
        GrammarRule {
            name: r#"cites_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"cites"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Literal { value: r#"locator"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 499,
        },
        GrammarRule {
            name: r#"quote_annotation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"quote"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Literal { value: r#"at"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Literal { value: r#"snapshot"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 507,
        },
        GrammarRule {
            name: r#"trust_tier"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"consensus"#.to_string() },
                GrammarElement::Literal { value: r#"authoritative"#.to_string() },
                GrammarElement::Literal { value: r#"empirical"#.to_string() },
                GrammarElement::Literal { value: r#"inferred"#.to_string() },
                GrammarElement::Literal { value: r#"unattributed"#.to_string() },
            ] },
            line_number: 509,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"IDENT"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                                GrammarElement::TokenReference { name: r#"VAR"#.to_string() },
                            ] }) },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                                        GrammarElement::TokenReference { name: r#"VAR"#.to_string() },
                                    ] }) },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 531,
        },
    ],
        version: 1,
    }
}
