// AUTO-GENERATED FILE — DO NOT EDIT
// Source: flow_matic.grammar
// Regenerate with: grammar-tools compile-grammar flow_matic.grammar
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
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"program_end"#.to_string() }) },
            ] },
            line_number: 46,
        },
        GrammarRule {
            name: r#"program_end"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::Literal { value: r#"END"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 47,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                GrammarElement::RuleReference { name: r#"clause"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::RuleReference { name: r#"clause"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"PERIOD"#.to_string() },
            ] },
            line_number: 54,
        },
        GrammarRule {
            name: r#"clause"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"input_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"output_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"hsp_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"compare_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"otherwise_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"transfer_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"move_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"jump_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"read_item_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"write_item_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"test_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"rewind_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"closeout_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"stop_clause"#.to_string() },
            ] },
            line_number: 63,
        },
        GrammarRule {
            name: r#"input_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"INPUT"#.to_string() },
                GrammarElement::RuleReference { name: r#"file_pair"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"file_pair"#.to_string() }) },
            ] },
            line_number: 74,
        },
        GrammarRule {
            name: r#"output_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"OUTPUT"#.to_string() },
                GrammarElement::RuleReference { name: r#"file_pair"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"file_pair"#.to_string() }) },
            ] },
            line_number: 75,
        },
        GrammarRule {
            name: r#"file_pair"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 76,
        },
        GrammarRule {
            name: r#"hsp_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"HSP"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 77,
        },
        GrammarRule {
            name: r#"compare_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"COMPARE"#.to_string() },
                GrammarElement::RuleReference { name: r#"field"#.to_string() },
                GrammarElement::Literal { value: r#"WITH"#.to_string() },
                GrammarElement::RuleReference { name: r#"field"#.to_string() },
            ] },
            line_number: 83,
        },
        GrammarRule {
            name: r#"field"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
            ] },
            line_number: 84,
        },
        GrammarRule {
            name: r#"if_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"IF"#.to_string() },
                GrammarElement::RuleReference { name: r#"condition"#.to_string() },
                GrammarElement::Literal { value: r#"GO"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::RuleReference { name: r#"target"#.to_string() },
            ] },
            line_number: 88,
        },
        GrammarRule {
            name: r#"otherwise_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"OTHERWISE"#.to_string() },
                GrammarElement::Literal { value: r#"GO"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::RuleReference { name: r#"target"#.to_string() },
            ] },
            line_number: 89,
        },
        GrammarRule {
            name: r#"condition"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"GREATER"#.to_string() },
                GrammarElement::Literal { value: r#"EQUAL"#.to_string() },
                GrammarElement::Literal { value: r#"LESS"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"END"#.to_string() },
                        GrammarElement::Literal { value: r#"OF"#.to_string() },
                        GrammarElement::Literal { value: r#"DATA"#.to_string() },
                    ] }) },
            ] },
            line_number: 90,
        },
        GrammarRule {
            name: r#"target"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"OPERATION"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"transfer_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"TRANSFER"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 94,
        },
        GrammarRule {
            name: r#"move_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"MOVE"#.to_string() },
                GrammarElement::RuleReference { name: r#"field"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::RuleReference { name: r#"field"#.to_string() },
            ] },
            line_number: 95,
        },
        GrammarRule {
            name: r#"jump_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"JUMP"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::RuleReference { name: r#"target"#.to_string() },
            ] },
            line_number: 96,
        },
        GrammarRule {
            name: r#"read_item_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"READ-ITEM"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 99,
        },
        GrammarRule {
            name: r#"write_item_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"WRITE-ITEM"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 100,
        },
        GrammarRule {
            name: r#"test_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"TEST"#.to_string() },
                GrammarElement::RuleReference { name: r#"field"#.to_string() },
                GrammarElement::Literal { value: r#"AGAINST"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                    ] }) },
            ] },
            line_number: 106,
        },
        GrammarRule {
            name: r#"rewind_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"REWIND"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 107,
        },
        GrammarRule {
            name: r#"closeout_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"CLOSE-OUT"#.to_string() },
                GrammarElement::Literal { value: r#"FILES"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"SEMICOLON"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
            ] },
            line_number: 113,
        },
        GrammarRule {
            name: r#"stop_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"STOP"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                        GrammarElement::Literal { value: r#"END"#.to_string() },
                        GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                    ] }) },
            ] },
            line_number: 116,
        },
    ],
        version: 1,
    }
}
