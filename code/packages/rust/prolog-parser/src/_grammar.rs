// AUTO-GENERATED FILE — DO NOT EDIT
// Source: iso.grammar
// Regenerate with: grammar-tools compile-grammar iso.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
            line_number: 16,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"query_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"dcg_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"rule_statement"#.to_string() },
                GrammarElement::RuleReference { name: r#"fact_statement"#.to_string() },
            ] },
            line_number: 18,
        },
        GrammarRule {
            name: r#"query_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"QUERY"#.to_string() },
                GrammarElement::RuleReference { name: r#"goal"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 19,
        },
        GrammarRule {
            name: r#"dcg_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"callable_term"#.to_string() },
                GrammarElement::TokenReference { name: r#"DCG"#.to_string() },
                GrammarElement::RuleReference { name: r#"dcg_body"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 20,
        },
        GrammarRule {
            name: r#"rule_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"callable_term"#.to_string() },
                GrammarElement::TokenReference { name: r#"RULE"#.to_string() },
                GrammarElement::RuleReference { name: r#"goal"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 21,
        },
        GrammarRule {
            name: r#"fact_statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"callable_term"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 22,
        },
        GrammarRule {
            name: r#"goal"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"disjunction"#.to_string() },
            line_number: 24,
        },
        GrammarRule {
            name: r#"disjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"conjunction"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"conjunction"#.to_string() },
                    ] }) },
            ] },
            line_number: 25,
        },
        GrammarRule {
            name: r#"conjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"goal_primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"goal_primary"#.to_string() },
                    ] }) },
            ] },
            line_number: 26,
        },
        GrammarRule {
            name: r#"goal_primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"CUT"#.to_string() },
                GrammarElement::RuleReference { name: r#"grouped_goal"#.to_string() },
                GrammarElement::RuleReference { name: r#"naf_goal"#.to_string() },
                GrammarElement::RuleReference { name: r#"equality_goal"#.to_string() },
                GrammarElement::RuleReference { name: r#"callable_goal"#.to_string() },
            ] },
            line_number: 27,
        },
        GrammarRule {
            name: r#"grouped_goal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"goal"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 28,
        },
        GrammarRule {
            name: r#"naf_goal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAF"#.to_string() },
                GrammarElement::RuleReference { name: r#"goal_primary"#.to_string() },
            ] },
            line_number: 29,
        },
        GrammarRule {
            name: r#"equality_goal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::RuleReference { name: r#"equality_operator"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
            ] },
            line_number: 30,
        },
        GrammarRule {
            name: r#"callable_goal"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"callable_term"#.to_string() },
            line_number: 31,
        },
        GrammarRule {
            name: r#"dcg_body"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"dcg_disjunction"#.to_string() },
            line_number: 33,
        },
        GrammarRule {
            name: r#"dcg_disjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"dcg_conjunction"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"dcg_conjunction"#.to_string() },
                    ] }) },
            ] },
            line_number: 34,
        },
        GrammarRule {
            name: r#"dcg_conjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"dcg_primary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"dcg_primary"#.to_string() },
                    ] }) },
            ] },
            line_number: 35,
        },
        GrammarRule {
            name: r#"dcg_primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"CUT"#.to_string() },
                GrammarElement::RuleReference { name: r#"grouped_dcg_body"#.to_string() },
                GrammarElement::RuleReference { name: r#"braced_goal"#.to_string() },
                GrammarElement::RuleReference { name: r#"equality_goal"#.to_string() },
                GrammarElement::RuleReference { name: r#"list_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"callable_goal"#.to_string() },
            ] },
            line_number: 36,
        },
        GrammarRule {
            name: r#"grouped_dcg_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"dcg_body"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 38,
        },
        GrammarRule {
            name: r#"braced_goal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LCURLY"#.to_string() },
                GrammarElement::RuleReference { name: r#"goal"#.to_string() },
                GrammarElement::TokenReference { name: r#"RCURLY"#.to_string() },
            ] },
            line_number: 39,
        },
        GrammarRule {
            name: r#"equality_operator"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"="#.to_string() },
                GrammarElement::Literal { value: r#"\="#.to_string() },
            ] },
            line_number: 41,
        },
        GrammarRule {
            name: r#"callable_term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"compound_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"atom_term"#.to_string() },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"list_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"compound_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"atom_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"variable_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"anonymous_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"number_term"#.to_string() },
                GrammarElement::RuleReference { name: r#"string_term"#.to_string() },
            ] },
            line_number: 44,
        },
        GrammarRule {
            name: r#"compound_term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"atom_token"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"term_arguments"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 47,
        },
        GrammarRule {
            name: r#"term_arguments"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
            ] },
            line_number: 48,
        },
        GrammarRule {
            name: r#"list_term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"list_body"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 50,
        },
        GrammarRule {
            name: r#"list_body"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"BAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
            ] },
            line_number: 51,
        },
        GrammarRule {
            name: r#"atom_term"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"atom_token"#.to_string() },
            line_number: 53,
        },
        GrammarRule {
            name: r#"atom_token"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"ATOM"#.to_string() },
            line_number: 54,
        },
        GrammarRule {
            name: r#"variable_term"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"VARIABLE"#.to_string() },
            line_number: 55,
        },
        GrammarRule {
            name: r#"anonymous_term"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"ANON_VAR"#.to_string() },
            line_number: 56,
        },
        GrammarRule {
            name: r#"number_term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"FLOAT"#.to_string() },
                GrammarElement::TokenReference { name: r#"INTEGER"#.to_string() },
            ] },
            line_number: 57,
        },
        GrammarRule {
            name: r#"string_term"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            line_number: 58,
        },
    ],
        version: 1,
    }
}
