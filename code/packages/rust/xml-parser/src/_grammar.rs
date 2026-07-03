// AUTO-GENERATED FILE — DO NOT EDIT
// Source: xml.grammar
// Regenerate with: grammar-tools compile-grammar xml.grammar
//
// This file embeds a ParserGrammar as native Rust data structures.
// Call `parser_grammar()` instead of reading and parsing the .grammar file.

use grammar_tools::parser_grammar::{GrammarElement, GrammarRule, ParserGrammar};

pub fn parser_grammar() -> ParserGrammar {
    ParserGrammar {
        rules: vec![
        GrammarRule {
            name: r#"document"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"misc"#.to_string() }) },
                GrammarElement::RuleReference { name: r#"element"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"misc"#.to_string() }) },
            ] },
            line_number: 45,
        },
        GrammarRule {
            name: r#"misc"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"pi"#.to_string() },
                GrammarElement::RuleReference { name: r#"comment"#.to_string() },
            ] },
            line_number: 50,
        },
        GrammarRule {
            name: r#"element"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"empty_element"#.to_string() },
                GrammarElement::RuleReference { name: r#"container_element"#.to_string() },
            ] },
            line_number: 60,
        },
        GrammarRule {
            name: r#"empty_element"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"OPEN_TAG_START"#.to_string() },
                GrammarElement::TokenReference { name: r#"TAG_NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"attribute"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"SELF_CLOSE"#.to_string() },
            ] },
            line_number: 62,
        },
        GrammarRule {
            name: r#"container_element"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"OPEN_TAG_START"#.to_string() },
                GrammarElement::TokenReference { name: r#"TAG_NAME"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"attribute"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"TAG_CLOSE"#.to_string() },
                GrammarElement::Repetition { element: Box::new(GrammarElement::RuleReference { name: r#"content"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"CLOSE_TAG_START"#.to_string() },
                GrammarElement::TokenReference { name: r#"TAG_NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"TAG_CLOSE"#.to_string() },
            ] },
            line_number: 64,
        },
        GrammarRule {
            name: r#"attribute"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"TAG_NAME"#.to_string() },
                GrammarElement::TokenReference { name: r#"ATTR_EQUALS"#.to_string() },
                GrammarElement::TokenReference { name: r#"ATTR_VALUE"#.to_string() },
            ] },
            line_number: 72,
        },
        GrammarRule {
            name: r#"content"#.to_string(),
            body: GrammarElement::Alternation { choices: vec![
                GrammarElement::RuleReference { name: r#"element"#.to_string() },
                GrammarElement::RuleReference { name: r#"comment"#.to_string() },
                GrammarElement::RuleReference { name: r#"cdata"#.to_string() },
                GrammarElement::RuleReference { name: r#"pi"#.to_string() },
                GrammarElement::TokenReference { name: r#"CHAR_REF"#.to_string() },
                GrammarElement::TokenReference { name: r#"ENTITY_REF"#.to_string() },
                GrammarElement::TokenReference { name: r#"TEXT"#.to_string() },
            ] },
            line_number: 87,
        },
        GrammarRule {
            name: r#"comment"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"COMMENT_START"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"COMMENT_TEXT"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"COMMENT_END"#.to_string() },
            ] },
            line_number: 91,
        },
        GrammarRule {
            name: r#"cdata"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"CDATA_START"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"CDATA_TEXT"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"CDATA_END"#.to_string() },
            ] },
            line_number: 95,
        },
        GrammarRule {
            name: r#"pi"#.to_string(),
            body: GrammarElement::Sequence { elements: vec![
                GrammarElement::TokenReference { name: r#"PI_START"#.to_string() },
                GrammarElement::TokenReference { name: r#"PI_TARGET"#.to_string() },
                GrammarElement::Optional { element: Box::new(GrammarElement::TokenReference { name: r#"PI_TEXT"#.to_string() }) },
                GrammarElement::TokenReference { name: r#"PI_END"#.to_string() },
            ] },
            line_number: 101,
        },
    ],
        version: 1,
    }
}
