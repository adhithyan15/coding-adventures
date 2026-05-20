//! # coding-adventures-ruby-lexer
//!
//! Ruby lexer driven by a TOML-encoded state machine.  See
//! `code/specs/ruby-parser.md` for the architectural overview and
//! `code/specs/ruby-lexer-state-machine.md` for the per-state
//! transition rules.
//!
//! ## Pipeline
//!
//! ```text
//! Ruby source (&str)
//!    │
//!    ▼  RubyLexer::new(version)
//!    │   ┌── EffectfulStateMachine — from `ruby-<ver>.lexer.states.toml`
//!    │
//!    ▼  push(source) — one character at a time
//! action interpreter (this crate) — turns effect strings into Tokens
//!    │
//!    ▼  finish() then drain_tokens()
//! Vec<Token>
//! ```
//!
//! ## Phase 1 scope
//!
//! Per `code/specs/ruby-parser.md`, Phase 1 covers the **paren-
//! required Ruby 1.8 baseline**: identifiers, integers, strings (no
//! interpolation), line comments, common operators and punctuation,
//! and newline-as-token.  Heredocs, regex disambiguation, percent
//! literals, string interpolation, and parser-driven `f /x/`
//! resolution all arrive in later phases.

use lexer::token::{Token, TokenType};
use state_machine::transducer::{EffectfulInput, EffectfulStateMachine};

mod machine;

/// Non-fatal diagnostic produced by the lexer.  Stray bytes /
/// unterminated strings / etc. are recorded here; the lexer keeps
/// going from the next character so callers always get a complete
/// token stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub line: usize,
    pub column: usize,
}

/// Stateful Ruby lexer.
pub struct RubyLexer {
    machine: EffectfulStateMachine,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    /// Accumulator the action interpreter writes into.  Cleared on
    /// each `clear_text`; flushed by every `emit_token`.
    text_buffer: String,
    /// Current source position of the next character to be fed.
    /// 1-based.
    line: usize,
    column: usize,
    /// Source position where the current token started accumulating.
    token_start_line: usize,
    token_start_column: usize,
}

impl RubyLexer {
    /// Build a fresh lexer for the given Ruby version.  Phase 1
    /// only supports `"1.8"`.
    pub fn new(version: &str) -> Result<Self, String> {
        let definition = machine::definition_for_version(version)?;
        let machine = EffectfulStateMachine::from_definition(&definition)
            .map_err(|e| format!("failed to build ruby lexer state machine: {e}"))?;
        Ok(Self {
            machine,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            text_buffer: String::new(),
            line: 1,
            column: 1,
            token_start_line: 1,
            token_start_column: 1,
        })
    }

    /// Feed the whole source into the lexer.
    pub fn push(&mut self, source: &str) -> Result<(), String> {
        for ch in source.chars() {
            self.step_char(ch)?;
        }
        Ok(())
    }

    /// Signal end-of-input.  Drains any pending state (some peek
    /// states need one or two EOF events to fully flush their
    /// accumulators) and ultimately emits an EOF token.
    pub fn finish(&mut self) -> Result<(), String> {
        const MAX_DRAIN: usize = 32;
        for _ in 0..MAX_DRAIN {
            if self.machine.is_final() {
                return Ok(());
            }
            let step = self
                .machine
                .process(EffectfulInput::end())
                .map_err(|e| format!("ruby lexer drain error: {e}"))?;
            self.apply_effects(&step.effects, None)?;
        }
        Err(format!(
            "ruby lexer did not reach final state within {MAX_DRAIN} drain iterations"
        ))
    }

    /// Take ownership of all tokens emitted so far.
    pub fn drain_tokens(&mut self) -> Vec<Token> {
        std::mem::take(&mut self.tokens)
    }

    /// Non-fatal diagnostics recorded during lexing.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Step the machine on one character.  Honours `consume = false`
    /// transitions by re-feeding the same character to the new
    /// state (capped — a state machine that ping-pongs without
    /// consuming input is a bug).
    fn step_char(&mut self, ch: char) -> Result<(), String> {
        let mut buf = [0u8; 4];
        let event = ch.encode_utf8(&mut buf);
        const MAX_REENTRY: usize = 8;
        for _ in 0..MAX_REENTRY {
            let step = self
                .machine
                .process(EffectfulInput::event(event))
                .map_err(|e| format!("ruby lexer error at {}:{}: {e}", self.line, self.column))?;
            self.apply_effects(&step.effects, Some(ch))?;
            if step.consume {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                return Ok(());
            }
        }
        Err(format!(
            "ruby lexer ping-pong at {}:{} — no transition consumed input",
            self.line, self.column
        ))
    }

    fn apply_effects(&mut self, effects: &[String], current: Option<char>) -> Result<(), String> {
        for raw in effects {
            self.apply_one_effect(raw, current)?;
        }
        Ok(())
    }

    fn apply_one_effect(&mut self, raw: &str, current: Option<char>) -> Result<(), String> {
        let (verb, arg) = split_verb_arg(raw);
        match verb {
            "clear_text" => {
                self.text_buffer.clear();
                self.token_start_line = self.line;
                self.token_start_column = self.column;
            }
            "set_text" => {
                self.text_buffer.clear();
                if let Some(a) = arg {
                    self.text_buffer.push_str(a);
                }
                self.token_start_line = self.line;
                self.token_start_column = self.column;
            }
            "append_text" => {
                let a = arg.ok_or_else(|| "append_text requires an argument".to_string())?;
                if a == "current" {
                    if let Some(c) = current {
                        if self.text_buffer.is_empty() {
                            self.token_start_line = self.line;
                            self.token_start_column = self.column;
                        }
                        self.text_buffer.push(c);
                    }
                } else {
                    self.text_buffer.push_str(&decode_action_literal(a));
                }
            }
            "emit" => {
                let kind = arg.ok_or_else(|| {
                    "emit requires a token-kind argument".to_string()
                })?;
                self.emit_token_by_name(kind);
            }
            "parse_error" => {
                self.diagnostics.push(Diagnostic {
                    code: arg.unwrap_or("unknown").to_string(),
                    line: self.line,
                    column: self.column,
                });
            }
            other => {
                return Err(format!("ruby lexer: unknown action verb `{other}`"));
            }
        }
        Ok(())
    }

    fn emit_token_by_name(&mut self, kind_name: &str) {
        match kind_name {
            "Eof" => self.push_token(TokenType::Eof, String::new()),
            "Newline" => self.push_token(TokenType::Newline, "\n".to_string()),
            "LParen" => self.push_token(TokenType::LParen, "(".to_string()),
            "RParen" => self.push_token(TokenType::RParen, ")".to_string()),
            "LBracket" => self.push_token(TokenType::LBracket, "[".to_string()),
            "RBracket" => self.push_token(TokenType::RBracket, "]".to_string()),
            "LBrace" => self.push_token(TokenType::LBrace, "{".to_string()),
            "RBrace" => self.push_token(TokenType::RBrace, "}".to_string()),
            "Comma" => self.push_token(TokenType::Comma, ",".to_string()),
            "Semi" => self.push_token(TokenType::Semicolon, ";".to_string()),
            "Colon" => self.push_token(TokenType::Colon, ":".to_string()),
            "ColonColon" => {
                // No dedicated TokenType for `::` yet — encode as
                // Colon with value `::` so the parser can dispatch
                // by value.
                self.push_token(TokenType::Colon, "::".to_string());
            }
            "Dot" => self.push_token(TokenType::Dot, ".".to_string()),
            "Int" => {
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::Number, text);
            }
            "String" => {
                let text = std::mem::take(&mut self.text_buffer);
                self.push_token(TokenType::String, text);
            }
            "Name" => {
                let text = std::mem::take(&mut self.text_buffer);
                let kind = classify_name_token(&text);
                self.push_token(kind, text);
            }
            "Op" => {
                let text = std::mem::take(&mut self.text_buffer);
                let kind = classify_op_token(&text);
                self.push_token(kind, text);
            }
            other => {
                self.diagnostics.push(Diagnostic {
                    code: format!("unknown-emit-kind:{}", other),
                    line: self.line,
                    column: self.column,
                });
            }
        }
    }

    fn push_token(&mut self, type_: TokenType, value: String) {
        self.tokens.push(Token {
            type_,
            value,
            line: self.token_start_line,
            column: self.token_start_column,
            type_name: None,
            flags: None,
        });
        // Reset start position so the next immediate-emit token
        // (e.g. an `LParen` right after a `Name`) gets the current
        // source position, not the prior token's start.
        self.token_start_line = self.line;
        self.token_start_column = self.column;
    }
}

/// Parse `verb(arg)` or `verb` into `(verb, Some(arg))` / `(verb, None)`.
fn split_verb_arg(s: &str) -> (&str, Option<&str>) {
    if let Some(open) = s.find('(') {
        if let Some(close) = s.rfind(')') {
            if close > open {
                return (&s[..open], Some(&s[open + 1..close]));
            }
        }
    }
    (s, None)
}

/// Decode `\n`, `\t`, `\r`, `\\`, `\"`, `\'` inside an action-string
/// argument.  Anything else passes through with the backslash.
fn decode_action_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Identifiers that are reserved keywords vs ordinary names.
fn classify_name_token(text: &str) -> TokenType {
    if is_ruby_keyword(text) {
        TokenType::Keyword
    } else {
        TokenType::Name
    }
}

/// Map operator lexemes to their dedicated `TokenType` where one
/// exists.  Operators without a dedicated kind (`!=`, `<=`, `>=`,
/// `&&`, `||`, `=>`, `**`, etc.) land on `TokenType::Name` with the
/// lexeme preserved in `value` — the parser dispatches by value.
fn classify_op_token(text: &str) -> TokenType {
    match text {
        "+" => TokenType::Plus,
        "-" => TokenType::Minus,
        "*" => TokenType::Star,
        "/" => TokenType::Slash,
        "=" => TokenType::Equals,
        "==" => TokenType::EqualsEquals,
        "!" => TokenType::Bang,
        _ => TokenType::Name,
    }
}

fn is_ruby_keyword(s: &str) -> bool {
    matches!(
        s,
        "BEGIN"
            | "END"
            | "alias"
            | "and"
            | "begin"
            | "break"
            | "case"
            | "class"
            | "def"
            | "defined?"
            | "do"
            | "else"
            | "elsif"
            | "end"
            | "ensure"
            | "false"
            | "for"
            | "if"
            | "in"
            | "module"
            | "next"
            | "nil"
            | "not"
            | "or"
            | "redo"
            | "rescue"
            | "retry"
            | "return"
            | "self"
            | "super"
            | "then"
            | "true"
            | "undef"
            | "unless"
            | "until"
            | "when"
            | "while"
            | "yield"
    )
}

// ---------------------------------------------------------------------------
// Convenience entry points — preserve the prior public surface so
// `ruby-parser` keeps working without changes.
// ---------------------------------------------------------------------------

/// Tokenize Ruby source (Phase-1 = Ruby 1.8 lexer).  Returns the
/// EOF-terminated token list.  Diagnostics are dropped silently;
/// call [`tokenize_ruby_diag`] to inspect them.
pub fn tokenize_ruby(source: &str) -> Vec<Token> {
    tokenize_ruby_diag(source).0
}

/// Same as [`tokenize_ruby`] but also returns recorded diagnostics.
pub fn tokenize_ruby_diag(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut lexer = RubyLexer::new("1.8").expect("ruby 1.8 lexer definition");
    if let Err(e) = lexer.push(source) {
        let mut diags = lexer.diagnostics.clone();
        diags.push(Diagnostic {
            code: format!("lex-error:{e}"),
            line: lexer.line,
            column: lexer.column,
        });
        return (lexer.drain_tokens(), diags);
    }
    if let Err(e) = lexer.finish() {
        let mut diags = lexer.diagnostics.clone();
        diags.push(Diagnostic {
            code: format!("finish-error:{e}"),
            line: lexer.line,
            column: lexer.column,
        });
        return (lexer.drain_tokens(), diags);
    }
    let tokens = lexer.drain_tokens();
    let diags = lexer.diagnostics.clone();
    (tokens, diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pull `(type_, value)` pairs out of a token list, omitting EOF.
    fn pairs(toks: &[Token]) -> Vec<(TokenType, &str)> {
        toks.iter()
            .filter(|t| t.type_ != TokenType::Eof)
            .map(|t| (t.type_, t.value.as_str()))
            .collect()
    }

    #[test]
    fn empty_source_just_eof() {
        let toks = tokenize_ruby("");
        assert_eq!(toks.len(), 1);
        assert_eq!(toks[0].type_, TokenType::Eof);
    }

    #[test]
    fn single_identifier() {
        let toks = tokenize_ruby("foo");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Name, "foo")]);
    }

    #[test]
    fn keyword_vs_name() {
        let toks = tokenize_ruby("def foo");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Keyword, "def"),
                (TokenType::Name, "foo"),
            ]
        );
    }

    #[test]
    fn integer_literal() {
        let toks = tokenize_ruby("42");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Number, "42")]);
    }

    #[test]
    fn integer_with_underscores() {
        let toks = tokenize_ruby("1_000_000");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Number, "1_000_000")]);
    }

    #[test]
    fn double_quoted_string() {
        let toks = tokenize_ruby(r#""hello""#);
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "hello")]);
    }

    #[test]
    fn single_quoted_string() {
        let toks = tokenize_ruby("'hello'");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "hello")]);
    }

    #[test]
    fn string_escapes() {
        let toks = tokenize_ruby(r#""line1\nline2""#);
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::String, "line1\nline2")]);
    }

    #[test]
    fn line_comment_skipped() {
        let toks = tokenize_ruby("# this is a comment\nfoo\n");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Newline, "\n"),
                (TokenType::Name, "foo"),
                (TokenType::Newline, "\n"),
            ]
        );
    }

    #[test]
    fn binary_operators_dispatch_to_dedicated_kinds() {
        let toks = tokenize_ruby("a + b - c * d / e");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "a"),
                (TokenType::Plus, "+"),
                (TokenType::Name, "b"),
                (TokenType::Minus, "-"),
                (TokenType::Name, "c"),
                (TokenType::Star, "*"),
                (TokenType::Name, "d"),
                (TokenType::Slash, "/"),
                (TokenType::Name, "e"),
            ]
        );
    }

    #[test]
    fn comparison_operators_preserve_lexeme() {
        let toks = tokenize_ruby("a == b\nc != d\ne <= f\ng >= h");
        let p = pairs(&toks);
        assert!(p.contains(&(TokenType::EqualsEquals, "==")));
        // `!=`, `<=`, `>=` currently land on Name with value preserved.
        assert!(p.iter().any(|(_, v)| *v == "!="));
        assert!(p.iter().any(|(_, v)| *v == "<="));
        assert!(p.iter().any(|(_, v)| *v == ">="));
    }

    #[test]
    fn assignment_and_hash_rocket() {
        let toks = tokenize_ruby("x = 1\nh = { :a => 1 }");
        let p = pairs(&toks);
        assert!(p.iter().any(|(t, _)| *t == TokenType::Equals));
        assert!(p.iter().any(|(_, v)| *v == "=>"));
    }

    #[test]
    fn parens_and_punctuation() {
        let toks = tokenize_ruby("foo(1, 2)");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "foo"),
                (TokenType::LParen, "("),
                (TokenType::Number, "1"),
                (TokenType::Comma, ","),
                (TokenType::Number, "2"),
                (TokenType::RParen, ")"),
            ]
        );
    }

    #[test]
    fn factorial_program_tokenizes() {
        let src = "def factorial(n)\n  if n == 0\n    1\n  else\n    n * factorial(n - 1)\n  end\nend\n";
        let toks = tokenize_ruby(src);
        let kw: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Keyword)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(kw, vec!["def", "if", "else", "end", "end"]);
        // Two `factorial` identifiers (def site + recursive call).
        let factorial_count = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Name && t.value == "factorial")
            .count();
        assert_eq!(factorial_count, 2);
    }

    #[test]
    fn method_name_with_query_suffix() {
        let toks = tokenize_ruby("empty?");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Name, "empty?")]);
    }

    #[test]
    fn method_name_with_bang_suffix() {
        let toks = tokenize_ruby("save!");
        let p = pairs(&toks);
        assert_eq!(p, vec![(TokenType::Name, "save!")]);
    }

    #[test]
    fn newline_is_significant() {
        let toks = tokenize_ruby("a\nb");
        let p = pairs(&toks);
        assert_eq!(
            p,
            vec![
                (TokenType::Name, "a"),
                (TokenType::Newline, "\n"),
                (TokenType::Name, "b"),
            ]
        );
    }

    #[test]
    fn double_colon_emits_colon_colon_value() {
        let toks = tokenize_ruby("Foo::Bar");
        let cc: Vec<&str> = toks.iter().map(|t| t.value.as_str()).collect();
        assert!(cc.contains(&"::"));
    }

    #[test]
    fn determinism() {
        let src = "def f(x)\n  x + 1\nend\n";
        let a = tokenize_ruby(src);
        let b = tokenize_ruby(src);
        let av: Vec<(TokenType, String)> =
            a.iter().map(|t| (t.type_, t.value.clone())).collect();
        let bv: Vec<(TokenType, String)> =
            b.iter().map(|t| (t.type_, t.value.clone())).collect();
        assert_eq!(av, bv);
    }

    #[test]
    fn class_def_tokenizes() {
        let toks = tokenize_ruby("class Foo\n  def bar\n    1\n  end\nend\n");
        let kw: Vec<&str> = toks
            .iter()
            .filter(|t| t.type_ == TokenType::Keyword)
            .map(|t| t.value.as_str())
            .collect();
        assert_eq!(kw, vec!["class", "def", "end", "end"]);
    }
}
