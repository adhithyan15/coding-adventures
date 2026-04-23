//! Tokenizer profile runtime for effectful state machines.
//!
//! The core `state-machine` crate owns the generic transition engine. This
//! crate owns tokenizer-specific state: buffers, current token construction,
//! diagnostics, source positions, and action interpretation.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt;

use state_machine::{EffectfulInput, EffectfulStateMachine};

/// Parsed token emitted by the tokenizer profile runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Buffered character data.
    Text(String),
    /// Start tag token.
    StartTag {
        /// Lower-level definitions decide casing; this runtime stores the
        /// already-interpreted name.
        name: String,
        /// Attributes collected on this start tag.
        attributes: Vec<Attribute>,
        /// Whether the source used self-closing syntax.
        self_closing: bool,
    },
    /// End tag token.
    EndTag {
        /// Tag name.
        name: String,
    },
    /// Comment token.
    Comment(String),
    /// Doctype token.
    Doctype {
        /// Optional doctype name.
        name: Option<String>,
        /// Whether parse errors should force quirks mode.
        force_quirks: bool,
    },
    /// End-of-file token.
    Eof,
}

/// Attribute attached to a start tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Attribute name.
    pub name: String,
    /// Attribute value.
    pub value: String,
}

/// Source position before a transition runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// UTF-8 byte offset from the beginning of the stream.
    pub byte_offset: usize,
    /// Unicode scalar offset from the beginning of the stream.
    pub char_offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

impl Default for SourcePosition {
    fn default() -> Self {
        Self {
            byte_offset: 0,
            char_offset: 0,
            line: 1,
            column: 1,
        }
    }
}

/// Recoverable tokenizer diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Stable diagnostic code from the tokenizer definition.
    pub code: String,
    /// Position where the diagnostic was reported.
    pub position: SourcePosition,
    /// State that reported the diagnostic.
    pub state: String,
}

/// Trace entry for one tokenizer loop iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenizerTraceEntry {
    /// Position before this transition.
    pub position: SourcePosition,
    /// State before the transition.
    pub from: String,
    /// Current input code point, or `None` for EOF.
    pub input: Option<char>,
    /// State after the transition.
    pub to: String,
    /// Portable actions interpreted by this runtime.
    pub actions: Vec<String>,
    /// Whether the current input was consumed.
    pub consume: bool,
}

/// Runtime errors for tokenizer execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizerError {
    /// The underlying state machine rejected a transition.
    Machine(String),
    /// The definition emitted an action outside the fixed portable vocabulary.
    UnknownAction(String),
    /// An action required a current code point but ran at EOF.
    MissingCurrentCodePoint { action: String },
    /// An action required a current token but none exists.
    MissingCurrentToken { action: String },
    /// A transition loop exceeded the per-input step budget.
    StepLimitExceeded {
        /// Current state when the budget was exceeded.
        state: String,
        /// Current position when the budget was exceeded.
        position: SourcePosition,
        /// Configured step limit.
        limit: usize,
    },
    /// Input was pushed after `finish`.
    AlreadyFinished,
}

impl fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine(error) => write!(f, "{error}"),
            Self::UnknownAction(action) => write!(f, "unknown tokenizer action `{action}`"),
            Self::MissingCurrentCodePoint { action } => {
                write!(f, "action `{action}` requires a current code point")
            }
            Self::MissingCurrentToken { action } => {
                write!(f, "action `{action}` requires a current token")
            }
            Self::StepLimitExceeded {
                state,
                position,
                limit,
            } => write!(
                f,
                "tokenizer exceeded {limit} steps at state `{state}` near char offset {}",
                position.char_offset
            ),
            Self::AlreadyFinished => write!(f, "cannot push input after tokenizer finish"),
        }
    }
}

impl Error for TokenizerError {}

/// Convenient result alias for tokenizer operations.
pub type Result<T> = std::result::Result<T, TokenizerError>;

/// Tokenizer profile runtime.
pub struct Tokenizer {
    machine: EffectfulStateMachine,
    text_buffer: String,
    current_token: Option<CurrentToken>,
    tokens: VecDeque<Token>,
    diagnostics: Vec<Diagnostic>,
    trace: Vec<TokenizerTraceEntry>,
    position: SourcePosition,
    max_steps_per_input: usize,
    finished: bool,
}

impl Tokenizer {
    /// Create a tokenizer from a statically linked effectful state machine.
    pub fn new(machine: EffectfulStateMachine) -> Self {
        Self {
            machine,
            text_buffer: String::new(),
            current_token: None,
            tokens: VecDeque::new(),
            diagnostics: Vec::new(),
            trace: Vec::new(),
            position: SourcePosition::default(),
            max_steps_per_input: 64,
            finished: false,
        }
    }

    /// Set the per-input step limit used to catch non-consuming transition loops.
    pub fn with_max_steps_per_input(mut self, limit: usize) -> Self {
        self.max_steps_per_input = limit.max(1);
        self
    }

    /// Push one chunk of Unicode text into the tokenizer.
    pub fn push(&mut self, chunk: &str) -> Result<()> {
        if self.finished {
            return Err(TokenizerError::AlreadyFinished);
        }

        for ch in chunk.chars() {
            self.process_code_point(ch)?;
        }
        Ok(())
    }

    /// Finish the stream and emit EOF.
    pub fn finish(&mut self) -> Result<()> {
        if self.finished {
            return Ok(());
        }

        for _ in 0..self.max_steps_per_input {
            let before = self.position;
            let from = self.machine.current_state().to_string();
            let step = self
                .machine
                .process(EffectfulInput::end())
                .map_err(TokenizerError::Machine)?;
            self.apply_actions(&step.effects, None, before, &from)?;
            self.trace.push(TokenizerTraceEntry {
                position: before,
                from,
                input: None,
                to: step.to,
                actions: step.effects,
                consume: step.consume,
            });
            if self.machine.is_final() {
                self.finished = true;
                return Ok(());
            }
        }

        Err(TokenizerError::StepLimitExceeded {
            state: self.machine.current_state().to_string(),
            position: self.position,
            limit: self.max_steps_per_input,
        })
    }

    /// Drain all currently queued tokens.
    pub fn drain_tokens(&mut self) -> Vec<Token> {
        self.tokens.drain(..).collect()
    }

    /// Pop the next queued token.
    pub fn next_token(&mut self) -> Option<Token> {
        self.tokens.pop_front()
    }

    /// Current state name from the underlying machine.
    pub fn current_state(&self) -> &str {
        self.machine.current_state()
    }

    /// Current source position.
    pub fn position(&self) -> SourcePosition {
        self.position
    }

    /// Recoverable diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Tokenizer transition trace.
    pub fn trace(&self) -> &[TokenizerTraceEntry] {
        &self.trace
    }

    /// Reset the tokenizer to its initial state.
    pub fn reset(&mut self) {
        self.machine.reset();
        self.text_buffer.clear();
        self.current_token = None;
        self.tokens.clear();
        self.diagnostics.clear();
        self.trace.clear();
        self.position = SourcePosition::default();
        self.finished = false;
    }

    fn process_code_point(&mut self, ch: char) -> Result<()> {
        let event = ch.to_string();
        let mut steps = 0;
        loop {
            if steps >= self.max_steps_per_input {
                return Err(TokenizerError::StepLimitExceeded {
                    state: self.machine.current_state().to_string(),
                    position: self.position,
                    limit: self.max_steps_per_input,
                });
            }
            steps += 1;

            let before = self.position;
            let from = self.machine.current_state().to_string();
            let step = self
                .machine
                .process(EffectfulInput::event(&event))
                .map_err(TokenizerError::Machine)?;
            self.apply_actions(&step.effects, Some(ch), before, &from)?;
            self.trace.push(TokenizerTraceEntry {
                position: before,
                from,
                input: Some(ch),
                to: step.to,
                actions: step.effects,
                consume: step.consume,
            });

            if step.consume {
                self.advance(ch);
                return Ok(());
            }
        }
    }

    fn apply_actions(
        &mut self,
        actions: &[String],
        current: Option<char>,
        position: SourcePosition,
        state: &str,
    ) -> Result<()> {
        for action in actions {
            match action.as_str() {
                "append_text(current)" => self.text_buffer.push(current.ok_or_else(|| {
                    TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    }
                })?),
                "append_text_replacement" => self.text_buffer.push('\u{FFFD}'),
                "flush_text" => self.flush_text(),
                "emit_current_as_text" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.text_buffer.push(ch);
                    self.flush_text();
                }
                "create_start_tag" => {
                    self.current_token = Some(CurrentToken::StartTag {
                        name: String::new(),
                        attributes: Vec::new(),
                        self_closing: false,
                    });
                }
                "create_end_tag" => {
                    self.current_token = Some(CurrentToken::EndTag {
                        name: String::new(),
                    });
                }
                "append_tag_name(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_tag_name(action, ch, false)?;
                }
                "append_tag_name(current_lowercase)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_tag_name(action, ch, true)?;
                }
                "emit_current_token" => self.emit_current_token(action)?,
                "emit(EOF)" => self.tokens.push_back(Token::Eof),
                _ if action.starts_with("append_text(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_text(")
                        .trim_end_matches(')');
                    self.text_buffer.push_str(literal);
                }
                _ if action.starts_with("parse_error(") && action.ends_with(')') => {
                    let code = action
                        .trim_start_matches("parse_error(")
                        .trim_end_matches(')')
                        .to_string();
                    self.diagnostics.push(Diagnostic {
                        code,
                        position,
                        state: state.to_string(),
                    });
                }
                _ => return Err(TokenizerError::UnknownAction(action.clone())),
            }
        }
        Ok(())
    }

    fn flush_text(&mut self) {
        if !self.text_buffer.is_empty() {
            self.tokens
                .push_back(Token::Text(std::mem::take(&mut self.text_buffer)));
        }
    }

    fn append_tag_name(&mut self, action: &str, ch: char, lowercase: bool) -> Result<()> {
        let current_token =
            self.current_token
                .as_mut()
                .ok_or_else(|| TokenizerError::MissingCurrentToken {
                    action: action.to_string(),
                })?;
        match current_token {
            CurrentToken::StartTag { name, .. } | CurrentToken::EndTag { name } => {
                if lowercase {
                    for lowered in ch.to_lowercase() {
                        name.push(lowered);
                    }
                } else {
                    name.push(ch);
                }
                Ok(())
            }
        }
    }

    fn emit_current_token(&mut self, action: &str) -> Result<()> {
        let token =
            self.current_token
                .take()
                .ok_or_else(|| TokenizerError::MissingCurrentToken {
                    action: action.to_string(),
                })?;
        match token {
            CurrentToken::StartTag {
                name,
                attributes,
                self_closing,
            } => self.tokens.push_back(Token::StartTag {
                name,
                attributes,
                self_closing,
            }),
            CurrentToken::EndTag { name } => self.tokens.push_back(Token::EndTag { name }),
        }
        Ok(())
    }

    fn advance(&mut self, ch: char) {
        self.position.byte_offset += ch.len_utf8();
        self.position.char_offset += 1;
        if ch == '\n' {
            self.position.line += 1;
            self.position.column = 1;
        } else {
            self.position.column += 1;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CurrentToken {
    StartTag {
        name: String,
        attributes: Vec<Attribute>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
}
