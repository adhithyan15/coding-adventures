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
        "Aring" => Some("\u{00C5}"),
        "Atilde" => Some("\u{00C3}"),
        "Auml" => Some("\u{00C4}"),
        "Beta" => Some("\u{0392}"),
        "Ccedil" => Some("\u{00C7}"),
        "Chi" => Some("\u{03A7}"),
        "COPY" | "copy" => Some("\u{00A9}"),
        "Dagger" => Some("\u{2021}"),
        "Delta" => Some("\u{0394}"),
        "ETH" => Some("\u{00D0}"),
        "Eacute" => Some("\u{00C9}"),
        "Ecirc" => Some("\u{00CA}"),
        "Egrave" => Some("\u{00C8}"),
        "Epsilon" => Some("\u{0395}"),
        "Eta" => Some("\u{0397}"),
        "Euml" => Some("\u{00CB}"),
        "Gamma" => Some("\u{0393}"),
        "GT" | "gt" => Some(">"),
        "Iacute" => Some("\u{00CD}"),
        "Icirc" => Some("\u{00CE}"),
        "Igrave" => Some("\u{00CC}"),
        "Iota" => Some("\u{0399}"),
        "Iuml" => Some("\u{00CF}"),
        "Kappa" => Some("\u{039A}"),
        "LT" | "lt" => Some("<"),
        "Lambda" => Some("\u{039B}"),
        "Mu" => Some("\u{039C}"),
        "NBSP" | "nbsp" => Some("\u{00A0}"),
        "Nu" => Some("\u{039D}"),
        "Ntilde" => Some("\u{00D1}"),
        "OElig" => Some("\u{0152}"),
        "Oacute" => Some("\u{00D3}"),
        "Ocirc" => Some("\u{00D4}"),
        "Ograve" => Some("\u{00D2}"),
        "Omega" => Some("\u{03A9}"),
        "Omicron" => Some("\u{039F}"),
        "Oslash" => Some("\u{00D8}"),
        "Otilde" => Some("\u{00D5}"),
        "Ouml" => Some("\u{00D6}"),
        "Phi" => Some("\u{03A6}"),
        "Pi" => Some("\u{03A0}"),
        "Prime" => Some("\u{2033}"),
        "Psi" => Some("\u{03A8}"),
        "QUOT" | "quot" => Some("\""),
        "REG" | "reg" => Some("\u{00AE}"),
        "Rho" => Some("\u{03A1}"),
        "Scaron" => Some("\u{0160}"),
        "Sigma" => Some("\u{03A3}"),
        "THORN" => Some("\u{00DE}"),
        "Tau" => Some("\u{03A4}"),
        "Theta" => Some("\u{0398}"),
        "Uacute" => Some("\u{00DA}"),
        "Ucirc" => Some("\u{00DB}"),
        "Ugrave" => Some("\u{00D9}"),
        "Uuml" => Some("\u{00DC}"),
        "Upsilon" => Some("\u{03A5}"),
        "Xi" => Some("\u{039E}"),
        "Yacute" => Some("\u{00DD}"),
        "Yuml" => Some("\u{0178}"),
        "Zeta" => Some("\u{0396}"),
        "aacute" => Some("\u{00E1}"),
        "acirc" => Some("\u{00E2}"),
        "acute" => Some("\u{00B4}"),
        "aelig" => Some("\u{00E6}"),
        "amp" => Some("&"),
        "AMP" => Some("&"),
        "alpha" => Some("\u{03B1}"),
        "and" => Some("\u{2227}"),
        "ang" => Some("\u{2220}"),
        "apos" | "APOS" => Some("'"),
        "asymp" => Some("\u{2248}"),
        "agrave" => Some("\u{00E0}"),
        "aring" => Some("\u{00E5}"),
        "atilde" => Some("\u{00E3}"),
        "auml" => Some("\u{00E4}"),
        "bdquo" => Some("\u{201E}"),
        "beta" => Some("\u{03B2}"),
        "brvbar" => Some("\u{00A6}"),
        "bull" => Some("\u{2022}"),
        "cap" => Some("\u{2229}"),
        "ccedil" => Some("\u{00E7}"),
        "cedil" => Some("\u{00B8}"),
        "cent" => Some("\u{00A2}"),
        "chi" => Some("\u{03C7}"),
        "circ" => Some("\u{02C6}"),
        "clubs" => Some("\u{2663}"),
        "cong" => Some("\u{2245}"),
        "crarr" => Some("\u{21B5}"),
        "cup" => Some("\u{222A}"),
        "curren" => Some("\u{00A4}"),
        "dArr" => Some("\u{21D3}"),
        "dagger" => Some("\u{2020}"),
        "darr" => Some("\u{2193}"),
        "deg" => Some("\u{00B0}"),
        "delta" => Some("\u{03B4}"),
        "diams" => Some("\u{2666}"),
        "divide" => Some("\u{00F7}"),
        "eacute" => Some("\u{00E9}"),
        "ecirc" => Some("\u{00EA}"),
        "egrave" => Some("\u{00E8}"),
        "empty" => Some("\u{2205}"),
        "emsp" => Some("\u{2003}"),
        "ensp" => Some("\u{2002}"),
        "epsilon" => Some("\u{03B5}"),
        "equiv" => Some("\u{2261}"),
        "eta" => Some("\u{03B7}"),
        "eth" => Some("\u{00F0}"),
        "euro" => Some("\u{20AC}"),
        "exist" => Some("\u{2203}"),
        "euml" => Some("\u{00EB}"),
        "fnof" => Some("\u{0192}"),
        "forall" => Some("\u{2200}"),
        "frac12" => Some("\u{00BD}"),
        "frac14" => Some("\u{00BC}"),
        "frac34" => Some("\u{00BE}"),
        "frasl" => Some("\u{2044}"),
        "gamma" => Some("\u{03B3}"),
        "ge" => Some("\u{2265}"),
        "hArr" => Some("\u{21D4}"),
        "harr" => Some("\u{2194}"),
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
        "iquest" => Some("\u{00BF}"),
        "iuml" => Some("\u{00EF}"),
        "kappa" => Some("\u{03BA}"),
        "lArr" => Some("\u{21D0}"),
        "lambda" => Some("\u{03BB}"),
        "laquo" => Some("\u{00AB}"),
        "larr" => Some("\u{2190}"),
        "lceil" => Some("\u{2308}"),
        "ldquo" => Some("\u{201C}"),
        "le" => Some("\u{2264}"),
        "lfloor" => Some("\u{230A}"),
        "lowast" => Some("\u{2217}"),
        "loz" => Some("\u{25CA}"),
        "lrm" => Some("\u{200E}"),
        "lsaquo" => Some("\u{2039}"),
        "lsquo" => Some("\u{2018}"),
        "macr" => Some("\u{00AF}"),
        "mdash" => Some("\u{2014}"),
        "minus" => Some("\u{2212}"),
        "micro" => Some("\u{00B5}"),
        "middot" => Some("\u{00B7}"),
        "mu" => Some("\u{03BC}"),
        "nabla" => Some("\u{2207}"),
        "ndash" => Some("\u{2013}"),
        "ne" => Some("\u{2260}"),
        "ni" => Some("\u{220B}"),
        "not" => Some("\u{00AC}"),
        "notin" => Some("\u{2209}"),
        "nsub" => Some("\u{2284}"),
        "nu" => Some("\u{03BD}"),
        "oelig" => Some("\u{0153}"),
        "ntilde" => Some("\u{00F1}"),
        "oacute" => Some("\u{00F3}"),
        "ocirc" => Some("\u{00F4}"),
        "ograve" => Some("\u{00F2}"),
        "omega" => Some("\u{03C9}"),
        "omicron" => Some("\u{03BF}"),
        "oplus" => Some("\u{2295}"),
        "or" => Some("\u{2228}"),
        "ordf" => Some("\u{00AA}"),
        "ordm" => Some("\u{00BA}"),
        "oslash" => Some("\u{00F8}"),
        "otimes" => Some("\u{2297}"),
        "otilde" => Some("\u{00F5}"),
        "ouml" => Some("\u{00F6}"),
        "para" => Some("\u{00B6}"),
        "part" => Some("\u{2202}"),
        "permil" => Some("\u{2030}"),
        "perp" => Some("\u{22A5}"),
        "phi" => Some("\u{03C6}"),
        "pi" => Some("\u{03C0}"),
        "piv" => Some("\u{03D6}"),
        "plusmn" => Some("\u{00B1}"),
        "pound" => Some("\u{00A3}"),
        "prime" => Some("\u{2032}"),
        "prod" => Some("\u{220F}"),
        "prop" => Some("\u{221D}"),
        "psi" => Some("\u{03C8}"),
        "rArr" => Some("\u{21D2}"),
        "radic" => Some("\u{221A}"),
        "raquo" => Some("\u{00BB}"),
        "rarr" => Some("\u{2192}"),
        "rceil" => Some("\u{2309}"),
        "rdquo" => Some("\u{201D}"),
        "real" => Some("\u{211C}"),
        "rho" => Some("\u{03C1}"),
        "rlm" => Some("\u{200F}"),
        "rfloor" => Some("\u{230B}"),
        "rsaquo" => Some("\u{203A}"),
        "rsquo" => Some("\u{2019}"),
        "sbquo" => Some("\u{201A}"),
        "scaron" => Some("\u{0161}"),
        "sdot" => Some("\u{22C5}"),
        "sect" => Some("\u{00A7}"),
        "shy" => Some("\u{00AD}"),
        "sigma" => Some("\u{03C3}"),
        "sigmaf" => Some("\u{03C2}"),
        "sim" => Some("\u{223C}"),
        "spades" => Some("\u{2660}"),
        "sub" => Some("\u{2282}"),
        "sube" => Some("\u{2286}"),
        "sum" => Some("\u{2211}"),
        "sup1" => Some("\u{00B9}"),
        "sup2" => Some("\u{00B2}"),
        "sup3" => Some("\u{00B3}"),
        "sup" => Some("\u{2283}"),
        "supe" => Some("\u{2287}"),
        "szlig" => Some("\u{00DF}"),
        "tau" => Some("\u{03C4}"),
        "there4" => Some("\u{2234}"),
        "theta" => Some("\u{03B8}"),
        "thetasym" => Some("\u{03D1}"),
        "thinsp" => Some("\u{2009}"),
        "thorn" => Some("\u{00FE}"),
        "tilde" => Some("\u{02DC}"),
        "times" => Some("\u{00D7}"),
        "trade" => Some("\u{2122}"),
        "uArr" => Some("\u{21D1}"),
        "uacute" => Some("\u{00FA}"),
        "uarr" => Some("\u{2191}"),
        "ucirc" => Some("\u{00FB}"),
        "ugrave" => Some("\u{00F9}"),
        "uml" => Some("\u{00A8}"),
        "uuml" => Some("\u{00FC}"),
        "upsih" => Some("\u{03D2}"),
        "upsilon" => Some("\u{03C5}"),
        "weierp" => Some("\u{2118}"),
        "xi" => Some("\u{03BE}"),
        "yacute" => Some("\u{00FD}"),
        "yen" => Some("\u{00A5}"),
        "zeta" => Some("\u{03B6}"),
        "zwj" => Some("\u{200D}"),
        "zwnj" => Some("\u{200C}"),
        "yuml" => Some("\u{00FF}"),
        _ => None,
    }
}
