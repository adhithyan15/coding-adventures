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
        /// Optional public identifier.
        public_identifier: Option<String>,
        /// Optional system identifier.
        system_identifier: Option<String>,
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
                "append_attribute_value_replacement" => {
                    self.attribute_mut(action)?.value.push('\u{FFFD}');
                }
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
                        public_identifier: None,
                        system_identifier: None,
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
                "append_tag_name_replacement" => {
                    self.append_tag_name(action, '\u{FFFD}', false)?;
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
                "append_attribute_name_replacement" => {
                    self.append_attribute_name(action, '\u{FFFD}', false)?;
                }
                "append_attribute_value(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_attribute_value(action, ch)?;
                }
                "commit_attribute" => self.commit_attribute(action)?,
                "commit_attribute_dedup" => self.commit_attribute_dedup(action, position, state)?,
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
                "append_comment_replacement" => {
                    self.append_comment(action, '\u{FFFD}', false)?;
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
                "append_doctype_name_replacement" => {
                    self.append_doctype_name(action, '\u{FFFD}', false)?;
                }
                "set_doctype_public_identifier_empty" => {
                    self.set_doctype_public_identifier_empty(action)?
                }
                "append_doctype_public_identifier(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_doctype_public_identifier(action, ch)?;
                }
                "append_doctype_public_identifier_replacement" => {
                    self.append_doctype_public_identifier(action, '\u{FFFD}')?;
                }
                "set_doctype_system_identifier_empty" => {
                    self.set_doctype_system_identifier_empty(action)?
                }
                "append_doctype_system_identifier(current)" => {
                    let ch = current.ok_or_else(|| TokenizerError::MissingCurrentCodePoint {
                        action: action.clone(),
                    })?;
                    self.append_doctype_system_identifier(action, ch)?;
                }
                "append_doctype_system_identifier_replacement" => {
                    self.append_doctype_system_identifier(action, '\u{FFFD}')?;
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
                    let reference = numeric_character_reference(&self.temporary_buffer);
                    self.temporary_buffer.clear();
                    self.text_buffer.push(reference.character);
                    self.record_diagnostics(reference.diagnostics, position, state);
                }
                "append_numeric_character_reference_to_attribute_value" => {
                    let reference = numeric_character_reference(&self.temporary_buffer);
                    self.temporary_buffer.clear();
                    self.attribute_mut(action)?.value.push(reference.character);
                    self.record_diagnostics(reference.diagnostics, position, state);
                }
                "append_named_character_reference_to_text" => {
                    let replacement =
                        named_character_reference(&self.temporary_buffer).unwrap_or("\u{FFFD}");
                    self.temporary_buffer.clear();
                    self.text_buffer.push_str(replacement);
                }
                "append_named_character_reference_to_attribute_value" => {
                    let replacement =
                        named_character_reference(&self.temporary_buffer).unwrap_or("\u{FFFD}");
                    self.temporary_buffer.clear();
                    self.attribute_mut(action)?.value.push_str(replacement);
                }
                "append_named_character_reference_or_temporary_buffer_to_text" => {
                    if let Some(reference) =
                        consume_named_character_reference(&self.temporary_buffer, false)
                    {
                        let remainder = reference.remainder.to_string();
                        let missing_semicolon = reference.missing_semicolon;
                        self.text_buffer.push_str(reference.replacement);
                        self.text_buffer.push_str(&remainder);
                        self.temporary_buffer.clear();
                        if missing_semicolon {
                            self.diagnostics.push(Diagnostic {
                                code: "missing-semicolon-after-character-reference".to_string(),
                                position,
                                state: state.to_string(),
                            });
                        }
                    } else {
                        self.text_buffer.push_str(&self.temporary_buffer);
                        self.temporary_buffer.clear();
                    }
                }
                "append_named_character_reference_or_temporary_buffer_to_attribute_value" => {
                    if let Some(reference) =
                        consume_named_character_reference(&self.temporary_buffer, true)
                    {
                        let remainder = reference.remainder.to_string();
                        let replacement = reference.replacement;
                        let missing_semicolon = reference.missing_semicolon;
                        self.temporary_buffer.clear();
                        self.attribute_mut(action)?.value.push_str(replacement);
                        self.attribute_mut(action)?.value.push_str(&remainder);
                        if missing_semicolon {
                            self.diagnostics.push(Diagnostic {
                                code: "missing-semicolon-after-character-reference".to_string(),
                                position,
                                state: state.to_string(),
                            });
                        }
                    } else {
                        let temporary_buffer = std::mem::take(&mut self.temporary_buffer);
                        self.attribute_mut(action)?
                            .value
                            .push_str(&temporary_buffer);
                    }
                }
                "recover_named_character_reference_to_text" => {
                    if let Some(reference) =
                        consume_named_character_reference(&self.temporary_buffer, false)
                    {
                        let remainder = reference.remainder.to_string();
                        self.text_buffer.push_str(reference.replacement);
                        self.text_buffer.push_str(&remainder);
                        self.temporary_buffer.clear();
                        self.diagnostics.push(Diagnostic {
                            code: "missing-semicolon-after-character-reference".to_string(),
                            position,
                            state: state.to_string(),
                        });
                    } else {
                        self.text_buffer.push_str(&self.temporary_buffer);
                        self.temporary_buffer.clear();
                    }
                }
                "recover_named_character_reference_to_attribute_value" => {
                    if let Some(reference) =
                        consume_named_character_reference(&self.temporary_buffer, true)
                    {
                        let remainder = reference.remainder.to_string();
                        let replacement = reference.replacement;
                        self.temporary_buffer.clear();
                        self.attribute_mut(action)?.value.push_str(replacement);
                        self.attribute_mut(action)?.value.push_str(&remainder);
                        self.diagnostics.push(Diagnostic {
                            code: "missing-semicolon-after-character-reference".to_string(),
                            position,
                            state: state.to_string(),
                        });
                    } else {
                        let temporary_buffer = std::mem::take(&mut self.temporary_buffer);
                        self.attribute_mut(action)?
                            .value
                            .push_str(&temporary_buffer);
                    }
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
                _ if action.starts_with("switch_to_if_temporary_buffer_equals(")
                    && action.ends_with(')') =>
                {
                    let arguments = action
                        .trim_start_matches("switch_to_if_temporary_buffer_equals(")
                        .trim_end_matches(')');
                    let parts = arguments.split(',').map(str::trim).collect::<Vec<_>>();
                    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
                        return Err(TokenizerError::UnknownAction(action.clone()));
                    }
                    let target = if self.temporary_buffer == parts[0] {
                        parts[1]
                    } else {
                        parts[2]
                    };
                    self.machine
                        .set_current_state(target.to_string())
                        .map_err(TokenizerError::Machine)?;
                }
                "emit_current_token" => self.emit_current_token(action)?,
                "emit_rcdata_end_tag_or_text" => self.emit_rcdata_end_tag_or_text(action)?,
                "emit_rcdata_end_tag_with_trailing_solidus_or_text" => {
                    self.emit_rcdata_end_tag_with_trailing_solidus_or_text(action, position, state)?
                }
                "emit_rcdata_end_tag_with_whitespace_or_text" => {
                    self.emit_rcdata_end_tag_with_whitespace_or_text(action, position, state)?
                }
                "emit_rcdata_end_tag_with_attributes_or_text" => {
                    self.emit_rcdata_end_tag_with_attributes_or_text(action, position, state)?
                }
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
                _ if action.starts_with("append_doctype_public_identifier(")
                    && action.ends_with(')') =>
                {
                    let literal = action
                        .trim_start_matches("append_doctype_public_identifier(")
                        .trim_end_matches(')');
                    self.doctype_public_identifier_mut(action)?
                        .push_str(literal);
                }
                _ if action.starts_with("append_doctype_system_identifier(")
                    && action.ends_with(')') =>
                {
                    let literal = action
                        .trim_start_matches("append_doctype_system_identifier(")
                        .trim_end_matches(')');
                    self.doctype_system_identifier_mut(action)?
                        .push_str(literal);
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

    fn commit_attribute_dedup(
        &mut self,
        action: &str,
        position: SourcePosition,
        state: &str,
    ) -> Result<()> {
        let attribute = self.current_attribute.take().ok_or_else(|| {
            TokenizerError::MissingCurrentAttribute {
                action: action.to_string(),
            }
        })?;
        let duplicate = match self.current_token_mut(action)? {
            CurrentToken::StartTag { attributes, .. } => {
                if attributes
                    .iter()
                    .any(|existing| existing.name == attribute.name)
                {
                    true
                } else {
                    attributes.push(attribute);
                    false
                }
            }
            other => {
                return Err(TokenizerError::InvalidCurrentToken {
                    action: action.to_string(),
                    expected: "start-tag",
                    actual: other.kind_name(),
                })
            }
        };
        if duplicate {
            self.diagnostics.push(Diagnostic {
                code: "duplicate-attribute".to_string(),
                position,
                state: state.to_string(),
            });
        }
        Ok(())
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

    fn set_doctype_public_identifier_empty(&mut self, action: &str) -> Result<()> {
        match self.current_token_mut(action)? {
            CurrentToken::Doctype {
                public_identifier, ..
            } => {
                public_identifier.get_or_insert_with(String::new);
                Ok(())
            }
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "doctype",
                actual: other.kind_name(),
            }),
        }
    }

    fn append_doctype_public_identifier(&mut self, action: &str, ch: char) -> Result<()> {
        self.doctype_public_identifier_mut(action)?.push(ch);
        Ok(())
    }

    fn set_doctype_system_identifier_empty(&mut self, action: &str) -> Result<()> {
        match self.current_token_mut(action)? {
            CurrentToken::Doctype {
                system_identifier, ..
            } => {
                system_identifier.get_or_insert_with(String::new);
                Ok(())
            }
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "doctype",
                actual: other.kind_name(),
            }),
        }
    }

    fn append_doctype_system_identifier(&mut self, action: &str, ch: char) -> Result<()> {
        self.doctype_system_identifier_mut(action)?.push(ch);
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

    fn record_diagnostics(
        &mut self,
        diagnostics: Vec<&'static str>,
        position: SourcePosition,
        state: &str,
    ) {
        self.diagnostics
            .extend(diagnostics.into_iter().map(|code| Diagnostic {
                code: code.to_string(),
                position,
                state: state.to_string(),
            }));
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
            CurrentToken::Doctype {
                name,
                public_identifier,
                system_identifier,
                force_quirks,
            } => self.tokens.push_back(Token::Doctype {
                name,
                public_identifier,
                system_identifier,
                force_quirks,
            }),
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

    fn emit_rcdata_end_tag_with_trailing_solidus_or_text(
        &mut self,
        action: &str,
        position: SourcePosition,
        state: &str,
    ) -> Result<()> {
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
            self.diagnostics.push(Diagnostic {
                code: "end-tag-with-trailing-solidus".to_string(),
                position,
                state: state.to_string(),
            });
            self.flush_text();
            self.emit_current_token("emit_current_token")?;
        } else {
            self.current_token = None;
            self.current_attribute = None;
            self.text_buffer.push_str("</");
            self.text_buffer.push_str(&self.temporary_buffer);
            self.text_buffer.push_str("/>");
        }
        self.temporary_buffer.clear();
        Ok(())
    }

    fn emit_rcdata_end_tag_with_whitespace_or_text(
        &mut self,
        action: &str,
        position: SourcePosition,
        state: &str,
    ) -> Result<()> {
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
            self.diagnostics.push(Diagnostic {
                code: "unexpected-whitespace-after-end-tag-name".to_string(),
                position,
                state: state.to_string(),
            });
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

    fn emit_rcdata_end_tag_with_attributes_or_text(
        &mut self,
        action: &str,
        position: SourcePosition,
        state: &str,
    ) -> Result<()> {
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
            self.diagnostics.push(Diagnostic {
                code: "end-tag-with-attributes".to_string(),
                position,
                state: state.to_string(),
            });
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

    fn doctype_public_identifier_mut(&mut self, action: &str) -> Result<&mut String> {
        match self.current_token_mut(action)? {
            CurrentToken::Doctype {
                public_identifier, ..
            } => Ok(public_identifier.get_or_insert_with(String::new)),
            other => Err(TokenizerError::InvalidCurrentToken {
                action: action.to_string(),
                expected: "doctype",
                actual: other.kind_name(),
            }),
        }
    }

    fn doctype_system_identifier_mut(&mut self, action: &str) -> Result<&mut String> {
        match self.current_token_mut(action)? {
            CurrentToken::Doctype {
                system_identifier, ..
            } => Ok(system_identifier.get_or_insert_with(String::new)),
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
        public_identifier: Option<String>,
        system_identifier: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct NumericCharacterReference {
    character: char,
    diagnostics: Vec<&'static str>,
}

fn numeric_character_reference(buffer: &str) -> NumericCharacterReference {
    let Some(raw_digits) = buffer.strip_prefix("&#") else {
        return NumericCharacterReference {
            character: '\u{FFFD}',
            diagnostics: vec!["absence-of-digits-in-numeric-character-reference"],
        };
    };
    let (radix, digits) = match raw_digits
        .strip_prefix('x')
        .or_else(|| raw_digits.strip_prefix('X'))
    {
        Some(hex_digits) => (16, hex_digits),
        None => (10, raw_digits),
    };
    if digits.is_empty() {
        return NumericCharacterReference {
            character: '\u{FFFD}',
            diagnostics: vec!["absence-of-digits-in-numeric-character-reference"],
        };
    }

    let Ok(value) = u32::from_str_radix(digits, radix) else {
        return NumericCharacterReference {
            character: '\u{FFFD}',
            diagnostics: vec!["character-reference-outside-unicode-range"],
        };
    };

    if value == 0 {
        return NumericCharacterReference {
            character: '\u{FFFD}',
            diagnostics: vec!["null-character-reference"],
        };
    }

    if value > 0x10FFFF {
        return NumericCharacterReference {
            character: '\u{FFFD}',
            diagnostics: vec!["character-reference-outside-unicode-range"],
        };
    }

    if (0xD800..=0xDFFF).contains(&value) {
        return NumericCharacterReference {
            character: '\u{FFFD}',
            diagnostics: vec!["surrogate-character-reference"],
        };
    }

    let mut diagnostics = Vec::new();
    if is_noncharacter(value) {
        diagnostics.push("noncharacter-character-reference");
    }

    let character = if let Some(replacement) = windows_1252_control_replacement(value) {
        diagnostics.push("control-character-reference");
        replacement
    } else {
        let character = char::from_u32(value).unwrap_or('\u{FFFD}');
        if is_control_character_reference(value) {
            diagnostics.push("control-character-reference");
        }
        character
    };

    NumericCharacterReference {
        character,
        diagnostics,
    }
}

fn is_noncharacter(value: u32) -> bool {
    (0xFDD0..=0xFDEF).contains(&value)
        || matches!(
            value,
            0xFFFE
                | 0xFFFF
                | 0x1FFFE
                | 0x1FFFF
                | 0x2FFFE
                | 0x2FFFF
                | 0x3FFFE
                | 0x3FFFF
                | 0x4FFFE
                | 0x4FFFF
                | 0x5FFFE
                | 0x5FFFF
                | 0x6FFFE
                | 0x6FFFF
                | 0x7FFFE
                | 0x7FFFF
                | 0x8FFFE
                | 0x8FFFF
                | 0x9FFFE
                | 0x9FFFF
                | 0xAFFFE
                | 0xAFFFF
                | 0xBFFFE
                | 0xBFFFF
                | 0xCFFFE
                | 0xCFFFF
                | 0xDFFFE
                | 0xDFFFF
                | 0xEFFFE
                | 0xEFFFF
                | 0xFFFFE
                | 0xFFFFF
                | 0x10FFFE
                | 0x10FFFF
        )
}

fn is_control_character_reference(value: u32) -> bool {
    value == 0x0D || (is_control(value) && !is_ascii_whitespace_control(value))
}

fn is_control(value: u32) -> bool {
    value <= 0x1F || (0x7F..=0x9F).contains(&value)
}

fn is_ascii_whitespace_control(value: u32) -> bool {
    matches!(value, 0x09 | 0x0A | 0x0C | 0x20)
}

fn windows_1252_control_replacement(value: u32) -> Option<char> {
    match value {
        0x80 => Some('\u{20AC}'),
        0x82 => Some('\u{201A}'),
        0x83 => Some('\u{0192}'),
        0x84 => Some('\u{201E}'),
        0x85 => Some('\u{2026}'),
        0x86 => Some('\u{2020}'),
        0x87 => Some('\u{2021}'),
        0x88 => Some('\u{02C6}'),
        0x89 => Some('\u{2030}'),
        0x8A => Some('\u{0160}'),
        0x8B => Some('\u{2039}'),
        0x8C => Some('\u{0152}'),
        0x8E => Some('\u{017D}'),
        0x91 => Some('\u{2018}'),
        0x92 => Some('\u{2019}'),
        0x93 => Some('\u{201C}'),
        0x94 => Some('\u{201D}'),
        0x95 => Some('\u{2022}'),
        0x96 => Some('\u{2013}'),
        0x97 => Some('\u{2014}'),
        0x98 => Some('\u{02DC}'),
        0x99 => Some('\u{2122}'),
        0x9A => Some('\u{0161}'),
        0x9B => Some('\u{203A}'),
        0x9C => Some('\u{0153}'),
        0x9E => Some('\u{017E}'),
        0x9F => Some('\u{0178}'),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ConsumedNamedCharacterReference<'a> {
    replacement: &'static str,
    remainder: &'a str,
    missing_semicolon: bool,
}

fn consume_named_character_reference(
    buffer: &str,
    in_attribute: bool,
) -> Option<ConsumedNamedCharacterReference<'_>> {
    let body = buffer.strip_prefix('&').unwrap_or(buffer);
    for end in body
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(body.len()))
        .filter(|end| *end > 0)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let candidate = &body[..end];
        let (name, missing_semicolon) = match candidate.strip_suffix(';') {
            Some(name) => (name, false),
            None => (candidate, true),
        };
        let Some(replacement) = named_character_reference_name(name) else {
            continue;
        };
        let remainder = &body[end..];

        if missing_semicolon
            && in_attribute
            && remainder
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '=')
        {
            return None;
        }

        return Some(ConsumedNamedCharacterReference {
            replacement,
            remainder,
            missing_semicolon,
        });
    }

    None
}

fn named_character_reference(buffer: &str) -> Option<&'static str> {
    let name = buffer.strip_prefix('&').unwrap_or(buffer);
    let name = name.strip_suffix(';').unwrap_or(name);
    named_character_reference_name(name)
}

fn named_character_reference_name(name: &str) -> Option<&'static str> {
    match name {
        "AElig" => Some("\u{00C6}"),
        "Aacute" => Some("\u{00C1}"),
        "Acirc" => Some("\u{00C2}"),
        "Agrave" => Some("\u{00C0}"),
        "Alpha" => Some("\u{0391}"),
        "And" => Some("\u{2A53}"),
        "ApplyFunction" => Some("\u{2061}"),
        "Aring" => Some("\u{00C5}"),
        "Atilde" => Some("\u{00C3}"),
        "Auml" => Some("\u{00C4}"),
        "Because" => Some("\u{2235}"),
        "Beta" => Some("\u{0392}"),
        "Bumpeq" => Some("\u{224E}"),
        "Cap" => Some("\u{22D2}"),
        "Ccedil" => Some("\u{00C7}"),
        "CenterDot" => Some("\u{00B7}"),
        "Chi" => Some("\u{03A7}"),
        "CircleDot" => Some("\u{2299}"),
        "CircleMinus" => Some("\u{2296}"),
        "CirclePlus" => Some("\u{2295}"),
        "CircleTimes" => Some("\u{2297}"),
        "COPY" | "copy" => Some("\u{00A9}"),
        "CapitalDifferentialD" => Some("\u{2145}"),
        "CloseCurlyDoubleQuote" => Some("\u{201D}"),
        "CloseCurlyQuote" => Some("\u{2019}"),
        "ContourIntegral" => Some("\u{222E}"),
        "Congruent" => Some("\u{2261}"),
        "Coproduct" => Some("\u{2210}"),
        "Cross" => Some("\u{2A2F}"),
        "Cup" => Some("\u{22D3}"),
        "CupCap" => Some("\u{224D}"),
        "DD" => Some("\u{2145}"),
        "Dagger" => Some("\u{2021}"),
        "Delta" => Some("\u{0394}"),
        "DiacriticalAcute" => Some("\u{00B4}"),
        "DiacriticalDot" => Some("\u{02D9}"),
        "DiacriticalDoubleAcute" => Some("\u{02DD}"),
        "DiacriticalGrave" => Some("`"),
        "DiacriticalTilde" => Some("\u{02DC}"),
        "DifferentialD" => Some("\u{2146}"),
        "DotEqual" => Some("\u{2250}"),
        "DoubleDownArrow" => Some("\u{21D3}"),
        "DoubleLeftArrow" => Some("\u{21D0}"),
        "DoubleLeftRightArrow" => Some("\u{21D4}"),
        "DoubleRightArrow" => Some("\u{21D2}"),
        "DoubleUpArrow" => Some("\u{21D1}"),
        "DoubleUpDownArrow" => Some("\u{21D5}"),
        "DownArrow" => Some("\u{2193}"),
        "DownArrowBar" => Some("\u{2913}"),
        "DownArrowUpArrow" => Some("\u{21F5}"),
        "DownLeftVector" => Some("\u{21BD}"),
        "DownLeftVectorBar" => Some("\u{2956}"),
        "DownRightVector" => Some("\u{21C1}"),
        "DownRightVectorBar" => Some("\u{2957}"),
        "DoubleContourIntegral" => Some("\u{222F}"),
        "DoubleVerticalBar" => Some("\u{2225}"),
        "ETH" => Some("\u{00D0}"),
        "Eacute" => Some("\u{00C9}"),
        "Ecirc" => Some("\u{00CA}"),
        "Egrave" => Some("\u{00C8}"),
        "Element" => Some("\u{2208}"),
        "Epsilon" => Some("\u{0395}"),
        "Equal" => Some("\u{2A75}"),
        "EqualTilde" => Some("\u{2242}"),
        "Eta" => Some("\u{0397}"),
        "Euml" => Some("\u{00CB}"),
        "EmptySmallSquare" => Some("\u{25FB}"),
        "EmptyVerySmallSquare" => Some("\u{25AB}"),
        "ExponentialE" => Some("\u{2147}"),
        "Exists" => Some("\u{2203}"),
        "FilledSmallSquare" => Some("\u{25FC}"),
        "FilledVerySmallSquare" => Some("\u{25AA}"),
        "Gamma" => Some("\u{0393}"),
        "Gammad" => Some("\u{03DC}"),
        "GT" | "gt" => Some(">"),
        "GreaterEqual" => Some("\u{2265}"),
        "GreaterFullEqual" => Some("\u{2267}"),
        "GreaterEqualLess" => Some("\u{22DB}"),
        "GreaterGreater" => Some("\u{2AA2}"),
        "GreaterLess" => Some("\u{2277}"),
        "GreaterSlantEqual" => Some("\u{2A7E}"),
        "GreaterTilde" => Some("\u{2273}"),
        "HumpDownHump" => Some("\u{224E}"),
        "HumpEqual" => Some("\u{224F}"),
        "Iacute" => Some("\u{00CD}"),
        "Icirc" => Some("\u{00CE}"),
        "Igrave" => Some("\u{00CC}"),
        "ImaginaryI" => Some("\u{2148}"),
        "Intersection" => Some("\u{22C2}"),
        "Integral" => Some("\u{222B}"),
        "InvisibleComma" => Some("\u{2063}"),
        "InvisibleTimes" => Some("\u{2062}"),
        "Iota" => Some("\u{0399}"),
        "Iuml" => Some("\u{00CF}"),
        "Kappa" => Some("\u{039A}"),
        "LT" | "lt" => Some("<"),
        "Lambda" => Some("\u{039B}"),
        "LeftAngleBracket" => Some("\u{27E8}"),
        "LeftArrow" => Some("\u{2190}"),
        "LeftArrowBar" => Some("\u{21E4}"),
        "LeftArrowRightArrow" => Some("\u{21C6}"),
        "LeftCeiling" => Some("\u{2308}"),
        "LeftDoubleBracket" => Some("\u{27E6}"),
        "LeftDownVector" => Some("\u{21C3}"),
        "LeftDownVectorBar" => Some("\u{2959}"),
        "LeftFloor" => Some("\u{230A}"),
        "LeftRightArrow" => Some("\u{2194}"),
        "LeftTeeArrow" => Some("\u{21A4}"),
        "LeftTriangle" => Some("\u{22B2}"),
        "LeftUpVector" => Some("\u{21BF}"),
        "LeftUpVectorBar" => Some("\u{2958}"),
        "LeftVector" => Some("\u{21BC}"),
        "LeftVectorBar" => Some("\u{2952}"),
        "LessEqualGreater" => Some("\u{22DA}"),
        "LessFullEqual" => Some("\u{2266}"),
        "LessGreater" => Some("\u{2276}"),
        "LessLess" => Some("\u{2AA1}"),
        "LessSlantEqual" => Some("\u{2A7D}"),
        "LessTilde" => Some("\u{2272}"),
        "LongLeftArrow" => Some("\u{27F5}"),
        "LongLeftRightArrow" => Some("\u{27F7}"),
        "LongRightArrow" => Some("\u{27F6}"),
        "Longleftarrow" => Some("\u{27F8}"),
        "Longleftrightarrow" => Some("\u{27FA}"),
        "Longrightarrow" => Some("\u{27F9}"),
        "Map" => Some("\u{2905}"),
        "MeasuredAngle" => Some("\u{2221}"),
        "MediumSpace" => Some("\u{205F}"),
        "Mu" => Some("\u{039C}"),
        "NBSP" | "nbsp" => Some("\u{00A0}"),
        "NegativeMediumSpace" => Some("\u{200B}"),
        "NegativeThickSpace" => Some("\u{200B}"),
        "NegativeThinSpace" => Some("\u{200B}"),
        "NegativeVeryThinSpace" => Some("\u{200B}"),
        "NestedGreaterGreater" => Some("\u{226B}"),
        "NestedLessLess" => Some("\u{226A}"),
        "NewLine" => Some("\n"),
        "NoBreak" => Some("\u{2060}"),
        "NonBreakingSpace" => Some("\u{00A0}"),
        "Not" => Some("\u{2AEC}"),
        "NotCupCap" => Some("\u{226D}"),
        "NotElement" => Some("\u{2209}"),
        "NotEqual" => Some("\u{2260}"),
        "NotEqualTilde" => Some("\u{2242}\u{0338}"),
        "NotExists" => Some("\u{2204}"),
        "NotGreater" => Some("\u{226F}"),
        "NotGreaterEqual" => Some("\u{2271}"),
        "NotGreaterFullEqual" => Some("\u{2267}\u{0338}"),
        "NotGreaterGreater" => Some("\u{226B}\u{0338}"),
        "NotGreaterLess" => Some("\u{2279}"),
        "NotGreaterSlantEqual" => Some("\u{2A7E}\u{0338}"),
        "NotGreaterTilde" => Some("\u{2275}"),
        "NotLess" => Some("\u{226E}"),
        "NotLessEqual" => Some("\u{2270}"),
        "NotLessGreater" => Some("\u{2278}"),
        "NotLessLess" => Some("\u{226A}\u{0338}"),
        "NotLessSlantEqual" => Some("\u{2A7D}\u{0338}"),
        "NotLessTilde" => Some("\u{2274}"),
        "NotNestedGreaterGreater" => Some("\u{2AA2}\u{0338}"),
        "NotNestedLessLess" => Some("\u{2AA1}\u{0338}"),
        "NotPrecedes" => Some("\u{2280}"),
        "NotPrecedesEqual" => Some("\u{2AAF}\u{0338}"),
        "NotPrecedesSlantEqual" => Some("\u{22E0}"),
        "NotReverseElement" => Some("\u{220C}"),
        "NotSubset" => Some("\u{2282}\u{20D2}"),
        "NotSubsetEqual" => Some("\u{2288}"),
        "NotSucceeds" => Some("\u{2281}"),
        "NotSucceedsEqual" => Some("\u{2AB0}\u{0338}"),
        "NotSucceedsSlantEqual" => Some("\u{22E1}"),
        "NotSucceedsTilde" => Some("\u{227F}\u{0338}"),
        "NotTilde" => Some("\u{2241}"),
        "NotTildeEqual" => Some("\u{2244}"),
        "NotTildeFullEqual" => Some("\u{2247}"),
        "NotTildeTilde" => Some("\u{2249}"),
        "NotVerticalBar" => Some("\u{2224}"),
        "Nu" => Some("\u{039D}"),
        "Ntilde" => Some("\u{00D1}"),
        "OElig" => Some("\u{0152}"),
        "Oacute" => Some("\u{00D3}"),
        "Ocirc" => Some("\u{00D4}"),
        "Ograve" => Some("\u{00D2}"),
        "Omega" => Some("\u{03A9}"),
        "Omicron" => Some("\u{039F}"),
        "OpenCurlyDoubleQuote" => Some("\u{201C}"),
        "OpenCurlyQuote" => Some("\u{2018}"),
        "Or" => Some("\u{2A54}"),
        "OverBar" => Some("\u{203E}"),
        "OverBrace" => Some("\u{23DE}"),
        "OverBracket" => Some("\u{23B4}"),
        "OverParenthesis" => Some("\u{23DC}"),
        "Oslash" => Some("\u{00D8}"),
        "Otilde" => Some("\u{00D5}"),
        "Ouml" => Some("\u{00D6}"),
        "Phi" => Some("\u{03A6}"),
        "Pi" => Some("\u{03A0}"),
        "Precedes" => Some("\u{227A}"),
        "PrecedesEqual" => Some("\u{2AAF}"),
        "PrecedesSlantEqual" => Some("\u{227C}"),
        "PrecedesTilde" => Some("\u{227E}"),
        "Prime" => Some("\u{2033}"),
        "Product" => Some("\u{220F}"),
        "Proportional" => Some("\u{221D}"),
        "Psi" => Some("\u{03A8}"),
        "QUOT" | "quot" => Some("\""),
        "REG" | "reg" => Some("\u{00AE}"),
        "ReverseElement" => Some("\u{220B}"),
        "RightAngleBracket" => Some("\u{27E9}"),
        "RightArrow" => Some("\u{2192}"),
        "RightArrowBar" => Some("\u{21E5}"),
        "RightArrowLeftArrow" => Some("\u{21C4}"),
        "RightCeiling" => Some("\u{2309}"),
        "RightDoubleBracket" => Some("\u{27E7}"),
        "RightDownVector" => Some("\u{21C2}"),
        "RightDownVectorBar" => Some("\u{2955}"),
        "RightFloor" => Some("\u{230B}"),
        "RightTeeArrow" => Some("\u{21A6}"),
        "RightTriangle" => Some("\u{22B3}"),
        "RightUpVector" => Some("\u{21BE}"),
        "RightUpVectorBar" => Some("\u{2954}"),
        "RightVector" => Some("\u{21C0}"),
        "RightVectorBar" => Some("\u{2953}"),
        "Rho" => Some("\u{03A1}"),
        "Scaron" => Some("\u{0160}"),
        "Sigma" => Some("\u{03A3}"),
        "SmallCircle" => Some("\u{2218}"),
        "Square" => Some("\u{25A1}"),
        "SquareIntersection" => Some("\u{2293}"),
        "SquareSubset" => Some("\u{228F}"),
        "SquareSubsetEqual" => Some("\u{2291}"),
        "SquareSuperset" => Some("\u{2290}"),
        "SquareSupersetEqual" => Some("\u{2292}"),
        "SquareUnion" => Some("\u{2294}"),
        "Sqrt" => Some("\u{221A}"),
        "Subset" => Some("\u{22D0}"),
        "SubsetEqual" => Some("\u{2286}"),
        "Succeeds" => Some("\u{227B}"),
        "SucceedsEqual" => Some("\u{2AB0}"),
        "SucceedsSlantEqual" => Some("\u{227D}"),
        "SucceedsTilde" => Some("\u{227F}"),
        "SuchThat" => Some("\u{220B}"),
        "Sum" => Some("\u{2211}"),
        "Supset" => Some("\u{22D1}"),
        "THORN" => Some("\u{00DE}"),
        "Tab" => Some("\t"),
        "Tau" => Some("\u{03A4}"),
        "Theta" => Some("\u{0398}"),
        "ThickSpace" => Some("\u{205F}\u{200A}"),
        "ThinSpace" => Some("\u{2009}"),
        "Tilde" => Some("\u{223C}"),
        "TildeEqual" => Some("\u{2243}"),
        "TildeFullEqual" => Some("\u{2245}"),
        "TildeTilde" => Some("\u{2248}"),
        "Therefore" => Some("\u{2234}"),
        "Uacute" => Some("\u{00DA}"),
        "UpArrow" => Some("\u{2191}"),
        "UpArrowBar" => Some("\u{2912}"),
        "UpArrowDownArrow" => Some("\u{21C5}"),
        "UpDownArrow" => Some("\u{2195}"),
        "Ucirc" => Some("\u{00DB}"),
        "Ugrave" => Some("\u{00D9}"),
        "Uuml" => Some("\u{00DC}"),
        "Upsi" => Some("\u{03D2}"),
        "Upsilon" => Some("\u{03A5}"),
        "Union" => Some("\u{22C3}"),
        "UnderBar" => Some("_"),
        "UnderBrace" => Some("\u{23DF}"),
        "UnderBracket" => Some("\u{23B5}"),
        "UnderParenthesis" => Some("\u{23DD}"),
        "Vee" => Some("\u{22C1}"),
        "VeryThinSpace" => Some("\u{200A}"),
        "VerticalBar" => Some("\u{2223}"),
        "VerticalLine" => Some("|"),
        "VerticalSeparator" => Some("\u{2758}"),
        "VerticalTilde" => Some("\u{2240}"),
        "Wedge" => Some("\u{22C0}"),
        "Xi" => Some("\u{039E}"),
        "Yacute" => Some("\u{00DD}"),
        "Yuml" => Some("\u{0178}"),
        "ZeroWidthSpace" => Some("\u{200B}"),
        "Zeta" => Some("\u{0396}"),
        "aacute" => Some("\u{00E1}"),
        "acirc" => Some("\u{00E2}"),
        "acute" => Some("\u{00B4}"),
        "aelig" => Some("\u{00E6}"),
        "alefsym" => Some("\u{2135}"),
        "amp" => Some("&"),
        "AMP" => Some("&"),
        "alpha" => Some("\u{03B1}"),
        "and" => Some("\u{2227}"),
        "ang" => Some("\u{2220}"),
        "angle" => Some("\u{2220}"),
        "angmsd" => Some("\u{2221}"),
        "angsph" => Some("\u{2222}"),
        "angrt" => Some("\u{221F}"),
        "angrtvb" => Some("\u{22BE}"),
        "angrtvbd" => Some("\u{299D}"),
        "angst" => Some("\u{00C5}"),
        "angzarr" => Some("\u{237C}"),
        "apos" | "APOS" => Some("'"),
        "asymp" => Some("\u{2248}"),
        "agrave" => Some("\u{00E0}"),
        "aring" => Some("\u{00E5}"),
        "atilde" => Some("\u{00E3}"),
        "auml" => Some("\u{00E4}"),
        "backepsilon" => Some("\u{03F6}"),
        "bdquo" => Some("\u{201E}"),
        "bepsi" => Some("\u{03F6}"),
        "beta" => Some("\u{03B2}"),
        "beth" => Some("\u{2136}"),
        "bigstar" => Some("\u{2605}"),
        "blacksquare" => Some("\u{25AA}"),
        "blacklozenge" => Some("\u{29EB}"),
        "blacktriangleleft" => Some("\u{25C2}"),
        "blacktriangleright" => Some("\u{25B8}"),
        "bbrk" => Some("\u{23B5}"),
        "bbrktbrk" => Some("\u{23B6}"),
        "bcong" => Some("\u{224C}"),
        "boxDL" => Some("\u{2557}"),
        "boxDR" => Some("\u{2554}"),
        "boxDl" => Some("\u{2556}"),
        "boxDr" => Some("\u{2553}"),
        "boxH" => Some("\u{2550}"),
        "boxHD" => Some("\u{2566}"),
        "boxHU" => Some("\u{2569}"),
        "boxHd" => Some("\u{2564}"),
        "boxHu" => Some("\u{2567}"),
        "boxUL" => Some("\u{255D}"),
        "boxUR" => Some("\u{255A}"),
        "boxUl" => Some("\u{255C}"),
        "boxUr" => Some("\u{2559}"),
        "boxV" => Some("\u{2551}"),
        "boxVH" => Some("\u{256C}"),
        "boxVL" => Some("\u{2563}"),
        "boxVR" => Some("\u{2560}"),
        "boxVh" => Some("\u{256B}"),
        "boxVl" => Some("\u{2562}"),
        "boxVr" => Some("\u{255F}"),
        "boxbox" => Some("\u{29C9}"),
        "boxdL" => Some("\u{2555}"),
        "boxdR" => Some("\u{2552}"),
        "boxdl" => Some("\u{2510}"),
        "boxdr" => Some("\u{250C}"),
        "boxh" => Some("\u{2500}"),
        "boxhD" => Some("\u{2565}"),
        "boxhU" => Some("\u{2568}"),
        "boxhd" => Some("\u{252C}"),
        "boxhu" => Some("\u{2534}"),
        "boxminus" => Some("\u{229F}"),
        "boxplus" => Some("\u{229E}"),
        "boxtimes" => Some("\u{22A0}"),
        "boxuL" => Some("\u{255B}"),
        "boxuR" => Some("\u{2558}"),
        "boxul" => Some("\u{2518}"),
        "boxur" => Some("\u{2514}"),
        "boxv" => Some("\u{2502}"),
        "boxvH" => Some("\u{256A}"),
        "boxvL" => Some("\u{2561}"),
        "boxvR" => Some("\u{255E}"),
        "boxvh" => Some("\u{253C}"),
        "boxvl" => Some("\u{2524}"),
        "boxvr" => Some("\u{251C}"),
        "brvbar" => Some("\u{00A6}"),
        "bsim" => Some("\u{223D}"),
        "bsime" => Some("\u{22CD}"),
        "bull" => Some("\u{2022}"),
        "bump" => Some("\u{224E}"),
        "bumpE" => Some("\u{2AAE}"),
        "bumpe" => Some("\u{224F}"),
        "bumpeq" => Some("\u{224F}"),
        "cap" => Some("\u{2229}"),
        "ccedil" => Some("\u{00E7}"),
        "cedil" => Some("\u{00B8}"),
        "cent" => Some("\u{00A2}"),
        "chi" => Some("\u{03C7}"),
        "circ" => Some("\u{02C6}"),
        "circeq" => Some("\u{2257}"),
        "clubsuit" => Some("\u{2663}"),
        "clubs" => Some("\u{2663}"),
        "coloneq" => Some("\u{2254}"),
        "compfn" => Some("\u{2218}"),
        "cong" => Some("\u{2245}"),
        "congdot" => Some("\u{2A6D}"),
        "crarr" => Some("\u{21B5}"),
        "cup" => Some("\u{222A}"),
        "curren" => Some("\u{00A4}"),
        "curlyeqprec" => Some("\u{22DE}"),
        "curlyeqsucc" => Some("\u{22DF}"),
        "cuvee" => Some("\u{22CE}"),
        "cuwed" => Some("\u{22CF}"),
        "dArr" => Some("\u{21D3}"),
        "dagger" => Some("\u{2020}"),
        "daleth" => Some("\u{2138}"),
        "darr" => Some("\u{2193}"),
        "dd" => Some("\u{2146}"),
        "deg" => Some("\u{00B0}"),
        "delta" => Some("\u{03B4}"),
        "diam" => Some("\u{22C4}"),
        "diamond" => Some("\u{22C4}"),
        "diamondsuit" => Some("\u{2666}"),
        "diams" => Some("\u{2666}"),
        "digamma" => Some("\u{03DD}"),
        "divide" => Some("\u{00F7}"),
        "dotsquare" => Some("\u{22A1}"),
        "doteq" => Some("\u{2250}"),
        "doteqdot" => Some("\u{2251}"),
        "eacute" => Some("\u{00E9}"),
        "ecirc" => Some("\u{00EA}"),
        "egrave" => Some("\u{00E8}"),
        "empty" => Some("\u{2205}"),
        "emptyset" => Some("\u{2205}"),
        "emptyv" => Some("\u{2205}"),
        "emsp" => Some("\u{2003}"),
        "ensp" => Some("\u{2002}"),
        "epsi" => Some("\u{03B5}"),
        "epsilon" => Some("\u{03B5}"),
        "epsiv" => Some("\u{03F5}"),
        "eqcirc" => Some("\u{2256}"),
        "eqcolon" => Some("\u{2255}"),
        "eqsim" => Some("\u{2242}"),
        "eqslantgtr" => Some("\u{2A96}"),
        "eqslantless" => Some("\u{2A95}"),
        "equals" => Some("="),
        "equiv" => Some("\u{2261}"),
        "equest" => Some("\u{225F}"),
        "equivDD" => Some("\u{2A78}"),
        "eqvparsl" => Some("\u{29E5}"),
        "eta" => Some("\u{03B7}"),
        "eth" => Some("\u{00F0}"),
        "euro" => Some("\u{20AC}"),
        "ee" => Some("\u{2147}"),
        "exist" => Some("\u{2203}"),
        "euml" => Some("\u{00EB}"),
        "female" => Some("\u{2640}"),
        "fnof" => Some("\u{0192}"),
        "forall" => Some("\u{2200}"),
        "frac12" => Some("\u{00BD}"),
        "frac14" => Some("\u{00BC}"),
        "frac34" => Some("\u{00BE}"),
        "frasl" => Some("\u{2044}"),
        "gamma" => Some("\u{03B3}"),
        "ge" => Some("\u{2265}"),
        "gimel" => Some("\u{2137}"),
        "gl" => Some("\u{2277}"),
        "glE" => Some("\u{2A92}"),
        "gla" => Some("\u{2AA5}"),
        "glj" => Some("\u{2AA4}"),
        "gnE" => Some("\u{2269}"),
        "gnap" => Some("\u{2A8A}"),
        "gnapprox" => Some("\u{2A8A}"),
        "gne" => Some("\u{2A88}"),
        "gneq" => Some("\u{2A88}"),
        "gneqq" => Some("\u{2269}"),
        "gnsim" => Some("\u{22E7}"),
        "gtrapprox" => Some("\u{2A86}"),
        "gtrarr" => Some("\u{2978}"),
        "gtrdot" => Some("\u{22D7}"),
        "gtreqless" => Some("\u{22DB}"),
        "gtreqqless" => Some("\u{2A8C}"),
        "gtrless" => Some("\u{2277}"),
        "gtrsim" => Some("\u{2273}"),
        "hArr" => Some("\u{21D4}"),
        "harr" => Some("\u{2194}"),
        "heartsuit" => Some("\u{2665}"),
        "hearts" => Some("\u{2665}"),
        "hellip" => Some("\u{2026}"),
        "iacute" => Some("\u{00ED}"),
        "icirc" => Some("\u{00EE}"),
        "iexcl" => Some("\u{00A1}"),
        "igrave" => Some("\u{00EC}"),
        "image" => Some("\u{2111}"),
        "infin" => Some("\u{221E}"),
        "int" => Some("\u{222B}"),
        "iota" => Some("\u{03B9}"),
        "isin" => Some("\u{2208}"),
        "isinE" => Some("\u{22F9}"),
        "isinv" => Some("\u{2208}"),
        "iquest" => Some("\u{00BF}"),
        "iuml" => Some("\u{00EF}"),
        "ii" => Some("\u{2148}"),
        "kappa" => Some("\u{03BA}"),
        "kappav" => Some("\u{03F0}"),
        "lArr" => Some("\u{21D0}"),
        "lambda" => Some("\u{03BB}"),
        "lang" => Some("\u{27E8}"),
        "langle" => Some("\u{27E8}"),
        "laquo" => Some("\u{00AB}"),
        "lbrace" => Some("{"),
        "lbrack" => Some("["),
        "lpar" => Some("("),
        "larr" => Some("\u{2190}"),
        "lceil" => Some("\u{2308}"),
        "ldquo" => Some("\u{201C}"),
        "le" => Some("\u{2264}"),
        "lessapprox" => Some("\u{2A85}"),
        "lessdot" => Some("\u{22D6}"),
        "lesseqgtr" => Some("\u{22DA}"),
        "lesseqqgtr" => Some("\u{2A8B}"),
        "lessgtr" => Some("\u{2276}"),
        "lesssim" => Some("\u{2272}"),
        "lfloor" => Some("\u{230A}"),
        "lg" => Some("\u{2276}"),
        "lgE" => Some("\u{2A91}"),
        "lnE" => Some("\u{2268}"),
        "lnap" => Some("\u{2A89}"),
        "lnapprox" => Some("\u{2A89}"),
        "lne" => Some("\u{2A87}"),
        "lneq" => Some("\u{2A87}"),
        "lneqq" => Some("\u{2268}"),
        "lnsim" => Some("\u{22E6}"),
        "llcorner" => Some("\u{231E}"),
        "lrcorner" => Some("\u{231F}"),
        "lowast" => Some("\u{2217}"),
        "lozenge" => Some("\u{25CA}"),
        "lobrk" => Some("\u{27E6}"),
        "loz" => Some("\u{25CA}"),
        "lozf" => Some("\u{29EB}"),
        "lrm" => Some("\u{200E}"),
        "lsaquo" => Some("\u{2039}"),
        "lsquo" => Some("\u{2018}"),
        "macr" => Some("\u{00AF}"),
        "male" => Some("\u{2642}"),
        "malt" => Some("\u{2720}"),
        "maltese" => Some("\u{2720}"),
        "map" => Some("\u{21A6}"),
        "mdash" => Some("\u{2014}"),
        "minus" => Some("\u{2212}"),
        "micro" => Some("\u{00B5}"),
        "measuredangle" => Some("\u{2221}"),
        "mid" => Some("\u{2223}"),
        "midast" => Some("*"),
        "midcir" => Some("\u{2AF0}"),
        "middot" => Some("\u{00B7}"),
        "minusb" => Some("\u{229F}"),
        "mu" => Some("\u{03BC}"),
        "nabla" => Some("\u{2207}"),
        "ndash" => Some("\u{2013}"),
        "ne" => Some("\u{2260}"),
        "ni" => Some("\u{220B}"),
        "niv" => Some("\u{220B}"),
        "not" => Some("\u{00AC}"),
        "notin" => Some("\u{2209}"),
        "notinva" => Some("\u{2209}"),
        "notinvb" => Some("\u{22F7}"),
        "notinvc" => Some("\u{22F6}"),
        "notniva" => Some("\u{220C}"),
        "notnivb" => Some("\u{22FE}"),
        "notnivc" => Some("\u{22FD}"),
        "nexist" => Some("\u{2204}"),
        "nGg" => Some("\u{22D9}\u{0338}"),
        "nGt" => Some("\u{226B}\u{20D2}"),
        "nGtv" => Some("\u{226B}\u{0338}"),
        "nLl" => Some("\u{22D8}\u{0338}"),
        "nLt" => Some("\u{226A}\u{20D2}"),
        "nLtv" => Some("\u{226A}\u{0338}"),
        "npar" => Some("\u{2226}"),
        "nparallel" => Some("\u{2226}"),
        "nparsl" => Some("\u{2AFD}\u{20E5}"),
        "npart" => Some("\u{2202}\u{0338}"),
        "nprec" => Some("\u{2280}"),
        "npreceq" => Some("\u{2AAF}\u{0338}"),
        "nsub" => Some("\u{2284}"),
        "nsubE" => Some("\u{2AC5}\u{0338}"),
        "nsube" => Some("\u{2288}"),
        "nsucc" => Some("\u{2281}"),
        "nsucceq" => Some("\u{2AB0}\u{0338}"),
        "nsup" => Some("\u{2285}"),
        "nsupE" => Some("\u{2AC6}\u{0338}"),
        "nsupe" => Some("\u{2289}"),
        "nu" => Some("\u{03BD}"),
        "oelig" => Some("\u{0153}"),
        "ntilde" => Some("\u{00F1}"),
        "oacute" => Some("\u{00F3}"),
        "ocirc" => Some("\u{00F4}"),
        "ocir" => Some("\u{229A}"),
        "oast" => Some("\u{229B}"),
        "odash" => Some("\u{229D}"),
        "odot" => Some("\u{2299}"),
        "ograve" => Some("\u{00F2}"),
        "oline" => Some("\u{203E}"),
        "omega" => Some("\u{03C9}"),
        "ominus" => Some("\u{2296}"),
        "omicron" => Some("\u{03BF}"),
        "oplus" => Some("\u{2295}"),
        "or" => Some("\u{2228}"),
        "ordf" => Some("\u{00AA}"),
        "ordm" => Some("\u{00BA}"),
        "oslash" => Some("\u{00F8}"),
        "osol" => Some("\u{2298}"),
        "otimes" => Some("\u{2297}"),
        "otilde" => Some("\u{00F5}"),
        "ouml" => Some("\u{00F6}"),
        "para" => Some("\u{00B6}"),
        "par" => Some("\u{2225}"),
        "parallel" => Some("\u{2225}"),
        "parsim" => Some("\u{2AF3}"),
        "parsl" => Some("\u{2AFD}"),
        "part" => Some("\u{2202}"),
        "permil" => Some("\u{2030}"),
        "perp" => Some("\u{22A5}"),
        "phi" => Some("\u{03C6}"),
        "phiv" => Some("\u{03D5}"),
        "pi" => Some("\u{03C0}"),
        "piv" => Some("\u{03D6}"),
        "phone" => Some("\u{260E}"),
        "pluscir" => Some("\u{2A22}"),
        "plusmn" => Some("\u{00B1}"),
        "pound" => Some("\u{00A3}"),
        "pr" => Some("\u{227A}"),
        "prE" => Some("\u{2AB3}"),
        "prap" => Some("\u{2AB7}"),
        "prcue" => Some("\u{227C}"),
        "pre" => Some("\u{2AAF}"),
        "prec" => Some("\u{227A}"),
        "precapprox" => Some("\u{2AB7}"),
        "preccurlyeq" => Some("\u{227C}"),
        "preceq" => Some("\u{2AAF}"),
        "precnapprox" => Some("\u{2AB9}"),
        "precneqq" => Some("\u{2AB5}"),
        "precnsim" => Some("\u{22E8}"),
        "precsim" => Some("\u{227E}"),
        "prime" => Some("\u{2032}"),
        "prod" => Some("\u{220F}"),
        "prop" => Some("\u{221D}"),
        "prnE" => Some("\u{2AB5}"),
        "prnap" => Some("\u{2AB9}"),
        "prnsim" => Some("\u{22E8}"),
        "prsim" => Some("\u{227E}"),
        "psi" => Some("\u{03C8}"),
        "rArr" => Some("\u{21D2}"),
        "radic" => Some("\u{221A}"),
        "raquo" => Some("\u{00BB}"),
        "rang" => Some("\u{27E9}"),
        "rangle" => Some("\u{27E9}"),
        "rbrace" => Some("}"),
        "rbrack" => Some("]"),
        "rpar" => Some(")"),
        "rarr" => Some("\u{2192}"),
        "rceil" => Some("\u{2309}"),
        "rdquo" => Some("\u{201D}"),
        "real" => Some("\u{211C}"),
        "rho" => Some("\u{03C1}"),
        "rhov" => Some("\u{03F1}"),
        "rlm" => Some("\u{200F}"),
        "rfloor" => Some("\u{230B}"),
        "robrk" => Some("\u{27E7}"),
        "rsaquo" => Some("\u{203A}"),
        "rsquo" => Some("\u{2019}"),
        "sbquo" => Some("\u{201A}"),
        "sc" => Some("\u{227B}"),
        "scE" => Some("\u{2AB4}"),
        "scap" => Some("\u{2AB8}"),
        "scaron" => Some("\u{0161}"),
        "sccue" => Some("\u{227D}"),
        "sce" => Some("\u{2AB0}"),
        "scnE" => Some("\u{2AB6}"),
        "scnap" => Some("\u{2ABA}"),
        "scnsim" => Some("\u{22E9}"),
        "scsim" => Some("\u{227F}"),
        "sdot" => Some("\u{22C5}"),
        "sdotb" => Some("\u{22A1}"),
        "sect" => Some("\u{00A7}"),
        "shy" => Some("\u{00AD}"),
        "sigma" => Some("\u{03C3}"),
        "sigmaf" => Some("\u{03C2}"),
        "sigmav" => Some("\u{03C2}"),
        "sim" => Some("\u{223C}"),
        "simdot" => Some("\u{2A6A}"),
        "sime" => Some("\u{2243}"),
        "simeq" => Some("\u{2243}"),
        "simg" => Some("\u{2A9E}"),
        "simgE" => Some("\u{2AA0}"),
        "siml" => Some("\u{2A9D}"),
        "simlE" => Some("\u{2A9F}"),
        "simne" => Some("\u{2246}"),
        "simplus" => Some("\u{2A24}"),
        "simrarr" => Some("\u{2972}"),
        "setminus" => Some("\u{2216}"),
        "shortmid" => Some("\u{2223}"),
        "shortparallel" => Some("\u{2225}"),
        "smallsetminus" => Some("\u{2216}"),
        "spades" => Some("\u{2660}"),
        "spadesuit" => Some("\u{2660}"),
        "sqcap" => Some("\u{2293}"),
        "sqcup" => Some("\u{2294}"),
        "squ" => Some("\u{25A1}"),
        "square" => Some("\u{25A1}"),
        "sqsub" => Some("\u{228F}"),
        "sqsube" => Some("\u{2291}"),
        "sqsup" => Some("\u{2290}"),
        "sqsupe" => Some("\u{2292}"),
        "squarf" => Some("\u{25AA}"),
        "squf" => Some("\u{25AA}"),
        "star" => Some("\u{2606}"),
        "starf" => Some("\u{2605}"),
        "straightepsilon" => Some("\u{03F5}"),
        "straightphi" => Some("\u{03D5}"),
        "sub" => Some("\u{2282}"),
        "subE" => Some("\u{2AC5}"),
        "sube" => Some("\u{2286}"),
        "subnE" => Some("\u{2ACB}"),
        "subne" => Some("\u{228A}"),
        "succ" => Some("\u{227B}"),
        "succapprox" => Some("\u{2AB8}"),
        "succcurlyeq" => Some("\u{227D}"),
        "succeq" => Some("\u{2AB0}"),
        "succnapprox" => Some("\u{2ABA}"),
        "succneqq" => Some("\u{2AB6}"),
        "succnsim" => Some("\u{22E9}"),
        "succsim" => Some("\u{227F}"),
        "sum" => Some("\u{2211}"),
        "sup1" => Some("\u{00B9}"),
        "sup2" => Some("\u{00B2}"),
        "sup3" => Some("\u{00B3}"),
        "sup" => Some("\u{2283}"),
        "supE" => Some("\u{2AC6}"),
        "supe" => Some("\u{2287}"),
        "supnE" => Some("\u{2ACC}"),
        "supne" => Some("\u{228B}"),
        "szlig" => Some("\u{00DF}"),
        "tau" => Some("\u{03C4}"),
        "there4" => Some("\u{2234}"),
        "theta" => Some("\u{03B8}"),
        "thetasym" => Some("\u{03D1}"),
        "thetav" => Some("\u{03D1}"),
        "thinsp" => Some("\u{2009}"),
        "thorn" => Some("\u{00FE}"),
        "tilde" => Some("\u{02DC}"),
        "times" => Some("\u{00D7}"),
        "timesb" => Some("\u{22A0}"),
        "trade" => Some("\u{2122}"),
        "triangleleft" => Some("\u{25C3}"),
        "triangleright" => Some("\u{25B9}"),
        "uArr" => Some("\u{21D1}"),
        "uacute" => Some("\u{00FA}"),
        "uarr" => Some("\u{2191}"),
        "ucirc" => Some("\u{00FB}"),
        "ugrave" => Some("\u{00F9}"),
        "uml" => Some("\u{00A8}"),
        "upsi" => Some("\u{03C5}"),
        "uuml" => Some("\u{00FC}"),
        "upsih" => Some("\u{03D2}"),
        "upsilon" => Some("\u{03C5}"),
        "ulcorner" => Some("\u{231C}"),
        "urcorner" => Some("\u{231D}"),
        "varepsilon" => Some("\u{03F5}"),
        "varkappa" => Some("\u{03F0}"),
        "varphi" => Some("\u{03D5}"),
        "varnothing" => Some("\u{2205}"),
        "varpi" => Some("\u{03D6}"),
        "varrho" => Some("\u{03F1}"),
        "varsigma" => Some("\u{03C2}"),
        "vartheta" => Some("\u{03D1}"),
        "weierp" => Some("\u{2118}"),
        "xcap" => Some("\u{22C2}"),
        "xcup" => Some("\u{22C3}"),
        "xvee" => Some("\u{22C1}"),
        "xwedge" => Some("\u{22C0}"),
        "xi" => Some("\u{03BE}"),
        "yacute" => Some("\u{00FD}"),
        "yen" => Some("\u{00A5}"),
        "zeta" => Some("\u{03B6}"),
        "zwj" => Some("\u{200D}"),
        "zwnj" => Some("\u{200C}"),
        "yuml" => Some("\u{00FF}"),
        "Abreve" => Some("\u{0102}"),
        "Amacr" => Some("\u{0100}"),
        "Aogon" => Some("\u{0104}"),
        "Cacute" => Some("\u{0106}"),
        "Ccaron" => Some("\u{010C}"),
        "Ccirc" => Some("\u{0108}"),
        "Cdot" => Some("\u{010A}"),
        "Dcaron" => Some("\u{010E}"),
        "Ecaron" => Some("\u{011A}"),
        "Edot" => Some("\u{0116}"),
        "Emacr" => Some("\u{0112}"),
        "Eogon" => Some("\u{0118}"),
        "Gbreve" => Some("\u{011E}"),
        "Gcedil" => Some("\u{0122}"),
        "Gcirc" => Some("\u{011C}"),
        "Gdot" => Some("\u{0120}"),
        "Hcirc" => Some("\u{0124}"),
        "Idot" => Some("\u{0130}"),
        "Imacr" => Some("\u{012A}"),
        "Iogon" => Some("\u{012E}"),
        "Itilde" => Some("\u{0128}"),
        "Jcirc" => Some("\u{0134}"),
        "Kcedil" => Some("\u{0136}"),
        "Lacute" => Some("\u{0139}"),
        "Lcaron" => Some("\u{013D}"),
        "Lcedil" => Some("\u{013B}"),
        "Lmidot" => Some("\u{013F}"),
        "Nacute" => Some("\u{0143}"),
        "Ncaron" => Some("\u{0147}"),
        "Ncedil" => Some("\u{0145}"),
        "Omacr" => Some("\u{014C}"),
        "Racute" => Some("\u{0154}"),
        "Rcaron" => Some("\u{0158}"),
        "Rcedil" => Some("\u{0156}"),
        "Sacute" => Some("\u{015A}"),
        "Scedil" => Some("\u{015E}"),
        "Scirc" => Some("\u{015C}"),
        "Tcaron" => Some("\u{0164}"),
        "Tcedil" => Some("\u{0162}"),
        "Ubreve" => Some("\u{016C}"),
        "Umacr" => Some("\u{016A}"),
        "Uogon" => Some("\u{0172}"),
        "Uring" => Some("\u{016E}"),
        "Utilde" => Some("\u{0168}"),
        "Wcirc" => Some("\u{0174}"),
        "Ycirc" => Some("\u{0176}"),
        "Zacute" => Some("\u{0179}"),
        "Zcaron" => Some("\u{017D}"),
        "Zdot" => Some("\u{017B}"),
        "abreve" => Some("\u{0103}"),
        "amacr" => Some("\u{0101}"),
        "aogon" => Some("\u{0105}"),
        "cacute" => Some("\u{0107}"),
        "ccaron" => Some("\u{010D}"),
        "ccirc" => Some("\u{0109}"),
        "cdot" => Some("\u{010B}"),
        "dcaron" => Some("\u{010F}"),
        "ecaron" => Some("\u{011B}"),
        "edot" => Some("\u{0117}"),
        "emacr" => Some("\u{0113}"),
        "eogon" => Some("\u{0119}"),
        "gbreve" => Some("\u{011F}"),
        "gcirc" => Some("\u{011D}"),
        "gdot" => Some("\u{0121}"),
        "hcirc" => Some("\u{0125}"),
        "imacr" => Some("\u{012B}"),
        "inodot" => Some("\u{0131}"),
        "iogon" => Some("\u{012F}"),
        "itilde" => Some("\u{0129}"),
        "jcirc" => Some("\u{0135}"),
        "kcedil" => Some("\u{0137}"),
        "lacute" => Some("\u{013A}"),
        "lcaron" => Some("\u{013E}"),
        "lcedil" => Some("\u{013C}"),
        "lmidot" => Some("\u{0140}"),
        "nacute" => Some("\u{0144}"),
        "ncaron" => Some("\u{0148}"),
        "ncedil" => Some("\u{0146}"),
        "omacr" => Some("\u{014D}"),
        "racute" => Some("\u{0155}"),
        "rcaron" => Some("\u{0159}"),
        "rcedil" => Some("\u{0157}"),
        "sacute" => Some("\u{015B}"),
        "scedil" => Some("\u{015F}"),
        "scirc" => Some("\u{015D}"),
        "tcaron" => Some("\u{0165}"),
        "tcedil" => Some("\u{0163}"),
        "ubreve" => Some("\u{016D}"),
        "umacr" => Some("\u{016B}"),
        "uogon" => Some("\u{0173}"),
        "uring" => Some("\u{016F}"),
        "utilde" => Some("\u{0169}"),
        "wcirc" => Some("\u{0175}"),
        "ycirc" => Some("\u{0177}"),
        "zacute" => Some("\u{017A}"),
        "zcaron" => Some("\u{017E}"),
        "zdot" => Some("\u{017C}"),
        "Darr" => Some("\u{21A1}"),
        "Downarrow" => Some("\u{21D3}"),
        "Larr" => Some("\u{219E}"),
        "Leftarrow" => Some("\u{21D0}"),
        "Leftrightarrow" => Some("\u{21D4}"),
        "Rarr" => Some("\u{21A0}"),
        "Rightarrow" => Some("\u{21D2}"),
        "Uarr" => Some("\u{219F}"),
        "Uparrow" => Some("\u{21D1}"),
        "Updownarrow" => Some("\u{21D5}"),
        "ShortDownArrow" => Some("\u{2193}"),
        "ShortLeftArrow" => Some("\u{2190}"),
        "ShortRightArrow" => Some("\u{2192}"),
        "ShortUpArrow" => Some("\u{2191}"),
        "LowerLeftArrow" => Some("\u{2199}"),
        "LowerRightArrow" => Some("\u{2198}"),
        "UpperLeftArrow" => Some("\u{2196}"),
        "UpperRightArrow" => Some("\u{2197}"),
        "DoubleLongLeftArrow" => Some("\u{27F8}"),
        "DoubleLongLeftRightArrow" => Some("\u{27FA}"),
        "DoubleLongRightArrow" => Some("\u{27F9}"),
        "DownTeeArrow" => Some("\u{21A7}"),
        "UpTeeArrow" => Some("\u{21A5}"),
        "downarrow" => Some("\u{2193}"),
        "leftarrow" => Some("\u{2190}"),
        "leftrightarrow" => Some("\u{2194}"),
        "longleftarrow" => Some("\u{27F5}"),
        "longleftrightarrow" => Some("\u{27F7}"),
        "longrightarrow" => Some("\u{27F6}"),
        "hookleftarrow" => Some("\u{21A9}"),
        "hookrightarrow" => Some("\u{21AA}"),
        "leftarrowtail" => Some("\u{21A2}"),
        "rightarrowtail" => Some("\u{21A3}"),
        "twoheadleftarrow" => Some("\u{219E}"),
        "twoheadrightarrow" => Some("\u{21A0}"),
        "curvearrowleft" => Some("\u{21B6}"),
        "curvearrowright" => Some("\u{21B7}"),
        "circlearrowleft" => Some("\u{21BA}"),
        "circlearrowright" => Some("\u{21BB}"),
        "looparrowleft" => Some("\u{21AB}"),
        "looparrowright" => Some("\u{21AC}"),
        "leftharpoonup" => Some("\u{21BC}"),
        "leftharpoondown" => Some("\u{21BD}"),
        "rightharpoonup" => Some("\u{21C0}"),
        "rightharpoondown" => Some("\u{21C1}"),
        "upharpoonleft" => Some("\u{21BF}"),
        "upharpoonright" => Some("\u{21BE}"),
        "downharpoonleft" => Some("\u{21C3}"),
        "downharpoonright" => Some("\u{21C2}"),
        "leftleftarrows" => Some("\u{21C7}"),
        "rightrightarrows" => Some("\u{21C9}"),
        "downdownarrows" => Some("\u{21CA}"),
        "upuparrows" => Some("\u{21C8}"),
        "leftrightarrows" => Some("\u{21C6}"),
        "rightleftarrows" => Some("\u{21C4}"),
        "leftrightharpoons" => Some("\u{21CB}"),
        "rightleftharpoons" => Some("\u{21CC}"),
        "rightsquigarrow" => Some("\u{219D}"),
        "leftrightsquigarrow" => Some("\u{21AD}"),
        "nleftarrow" => Some("\u{219A}"),
        "nrightarrow" => Some("\u{219B}"),
        "nleftrightarrow" => Some("\u{21AE}"),
        "nLeftarrow" => Some("\u{21CD}"),
        "nRightarrow" => Some("\u{21CF}"),
        "nLeftrightarrow" => Some("\u{21CE}"),
        "mapsto" => Some("\u{21A6}"),
        "longmapsto" => Some("\u{27FC}"),
        "mapstoleft" => Some("\u{21A4}"),
        "mapstoup" => Some("\u{21A5}"),
        "mapstodown" => Some("\u{21A7}"),
        "nearrow" => Some("\u{2197}"),
        "searrow" => Some("\u{2198}"),
        "swarrow" => Some("\u{2199}"),
        "nwarrow" => Some("\u{2196}"),
        "Aopf" => Some("\u{1D538}"),
        "Bopf" => Some("\u{1D539}"),
        "Copf" => Some("\u{2102}"),
        "Dopf" => Some("\u{1D53B}"),
        "Eopf" => Some("\u{1D53C}"),
        "Fopf" => Some("\u{1D53D}"),
        "Gopf" => Some("\u{1D53E}"),
        "Hopf" => Some("\u{210D}"),
        "Iopf" => Some("\u{1D540}"),
        "Jopf" => Some("\u{1D541}"),
        "Kopf" => Some("\u{1D542}"),
        "Lopf" => Some("\u{1D543}"),
        "Mopf" => Some("\u{1D544}"),
        "Nopf" => Some("\u{2115}"),
        "Oopf" => Some("\u{1D546}"),
        "Popf" => Some("\u{2119}"),
        "Qopf" => Some("\u{211A}"),
        "Ropf" => Some("\u{211D}"),
        "Sopf" => Some("\u{1D54A}"),
        "Topf" => Some("\u{1D54B}"),
        "Uopf" => Some("\u{1D54C}"),
        "Vopf" => Some("\u{1D54D}"),
        "Wopf" => Some("\u{1D54E}"),
        "Xopf" => Some("\u{1D54F}"),
        "Yopf" => Some("\u{1D550}"),
        "Zopf" => Some("\u{2124}"),
        "aopf" => Some("\u{1D552}"),
        "bopf" => Some("\u{1D553}"),
        "copf" => Some("\u{1D554}"),
        "dopf" => Some("\u{1D555}"),
        "eopf" => Some("\u{1D556}"),
        "fopf" => Some("\u{1D557}"),
        "gopf" => Some("\u{1D558}"),
        "hopf" => Some("\u{1D559}"),
        "iopf" => Some("\u{1D55A}"),
        "jopf" => Some("\u{1D55B}"),
        "kopf" => Some("\u{1D55C}"),
        "lopf" => Some("\u{1D55D}"),
        "mopf" => Some("\u{1D55E}"),
        "nopf" => Some("\u{1D55F}"),
        "oopf" => Some("\u{1D560}"),
        "popf" => Some("\u{1D561}"),
        "qopf" => Some("\u{1D562}"),
        "ropf" => Some("\u{1D563}"),
        "sopf" => Some("\u{1D564}"),
        "topf" => Some("\u{1D565}"),
        "uopf" => Some("\u{1D566}"),
        "vopf" => Some("\u{1D567}"),
        "wopf" => Some("\u{1D568}"),
        "xopf" => Some("\u{1D569}"),
        "yopf" => Some("\u{1D56A}"),
        "zopf" => Some("\u{1D56B}"),
        "Ascr" => Some("\u{1D49C}"),
        "Bscr" => Some("\u{212C}"),
        "Cscr" => Some("\u{1D49E}"),
        "Dscr" => Some("\u{1D49F}"),
        "Escr" => Some("\u{2130}"),
        "Fscr" => Some("\u{2131}"),
        "Gscr" => Some("\u{1D4A2}"),
        "Hscr" => Some("\u{210B}"),
        "Iscr" => Some("\u{2110}"),
        "Jscr" => Some("\u{1D4A5}"),
        "Kscr" => Some("\u{1D4A6}"),
        "Lscr" => Some("\u{2112}"),
        "Mscr" => Some("\u{2133}"),
        "Nscr" => Some("\u{1D4A9}"),
        "Oscr" => Some("\u{1D4AA}"),
        "Pscr" => Some("\u{1D4AB}"),
        "Qscr" => Some("\u{1D4AC}"),
        "Rscr" => Some("\u{211B}"),
        "Sscr" => Some("\u{1D4AE}"),
        "Tscr" => Some("\u{1D4AF}"),
        "Uscr" => Some("\u{1D4B0}"),
        "Vscr" => Some("\u{1D4B1}"),
        "Wscr" => Some("\u{1D4B2}"),
        "Xscr" => Some("\u{1D4B3}"),
        "Yscr" => Some("\u{1D4B4}"),
        "Zscr" => Some("\u{1D4B5}"),
        "ascr" => Some("\u{1D4B6}"),
        "bscr" => Some("\u{1D4B7}"),
        "cscr" => Some("\u{1D4B8}"),
        "dscr" => Some("\u{1D4B9}"),
        "escr" => Some("\u{212F}"),
        "fscr" => Some("\u{1D4BB}"),
        "gscr" => Some("\u{210A}"),
        "hscr" => Some("\u{1D4BD}"),
        "iscr" => Some("\u{1D4BE}"),
        "jscr" => Some("\u{1D4BF}"),
        "kscr" => Some("\u{1D4C0}"),
        "lscr" => Some("\u{1D4C1}"),
        "mscr" => Some("\u{1D4C2}"),
        "nscr" => Some("\u{1D4C3}"),
        "oscr" => Some("\u{2134}"),
        "pscr" => Some("\u{1D4C5}"),
        "qscr" => Some("\u{1D4C6}"),
        "rscr" => Some("\u{1D4C7}"),
        "sscr" => Some("\u{1D4C8}"),
        "tscr" => Some("\u{1D4C9}"),
        "uscr" => Some("\u{1D4CA}"),
        "vscr" => Some("\u{1D4CB}"),
        "wscr" => Some("\u{1D4CC}"),
        "xscr" => Some("\u{1D4CD}"),
        "yscr" => Some("\u{1D4CE}"),
        "zscr" => Some("\u{1D4CF}"),
        "Afr" => Some("\u{1D504}"),
        "Bfr" => Some("\u{1D505}"),
        "Cfr" => Some("\u{212D}"),
        "Dfr" => Some("\u{1D507}"),
        "Efr" => Some("\u{1D508}"),
        "Ffr" => Some("\u{1D509}"),
        "Gfr" => Some("\u{1D50A}"),
        "Hfr" => Some("\u{210C}"),
        "Ifr" => Some("\u{2111}"),
        "Jfr" => Some("\u{1D50D}"),
        "Kfr" => Some("\u{1D50E}"),
        "Lfr" => Some("\u{1D50F}"),
        "Mfr" => Some("\u{1D510}"),
        "Nfr" => Some("\u{1D511}"),
        "Ofr" => Some("\u{1D512}"),
        "Pfr" => Some("\u{1D513}"),
        "Qfr" => Some("\u{1D514}"),
        "Rfr" => Some("\u{211C}"),
        "Sfr" => Some("\u{1D516}"),
        "Tfr" => Some("\u{1D517}"),
        "Ufr" => Some("\u{1D518}"),
        "Vfr" => Some("\u{1D519}"),
        "Wfr" => Some("\u{1D51A}"),
        "Xfr" => Some("\u{1D51B}"),
        "Yfr" => Some("\u{1D51C}"),
        "Zfr" => Some("\u{2128}"),
        "afr" => Some("\u{1D51E}"),
        "bfr" => Some("\u{1D51F}"),
        "cfr" => Some("\u{1D520}"),
        "dfr" => Some("\u{1D521}"),
        "efr" => Some("\u{1D522}"),
        "ffr" => Some("\u{1D523}"),
        "gfr" => Some("\u{1D524}"),
        "hfr" => Some("\u{1D525}"),
        "ifr" => Some("\u{1D526}"),
        "jfr" => Some("\u{1D527}"),
        "kfr" => Some("\u{1D528}"),
        "lfr" => Some("\u{1D529}"),
        "mfr" => Some("\u{1D52A}"),
        "nfr" => Some("\u{1D52B}"),
        "ofr" => Some("\u{1D52C}"),
        "pfr" => Some("\u{1D52D}"),
        "qfr" => Some("\u{1D52E}"),
        "rfr" => Some("\u{1D52F}"),
        "sfr" => Some("\u{1D530}"),
        "tfr" => Some("\u{1D531}"),
        "ufr" => Some("\u{1D532}"),
        "vfr" => Some("\u{1D533}"),
        "wfr" => Some("\u{1D534}"),
        "xfr" => Some("\u{1D535}"),
        "yfr" => Some("\u{1D536}"),
        "zfr" => Some("\u{1D537}"),
        "Acy" => Some("\u{410}"),
        "acy" => Some("\u{430}"),
        "Bcy" => Some("\u{411}"),
        "bcy" => Some("\u{431}"),
        "CHcy" => Some("\u{427}"),
        "chcy" => Some("\u{447}"),
        "Dcy" => Some("\u{414}"),
        "dcy" => Some("\u{434}"),
        "DJcy" => Some("\u{402}"),
        "djcy" => Some("\u{452}"),
        "DScy" => Some("\u{405}"),
        "dscy" => Some("\u{455}"),
        "DZcy" => Some("\u{40F}"),
        "dzcy" => Some("\u{45F}"),
        "Ecy" => Some("\u{42D}"),
        "ecy" => Some("\u{44D}"),
        "Fcy" => Some("\u{424}"),
        "fcy" => Some("\u{444}"),
        "Gcy" => Some("\u{413}"),
        "gcy" => Some("\u{433}"),
        "GJcy" => Some("\u{403}"),
        "gjcy" => Some("\u{453}"),
        "HARDcy" => Some("\u{42A}"),
        "hardcy" => Some("\u{44A}"),
        "Icy" => Some("\u{418}"),
        "icy" => Some("\u{438}"),
        "IEcy" => Some("\u{415}"),
        "iecy" => Some("\u{435}"),
        "IOcy" => Some("\u{401}"),
        "iocy" => Some("\u{451}"),
        "Iukcy" => Some("\u{406}"),
        "iukcy" => Some("\u{456}"),
        "Jcy" => Some("\u{419}"),
        "jcy" => Some("\u{439}"),
        "Jsercy" => Some("\u{408}"),
        "jsercy" => Some("\u{458}"),
        "Jukcy" => Some("\u{404}"),
        "jukcy" => Some("\u{454}"),
        "Kcy" => Some("\u{41A}"),
        "kcy" => Some("\u{43A}"),
        "KHcy" => Some("\u{425}"),
        "khcy" => Some("\u{445}"),
        "KJcy" => Some("\u{40C}"),
        "kjcy" => Some("\u{45C}"),
        "Lcy" => Some("\u{41B}"),
        "lcy" => Some("\u{43B}"),
        "LJcy" => Some("\u{409}"),
        "ljcy" => Some("\u{459}"),
        "Mcy" => Some("\u{41C}"),
        "mcy" => Some("\u{43C}"),
        "Ncy" => Some("\u{41D}"),
        "ncy" => Some("\u{43D}"),
        "NJcy" => Some("\u{40A}"),
        "njcy" => Some("\u{45A}"),
        "Ocy" => Some("\u{41E}"),
        "ocy" => Some("\u{43E}"),
        "Pcy" => Some("\u{41F}"),
        "pcy" => Some("\u{43F}"),
        "Rcy" => Some("\u{420}"),
        "rcy" => Some("\u{440}"),
        "Scy" => Some("\u{421}"),
        "scy" => Some("\u{441}"),
        "SHCHcy" => Some("\u{429}"),
        "shchcy" => Some("\u{449}"),
        "SHcy" => Some("\u{428}"),
        "shcy" => Some("\u{448}"),
        "SOFTcy" => Some("\u{42C}"),
        "softcy" => Some("\u{44C}"),
        "Tcy" => Some("\u{422}"),
        "tcy" => Some("\u{442}"),
        "TScy" => Some("\u{426}"),
        "tscy" => Some("\u{446}"),
        "TSHcy" => Some("\u{40B}"),
        "tshcy" => Some("\u{45B}"),
        "Ubrcy" => Some("\u{40E}"),
        "ubrcy" => Some("\u{45E}"),
        "Ucy" => Some("\u{423}"),
        "ucy" => Some("\u{443}"),
        "Vcy" => Some("\u{412}"),
        "vcy" => Some("\u{432}"),
        "YAcy" => Some("\u{42F}"),
        "yacy" => Some("\u{44F}"),
        "Ycy" => Some("\u{42B}"),
        "ycy" => Some("\u{44B}"),
        "YIcy" => Some("\u{407}"),
        "yicy" => Some("\u{457}"),
        "YUcy" => Some("\u{42E}"),
        "yucy" => Some("\u{44E}"),
        "Zcy" => Some("\u{417}"),
        "zcy" => Some("\u{437}"),
        "ZHcy" => Some("\u{416}"),
        "zhcy" => Some("\u{436}"),
        _ => None,
    }
}
