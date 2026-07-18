// AUTO-GENERATED FILE — DO NOT EDIT
// Source: reduce.grammar
// Regenerate with: grammar-tools compile-grammar reduce.grammar
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
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement_line"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
            ] },
            line_number: 120,
        },
        GrammarRule {
            name: r#"statement_line"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"SEMI"#.to_string() },
                        GrammarElement::TokenReference { name: r#"DOLLAR"#.to_string() },
                    ] }) },
            ] },
            line_number: 129,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            line_number: 131,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"if_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"group_expr"#.to_string() },
                GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            ] },
            line_number: 138,
        },
        GrammarRule {
            name: r#"if_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Literal { value: r#"then"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"else"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 155,
        },
        GrammarRule {
            name: r#"group_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"GROUP_OPEN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"SEMI"#.to_string() },
                                GrammarElement::TokenReference { name: r#"DOLLAR"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"GROUP_CLOSE"#.to_string() },
            ] },
            line_number: 164,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"ASSIGN"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 184,
        },
        GrammarRule {
            name: r#"logical_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"or"#.to_string() },
                        GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 186,
        },
        GrammarRule {
            name: r#"logical_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                    ] }) },
            ] },
            line_number: 187,
        },
        GrammarRule {
            name: r#"logical_not"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"not"#.to_string() },
                    GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
            ] },
            line_number: 188,
        },
        GrammarRule {
            name: r#"comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"cons"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                                GrammarElement::Literal { value: r#"neq"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LESS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"cons"#.to_string() },
                    ] }) },
            ] },
            line_number: 198,
        },
        GrammarRule {
            name: r#"cons"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                        GrammarElement::RuleReference { name: r#"cons"#.to_string() },
                    ] }) },
            ] },
            line_number: 201,
        },
        GrammarRule {
            name: r#"additive"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"multiplicative"#.to_string() },
                    ] }) },
            ] },
            line_number: 203,
        },
        GrammarRule {
            name: r#"multiplicative"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"TIMES"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 204,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"power"#.to_string() },
            ] },
            line_number: 209,
        },
        GrammarRule {
            name: r#"power"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"CARET"#.to_string() },
                                GrammarElement::TokenReference { name: r#"POW"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 217,
        },
        GrammarRule {
            name: r#"postfix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arglist"#.to_string() }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 228,
        },
        GrammarRule {
            name: r#"arglist"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                    ] }) },
            ] },
            line_number: 230,
        },
        GrammarRule {
            name: r#"atom"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::RuleReference { name: r#"list_literal"#.to_string() },
                GrammarElement::RuleReference { name: r#"group"#.to_string() },
            ] },
            line_number: 232,
        },
        GrammarRule {
            name: r#"list_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arglist"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 242,
        },
        GrammarRule {
            name: r#"group"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 244,
        },
    ],
        version: 1,
    }
}
