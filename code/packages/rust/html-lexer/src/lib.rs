//! Rust HTML lexer built on the generic state-machine lexer runtime.
//!
//! The HTML standard calls its lexical stage a tokenizer. This package uses the
//! repo's lexer naming while preserving standard state names like `data` and
//! `tag_open`.

use state_machine::{EffectfulStateMachine, StateMachineDefinition};

pub use state_machine_tokenizer::{
    Attribute, Diagnostic, Result, SourcePosition, Token, Tokenizer as HtmlLexer, TokenizerError,
    TokenizerTraceEntry,
};

mod generated_html1;
mod generated_html_skeleton;

/// Return the generated typed definition for the HTML 1.x compatibility-floor lexer.
pub fn html1_definition() -> StateMachineDefinition {
    generated_html1::html1_lexer_definition()
}

/// Build the statically linked HTML 1.x compatibility-floor lexer machine.
pub fn html1_machine() -> std::result::Result<EffectfulStateMachine, String> {
    generated_html1::html1_lexer_transducer()
}

/// Return the generated typed definition for the current HTML lexer skeleton.
pub fn html_skeleton_definition() -> StateMachineDefinition {
    generated_html_skeleton::html_skeleton_lexer_definition()
}

/// Build the first statically linked HTML lexer skeleton.
pub fn html_skeleton_machine() -> std::result::Result<EffectfulStateMachine, String> {
    generated_html_skeleton::html_skeleton_lexer_transducer()
}

/// Build a Rust HTML lexer over the statically linked HTML 1.x compatibility floor.
pub fn create_html_lexer() -> Result<HtmlLexer> {
    html1_machine()
        .map(|machine| HtmlLexer::new(machine).with_normalized_carriage_returns())
        .map_err(TokenizerError::Machine)
}

/// Lex one complete HTML string with the current compatibility-floor machine.
pub fn lex_html(source: &str) -> Result<Vec<Token>> {
    let mut lexer = create_html_lexer()?;
    lexer.push(source)?;
    lexer.finish()?;
    Ok(lexer.drain_tokens())
}
