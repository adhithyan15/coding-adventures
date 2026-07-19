// AUTO-GENERATED FILE — DO NOT EDIT
// Source: cobol.grammar
// Regenerate with: grammar-tools compile-grammar cobol.grammar
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
                GrammarElement::RuleReference { name: r#"identification_division"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"environment_division"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"data_division"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"procedure_division"#.to_string() },
            ] },
            line_number: 33,
        },
        GrammarRule {
            name: r#"identification_division"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"IDENTIFICATION"#.to_string() },
                GrammarElement::Literal { value: r#"DIVISION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::RuleReference { name: r#"program_id"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"id_paragraph"#.to_string() }) },
            ] },
            line_number: 44,
        },
        GrammarRule {
            name: r#"program_id"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"PROGRAM-ID"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 49,
        },
        GrammarRule {
            name: r#"id_paragraph"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"id_keyword"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"comment_word"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 51,
        },
        GrammarRule {
            name: r#"id_keyword"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"AUTHOR"#.to_string() },
                GrammarElement::Literal { value: r#"INSTALLATION"#.to_string() },
                GrammarElement::Literal { value: r#"DATE-WRITTEN"#.to_string() },
                GrammarElement::Literal { value: r#"DATE-COMPILED"#.to_string() },
                GrammarElement::Literal { value: r#"SECURITY"#.to_string() },
                GrammarElement::Literal { value: r#"REMARKS"#.to_string() },
            ] },
            line_number: 52,
        },
        GrammarRule {
            name: r#"comment_word"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
            ] },
            line_number: 55,
        },
        GrammarRule {
            name: r#"environment_division"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"ENVIRONMENT"#.to_string() },
                GrammarElement::Literal { value: r#"DIVISION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"env_section"#.to_string() }) },
            ] },
            line_number: 64,
        },
        GrammarRule {
            name: r#"env_section"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"configuration_section"#.to_string() },
                GrammarElement::RuleReference { name: r#"input_output_section"#.to_string() },
            ] },
            line_number: 65,
        },
        GrammarRule {
            name: r#"configuration_section"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"CONFIGURATION"#.to_string() },
                GrammarElement::Literal { value: r#"SECTION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"config_paragraph"#.to_string() }) },
            ] },
            line_number: 67,
        },
        GrammarRule {
            name: r#"config_paragraph"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"SOURCE-COMPUTER"#.to_string() },
                        GrammarElement::Literal { value: r#"OBJECT-COMPUTER"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 69,
        },
        GrammarRule {
            name: r#"input_output_section"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"INPUT-OUTPUT"#.to_string() },
                GrammarElement::Literal { value: r#"SECTION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Literal { value: r#"FILE-CONTROL"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"select_sentence"#.to_string() }) },
            ] },
            line_number: 71,
        },
        GrammarRule {
            name: r#"select_sentence"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"SELECT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"ASSIGN"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 73,
        },
        GrammarRule {
            name: r#"data_division"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DATA"#.to_string() },
                GrammarElement::Literal { value: r#"DIVISION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"data_section"#.to_string() }) },
            ] },
            line_number: 81,
        },
        GrammarRule {
            name: r#"data_section"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"working_storage_section"#.to_string() },
                GrammarElement::RuleReference { name: r#"file_section"#.to_string() },
            ] },
            line_number: 82,
        },
        GrammarRule {
            name: r#"working_storage_section"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"WORKING-STORAGE"#.to_string() },
                GrammarElement::Literal { value: r#"SECTION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"data_entry"#.to_string() }) },
            ] },
            line_number: 84,
        },
        GrammarRule {
            name: r#"file_section"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"FILE"#.to_string() },
                GrammarElement::Literal { value: r#"SECTION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"file_or_data_entry"#.to_string() }) },
            ] },
            line_number: 85,
        },
        GrammarRule {
            name: r#"file_or_data_entry"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"fd_entry"#.to_string() },
                GrammarElement::RuleReference { name: r#"data_entry"#.to_string() },
            ] },
            line_number: 86,
        },
        GrammarRule {
            name: r#"fd_entry"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"FD"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"comment_word"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 87,
        },
        GrammarRule {
            name: r#"data_entry"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        GrammarElement::Literal { value: r#"FILLER"#.to_string() },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"data_clause"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 89,
        },
        GrammarRule {
            name: r#"data_clause"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"picture_clause"#.to_string() },
                GrammarElement::RuleReference { name: r#"value_clause"#.to_string() },
            ] },
            line_number: 90,
        },
        GrammarRule {
            name: r#"picture_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"PICTURE"#.to_string() },
                        GrammarElement::Literal { value: r#"PIC"#.to_string() },
                    ] }) },
                GrammarElement::TokenReference { name: r#"PIC_STRING"#.to_string() },
            ] },
            line_number: 92,
        },
        GrammarRule {
            name: r#"value_clause"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"VALUE"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"IS"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"value_item"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"value_item"#.to_string() }) },
            ] },
            line_number: 98,
        },
        GrammarRule {
            name: r#"value_item"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"literal"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"THRU"#.to_string() },
                                GrammarElement::Literal { value: r#"THROUGH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"literal"#.to_string() },
                    ] }) },
            ] },
            line_number: 99,
        },
        GrammarRule {
            name: r#"procedure_division"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"PROCEDURE"#.to_string() },
                GrammarElement::Literal { value: r#"DIVISION"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"paragraph"#.to_string() }) },
            ] },
            line_number: 108,
        },
        GrammarRule {
            name: r#"paragraph"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"sentence"#.to_string() }) },
            ] },
            line_number: 109,
        },
        GrammarRule {
            name: r#"sentence"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"statement"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"DOT"#.to_string() },
            ] },
            line_number: 110,
        },
        GrammarRule {
            name: r#"statement"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"move_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"display_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"accept_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"add_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"subtract_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"multiply_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"divide_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"compute_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"perform_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"goto_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"if_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"set_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"evaluate_stmt"#.to_string() },
                GrammarElement::RuleReference { name: r#"stop_stmt"#.to_string() },
            ] },
            line_number: 112,
        },
        GrammarRule {
            name: r#"move_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"MOVE"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::TokenReference { name: r#"NAME"#.to_string() }) },
            ] },
            line_number: 116,
        },
        GrammarRule {
            name: r#"set_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"SET"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::Literal { value: r#"TRUE"#.to_string() },
            ] },
            line_number: 119,
        },
        GrammarRule {
            name: r#"evaluate_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"EVALUATE"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"when_branch"#.to_string() }) },
                GrammarElement::Literal { value: r#"END-EVALUATE"#.to_string() },
            ] },
            line_number: 126,
        },
        GrammarRule {
            name: r#"when_branch"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"WHEN"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"OTHER"#.to_string() },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"when_value"#.to_string() },
                            GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"when_value"#.to_string() }) },
                        ] },
                    ] }) },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
            ] },
            line_number: 127,
        },
        GrammarRule {
            name: r#"when_value"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::Literal { value: r#"THRU"#.to_string() },
                                GrammarElement::Literal { value: r#"THROUGH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                    ] }) },
            ] },
            line_number: 128,
        },
        GrammarRule {
            name: r#"display_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DISPLAY"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"operand"#.to_string() }) },
            ] },
            line_number: 129,
        },
        GrammarRule {
            name: r#"accept_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"ACCEPT"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 130,
        },
        GrammarRule {
            name: r#"add_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"ADD"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"operand"#.to_string() }) },
                GrammarElement::Literal { value: r#"TO"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"GIVING"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"ROUNDED"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"size_error"#.to_string() }) },
            ] },
            line_number: 133,
        },
        GrammarRule {
            name: r#"subtract_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"SUBTRACT"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"operand"#.to_string() }) },
                GrammarElement::Literal { value: r#"FROM"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"GIVING"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"ROUNDED"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"size_error"#.to_string() }) },
            ] },
            line_number: 135,
        },
        GrammarRule {
            name: r#"multiply_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"MULTIPLY"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Literal { value: r#"BY"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"GIVING"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"ROUNDED"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"size_error"#.to_string() }) },
            ] },
            line_number: 137,
        },
        GrammarRule {
            name: r#"divide_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"DIVIDE"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Literal { value: r#"INTO"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"GIVING"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"ROUNDED"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"size_error"#.to_string() }) },
            ] },
            line_number: 139,
        },
        GrammarRule {
            name: r#"compute_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"COMPUTE"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"ROUNDED"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::RuleReference { name: r#"size_error"#.to_string() }) },
            ] },
            line_number: 147,
        },
        GrammarRule {
            name: r#"size_error"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"ON"#.to_string() },
                GrammarElement::Literal { value: r#"SIZE"#.to_string() },
                GrammarElement::Literal { value: r#"ERROR"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
            ] },
            line_number: 148,
        },
        GrammarRule {
            name: r#"arith_expr"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"arith_term"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                                GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"arith_term"#.to_string() },
                    ] }) },
            ] },
            line_number: 164,
        },
        GrammarRule {
            name: r#"arith_term"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"arith_factor"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                                GrammarElement::TokenReference { name: r#"STAR"#.to_string() },
                                GrammarElement::TokenReference { name: r#"SLASH"#.to_string() },
                            ] }) },
                        GrammarElement::RuleReference { name: r#"arith_factor"#.to_string() },
                    ] }) },
            ] },
            line_number: 165,
        },
        GrammarRule {
            name: r#"arith_factor"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"arith_unary"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::TokenReference { name: r#"POW"#.to_string() },
                        GrammarElement::RuleReference { name: r#"arith_unary"#.to_string() },
                    ] }) },
            ] },
            line_number: 166,
        },
        GrammarRule {
            name: r#"arith_unary"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::TokenReference { name: r#"PLUS"#.to_string() },
                        GrammarElement::TokenReference { name: r#"MINUS"#.to_string() },
                    ] }) },
                GrammarElement::RuleReference { name: r#"arith_primary"#.to_string() },
            ] },
            line_number: 167,
        },
        GrammarRule {
            name: r#"arith_primary"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"arith_expr"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 168,
        },
        GrammarRule {
            name: r#"perform_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"PERFORM"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"THROUGH"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"THRU"#.to_string() },
                            GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                        ] },
                    ] }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                            GrammarElement::Literal { value: r#"TIMES"#.to_string() },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"UNTIL"#.to_string() },
                            GrammarElement::RuleReference { name: r#"condition"#.to_string() },
                        ] },
                        GrammarElement::RuleReference { name: r#"perform_varying"#.to_string() },
                    ] }) },
            ] },
            line_number: 169,
        },
        GrammarRule {
            name: r#"perform_varying"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"VARYING"#.to_string() },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::Literal { value: r#"FROM"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Literal { value: r#"BY"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::Literal { value: r#"UNTIL"#.to_string() },
                GrammarElement::RuleReference { name: r#"condition"#.to_string() },
            ] },
            line_number: 173,
        },
        GrammarRule {
            name: r#"goto_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"GO"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"TO"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            ] },
            line_number: 174,
        },
        GrammarRule {
            name: r#"stop_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"STOP"#.to_string() },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Literal { value: r#"RUN"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                        GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                    ] }) },
            ] },
            line_number: 175,
        },
        GrammarRule {
            name: r#"if_stmt"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Literal { value: r#"IF"#.to_string() },
                GrammarElement::RuleReference { name: r#"condition"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"ELSE"#.to_string() },
                        GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"statement"#.to_string() }) },
                    ] }) },
            ] },
            line_number: 191,
        },
        GrammarRule {
            name: r#"condition"#.to_string(),
            body: GrammarElement::RuleReference { name: r#"disjunction"#.to_string() },
            line_number: 192,
        },
        GrammarRule {
            name: r#"disjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"conjunction"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"OR"#.to_string() },
                        GrammarElement::RuleReference { name: r#"conjunction"#.to_string() },
                    ] }) },
            ] },
            line_number: 193,
        },
        GrammarRule {
            name: r#"conjunction"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"negation"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::Sequence { elements: vec![
                        GrammarElement::Literal { value: r#"AND"#.to_string() },
                        GrammarElement::RuleReference { name: r#"negation"#.to_string() },
                    ] }) },
            ] },
            line_number: 194,
        },
        GrammarRule {
            name: r#"negation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"NOT"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"simple_condition"#.to_string() },
            ] },
            line_number: 200,
        },
        GrammarRule {
            name: r#"simple_condition"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"relation"#.to_string() },
                GrammarElement::RuleReference { name: r#"condition_name"#.to_string() },
                GrammarElement::Sequence { elements: vec![
                    GrammarElement::TokenReference { name: r#"LPAREN"#.to_string() },
                    GrammarElement::RuleReference { name: r#"condition"#.to_string() },
                    GrammarElement::TokenReference { name: r#"RPAREN"#.to_string() },
                ] },
            ] },
            line_number: 201,
        },
        GrammarRule {
            name: r#"relation"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
                GrammarElement::RuleReference { name: r#"relop"#.to_string() },
                GrammarElement::RuleReference { name: r#"operand"#.to_string() },
            ] },
            line_number: 202,
        },
        GrammarRule {
            name: r#"condition_name"#.to_string(),
            body: GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
            line_number: 203,
        },
        GrammarRule {
            name: r#"relop"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"IS"#.to_string() }) },
                GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"NOT"#.to_string() }) },
                GrammarElement::Group { element: Box::new(GrammarElement::Alternation { choices: vec![
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"GREATER"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"THAN"#.to_string() }) },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"LESS"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"THAN"#.to_string() }) },
                        ] },
                        GrammarElement::Sequence { elements: vec![
                            GrammarElement::Literal { value: r#"EQUAL"#.to_string() },
                            GrammarElement::Optional { element: Box::new(GrammarElement::Literal { value: r#"TO"#.to_string() }) },
                        ] },
                        GrammarElement::TokenReference { name: r#"GE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"NE"#.to_string() },
                        GrammarElement::TokenReference { name: r#"GT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"LT"#.to_string() },
                        GrammarElement::TokenReference { name: r#"EQ"#.to_string() },
                    ] }) },
            ] },
            line_number: 207,
        },
        GrammarRule {
            name: r#"operand"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NAME"#.to_string() },
                GrammarElement::RuleReference { name: r#"literal"#.to_string() },
            ] },
            line_number: 214,
        },
        GrammarRule {
            name: r#"literal"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::TokenReference { name: r#"NUMBER"#.to_string() },
                GrammarElement::TokenReference { name: r#"STRING"#.to_string() },
                GrammarElement::RuleReference { name: r#"figurative"#.to_string() },
            ] },
            line_number: 215,
        },
        GrammarRule {
            name: r#"figurative"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::Literal { value: r#"ZERO"#.to_string() },
                GrammarElement::Literal { value: r#"ZEROS"#.to_string() },
                GrammarElement::Literal { value: r#"ZEROES"#.to_string() },
                GrammarElement::Literal { value: r#"SPACE"#.to_string() },
                GrammarElement::Literal { value: r#"SPACES"#.to_string() },
                GrammarElement::Literal { value: r#"HIGH-VALUE"#.to_string() },
                GrammarElement::Literal { value: r#"HIGH-VALUES"#.to_string() },
                GrammarElement::Literal { value: r#"LOW-VALUE"#.to_string() },
                GrammarElement::Literal { value: r#"LOW-VALUES"#.to_string() },
                GrammarElement::Literal { value: r#"QUOTE"#.to_string() },
                GrammarElement::Literal { value: r#"QUOTES"#.to_string() },
            ] },
            line_number: 216,
        },
    ],
        version: 1,
    }
}
