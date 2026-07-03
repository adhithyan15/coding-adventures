// AUTO-GENERATED FILE — DO NOT EDIT
// Source: wolfram.grammar
// Regenerate with: grammar-tools compile-grammar wolfram.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"program"#.to_string(),
            body: GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement_line"#.to_string() }) },
            line_number: 51,
        },
        GrammarRule {
            name: r#"statement_line"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"SEMI"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"SEMI"#.to_string() },
            ] },
            line_number: 55,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"expr"#.to_string() },
            line_number: 60,
        },
        GrammarRule {
            name: r#"expr"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
            line_number: 62,
        },
        GrammarRule {
            name: r#"assignment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"replaceall"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"SET"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SETDELAYED"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"assignment"#.to_string() },
                    ] }) },
            ] },
            line_number: 65,
        },
        GrammarRule {
            name: r#"replaceall"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"rule"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"REPLACEALL"#.to_string() },
                                GrammarElement::TokenReference { name: r#"REPLACEREPEATED"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"rule"#.to_string() },
                    ] }) },
            ] },
            line_number: 73,
        },
        GrammarRule {
            name: r#"rule"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"condition"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"RULE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"RULEDELAYED"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"rule"#.to_string() },
                    ] }) },
            ] },
            line_number: 77,
        },
        GrammarRule {
            name: r#"condition"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"alternatives"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"CONDITION"#.to_string() },
                        GrammarElement::RuleReference { name: r#"condition"#.to_string() },
                    ] }) },
            ] },
            line_number: 83,
        },
        GrammarRule {
            name: r#"alternatives"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"ALTERNATIVES"#.to_string() },
                        GrammarElement::RuleReference { name: r#"logical_or"#.to_string() },
                    ] }) },
            ] },
            line_number: 88,
        },
        GrammarRule {
            name: r#"logical_or"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"OR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"logical_and"#.to_string() },
                    ] }) },
            ] },
            line_number: 90,
        },
        GrammarRule {
            name: r#"logical_and"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"AND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                    ] }) },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"logical_not"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NOT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"logical_not"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"amp"#.to_string() },
            ] },
            line_number: 92,
        },
        GrammarRule {
            name: r#"amp"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                    GrammarElement::TokenReference { name: r#"AMP"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"AMP"#.to_string() }) },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"amp_apply"#.to_string() }) },
                ] },
                GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
            ] },
            line_number: 115,
        },
        GrammarRule {
            name: r#"amp_apply"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arglist"#.to_string() }) },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LDBRACKET"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arglist"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                ] },
            ] },
            line_number: 120,
        },
        GrammarRule {
            name: r#"comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"EQUAL"#.to_string() },
                                GrammarElement::TokenReference { name: r#"UNEQUAL"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LESS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GREATER"#.to_string() },
                                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"additive"#.to_string() },
                    ] }) },
            ] },
            line_number: 124,
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
            line_number: 126,
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
            line_number: 127,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"power"#.to_string() },
            ] },
            line_number: 130,
        },
        GrammarRule {
            name: r#"power"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"mapapply"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"POWER"#.to_string() },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 133,
        },
        GrammarRule {
            name: r#"mapapply"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"patterntest"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"MAP"#.to_string() },
                                GrammarElement::TokenReference { name: r#"APPLY"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"patterntest"#.to_string() },
                    ] }) },
            ] },
            line_number: 142,
        },
        GrammarRule {
            name: r#"patterntest"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PATTERNTEST"#.to_string() },
                        GrammarElement::RuleReference { name: r#"postfix"#.to_string() },
                    ] }) },
            ] },
            line_number: 149,
        },
        GrammarRule {
            name: r#"postfix"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"atom"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arglist"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LDBRACKET"#.to_string() },
                            GrammarElement::RuleReference { name: r#"arglist"#.to_string() },
                            GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                            GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
                        ] },
                    ] }) },
            ] },
            line_number: 163,
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
            line_number: 165,
        },
        GrammarRule {
            name: r#"atom"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"slot"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"BLANK"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"BLANK"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                ] },
                GrammarElement::RuleReference { name: r#"list"#.to_string() },
                GrammarElement::RuleReference { name: r#"group"#.to_string() },
            ] },
            line_number: 175,
        },
        GrammarRule {
            name: r#"slot"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"HASH"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() }) },
                ] },
                GrammarElement::TokenReference { name: r#"SLOTSEQ"#.to_string() },
            ] },
            line_number: 191,
        },
        GrammarRule {
            name: r#"list"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"arglist"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RBRACE"#.to_string() },
            ] },
            line_number: 193,
        },
        GrammarRule {
            name: r#"group"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"expr"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 194,
        },
    ],
        version: 1,
    }
}
