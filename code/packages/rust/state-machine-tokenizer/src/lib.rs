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
    /// An action required a current attribute but none exists.
    MissingCurrentAttribute { action: String },
    /// An action required a stored return state but none exists.
    MissingReturnState { action: String },
    /// An action expected one current token kind but saw another.
    InvalidCurrentToken {
        /// Portable action that failed.
        action: String,
        /// Expected token kind.
        expected: &'static str,
        /// Actual token kind.
        actual: &'static str,
    },
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
            Self::MissingCurrentAttribute { action } => {
                write!(f, "action `{action}` requires a current attribute")
            }
            Self::MissingReturnState { action } => {
                write!(f, "action `{action}` requires a stored return state")
            }
            Self::InvalidCurrentToken {
                action,
                expected,
                actual,
            } => write!(
                f,
                "action `{action}` expected current token kind `{expected}`, found `{actual}`"
            ),
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
    temporary_buffer: String,
    current_token: Option<CurrentToken>,
    current_attribute: Option<Attribute>,
    return_state: Option<String>,
    last_start_tag: Option<String>,
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
            temporary_buffer: String::new(),
            current_token: None,
            current_attribute: None,
            return_state: None,
            last_start_tag: None,
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

    /// Seed the tokenizer at one of the machine's declared states.
    pub fn with_initial_state(mut self, state: &str) -> Result<Self> {
        self.set_initial_state(state)?;
        Ok(self)
    }

    /// Force the tokenizer into one of the machine's declared states.
    pub fn set_initial_state(&mut self, state: &str) -> Result<()> {
        self.machine
            .set_current_state(state.to_string())
            .map_err(TokenizerError::Machine)
    }

    /// Seed the last emitted start-tag name used by HTML tokenizer submodes.
    pub fn with_last_start_tag(mut self, tag: impl Into<String>) -> Self {
        self.set_last_start_tag(tag);
        self
    }

    /// Store the last emitted start-tag name used by HTML tokenizer submodes.
    pub fn set_last_start_tag(&mut self, tag: impl Into<String>) {
        self.last_start_tag = Some(tag.into());
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
                to: self.machine.current_state().to_string(),
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
        self.temporary_buffer.clear();
        self.current_token = None;
        self.current_attribute = None;
        self.return_state = None;
        self.last_start_tag = None;
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
                to: self.machine.current_state().to_string(),
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
                    self.current_attribute = None;
                }
                "create_end_tag" => {
                    self.current_token = Some(CurrentToken::EndTag {
                        name: String::new(),
                    });
                    self.current_attribute = None;
                }
                "create_comment" => {
                    self.current_token = Some(CurrentToken::Comment {
                        data: String::new(),
                    });
                    self.current_attribute = None;
                }
                "create_doctype" => {
                    self.current_token = Some(CurrentToken::Doctype {
                        name: None,
                        force_quirks: false,
                    });
                    self.current_attribute = None;
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
                "start_attribute" => {
                    self.current_attribute = Some(Attribute {
                        name: String::new(),
                        value: String::new(),
                    });
                }
                "append_attribute_name(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_attribute_name(action, ch, false)?;
                }
                "append_attribute_name(current_lowercase)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_attribute_name(action, ch, true)?;
                }
                "append_attribute_value(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_attribute_value(action, ch)?;
                }
                "commit_attribute" => self.commit_attribute(action)?,
                "mark_self_closing" => self.mark_self_closing(action)?,
                "append_comment(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_comment(action, ch, false)?;
                }
                "append_comment(current_lowercase)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_comment(action, ch, true)?;
                }
                "append_doctype_name(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_doctype_name(action, ch, false)?;
                }
                "append_doctype_name(current_lowercase)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_doctype_name(action, ch, true)?;
                }
                "mark_force_quirks" => self.mark_force_quirks(action)?,
                "clear_temporary_buffer" => self.temporary_buffer.clear(),
                "append_temporary_buffer(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.temporary_buffer.push(ch);
                }
                "append_temporary_buffer(current_lowercase)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    push_lowercase(&mut self.temporary_buffer, ch);
                }
                "append_temporary_buffer_to_text" => {
                    self.text_buffer.push_str(&self.temporary_buffer);
                    self.temporary_buffer.clear();
                }
                "append_temporary_buffer_to_attribute_value" => {
                    let temporary_buffer = std::mem::take(&mut self.temporary_buffer);
                    self.attribute_mut(action)?
                        .value
                        .push_str(&temporary_buffer);
                }
                "append_numeric_character_reference_to_text" => {
                    let ch = numeric_character_reference(&self.temporary_buffer);
                    self.temporary_buffer.clear();
                    self.text_buffer.push(ch);
                }
                "append_numeric_character_reference_to_attribute_value" => {
                    let ch = numeric_character_reference(&self.temporary_buffer);
                    self.temporary_buffer.clear();
                    self.attribute_mut(action)?.value.push(ch);
                }
                "discard_current_token" => {
                    self.current_token = None;
                    self.current_attribute = None;
                }
                "switch_to_return_state" => {
                    let return_state = self.return_state.clone().ok_or_else(|| {
                        TokenizerError::MissingReturnState {
                            action: action.clone(),
                        }
                    })?;
                    self.machine
                        .set_current_state(return_state)
                        .map_err(TokenizerError::Machine)?;
                }
                "emit_current_token" => self.emit_current_token(action)?,
                "emit_rcdata_end_tag_or_text" => self.emit_rcdata_end_tag_or_text(action)?,
                "emit(EOF)" => self.tokens.push_back(Token::Eof),
                _ if action.starts_with("set_return_state(") && action.ends_with(')') => {
                    let state = action
                        .trim_start_matches("set_return_state(")
                        .trim_end_matches(')');
                    if !self.machine.has_state(state) {
                        return Err(TokenizerError::Machine(format!("Unknown state '{state}'")));
                    }
                    self.return_state = Some(state.to_string());
                }
                _ if action.starts_with("switch_to(") && action.ends_with(')') => {
                    let state = action
                        .trim_start_matches("switch_to(")
                        .trim_end_matches(')');
                    self.machine
                        .set_current_state(state.to_string())
                        .map_err(TokenizerError::Machine)?;
                }
                _ if action.starts_with("append_text(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_text(")
                        .trim_end_matches(')');
                    self.text_buffer.push_str(literal);
                }
                _ if action.starts_with("append_attribute_name(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_attribute_name(")
                        .trim_end_matches(')');
                    self.attribute_mut(action)?.name.push_str(literal);
                }
                _ if action.starts_with("append_attribute_value(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_attribute_value(")
                        .trim_end_matches(')');
                    self.attribute_mut(action)?.value.push_str(literal);
                }
                _ if action.starts_with("append_comment(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_comment(")
                        .trim_end_matches(')');
                    self.comment_data_mut(action)?.push_str(literal);
                }
                _ if action.starts_with("append_doctype_name(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_doctype_name(")
                        .trim_end_matches(')');
                    self.doctype_name_mut(action)?.push_str(literal);
                }
                _ if action.starts_with("append_temporary_buffer(") && action.ends_with(')') => {
                    let literal = action
                        .trim_start_matches("append_temporary_buffer(")
                        .trim_end_matches(')');
                    self.temporary_buffer.push_str(literal);
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
        match self.current_token_mut(action)? {
            CurrentToken::StartTag { name, .. } | CurrentToken::EndTag { name } => {
                if lowercase {
                    push_lowercase(name, ch);
                } else {
                    name.push(ch);
                }
                Ok(())
            }
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "start-tag-or-end-tag",
                actual: other.kind_name(),
            }),
        }
    }

    fn append_attribute_name(&mut self, action: &str, ch: char, lowercase: bool) -> Result<()> {
        let attribute = self.attribute_mut(action)?;
        if lowercase {
            push_lowercase(&mut attribute.name, ch);
        } else {
            attribute.name.push(ch);
        }
        Ok(())
    }

    fn append_attribute_value(&mut self, action: &str, ch: char) -> Result<()> {
        self.attribute_mut(action)?.value.push(ch);
        Ok(())
    }

    fn commit_attribute(&mut self, action: &str) -> Result<()> {
        let attribute = self.current_attribute.take().ok_or_else(|| {
            TokenizerError::MissingCurrentAttribute {
                action: action.to_string(),
            }
        })?;
        match self.current_token_mut(action)? {
            CurrentToken::StartTag { attributes, .. } => {
                attributes.push(attribute);
                Ok(())
            }
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "start-tag",
                actual: other.kind_name(),
            }),
        }
    }

    fn mark_self_closing(&mut self, action: &str) -> Result<()> {
        match self.current_token_mut(action)? {
            CurrentToken::StartTag { self_closing, .. } => {
                *self_closing = true;
                Ok(())
            }
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "start-tag",
                actual: other.kind_name(),
            }),
        }
    }

    fn append_comment(&mut self, action: &str, ch: char, lowercase: bool) -> Result<()> {
        let data = self.comment_data_mut(action)?;
        if lowercase {
            push_lowercase(data, ch);
        } else {
            data.push(ch);
        }
        Ok(())
    }

    fn append_doctype_name(&mut self, action: &str, ch: char, lowercase: bool) -> Result<()> {
        let name = self.doctype_name_mut(action)?;
        if lowercase {
            push_lowercase(name, ch);
        } else {
            name.push(ch);
        }
        Ok(())
    }

    fn mark_force_quirks(&mut self, action: &str) -> Result<()> {
        match self.current_token_mut(action)? {
            CurrentToken::Doctype { force_quirks, .. } => {
                *force_quirks = true;
                Ok(())
            }
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "doctype",
                actual: other.kind_name(),
            }),
        }
    }

    fn emit_current_token(&mut self, action: &str) -> Result<()> {
        if matches!(self.current_token, Some(CurrentToken::StartTag { .. }))
            && self.current_attribute.is_some()
        {
            self.commit_attribute("commit_attribute")?;
        }
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
            } => {
                self.last_start_tag = Some(name.clone());
                self.tokens.push_back(Token::StartTag {
                    name,
                    attributes,
                    self_closing,
                });
            }
            CurrentToken::EndTag { name } => self.tokens.push_back(Token::EndTag { name }),
            CurrentToken::Comment { data } => self.tokens.push_back(Token::Comment(data)),
            CurrentToken::Doctype { name, force_quirks } => {
                self.tokens.push_back(Token::Doctype { name, force_quirks })
            }
        }
        Ok(())
    }

    fn emit_rcdata_end_tag_or_text(&mut self, action: &str) -> Result<()> {
        let candidate = match self.current_token.as_ref() {
            Some(CurrentToken::EndTag { name }) => name.clone(),
            Some(other) => {
                return Err(TokenizerError::InvalidCurrentToken {
                    action: action.to_string(),
                    expected: "end-tag",
                    actual: other.kind_name(),
                })
            }
            None => {
                return Err(TokenizerError::MissingCurrentToken {
                    action: action.to_string(),
                })
            }
        };

        if self.last_start_tag.as_deref() == Some(candidate.as_str()) {
            self.flush_text();
            self.emit_current_token("emit_current_token")?;
        } else {
            self.current_token = None;
            self.current_attribute = None;
            self.text_buffer.push_str("</");
            self.text_buffer.push_str(&self.temporary_buffer);
            self.text_buffer.push('>');
        }
        self.temporary_buffer.clear();
        Ok(())
    }

    fn current_token_mut(&mut self, action: &str) -> Result<&mut CurrentToken> {
        self.current_token
            .as_mut()
            .ok_or_else(|| TokenizerError::MissingCurrentToken {
                action: action.to_string(),
            })
    }

    fn attribute_mut(&mut self, action: &str) -> Result<&mut Attribute> {
        self.current_attribute
            .as_mut()
            .ok_or_else(|| TokenizerError::MissingCurrentAttribute {
                action: action.to_string(),
            })
    }

    fn comment_data_mut(&mut self, action: &str) -> Result<&mut String> {
        match self.current_token_mut(action)? {
            CurrentToken::Comment { data } => Ok(data),
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "comment",
                actual: other.kind_name(),
            }),
        }
    }

    fn doctype_name_mut(&mut self, action: &str) -> Result<&mut String> {
        match self.current_token_mut(action)? {
            CurrentToken::Doctype { name, .. } => Ok(name.get_or_insert_with(String::new)),
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "doctype",
                actual: other.kind_name(),
            }),
        }
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
    Comment {
        data: String,
    },
    Doctype {
        name: Option<String>,
        force_quirks: bool,
    },
}

impl CurrentToken {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::StartTag { .. } => "start-tag",
            Self::EndTag { .. } => "end-tag",
            Self::Comment { .. } => "comment",
            Self::Doctype { .. } => "doctype",
        }
    }
}

fn push_lowercase(target: &mut String, ch: char) {
    for lowered in ch.to_lowercase() {
        target.push(lowered);
    }
}

fn numeric_character_reference(buffer: &str) -> char {
    let Some(raw_digits) = buffer.strip_prefix("&#") else {
        return '\u{FFFD}';
    };
    let (radix, digits) = match raw_digits
        .strip_prefix('x')
        .or_else(|| raw_digits.strip_prefix('X'))
    {
        Some(hex_digits) => (16, hex_digits),
        None => (10, raw_digits),
    };
    if digits.is_empty() {
        return '\u{FFFD}';
    }

    u32::from_str_radix(digits, radix)
        .ok()
        .and_then(|value| {
            if value == 0 {
                None
            } else {
                char::from_u32(value)
            }
        })
        .unwrap_or('\u{FFFD}')
}
