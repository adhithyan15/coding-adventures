//! Rust HTML lexer built on the generic state-machine lexer runtime.
//!
//! The HTML standard calls its lexical stage a tokenizer. This package uses the
//! repo's lexer naming while preserving standard state names like `data` and
//! `tag_open`.

use std::collections::HashSet;

use state_machine::{EffectfulMatcher, EffectfulStateMachine, EffectfulTransition};

pub use state_machine_tokenizer::{
    Attribute, Diagnostic, Result, SourcePosition, Token, Tokenizer as HtmlLexer, TokenizerError,
    TokenizerTraceEntry,
};

/// Build the first statically linked HTML lexer skeleton.
pub fn html_skeleton_machine() -> std::result::Result<EffectfulStateMachine, String> {
    EffectfulStateMachine::new(
        set(&[
            "data",
            "tag_open",
            "tag_name",
            "end_tag_open",
            "end_tag_name",
            "done",
        ]),
        set(&["<", "/", ">"]),
        vec![
            EffectfulTransition::new("data", EffectfulMatcher::Event("<".to_string()), "tag_open")
                .with_effects(&["flush_text"]),
            EffectfulTransition::new("data", EffectfulMatcher::End, "done")
                .with_effects(&["flush_text", "emit(EOF)"])
                .consuming(false),
            EffectfulTransition::new("data", EffectfulMatcher::Any, "data")
                .with_effects(&["append_text(current)"]),
            EffectfulTransition::new(
                "tag_open",
                EffectfulMatcher::Event("/".to_string()),
                "end_tag_open",
            ),
            EffectfulTransition::new("tag_open", EffectfulMatcher::End, "done")
                .with_effects(&[
                    "parse_error(eof-in-tag-open-state)",
                    "append_text(<)",
                    "flush_text",
                    "emit(EOF)",
                ])
                .consuming(false),
            EffectfulTransition::new("tag_open", EffectfulMatcher::Any, "tag_name")
                .with_effects(&["create_start_tag", "append_tag_name(current_lowercase)"]),
            EffectfulTransition::new("tag_name", EffectfulMatcher::Event(">".to_string()), "data")
                .with_effects(&["emit_current_token"]),
            EffectfulTransition::new("tag_name", EffectfulMatcher::End, "done")
                .with_effects(&[
                    "parse_error(eof-in-tag-name-state)",
                    "emit_current_token",
                    "emit(EOF)",
                ])
                .consuming(false),
            EffectfulTransition::new("tag_name", EffectfulMatcher::Any, "tag_name")
                .with_effects(&["append_tag_name(current_lowercase)"]),
            EffectfulTransition::new("end_tag_open", EffectfulMatcher::Any, "end_tag_name")
                .with_effects(&["create_end_tag", "append_tag_name(current_lowercase)"]),
            EffectfulTransition::new(
                "end_tag_name",
                EffectfulMatcher::Event(">".to_string()),
                "data",
            )
            .with_effects(&["emit_current_token"]),
            EffectfulTransition::new("end_tag_name", EffectfulMatcher::End, "done")
                .with_effects(&[
                    "parse_error(eof-in-end-tag-name-state)",
                    "emit_current_token",
                    "emit(EOF)",
                ])
                .consuming(false),
            EffectfulTransition::new("end_tag_name", EffectfulMatcher::Any, "end_tag_name")
                .with_effects(&["append_tag_name(current_lowercase)"]),
        ],
        "data".to_string(),
        set(&["done"]),
    )
}

/// Build a Rust HTML lexer over the statically linked skeleton machine.
pub fn create_html_lexer() -> Result<HtmlLexer> {
    html_skeleton_machine()
        .map(HtmlLexer::new)
        .map_err(TokenizerError::Machine)
}

/// Lex one complete HTML string with the current skeleton machine.
pub fn lex_html(source: &str) -> Result<Vec<Token>> {
    let mut lexer = create_html_lexer()?;
    lexer.push(source)?;
    lexer.finish()?;
    Ok(lexer.drain_tokens())
}

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}
