// AUTO-GENERATED FILE — DO NOT EDIT
// Source: adjacency.grammar
// Regenerate with: grammar-tools compile-grammar adjacency.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"command_call"#.to_string() },
                GrammarElement::RuleReference { name: r#"expression"#.to_string() },
            ] },
            line_number: 2,
        },
        GrammarRule {
            name: r#"command_call"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::SpaceBefore },
                GrammarElement::NegativeLookahead { element: Box::new(GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::SpaceAfterNext }) },
                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::NewlineBefore }) },
            ] },
            line_number: 3,
        },
        GrammarRule {
            name: r#"expression"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::NoNewlineBefore },
                        GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                        GrammarElement::RuleReference { name: r#"term"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::PositiveLookahead { element: Box::new(GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::NoSpaceBefore }) },
                        GrammarElement::AdjacencyPredicate { predicate: grammar_tools::parser_grammar::AdjacencyPredicate::SpaceAfterNext },
                    ] }) },
            ] },
            line_number: 4,
        },
        GrammarRule {
            name: r#"term"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 5,
        },
    ],
        version: 0,
    }
}
