// AUTO-GENERATED FILE — DO NOT EDIT
// Source: warrant.grammar
// Regenerate with: grammar-tools compile-grammar warrant.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"file"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"warrant"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::RuleReference { name: r#"declaration"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
            ] },
            line_number: 3,
        },
        GrammarRule {
            name: r#"declaration"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"model_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"error_declaration"#.to_string() },
                GrammarElement::RuleReference { name: r#"operation"#.to_string() },
                GrammarElement::RuleReference { name: r#"example"#.to_string() },
                GrammarElement::RuleReference { name: r#"claim"#.to_string() },
                GrammarElement::RuleReference { name: r#"assumptions"#.to_string() },
                GrammarElement::RuleReference { name: r#"labels"#.to_string() },
                GrammarElement::RuleReference { name: r#"input"#.to_string() },
                GrammarElement::RuleReference { name: r#"bound"#.to_string() },
            ] },
            line_number: 4,
        },
        GrammarRule {
            name: r#"model_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"model"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                            GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                            GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                            GrammarElement::RuleReference { name: r#"type"#.to_string() },
                            GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                            GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                        ] },
                    ] }) },
            ] },
            line_number: 6,
        },
        GrammarRule {
            name: r#"error_declaration"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"error"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 7,
        },
        GrammarRule {
            name: r#"operation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"op"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                        GrammarElement::RuleReference { name: r#"static_parameters"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"parameters"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"fresh"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 8,
        },
        GrammarRule {
            name: r#"static_parameters"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::Literal { value: r#"Nat"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                        GrammarElement::Literal { value: r#"Nat"#.to_string() },
                    ] }) },
            ] },
            line_number: 9,
        },
        GrammarRule {
            name: r#"parameters"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                    ] }) },
            ] },
            line_number: 10,
        },
        GrammarRule {
            name: r#"parameter"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"type"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 11,
        },
        GrammarRule {
            name: r#"type"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"Bytes"#.to_string() },
                    GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                            GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                            GrammarElement::RuleReference { name: r#"width"#.to_string() },
                            GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                        ] }) },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"List"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"Option"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"Result"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                    GrammarElement::RuleReference { name: r#"type"#.to_string() },
                    GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                ] },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 12,
        },
        GrammarRule {
            name: r#"width"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"INT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 13,
        },
        GrammarRule {
            name: r#"example"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"example"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
            ] },
            line_number: 15,
        },
        GrammarRule {
            name: r#"claim"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"claim"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
            ] },
            line_number: 16,
        },
        GrammarRule {
            name: r#"assumptions"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"assume"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"platform"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                            GrammarElement::RuleReference { name: r#"platform_rows"#.to_string() },
                            GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"environment"#.to_string() },
                            GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                            GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                            GrammarElement::RuleReference { name: r#"environment_rows"#.to_string() },
                            GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
                        ] },
                    ] }) },
            ] },
            line_number: 17,
        },
        GrammarRule {
            name: r#"platform_rows"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"none"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"platform_row"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"platform_row"#.to_string() }) },
                ] },
            ] },
            line_number: 18,
        },
        GrammarRule {
            name: r#"platform_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 19,
        },
        GrammarRule {
            name: r#"environment_rows"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"none"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::RuleReference { name: r#"environment_row"#.to_string() },
                    GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"environment_row"#.to_string() }) },
                ] },
            ] },
            line_number: 20,
        },
        GrammarRule {
            name: r#"environment_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 21,
        },
        GrammarRule {
            name: r#"labels"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"labels"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                GrammarElement::RuleReference { name: r#"label_row"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"label_row"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
            ] },
            line_number: 22,
        },
        GrammarRule {
            name: r#"label_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"public"#.to_string() },
                        GrammarElement::Literal { value: r#"secret"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 23,
        },
        GrammarRule {
            name: r#"input"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"input"#.to_string() },
                GrammarElement::TokenReference { name: r#"QNAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                GrammarElement::Literal { value: r#"hostile"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"rejection"#.to_string() }) },
                GrammarElement::Literal { value: r#"ensures"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
            ] },
            line_number: 24,
        },
        GrammarRule {
            name: r#"rejection"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"rejects"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"with"#.to_string() },
                GrammarElement::TokenReference { name: r#"QNAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 25,
        },
        GrammarRule {
            name: r#"bound"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"bound"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"COLON"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
                GrammarElement::TokenReference { name: r#"INDENT"#.to_string() },
                GrammarElement::RuleReference { name: r#"bound_row"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"bound_row"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"DEDENT"#.to_string() },
            ] },
            line_number: 26,
        },
        GrammarRule {
            name: r#"bound_row"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::TokenReference { name: r#"NEWLINE"#.to_string() },
            ] },
            line_number: 27,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"quantified"#.to_string() },
                GrammarElement::RuleReference { name: r#"conditional"#.to_string() },
                GrammarElement::RuleReference { name: r#"match_expression"#.to_string() },
                GrammarElement::RuleReference { name: r#"implication"#.to_string() },
            ] },
            line_number: 29,
        },
        GrammarRule {
            name: r#"quantified"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"forall"#.to_string() },
                        GrammarElement::Literal { value: r#"exists"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"binders"#.to_string() },
                GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::SpaceAfterNext },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 30,
        },
        GrammarRule {
            name: r#"binders"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"parameter"#.to_string() },
                    ] }) },
            ] },
            line_number: 31,
        },
        GrammarRule {
            name: r#"conditional"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"if"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"then"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"else"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 32,
        },
        GrammarRule {
            name: r#"match_expression"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"match"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                GrammarElement::Literal { value: r#"with"#.to_string() },
                GrammarElement::RuleReference { name: r#"match_arm"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"PIPE"#.to_string() },
                        GrammarElement::RuleReference { name: r#"match_arm"#.to_string() },
                    ] }) },
            ] },
            line_number: 33,
        },
        GrammarRule {
            name: r#"match_arm"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"pattern"#.to_string() },
                GrammarElement::TokenReference { name: r#"ARROW"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 34,
        },
        GrammarRule {
            name: r#"pattern"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"None"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"Some"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"Ok"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Literal { value: r#"Err"#.to_string() },
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 35,
        },
        GrammarRule {
            name: r#"implication"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"disjunction"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"IMPLIES"#.to_string() },
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    ] }) },
            ] },
            line_number: 36,
        },
        GrammarRule {
            name: r#"disjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"conjunction"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"or"#.to_string() },
                        GrammarElement::RuleReference { name: r#"conjunction"#.to_string() },
                    ] }) },
            ] },
            line_number: 37,
        },
        GrammarRule {
            name: r#"conjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"and"#.to_string() },
                        GrammarElement::RuleReference { name: r#"comparison"#.to_string() },
                    ] }) },
            ] },
            line_number: 38,
        },
        GrammarRule {
            name: r#"comparison"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"sum"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"comparison_operator"#.to_string() },
                        GrammarElement::RuleReference { name: r#"sum"#.to_string() },
                    ] }) },
            ] },
            line_number: 39,
        },
        GrammarRule {
            name: r#"comparison_operator"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"EQEQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"NOTEQ"#.to_string() },
                GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                GrammarElement::TokenReference { name: r#"GE"#.to_string() },
            ] },
            line_number: 40,
        },
        GrammarRule {
            name: r#"sum"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"product"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"product"#.to_string() },
                    ] }) },
            ] },
            line_number: 41,
        },
        GrammarRule {
            name: r#"product"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 42,
        },
        GrammarRule {
            name: r#"unary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                            GrammarElement::Literal { value: r#"not"#.to_string() },
                            GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                        ] }) },
                    GrammarElement::RuleReference { name: r#"unary"#.to_string() },
                ] },
                GrammarElement::RuleReference { name: r#"primary"#.to_string() },
            ] },
            line_number: 43,
        },
        GrammarRule {
            name: r#"primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"literal"#.to_string() },
                GrammarElement::RuleReference { name: r#"list_literal"#.to_string() },
                GrammarElement::RuleReference { name: r#"reference"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 44,
        },
        GrammarRule {
            name: r#"literal"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"true"#.to_string() },
                GrammarElement::Literal { value: r#"false"#.to_string() },
                GrammarElement::TokenReference { name: r#"U64_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"U8_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"U16_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"U32_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"I8_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"I16_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"I32_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"I64_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"HEX_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"BYTES_LITERAL"#.to_string() },
                GrammarElement::TokenReference { name: r#"INT"#.to_string() },
            ] },
            line_number: 45,
        },
        GrammarRule {
            name: r#"list_literal"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LBRACKET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                            ] }) },
                    ] }) },
                GrammarElement::TokenReference { name: r#"RBRACKET"#.to_string() },
            ] },
            line_number: 46,
        },
        GrammarRule {
            name: r#"reference"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"qualified_name"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                        GrammarElement::RuleReference { name: r#"widths"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                                        GrammarElement::RuleReference { name: r#"expression"#.to_string() },
                                    ] }) },
                            ] }) },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 47,
        },
        GrammarRule {
            name: r#"qualified_name"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"QNAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 48,
        },
        GrammarRule {
            name: r#"widths"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"width"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"COMMA"#.to_string() },
                        GrammarElement::RuleReference { name: r#"width"#.to_string() },
                    ] }) },
            ] },
            line_number: 49,
        },
    ],
        version: 0,
    }
}
